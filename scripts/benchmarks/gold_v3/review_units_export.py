#!/usr/bin/python3
"""Export sentence-unit gold rows (gold-v4-sentence) for the sentence review page: one JSON document per sentence.

Each document carries the assembled sentence, its headings and context, its gold rows (proposal, rules, reason,
difficulty), the justifier's policy notes, Jason's earlier row-level decisions on its members, and the silver labels
on its member rows. Files go to data/gold/v4/review/<doc_id>.json with a manifest for ArtifactData batch writes;
decisions come back with review_import.py --gold-version gold-v4-sentence.

  review_units_export.py
"""

import argparse
import json
import os
import sys

import psycopg2

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from review_export import FIELD_ORDER, doc_id, row_key, rule_texts  # noqa: E402

ROOT = "/var/home/jason/fractalaw"
PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
V4 = os.path.join(ROOT, "data/gold/v4")
SPECIAL = {"CARRIED": "Carried over from your row-level review", "NEW": "No rule fits: candidate new rule",
           "ADDED": "A second role for this label, added in review (#78)",
           "P:inherit": "Takes the purpose of its section"}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--gold-version", default="gold-v4-sentence")
    ap.add_argument("--out", default=os.path.join(V4, "review"))
    args = ap.parse_args()

    rules = rule_texts()
    ev = {e["unit_id"]: e for e in map(json.loads, open(os.path.join(V4, "evidence.jsonl")))}
    order = {u: i for i, u in enumerate(ev)}
    npath = os.path.join(V4, "policy_notes.json")
    notes = json.load(open(npath)) if os.path.exists(npath) else {}
    cur = psycopg2.connect(PG).cursor()
    cur.execute("""SELECT section_id, field, actor_label, actor_position, proposed, rule_ids, reason, difficulty, catalogue_ver
                   FROM drrp_gold WHERE gold_version = %s""", (args.gold_version,))
    by: dict[str, list] = {}
    for uid, field, actor, pos, proposed, rule_ids, reason, diff, cat in cur.fetchall():
        by.setdefault(uid, []).append({
            "key": row_key(uid, field, actor, pos), "field": field, "actor_label": actor, "actor_position": pos,
            "proposed": proposed,
            "rules": [{"id": r, "text": SPECIAL.get(r) or rules.get(r) or ("Purpose class" if r.startswith("P:") else
                                                                       "Precedent pattern (catalogue v2)" if r.startswith("PREC:") else "")}
                      for r in rule_ids],
            "reason": reason, "evidence": {}, "agree": None, "difficulty": diff, "catalogue": cat})
    os.makedirs(args.out, exist_ok=True)
    for f in os.listdir(args.out):
        os.remove(os.path.join(args.out, f))
    rank = {"new_edge": 0, "hard": 1, "easy": 2}
    manifest = []
    for uid, rows in sorted(by.items(), key=lambda kv: order.get(kv[0], 1e9)):
        e = ev[uid]
        rows.sort(key=lambda r: (FIELD_ORDER[r["field"]], r["actor_label"], r["actor_position"]))
        doc = {
            "order": order.get(uid, 0), "section_id": uid, "unit": True, "law_name": e["law_name"],
            "law_title": e.get("law_title") or e["law_name"], "headings": e["headings"], "text": e["text"],
            "stems": [], "items": [], "context": e["context"], "selection": e["kind"], "kind": e["kind"],
            "members": e["members"], "flags": e["flags"], "policy_notes": notes.get(uid, []),
            "row_decisions": e["row_decisions"], "gold_version": args.gold_version,
            "hardest": min(rank.get(r["difficulty"], 1) for r in rows), "n_rows": len(rows), "rows": rows,
        }
        did = doc_id(uid)
        path = os.path.join(args.out, did + ".json")
        json.dump(doc, open(path, "w"), ensure_ascii=False)
        manifest.append({"op": "set", "collection": "provisions", "doc_id": did, "file_path": path})
    json.dump(manifest, open(os.path.join(args.out, "_manifest.json"), "w"), indent=0)
    print(f"{len(by)} sentences, {sum(len(r) for r in by.values())} rows → {args.out} ({len(manifest)} docs)")


if __name__ == "__main__":
    main()
