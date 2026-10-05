#!/usr/bin/python3
"""Cross-instrument duty holders (#77): write APPROVED agent decisions to the adjudicated tier.

decisions.jsonl, one row per candidate provision (written by the review agent, approved by Jason):
  {"section_id": "...", "parent_section_id": "...", "holders": [{"label": "...", "drrp": "Obligation"}],
   "rationale": "...", "approved": true}
An empty "holders" list records "no holder" (the provision stays holder unknown) and writes nothing.

Writes provision_actors (section_id, actor_label) with adj_drrp, adj_position = 'active', adj_note, inserting
the row if it doesn't exist. Never overwrites an existing adjudicated value. Dry run unless --apply.
--apply also stamps the skill's last_run (the reminder hook reads it).

  apply_decisions.py data/audit/cross_instrument/<date>/decisions.jsonl            # dry run
  apply_decisions.py data/audit/cross_instrument/<date>/decisions.jsonl --apply
"""

import argparse
import datetime as dt
import json
import os

import psycopg2
import yaml

ROOT = "/var/home/jason/fractalaw"
PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
DICTIONARY = os.path.join(ROOT, "crates/fractalaw-core/data/actor-dictionary.yaml")
STAMP = os.path.join(ROOT, ".claude/skills/cross-instrument-holders/last_run")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("decisions")
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()

    entries = {e["label"]: e for e in yaml.safe_load(open(DICTIONARY))}
    rows = [json.loads(line) for line in open(args.decisions) if line.strip()]
    today = dt.date.today().isoformat()
    conn = psycopg2.connect(PG)
    cur = conn.cursor()
    counts = {"written": 0, "no_holder": 0, "not_approved": 0, "skipped": 0}

    for r in rows:
        sid = r["section_id"]
        if not r.get("approved"):
            counts["not_approved"] += 1
            continue
        if not r.get("holders"):
            counts["no_holder"] += 1
            continue
        cur.execute("SELECT 1 FROM legislation_text WHERE section_id = %s", (sid,))
        if not cur.fetchone():
            print(f"skip {sid}: provision no longer in the hub")
            counts["skipped"] += 1
            continue
        for h in r["holders"]:
            label, drrp = h["label"], h.get("drrp", "Obligation")
            if label not in entries:
                print(f"skip {sid} {label}: not a dictionary label")
                counts["skipped"] += 1
                continue
            cur.execute("SELECT adj_position FROM provision_actors WHERE section_id = %s AND actor_label = %s", (sid, label))
            existing = cur.fetchone()
            if existing and existing[0] is not None:
                print(f"skip {sid} {label}: already adjudicated ({existing[0]})")
                counts["skipped"] += 1
                continue
            note = (f"cross-instrument holder (#77) from {r['parent_section_id']}; Claude agent, "
                    f"approved by Jason {today}: {r.get('rationale', '')}")[:1000]
            print(f"{'write' if args.apply else 'would write'} {sid} {label} {drrp}/active")
            if args.apply:
                cur.execute(
                    "INSERT INTO provision_actors (section_id, actor_label, actor_category, adj_drrp, adj_position, adj_note) "
                    "VALUES (%s, %s, %s, %s, 'active', %s) ON CONFLICT (section_id, actor_label) DO UPDATE SET "
                    "adj_drrp = EXCLUDED.adj_drrp, adj_position = EXCLUDED.adj_position, adj_note = EXCLUDED.adj_note "
                    "WHERE provision_actors.adj_position IS NULL",
                    (sid, label, entries[label].get("category", "Unknown"), drrp, note),
                )
                if cur.rowcount != 1:
                    raise SystemExit(f"{sid} {label}: expected 1 row, got {cur.rowcount}; rolled back")
            counts["written"] += 1

    if args.apply:
        conn.commit()
        with open(STAMP, "w") as f:
            f.write(today + "\n")
        print(f"stamped {STAMP}")
    else:
        conn.rollback()
    print(("applied: " if args.apply else "dry run: ") + ", ".join(f"{k} {v}" for k, v in counts.items()))


if __name__ == "__main__":
    main()
