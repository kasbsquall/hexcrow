"""Export the recorded Esmeralda run as the data file of the web demo.

Reads `hexcrow inspect <n>` for every invoice and the evidence log, and writes web/data/run.json.
Approval codes are only exported once they are public on chain (after approval).

Usage, from the repository root:  python tools/export_run.py
"""

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CLI = ROOT / "cli"
EVIDENCE = ROOT / "evidence" / "esmeralda-run.jsonl"
OUT = ROOT / "web" / "data" / "run.json"
STATE = CLI / ".hexcrow" / "state.json"
BIN = CLI / "target" / "release" / ("hexcrow.exe" if sys.platform == "win32" else "hexcrow")

ATTACKS = {
    "stranger-claim": ("A stranger tries to claim", "Refused by the chain: access denied on the escrow spend"),
    "stranger-claim:after-approval": ("A stranger claims with the now-public approval code", "Refused by the chain: access denied on the escrow spend"),
    "claim-without-code": ("The contractor claims without the approval code", "Refused by the chain: hash lock not satisfied"),
    "early-refund": ("The client refunds before the claim window closes", "Refused by the chain: claim window still open"),
    "contractor-sets-lock": ("The contractor funds its own invoice with a code it chose", "Refused by the book: only the client sets the approval lock"),
    "refund-after-approval": ("The client approves, then tries to take the money back", "Refused by the chain: claim window still open"),
    "self-approve": ("The contractor approves its own invoice", "Refused by the book: the signer is not the client"),
    "mediator-alone": ("The mediator tries to pay itself through a ruling", "Refused by the chain: a ruling also needs the key of the side it pays"),
    "ruling-without-mediator": ("The contractor tries a ruling without the mediator", "Refused by the chain: a ruling needs the mediator's key"),
    "late-ruling": ("The mediator and the contractor rule after the claim window closed", "Refused by the chain: rulings close with the claim window"),
}

RULED = {
    "contractor": "Ruled by the mediator for the contractor, who co-signed: the escrow paid the contractor",
    "client": "Ruled by the mediator for the client, who co-signed: the escrow went back to the client",
}

STEPS = {
    "issue": ("contractor", "Issued by the contractor"),
    "fund": ("client", "Funded by the client"),
    "approve": ("client", "Approved by the client, approval code revealed"),
    "claim": ("contractor", "Claimed by the contractor, invoice settled"),
    "refund": ("client", "Refunded to the client after the claim window closed"),
}


def inspect(n: int) -> dict:
    out = subprocess.run([str(BIN), "inspect", str(n)], cwd=CLI, capture_output=True, text=True, check=True)
    return json.loads(out.stdout)


def role(persona: str) -> str:
    """`client-3` acts as the client of invoice 3; the run log keeps the one-time persona name."""
    return persona.rsplit("-", 1)[0]


def engine_tests_passed() -> int:
    text = (ROOT / "evidence" / "local-engine-tests.txt").read_text(encoding="utf-8")
    return sum(1 for l in text.splitlines() if l.startswith("test ") and l.endswith("... ok"))


def contractor_check(n: int, entries: list[dict]) -> dict | None:
    """The last read-only `hexcrow verify n` the contractor ran before starting work."""
    checks = [e for e in entries if e["step"] == f"verify:{n}" and e.get("read_only")]
    if not checks:
        return None
    last = checks[-1]
    return {"recorded_at": last["recorded_at"], **last["result"]}


def run_entries() -> list[dict]:
    """Entries after the last note, which marks the current template version."""
    lines = [json.loads(l) for l in EVIDENCE.read_text(encoding="utf-8").splitlines() if l.strip()]
    notes = [i for i, l in enumerate(lines) if l.get("step") == "note"]
    return lines[notes[-1] + 1:] if notes else lines


def timeline_for(n: int, entries: list[dict]) -> list[dict]:
    rows = []
    for e in entries:
        parts = e["step"].split(":")
        if parts[0] == "attack":
            kind = parts[1] + (":" + parts[3] if len(parts) > 3 and parts[3] == "after-approval" else "")
            if parts[2] != str(n):
                continue
            label, why = ATTACKS[kind]
            rows.append({"kind": "attack", "attack": kind, "label": label, "why": why, "actor": role(e["signer_persona"])})
        elif parts[0] == "rule" and parts[1] == str(n):
            rows.append({"kind": "rule", "label": RULED[parts[2]], "actor": "mediator", "for": parts[2]})
        elif parts[0] in STEPS and len(parts) > 1 and parts[1] == str(n):
            actor, label = STEPS[parts[0]]
            rows.append({"kind": parts[0], "label": label, "actor": actor})
        else:
            continue
        rows[-1].update({
            "tx": e["tx_id"],
            "accepted": e["accepted"],
            # A rejection that still took the fee (OnlyFeeCommit) is on chain with its fee.
            "fee_micro_tari": e["fee_micro_tari"] if (e["accepted"] or e["outcome"].startswith("OnlyFeeCommit")) else None,
        })
    return rows


def main() -> None:
    state = json.loads(STATE.read_text(encoding="utf-8"))
    entries = run_entries()
    book_steps = {e["step"]: e["tx_id"] for e in entries if e["step"] in ("publish-template", "new-book")}
    invoices = []
    for key in sorted(state["invoices"], key=int):
        n = int(key)
        local = state["invoices"][key]
        view = inspect(n)
        timeline = timeline_for(n, entries)
        approved = any(r["kind"] == "approve" and r["accepted"] for r in timeline)
        ruling = view.get("mediator_ruling")
        invoices.append({
            "no": n,
            "invoice_hash": view["public"]["invoice_hash"],
            "deadline_epoch": view["public"]["deadline_epoch"],
            "claim_until": view["public"]["claim_until"],
            "escrow_commitment": view["public"]["escrow_commitment"],
            "escrow_root": view["public"]["escrow_root"],
            "client_key": view["parties"]["client_key"],
            "contractor_key": view["parties"]["contractor_key"],
            "mediator_key": view["parties"]["mediator_key"],
            # Public in the book: who the ruling paid and the hash of its text.
            # Invoice 2's ruling salt was overwritten by a second copy of the stage B script before it was saved:
            # the hash in the book stands, but nobody can open it, so its text is not exported as checkable.
            "ruling": {"for": ruling["for"], "ruling_hash": ruling["ruling_hash"],
                       **({"salt_lost": True} if not ruling["salt"] else {})} if ruling else None,
            "parties_only": {
                "amount_micro_tari": view["escrow_view_key_holders_see"]["amount_micro_tari"],
                "memo": view["escrow_view_key_holders_see"]["memo"],
                "text": view["accountant_opening"]["doc"],
                "salt": view["accountant_opening"]["salt"],
                **({"ruling_text": ruling["text"], "ruling_salt": ruling["salt"]} if ruling and ruling["salt"] else {}),
            },
            "approval_code": local["approval_code"] if approved else None,
            "contractor_check": contractor_check(n, entries),
            "timeline": timeline,
        })
    run = {
        "network": "esmeralda",
        "indexer": "https://ootle-indexer-a.tari.com/",
        "template": view["template"],
        "book": view["book"],
        "book_txs": book_steps,
        "engine_tests_passed": engine_tests_passed(),
        "invoices": invoices,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(run, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)}: {len(invoices)} invoices")


if __name__ == "__main__":
    main()
