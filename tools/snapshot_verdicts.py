"""Snapshot every transaction verdict in web/data/run.json from the public indexer.

The indexer keeps transactions for 50 epochs (`transaction_retention_epochs` in /info). The demo
reads verdicts live; once the indexer answers 404 for a pruned transaction, it falls back to this
snapshot and says so. The book state never expires and is always read live.

Usage: python tools/snapshot_verdicts.py
"""

import json
import pathlib
import urllib.request
from datetime import datetime, timezone

ROOT = pathlib.Path(__file__).resolve().parent.parent
RUN = ROOT / "web" / "data" / "run.json"
OUT = ROOT / "web" / "data" / "verdicts.json"


def fetch(url):
    # The indexer answers 403 to urllib's default user agent.
    req = urllib.request.Request(url, headers={"User-Agent": "hexcrow-snapshot/1"})
    with urllib.request.urlopen(req, timeout=20) as r:
        return json.load(r)


def main():
    run = json.loads(RUN.read_text(encoding="utf-8"))
    epoch = fetch(run["indexer"] + "network")["epoch"]
    txs = [row["tx"] for inv in run["invoices"] for row in inv["timeline"]]
    verdicts = {}
    for tx in txs:
        fin = fetch(f"{run['indexer']}transactions/{tx}/result")["result"].get("Finalized")
        if not fin:
            raise SystemExit(f"{tx} is not finalized on the indexer; refusing to snapshot it")
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
