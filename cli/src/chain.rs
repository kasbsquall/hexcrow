// SPDX-License-Identifier: MIT
//
// Talking to Esmeralda: a provider per persona, fees paid from that persona's own stealth coin,
// a dry run to price every transaction, and an evidence log of everything that was submitted.

use std::{fs::OpenOptions, io::Write, path::PathBuf, time::Duration};

use anyhow::{Context, Result, anyhow};
use ootle_rs::{
    Address,
    TransactionRequest,
    default_indexer_url,
    provider::{IndexerProvider, ProviderBuilder},
    stealth::{Output, StealthTransfer},
    template_types::{
        Amount,
        UtxoAddress,
        constants::TARI_TOKEN,
        crypto::{PedersenCommitmentBytes, RistrettoPublicKeyBytes},
        stealth::{StealthInput, StealthTransferStatement},
    },
    wallet::OotleWallet,
};
use serde_json::{Value, json};
use tari_ootle_transaction::{Epoch, TransactionBuilder, UnsignedTransaction};

use ootle_rs::{provider::{Provider as _, WalletProvider as _}, transaction::TransactionSigner as _};

use crate::personas::{NETWORK, Personas};

pub type Provider = IndexerProvider<OotleWallet>;

/// Generous ceiling for a dry run. The real submission carries the floor the dry run reports.
pub const DRY_RUN_FEE: u64 = 2_000_000;

pub struct Session {
    pub persona: String,
    pub address: Address,
    pub provider: Provider,
}

impl Session {
    /// A provider whose wallet seals with `persona` and can also decrypt outputs owned by `extra`
    /// (the escrow viewing key, when spending the escrow output).
    pub async fn connect(personas: &Personas, persona: &str, extra: &[&str]) -> Result<Self> {
        let main = personas.provider(persona)?;
        let address = main.address().clone();
        let mut wallet = OotleWallet::from(main);
        for name in extra {
            wallet.register_key_provider(personas.provider(name)?);
        }
        wallet.set_default_signer(&address)?;
        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .connect_with_transaction_timeout(default_indexer_url(NETWORK), Duration::from_secs(180))
            .await
            .context("connecting to the Esmeralda indexer")?;
        Ok(Self {
            persona: persona.to_string(),
            address,
            provider,
        })
    }

    pub fn account_key(&self) -> RistrettoPublicKeyBytes {
        *self.address.account_public_key()
    }

    pub async fn epoch(&self) -> Result<u64> {
        Ok(self.provider.get_epoch().await?.as_u64())
    }

    pub async fn max_epoch(&self) -> Result<Epoch> {
        Ok(Epoch(self.epoch().await? + 10))
    }
}

/// What a fee-paying stealth transfer looks like before the fee is known.
pub struct Spend {
    /// Inputs to spend: (owner address, input). The first one owned by the session pays the fee.
    pub inputs: Vec<(Address, StealthInput)>,
    /// Total value of the inputs, in µT.
    pub input_value: u64,
    /// Outputs other than change, in order.
    pub outputs: Vec<Output>,
    /// Where the change goes (None means no change output).
    pub change_to: Option<Address>,
}

pub struct Prepared {
    pub statement: StealthTransferStatement,
    pub signatures: ootle_rs::stealth::SignatureRequirements,
}

impl Spend {
    pub fn outputs_total(&self) -> u64 {
        self.outputs.iter().map(|o| o.amount.get()).sum()
    }

    /// Builds the statement for a given fee. The revealed fee slice is paid to the sealing account
    /// key, so the account key seals the whole transaction (the book checks that signer).
    pub async fn prepare(&self, session: &Session, fee: u64) -> Result<Prepared> {
        let change = self
            .input_value
            .checked_sub(self.outputs_total() + fee)
            .ok_or_else(|| anyhow!("inputs ({} µT) do not cover outputs and fee", self.input_value))?;
        let mut transfer = StealthTransfer::new(TARI_TOKEN, &session.provider);
        for (owner, input) in &self.inputs {
            transfer = transfer.spend_stealth_input(owner.clone(), input.clone());
        }
        transfer = transfer.to_revealed_output_for(fee, session.account_key());
        for out in &self.outputs {
            transfer = transfer.to_stealth_output(out.clone());
        }
        if change > 0 {
            if let Some(to) = &self.change_to {
                transfer = transfer.to_stealth_output(Output::new(
                    to.clone(),
                    TARI_TOKEN,
                    std::num::NonZeroU64::new(change).unwrap(),
                ));
            }
        }
        let (statement, signatures) = transfer.prepare().await.map_err(|e| anyhow!("stealth transfer: {e}"))?;
        let signatures = signatures.merge(ootle_rs::stealth::SignatureRequirements::account_key_seal());
        Ok(Prepared { statement, signatures })
    }
}

/// Fee instructions that pay `statement`'s revealed slice as the fee.
pub fn with_stealth_fee(builder: TransactionBuilder, statement: StealthTransferStatement) -> TransactionBuilder {
    builder.with_fee_instructions_builder(|b| {
        b.stealth_transfer(TARI_TOKEN, statement)
            .put_last_instruction_output_on_workspace("fee")
            .pay_fee_from_bucket("fee")
    })
}

pub fn utxo_input(commitment: &PedersenCommitmentBytes) -> UtxoAddress {
    UtxoAddress::new(TARI_TOKEN, (*commitment).into())
}

pub struct Submitted {
    pub tx_id: String,
    pub outcome: String,
    pub accepted: bool,
    pub fee: u64,
    pub receipt: Option<Value>,
}

pub struct Priced {
    pub unsigned: UnsignedTransaction,
    pub prepared: Prepared,
    pub fee: u64,
    pub dry_run: String,
}

/// Dry-runs `build(fee)` with a generous fee and rebuilds it with the floor the dry run reports.
/// `build` must be deterministic apart from the fee.
pub async fn price<F, Fut>(session: &Session, label: &str, build: F) -> Result<Priced>
where
    F: Fn(u64) -> Fut,
    Fut: std::future::Future<Output = Result<(UnsignedTransaction, Prepared)>>,
{
    let (dry, prepared) = build(DRY_RUN_FEE).await?;
    let dry_tx = TransactionRequest::default()
        .with_transaction(dry.with_dry_run(true))
        .build(&session.provider.wallet().stealth_authorizer(prepared.signatures))
        .await?;
    let result = session.provider.send_dry_run(dry_tx).await?;
    let fee = result.finalize.required_fees();
    let full = format!("{:?}", result.finalize.result);
    // Keep the verdict and, for a rejection, its reason; the full substate diff is noise here.
    let dry_run = match full.find("SubstateDiff") {
        Some(i) => full[..i].trim_end_matches('(').to_string(),
        None => full.chars().take(400).collect(),
    };
    println!("[{label}] dry run: {dry_run}, fee floor {fee} µT");
    let (unsigned, prepared) = build(fee).await?;
    Ok(Priced {
        unsigned,
        prepared,
        fee,
        dry_run,
    })
}

/// Signs and submits a priced transaction, waits for the result and logs it as evidence.
pub async fn send(session: &mut Session, label: &str, priced: &Priced) -> Result<Submitted> {
    let fee = priced.fee;
    let tx = TransactionRequest::default()
        .with_transaction(priced.unsigned.clone())
        .build(&session.provider.wallet().stealth_authorizer(priced.prepared.signatures.clone()))
        .await?;
    let pending = session.provider.send_transaction(tx).await?;
    let tx_id = pending.tx_id().to_string();
    println!("[{label}] submitted {tx_id}");
    let (outcome, accepted, receipt) = match pending.watch().await {
        Ok(o) => {
            let accepted = o.is_commit();
            let receipt = pending.get_receipt().await.ok().map(|r| serde_json::to_value(&r).unwrap_or(Value::Null));
            (format!("{o:?}"), accepted, receipt)
        },
        Err(e) => (format!("rejected: {e}"), false, None),
    };
    println!("[{label}] {outcome}");
    let submitted = Submitted {
        tx_id,
        outcome,
        accepted,
        fee,
        receipt,
    };
    log_evidence(label, &session.persona, &submitted, json!({ "dry_run": priced.dry_run }))?;
    Ok(submitted)
}

pub fn evidence_path() -> PathBuf {
    PathBuf::from("../evidence/esmeralda-run.jsonl")
}

pub fn log_evidence(label: &str, persona: &str, s: &Submitted, extra: Value) -> Result<()> {
    let line = json!({
        "step": label,
        "signer_persona": persona,
        "tx_id": s.tx_id,
        "accepted": s.accepted,
        "outcome": s.outcome,
        "fee_micro_tari": s.fee,
        "recorded_at": chrono_now(),
        "extra": extra,
    });
    let path = evidence_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")?;
    Ok(())
}

fn chrono_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    format!("unix:{secs}")
}

pub fn amount(v: u64) -> Amount {
    Amount::from(v)
}
