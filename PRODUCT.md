# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

delegated: static HTML, CSS and vanilla JS, no build step. The demo is hosted as static files and reads the public Esmeralda indexer (https://ootle-indexer-a.tari.com/) from the browser. A judge must be able to use it without a wallet, an account or an install.

## Users

Primary: an independent contractor (designer, developer, writer) who bills a company per milestone and wants to know the money is there before starting, without publishing what they earn. Second: the company paying that contractor, which wants to release money only on approval and get it back if work never arrives. Third: an accountant or auditor who must check what was paid without being able to move it.

Evaluating audience for this build: the Tari Council (Fox, ZhaoXi, solix78, naveen, kinkajou, coin_artist), a global privacy-focused community voting the October "Spooky Secrets" build contest. solix78 asked on 25-oct for confidential invoicing, private contractor payments, milestone escrow and selective disclosure.

## Product Purpose

Hexcrow is a confidential milestone escrow on Tari Ootle. The contractor issues an invoice, the client locks the payment in an escrow nobody else can read, the client approves the delivered milestone with a code, and only then the contractor can claim. If the deadline passes without approval, only the client can take the money back. Success: one invoice can be followed end to end on the public chain while the amount, the parties' accounts and the invoice text stay hidden from everyone except the people entitled to see them.

## Positioning

The escrow is a stealth (confidential) output whose spending rules are enforced natively by the Ootle engine: claim needs the approval code, the contractor's one-time key and a time before the deadline; refund needs the client's key and a time after the deadline. No contract custody, no keyed path. A public invoice book records only hashes, one-time keys, the deadline and the status. On a transparent chain the same escrow would publish the price, the payer and the payee.

## Operating Context

The key screen shows one real invoice from a recorded run on Esmeralda testnet, seen by four actors who see different things:
- Public (anyone): invoice hash, deadline epoch, status, escrow commitment and condition root, transaction ids. No amount, no names.
- Client: everything public plus the amount, the approval code it holds, and its actions (fund, approve, refund after deadline).
- Contractor: everything public plus the amount it is owed and its action (claim after approval).
- Accountant: given the invoice salt and document, recomputes the invoice hash and sees the amount through the escrow view key, but cannot spend.

The run includes attacks the chain rejected, each with its real reason: a stranger trying to claim (AccessDenied), a claim without the approval code (hash lock not satisfied), a refund before the deadline (AfterEpoch), the contractor approving its own invoice (signer check), and a stranger claiming with the approval code already public (AccessDenied).

## Capabilities and Constraints

- Real: template `template_87eb6ce88e3654e1944867edc80edf7a51bee7787e148431ec2ddab9cd893e1c`, book `component_ae712c5fbd92accadfacbb8e867b5779bb25cad9434d3d88787d43553a459353`, invoice 1 issued, funded with 250 tTARI, approved, claimed and settled; invoice 2 issued and funded with 40 tTARI, waiting for its deadline to be refunded.
- Amounts are testnet tTARI (µT internally, 1 tTARI = 1,000,000 µT). Epochs are long on Esmeralda, so deadlines are counted in epochs, not hours.
- Undecided: whether a live Sapient wallet mode ships (bonus track); the guided mode over the recorded run must work without it.
- Terminology: invoice, milestone, escrow, approval code, deadline epoch, claim, refund, settle. Avoid protocol jargon in headlines (no "MAST", "UTXO", "HTLC" above the fold).

## Brand Commitments

Name: Hexcrow. Contest theme: "Spooky Secrets" (October, Halloween). Voice: plain, exact, no hype words, no emojis. Public repo, MIT licence: fonts and sounds must be OFL or CC0.

## Evidence on Hand

- `evidence/esmeralda-run.jsonl`: every submitted transaction with id, signer persona, verdict, fee and dry-run result.
- `evidence/local-engine-tests.txt`: 12 engine tests passing.
- `hexcrow inspect <n>`: per-actor JSON for each invoice.
- No users, testimonials or adoption numbers exist. Do not invent any.
