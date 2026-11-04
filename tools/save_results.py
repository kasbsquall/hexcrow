"""Save the indexer's raw result for every transaction from template v3 on in the run log.

When the indexer has already dropped a result (404), its transaction record is saved instead as
<tx>.tx.json: the indexer keeps that record, with a summary of the verdict, after it drops the result.

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


def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "hexcrow-save/1"})
    with urllib.request.urlopen(req, timeout=20) as r:
        return r.read()


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    saved = records = missing = 0
    for tx in tx_ids_since_v3():
        dest = OUT / f"{tx}.json"
        record = OUT / f"{tx}.tx.json"
        if dest.exists() or record.exists():
            continue
        try:
            dest.write_bytes(get(f"{INDEXER}transactions/{tx}/result"))
            saved += 1
        except urllib.error.HTTPError as e:
            if e.code != 404:
                raise
            try:
                raw = get(f"{INDEXER}transactions/{tx}")
                t = json.loads(raw)["transaction"]
                if not (t.get("summary") or t.get("rejected_reason")):
                    # Still pending, or no verdict yet: saving it would freeze an empty record.
                    print(f"{tx}: no verdict in its record yet")
                    missing += 1
                    continue
                record.write_bytes(raw)
                records += 1
            except urllib.error.HTTPError as e2:
                print(f"{tx}: HTTP {e2.code}")
                missing += 1
    print(f"saved {saved} results and {records} records, missing {missing}, in {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
