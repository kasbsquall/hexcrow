# Hexcrow

**Milestone escrow nobody else can read.** A contractor invoices, the client locks the money, approval releases it. On Tari Ootle the amount and the parties stay hidden; only the rules are public.

Built for the Tari Council October 2026 build contest, "Spooky Secrets".

- **Hosted demo:** https://kasbsquall.github.io/hexcrow/ No wallet, no install: it replays a real run on Esmeralda testnet and checks every public fact against the public indexer while you watch.
- **Proof:** every transaction of the run, with how to check it yourself, in [PROOF.md](PROOF.md).
- **What stays hidden and what leaks:** [THREAT_MODEL.md](THREAT_MODEL.md).

![The public copy of invoice 0001: the amount and the invoice text did not transfer to this copy](docs/demo-public.png)

## The problem

An independent designer starts each milestone without knowing whether the client has the money. If they get paid on a transparent chain, anyone can see what they charge, who they bill and how much they earn each month. The company paying them has the opposite worry: it wants to release money only on approval, and get it back if the work never arrives.

A member of the Tari Council asked for this on the forum on 25 October: confidential invoicing, private payments to contractors, milestone escrow for commercial work, and selective disclosure to authorised parties ([Field Notes for Ootle Builders](https://community.tari.com/t/486)).

## How it works

One invoice, four copies. Like a carbon-copy invoice pad, each viewer holds a different copy, and what did not transfer to a copy is what its holder cannot see.

| Viewer | Sees |
|---|---|
| Public (anyone with the chain) | Invoice hash, deadlines, status, escrow commitment and condition root, one-time keys, transaction ids. No amount, no names, no invoice text |
| Client | The above, plus the amount, the invoice text and the approval code it chose |
| Contractor | The above, plus the amount it is owed, and a check that the escrow exists before it starts work |
| Accountant | Given the invoice salt and text, recomputes the invoice hash and reads the amount with that invoice's escrow view key. Cannot spend |

![The client copy of the same invoice](docs/demo-client.png)

**The money never sits in a contract.** The escrow is a stealth (confidential) TARI output paid to a condition tree with no key path ([cli/src/escrow.rs](cli/src/escrow.rs)). The Ootle engine itself only lets it be spent two ways:

| Path | All of these are required |
|---|---|
| Claim | the approval code (hash lock), the contractor's one-time key, before the claim window closes |
| Refund | the client's one-time key, once the claim window has closed |

Each invoice has two epochs: the client approves before the **deadline**, and the contractor can claim until the **claim window** closes a few epochs later. An approval given in the last epoch still leaves the contractor time to collect, and the refund leaf cannot open before then.

**The invoice book is the audit trail.** A small template ([template/src/lib.rs](template/src/lib.rs)) records, per invoice, the salted hash of the invoice text, the two one-time keys, the deadline and the claim window, the hash of the approval code, the escrow's condition root and the status: issued, funded, approved, settled or refunded. It stores no amount, no account and no name. It has no owner, so nobody can rewrite its rules. Each step checks its signer: only the client funds, approves or refunds; only the contractor issues and settles.

**The client holds the approval code.** The contractor issues the invoice without one. The client picks the code when it funds and records only its hash, in the book and in the escrow's claim leaf, so the contractor cannot claim with a code it chose itself.

**Every invoice has its own keys.** The client, the contractor and the escrow viewing key are created for one invoice and used for nothing else, so two invoices of the same people share no key on chain.

**The contractor checks before it works.** `hexcrow verify <invoice> --expect-tari <amount>` reads the book and the escrow output from the public indexer. It rebuilds the condition root from the book's terms and confirms, against a verified proof, that the escrow exists unspent and is locked by exactly that root with no key path. It also decrypts the amount with the invoice's escrow view key and checks it against what the contractor agreed to, and checks that the client can still approve. Passing means the money is locked as agreed; being paid still needs the client's approval.

**The client checks before it funds.** `hexcrow fund` reads the invoice from the book, not from what it was told, and refuses to fund an invoice that is not addressed to its key or whose claim window runs more than 48 epochs past the deadline.

Funding creates the escrow output and records it in the book in one transaction. Claiming and refunding spend the escrow and update the book in one transaction.

## What ran on Esmeralda

Template `template_0b82e0db…a9f754`, book `component_40bf6180…263931`, on 29 October 2026. Full ids and how to verify each one are in [PROOF.md](PROOF.md).

**Invoice 0001, approved and settled.** Issued, funded with 250 tTARI, checked by the contractor, approved, claimed. Seven attacks along the way, all rejected:

| Attack | Rejected by |
|---|---|
| The contractor funds its own invoice with a code it chose | the book: the signer is not the client |
| A stranger tries to claim | the engine: access denied on the escrow spend |
| The contractor claims without the approval code | the engine: hash lock not satisfied |
| The client takes the money back before the deadline | the engine: `AfterEpoch(12718)` not satisfied |
| The contractor approves its own invoice | the book: the signer is not the client |
| The client approves, then tries to take the money back | the engine: `AfterEpoch(12718)` not satisfied |
| A stranger claims with the approval code, after it became public | the engine: access denied on the escrow spend |

**Invoice 0002, refunded.** Issued, funded with 40 tTARI, checked by the contractor, never approved, refunded to the client after its claim window closed.

**Invoice 0003, the next milestone of the same project as 0001.** Issued, funded with 60 tTARI, checked by the contractor with the amount check (it fails when told to expect 600 tTARI and passes at 60), approved, claimed.

The three invoices share no key: each has its own client, contractor and escrow keys, so the chain cannot tell that 0001 and 0003 involve the same two people.

## Run it

Engine tests: 15 tests run the compiled template in the same engine the network uses (`tari_template_test_tooling`). The engine's Cranelift backend does not build on Windows, so they run in Docker:

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
cargo run -- faucet contractor-4
cargo run -- faucet client-4
cargo run -- issue --doc "Invoice 0004 | ..." --deadline-in 30 --grace 2
cargo run -- fund 4 --amount-tari 50
cargo run -- verify 4 --expect-tari 50
cargo run -- approve 4
cargo run -- claim 4
cargo run -- inspect 4
```

`cargo run -- attack <invoice> <kind>` sends an attack and records its rejection; the kinds are `contractor-sets-lock`, `stranger-claim`, `claim-without-code`, `early-refund`, `self-approve` and `refund-after-approval`. Every submitted transaction is appended to [evidence/esmeralda-run.jsonl](evidence/esmeralda-run.jsonl). `python tools/export_run.py` turns the run into the demo's data file.

## Limits, stated plainly

- Testnet only. Amounts are tTARI.
- Timing, status and fees are public, as on any Ootle transaction. In this run the one-time keys were funded from the public testnet faucet within minutes of each other, so timing can hint that they belong together.
- The book records what the client says it funded, and the escrow leaves do not call the book. The contractor's `verify` is what ties the record to a real escrow output; after that, a party who spends a leaf without updating the book leaves the book one step behind the escrow, which the demo shows by reading the escrow output live.
- Hexcrow enforces the rules both parties agreed to. It does not judge the work: an invoice the client never approves goes back to the client after the claim window.
- The public indexers dropped our transaction results within hours, well before the 50 epochs they advertise. The raw results are saved in [evidence/indexer-results/](evidence/indexer-results/), and the demo falls back to a dated snapshot of them.
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
