"""Snapshot every transaction verdict in web/data/run.json.

The public indexers drop transaction results within hours, well before the 50 epochs their /info
advertises. The demo reads verdicts live; once the indexer answers 404 for a pruned transaction, it
falls back to this snapshot and says so. Each verdict comes from the raw result saved by
tools/save_results.py (evidence/indexer-results/<tx>.json), or from the transaction record it saved
when the result was already gone (<tx>.tx.json), or from the indexer when neither is saved.
The book state and the escrow outputs never expire and are always read live.

Usage: python tools/snapshot_verdicts.py
"""

import base64
import json
import pathlib
import struct
import urllib.request
from datetime import datetime, timezone

ROOT = pathlib.Path(__file__).resolve().parent.parent
RUN = ROOT / "web" / "data" / "run.json"
SAVED = ROOT / "evidence" / "indexer-results"
OUT = ROOT / "web" / "data" / "verdicts.json"


def fetch(url):
    # The indexer answers 403 to urllib's default user agent.
    req = urllib.request.Request(url, headers={"User-Agent": "hexcrow-snapshot/1"})
    with urllib.request.urlopen(req, timeout=20) as r:
        return json.load(r)


# `TransactionResult` variants in the order the engine encodes them (tari_engine_types commit_result.rs).
RESULTS = ["Accept", "AcceptFeeRejectRest", "Reject"]


def from_cbor(b64):
    """Reads base64 CBOR (RFC 8949), the form indexer 0.50 serves execution results in.
    Byte strings come back as bytes, maps as [key, value] pairs, tags are dropped."""
    b = base64.b64decode(b64)
    pos = 0

    def take(n):
        nonlocal pos
        pos += n
        return b[pos - n:pos]

    def item():
        ib = take(1)[0]
        major, info = ib >> 5, ib & 31
        if major == 7:
            simple = {20: False, 21: True, 22: None, 23: None}
            if info in simple:
                return simple[info]
            if info in (26, 27):
                return struct.unpack(">f" if info == 26 else ">d", take(4 if info == 26 else 8))[0]
            raise ValueError("unsupported CBOR simple value")
        if info > 27:
            raise ValueError("unsupported CBOR length")
        n = info if info < 24 else int.from_bytes(take(1 << (info - 24)), "big")
        if major == 0:
            return n
        if major == 1:
            return -1 - n
        if major == 2:
            return take(n)
        if major == 3:
            return take(n).decode("utf-8")
        if major == 4:
            return [item() for _ in range(n)]
        if major == 5:
            return [[item(), item()] for _ in range(n)]
        return item()  # 6: a tag; its content stands for it

    return item()


def result_name(execution_result):
    # Before indexer 0.50 the result was JSON ({"finalize": {"result": {"Accept": ...}}}); since then it is
    # base64 CBOR, [finalize, ...] with finalize[3] = [variant, ...].
    if isinstance(execution_result, dict):
        return next(iter(execution_result["finalize"]["result"]))
    return RESULTS[from_cbor(execution_result)[0][3][0]]


# The summary the indexer keeps in a transaction record after dropping its result.
SUMMARY = {"Commit": "Accept", "FeeIntentCommit": "AcceptFeeRejectRest"}


def from_record(record):
    """A verdict from /transactions/<tx>: a summary for what it committed, a reason for what it rejected."""
    t = record["transaction"]
    summary = t.get("summary")
    if summary:
        result = SUMMARY.get(summary["outcome"])
        if not result:
            raise SystemExit(f"{t['transaction_id']}: unknown outcome {summary['outcome']}")
        return {"final_decision": "Commit", "result": result, "finalized_time": summary["finalized_at"],
                "from": "transaction record"}
    if t.get("rejected_reason"):
        return {"final_decision": "Reject", "result": "Reject", "finalized_time": None, "from": "transaction record"}
    raise SystemExit(f"{t['transaction_id']} has no verdict in its record; refusing to snapshot it")


def verdict_of(indexer, tx):
    saved = SAVED / f"{tx}.json"
    record = SAVED / f"{tx}.tx.json"
    if saved.exists():
        res = json.loads(saved.read_text(encoding="utf-8"))
    elif record.exists():
        return from_record(json.loads(record.read_text(encoding="utf-8")))
    else:
        res = fetch(f"{indexer}transactions/{tx}/result")
    rejected = res["result"].get("Rejected")
    if rejected:
        return {"final_decision": "Reject", "result": "Reject", "finalized_time": rejected.get("rejected_time")}
    fin = res["result"].get("Finalized")
    if not fin:
        raise SystemExit(f"{tx} is not finalized; refusing to snapshot it")
    return {
        "final_decision": fin["final_decision"],
        "result": result_name(fin["execution_result"]),
        "finalized_time": fin.get("finalized_time"),
    }


def main():
    run = json.loads(RUN.read_text(encoding="utf-8"))
    epoch = fetch(run["indexer"] + "network")["epoch"]
    txs = [row["tx"] for inv in run["invoices"] for row in inv["timeline"]]
    verdicts = {}
    for tx in txs:
        verdicts[tx] = verdict_of(run["indexer"], tx)
    OUT.write_text(json.dumps({
        "taken_utc": datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M"),
        "taken_epoch": epoch,
        "indexer": run["indexer"],
        "verdicts": verdicts,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"{len(verdicts)} verdicts at epoch {epoch} -> {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
