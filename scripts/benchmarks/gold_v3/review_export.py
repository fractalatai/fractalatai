#!/usr/bin/python3
"""Export gold v3 rows for the review page (phase 0a): one JSON document per provision.

Each document carries the provision (law title, headings, text, the models' context) and its gold rows from
drrp_gold (proposed value, rule IDs with their one-line rule text from the catalogue, reason, evidence,
agree, difficulty). Files go to data/gold/v3/review/<doc_id>.json and are written to the page's database
with the ArtifactData tool (batch, file_path). Decisions come back with review_import.py.

  review_export.py                       # every provision in drrp_gold (gold-v3-draft)
  review_export.py --only data/gold/v3/justified/pilot.jsonl
"""

import argparse
import hashlib
import json
import os
import re

import sys

import psycopg2

sys.path.insert(0, "/var/home/jason/fractalaw/scripts")
from drrp_prompt import ancestors  # noqa: E402

ROOT = "/var/home/jason/fractalaw"
PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
CATALOGUE = os.path.join(ROOT, "docs/architecture/DRRP-RULE-CATALOGUE.md")
# derived at freeze, not reviewed (Jason, 2026-10-06): purpose_fine = Requirements + relation yes/no
DERIVED = {"purpose_fine"}
FIELD_ORDER = {"relation": 0, "raw_type": 1, "purpose": 2, "purpose_fine": 3, "actor": 4}


def doc_id(section_id: str) -> str:
    return "p-" + hashlib.md5(section_id.encode()).hexdigest()[:16]


def row_key(section_id: str, field: str, actor: str, position: str = "") -> str:
    """Page key for a gold row; a second entry for a label in another role (#78) appends that role."""
    return (doc_id(section_id) + "~" + field + ("~" + hashlib.md5(actor.encode()).hexdigest()[:10] if actor else "")
            + ("~" + position if position else ""))


def stems(context: str) -> list[dict]:
    """The STEM CONTEXT block of drrp_prompt.user_prompt, outermost first: [{id, text}]."""
    m = re.search(r"STEM CONTEXT \(ancestors, outermost first\):\n(.*?)\n\nREFERENCED", context, re.S)
    out = []
    for line in (m.group(1).split("\n") if m else []):
        hit = re.match(r"^\[([^\]]+)\] (.*)$", line)
        if hit:
            out.append({"id": hit.group(1), "text": hit.group(2)})
        elif out:
            out[-1]["text"] += "\n" + line
    return out


def rule_texts() -> dict:
    out = {}
    for line in open(CATALOGUE):
        m = re.match(r"^\| (?:~~)?([A-Z]{3,4}-\d+)(?:~~)? \| (.+?) \| ", line)
        if m:
            out[m.group(1)] = re.sub(r"~~|\*\*", "", m.group(2)).strip()
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--gold-version", default="gold-v3-draft")
    ap.add_argument("--evidence", default=os.path.join(ROOT, "data/gold/v3/evidence.jsonl"))
    ap.add_argument("--only", help="a justified JSONL: export just its provisions")
    ap.add_argument("--out", default=os.path.join(ROOT, "data/gold/v3/review"))
    args = ap.parse_args()

    rules = rule_texts()
    evidence = {e["section_id"]: e for e in map(json.loads, open(args.evidence))}
    order = {sid: i for i, sid in enumerate(evidence)}
    only = {json.loads(l)["section_id"] for l in open(args.only)} if args.only else None
    cur = psycopg2.connect(PG).cursor()
    cur.execute("""SELECT section_id, field, actor_label, proposed, rule_ids, reason, evidence, agree, difficulty, catalogue_ver
                   FROM drrp_gold WHERE gold_version = %s""", (args.gold_version,))
    by_sid: dict[str, list] = {}
    for sid, field, actor, proposed, rule_ids, reason, evid, agree, diff, cat in cur.fetchall():
        if only and sid not in only:
            continue
        if field in DERIVED:
            continue
        by_sid.setdefault(sid, []).append({
            "key": row_key(sid, field, actor), "field": field, "actor_label": actor, "proposed": proposed,
            "rules": [{"id": r, "text": rules.get(r, "Purpose class (no catalogue entry yet)" if r.startswith("P:") else
                                                  "No rule fits: candidate new rule" if r == "NEW" else "")} for r in rule_ids],
            "reason": reason, "evidence": evid, "agree": agree, "difficulty": diff, "catalogue": cat,
        })
    os.makedirs(args.out, exist_ok=True)
    for f in os.listdir(args.out):
        os.remove(os.path.join(args.out, f))
    # list items under each provision, so the page can show a stem as printed (lead-in, items, closing words)
    laws = sorted({evidence[s]["law_name"] for s in by_sid})
    cur.execute("SELECT section_id, text FROM legislation_text WHERE law_name = ANY(%s) ORDER BY sort_key", (laws,))
    children: dict[str, list] = {}
    for csid, ctext in cur.fetchall():
        anc = ancestors(csid)
        if anc and anc[0] in by_sid and ctext:
            children.setdefault(anc[0], []).append({"id": csid, "label": csid[len(anc[0]):], "text": ctext})
    rank = {"new_edge": 0, "hard": 1, "easy": 2}
    manifest = []
    for sid, rows in sorted(by_sid.items(), key=lambda kv: order.get(kv[0], 1e9)):
        ev = evidence[sid]
        rows.sort(key=lambda r: (FIELD_ORDER[r["field"]], r["actor_label"]))
        doc = {
            "order": order.get(sid, 0), "section_id": sid, "law_name": ev["law_name"], "law_title": ev.get("law_title") or ev["law_name"],
            "headings": [h["title"] for h in ev.get("headings") or []], "text": ev["text"], "stems": stems(ev["context"]), "items": children.get(sid, []), "context": ev["context"],
            "selection": ev["selection"], "text_md5": ev["text_md5"], "gold_version": args.gold_version,
            "hardest": min(rank.get(r["difficulty"], 1) for r in rows), "n_rows": len(rows), "rows": rows,
        }
        did = doc_id(sid)
        path = os.path.join(args.out, did + ".json")
        json.dump(doc, open(path, "w"), ensure_ascii=False)
        manifest.append({"op": "set", "collection": "provisions", "doc_id": did, "file_path": path})
    json.dump(manifest, open(os.path.join(args.out, "_manifest.json"), "w"), indent=0)
    n = sum(len(r) for r in by_sid.values())
    print(f"{len(by_sid)} provisions, {n} rows → {args.out} (manifest: _manifest.json, {len(manifest)} writes)")


if __name__ == "__main__":
    main()
