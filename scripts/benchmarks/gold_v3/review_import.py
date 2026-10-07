#!/usr/bin/python3
"""Import Jason's review decisions from the gold review page into drrp_gold (phase 0a).

The page keeps one document per provision in its `decisions` collection: {section_id, rows: {slot: decision|null}},
where each decision is {key, field, actor_label, decision, decided, comment, at}. Save them locally first with the
ArtifactData tool (action list, collection decisions, out_dir), then:

  review_import.py <out_dir>/decisions               # dry run: counts
  review_import.py <out_dir>/decisions --write

A slot set to null (Undo on the page) clears the decision in drrp_gold. A decision "add" (sentence page, #78) is a
second entry for an actor label in another role: it is inserted as an approved row keyed by that role
(actor_position), and its value's position must differ from the label's existing entries.
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
from load import RENAME  # noqa: E402
from review_export import row_key  # noqa: E402

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
VALID = {"approve", "change", "query"}
POSITIONS = {"active", "counterparty", "beneficiary", "mentioned"}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("decisions_dir")
    ap.add_argument("--gold-version", default="gold-v3-draft")
    ap.add_argument("--write", action="store_true")
    args = ap.parse_args()

    conn = psycopg2.connect(PG)
    cur = conn.cursor()
    cur.execute("SELECT section_id, field, actor_label, actor_position, text_md5 FROM drrp_gold WHERE gold_version = %s",
                (args.gold_version,))
    got = cur.fetchall()
    by_key = {row_key(s, f, a, p): (s, f, a, p) for s, f, a, p, _ in got}
    md5 = {s: m for s, _, _, _, m in got}
    roles = collections.defaultdict(set)  # (section, label) → positions already held
    cur.execute("""SELECT section_id, actor_label, coalesce(decided, proposed)->>'position' FROM drrp_gold
                   WHERE gold_version = %s AND field = 'actor'""", (args.gold_version,))
    for s_, a_, p_ in cur.fetchall():
        roles[(s_, a_)].add(p_)

    updates, cleared, unknown, added, bad = [], [], [], [], []
    for path in sorted(glob.glob(os.path.join(args.decisions_dir, "*.json"))):
        doc = json.load(open(path))
        doc = doc.get("data", doc)  # tolerate a {data: …} wrapper
        for slot, d in (doc.get("rows") or {}).items():
            pid = os.path.basename(path)[:-5]
            key = pid + "~" + slot.replace("__", "~")
            if d and d.get("decision") == "add":
                v, pos = d.get("decided") or {}, key.rsplit("~", 1)[-1]
                sid = doc.get("section_id")
                if (pos not in POSITIONS or v.get("position") != pos or not d.get("actor_label") or sid not in md5
                        or pos in roles[(sid, d["actor_label"])]):
                    bad.append(key)
                    continue
                added.append((args.gold_version, sid, md5[sid], d["actor_label"], pos, Json(v),
                              "Added in review" + (": " + d["comment"] if d.get("comment") else ""), d.get("comment") or None, d.get("at")))
                roles[(sid, d["actor_label"])].add(pos)
                continue
            if key not in by_key:
                unknown.append(key)
                continue
            if not d:
                cleared.append(by_key[key])
            elif d.get("decision") in VALID:
                decided = d.get("decided")
                if isinstance(decided, str):
                    decided = RENAME.get(decided, decided)  # purpose names decided before a rename
                d["decided"] = decided
                updates.append((d["decision"], Json(d.get("decided")) if d["decision"] == "change" else None,
                                d.get("comment") or None, d.get("at"), *by_key[key]))
    c = collections.Counter(u[0] for u in updates)
    print(f"decisions: approve {c['approve']}, change {c['change']}, query {c['query']}; added {len(added)}; "
          f"cleared {len(cleared)}; unknown keys {len(unknown)}; rejected adds {len(bad)}")
    for k in unknown[:5] + bad[:5]:
        print("  ! unknown" if k in unknown else "  ! rejected add (role invalid or already held)", k)
    if not args.write:
        print("dry run: --write to import")
        return
    cur.executemany("""UPDATE drrp_gold SET decision = %s, decided = %s, comment = %s, decided_at = %s
                       WHERE gold_version = '""" + args.gold_version.replace("'", "") + """' AND section_id = %s AND field = %s AND actor_label = %s
                         AND actor_position = %s""",
                    updates)
    cur.executemany("""UPDATE drrp_gold SET decision = NULL, decided = NULL, comment = NULL, decided_at = NULL
                       WHERE gold_version = '""" + args.gold_version.replace("'", "") + """' AND section_id = %s AND field = %s AND actor_label = %s
                         AND actor_position = %s""",
                    cleared)
    cur.executemany("""INSERT INTO drrp_gold (gold_version, section_id, text_md5, field, actor_label, actor_position, proposed,
                                              rule_ids, reason, difficulty, catalogue_ver, decision, comment, decided_at)
                       VALUES (%s, %s, %s, 'actor', %s, %s, %s, '{ADDED}', %s, 'hard', 'review', 'approve', %s, %s)
                       ON CONFLICT DO NOTHING""", added)
    conn.commit()
    print(f"imported into drrp_gold ({args.gold_version})")


if __name__ == "__main__":
    main()
