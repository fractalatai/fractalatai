#!/usr/bin/python3
"""Import Jason's review decisions from the gold review page into drrp_gold (phase 0a).

The page keeps one document per provision in its `decisions` collection: {section_id, rows: {slot: decision|null}},
where each decision is {key, field, actor_label, decision, decided, comment, at}. Save them locally first with the
ArtifactData tool (action list, collection decisions, out_dir), then:

  review_import.py <out_dir>/decisions               # dry run: counts
  review_import.py <out_dir>/decisions --write

A slot set to null (Undo on the page) clears the decision in drrp_gold.
"""

import argparse
import collections
import glob
import json
import os
import sys

import psycopg2
from psycopg2.extras import Json

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from review_export import row_key  # noqa: E402

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
VALID = {"approve", "change", "query"}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("decisions_dir")
    ap.add_argument("--gold-version", default="gold-v3-draft")
    ap.add_argument("--write", action="store_true")
    args = ap.parse_args()

    conn = psycopg2.connect(PG)
    cur = conn.cursor()
    cur.execute("SELECT section_id, field, actor_label FROM drrp_gold WHERE gold_version = %s", (args.gold_version,))
    by_key = {row_key(s, f, a): (s, f, a) for s, f, a in cur.fetchall()}

    updates, cleared, unknown = [], [], []
    for path in sorted(glob.glob(os.path.join(args.decisions_dir, "*.json"))):
        doc = json.load(open(path))
        doc = doc.get("data", doc)  # tolerate a {data: …} wrapper
        for slot, d in (doc.get("rows") or {}).items():
            pid = os.path.basename(path)[:-5]
            key = pid + "~" + slot.replace("__", "~")
            if key not in by_key:
                unknown.append(key)
                continue
            if not d:
                cleared.append(by_key[key])
            elif d.get("decision") in VALID:
                updates.append((d["decision"], Json(d.get("decided")) if d["decision"] == "change" else None,
                                d.get("comment") or None, d.get("at"), *by_key[key]))
    c = collections.Counter(u[0] for u in updates)
    print(f"decisions: approve {c['approve']}, change {c['change']}, query {c['query']}; cleared {len(cleared)}; unknown keys {len(unknown)}")
    for k in unknown[:5]:
        print("  ! unknown", k)
    if not args.write:
        print("dry run: --write to import")
        return
    cur.executemany("""UPDATE drrp_gold SET decision = %s, decided = %s, comment = %s, decided_at = %s
                       WHERE gold_version = '""" + args.gold_version.replace("'", "") + """' AND section_id = %s AND field = %s AND actor_label = %s""",
                    updates)
    cur.executemany("""UPDATE drrp_gold SET decision = NULL, decided = NULL, comment = NULL, decided_at = NULL
                       WHERE gold_version = '""" + args.gold_version.replace("'", "") + """' AND section_id = %s AND field = %s AND actor_label = %s""",
                    cleared)
    conn.commit()
    print(f"imported into drrp_gold ({args.gold_version})")


if __name__ == "__main__":
    main()
