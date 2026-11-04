// SPDX-License-Identifier: MIT
//
// hexcrow: drives private milestone escrows on the Tari Ootle (Esmeralda testnet).

mod burn_proof;
mod chain;
mod escrow;
mod personas;

use std::{collections::BTreeMap, fs, num::NonZeroU64, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use chain::{Prepared, Session, Spend, utxo_input, with_stealth_fee};
use clap::{Parser, Subcommand};
use escrow::{EscrowTerms, Leaf, sha256};
use ootle_rs::{
    TransactionRequest,
    builtin_templates::{UnsignedTransactionBuilder as _, faucet::IFaucet},
    claim_burn::{ClaimBurn, MinotariBurnClaimProof},
    crypto::stealth::script_path_witness_with_data,
    stealth::Output,
    template_types::{
        ComponentAddress,
        Hash32,
        TemplateAddress,
        bytes::Bytes,
        constants::TARI_TOKEN,
        EncryptedData,
        crypto::PedersenCommitmentBytes,
        stealth::StealthInput,
    },
};
use ootle_rs::{provider::{Provider as _, WalletProvider as _}, transaction::TransactionSigner as _};
use personas::Personas;
use serde::{Deserialize, Serialize};
use tari_ootle_transaction::{Instruction, TransactionBuilder, args};
use tari_utilities::hex::Hex;

const TARI: u64 = 1_000_000;
/// The longest claim window past the deadline a client agrees to fund, in epochs.
const MAX_GRACE_EPOCHS: u64 = 48;
/// Revealed from the claimed funds to pay a burn claim's fee, in µT. It is spent in full, so the
/// claimed coin holds the burned amount less this.
const CLAIM_MAX_FEE: u64 = 50_000;

#[derive(Parser)]
#[command(about = "Private milestone escrows on the Tari Ootle")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show persona addresses and the current epoch.
    Status,
    /// Claim 1,000 tTARI from the network faucet into a persona (created if new), as a private coin.
    Faucet { who: String },
    /// Claim a burn made on the L1 into a persona (created if new), as one private coin. Without
    /// `--proof`, print the claim key the L1 burn must be addressed to.
    ClaimBurn {
        who: String,
        /// The burn proof file the L1 wallet writes once the burn is confirmed.
        #[arg(long)]
        proof: Option<PathBuf>,
    },
    /// Pay `amount_tari` from one persona's coins to another persona (created if new), as a private coin.
    Give { from: String, to: String, amount_tari: u64 },
    /// Publish the hexcrow_book template (paid by the publisher persona).
    Publish { wasm: PathBuf },
    /// Create the invoice book component.
    NewBook,
    /// Contractor `contractor-N` issues invoice N to the one-time key `client-N`. No approval lock yet:
    /// the client sets it when funding.
    Issue {
        #[arg(long)]
        doc: String,
        /// Epochs from now until the client can no longer approve.
        #[arg(long, default_value_t = 30)]
        deadline_in: u64,
        /// Epochs after the deadline during which the contractor can still claim an approved invoice.
        #[arg(long, default_value_t = 2)]
        grace: u64,
    },
    /// Client funds an invoice: picks the approval code, creates the escrow output and records its
    /// lock and condition root in the book, in one transaction.
    Fund {
        invoice: u64,
        #[arg(long)]
        amount_tari: u64,
    },
    /// Client approves the milestone by revealing the approval code.
    Approve { invoice: u64 },
    /// Contractor claims the escrow through the claim leaf and marks the invoice settled.
    Claim { invoice: u64 },
    /// Client takes the escrow back through the refund leaf after the deadline.
    Refund { invoice: u64 },
    /// The mediator rules on a funded invoice nobody approved, together with the party it rules
    /// for: the escrow pays that party and the book records the ruling's hash.
    Rule {
        invoice: u64,
        /// Who the ruling pays: `contractor` or `client`.
        #[arg(long = "for")]
        for_party: String,
        /// The mediator's written ruling, kept off chain; only its salted hash goes in the book.
        #[arg(long)]
        text: String,
    },
    /// Try to break an invoice. The network should refuse each attempt.
    Attack { invoice: u64, kind: String },
    /// Contractor checks, from the public indexer, that the escrow exists and matches the invoice
    /// terms before starting work.
    Verify {
        invoice: u64,
        /// The amount the contractor agreed to, which the escrow must hold.
        #[arg(long)]
        expect_tari: u64,
    },
    /// Print what each party can read about an invoice, as JSON for the web demo.
    Inspect { invoice: u64 },
    /// Trust the validator set handed over by the checkpoint ending EPOCH, for a first run after that epoch.
    AnchorTrust { epoch: u64 },
}

#[derive(Default, Serialize, Deserialize)]
struct State {
    template: Option<String>,
    book: Option<String>,
    /// Unspent private coins per persona: commitment hex -> value in µT.
    coins: BTreeMap<String, BTreeMap<String, u64>>,
    invoices: BTreeMap<u64, InvoiceLocal>,
}

#[derive(Serialize, Deserialize, Clone)]
struct InvoiceLocal {
    doc: String,
    salt: String,
    invoice_hash: String,
    /// Chosen by the client when funding; public once approved.
    approval_code: Option<String>,
    deadline_epoch: u64,
    claim_until: u64,
    contractor: String,
    client: String,
    escrow: String,
    escrow_commitment: Option<String>,
    escrow_value: Option<u64>,
    escrow_root: Option<String>,
    txs: BTreeMap<String, String>,
    #[serde(default)]
    mediator: String,
    /// The mediator's ruling, if there was one. Its text and salt stay off chain.
    #[serde(default)]
    ruling: Option<RulingLocal>,
}

#[derive(Serialize, Deserialize, Clone)]
struct RulingLocal {
    for_party: String,
    text: String,
    salt: String,
    hash: String,
}

fn state_path() -> PathBuf {
    PathBuf::from(".hexcrow/state.json")
}

fn load_state() -> Result<State> {
    let p = state_path();
    if !p.exists() {
        return Ok(State::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(p)?)?)
}

fn save_state(s: &State) -> Result<()> {
    fs::create_dir_all(".hexcrow")?;
    fs::write(state_path(), serde_json::to_string_pretty(s)?)?;
    Ok(())
}

fn commitment_from_hex(h: &str) -> Result<PedersenCommitmentBytes> {
    let bytes = Vec::<u8>::from_hex(h).map_err(|e| anyhow!("bad commitment hex: {e}"))?;
    PedersenCommitmentBytes::try_from(bytes.as_slice()).map_err(|_| anyhow!("commitment must be 32 bytes"))
}

fn random_hex(n: usize) -> Result<String> {
    let mut bytes = vec![0u8; n];
    getrandom::getrandom(&mut bytes).map_err(|e| anyhow!("OS random source: {e}"))?;
    Ok(bytes.to_hex())
}

/// The persona's largest coin, as a fee-paying input.
fn fee_coin(state: &State, persona: &str) -> Result<(String, u64)> {
    state
        .coins
        .get(persona)
        .and_then(|c| c.iter().max_by_key(|(_, v)| **v).map(|(k, v)| (k.clone(), *v)))
        .ok_or_else(|| anyhow!("{persona} has no private coin; run `hexcrow faucet {persona}` first"))
}

/// After an accepted spend: drop the spent coin, record change (always the last stealth output).
fn record_change(state: &mut State, persona: &str, spent: &str, prepared: &Prepared, change_value: Option<u64>) {
    let coins = state.coins.entry(persona.to_string()).or_default();
    coins.remove(spent);
    if let (Some(v), Some(out)) = (change_value, prepared.statement.stealth_outputs().last()) {
        coins.insert(out.commitment().to_hex(), v);
    }
}

fn book_address(state: &State) -> Result<ComponentAddress> {
    let b = state.book.as_ref().ok_or_else(|| anyhow!("no book yet; run `hexcrow new-book`"))?;
    ComponentAddress::from_str_hex(b)
}

trait FromStrHex: Sized {
    fn from_str_hex(s: &str) -> Result<Self>;
}

impl FromStrHex for ComponentAddress {
    fn from_str_hex(s: &str) -> Result<Self> {
        s.parse().map_err(|e| anyhow!("bad component address {s}: {e:?}"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut personas = Personas::load_or_create(&PathBuf::from(".hexcrow/personas.json"))?;
    let mut state = load_state()?;
    match cli.cmd {
        Cmd::Status => status(&personas, &state).await?,
        Cmd::Faucet { who } => {
            personas.ensure(&who)?;
            faucet(&personas, &mut state, &who).await?
        },
        Cmd::ClaimBurn { who, proof } => {
            personas.ensure(&who)?;
            claim_burn(&personas, &mut state, &who, proof).await?
        },
        Cmd::Give { from, to, amount_tari } => {
            fee_coin(&state, &from)?;
            personas.ensure(&to)?;
            give(&personas, &mut state, &from, &to, amount_tari).await?
        },
        Cmd::Publish { wasm } => publish(&personas, &mut state, wasm).await?,
        Cmd::NewBook => new_book(&personas, &mut state).await?,
        Cmd::Issue { doc, deadline_in, grace } => issue(&mut personas, &mut state, doc, deadline_in, grace).await?,
        Cmd::Fund { invoice, amount_tari } => fund(&personas, &mut state, invoice, amount_tari).await?,
        Cmd::Approve { invoice } => book_call(&personas, &mut state, invoice, "approve").await?,
        Cmd::Claim { invoice } => spend_escrow(&personas, &mut state, invoice, false).await?,
        Cmd::Refund { invoice } => spend_escrow(&personas, &mut state, invoice, true).await?,
        Cmd::Rule { invoice, for_party, text } => rule(&personas, &mut state, invoice, &for_party, text).await?,
        Cmd::Attack { invoice, kind } => attack(&personas, &mut state, invoice, &kind).await?,
        Cmd::Verify { invoice, expect_tari } => verify(&personas, &state, invoice, expect_tari).await?,
        Cmd::Inspect { invoice } => inspect(&personas, &state, invoice).await?,
        Cmd::AnchorTrust { epoch } => chain::anchor_trust(epoch).await?,
    }
    save_state(&state)?;
    Ok(())
}

async fn status(personas: &Personas, state: &State) -> Result<()> {
    let s = Session::connect(personas, "publisher", &[]).await?;
    println!("epoch: {}", s.epoch().await?);
    let per_invoice = state.invoices.keys().flat_map(|n| personas::invoice_names(*n));
    for name in personas::NAMES.iter().map(|n| n.to_string()).chain(per_invoice) {
        let name = name.as_str();
        let p = personas.provider(name)?;
        let coins: u64 = state.coins.get(name).map(|c| c.values().sum()).unwrap_or(0);
        println!("{name:<10} {}  coins: {} µT", p.address(), coins);
    }
    println!("template: {:?}\nbook: {:?}", state.template, state.book);
    Ok(())
}

async fn faucet(personas: &Personas, state: &mut State, who: &str) -> Result<()> {
    let mut s = Session::connect(personas, who, &[]).await?;
    let claim = 1_000 * TARI;
    let me = s.address.clone();
    let my_key = s.account_key();
    let max_epoch = s.max_epoch().await?;
    let session = &s;
    // The network's own faucet pays 1,000 tTARI into the persona's account; the same transaction
    // withdraws all of it into one private coin and pays the fee from a revealed slice.
    let build = |fee: u64| {
        let me = me.clone();
        async move {
            let transfer = ootle_rs::stealth::StealthTransfer::new(TARI_TOKEN, &session.provider)
                .spend_revealed_input(claim)
                .to_revealed_output_for(fee, my_key)
                .to_stealth_output(Output::new(me, TARI_TOKEN, NonZeroU64::new(claim - fee).unwrap()));
            let (statement, signatures) = transfer.prepare().await.map_err(|e| anyhow!("{e}"))?;
            let prepared = Prepared {
                statement: statement.clone(),
                signatures,
            };
            let tx = IFaucet::new(&session.provider, max_epoch)
                .take_faucet_funds()
                .into_stealth_transfer(statement)
                .and_pay_fee_from_revealed_output()
                .prepare()
                .await?;
            Ok((tx, prepared))
        }
    };
    let label = format!("faucet:{who}");
    let priced = chain::price(&s, &label, build).await?;
    let res = (chain::send(&mut s, &label, &priced).await?, priced.prepared);
    if res.0.accepted {
        let commitment = res.1.statement.stealth_outputs()[0].commitment().to_hex();
        state.coins.entry(who.to_string()).or_default().insert(commitment, claim - res.0.fee);
    }
    Ok(())
}

async fn claim_burn(personas: &Personas, state: &mut State, who: &str, proof: Option<PathBuf>) -> Result<()> {
    let mut s = Session::connect(personas, who, &[]).await?;
    let Some(proof) = proof else {
        println!("claim public key for {who}: {}", s.account_key().to_hex());
        return Ok(());
    };
    let text = fs::read_to_string(&proof).with_context(|| format!("reading {}", proof.display()))?;
    // The file `minotari_console_wallet` writes; `mined_in_epoch` is the epoch Ootle must pass before it can accept
    // the claim (a claim sent earlier is rejected for good).
    let file: tari_sidechain::CompleteClaimBurnProof = serde_json::from_str(&text).context("parsing the burn proof")?;
    let encrypted_data = EncryptedData::try_from(file.encrypted_data.clone())
        .map_err(|e| anyhow!("the burn proof's encrypted data: {e:?}"))?;
    let epoch = s.provider.get_epoch().await?.as_u64();
    if epoch <= file.mined_in_epoch {
        bail!("Ootle is at epoch {epoch}; wait until it passes epoch {} before claiming", file.mined_in_epoch);
    }
    let claim_proof: MinotariBurnClaimProof = burn_proof::claim_proof_from_l1(&file.claim_proof)?;
    let value = claim_proof
        .value
        .checked_sub(CLAIM_MAX_FEE)
        .filter(|v| *v > 0)
        .ok_or_else(|| anyhow!("the burn does not cover the claim fee of {CLAIM_MAX_FEE} µT"))?;
    let max_epoch = s.max_epoch().await?;
    // The L1 burn is addressed to a stealth key derived from the persona's account key, so the claim
    // is sealed by that derived key; the claimed funds become one stealth output to the persona.
    let (unsigned, sealer) = ClaimBurn::new(&s.provider, claim_proof, encrypted_data, max_epoch)
        .with_max_fee(CLAIM_MAX_FEE)
        .prepare()
        .await?;
    let commitment = unsigned
        .fee_instructions()
        .iter()
        .chain(unsigned.instructions())
        .find_map(|i| match i {
            Instruction::StealthTransfer { statement, .. } => statement.stealth_outputs().first().map(|o| o.commitment().to_hex()),
            _ => None,
        })
        .ok_or_else(|| anyhow!("the claim transaction has no stealth output"))?;
    let label = format!("claim-burn:{who}");
    let dry = s.provider.sign_and_send_dry_run_with(&sealer, unsigned.clone()).await?;
    let verdict = format!("{:?}", dry.finalize.result);
    let verdict = verdict.find("SubstateDiff").map_or(verdict.as_str(), |i| verdict[..i].trim_end_matches('(')).to_string();
    println!("[{label}] dry run: {verdict}");
    if verdict != "Accept" {
        bail!("[{label}] dry run did not accept the claim: {verdict}");
    }
    let tx = TransactionRequest::default().with_transaction(unsigned).build(&sealer).await?;
    let pending = s.provider.send_transaction(tx).await?;
    let tx_id = pending.tx_id().to_string();
    println!("[{label}] submitted {tx_id}");
    let (outcome, accepted) = match pending.watch().await {
        Ok(o) => (format!("{o:?}"), o.is_commit()),
        Err(e) => (format!("rejected: {e}"), false),
    };
    println!("[{label}] {outcome}");
    // The proof cannot be claimed twice: record the coin before anything else can fail.
    if accepted {
        println!("[{label}] coin {commitment}: {value} µT");
        state.coins.entry(who.to_string()).or_default().insert(commitment, value);
        save_state(state)?;
    }
    let submitted = chain::Submitted {
        tx_id,
        outcome,
        accepted,
        fee_committed: accepted,
        fee: CLAIM_MAX_FEE,
        receipt: None,
    };
    chain::log_evidence(&label, who, &submitted, serde_json::json!({ "dry_run": verdict }))?;
    chain::save_trust(&s.provider).await?;
    Ok(())
}

async fn give(personas: &Personas, state: &mut State, from: &str, to: &str, amount_tari: u64) -> Result<()> {
    let amount = amount_tari.checked_mul(TARI).and_then(NonZeroU64::new).ok_or_else(|| anyhow!("amount must be positive"))?;
    let to_address = personas.provider(to)?.address().clone();
    let output = Output::new(to_address, TARI_TOKEN, amount);
    let (res, prepared) = paid_call(personas, state, from, &format!("give:{from}:{to}"), vec![output], |tx| tx).await?;
    if res.accepted {
        // The given coin is the first stealth output; the payer's change comes last.
        let commitment = prepared.statement.stealth_outputs()[0].commitment().to_hex();
        state.coins.entry(to.to_string()).or_default().insert(commitment, amount.get());
    }
    Ok(())
}

/// A transaction whose main instructions come from `main`, with the fee paid privately from the
/// persona's largest coin and change returned to the persona.
async fn paid_call<M>(
    personas: &Personas,
    state: &mut State,
    persona: &str,
    label: &str,
    extra_outputs: Vec<Output>,
    main: M,
) -> Result<(chain::Submitted, Prepared)>
where
    M: Fn(TransactionBuilder) -> TransactionBuilder + Clone,
{
    let mut s = Session::connect(personas, persona, &[]).await?;
    let (coin_hex, coin_value) = fee_coin(state, persona)?;
    let coin = commitment_from_hex(&coin_hex)?;
    let spend = Spend {
        inputs: vec![(s.address.clone(), StealthInput::from(coin))],
        input_value: coin_value,
        outputs: extra_outputs,
        change_to: Some(s.address.clone()),
    };
    let max_epoch = s.max_epoch().await?;
    let session = &s;
    let spend_ref = &spend;
    let build = move |fee: u64| {
        let main = main.clone();
        async move {
            let prepared = spend_ref.prepare(session, fee).await?;
            let tx = TransactionBuilder::new(session.provider.network(), max_epoch)
                .with_auto_fill_inputs()
                .add_input(TARI_TOKEN)
                .add_input(utxo_input(&coin));
            let tx = with_stealth_fee(tx, prepared.statement.clone());
            let tx = main(tx).build_unsigned();
            Ok((tx, prepared))
        }
    };
    let priced = chain::price(&s, label, build).await?;
    let res = chain::send(&mut s, label, &priced).await?;
    let prepared = priced.prepared;
    if res.fee_committed {
        // The whole stealth transfer (outputs and change) lives in the fee intent, so it commits
        // even when the main instructions are rejected.
        let change = coin_value - spend.outputs_total() - res.fee;
        record_change(state, persona, &coin_hex, &prepared, (change > 0).then_some(change));
    }
    Ok((res, prepared))
}

async fn publish(personas: &Personas, state: &mut State, wasm: PathBuf) -> Result<()> {
    let binary = fs::read(&wasm).with_context(|| format!("reading {}", wasm.display()))?;
    println!("template binary: {} bytes", binary.len());
    let (res, _) = paid_call(personas, state, "publisher", "publish-template", vec![], move |tx| {
        tx.publish_template(binary.clone())
    })
    .await?;
    if let Some(r) = &res.receipt {
        let text = r.to_string();
        if let Some(i) = text.find("template_") {
            let addr: String = text[i..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            println!("template address: {addr}");
            state.template = Some(addr);
        }
    }
    Ok(())
}

async fn new_book(personas: &Personas, state: &mut State) -> Result<()> {
    let t = state.template.clone().ok_or_else(|| anyhow!("publish the template first"))?;
    let template: TemplateAddress = t
        .trim_start_matches("template_")
        .parse()
        .map_err(|e| anyhow!("bad template address: {e:?}"))?;
    let (res, _) = paid_call(personas, state, "publisher", "new-book", vec![], move |tx| {
        tx.call_function(template, "new", args![])
    })
    .await?;
    if let Some(r) = &res.receipt {
        let text = r.to_string();
        if let Some(i) = text.find("component_") {
            let addr: String = text[i..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            println!("book component: {addr}");
            state.book = Some(addr);
        }
    }
    Ok(())
}

fn persona_key(personas: &Personas, name: &str) -> Result<ootle_rs::template_types::crypto::RistrettoPublicKeyBytes> {
    Ok(*personas.provider(name)?.address().account_public_key())
}

fn approval_code(inv: &InvoiceLocal) -> Result<&str> {
    inv.approval_code.as_deref().ok_or_else(|| anyhow!("the client has not funded this invoice yet"))
}

fn terms(personas: &Personas, inv: &InvoiceLocal) -> Result<EscrowTerms> {
    Ok(EscrowTerms {
        approval_lock: sha256(approval_code(inv)?.as_bytes()),
        claim_until: inv.claim_until,
        contractor_key: persona_key(personas, &inv.contractor)?,
        client_key: persona_key(personas, &inv.client)?,
        mediator_key: persona_key(personas, &inv.mediator)?,
    })
}

async fn issue(personas: &mut Personas, state: &mut State, doc: String, deadline_in: u64, grace: u64) -> Result<()> {
    let book = book_address(state)?;
    let id = state.invoices.keys().max().copied().unwrap_or(0) + 1;
    let [client, contractor, escrow, mediator] = personas::invoice_names(id);
    for name in [&client, &contractor, &escrow, &mediator] {
        personas.ensure(name)?;
    }
    let s = Session::connect(personas, &contractor, &[]).await?;
    let deadline_epoch = s.epoch().await? + deadline_in;
    drop(s);
    let claim_until = deadline_epoch + grace;
    let salt = random_hex(16)?;
    let invoice_hash = sha256(format!("{salt}|{doc}").as_bytes());
    let client_key = persona_key(personas, &client)?;
    // The mediator both parties agreed on, with a key it uses for this invoice only.
    let mediator_key = persona_key(personas, &mediator)?;
    let (res, _) = paid_call(personas, state, &contractor, &format!("issue:{id}"), vec![], move |tx| {
        tx.call_method(book, "issue", args![invoice_hash, client_key, mediator_key, deadline_epoch, grace])
    })
    .await?;
    if !res.accepted {
        bail!("issue was not accepted: {}", res.outcome);
    }
    let mut txs = BTreeMap::new();
    txs.insert("issue".into(), res.tx_id);
    state.invoices.insert(id, InvoiceLocal {
        doc,
        salt,
        invoice_hash: invoice_hash.to_string(),
        approval_code: None,
        deadline_epoch,
        claim_until,
        contractor,
        client,
        escrow,
        escrow_commitment: None,
        escrow_value: None,
        escrow_root: None,
        txs,
        mediator,
        ruling: None,
    });
    println!("invoice {id} issued: approve before epoch {deadline_epoch}, claim before epoch {claim_until}");
    Ok(())
}

async fn fund(personas: &Personas, state: &mut State, invoice: u64, amount_tari: u64) -> Result<()> {
    let book = book_address(state)?;
    let mut inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    // The client funds the terms the book holds, not the ones it was told: its own key, an open
    // invoice, and a claim window it accepts.
    let entry = read_book_entry(state, invoice).await?;
    if entry.client_key != persona_key(personas, &inv.client)?.to_string() || entry.status != 0 {
        bail!("invoice {invoice} in the book is not an open invoice to this client");
    }
    if entry.mediator_key != persona_key(personas, &inv.mediator)?.to_string() {
        bail!("invoice {invoice} in the book names a mediator this client did not agree to");
    }
    if entry.claim_until.saturating_sub(entry.deadline_epoch) > MAX_GRACE_EPOCHS {
        bail!("the claim window of invoice {invoice} runs {} epochs past the deadline; refusing to fund", entry.claim_until - entry.deadline_epoch);
    }
    inv.deadline_epoch = entry.deadline_epoch;
    inv.claim_until = entry.claim_until;
    // The approval code is the client's: generated here, on the funding side, and shared with no
    // one until approval. Only its hash goes on chain, in the book and in the escrow's claim leaf.
    // It is saved before the transaction is sent, so an error after acceptance cannot lose it.
    let code = format!("hexcrow-approve-{}", random_hex(16)?);
    inv.approval_code = Some(code.clone());
    state.invoices.insert(invoice, inv.clone());
    save_state(state)?;
    let t = terms(personas, &inv)?;
    let lock = t.approval_lock;
    let root: Hash32 = t.root()?;
    let escrow_addr = personas.provider(&inv.escrow)?.address().clone();
    let value = amount_tari * TARI;
    let escrow_out = Output::new(escrow_addr, TARI_TOKEN, NonZeroU64::new(value).unwrap())
        .with_spend_conditions(t.conditions())
        .with_memo_message(format!("Hexcrow invoice {invoice}"));
    let (res, prepared) = paid_call(personas, state, &inv.client, &format!("fund:{invoice}"), vec![escrow_out], move |tx| {
        tx.call_method(book, "fund", args![invoice, lock, root])
    })
    .await?;
    if !res.accepted {
        bail!("fund was not accepted: {}", res.outcome);
    }
    let escrow_commitment = prepared.statement.stealth_outputs()[0].commitment().to_hex();
    let inv = state.invoices.get_mut(&invoice).unwrap();
    inv.approval_code = Some(code);
    inv.escrow_commitment = Some(escrow_commitment.clone());
    inv.escrow_value = Some(value);
    inv.escrow_root = Some(root.to_string());
    inv.txs.insert("fund".into(), res.tx_id);
    println!("escrow output {escrow_commitment} holds the milestone; root {root}");
    Ok(())
}

async fn book_call(personas: &Personas, state: &mut State, invoice: u64, method: &str) -> Result<()> {
    let book = book_address(state)?;
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let code = approval_code(&inv)?.as_bytes().to_vec();
    let m = method.to_string();
    let (res, _) = paid_call(personas, state, &inv.client, &format!("{method}:{invoice}"), vec![], move |tx| {
        if m == "approve" {
            tx.call_method(book, "approve", args![invoice, code.clone()])
        } else {
            tx.call_method(book, m.as_str(), args![invoice])
        }
    })
    .await?;
    state.invoices.get_mut(&invoice).unwrap().txs.insert(method.into(), res.tx_id);
    Ok(())
}

/// Spends the escrow output through the claim or the refund leaf. The fee comes out of the escrow
/// itself, so neither party has to touch another coin.
async fn spend_escrow(personas: &Personas, state: &mut State, invoice: u64, refund: bool) -> Result<()> {
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let (party, leaf, label, method) = if refund {
        (inv.client.clone(), Leaf::Refund, format!("refund:{invoice}"), "refund")
    } else {
        (inv.contractor.clone(), Leaf::Claim, format!("claim:{invoice}"), "settle")
    };
    let witness = if refund { Vec::new() } else { approval_code(&inv)?.as_bytes().to_vec() };
    run_escrow_spend(personas, state, invoice, &[party.as_str()], &party, &label, BookCall::plain(method), leaf, witness)
        .await
        .map(|_| ())
}

/// The mediator rules on an invoice for one party. Both sign the escrow spend through that party's
/// ruling leaf, the escrow pays that party, and the book records the salted hash of the ruling.
async fn rule(personas: &Personas, state: &mut State, invoice: u64, for_party: &str, text: String) -> Result<()> {
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let (party, leaf, method) = match for_party {
        "contractor" => (inv.contractor.clone(), Leaf::RulingForContractor, "rule_for_contractor"),
        "client" => (inv.client.clone(), Leaf::RulingForClient, "rule_for_client"),
        other => bail!("a ruling pays the contractor or the client, not {other}"),
    };
    let salt = random_hex(16)?;
    let hash = sha256(format!("{salt}|{text}").as_bytes());
    // Saved before sending, so an error after acceptance cannot lose the text the hash commits to.
    state.invoices.get_mut(&invoice).unwrap().ruling = Some(RulingLocal {
        for_party: for_party.to_string(),
        text,
        salt,
        hash: hash.to_string(),
    });
    save_state(state)?;
    let call = BookCall {
        method: method.to_string(),
        ruling_hash: Some(hash),
    };
    let label = format!("rule:{invoice}:{for_party}");
    let cosigners = [inv.mediator.as_str(), party.as_str()];
    let accepted = run_escrow_spend(personas, state, invoice, &cosigners, &party, &label, call, leaf, Vec::new()).await?;
    if !accepted {
        state.invoices.get_mut(&invoice).unwrap().ruling = None;
    }
    Ok(())
}

/// What an escrow spend asks the book to record in the same transaction: `method(invoice)`, or
/// `method(invoice, ruling_hash)` for a ruling.
struct BookCall {
    method: String,
    ruling_hash: Option<Hash32>,
}

impl BookCall {
    fn plain(method: &str) -> Self {
        Self {
            method: method.to_string(),
            ruling_hash: None,
        }
    }
}

/// Spends the escrow output through `leaf`, paying what is left after the fee to `payee`. The
/// escrow key seals (it is the only key that can decrypt the escrow output) and each of
/// `cosigners` signs; the leaf's access rules and the book both look for those signatures.
/// Returns whether the network accepted it.
#[allow(clippy::too_many_arguments)]
async fn run_escrow_spend(
    personas: &Personas,
    state: &mut State,
    invoice: u64,
    cosigners: &[&str],
    payee: &str,
    label: &str,
    call: BookCall,
    leaf: Leaf,
    witness_data: Vec<u8>,
) -> Result<bool> {
    let book = book_address(state)?;
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let t = terms(personas, &inv)?;
    let conditions = t.conditions();
    let (witness, _root) = script_path_witness_with_data(&conditions, &t.leaf(leaf), Bytes::from_vec(witness_data))
        .map_err(|e| anyhow!("witness: {e}"))?;
    let commitment = commitment_from_hex(inv.escrow_commitment.as_ref().ok_or_else(|| anyhow!("invoice not funded"))?)?;
    let value = inv.escrow_value.unwrap_or_default();
    let mut s = Session::connect(personas, &inv.escrow, cosigners).await?;
    let escrow_addr = s.address.clone();
    let payee_addr = personas.provider(payee)?.address().clone();
    let spend = Spend {
        inputs: vec![(escrow_addr, StealthInput::with_witness(commitment, witness))],
        input_value: value,
        outputs: vec![],
        change_to: Some(payee_addr),
    };
    let max_epoch = s.max_epoch().await?;
    let session = &s;
    let spend_ref = &spend;
    let BookCall { method, ruling_hash } = call;
    let build = move |fee: u64| {
        let method = method.clone();
        async move {
            let prepared = spend_ref.prepare(session, fee).await?;
            let tx = TransactionBuilder::new(session.provider.network(), max_epoch)
                .with_auto_fill_inputs()
                .add_input(TARI_TOKEN)
                .add_input(utxo_input(&commitment));
            let tx = with_stealth_fee(tx, prepared.statement.clone());
            let tx = match ruling_hash {
                Some(hash) => tx.call_method(book, method.as_str(), args![invoice, hash]),
                None => tx.call_method(book, method.as_str(), args![invoice]),
            };
            Ok((tx.build_unsigned(), prepared))
        }
    };
    let priced = chain::price(&s, label, build).await?;
    let res = chain::send(&mut s, label, &priced).await?;
    let prepared = priced.prepared;
    if res.accepted {
        let out = prepared.statement.stealth_outputs().last().map(|o| o.commitment().to_hex());
        if let Some(c) = out {
            state.coins.entry(payee.to_string()).or_default().insert(c, value - res.fee);
        }
    }
    state.invoices.get_mut(&invoice).unwrap().txs.insert(label.to_string(), res.tx_id);
    Ok(res.accepted)
}

async fn attack(personas: &Personas, state: &mut State, invoice: u64, kind: &str) -> Result<()> {
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let code = approval_code(&inv).map(|c| c.as_bytes().to_vec()).unwrap_or_default();
    let label = format!("attack:{kind}:{invoice}");
    let ruling = || BookCall {
        method: "rule_for_contractor".into(),
        ruling_hash: Some(sha256(b"a ruling nobody wrote")),
    };
    let (contractor, client, mediator) = (inv.contractor.as_str(), inv.client.as_str(), inv.mediator.as_str());
    let accepted = match kind {
        // A stranger who learned the approval code tries to take the money.
        "stranger-claim" => {
            let when = if inv.txs.contains_key("approve") { "after-approval" } else { "before-approval" };
            let l = format!("{label}:{when}");
            run_escrow_spend(personas, state, invoice, &["stranger"], "stranger", &l, BookCall::plain("settle"), Leaf::Claim, code).await
        },
        // The contractor tries to claim without the code.
        "claim-without-code" => {
            let guess = b"i-did-the-work".to_vec();
            run_escrow_spend(personas, state, invoice, &[contractor], contractor, &label, BookCall::plain("settle"), Leaf::Claim, guess).await
        },
        // The client tries to take the money back before the claim window closes, before or after
        // approving.
        "early-refund" | "refund-after-approval" => {
            run_escrow_spend(personas, state, invoice, &[client], client, &label, BookCall::plain("refund"), Leaf::Refund, Vec::new()).await
        },
        // The mediator tries to pay itself through the ruling leaf, without the contractor.
        "mediator-alone" => {
            run_escrow_spend(personas, state, invoice, &[mediator], mediator, &label, ruling(), Leaf::RulingForContractor, Vec::new()).await
        },
        // The contractor tries the ruling leaf without the mediator.
        "ruling-without-mediator" => {
            run_escrow_spend(personas, state, invoice, &[contractor], contractor, &label, ruling(), Leaf::RulingForContractor, Vec::new()).await
        },
        // The mediator and the contractor rule after the claim window has closed.
        "late-ruling" => {
            let both = [mediator, contractor];
            run_escrow_spend(personas, state, invoice, &both, contractor, &label, ruling(), Leaf::RulingForContractor, Vec::new()).await
        },
        // The contractor tries to fund its own invoice with an approval code it chose itself.
        "contractor-sets-lock" => {
            let book = book_address(state)?;
            let own_lock = sha256(b"a code the contractor already knows");
            paid_call(personas, state, contractor, &label, vec![], move |tx| {
                tx.call_method(book, "fund", args![invoice, own_lock, own_lock])
            })
            .await
            .map(|(res, _)| res.accepted)
        },
        // The contractor tries to approve their own invoice in the book.
        "self-approve" => {
            let book = book_address(state)?;
            paid_call(personas, state, contractor, &label, vec![], move |tx| {
                tx.call_method(book, "approve", args![invoice, code.clone()])
            })
            .await
            .map(|(res, _)| res.accepted)
        },
        other => bail!("unknown attack {other}"),
    }?;
    if accepted {
        println!("WARNING: the network accepted attack {kind} on invoice {invoice}");
    }
    Ok(())
}

/// The book entry of one invoice, as the public indexer serves it. Field order follows `Invoice`
/// in the template: hash, contractor, client, deadline, claim_until, lock, root, code, status,
/// mediator, ruling hash.
struct BookEntry {
    contractor_key: String,
    client_key: String,
    deadline_epoch: u64,
    claim_until: u64,
    approval_lock: Option<String>,
    escrow_root: Option<String>,
    status: u64,
    mediator_key: String,
}

fn cbor_hex(v: &serde_json::Value) -> Option<String> {
    v.get("hex").and_then(|h| h.as_str()).map(str::to_string)
}

/// Turns a CBOR value into the JSON shape the indexer served before 0.50: bytes as `{"hex": ..}`,
/// maps as `[key, value]` pairs, tags dropped.
fn cbor_to_json(v: ciborium::Value) -> serde_json::Value {
    use ciborium::Value as C;
    use serde_json::Value as J;
    match v {
        C::Integer(i) => {
            let i = i128::from(i);
            u64::try_from(i).map(J::from).or_else(|_| i64::try_from(i).map(J::from)).unwrap_or(J::Null)
        },
        C::Bytes(b) => serde_json::json!({ "hex": hex::encode(b) }),
        C::Float(f) => J::from(f),
        C::Text(s) => J::String(s),
        C::Bool(b) => J::Bool(b),
        C::Null => J::Null,
        C::Tag(_, inner) => cbor_to_json(*inner),
        C::Array(a) => J::Array(a.into_iter().map(cbor_to_json).collect()),
        C::Map(m) => J::Array(m.into_iter().map(|(k, v)| J::Array(vec![cbor_to_json(k), cbor_to_json(v)])).collect()),
        _ => J::Null,
    }
}

/// Reads one invoice from the book from the public indexer, the read the web demo makes. Since
/// indexer 0.50 the substate comes as base64 CBOR: `[0 (Component), [[header, [[entries, next]]]]]`.
/// Unlike the escrow output in `verify`, this read is not checked against a proof.
async fn read_book_entry(state: &State, invoice: u64) -> Result<BookEntry> {
    use base64::Engine as _;
    let book = state.book.as_ref().ok_or_else(|| anyhow!("no book yet"))?;
    let indexer = ootle_rs::default_indexer_url(personas::NETWORK).to_string();
    let url = format!("{}/substates/{book}", indexer.trim_end_matches('/'));
    let json: serde_json::Value = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "hexcrow-cli")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let encoded = json
        .pointer("/state/up/substate")
        .and_then(|s| s.as_str())
        .ok_or_else(|| anyhow!("the indexer did not return the book as an up substate"))?;
    let raw = base64::engine::general_purpose::STANDARD.decode(encoded).context("decoding the book's base64")?;
    let value: ciborium::Value = ciborium::from_reader(raw.as_slice()).context("decoding the book's CBOR")?;
    let substate = cbor_to_json(value);
    let entries = substate
        .pointer("/1/0/1/0/0")
        .and_then(|e| e.as_array())
        .ok_or_else(|| anyhow!("unexpected book state shape"))?;
    let f = entries
        .iter()
        .find(|e| e[0].as_u64() == Some(invoice))
        .map(|e| &e[1])
        .ok_or_else(|| anyhow!("invoice {invoice} is not in the book"))?;
    Ok(BookEntry {
        contractor_key: cbor_hex(&f[1]).unwrap_or_default(),
        client_key: cbor_hex(&f[2]).unwrap_or_default(),
        deadline_epoch: f[3].as_u64().unwrap_or_default(),
        claim_until: f[4].as_u64().unwrap_or_default(),
        approval_lock: cbor_hex(&f[5]),
        escrow_root: cbor_hex(&f[6]),
        status: f[8][0].as_u64().unwrap_or(u64::MAX),
        mediator_key: cbor_hex(&f[9]).unwrap_or_default(),
    })
}

/// The escrow output as a substate, read through the provider, which checks the indexer's proof
/// against the validator set: its condition root, whether it has a key path, and its value,
/// decrypted with the invoice's escrow view key.
struct EscrowSeen {
    condition_root: Option<String>,
    has_key_path: bool,
    value: Option<u64>,
}

async fn read_escrow(personas: &Personas, inv: &InvoiceLocal) -> Result<Option<EscrowSeen>> {
    use ootle_rs::key_provider::DiffieHellmanKdfKeyProvider as _;
    use tari_utilities::ByteArray as _;

    let commitment = commitment_from_hex(inv.escrow_commitment.as_ref().ok_or_else(|| anyhow!("no escrow commitment"))?)?;
    let s = Session::connect(personas, &inv.escrow, &[]).await?;
    let Some(substate) = s.provider.get_substate(utxo_input(&commitment)).await? else {
        return Ok(None);
    };
    let Some(out) = substate.substate_value().as_utxo().and_then(|u| u.output.clone()) else {
        return Ok(None);
    };
    let nonce = tari_crypto::ristretto::RistrettoPublicKey::from_canonical_bytes(out.output.public_nonce.as_ref())
        .map_err(|e| anyhow!("escrow nonce: {e}"))?;
    let key = personas.provider(&inv.escrow)?.create_kdf_dh_encrypted_data_key(&nonce).await?;
    let value = ootle_rs::crypto::encrypted_data::unblind_output(&commitment, &out.output.encrypted_data, &key, true)
        .ok()
        .map(|d| d.mask_and_value.value);
    Ok(Some(EscrowSeen {
        condition_root: out.auth.condition_root().map(|r| r.to_string()),
        has_key_path: out.auth.spend_key().is_some(),
        value,
    }))
}

/// What the contractor checks before starting work, with public data, its own key and the
/// invoice's escrow view key: the book lists it as contractor, the invoice is funded and can still
/// be approved, the escrow output exists unspent, is locked by exactly the condition root the
/// book's terms produce, and holds the amount the contractor expects. Passing means the money is
/// locked as agreed; being paid still needs the client's approval.
async fn verify(personas: &Personas, state: &State, invoice: u64, expect_tari: u64) -> Result<()> {
    let inv = state.invoices.get(&invoice).ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let entry = read_book_entry(state, invoice).await?;
    let epoch = Session::connect(personas, &inv.contractor, &[]).await?.epoch().await?;

    let me = persona_key(personas, &inv.contractor)?.to_string();
    let mut checks = Vec::new();
    checks.push(("the book names my key as contractor".to_string(), entry.contractor_key == me));
    checks.push((
        "the book names the mediator we agreed on".to_string(),
        entry.mediator_key == persona_key(personas, &inv.mediator)?.to_string(),
    ));
    checks.push(("the invoice is funded".to_string(), entry.status == 1));
    checks.push((
        format!("the client can still approve: epoch {epoch}, deadline {}", entry.deadline_epoch),
        epoch < entry.deadline_epoch,
    ));
    let rebuilt = match &entry.approval_lock {
        Some(lock) => EscrowTerms {
            approval_lock: Hash32::from_hex(lock).map_err(|e| anyhow!("lock: {e:?}"))?,
            claim_until: entry.claim_until,
            contractor_key: entry.contractor_key.parse().map_err(|e| anyhow!("contractor key: {e:?}"))?,
            client_key: entry.client_key.parse().map_err(|e| anyhow!("client key: {e:?}"))?,
            mediator_key: entry.mediator_key.parse().map_err(|e| anyhow!("mediator key: {e:?}"))?,
        }
        .root()?
        .to_string(),
        None => String::new(),
    };
    checks.push(("the book's terms rebuild the book's condition root".to_string(), entry.escrow_root.as_deref() == Some(rebuilt.as_str())));

    let seen = read_escrow(personas, inv).await?;
    checks.push(("the escrow output exists and is unspent, by a verified proof".to_string(), seen.is_some()));
    checks.push((
        "the escrow output is locked by exactly that root, with no key path".to_string(),
        seen.as_ref().is_some_and(|e| !e.has_key_path && e.condition_root.as_deref() == Some(rebuilt.as_str())),
    ));
    let held = seen.as_ref().and_then(|e| e.value);
    checks.push((
        format!("the escrow holds the expected {expect_tari} tTARI, read with the escrow view key"),
        held == Some(expect_tari * TARI),
    ));

    let locked = checks.iter().all(|(_, ok)| *ok);
    println!("invoice {invoice}: approve before epoch {}, claim before epoch {}", entry.deadline_epoch, entry.claim_until);
    for (what, ok) in &checks {
        println!("  [{}] {what}", if *ok { "ok" } else { "FAIL" });
    }
    println!(
        "{}",
        if locked {
            "funds locked as agreed; payment still needs the client's approval"
        } else {
            "do not start work"
        }
    );
    chain::log_check(&format!("verify:{invoice}"), &inv.contractor, serde_json::json!({
        "funds_locked_as_agreed": locked,
        "checks": checks.iter().map(|(w, ok)| serde_json::json!({ "check": w, "ok": ok })).collect::<Vec<_>>(),
        "escrow_value_micro_tari": held,
        "condition_root": rebuilt,
    }))?;
    Ok(())
}

async fn inspect(personas: &Personas, state: &State, invoice: u64) -> Result<()> {
    let inv = state.invoices.get(&invoice).ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let out = serde_json::json!({
        "invoice": invoice,
        "book": state.book,
        "template": state.template,
        "public": {
            "invoice_hash": inv.invoice_hash,
            "deadline_epoch": inv.deadline_epoch,
            "claim_until": inv.claim_until,
            "escrow_commitment": inv.escrow_commitment,
            "escrow_root": inv.escrow_root,
            "txs": inv.txs,
        },
        "parties": {
            "contractor_key": persona_key(personas, &inv.contractor)?.to_string(),
            "client_key": persona_key(personas, &inv.client)?.to_string(),
            "mediator_key": persona_key(personas, &inv.mediator)?.to_string(),
        },
        "mediator_ruling": inv.ruling.as_ref().map(|r| serde_json::json!({
            "for": r.for_party,
            "text": r.text,
            "salt": r.salt,
            "ruling_hash": r.hash,
            "check": "sha256(salt|text) == ruling_hash in the book",
        })),
        "escrow_view_key_holders_see": {
            "amount_micro_tari": inv.escrow_value,
            "memo": format!("Hexcrow invoice {invoice}"),
        },
        "accountant_opening": {
            "doc": inv.doc,
            "salt": inv.salt,
            "check": "sha256(salt|doc) == public.invoice_hash",
        },
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
