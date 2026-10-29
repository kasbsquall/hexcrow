"""Save the indexer's raw result for every transaction from template v3 on in the run log.

The public indexers drop transaction results within hours, well before the 50 epochs their /info
advertises (seen on 29-oct-2026). Each result is saved once, as served, under
evidence/indexer-results/<tx>.json, so the verdicts stay checkable after the indexer forgets them.

Usage: python tools/save_results.py
"""

import json
import pathlib
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
LOG = ROOT / "evidence" / "esmeralda-run.jsonl"
OUT = ROOT / "evidence" / "indexer-results"
INDEXER = "https://ootle-indexer-a.tari.com/"
V3_NOTE = "Entries below this line use template v3"


def tx_ids_since_v3():
    lines = LOG.read_text(encoding="utf-8").splitlines()
    start = max(i for i, l in enumerate(lines) if V3_NOTE in l)
    for line in lines[start + 1:]:
        tx = json.loads(line).get("tx_id")
        if tx:
            yield tx


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    saved = missing = 0
    for tx in tx_ids_since_v3():
        dest = OUT / f"{tx}.json"
        if dest.exists():
            continue
        req = urllib.request.Request(f"{INDEXER}transactions/{tx}/result", headers={"User-Agent": "hexcrow-save/1"})
        try:
            with urllib.request.urlopen(req, timeout=20) as r:
                dest.write_bytes(r.read())
            saved += 1
        except urllib.error.HTTPError as e:
            print(f"{tx}: HTTP {e.code}")
            missing += 1
    print(f"saved {saved}, missing {missing}, in {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
