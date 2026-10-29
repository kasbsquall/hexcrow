// SPDX-License-Identifier: MIT
//
// hexcrow: drives private milestone escrows on the Tari Ootle (Esmeralda testnet).

mod chain;
mod escrow;
mod personas;

use std::{collections::BTreeMap, fs, num::NonZeroU64, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use chain::{Prepared, Session, Spend, utxo_input, with_stealth_fee};
use clap::{Parser, Subcommand};
use escrow::{EscrowTerms, sha256};
use ootle_rs::{
    crypto::stealth::script_path_witness_with_data,
    stealth::Output,
    template_types::{
        ComponentAddress,
        Hash32,
        TemplateAddress,
        bytes::Bytes,
        constants::TARI_TOKEN,
        crypto::PedersenCommitmentBytes,
        stealth::StealthInput,
    },
};
use ootle_rs::{provider::{Provider as _, WalletProvider as _}, transaction::TransactionSigner as _};
use personas::Personas;
use serde::{Deserialize, Serialize};
use tari_ootle_transaction::{TransactionBuilder, args};
use tari_utilities::hex::Hex;

const TARI: u64 = 1_000_000;
const CARAVEL_FAUCET: &str = "component_568f84a0cc7ccfe49116ee86d072f02b99e246a4750e680a8cbfcd2b7862f37b";
const CARAVEL_VAULT: &str = "vault_56c3c95ef08e843656e755100345d8fa0bf0a58853b40cd9d2894ab48a112ec6";
const CARAVEL_RECEIPTS: &str = "resource_5694c70e0eaa25809e593c2f4ee85862b3fa71ea45d9ad01bf18f8accf35d621";

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
    /// Claim 1,000 tTARI from the public Caravel faucet into a persona, as a private coin.
    Faucet { who: String },
    /// Publish the hexcrow_book template (paid by the publisher persona).
    Publish { wasm: PathBuf },
    /// Create the invoice book component.
    NewBook,
    /// Contractor issues an invoice; the client's approval code is generated here and kept local.
    Issue {
        #[arg(long)]
        doc: String,
        #[arg(long, default_value_t = 30)]
        deadline_in: u64,
    },
    /// Client funds an invoice: creates the escrow output and records it in the book, in one transaction.
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
    /// Try to break an invoice. The network should refuse each attempt.
    Attack { invoice: u64, kind: String },
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
    approval_code: String,
    deadline_epoch: u64,
    contractor: String,
    client: String,
    escrow_commitment: Option<String>,
    escrow_value: Option<u64>,
    escrow_root: Option<String>,
    txs: BTreeMap<String, String>,
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

fn random_hex(n: usize) -> String {
    let bytes: Vec<u8> = (0..n).map(|_| rand_byte()).collect();
    bytes.to_hex()
}

fn rand_byte() -> u8 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    (h.finish() & 0xff) as u8
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
    let personas = Personas::load_or_create(&PathBuf::from(".hexcrow/personas.json"))?;
    let mut state = load_state()?;
    match cli.cmd {
        Cmd::Status => status(&personas, &state).await?,
        Cmd::Faucet { who } => faucet(&personas, &mut state, &who).await?,
        Cmd::Publish { wasm } => publish(&personas, &mut state, wasm).await?,
        Cmd::NewBook => new_book(&personas, &mut state).await?,
        Cmd::Issue { doc, deadline_in } => issue(&personas, &mut state, doc, deadline_in).await?,
        Cmd::Fund { invoice, amount_tari } => fund(&personas, &mut state, invoice, amount_tari).await?,
        Cmd::Approve { invoice } => book_call(&personas, &mut state, invoice, "client", "approve").await?,
        Cmd::Claim { invoice } => spend_escrow(&personas, &mut state, invoice, false).await?,
        Cmd::Refund { invoice } => spend_escrow(&personas, &mut state, invoice, true).await?,
        Cmd::Attack { invoice, kind } => attack(&personas, &mut state, invoice, &kind).await?,
        Cmd::Inspect { invoice } => inspect(&personas, &state, invoice).await?,
        Cmd::AnchorTrust { epoch } => chain::anchor_trust(epoch).await?,
    }
    save_state(&state)?;
    Ok(())
}

async fn status(personas: &Personas, state: &State) -> Result<()> {
    let s = Session::connect(personas, "publisher", &[]).await?;
    println!("epoch: {}", s.epoch().await?);
    for name in personas::NAMES {
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
    let faucet: ComponentAddress = CARAVEL_FAUCET.parse().map_err(|e| anyhow!("{e:?}"))?;
    let session = &s;
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
            let tx = TransactionBuilder::new(session.provider.network(), max_epoch)
                .with_auto_fill_inputs()
                .add_input(faucet)
                .add_input(CARAVEL_VAULT.parse::<ootle_rs::template_types::VaultId>().map_err(|e| anyhow!("{e:?}"))?)
                .add_input(CARAVEL_RECEIPTS.parse::<ootle_rs::template_types::ResourceAddress>().map_err(|e| anyhow!("{e:?}"))?)
                .add_input(TARI_TOKEN)
                .with_fee_instructions_builder(|b| {
                    b.call_method(faucet, "claim", args![my_key])
                        .put_last_instruction_output_on_workspace("payout")
                        .stealth_transfer_with_input_bucket(TARI_TOKEN, statement, "payout")
                        .put_last_instruction_output_on_workspace("fee")
                        .pay_fee_from_bucket("fee")
                })
                .build_unsigned();
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

fn terms(personas: &Personas, inv: &InvoiceLocal) -> Result<EscrowTerms> {
    Ok(EscrowTerms {
        approval_lock: sha256(inv.approval_code.as_bytes()),
        deadline_epoch: inv.deadline_epoch,
        contractor_key: persona_key(personas, &inv.contractor)?,
        client_key: persona_key(personas, &inv.client)?,
    })
}

async fn issue(personas: &Personas, state: &mut State, doc: String, deadline_in: u64) -> Result<()> {
    let book = book_address(state)?;
    let s = Session::connect(personas, "contractor", &[]).await?;
    let deadline_epoch = s.epoch().await? + deadline_in;
    drop(s);
    let salt = random_hex(16);
    let invoice_hash = sha256(format!("{salt}|{doc}").as_bytes());
    // The approval code belongs to the client; it only shares its hash with the contractor.
    let approval_code = format!("hexcrow-approve-{}", random_hex(16));
    let approval_lock = sha256(approval_code.as_bytes());
    let client_key = persona_key(personas, "client")?;
    let id = state.invoices.keys().max().copied().unwrap_or(0) + 1;
    let (res, _) = paid_call(personas, state, "contractor", &format!("issue:{id}"), vec![], move |tx| {
        tx.call_method(book, "issue", args![invoice_hash, client_key, approval_lock, deadline_epoch])
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
        approval_code,
        deadline_epoch,
        contractor: "contractor".into(),
        client: "client".into(),
        escrow_commitment: None,
        escrow_value: None,
        escrow_root: None,
        txs,
    });
    println!("invoice {id} issued, deadline epoch {deadline_epoch}");
    Ok(())
}

async fn fund(personas: &Personas, state: &mut State, invoice: u64, amount_tari: u64) -> Result<()> {
    let book = book_address(state)?;
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let t = terms(personas, &inv)?;
    let root: Hash32 = t.root()?;
    let escrow_addr = personas.provider("escrow")?.address().clone();
    let value = amount_tari * TARI;
    let escrow_out = Output::new(escrow_addr, TARI_TOKEN, NonZeroU64::new(value).unwrap())
        .with_spend_conditions(t.conditions())
        .with_memo_message(format!("Hexcrow invoice {invoice}"));
    let (res, prepared) = paid_call(personas, state, "client", &format!("fund:{invoice}"), vec![escrow_out], move |tx| {
        tx.call_method(book, "fund", args![invoice, root])
    })
    .await?;
    if !res.accepted {
        bail!("fund was not accepted: {}", res.outcome);
    }
    let escrow_commitment = prepared.statement.stealth_outputs()[0].commitment().to_hex();
    let inv = state.invoices.get_mut(&invoice).unwrap();
    inv.escrow_commitment = Some(escrow_commitment.clone());
    inv.escrow_value = Some(value);
    inv.escrow_root = Some(root.to_string());
    inv.txs.insert("fund".into(), res.tx_id);
    println!("escrow output {escrow_commitment} holds the milestone; root {root}");
    Ok(())
}

async fn book_call(personas: &Personas, state: &mut State, invoice: u64, persona: &str, method: &str) -> Result<()> {
    let book = book_address(state)?;
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let code = inv.approval_code.as_bytes().to_vec();
    let m = method.to_string();
    let (res, _) = paid_call(personas, state, persona, &format!("{method}:{invoice}"), vec![], move |tx| {
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

/// Spends the escrow output through one of its leaves. The fee comes out of the escrow itself, so
/// neither party has to touch another coin.
async fn spend_escrow(personas: &Personas, state: &mut State, invoice: u64, refund: bool) -> Result<()> {
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let persona = if refund { inv.client.clone() } else { inv.contractor.clone() };
    let label = format!("{}:{invoice}", if refund { "refund" } else { "claim" });
    let book_method = if refund { "refund" } else { "settle" };
    let witness_code = if refund { Vec::new() } else { inv.approval_code.as_bytes().to_vec() };
    run_escrow_spend(personas, state, invoice, &persona, &label, book_method, refund, witness_code).await
}

#[allow(clippy::too_many_arguments)]
async fn run_escrow_spend(
    personas: &Personas,
    state: &mut State,
    invoice: u64,
    persona: &str,
    label: &str,
    book_method: &str,
    refund_leaf: bool,
    witness_data: Vec<u8>,
) -> Result<()> {
    let book = book_address(state)?;
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let t = terms(personas, &inv)?;
    let conditions = t.conditions();
    let leaf = if refund_leaf { t.refund_leaf() } else { t.claim_leaf() };
    let (witness, _root) = script_path_witness_with_data(&conditions, &leaf, Bytes::from_vec(witness_data))
        .map_err(|e| anyhow!("witness: {e}"))?;
    let commitment = commitment_from_hex(inv.escrow_commitment.as_ref().ok_or_else(|| anyhow!("invoice not funded"))?)?;
    let value = inv.escrow_value.unwrap_or_default();
    // The escrow key seals (it is the only key that can decrypt the escrow output) and the acting
    // party co-signs; the leaf's AccessRule and the book both look for the party's signature.
    let mut s = Session::connect(personas, "escrow", &[persona]).await?;
    let escrow_addr = s.address.clone();
    let party_addr = personas.provider(persona)?.address().clone();
    let spend = Spend {
        inputs: vec![(escrow_addr, StealthInput::with_witness(commitment, witness))],
        input_value: value,
        outputs: vec![],
        change_to: Some(party_addr),
    };
    let max_epoch = s.max_epoch().await?;
    let session = &s;
    let spend_ref = &spend;
    let method = book_method.to_string();
    let build = move |fee: u64| {
        let method = method.clone();
        async move {
            let prepared = spend_ref.prepare(session, fee).await?;
            let tx = TransactionBuilder::new(session.provider.network(), max_epoch)
                .with_auto_fill_inputs()
                .add_input(TARI_TOKEN)
                .add_input(utxo_input(&commitment));
            let tx = with_stealth_fee(tx, prepared.statement.clone())
                .call_method(book, method.as_str(), args![invoice])
                .build_unsigned();
            Ok((tx, prepared))
        }
    };
    let priced = chain::price(&s, label, build).await?;
    let res = chain::send(&mut s, label, &priced).await?;
    let prepared = priced.prepared;
    if res.accepted {
        let out = prepared.statement.stealth_outputs().last().map(|o| o.commitment().to_hex());
        if let Some(c) = out {
            state.coins.entry(persona.to_string()).or_default().insert(c, value - res.fee);
        }
    }
    state.invoices.get_mut(&invoice).unwrap().txs.insert(label.to_string(), res.tx_id);
    Ok(())
}

async fn attack(personas: &Personas, state: &mut State, invoice: u64, kind: &str) -> Result<()> {
    let inv = state.invoices.get(&invoice).cloned().ok_or_else(|| anyhow!("unknown invoice {invoice}"))?;
    let code = inv.approval_code.as_bytes().to_vec();
    match kind {
        // A stranger who learned the approval code tries to take the money.
        "stranger-claim" => {
            run_escrow_spend(personas, state, invoice, "stranger", &format!("attack:stranger-claim:{invoice}:{}", if inv.txs.contains_key("approve") { "after-approval" } else { "before-approval" }), "settle", false, code).await
        },
        // The contractor tries to claim without the code.
        "claim-without-code" => run_escrow_spend(
            personas,
            state,
            invoice,
            &inv.contractor,
            &format!("attack:claim-without-code:{invoice}"),
            "settle",
            false,
            b"i-did-the-work".to_vec(),
        )
        .await,
        // The client tries to take the money back before the deadline.
        "early-refund" => run_escrow_spend(
            personas,
            state,
            invoice,
            &inv.client,
            &format!("attack:early-refund:{invoice}"),
            "refund",
            true,
            Vec::new(),
        )
        .await,
        // The contractor tries to approve their own invoice in the book.
        "self-approve" => {
            let book = book_address(state)?;
            paid_call(personas, state, &inv.contractor, &format!("attack:self-approve:{invoice}"), vec![], move |tx| {
                tx.call_method(book, "approve", args![invoice, code.clone()])
            })
            .await
            .map(|_| ())
        },
        other => bail!("unknown attack {other}"),
    }
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
            "escrow_commitment": inv.escrow_commitment,
            "escrow_root": inv.escrow_root,
            "txs": inv.txs,
        },
        "parties": {
            "contractor_key": persona_key(personas, &inv.contractor)?.to_string(),
            "client_key": persona_key(personas, &inv.client)?.to_string(),
        },
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
