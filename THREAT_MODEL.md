# Threat model

What Hexcrow hides, what it does not, and what an attacker can still do. Every claim below points at code or at a transaction on Esmeralda testnet.

## What the chain enforces

The escrow is a stealth output paid to a condition tree with no key path (`cli/src/escrow.rs`). It can be spent in exactly two ways:

| Leaf | Conditions, all required |
|---|---|
| Claim | `HashLock(sha256(approval_code))`, `BeforeEpoch(deadline)`, signature of the contractor's key |
| Refund | `AfterEpoch(deadline)`, signature of the client's key |

Hexcrow has no admin key and the invoice book has no owner (`template/src/lib.rs`, `OwnerRule::None`). Nobody, including the Hexcrow author, can move an escrow outside these two leaves.

Attacks run against the real network on 29-oct-2026, all rejected (`evidence/esmeralda-run.jsonl`):

| Attempt | Rejected by | Transaction |
|---|---|---|
| A stranger claims | Engine: access denied on the stealth spend | 1bfb0547… |
| Contractor claims without the approval code | Engine: hash lock not satisfied | bd58de7e… |
| Client refunds before the deadline | Engine: `AfterEpoch(12735)` not satisfied | 08bb2af7… |
| Contractor approves its own invoice | Book: signer is not the client | db6d41e9… |
| A stranger claims with the approval code already public | Engine: access denied on the stealth spend | c43ae82c… |

## What stays hidden from the public

- The amount. It lives in a Pedersen commitment; only holders of the escrow view key decrypt it.
- The invoice text. The book stores `sha256(salt | text)`; without the salt the hash reveals nothing useful.
- Which long-lived accounts are involved, as long as each invoice uses fresh one-time keys (see the limit below).

## What leaks, by design or by limitation

1. **Timing and status.** Issue, fund, approve, claim and refund are public events with their epochs. Anyone can see that an invoice was funded and later settled or refunded.
2. **Fees.** Each transaction pays its fee through a revealed output (`cli/src/chain.rs`, `Spend::prepare`). Fee amounts are public, as on any Ootle transaction.
3. **The approval code becomes public on approval.** The book stores it so that the approval is auditable. This is safe because the claim leaf also needs the contractor's signature; the attack in c43ae82c… shows a stranger holding the public code still cannot claim.
4. **Key reuse in this demo.** The demo personas reuse the same client and contractor keys for invoices 1 and 2, so those two invoices are linkable to each other. A real deployment must derive fresh keys per invoice; the book only ever sees those one-time keys.
5. **The escrow view key is shared.** Both parties (and any accountant they hand it to) hold the per-invoice view key. Anyone who obtains it sees the amount. It cannot spend: the output has no key path.
6. **Fee intent is not atomic with the book call.** The stealth transfer that creates or spends the escrow rides in the fee intent, so it commits even if the book call in the same transaction fails (`OnlyFeeCommit`). The CLI refuses to submit a real step unless the dry run accepts the whole transaction (`cli/src/chain.rs`, `send`). A malicious client could still bypass the CLI and create an escrow that the book never records; the contractor should check the book shows `Funded` with the expected condition root before starting work.

## What Hexcrow does not solve

- **Disputes.** If the client never approves, the client gets the money back after the deadline even if the work was delivered. Hexcrow guarantees the money exists and follows the rules both parties signed up to; it does not judge the work.
- **Deadline granularity.** Deadlines are Ootle epochs. On Esmeralda an epoch lasted more than half an hour during our run, so short deadlines are coarse.
- **Spam.** The book is open: anyone can issue invoices against any client key. An invoice only matters to a client who chooses to fund it.

## Trust in the indexer

The CLI verifies what the indexer serves against the network's validator sets (`ootle-rs`, `IndexerTrust::Verify`). A fresh run anchors on the latest end-of-epoch checkpoint and then refuses inputs proven in an earlier epoch, which is exactly the case of an escrow refunded after its deadline. The first refund attempt failed that way on 29-oct-2026, right after the epoch moved from 12705 to 12706. The CLI now saves the trusted validator set after each transaction (`.hexcrow/trust.json`) and walks the signed handovers from it on the next run. `hexcrow anchor-trust <epoch>` anchors on an earlier checkpoint for a first run; like any first-use anchor, it takes the indexer at its word for that one checkpoint, and every later handover is checked against signatures. The refund of invoice 2 (4532b675…) went through after anchoring on the checkpoint ending epoch 12704.

The web demo reads the indexer without verifying proofs. It shows what the indexer reports and labels it as such.

## Out of scope for this build

Wallet compromise, indexer availability, and the cryptographic soundness of the Ootle engine itself.
