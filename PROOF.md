# Proof

Everything below ran on Tari Ootle **Esmeralda testnet** on 29 October 2026 and can be checked against the public indexer `https://ootle-indexer-a.tari.com/`. The raw log of every submission, including the dry-run verdict and the fee, is [evidence/esmeralda-run.jsonl](evidence/esmeralda-run.jsonl).

## Check it in one minute

The current state of the invoice book (status per invoice):

```bash
curl -s https://ootle-indexer-a.tari.com/substates/component_ae712c5fbd92accadfacbb8e867b5779bb25cad9434d3d88787d43553a459353
```

In `state.up.substate.Component.body.state[0].entries`, each entry is `[invoice, [invoice_hash, contractor_key, client_key, approval_lock, deadline_epoch, escrow_root, approval_code, [status, []]]]`. Status is an index: 0 issued, 1 funded, 2 approved, 3 settled, 4 refunded. Invoice 1 reads 3, invoice 2 reads 4. There is no amount anywhere in it.

The verdict of any transaction:

```bash
curl -s https://ootle-indexer-a.tari.com/transactions/<id>/result
```

Under `result.Finalized`: `final_decision` is `Commit` or `{"Abort": "ExecutionFailure"}`, and `execution_result.finalize.result` is `Accept`, `AcceptFeeRejectRest` (the main instructions were rejected and only the fee was taken), or `Reject` with the reason.

The indexer keeps transactions for 50 epochs (`transaction_retention_epochs` in `https://ootle-indexer-a.tari.com/info`). The book state above does not expire.

## Deployment

| What | Id | Transaction |
|---|---|---|
| Template `hexcrow_book` v2 | `template_87eb6ce88e3654e1944867edc80edf7a51bee7787e148431ec2ddab9cd893e1c` | `5327c8c89b3e7d9e9a0eb7d06935a11a562cd745653265f4b82764236eabc89c` |
| Invoice book | `component_ae712c5fbd92accadfacbb8e867b5779bb25cad9434d3d88787d43553a459353` | `106fb475576b7ecc3fc1e524f55577c47b0383d11bf143849316419cd4782e3a` |

## Invoice 1: funded, attacked, approved, settled

Deadline epoch 12735. Escrow commitment `c262f8b876f5faed9df5b1214ece679b9c93f6390af1f8200c91230a285b7203`, condition root `0e63671c1553a8bf41e97572d7d59140e781da7ad15af0004ec2599c66f983f7`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `244bfc910b85799402ce5aedd66f62f27078c777acaa7aa8d9cc3c39e8066b2c` | `Accept` |
| 2 | Fund, 250 tTARI into the escrow | client | `f23563ce1bee6c90891f48f6ad64a74fe95ce382be864fe63bf914a6eebe1dce` | `Accept` |
| 3 | Attack: a stranger claims | stranger | `1bfb05471e2c79f93a319d504f67e03465c4439aa8252097ed02d80e4310e717` | `Reject`, `AccessDenied: native.stealth_utxo.spend` |
| 4 | Attack: claim without the approval code | contractor | `bd58de7e876a8a8986df542ab59e42e2817b4aece84fb9c953da96ce180dc697` | `Reject`, `HashLock` predicate not satisfied |
| 5 | Attack: refund before the deadline | client | `08bb2af75a2aab76b89d6abba2049a0c41a6ca26e20a075234a46fe38f5a3a9a` | `Reject`, `AfterEpoch(12735)` not satisfied |
| 6 | Attack: the contractor approves its own invoice | contractor | `db6d41e969a713d3e9e361475388abf1dd33074af662a865dabcc857e104e126` | `AcceptFeeRejectRest`: the book refused the signer, the fee was taken |
| 7 | Approve, approval code revealed | client | `e1b0df7d7783b052a21c3bd7b26b41453cdf73e52c70faa5d5051d1ec9024095` | `Accept` |
| 8 | Attack: a stranger claims with the now-public code | stranger | `c43ae82cc50ea9e85f4f374f39dd23ca106f713abd8911e6a4a14c7ad7021eb0` | `Reject`, `AccessDenied: native.stealth_utxo.spend` |
| 9 | Claim and settle | contractor (co-signing with the escrow key) | `dd33a6e01c886ffb1aac6d25ff15894361d35c174ddaa3cabc900f6bb91b458a` | `Accept` |

In transaction 9 the spent input is the escrow commitment above, and its witness shows the claim leaf the engine checked: `HashLock`, `BeforeEpoch(12735)` and the contractor's key.

## Invoice 2: funded, never approved, refunded

Deadline epoch 12706. Escrow commitment `36b9ab8397078e223ca0f385a0ccf72c5770daeee8e15d9139338aa2d9f16a00`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `bd59acdf6c8b00637103e34913462c4afd63aef1d61514c5afe6f05ef7c25ac7` | `Accept` |
| 2 | Fund, 40 tTARI into the escrow | client | `26e0116722f7acb641d48447fe917e2534fd87ef55d48876f4979b97cc0cd08f` | `Accept` |
| 3 | Refund after epoch 12706 | client (co-signing with the escrow key) | `4532b675c4eb558ca78f09e95f5b15679579e45d4959d9082507a8f30f557741` | `Accept` |

## The accountant's check

For invoice 1, `sha256("92c4ea0d49897830b88e53b2041e9cd6|Invoice 0001 | Brand identity for a privacy wallet, milestone 2 of 3: logo system and icon set | 250 tTARI")` equals the invoice hash stored in the book, `7a4a0424e2371ecf3358bad08640a987d1b63336d7adbaa9d3e01ae2502198d5`. The hosted demo computes this in your browser.

```bash
printf '%s' '92c4ea0d49897830b88e53b2041e9cd6|Invoice 0001 | Brand identity for a privacy wallet, milestone 2 of 3: logo system and icon set | 250 tTARI' | sha256sum
```

## Engine tests

12 tests run the compiled template in `tari_template_test_tooling`, the engine the network uses: [evidence/local-engine-tests.txt](evidence/local-engine-tests.txt). They cover the happy path, the refund after the deadline, a stranger on every method, the contractor approving its own invoice, a wrong approval code, settling without approval, approving twice, refunding an approved invoice, funding after the deadline, past deadlines and self-dealing at issue, the absence of any amount in the book's state, and the co-signed settle.

## History, for completeness

A first version of the template (`template_aecbb45e…`, book `component_a3dba764…`) checked only the sealing signer. On Esmeralda the escrow spend has to be sealed by the invoice's escrow key, because the SDK decrypts inputs with the wallet's default key, so the acting party co-signs. Version 2 accepts the co-signer. The v1 entries are kept in the run log above a note that marks the switch. One v1 escrow of 250 tTARI is still locked and becomes refundable at epoch 12735.

## Built during the contest

All code was written in October 2026. The commit history of this repository starts on 29 October 2026.
