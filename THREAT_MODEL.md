# Threat model

What Hexcrow hides, what it does not, and what an attacker can still do. Every claim below points at code or at a transaction on Esmeralda testnet.

## What the chain enforces

The escrow is a stealth output paid to a condition tree with no key path (`cli/src/escrow.rs`). It can be spent in exactly four ways:

| Leaf | Conditions, all required |
|---|---|
| Claim | `HashLock(sha256(approval_code))`, `BeforeEpoch(claim_until)`, signature of the contractor's key |
| Ruling for the contractor | `BeforeEpoch(claim_until)`, signatures of the mediator's key and of the contractor's key |
| Ruling for the client | `BeforeEpoch(claim_until)`, signatures of the mediator's key and of the client's key |
| Refund | `AfterEpoch(claim_until)`, signature of the client's key |

`claim_until` is the approval deadline plus a grace period the contractor sets when it issues the invoice. The book refuses an approval from the deadline on, so every approval leaves at least the grace period to claim, and the refund leaf cannot open while an approved invoice can still be claimed. The same window gives a contractor whose client never answers the time to ask the mediator for a ruling.

Hexcrow has no admin key and the invoice book has no owner (`template/src/lib.rs`, `OwnerRule::None`). Nobody, including the Hexcrow author, can move an escrow outside these four leaves.

## The mediator

Both parties agree on a mediator before work starts. The contractor names the mediator's one-time key in `issue`; the client accepts it by funding, and the CLI refuses to fund an invoice whose book entry names another mediator (`cli/src/main.rs`, `fund`). The contractor's `verify` checks the same thing before work starts.

What the mediator can do: sign a ruling leaf together with the side it rules for, before the claim window closes. The escrow then pays that side, and the book records `sha256(salt | ruling text)` (`rule_for_contractor`, `rule_for_client`).

What a ruling cannot do. The escrow's ruling leaves enforce the first three points on every spend, the network refused the attempts listed in the table below, and the book's engine tests cover the same cases for the record (`a_ruling_needs_the_mediator_and_the_party_it_pays`, `no_ruling_after_approval_or_after_the_claim_window`):

- pay anyone on the mediator's signature alone: each ruling leaf also needs the key of the side it pays;
- happen without the mediator: neither party alone can use a ruling leaf;
- happen after the claim window closes;
- be recorded after the client approved. The book refuses it, but the escrow's leaves do not read the book: a mediator and a client acting together could still spend the ruling leaf for the client after an approval, until the contractor claims. The contractor closes that gap by claiming as soon as the approval code is public, as in invoice 1.

What the mediator sees: nothing, unless a party calls it in. Its copy of an invoice starts blank. A party that calls it in shows it the invoice text and salt, which the mediator can check against the hash in the book; the ruling text it writes stays off chain, with only its hash in the book.

What it does not protect against: a mediator that colludes with one side can rule for that side. This is the trust any arbitrated escrow places in its arbiter; Hexcrow narrows it to one invoice, one window and a signature that both the mediator and the paid side must give. A mediator that never answers changes nothing: the invoice behaves as without one, and the refund leaf opens when the window closes.

## Attacks sent to the real network

Run on 29-oct-2026 with template v4, all refused (`evidence/esmeralda-run.jsonl`, raw results in `evidence/indexer-results/`):

| Attempt | Refused by | Transaction |
|---|---|---|
| The contractor funds its own invoice with an approval code it chose | Book: the signer is not the client | a5523193… |
| A stranger claims | Engine: access denied on the stealth spend | 6e609c05… |
| The contractor claims without the approval code | Engine: hash lock not satisfied | c1951cdb… |
| The client refunds before the claim window closes | Engine: `AfterEpoch(12722)` not satisfied | a2de96f2… |
| The contractor approves its own invoice | Book: the signer is not the client | ea2d059b… |
| The mediator tries to pay itself through a ruling leaf | Engine: access denied, the contractor did not sign | c81bdfeb… |
| The contractor uses a ruling leaf without the mediator | Engine: access denied, the mediator did not sign | cb635914… |
| The client approves, then tries to refund while the claim window is open | Engine: `AfterEpoch(12722)` not satisfied | acfc88f8… |
| A stranger claims with the approval code already public | Engine: access denied on the stealth spend | 1f3b07c8… |
| The mediator and the contractor rule after the claim window closed | Engine: `BeforeEpoch(12719)` not satisfied | 993341d9… |

## Fixed after review

**Template v2, two design flaws.** A review of v2 (the first run of 29-oct-2026) found that the contractor chose the approval lock, so it could claim with a code it already knew, and that an approval in the last epoch did not protect the contractor, because the claim leaf closed when the refund leaf opened. v3 fixed both: the client sets the lock when it funds (`only_the_client_sets_the_approval_lock`), and the two leaves meet at `claim_until`, after a grace period (`approval_in_the_last_epoch_still_leaves_the_claim_window`). The same review found one escrow viewing key shared by every invoice; since v3 every invoice has its own keys (`cli/src/personas.rs`, `invoice_names`).

**Template v3, no recourse for the contractor.** In v3 a client who never approved got the money back after the claim window, even for delivered work. v4 adds the mediator and its two ruling leaves. Invoice 2 of the v4 run is that case: the client went silent and the mediator's ruling paid the contractor.

## What stays hidden from the public

- The amount. It lives in a Pedersen commitment; only holders of that invoice's escrow view key decrypt it.
- The invoice text and the ruling text. The book stores salted SHA-256 hashes; without the salt a hash reveals nothing useful.
- Which long-lived accounts are involved, and which invoices share a party or a mediator. Each invoice uses keys created for it alone.

## What leaks, by design or by limitation

1. **Timing and status.** Issue, fund, approve, claim, ruling and refund are public events with their epochs. Anyone can see that an invoice was funded and later settled or refunded, and whether a ruling closed it.
2. **Fees.** Each transaction pays its fee through a revealed output to its sealing key (`cli/src/chain.rs`, `Spend::prepare`). Fee amounts are public, as on any Ootle transaction. Since every sealing key belongs to one invoice, the fee outputs do not link invoices.
3. **The approval code becomes public on approval.** The book stores it so that the approval is auditable. This is safe because the claim leaf also needs the contractor's signature; 1f3b07c8… shows a stranger holding the public code still cannot claim.
4. **Funding the one-time keys.** In this run each one-time key was funded from the public testnet faucet, and the eight faucet claims of the four invoices happened within 21 seconds (19:16:34 to 19:16:55 UTC). Timing can tell that they belong together. A real client would fund its one-time key with a private transfer from its own wallet, at a time of its choosing.
5. **The escrow view key is shared within one invoice.** Both parties, and any accountant or mediator they hand it to, hold that invoice's escrow view key. Anyone who obtains it sees that invoice's amount, and only that one. It cannot spend: the output has no key path.
6. **The book records what the client says.** The book cannot see stealth outputs, so `fund` records the client's claim of a condition root, and the escrow leaves do not call the book. The contractor closes that gap before starting work with `hexcrow verify` (`cli/src/main.rs`, `verify`). It checks that the book names its key and the agreed mediator, rebuilds the condition root from the book's terms, and checks that the client can still approve. It reads the escrow output through the provider, which checks the indexer's proof against the validator set, and confirms the output is unspent and locked by exactly that root with no key path. It also decrypts the amount with the invoice's escrow view key and compares it with the amount the contractor agreed to: told to expect 2,500 tTARI for invoice 1 the check fails, at 250 it passes (`verify:1` in the run log). The book itself is read as plain JSON, without a proof; a lying indexer could misreport it, but not the verified escrow output it must match. After funding, a party can spend a leaf without updating the book, which would leave the book one step behind; the web demo reads the escrow output live, so a spent escrow shows as spent whatever the book says.
7. **Fee intent is not atomic with the book call.** The stealth transfer that creates or spends the escrow rides in the fee intent, so it commits even if the book call in the same transaction fails. The indexer reports that case as `AcceptFeeRejectRest`, as for ea2d059b…, where the fee was taken and the book call refused. The CLI refuses to submit a real step unless the dry run accepts the whole transaction (`cli/src/chain.rs`, `send`).
8. **Spending reveals the leaf that was used.** The spend witness carries that leaf's conditions so the engine can check them: a claim shows the hash lock, the epoch and the contractor's key; a ruling shows the mediator's key next to the paid side's. The leaves that were not used stay behind the condition root. Nothing reveals the amount.
9. **The contractor sets the claim window.** `issue` takes any grace period. A contractor could set one so long that an unapproved invoice never returns to the client. The client's `fund` reads the terms from the book and refuses a claim window longer than 48 epochs past the deadline (`cli/src/main.rs`, `MAX_GRACE_EPOCHS`); a client using another tool has to check `claim_until` in the book before funding.

## What Hexcrow does not solve

- **Judging the work.** A dispute needs a mediator both sides trust. Without one, or if it never answers, an invoice the client does not approve goes back to the client after the claim window.
- **Deadline granularity.** Deadlines are Ootle epochs. On Esmeralda an epoch lasted about 70 minutes during our runs, so short deadlines are coarse.
- **Spam.** The book is open: anyone can issue invoices against any client key. An invoice only matters to a client who chooses to fund it.

## Trust in the indexer

The CLI verifies what the indexer serves against the network's validator sets (`ootle-rs`, `IndexerTrust::Verify`). A fresh run anchors on the latest end-of-epoch checkpoint and then refuses inputs proven in an earlier epoch, which is exactly the case of an escrow refunded after its deadline. The first refund attempt of the v2 run failed that way on 29-oct-2026. The CLI now saves the trusted validator set after each transaction (`.hexcrow/trust.json`) and walks the signed handovers from it on the next run. `hexcrow anchor-trust <epoch>` anchors on an earlier checkpoint for a first run; like any first-use anchor, it takes the indexer at its word for that one checkpoint, and every later handover is checked against signatures.

The public indexers (`ootle-indexer-a` and `-b`) advertise a 50-epoch transaction retention, but on 29-oct-2026 both answered 404 for transactions of that morning fewer than 10 epochs later. Substates do not expire: the book and the escrow outputs (`utxo_…`, up while unspent, down once spent) stay readable. The raw transaction results of the run are saved in `evidence/indexer-results/` right after each step, and the web demo links each archived result when the indexer no longer serves it.

The web demo reads the indexer without verifying proofs. It shows what the indexer reports and labels it as such.

## Out of scope for this build

Wallet compromise, indexer availability, and the cryptographic soundness of the Ootle engine itself.
