"""Snapshot every transaction verdict in web/data/run.json.

The public indexers drop transaction results within hours, well before the 50 epochs their /info
advertises. The demo reads verdicts live; once the indexer answers 404 for a pruned transaction, it
falls back to this snapshot and says so. Each verdict comes from the raw result saved by
tools/save_results.py (evidence/indexer-results/<tx>.json), or from the indexer when none is saved.
The book state and the escrow outputs never expire and are always read live.

Usage: python tools/snapshot_verdicts.py
"""

import json
import pathlib
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


def result_of(indexer, tx):
    saved = SAVED / f"{tx}.json"
    if saved.exists():
        return json.loads(saved.read_text(encoding="utf-8"))
    return fetch(f"{indexer}transactions/{tx}/result")


def main():
    run = json.loads(RUN.read_text(encoding="utf-8"))
    epoch = fetch(run["indexer"] + "network")["epoch"]
    txs = [row["tx"] for inv in run["invoices"] for row in inv["timeline"]]
    verdicts = {}
    for tx in txs:
        fin = result_of(run["indexer"], tx)["result"].get("Finalized")
        if not fin:
            raise SystemExit(f"{tx} is not finalized; refusing to snapshot it")
        result = fin["execution_result"]["finalize"]["result"]
        verdicts[tx] = {
            "final_decision": fin["final_decision"],
            "result": next(iter(result)),
            "finalized_time": fin.get("finalized_time"),
        }
    OUT.write_text(json.dumps({
        "taken_utc": datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M"),
        "taken_epoch": epoch,
        "indexer": run["indexer"],
        "verdicts": verdicts,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"{len(verdicts)} verdicts at epoch {epoch} -> {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
