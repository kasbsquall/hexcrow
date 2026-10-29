# Hexcrow

**Milestone escrow nobody else can read.** A contractor invoices, the client locks the money, approval releases it, and a mediator both sides chose settles a dispute. On Tari Ootle the amount and the parties stay hidden; only the rules are public.

Built for the Tari Council October 2026 build contest, "Spooky Secrets".

- **Hosted demo:** https://kasbsquall.github.io/hexcrow/ No wallet, no install. It replays a real run on Esmeralda testnet: four invoices, five copies of each, and ten attacks you can replay yourself. Each replay asks the public indexer again and, once the indexer has dropped the result, shows the one archived right after the run.
- **Proof:** every transaction of the run, with how to check it yourself, in [PROOF.md](PROOF.md).
- **What stays hidden and what leaks:** [THREAT_MODEL.md](THREAT_MODEL.md).

![The public copy of invoice 0001: the amount and the invoice text did not transfer to this copy](docs/demo-public.png)

## The problem

An independent designer starts each milestone without knowing whether the client has the money. If they get paid on a transparent chain, anyone can see what they charge, who they bill and how much they earn each month. The company paying them has the opposite worry: it wants to release money only on approval, and get it back if the work never arrives. When the two disagree, someone they both trust has to be able to settle it without seeing more than the invoice.

A member of the Tari Council asked for this on the forum on 25 October: confidential invoicing, private payments to contractors, milestone escrow for commercial work, and selective disclosure to authorised parties ([Field Notes for Ootle Builders](https://community.tari.com/t/486)).

## How it works

One invoice, five copies. Like a carbon-copy invoice pad, each viewer holds a different copy, and what did not transfer to a copy is what its holder cannot see.

| Copy | Sees |
|---|---|
| Public (anyone with the chain) | Invoice hash, deadlines, status, escrow commitment and condition root, one-time keys, transaction ids. No amount, no names, no invoice text |
| Client | The above, plus the amount, the invoice text and the approval code it chose |
| Contractor | The above, plus the amount it is owed, and a check that the escrow holds it before work starts |
| Mediator | Nothing private until a party calls it in. Then the invoice text, which it checks against the hash in the book, and the ruling it writes |
| Accountant | Given the invoice salt and text, recomputes the invoice hash and reads the amount with that invoice's escrow view key. Cannot spend |

![The client copy of the same invoice](docs/demo-client.png)

**The money never sits in a contract.** The escrow is a stealth (confidential) TARI output paid to a condition tree with no key path ([cli/src/escrow.rs](cli/src/escrow.rs)). The Ootle engine itself only lets it be spent four ways:

| Leaf | All of these are required |
|---|---|
| Claim | the approval code (hash lock), the contractor's key, before the claim window closes |
| Ruling for the contractor | the mediator's key and the contractor's key, before the claim window closes |
| Ruling for the client | the mediator's key and the client's key, before the claim window closes |
| Refund | the client's key, once the claim window has closed |

Each invoice has two epochs: the client approves before the **deadline**, and the contractor can claim until the **claim window** closes a few epochs later. An approval given in the last epoch still leaves the contractor time to collect, and the refund leaf cannot open before then. The mediator can rule during the same window.

**The invoice book is the audit trail.** A small template ([template/src/lib.rs](template/src/lib.rs)) records, per invoice, the salted hash of the invoice text, the three one-time keys, the deadline and the claim window, the hash of the approval code, the escrow's condition root, the hash of a ruling if there was one, and the status: issued, funded, approved, settled or refunded. It stores no amount, no account and no name. It has no owner, so nobody can rewrite its rules. Each step checks its signer: only the client funds, approves or refunds; only the contractor issues and settles; a ruling needs the mediator and the side it pays.

**A mediator for disputes.** The contractor names a mediator's one-time key when it issues the invoice, and the client accepts that mediator by funding. If the client goes silent after the work is delivered, or the contractor walks away, the mediator rules. It cannot pay anyone alone: each ruling leaf also needs the key of the side it pays, and both close with the claim window. The ruling text stays off chain and the book keeps its salted hash, so either side can later show the ruling and anyone can check it.

**The client holds the approval code.** The contractor issues the invoice without one. The client picks the code when it funds and records only its hash, in the book and in the escrow's claim leaf, so the contractor cannot claim with a code it chose itself.

**Every invoice has its own keys.** The client, the contractor, the mediator and the escrow viewing key are created for one invoice and used for nothing else, so two invoices of the same people share no key on chain.

**The contractor checks before it works.** `hexcrow verify <invoice> --expect-tari <amount>` reads the book and the escrow output from the public indexer. It checks that the book names its key and the agreed mediator, rebuilds the condition root from the book's terms and confirms, against a verified proof, that the escrow exists unspent and is locked by exactly that root with no key path. It also decrypts the amount with the invoice's escrow view key and checks it against what the contractor agreed to, and checks that the client can still approve. Passing means the money is locked as agreed; being paid still needs the client's approval or a ruling.

**The client checks before it funds.** `hexcrow fund` reads the invoice from the book, not from what it was told, and refuses to fund an invoice that is not addressed to its key, names a mediator other than the agreed one, or has a claim window running more than 48 epochs past the deadline.

Funding creates the escrow output and records it in the book in one transaction. Claiming, ruling and refunding spend the escrow and update the book in one transaction.

## What the escrow adds to a hash lock

Ootle's spend conditions already give a stealth output a hash lock and a timeout refund. Hexcrow adds what an invoice needs on top of them:

- **An invoice book an accountant can check.** Each invoice is a salted hash in a public book with no owner. Whoever is shown the text recomputes the hash and finds the record, its deadlines and what happened to it.
- **A key per invoice for every party.** Client, contractor, mediator and view key are new for each invoice, so two invoices between the same people share nothing on chain.
- **A check before the work starts.** The contractor confirms from public data that the escrow exists, holds the agreed amount and is locked by exactly these rules, with no key that could take it back early.
- **A mediator for disputes.** Two ruling leaves, each needing the mediator and the side it pays, closing with the claim window.

## What ran on Esmeralda

Template v4 `template_8eba2e85…6b5fd3`, book `component_f427dbf5…6372ba`, on 29 October 2026. Full ids and how to verify each one are in [PROOF.md](PROOF.md). The four stories are scripted with throwaway testnet keys; every transaction is real.

Four invoices, each leaving the escrow through a different leaf:

| Invoice | What happened | Leaf |
|---|---|---|
| 0001, 250 tTARI | Funded, checked by the contractor, approved, claimed. Nine attacks along the way, all refused | Claim |
| 0002, 120 tTARI | The work was delivered and the client went silent past the deadline. The mediator and the contractor signed a ruling for the contractor | Ruling for the contractor |
| 0003, 60 tTARI | The contractor withdrew. The mediator and the client signed a ruling for the client, without waiting for the claim window to close | Ruling for the client |
| 0004, 40 tTARI | Nobody acted. A ruling sent after the claim window closed was refused, and the client took the money back | Refund |

Every attack was sent to the network, signed by the party named:

| Attack | Refused by |
|---|---|
| The contractor funds its own invoice with a code it chose | the book: the signer is not the client |
| A stranger tries to claim | the engine: access denied on the escrow spend |
| The contractor claims without the approval code | the engine: hash lock not satisfied |
| The client takes the money back before the claim window closes | the engine: `AfterEpoch(12722)` not satisfied |
| The contractor approves its own invoice | the book: the signer is not the client |
| The mediator tries to pay itself through a ruling leaf | the engine: access denied on the escrow spend |
| The contractor uses a ruling leaf without the mediator | the engine: access denied on the escrow spend |
| The client approves, then tries to take the money back | the engine: `AfterEpoch(12722)` not satisfied |
| A stranger claims with the approval code, after it became public | the engine: access denied on the escrow spend |
| The mediator and the contractor rule after the claim window closed | the engine: `BeforeEpoch(12719)` not satisfied |

The four invoices share no key: each has its own client, contractor, mediator and escrow keys, so the chain cannot tell that 0001 and 0002 are two milestones of the same project.

## Run it

Engine tests: 19 tests run the compiled template in the same engine the network uses (`tari_template_test_tooling`). The engine's Cranelift backend does not build on Windows, so they run in Docker:

```bash
cd template
docker run --rm -v "$PWD":/w -w /w rust:1-bookworm bash -c "rustup target add wasm32-unknown-unknown && cargo test"
```

Last run: [evidence/local-engine-tests.txt](evidence/local-engine-tests.txt).

Web demo, locally:

```bash
cd web
python -m http.server 8765
```

CLI against Esmeralda (`cli/`, Rust, `ootle-rs` 0.29). It keeps testnet keys in `cli/.hexcrow/`, which is git-ignored.

```bash
cd cli
cargo run -- faucet contractor-5
cargo run -- faucet client-5
cargo run -- issue --doc "Invoice 0005 | ..." --deadline-in 30 --grace 2
cargo run -- fund 5 --amount-tari 50
cargo run -- verify 5 --expect-tari 50
cargo run -- approve 5
cargo run -- claim 5
cargo run -- inspect 5
```

In a dispute the mediator rules instead: `cargo run -- rule 5 --for contractor --text "..."`, or `--for client`, spends the matching ruling leaf with the mediator's key and the paid side's key, and records the ruling's salted hash in the book.

`cargo run -- attack <invoice> <kind>` sends an attack and records its refusal; the kinds are `contractor-sets-lock`, `stranger-claim`, `claim-without-code`, `early-refund`, `self-approve`, `refund-after-approval`, `mediator-alone`, `ruling-without-mediator` and `late-ruling`. Every submitted transaction is appended to [evidence/esmeralda-run.jsonl](evidence/esmeralda-run.jsonl). `python tools/export_run.py` turns the run into the demo's data file.

## Limits, stated plainly

- Testnet only. Amounts are tTARI.
- Timing, status and fees are public, as on any Ootle transaction. In this run the one-time keys were funded from the public testnet faucet within 21 seconds of each other, so timing can hint that they belong together.
- The book records what the client says it funded, and the escrow leaves do not call the book. The contractor's `verify` is what ties the record to a real escrow output; after that, a party who spends a leaf without updating the book leaves the book one step behind the escrow, which the demo shows by reading the escrow output live.
- The mediator is trusted to judge the work. A mediator colluding with one side can rule for that side, as with any arbiter; it still needs that side's key, and only one invoice and one claim window are at stake. A mediator that never answers changes nothing: the invoice goes back to the client after the claim window.
- The public indexers dropped our transaction results within hours, well before the 50 epochs they advertise. The raw results are saved in [evidence/indexer-results/](evidence/indexer-results/), and the demo links each archived result when the indexer no longer serves it.
- To let you open every copy, the hosted demo carries the parties' private data for this run, made with throwaway testnet keys.

Details in [THREAT_MODEL.md](THREAT_MODEL.md).

## Layout

```
template/   the invoice book template (Rust to WASM) and its engine tests
cli/        the client that ran everything on Esmeralda
web/        the hosted demo: static HTML, CSS and JS, no build step
evidence/   the run log, the indexer's raw results and the test output
tools/      export of the run for the demo, verdict snapshots, WASM section stripping
```

## License

MIT, see [LICENSE](LICENSE). Interface sounds by Kenney, CC0 ([web/sounds/](web/sounds/)).
