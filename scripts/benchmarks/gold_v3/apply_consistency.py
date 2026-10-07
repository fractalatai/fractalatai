#!/usr/bin/python3
"""Apply Jason's accepted consistency-check groups (phase B) to gold-v4-sentence.

Reads data/gold/v4/consistency/conflicts.json (consistency agents' output, grouped). Each accepted conflict becomes:
- a `change` decision on the existing row (relation, raw_type, purpose, actor), commented with the principle;
- a new approved row for an added actor or a second entry for a label in another role (#78, actor_position);
- `{"listed": false}` for a removed actor;
- a `deemed-holder` tag appended to the relation row's rule_ids;
- for `exclude`, the unit goes to data/gold/v4/excluded_units.csv and its gold rows are deleted.
Every change cites "v2 consistency (Jason 2026-10-07, <group>)". Back up drrp_gold first.

  apply_consistency.py --groups G1,G2,G3,G4,G5,G6,G8,G9            # dry run
  apply_consistency.py --groups G1,G2,G3,G4,G5,G6,G8,G9 --write
"""

import argparse
import csv
import json
import os
import re

import psycopg2
from psycopg2.extras import Json

ROOT = "/var/home/jason/fractalaw"
PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
V4 = os.path.join(ROOT, "data/gold/v4")
GV = "gold-v4-sentence"
# labels the agents wrote informally → dictionary labels
LABEL = {"Gvt: Agency: HSE (the Executive)": "Gvt: Agency: Health and Safety Executive",
         "Aviation: Crew (commander)": "Aviation: Crew",
         "partners of a panel (label to pick; Spc: Participant entry was deleted)":
             "OTHER: Partner of a support panel (Counter-Terrorism and Security Act 2015 s.38)"}


def parse_entry(text: str) -> dict:
    """'add second entry: counterparty, holds none, act ['pay']' / '…: counterparty/none/act [consult]' → value."""
    t = text.split("add ", 1)[1]
    pos = re.search(r"\b(active|counterparty|beneficiary|mentioned)\b", t).group(1)
    holds = re.search(r"\b(Obligation|Liberty|both|none)\b", t)
    acts = re.search(r"act \[([^\]]*)\]", t)
    return {"position": pos, "holds": holds.group(1) if holds else "none", "inferred": False,
            "act": [a.strip(" '\"") for a in acts.group(1).split(",") if a.strip(" '\"")] if acts else []}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--groups", required=True)
    ap.add_argument("--write", action="store_true")
    args = ap.parse_args()
    groups = set(args.groups.split(","))
    conflicts = [c for c in json.load(open(os.path.join(V4, "consistency/conflicts.json"))) if c["group"] in groups]

    conn = psycopg2.connect(PG)
    cur = conn.cursor()
    cur.execute("SELECT section_id, field, actor_label, actor_position, text_md5 FROM drrp_gold WHERE gold_version = %s", (GV,))
    have = {(s, f, a, p) for s, f, a, p, _ in cur.fetchall() for _ in [0]}
    cur.execute("SELECT DISTINCT section_id, text_md5 FROM drrp_gold WHERE gold_version = %s", (GV,))
    md5 = dict(cur.fetchall())

    ops, excluded = [], []
    for c in conflicts:
        uid, field, v2 = c["unit_id"], c["field"], c["v2"]
        note = f"v2 consistency (Jason 2026-10-07, {c['group']}): {c['principle']}: {c['why']}"
        if c["kind"] == "exclude":
            excluded.append((uid, c["why"]))
            continue
        if field == "tag":
            ops.append(("tag", uid, "relation", "", "", v2, note))
            continue
        actor = LABEL.get(c["actor"] or "", c["actor"] or "")
        if field != "actor":
            ops.append(("change", uid, field, "", "", v2, note))
        elif isinstance(v2, str) and v2.startswith("remove"):
            ops.append(("change", uid, field, actor, "", {"listed": False}, note))
        elif isinstance(v2, str) and "second entry" in v2:
            val = parse_entry(v2)
            ops.append(("insert", uid, field, actor, val["position"], val, note))
        elif isinstance(v2, str) and v2.startswith("add entry"):
            ops.append(("insert", uid, field, actor, "", parse_entry(v2), note))
        elif isinstance(v2, dict):
            ops.append(("change" if (uid, field, actor, "") in have else "insert", uid, field, actor, "", v2, note))
        else:
            raise SystemExit(f"unhandled: {c}")
        # a sentence that becomes `continues` has no raw_type
        if field == "relation" and v2 == "continues":
            ops.append(("change", uid, "raw_type", "", "", None, note))

    for op in ops:
        print(op[0], op[1], op[2], op[3], op[4] or "", json.dumps(op[5])[:90])
    print(f"{len(ops)} operations; exclude {len(excluded)}: {[u for u, _ in excluded]}")
    if not args.write:
        print("dry run: --write to apply")
        return
    for kind, uid, field, actor, pos, val, note in ops:
        if kind == "change":
            cur.execute("""UPDATE drrp_gold SET decision = 'change', decided = %s, comment = %s, decided_at = now()
                           WHERE gold_version = %s AND section_id = %s AND field = %s AND actor_label = %s AND actor_position = ''""",
                        (Json(val), note, GV, uid, field, actor))
            assert cur.rowcount == 1, (uid, field, actor)
        elif kind == "insert":
            cur.execute("""INSERT INTO drrp_gold (gold_version, section_id, text_md5, field, actor_label, actor_position, proposed,
                                                  rule_ids, reason, difficulty, catalogue_ver, decision, comment, decided_at)
                           VALUES (%s, %s, %s, 'actor', %s, %s, %s, %s, %s, 'hard', 'v2', 'approve', %s, now())""",
                        (GV, uid, md5[uid], actor, pos, Json(val), ["CONSISTENCY"], note, note))
        elif kind == "tag":
            cur.execute("""UPDATE drrp_gold SET rule_ids = array_append(rule_ids, %s), comment = coalesce(comment || ' | ', '') || %s
                           WHERE gold_version = %s AND section_id = %s AND field = 'relation' AND NOT (%s = ANY(rule_ids))""",
                        (val, note, GV, uid, val))
    path = os.path.join(V4, "excluded_units.csv")
    known = {r["unit_id"] for r in csv.DictReader(open(path))}
    with open(path, "a", newline="") as f:
        w = csv.writer(f)
        for uid, why in excluded:
            if uid not in known:
                w.writerow([uid, "amending text (catalogue v2 REL-07; Jason 2026-10-07): " + why, "2026-10-07"])
            cur.execute("DELETE FROM drrp_gold WHERE gold_version = %s AND section_id = %s", (GV, uid))
    conn.commit()
    print("applied")


if __name__ == "__main__":
    main()
