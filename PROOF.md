# Proof

Everything below ran on Tari Ootle **Esmeralda testnet** on 4 November 2026, between 16:11 and 19:00 UTC, after the network reset of 2 November. It can be checked against the public indexer `https://ootle-indexer-a.tari.com/`. The raw log of every submission, including the dry-run verdict and the fee, is [evidence/esmeralda-run.jsonl](evidence/esmeralda-run.jsonl); this run is everything below its last `note` entry.

The rules are the same as in the 29 October run. The changes are only what the reset network required: ootle-rs 0.31 and tari_template_lib 0.37, the signer check in the book written with the new badge API, and readers for the indexer's new encoding (0.50 serves substates and results as base64 CBOR instead of JSON). The 29 October run, whose ids the reset erased, is kept in [PROOF-2026-10-29.md](PROOF-2026-10-29.md).

## Check it in one minute

The current state of the invoice book. Since indexer 0.50 the substate is base64 CBOR; this decodes it (`pip install cbor2`) and prints each invoice's number, status and whether it carries a ruling:

```bash
curl -s https://ootle-indexer-a.tari.com/substates/component_e668bbfecc2f967a9f00980d93dc792662ec42085b9da2c84b078d391f886e9c \
| python -c "import sys,json,base64,cbor2; b=cbor2.loads(base64.b64decode(json.load(sys.stdin)['state']['up']['substate'])); [print(n, f[8][0], 'ruling' if len(f) > 10 and f[10] else '-') for n, f in b[1][0][1][0][0].items()]"
```

Each entry is `[invoice_hash, contractor_key, client_key, deadline_epoch, claim_until, approval_lock, escrow_root, approval_code, [status, []], mediator_key, ruling_hash]`. `approval_code` is empty until the client approves, and `ruling_hash` is present only once a mediator has ruled. Status is an index: 0 issued, 1 funded, 2 approved, 3 settled, 4 refunded. Read after the run: invoice 1 is 3; invoice 2 is 3, with a ruling; invoice 3 is 4, with a ruling; invoice 4 is 4. There is no amount anywhere in it.

Each escrow output is a substate too. While unspent its authorization is a script whose root is the one recorded in the book, with no key; once spent the indexer answers 404:

```bash
curl -s https://ootle-indexer-a.tari.com/substates/utxo_0101010101010101010101010101010101010101010101010101010101010101_<escrow commitment>
```

The verdict of any transaction:

```bash
curl -s https://ootle-indexer-a.tari.com/transactions/<id>/result
curl -s https://ootle-indexer-a.tari.com/transactions/<id>
```

The first answer is the full result, under `result.Finalized` (`final_decision` and a base64 CBOR `execution_result` whose transaction result is `Accept`, `AcceptFeeRejectRest` or `Reject`), or, for a rejected transaction, the short form `{"result": {"Rejected": {"details": "<reason>", ...}}}`. **It expires:** on 4 November the indexer had dropped the full result of every committed transaction within three hours. The second answer, the transaction record, stays: its `summary.outcome` is `Commit` for an accepted transaction or `FeeIntentCommit` when the instructions were refused and only the fee was taken, and a rejected transaction has no summary and a `rejected_reason`. Everything either call returned after the run is saved in [evidence/indexer-results/](evidence/indexer-results/), as `<id>.json` for a full result and `<id>.tx.json` for a record. The book and the escrow substates do not expire.

## Deployment

| What | Id | Transaction |
|---|---|---|
| Test funds: 1,000 tTARI burned on the Esmeralda L1 (transaction 7347606474537143485, block 1028080), claimed on the Ootle | | `1ea39faf9859879ef6badd44035a50af4b82a63718eb9743b2b09cc7b304441c` |
| Template `hexcrow_book` v4 | `template_666ec8baf770061d128c3c0caa9af00a9a0ec70370cb7a820c994c161d9dcd50` | `59cb2aeab1803a046991a9cb0465231474ff9c19be81f8f541e57bf6016fa78f` |
| Invoice book | `component_e668bbfecc2f967a9f00980d93dc792662ec42085b9da2c84b078d391f886e9c` | `027b73c6a8bc246ecaf9220fb01ac44ab854ee062708b7459f47df7b6cb909ae` |

The network faucet was empty after the reset, so the funds came from the L1 burn; the `give:` entries in the run log split them among the run's one-time keys.

## Invoice 1: funded, checked, attacked, approved, claimed

Approve before epoch 12862, claim before epoch 12864. Its own keys: client `02ac3fe2e2aab698b7624d625b9bf22e99cdaeb070bfd7c321a73add8409d16e`, contractor `90ef869539dc95e5bbb1080b012fe7806dfe0b5fbc6913fc8a7a03dfba27c11d`, mediator `e610e80c6ebfdf47b3b249d23531473162c522bca2d3a47571d7e3efa0266e76`. Escrow commitment `120eaa44ddb8d184a16338345a07844d8bd4e9d5696d0674b569b5fde05d6c20`, condition root `c57e59070050876e3c482f45d9894743cff3086fb4cc4689745bf635a4f913b3`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue, naming the mediator, no approval lock | contractor | `d036ff413a33e756f4c439f827b790503b67bcf4659aba6a4d48207e202bbf3a` | `Accept` |
| 2 | Attack: the contractor funds with a code it chose | contractor | `c2f3d685b63541605f346bedeebf85132dde7d93168457d4345b063a59b7b989` | `AcceptFeeRejectRest`: the call did not carry the client's badge, the fee was taken |
| 3 | Fund, 250 tTARI into the escrow, approval lock set by the client | client | `9cbee861ce013380bc5a329223f783506f24974a05738f739d3913764a184f13` | `Accept` |
| 4 | Attack: a stranger claims | stranger | `775626899a8b5892ec10b7f5c9bfe08f005cdddf3eb3f2f7ec9f1970b517e4a6` | `Reject`, `Access Denied: native.stealth_utxo.spend` |
| 5 | Attack: claim without the approval code | contractor | `d14cff6c2b495145501914011c7aff041a8e67f2500b77d4c6b8430af298eff5` | `Reject`, `HashLock` predicate not satisfied |
| 6 | Attack: refund before the claim window closes | client | `d311afb0f40f142f903ebf1f2bc4b10eb687807847ca6f63f31f0b0868a7b062` | `Reject`, `AfterEpoch(12864)` not satisfied |
| 7 | Attack: the contractor approves its own invoice | contractor | `2ad70c433714a65c19bc7e627c19a004dbaa2fce7dd78150d22646389290c9f6` | `AcceptFeeRejectRest`: the call did not carry the client's badge, the fee was taken |
| 8 | Attack: the mediator alone spends the ruling leaf for the contractor, paying itself | mediator | `201e585b409176e5d993babb7f71525ae7574298bf9ee95bfdff12ae63b2359f` | `Reject`, `Access Denied: native.stealth_utxo.spend` |
| 9 | Attack: the contractor spends the same ruling leaf without the mediator | contractor | `f6c4cd4640b46431d3c112140bc3ab102a1e20d78ab1549fd13cfa7bd07018a5` | `Reject`, `Access Denied: native.stealth_utxo.spend` |
| 10 | Approve, approval code revealed | client | `9d6bf830248401e98960953402ae04670481f8b12b2fd3e8d9bc152179fc6b48` | `Accept` |
| 11 | Attack: the client refunds after approving | client | `01b09f7b0ad6b9c3eb5bea233c62d1888cf53e6f2d4a096b057d41e45f764b35` | `Reject`, `AfterEpoch(12864)` not satisfied |
| 12 | Attack: a stranger claims with the now-public code | stranger | `0d7362a21270cc574570b8f0fc88bd17fd4401e737a2565b0ba6881a07e76916` | `Reject`, `Access Denied: native.stealth_utxo.spend` |
| 13 | Claim and settle | contractor (co-signing with the invoice's escrow key) | `95d1fded20a1a44a69282fd0823a51807a36ce44188ddcd2d2d02174a3600df2` | `Accept` |

Between steps 3 and 4 the contractor ran `hexcrow verify 1 --expect-tari 250`, recorded as `verify:1`, and every check passed: the book names its key and the agreed mediator, the invoice is funded, the client can still approve (epoch 12858, deadline 12862), the book's terms rebuild the book's condition root, the escrow output exists unspent, read through a verified proof, locked by exactly that root with no key path, and it holds 250 tTARI, decrypted with the invoice's escrow view key.

## Invoice 2: the client went silent, the mediator ruled for the contractor

The last milestone of the project in invoice 1, with its own keys: client `bc25bfdada0efabb03f029c2f5da78b1fe818557ba67843df53cf39117887e5a`, contractor `44904d9bdfccfd01c865eb581c0d1ba146a70d77958c45c247bda4e0946e826b`, mediator `28d82b22afa753c5db6caf716b9e0d63df0359c6d03f145007f5c9033de55869`. Nothing on chain links them to invoice 1. Approve before epoch 12860, claim before epoch 12862. Escrow commitment `3450f114f28f9c38d5b5408e1cc57db4083a5250b37548b9d07b5b31f430cd5f`, condition root `2194dbb17db53d92a8f95270ecca0e80e415fab2765697a1b9bf62b79693b8e6`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `29d2eb1f6d4a96b9ba3970dc99d8dbaf075d1ff54a1ffdea0e68f661e2fa7f7c` | `Accept` |
| 2 | Fund, 120 tTARI into the escrow | client | `9a80740061cdac7317a6d712dab8571cc15ffbc29b85c184bc6a4613a1a40fc6` | `Accept` |
| 3 | Ruling for the contractor, in epoch 12860, once the deadline had passed without an approval | mediator and contractor (co-signing with the invoice's escrow key) | `95efb87b984225a73492402327d290e059587a9c9045f4782e3a2da68bfe3872` | `Accept` |

The contractor ran `hexcrow verify 2` after funding, recorded as `verify:2`, and every check passed. The book records the ruling's hash, `f5625c4289b5dbeb14d7f4ab329e7491529174e48e50c55d4d565c0abdb60258`; this time its salt and text were kept, and the demo checks the hash in the browser.

## Invoice 3: the contractor withdrew, the mediator ruled for the client

Its own keys: client `4e6b26fdd97209744219b7657c139b345bdbf991614e0e5fc23f150b24546d03`, contractor `4038bf5ebd074d63ea4eaf72065e409ba66420b71c2dde290752feb9e53b1e06`, mediator `627969dc33093b259f18a79a72128102f4b48893813b521dfb0c9690746d0861`. Approve before epoch 12862, claim before epoch 12864. Escrow commitment `d26483a876055ef97d62c5950a51220137ab1855d8e370ef03411cc0171e8d06`, condition root `6acc9f5c5a6e52a5cd87aca8150785f92c47913d455f97c90d791b45ac00370a`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `9b3fff10f7996a4e3848478fc7c21f03fa5978fa0d899d812143586fb30ece50` | `Accept` |
| 2 | Fund, 60 tTARI into the escrow | client | `f3c9c3090e67f15d157cb7029a5403f35e50a426ebc90da21c1d4debbbda8ff6` | `Accept` |
| 3 | Ruling for the client, in epoch 12858, six epochs before the refund leaf opens | mediator and client (co-signing with the invoice's escrow key) | `fa4a4f17f7560026c960eda7e3578356e9cd91c39194a1c1e32940d48905ba43` | `Accept` |

The contractor ran `hexcrow verify 3` after funding, recorded as `verify:3`, and every check passed. The book now marks invoice 3 refunded and stores the ruling's hash, `15b1b861f3fb0053299f18917130985da423adcc0d62ec891084c1a38cfe8871`.

## Invoice 4: nobody acted, a late ruling was refused, refunded

Its own keys: client `40fd6bedaf3aad5dd8db6fad1f00d47d8d0a11b15e4fbdaff411bd9c1fb2f06b`, contractor `184b3cf23c29f5142314d8ad343f33b8b28e6cfef6c16a43fc9f6dd3786be279`, mediator `b05cab577ea9d103a5e9eefea3f31e9e9af9ce04d8ed3fcc89d39ff1dc1cc316`. Approve before epoch 12860, claim before epoch 12861. Escrow commitment `28535382e2784259cdc96fa307f62303c8007bc2a5cdab972ec0115b15b01678`, condition root `1745b7803c32e65bcdd257ed5774a708fa9c0fa2b99ac74ec924603830ee1ba0`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `2fa088da9a4f4e35d4a2649d1452f84c9b90da90e3e0d905e89c8c4a26db0254` | `Accept` |
| 2 | Fund, 40 tTARI into the escrow | client | `fd5aeebdd0e4ad0b3d93ef0abc8dae84aab07efa47378e4e3bfcd940d17281ec` | `Accept` |
| 3 | Attack: the mediator and the contractor rule after the claim window closed | mediator and contractor | `22705306fbad9012348af20d03368b4e8333671268bae4d1c333da8ead992cf1` | `Reject`, `BeforeEpoch(12861)` not satisfied |
| 4 | Refund from epoch 12861 | client (co-signing with the invoice's escrow key) | `a61d38d92e53efb6004d8a55c2f6494a4e748b497c30a80706608149729056b3` | `Accept` |

The contractor ran `hexcrow verify 4` after funding, recorded as `verify:4`, and every check passed.

## The accountant's check

For invoice 1, `sha256("d509099ea4b433a3196d3163a25f989c|Invoice 0001 | Brand identity for a privacy wallet, milestone 2 of 3: logo system and icon set | 250 tTARI")` equals the invoice hash stored in the book, `6109cfd03fb84f5c155fc6cf02d0935da80d92cf78f80eb1b4e48a3e77e8d637`. The hosted demo computes this in your browser.

```bash
printf '%s' 'd509099ea4b433a3196d3163a25f989c|Invoice 0001 | Brand identity for a privacy wallet, milestone 2 of 3: logo system and icon set | 250 tTARI' | sha256sum
```

## The mediator's check

A ruling works the same way. For invoice 3, the ruling text and its salt hash to the `ruling_hash` the book stores, `15b1b861f3fb0053299f18917130985da423adcc0d62ec891084c1a38cfe8871`. The mediator's copy in the hosted demo computes this in your browser, for invoice 2 as well.

```bash
printf '%s' 'b23fe7407e6be3ce4c5a2ba03cdf2f01|Ruling on invoice 0003. The contractor withdrew from the project on 4 November 2026 and will not deliver the storyboard or the voice-over. The escrow is returned to the client now, without waiting for the claim window to close.' | sha256sum
```

## Engine tests

19 tests run the compiled template in `tari_template_test_tooling`, the engine the network uses, rerun on the post-reset engine: [evidence/local-engine-tests.txt](evidence/local-engine-tests.txt). They cover the happy path, the refund only after the claim window, a stranger on every method, the contractor approving its own invoice, the contractor trying to set the approval lock, a wrong approval code, settling without approval, approving twice, refunding an approved invoice, an approval in the last epoch that still leaves the claim window, no approval after the deadline and no settle after the claim window, funding after the deadline, past deadlines, zero grace, self-dealing at issue and a mediator who is one of the parties, the absence of any amount in the book's state, and the co-signed settle. Four cover the mediator: a ruling for the contractor, a ruling for the client, seven signer combinations that must fail because the mediator or the paid side did not sign, and no ruling after an approval or after the claim window.

## History, for completeness

Before the reset, four templates ran on Esmeralda on 29 October: v1, v2 and v3, then the v4 run in [PROOF-2026-10-29.md](PROOF-2026-10-29.md), which this run repeats on the new network. [THREAT_MODEL.md](THREAT_MODEL.md) describes the flaws earlier reviews found and the fixes.

## Built during the contest

All code was written in October 2026, and the changes for the reset in November. The commit history of this repository starts on 29 October 2026.
