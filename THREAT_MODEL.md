# Threat model

What Hexcrow hides, what it does not, and what an attacker can still do. Every claim below points at code or at a transaction on Esmeralda testnet.

## What the chain enforces

The escrow is a stealth output paid to a condition tree with no key path (`cli/src/escrow.rs`). It can be spent in exactly two ways:

| Leaf | Conditions, all required |
|---|---|
| Claim | `HashLock(sha256(approval_code))`, `BeforeEpoch(claim_until)`, signature of the contractor's key |
| Refund | `AfterEpoch(claim_until)`, signature of the client's key |

`claim_until` is the approval deadline plus a grace period the contractor sets when it issues the invoice. The book refuses an approval from the deadline on, so every approval leaves at least the grace period to claim, and the refund leaf cannot open while an approved invoice can still be claimed.

Hexcrow has no admin key and the invoice book has no owner (`template/src/lib.rs`, `OwnerRule::None`). Nobody, including the Hexcrow author, can move an escrow outside these two leaves.

Attacks run against the real network on 29-oct-2026 with template v3 on invoice 1, all rejected (`evidence/esmeralda-run.jsonl`, raw results in `evidence/indexer-results/`):

| Attempt | Rejected by | Transaction |
|---|---|---|
| The contractor funds its own invoice with an approval code it chose | Book: signer is not the client | 22cab3e8… |
| A stranger claims | Engine: access denied on the stealth spend | 0e061571… |
| The contractor claims without the approval code | Engine: hash lock not satisfied | 3868016e… |
| The client refunds before the deadline | Engine: `AfterEpoch(12718)` not satisfied | 0c8fd4c7… |
| The contractor approves its own invoice | Book: signer is not the client | 5cf4e8df… |
| The client approves, then tries to refund while the claim window is open | Engine: `AfterEpoch(12718)` not satisfied | d7daf43f… |
| A stranger claims with the approval code already public | Engine: access denied on the stealth spend | cb742b3a… |

## Fixed after review: two design flaws in template v2

A review of the earlier version (template v2, the first run of 29-oct-2026) found two flaws. Both are fixed in v3 and covered by engine tests and by the attacks above.

1. **The contractor chose the approval lock.** In v2 the contractor's `issue` call carried the hash of the approval code, and the CLI generated the code in the contractor's step. A dishonest contractor could lock the escrow to a code it already knew and claim without approval. In v3 `issue` takes no lock; the client picks the code when it funds and records its hash in `fund`, which only the client can call. Test: `only_the_client_sets_the_approval_lock`. On chain: 22cab3e8….
2. **An approval did not protect the contractor.** In v2 the claim leaf closed at the deadline and the refund leaf opened at the same epoch. A client could approve in the last epoch and refund right after. In v3 the two leaves meet at `claim_until`, after a grace period, and the book refuses to settle after it or to approve from the deadline on. Tests: `approval_in_the_last_epoch_still_leaves_the_claim_window`, `no_approval_after_deadline_and_no_settle_after_claim_window`. On chain: d7daf43f….

The same review found that v2 used one escrow viewing key for every invoice, which linked invoices to each other and let one key read every amount. In v3 every invoice has its own client, contractor and escrow keys (`cli/src/personas.rs`, `invoice_names`), and salts and approval codes come from the OS random source.

## What stays hidden from the public

- The amount. It lives in a Pedersen commitment; only holders of that invoice's escrow view key decrypt it.
- The invoice text. The book stores `sha256(salt | text)`; without the salt the hash reveals nothing useful.
- Which long-lived accounts are involved, and which invoices share a party. Each invoice uses keys created for it alone.

## What leaks, by design or by limitation

1. **Timing and status.** Issue, fund, approve, claim and refund are public events with their epochs. Anyone can see that an invoice was funded and later settled or refunded.
2. **Fees.** Each transaction pays its fee through a revealed output to its sealing key (`cli/src/chain.rs`, `Spend::prepare`). Fee amounts are public, as on any Ootle transaction. Since every sealing key belongs to one invoice, the fee outputs do not link invoices.
3. **The approval code becomes public on approval.** The book stores it so that the approval is auditable. This is safe because the claim leaf also needs the contractor's signature; the attack in cb742b3a… shows a stranger holding the public code still cannot claim.
4. **Funding the one-time keys.** In this run each one-time key was funded from the public testnet faucet, and the six faucet claims of the three invoices happened within 15 minutes of each other (16:18 to 16:32 UTC). Timing can hint that they belong together. A real client would fund its one-time key with a private transfer from its own wallet, at a time of its choosing.
5. **The escrow view key is shared within one invoice.** Both parties (and any accountant they hand it to) hold that invoice's escrow view key. Anyone who obtains it sees that invoice's amount, and only that one. It cannot spend: the output has no key path.
6. **The book records what the client says.** The book cannot see stealth outputs, so `fund` records the client's claim of a condition root, and the escrow leaves do not call the book. The contractor closes that gap before starting work with `hexcrow verify` (`cli/src/main.rs`, `verify`). It rebuilds the condition root from the book's terms and checks that the client can still approve. It reads the escrow output through the provider, which checks the indexer's proof against the validator set, and confirms the output is unspent and locked by exactly that root with no key path. It also decrypts the amount with the invoice's escrow view key and compares it with the amount the contractor agreed to. Invoice 3 shows both outcomes: told to expect 600 tTARI the check fails, at 60 it passes (`verify:3` in the run log). Invoices 1 and 2 ran an earlier version of the check without the amount and deadline steps. The book itself is read as plain JSON, without a proof; a lying indexer could misreport it, but not the verified escrow output it must match. Passing means the money is locked as agreed; being paid still needs the client's approval. After funding, a party can spend a leaf without updating the book, which would leave the book one step behind; the web demo reads the escrow output live, so a spent escrow shows as spent whatever the book says.
7. **Fee intent is not atomic with the book call.** The stealth transfer that creates or spends the escrow rides in the fee intent, so it commits even if the book call in the same transaction fails. The indexer reports that case as `AcceptFeeRejectRest`, as for 5cf4e8df…, where the fee was taken and the book call refused. The CLI refuses to submit a real step unless the dry run accepts the whole transaction (`cli/src/chain.rs`, `send`).
8. **Spending reveals the leaf that was used.** The spend witness carries that leaf's conditions so the engine can check them: the claim of invoice 1 (f1907e76…) shows the hash lock, `BeforeEpoch(12718)` and the contractor's one-time key. The leaf that was not used stays behind the condition root. It does not reveal the amount.
9. **The contractor sets the claim window.** `issue` takes any grace period. A contractor could set one so long that an unapproved invoice never returns to the client. The client's `fund` reads the terms from the book and refuses a claim window longer than 48 epochs past the deadline (`cli/src/main.rs`, `MAX_GRACE_EPOCHS`); a client using another tool has to check `claim_until` in the book before funding.

## What Hexcrow does not solve

- **Disputes.** If the client never approves, the client gets the money back after the claim window even if the work was delivered. Hexcrow guarantees the escrow follows the rules both parties signed up to; it does not judge the work.
- **Deadline granularity.** Deadlines are Ootle epochs. On Esmeralda an epoch lasted more than half an hour during our runs, so short deadlines are coarse.
- **Spam.** The book is open: anyone can issue invoices against any client key. An invoice only matters to a client who chooses to fund it.

## Trust in the indexer

The CLI verifies what the indexer serves against the network's validator sets (`ootle-rs`, `IndexerTrust::Verify`). A fresh run anchors on the latest end-of-epoch checkpoint and then refuses inputs proven in an earlier epoch, which is exactly the case of an escrow refunded after its deadline. The first refund attempt of the v2 run failed that way on 29-oct-2026. The CLI now saves the trusted validator set after each transaction (`.hexcrow/trust.json`) and walks the signed handovers from it on the next run. `hexcrow anchor-trust <epoch>` anchors on an earlier checkpoint for a first run; like any first-use anchor, it takes the indexer at its word for that one checkpoint, and every later handover is checked against signatures.

The public indexers (`ootle-indexer-a` and `-b`) advertise a 50-epoch transaction retention, but by 16:06 UTC on 29-oct-2026 both answered 404 for every transaction of that morning, fewer than 10 epochs later. Substates do not expire: the book and the escrow outputs (`utxo_…`, up while unspent, down once spent) stay readable. The raw transaction results of the v3 run are saved in `evidence/indexer-results/`, and the web demo falls back to a dated snapshot of them (`web/data/verdicts.json`), labelled as such.

The web demo reads the indexer without verifying proofs. It shows what the indexer reports and labels it as such.

## Out of scope for this build

Wallet compromise, indexer availability, and the cryptographic soundness of the Ootle engine itself.
