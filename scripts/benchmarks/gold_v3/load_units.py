#!/usr/bin/python3
"""Load sentence-unit gold rows into drrp_gold (gold-v4-sentence; meta-plan phases A and C).

Two sources:
- `--carry-singles`: single-row units take Jason's row-level decisions (gold-v3-draft) as they are: same unit,
  same judgment. They load as already decided.
- justified JSONL (JUSTIFY_UNITS.md output): one proposal per field per unit. On a `stem_reviewed` unit, a field
  whose proposal equals Jason's decision on the stem row loads as decided (carried); everything else is open.
Policy notes go to data/gold/v4/policy_notes.json for the review page. Decided rows are never overwritten.
An actor label proposed twice in different roles (#78) loads as two rows: the first keyed '' and the other keyed by its
position (actor_position); `act` is a list.

  load_units.py --carry-singles [--write]
  load_units.py data/gold/v4/justified/load1a.jsonl data/gold/v4/justified/load1b.jsonl [--write]
"""

import argparse
import collections
import hashlib
import json
import os

import psycopg2
from psycopg2.extras import Json

ROOT = "/var/home/jason/fractalaw"
PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
V4 = os.path.join(ROOT, "data/gold/v4")


def actlist(v):
    """An actor value with `act` as a list (#78: one party can have more than one act)."""
    if isinstance(v, dict) and "position" in v and not isinstance(v.get("act"), list):
        return {**v, "act": [v["act"]] if v.get("act") else []}
    return v


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("justified", nargs="*")
    ap.add_argument("--carry-singles", action="store_true")
    ap.add_argument("--evidence", default=os.path.join(V4, "evidence.jsonl"))
    ap.add_argument("--gold-version", default="gold-v4-sentence")
    ap.add_argument("--write", action="store_true")
    args = ap.parse_args()

    ev = {e["unit_id"]: e for e in map(json.loads, open(args.evidence))}
    md5 = {u: hashlib.md5(e["text"].encode()).hexdigest() for u, e in ev.items()}
    rows, notes, excludes = [], {}, []
    c = collections.Counter()

    if args.carry_singles:
        for uid, e in ev.items():
            if e["kind"] != "single":
                continue
            for d in e["row_decisions"]:
                note = f"Carried from your row-level review ({d['decision']})" + (f": {d['comment']}" if d["comment"] else "")
                rows.append((uid, d["field"], d["actor_label"], "", actlist(d["value"]) if d["field"] == "actor" else d["value"], ["CARRIED"], note, "easy", "carried",
                             "approve" if d["decision"] != "query" else "query", d["comment"]))
                c["carried " + d["decision"]] += 1

    for path in args.justified:
        for line in open(path):
            j = json.loads(line)
            uid = j["unit_id"]
            if uid not in ev:  # dropped from gold (excluded_units.csv)
                continue
            if j.get("exclude"):  # JUSTIFY_V2: amending text, out of domain → excluded_units.csv after Jason agrees
                c["exclude proposed"] += 1
                excludes.append((uid, j["exclude"]))
                continue
            e = ev[uid]
            # JUSTIFY_V2 notes {kind, note}; phase A policy_notes {default, note}
            notes[uid] = [({"default": None, "note": f"{n['kind']}: {n['note']}"} if "kind" in n else n)
                          for n in (j.get("notes") or j.get("policy_notes") or [])]
            stem = {(d["field"], d["actor_label"]): d for d in e["row_decisions"] if d["row"] == uid and d["decision"] != "query"}
            seen: set = set()
            for f in j["fields"]:
                actor = f.get("actor_label") or ""
                val = actlist(f["proposed"]) if f["field"] == "actor" else f["proposed"]
                pos = ""
                if f["field"] == "actor" and actor in seen:  # same label, another role (#78)
                    pos = (val or {}).get("position") or ""
                seen.add(actor)
                prev = stem.get((f["field"], actor)) if e["kind"] == "stem_reviewed" and not pos else None
                same = prev is not None and actlist(prev["value"]) == val
                rows.append((uid, f["field"], actor, pos, val, f.get("rule_ids") or [], f["reason"], f["difficulty"],
                             os.path.basename(path), "approve" if same else None,
                             "Carried: matches your decision on the stem row" if same else None))
                c[("carried" if same else "open") + " " + f["difficulty"]] += 1

    print(f"{len(rows)} rows for {len({r[0] for r in rows})} units: " + ", ".join(f"{k} {v}" for k, v in sorted(c.items())))
    for uid, why in excludes:
        print(f"  exclusion proposed (not loaded; add to excluded_units.csv when Jason agrees): {uid}: {why}")
    if not args.write:
        print("dry run: --write to load")
        return
    conn = psycopg2.connect(PG)
    cur = conn.cursor()
    cur.execute("SELECT section_id, field, actor_label, actor_position FROM drrp_gold WHERE gold_version = %s AND decision IS NOT NULL",
                (args.gold_version,))
    decided = set(cur.fetchall())
    n = 0
    for uid, field, actor, pos, proposed, rule_ids, reason, diff, src, decision, comment in rows:
        if (uid, field, actor, pos) in decided:
            continue
        cur.execute("""INSERT INTO drrp_gold (gold_version, section_id, text_md5, field, actor_label, actor_position, proposed,
                                              rule_ids, reason, evidence, agree, difficulty, catalogue_ver, decision, comment, decided_at)
                       VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s,%s,NULL,%s,%s,%s,%s, CASE WHEN %s::text IS NULL THEN NULL ELSE now() END)
                       ON CONFLICT (gold_version, section_id, field, actor_label) DO UPDATE SET
                         text_md5 = EXCLUDED.text_md5, proposed = EXCLUDED.proposed, rule_ids = EXCLUDED.rule_ids,
                         reason = EXCLUDED.reason, difficulty = EXCLUDED.difficulty, catalogue_ver = EXCLUDED.catalogue_ver,
                         decision = EXCLUDED.decision, comment = EXCLUDED.comment, decided_at = EXCLUDED.decided_at""",
                    (args.gold_version, uid, md5[uid], field, actor, pos, Json(proposed), rule_ids, reason, Json({}), diff, src,
                     decision, comment, decision))
        n += 1
    # an open proposal the justifier no longer makes is dropped
    for uid in {r[0] for r in rows if r[7] != "carried"}:
        keep = {(r[1], r[2], r[3]) for r in rows if r[0] == uid}
        cur.execute("""SELECT field, actor_label, actor_position FROM drrp_gold
                       WHERE gold_version = %s AND section_id = %s AND decision IS NULL""", (args.gold_version, uid))
        for field, actor, pos in cur.fetchall():
            if (field, actor, pos) not in keep:
                cur.execute("""DELETE FROM drrp_gold WHERE gold_version = %s AND section_id = %s AND field = %s
                                 AND actor_label = %s AND actor_position = %s""", (args.gold_version, uid, field, actor, pos))
    conn.commit()
    if notes:
        path = os.path.join(V4, "policy_notes.json")
        allnotes = json.load(open(path)) if os.path.exists(path) else {}
        allnotes.update(notes)
        json.dump(allnotes, open(path, "w"), ensure_ascii=False, indent=1)
    print(f"written {n} rows to drrp_gold ({args.gold_version}); decided rows untouched")


if __name__ == "__main__":
    main()
