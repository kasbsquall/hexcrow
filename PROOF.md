# Proof

Everything below ran on Tari Ootle **Esmeralda testnet** on 29 October 2026 and can be checked against the public indexer `https://ootle-indexer-a.tari.com/`. The raw log of every submission, including the dry-run verdict and the fee, is [evidence/esmeralda-run.jsonl](evidence/esmeralda-run.jsonl). This run, on template v4, starts at the `faucet:contractor-1` entry recorded at 19:16:34 UTC, one line above the note that introduces v4; the note was written a few seconds after that first faucet claim.

## Check it in one minute

The current state of the invoice book (status per invoice):

```bash
curl -s https://ootle-indexer-a.tari.com/substates/component_f427dbf5e39837919640366002669ccef31bc569897efd16ae6daadeb96372ba
```

In `state.up.substate.Component.body.state[0].entries`, each entry is `[invoice, [invoice_hash, contractor_key, client_key, deadline_epoch, claim_until, approval_lock, escrow_root, approval_code, [status, []], mediator_key, ruling_hash]]`. `approval_code` is null until the client approves, and `ruling_hash` appears only once a mediator has ruled. Status is an index: 0 issued, 1 funded, 2 approved, 3 settled, 4 refunded. Invoice 1 reads 3; invoice 2 reads 3, with a ruling hash; invoice 3 reads 4, with a ruling hash; invoice 4 reads 4. There is no amount anywhere in it.

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
| Template `hexcrow_book` v4 | `template_8eba2e85761d26e82117bc075236f9cae78e72919e7421bda33cae867d6b5fd3` | `46817759d2b68b3c77cbdf9582ddbb2936f50f9ffb23850af2f91248828944a2` |
| Invoice book | `component_f427dbf5e39837919640366002669ccef31bc569897efd16ae6daadeb96372ba` | `0b8b7f82b6bcb3ddf74fed9a82e7874753164848a5bdc89cbea036dd50e4189a` |

## Invoice 1: funded, checked, attacked, approved, claimed

Approve before epoch 12720, claim before epoch 12722. Its own keys: client `7aa199f6a082cb88c9f8ac5ec678bff46f54a52ecbdbe6280bce6a9085ca7f59`, contractor `689c739b8d5ffe754376615dfcd2dfa784970f5f8eab138bc4422ca621e00c1b`, mediator `5a719dcee34e49f2a25edd8590d95c027cf3039c67fe427c33487957af622530`. Escrow commitment `ae57e0addccf5300c24e356147d27f8598b4520c6133c6092c9cc17fb0a4d24b`, condition root `6f4dc376aafc77567f622b97ede32cb82c42db1a7d973c31d88fc5beba4a2dc8`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue, naming the mediator, no approval lock | contractor | `20054eebb90806608ca03b48e0df54c03ce87b142eab07cb170681cf31ca4078` | `Accept` |
| 2 | Attack: the contractor funds with a code it chose | contractor | `a552319353b4b50a9ec3c070c3dac852b80f5782aa11ac2e404e9a27c1e7fdb5` | `AcceptFeeRejectRest`: the book asked for the client's signature, the fee was taken |
| 3 | Fund, 250 tTARI into the escrow, approval lock set by the client | client | `0406b243067a98e9e109e333b6cb4b5535f58247e98f10f9e7083ea0c6bd3186` | `Accept` |
| 4 | Attack: a stranger claims | stranger | `6e609c05baf4efa7b10306663b5e012da61b8b8406427f1027e4411494ac8651` | `Reject`, `AccessDenied: native.stealth_utxo.spend` |
| 5 | Attack: claim without the approval code | contractor | `c1951cdb45860173c063a488b2299c787a8b8ff977c40b9391c79683b65d5e8b` | `Reject`, `HashLock` predicate not satisfied |
| 6 | Attack: refund before the claim window closes | client | `a2de96f29edcdae044ffaf57e76c452ca831cedfcde4312bbadec504195e07b7` | `Reject`, `AfterEpoch(12722)` not satisfied |
| 7 | Attack: the contractor approves its own invoice | contractor | `ea2d059bc137057dd63afee267bc2ee3adf372ddf305fd93293755d0d5566557` | `AcceptFeeRejectRest`: the book refused the signer, the fee was taken |
| 8 | Attack: the mediator alone spends the ruling leaf for the contractor, paying itself | mediator | `c81bdfeb8124ec2475ea74eafb5f2e531594a27e7fa2a23022c194c78360b466` | `Reject`, `AccessDenied: native.stealth_utxo.spend` |
| 9 | Attack: the contractor spends the same ruling leaf without the mediator | contractor | `cb635914858f03be08597349728bd8f6d9eb70fcf4b5ba469f40d1b7cd8003ef` | `Reject`, `AccessDenied: native.stealth_utxo.spend` |
| 10 | Approve, approval code revealed | client | `c36496032076e77f1a50785caf41bd9c41846cbcbf936d934801080fc756957c` | `Accept` |
| 11 | Attack: the client refunds after approving | client | `acfc88f86782699f465bbf7fb84ff74d5e2227b850345890fe9af8e2dd5d91dc` | `Reject`, `AfterEpoch(12722)` not satisfied |
| 12 | Attack: a stranger claims with the now-public code | stranger | `1f3b07c8f667009023cdf875b2192713638401a5f12fe62eed49844625f4afea` | `Reject`, `AccessDenied: native.stealth_utxo.spend` |
| 13 | Claim and settle | contractor (co-signing with the invoice's escrow key) | `5faff53665f08259678b14cb64d51710c4955dea3ee0ca35005b4c8d9d6f2338` | `Accept` |

Between steps 3 and 4 the contractor ran `hexcrow verify 1` twice, both recorded as `verify:1`. Told to expect 2,500 tTARI, the amount check failed: the escrow, decrypted with the invoice's escrow view key, holds 250. Told to expect 250, every check passed: the book names its key and the agreed mediator, the invoice is funded, the client can still approve, the book's terms rebuild the book's condition root, and the escrow output exists unspent, read through a verified proof, locked by exactly that root with no key path. In transaction 13 the spent input is the escrow commitment above, and its witness shows the claim leaf the engine checked: `HashLock`, `BeforeEpoch(12722)` and the contractor's key.

## Invoice 2: the client went silent, the mediator ruled for the contractor

The last milestone of the project in invoice 1, with its own keys: client `6ad2f0f49f07642ecebe6ce6a4c981240b174170e1ac290604e8fa0cf92adc31`, contractor `72cf1f1bfe83b1cd33e874d9bc7794b08a2ba51476feca4cfe3cd863a9600b34`, mediator `8af07e42cb1510a2abf3acf9ff09e1bf59a803e39ef14999c16cb0267ae0a32b`. Nothing on chain links them to invoice 1. Approve before epoch 12718, claim before epoch 12720. Escrow commitment `dc2b5a4f2c06e23cf73fae2ca44ce8c0e52e22f6d376a46e8791162031759612`, condition root `e3798d0221ad73f9b0be9c9dca4a76284ad712ac9d7d115bc6fa0ec514d44fa7`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `be798041c486b37cd9ced72bbfd8e91244ce48976ebc08ca56c7c2c78fd2f6d1` | `Accept` |
| 2 | Fund, 120 tTARI into the escrow | client | `9a038fcc883122961f27e312466fc551290eef8f4c263bbcc00ba8ddf55813ae` | `Accept` |
| 3 | Ruling for the contractor, once the deadline had passed without an approval | mediator and contractor (co-signing with the invoice's escrow key) | `15d5cd6781b6718342b9bd3ccb1ca96f83b095962407a2bfcda1c20b61fb70c0` | `Accept` |

The contractor ran `hexcrow verify 2` after funding, recorded as `verify:2`, and every check passed. In transaction 3 the witness shows the ruling leaf the engine checked: `BeforeEpoch(12720)`, the mediator's key and the contractor's key. The book records the ruling's hash, `51e21e8437f07d91c6741445675cdf7be44bdd8933d46a9f8a04d93978e41d1b`. Nobody can open that hash: a second copy of our stage B script, started by mistake, began the same ruling and overwrote the salt in the local state before it was saved. Its own submission failed, since the escrow output was already spent, so only one ruling reached the chain and the money moved once. The ruling on invoice 3 below keeps its salt and text, and the demo checks that hash in the browser.

## Invoice 3: the contractor withdrew, the mediator ruled for the client

Its own keys: client `f45eedc51951bec34aff72e653b4f50b4a7e0acb2ef08657b9d8fc52b4ae1a0f`, contractor `8e7eaf3722b2227a2ded7a333f63393e0ebc2edc31a19f30a32133c1cb61e61d`, mediator `4a0a538455c979cadd121802dab1b1651b2fcb38a699a8d6fcf4587d698e0342`. Approve before epoch 12720, claim before epoch 12722. Escrow commitment `36932d4bd25d6bf529cbe930b4024eca2da7ac238f153d8ef8ecf18adf5e8d40`, condition root `3a024609643ae8fcab20cc6a5fd1cc4c369802e26003135059a1b56e115ca119`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `7fb1c092041356026494ca0ac6f11ca29858907bb5365b2b1d1407d7fce13dd6` | `Accept` |
| 2 | Fund, 60 tTARI into the escrow | client | `9713f5faccec56254b697e2a766ec4eb52bda8723563cb4c24023fb619bfd491` | `Accept` |
| 3 | Ruling for the client, in epoch 12716, six epochs before the refund leaf opens | mediator and client (co-signing with the invoice's escrow key) | `6a84034b98e7c256c897985945b3ddd0c6f977ab1a14a39b8a2bc3ea0dcb2364` | `Accept` |

The contractor ran `hexcrow verify 3` after funding, recorded as `verify:3`, and every check passed. The book now marks invoice 3 refunded and stores the ruling's hash, `40593b0750711ee3e9a8a85c0366535c771fa14558fd1ee9282b9abb4be190ea`.

## Invoice 4: nobody acted, a late ruling was refused, refunded

Its own keys: client `083b9ed5a59530eeffc56d94e74b1df9db368926d4a242718735712083719017`, contractor `044a260d83dd78f578be5f5a7e2b8d1e7f9879518f60564489e6142934ff271c`, mediator `64aa458719f0238f6b921bb4c828a013c2e15a8a976a0155c9fb96f703a87f0b`. Approve before epoch 12718, claim before epoch 12719. Escrow commitment `ea7e9941adc2fabd5565af2ab1c1497da7c811b673323958e9c4b6c63adc954a`, condition root `d266326d0147f0e1e300aeb8258a6658fc8d5c711796d618b78f988891c672c9`.

| # | Step | Signed by | Transaction | Expected result |
|---|---|---|---|---|
| 1 | Issue | contractor | `5074575ca73f387eb8fd53bb6ba1a12b0542b23d9d3f7db5592aa1d6e530280b` | `Accept` |
| 2 | Fund, 40 tTARI into the escrow | client | `31726df1e70667ae9d8c349ca25a148a1a91de17c39f7a52b1fced530cf090b4` | `Accept` |
| 3 | Attack: the mediator and the contractor rule after the claim window closed | mediator and contractor | `993341d98ca51866dea5830fcd03a9a32e86fb4ba8fa07c847f9cd4efee5436d` | `Reject`, `BeforeEpoch(12719)` not satisfied |
| 4 | Refund from epoch 12719 | client (co-signing with the invoice's escrow key) | `7686ac6e3c06f51026583abaaff354f3a20a6cbdb95d085103d8ef0dee271eca` | `Accept` |

The contractor ran `hexcrow verify 4` after funding, recorded as `verify:4`, and every check passed.

## The accountant's check

For invoice 1, `sha256("d0edd5a4a4fa49ebf91564a728238b9f|Invoice 0001 | Brand identity for a privacy wallet, milestone 2 of 3: logo system and icon set | 250 tTARI")` equals the invoice hash stored in the book, `622e82c52d9086cad29cdfdec1fcfb3145b7d939c988e1a97fa2003fa34c76e6`. The hosted demo computes this in your browser.

```bash
printf '%s' 'd0edd5a4a4fa49ebf91564a728238b9f|Invoice 0001 | Brand identity for a privacy wallet, milestone 2 of 3: logo system and icon set | 250 tTARI' | sha256sum
```

## The mediator's check

A ruling works the same way. For invoice 3, the ruling text and its salt hash to the `ruling_hash` the book stores, `40593b0750711ee3e9a8a85c0366535c771fa14558fd1ee9282b9abb4be190ea`. The mediator's copy in the hosted demo computes this in your browser.

```bash
printf '%s' '69ec1df7a959734c24d783febb8cd59b|Ruling on invoice 0003. The contractor withdrew from the project on 29 October 2026 and will not deliver the storyboard or the voice-over. The escrow is returned to the client now, without waiting for the claim window to close.' | sha256sum
```

## Engine tests

19 tests run the compiled template in `tari_template_test_tooling`, the engine the network uses: [evidence/local-engine-tests.txt](evidence/local-engine-tests.txt). They cover the happy path, the refund only after the claim window, a stranger on every method, the contractor approving its own invoice, the contractor trying to set the approval lock, a wrong approval code, settling without approval, approving twice, refunding an approved invoice, an approval in the last epoch that still leaves the claim window, no approval after the deadline and no settle after the claim window, funding after the deadline, past deadlines, zero grace, self-dealing at issue and a mediator who is one of the parties, the absence of any amount in the book's state, and the co-signed settle. Four cover the mediator: a ruling for the contractor, a ruling for the client, seven signer combinations that must fail because the mediator or the paid side did not sign, and no ruling after an approval or after the claim window.

## History, for completeness

Three earlier templates ran on Esmeralda on the same day, and their entries stay in the run log above the notes that mark each switch.

- **v1** (`template_aecbb45e…`, book `component_a3dba764…`) checked only the sealing signer. On Esmeralda the escrow spend has to be sealed by the escrow key, because the SDK decrypts inputs with the wallet's default key, so the acting party co-signs. One v1 escrow of 250 tTARI stays locked until its refund leaf opens at epoch 12735.
- **v2** (`template_87eb6ce8…`, book `component_ae712c5f…`) settled one invoice and refunded another, but a review found that the contractor chose the approval lock, that an approval in the last epoch did not protect the contractor, and that one escrow key served every invoice.
- **v3** (`template_0b82e0db…`, book `component_40bf6180…`) fixed those three flaws, settled two invoices and refunded one, and refused seven attacks. It left the contractor without recourse when a client never approved delivered work, which v4 answers with the mediator.

[THREAT_MODEL.md](THREAT_MODEL.md) describes the flaws and the fixes.

## Built during the contest

All code was written in October 2026. The commit history of this repository starts on 29 October 2026.
