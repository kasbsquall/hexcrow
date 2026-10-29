# Proof

Everything below ran on Tari Ootle **Esmeralda testnet** on 29 October 2026 and can be checked against the public indexer `https://ootle-indexer-a.tari.com/`. The raw log of every submission, including the dry-run verdict and the fee, is [evidence/esmeralda-run.jsonl](evidence/esmeralda-run.jsonl); this run is the part below the note that introduces template v3.

## Check it in one minute

The current state of the invoice book (status per invoice):

```bash
curl -s https://ootle-indexer-a.tari.com/substates/component_40bf618048f814d859011d5d13a7cc20c978c917841571bb0a1bce2b52263931
```

In `state.up.substate.Component.body.state[0].entries`, each entry is `[invoice, [invoice_hash, contractor_key, client_key, deadline_epoch, claim_until, approval_lock, escrow_root, approval_code, [status, []]]]`. Status is an index: 0 issued, 1 funded, 2 approved, 3 settled, 4 refunded. Invoice 1 reads 3, invoice 2 reads 4, invoice 3 reads 3. There is no amount anywhere in it.

Each escrow output is a substate too. While unspent it shows `"auth": {"Script": "<condition root>"}`, the root recorded in the book, and no key; once spent the indexer answers that it is down:

```bash
curl -s https://ootle-indexer-a.tari.com/substates/utxo_0101010101010101010101010101010101010101010101010101010101010101_<escrow commitment>
```

The verdict of any transaction:

```bash
curl -s https://ootle-indexer-a.tari.com/transactions/<id>/result
```

Under `result.Finalized`: `final_decision` is `Commit` or `{"Abort": "ExecutionFailure"}`, and `execution_result.finalize.result` is `Accept`, `AcceptFeeRejectRest` (the main instructions were rejected and only the fee was taken), or `Reject` with the reason. After a while, a transaction the chain rejected may come back in a short form instead, `{"result": {"Rejected": {"details": "<reason>", "rejected_time": "<time>"}}}`, with the same reason.

**Transaction results expire.** The indexers advertise a 50-epoch retention, but on 29 October they dropped results in fewer than 10 epochs. If a `transactions/<id>/result` call answers 404, the result as the indexer served it right after the run is in [evidence/indexer-results/](evidence/indexer-results/)`<id>.json`. The book and the escrow substates above do not expire.

## Deployment

| What | Id | Transaction |
|---|---|---|
| Template `hexcrow_book` v3 | `template_0b82e0dbc489b2a8d21897945b2b4ba69dc49d0b3af6e1314a80853a9ca9f754` | `436aba85f70286be129de19bc5d23cf3c6f6a6017272d965338e52dfaa6624bc` |
| Invoice book | `component_40bf618048f814d859011d5d13a7cc20c978c917841571bb0a1bce2b52263931` | `453885e5cea4f176e249e58d7e46942eae454810d790746b9bd4c37c98487c83` |

## Invoice 1: funded, checked, attacked, approved, settled

Approve before epoch 12716, claim before epoch 12718. Its own keys: client `d65a4df4476c26a8a58522aee98d4b968a2bad883809508730314107a7586703`, contractor `6e8f9250d7ca1b1364ae74f9c23d46172073e718292da9d292d3e65a4da6064e`. Escrow commitment `c4cbab7000d70b2cd1e5f4565cb9356a7acf11361fbb0205f1b94fbadc834828`, condition root `65d241226986ab7bc2b090ad5a17c98cd961a7fc61621497c53faecf92438678`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue, no approval lock | contractor | `7df3988299bc7d13f8cafe15ffde91458562621227fe259717f5d8a559d07872` | `Accept` |
| 2 | Attack: the contractor funds with a code it chose | contractor | `22cab3e81c433605d1795924a7ee871e41e215e940b1fe40de5683fcbe3585af` | `AcceptFeeRejectRest`: the book asked for the client's signature, the fee was taken |
| 3 | Fund, 250 tTARI into the escrow, approval lock set by the client | client | `f2aa03863ff199469bb0514633dff86a112e252b6034fbe5c6468c4a4de777bb` | `Accept` |
| 4 | Attack: a stranger claims | stranger | `0e06157117bfc09f8c84d4ea750e7918dcdaaaf327b9117c23eb78b2a6384f43` | `Reject`, `AccessDenied: native.stealth_utxo.spend` |
| 5 | Attack: claim without the approval code | contractor | `3868016e026bfb2a3c28e444f066e9d7b494a4a23ae839588eb5b8da29484dd6` | `Reject`, `HashLock` predicate not satisfied |
| 6 | Attack: refund before the deadline | client | `0c8fd4c7991ec0d22b5367efe6e344eded2a16fea333e18b144302bfb330ae5b` | `Reject`, `AfterEpoch(12718)` not satisfied |
| 7 | Attack: the contractor approves its own invoice | contractor | `5cf4e8df18a0d2c846d56384c79812bbd721a20a1258d58e8e3d372a71b462aa` | `AcceptFeeRejectRest`: the book refused the signer, the fee was taken |
| 8 | Approve, approval code revealed | client | `6364f810154006dff341b4e0238ddaf19364a25b1cf1a40eb4e01424a6e3cb93` | `Accept` |
| 9 | Attack: the client refunds after approving | client | `d7daf43fa07a5da5494ae6fef45f608cb9810dbef00121b5a393028ccb675dba` | `Reject`, `AfterEpoch(12718)` not satisfied |
| 10 | Attack: a stranger claims with the now-public code | stranger | `cb742b3ae96baf966f91609a582e74324c40f594b9be1cfa1bb3d94be29495c2` | `Reject`, `AccessDenied: native.stealth_utxo.spend` |
| 11 | Claim and settle | contractor (co-signing with the invoice's escrow key) | `f1907e76195027d64a3511e1efed38c683279aa2553c247a4803d4c7e4b1884e` | `Accept` |

Between steps 3 and 4 the contractor ran `hexcrow verify 1`, a read-only check recorded in the run log as `verify:1`: the book names its key, the invoice is funded, the book's terms rebuild the book's condition root, and the escrow output exists unspent, locked by exactly that root. In transaction 11 the spent input is the escrow commitment above, and its witness shows the claim leaf the engine checked: `HashLock`, `BeforeEpoch(12718)` and the contractor's key.

## Invoice 2: funded, checked, never approved, refunded

Approve before epoch 12714, claim before epoch 12715. Its own keys: client `fab713aecd977cda31a51d5322cefc347ea5e6a98b7b670e7dcf3f20e971f94d`, contractor `c6ad28a9dc1999c576674cc28be6c812af0106baa61580f519de8aadd8cd6054`, none shared with invoice 1. Escrow commitment `2cf9e45f9a4874bc37286625b95a6b821b8abcf6753d65e1975b60ff3f12835d`, condition root `98daeb395a0e52a48ba24d03e7abb11d452f37f5bbed2407fe51cca76f8cef44`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `cbf10cb3dfba251dab4b5c53d34a8ab10f3f89f314bf24ab02844ea8f0278dc3` | `Accept` |
| 2 | Fund, 40 tTARI into the escrow | client | `ac3cda54d1dcc33e4bac28e02eb7c6ce160a9ec36e812fca45d77dda7c102e45` | `Accept` |
| 3 | Refund from epoch 12715 | client (co-signing with the invoice's escrow key) | `19ecd21f22dd1bb9952c86b03d1037502f600e60debf8f78a0d5da41db2396f7` | `Accept` |

The contractor ran `hexcrow verify 2` after funding, recorded as `verify:2`. The first run failed the lock check because the CLI misread how the indexer returns the escrow's script; after that fix, the second run passed every check.

## Invoice 3: the next milestone, checked with the amount

The third milestone of the project in invoice 1, with its own keys: client `563f915de34d02e83c3fbf013fb13cbaeb9a5874e346629f0e7993e34cf65d28`, contractor `7c552b58f1daf642b83e420ab181cd14e24a246298ff12b5a7cc9efa6b130970`. Nothing on chain links them to invoice 1. Approve before epoch 12716, claim before epoch 12718. Escrow commitment `56c82d2f76f7f896b8cf272c1dd95aa6370428f394ec405d29560a6b2f6d1c15`, condition root `2c385b31e39438f3b46c29c53a90f4e629eca2612d30bfb77951ebe1e1d33fda`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `b5a2e42e5296b144c0b801d93c5e60c61bde0f89f5f570094ce4708592243561` | `Accept` |
| 2 | Fund, 60 tTARI into the escrow | client | `29b8e91e52b08c902e719ad2271cc5cc86772210f4478c908f78f3c150f5f37a` | `Accept` |
| 3 | Approve | client | `e8bd4e970f6f2c64dd2438ab6ac5be05c183098acc2862951f77745174ea0917` | `Accept` |
| 4 | Claim and settle | contractor (co-signing with the invoice's escrow key) | `04ba6480d1be5beddfa6ef9d63788e4e516e81344175291ab0c3b1a3522801c2` | `Accept` |

Between steps 2 and 3 the contractor ran `hexcrow verify 3` twice, both recorded as `verify:3`. Told to expect 600 tTARI, the amount check failed: the escrow, decrypted with the invoice's escrow view key, holds 60. Told to expect 60, every check passed, including that the client could still approve and that the escrow output, read through a verified proof, is locked by exactly the book's condition root with no key path.

## The accountant's check

For invoice 1, `sha256("93aa97ceec4e7ea06b7834cc63d8e5f3|Invoice 0001 | Brand identity for a privacy wallet, milestone 2 of 3: logo system and icon set | 250 tTARI")` equals the invoice hash stored in the book, `13f9088b5ec1302cd969fb23e22a000a244bba871454b18e26d22801d1261306`. The hosted demo computes this in your browser.

```bash
printf '%s' '93aa97ceec4e7ea06b7834cc63d8e5f3|Invoice 0001 | Brand identity for a privacy wallet, milestone 2 of 3: logo system and icon set | 250 tTARI' | sha256sum
```

## Engine tests

15 tests run the compiled template in `tari_template_test_tooling`, the engine the network uses: [evidence/local-engine-tests.txt](evidence/local-engine-tests.txt). They cover the happy path, the refund only after the claim window, a stranger on every method, the contractor approving its own invoice, the contractor trying to set the approval lock, a wrong approval code, settling without approval, approving twice, refunding an approved invoice, an approval in the last epoch that still leaves the claim window, no approval after the deadline and no settle after the claim window, funding after the deadline, past deadlines, zero grace and self-dealing at issue, the absence of any amount in the book's state, and the co-signed settle.

## History, for completeness

Two earlier templates ran on Esmeralda on the same day, and their entries stay in the run log above the notes that mark each switch.

- **v1** (`template_aecbb45e…`, book `component_a3dba764…`) checked only the sealing signer. On Esmeralda the escrow spend has to be sealed by the escrow key, because the SDK decrypts inputs with the wallet's default key, so the acting party co-signs. One v1 escrow of 250 tTARI stays locked until its refund leaf opens at epoch 12735.
- **v2** (`template_87eb6ce8…`, book `component_ae712c5f…`) settled one invoice and refunded another, but a review found that the contractor chose the approval lock, that an approval in the last epoch did not protect the contractor, and that one escrow key served every invoice. [THREAT_MODEL.md](THREAT_MODEL.md) describes the flaws and the fixes in v3.

## Built during the contest

All code was written in October 2026. The commit history of this repository starts on 29 October 2026.
