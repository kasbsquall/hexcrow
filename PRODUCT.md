# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

delegated: static HTML, CSS and vanilla JS, no build step. The demo is hosted as static files and reads the public Esmeralda indexer (https://ootle-indexer-a.tari.com/) from the browser. A judge must be able to use it without a wallet, an account or an install.

## Users

Primary: an independent contractor (designer, developer, writer) who bills a company per milestone and wants to know the money is there before starting, without publishing what they earn. Second: the company paying that contractor, which wants to release money only on approval and get it back if work never arrives. Third: a mediator both sides chose, who settles a dispute without being able to take the money. Fourth: an accountant or auditor who must check what was paid without being able to move it.

Evaluating audience for this build: the Tari Council and community voting the October "Spooky Secrets" build contest, a global privacy-focused group. A Council member asked on 25-oct for confidential invoicing, private contractor payments, milestone escrow and selective disclosure.

## Product Purpose

Hexcrow is a confidential milestone escrow on Tari Ootle. The contractor issues an invoice, the client locks the payment in an escrow nobody else can read, and the client's approval code releases it to the contractor. If they disagree, a mediator named when the invoice was issued can rule for either side, always together with that side's key and only while the claim window is open. If nobody acts, the client gets the money back once the window closes. Success: one invoice can be followed end to end on the public chain while the amount, the parties and the invoice text stay hidden from everyone except the people entitled to see them.

## Positioning

The escrow is a stealth (confidential) output whose spending rules are enforced natively by the Ootle engine, with four ways out and no key path: claim (approval code, contractor key, before the window closes), a ruling for the contractor (mediator and contractor keys, before it closes), a ruling for the client (mediator and client keys, before it closes), and refund (client key, after it closes). No contract custody. A public invoice book with no owner records only hashes, one-time keys, deadlines and status. On a transparent chain the same escrow would publish the price, the payer, the payee and the arbiter.

## Operating Context

The key screen shows one real invoice from a recorded run on Esmeralda testnet as five carbon copies, one per reader:
- Public (anyone): invoice hash, deadlines, status, one-time keys, escrow commitment and condition root, transaction ids. No amount, no names, no invoice text.
- Client: everything public plus the amount, the work, the approval code it holds, and its actions (fund, approve, refund after the window, co-sign a ruling for itself).
- Contractor: everything public plus the amount it is owed, the check it ran before starting work (`hexcrow verify`), and its actions (claim after approval, co-sign a ruling for itself).
- Mediator: blank until a party calls it in; then the invoice it was shown and its ruling, whose hash the book keeps.
- Accountant: given the invoice salt and text, recomputes the invoice hash in the browser and reads the amount through the escrow view key, but cannot spend.

The run has four invoices that left the escrow four different ways (claim, a ruling for each side, refund) and ten attacks the chain refused, each with its real reason, which a visitor can replay from the page.

## Capabilities and Constraints

- Real: template v4 `template_8eba2e85761d26e82117bc075236f9cae78e72919e7421bda33cae867d6b5fd3`, book `component_f427dbf5e39837919640366002669ccef31bc569897efd16ae6daadeb96372ba`, four invoices on Esmeralda on 29 October 2026. Every transaction is in `PROOF.md`.
- Amounts are testnet tTARI (µT internally, 1 tTARI = 1,000,000 µT). Epochs last about 70 minutes on Esmeralda, so deadlines are counted in epochs, not hours.
- The public indexers drop transaction results within hours; the page falls back to the raw results archived right after the run, and says so.
- Terminology: invoice, milestone, escrow, approval code, deadline epoch, claim window, claim, ruling, refund, settle. Avoid protocol jargon in headlines (no "MAST", "UTXO", "HTLC", "leaf" above the fold).

## Brand Commitments

Name: Hexcrow. Contest theme: "Spooky Secrets" (October, Halloween). Voice: plain, exact, no hype words, no emojis. Public repo, MIT licence: fonts and sounds must be OFL or CC0.

## Evidence on Hand

- `evidence/esmeralda-run.jsonl`: every submitted transaction with id, signer persona, verdict, fee and dry-run result.
- `evidence/indexer-results/`: the raw result of each transaction as the indexer served it.
- `evidence/local-engine-tests.txt`: 19 engine tests passing.
- `hexcrow inspect <n>`: per-reader JSON for each invoice.
- No users, testimonials or adoption numbers exist. Do not invent any.
