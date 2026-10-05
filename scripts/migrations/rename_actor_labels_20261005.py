#!/usr/bin/python3
"""Rename actor labels in existing rows after the 2026-10-05 reconciliation with sertantai-legal.

The rename map comes from the dictionary's `renamed_from` fields
(crates/fractalaw-core/data/actor-dictionary.yaml), e.g. SC: Applicant → Ind: Applicant,
Spc: Licence Holder → Ind: Licensee, Public: Keeper → SC: Keeper.

Tables: provision_actors (all tiers), gold_benchmarks, gold_v2. Raw LLM responses (gold_v2_raw,
drrp_training_labels_raw) stay as they were returned. Law-level arrays (legislation_text actors and
governed/government lists; DuckDB holder lists) are rebuilt by the next backfill.

Collisions (a provision already has the new label): the old row's values fill the new row's empty
columns (COALESCE(new, old) per column), then the old row is removed. No tier's data is lost; where
both rows have a value, the new label's value is kept and the old one is reported.

  rename_actor_labels_20261005.py            # dry run: counts and collisions
  rename_actor_labels_20261005.py --apply    # one transaction
"""

import argparse
import os

import psycopg2
import yaml

ROOT = "/var/home/jason/fractalaw"
PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
DICTIONARY = os.path.join(ROOT, "crates/fractalaw-core/data/actor-dictionary.yaml")
# provision_actors value columns merged on collision (everything except the key)
PA_COLS = ["actor_category", "regex_drrp", "regex_position", "cls_drrp", "cls_position", "cls_confidence",
           "llm_drrp", "llm_position", "drrp", "position", "extraction_method", "inferred_drrp", "inferred_position",
           "dep_is_subject", "dep_is_object", "dep_is_agent", "dep_is_attr", "dep_voice_passive", "dep_has_modal",
           "dep_verb_distance", "reconcile_confidence", "slm_drrp", "slm_position", "slm_confidence",
           "adj_drrp", "adj_position", "adj_note", "llm_prompt_version"]
TABLES = {"provision_actors": PA_COLS, "gold_benchmarks": ["gold_drrp", "gold_position"],
          "gold_v2": ["law_name", "text_md5", "position", "holds", "inferred", "source", "note", "prompt_version", "created_at"]}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()
    entries = yaml.safe_load(open(DICTIONARY))
    renames = {old: e["label"] for e in entries for old in e.get("renamed_from", [])}
    categories = {e["label"]: e.get("category") for e in entries}
    print("rename map:", ", ".join(f"{o} → {n}" for o, n in renames.items()))

    conn = psycopg2.connect(PG)
    cur = conn.cursor()
    for table, cols in TABLES.items():
        for old, new in renames.items():
            cur.execute(f"SELECT count(*) FROM {table} WHERE actor_label = %s", (old,))
            n = cur.fetchone()[0]
            if not n:
                continue
            cur.execute(f"SELECT o.section_id FROM {table} o JOIN {table} n USING (section_id) "
                        f"WHERE o.actor_label = %s AND n.actor_label = %s", (old, new))
            clash = [r[0] for r in cur.fetchall()]
            conflicts = 0
            for sid in clash:  # both rows hold a different non-null value
                cur.execute(f"SELECT {', '.join(cols)} FROM {table} WHERE section_id = %s AND actor_label = ANY(%s) "
                            f"ORDER BY actor_label = %s DESC", (sid, [new, old], new))
                nv, ov = cur.fetchall()
                conflicts += sum(1 for a, b in zip(nv, ov) if a is not None and b is not None and a != b)
            print(f"  {table}: {old} → {new}: {n} rows, {len(clash)} collisions ({conflicts} conflicting values, new kept)")
            if not args.apply:
                continue
            for sid in clash:
                sets = ", ".join(f"{c} = COALESCE(n.{c}, o.{c})" for c in cols)
                cur.execute(f"UPDATE {table} n SET {sets} FROM {table} o WHERE n.section_id = %s AND n.actor_label = %s "
                            f"AND o.section_id = n.section_id AND o.actor_label = %s", (sid, new, old))
                cur.execute(f"DELETE FROM {table} WHERE section_id = %s AND actor_label = %s", (sid, old))
            if table == "provision_actors":
                cur.execute("UPDATE provision_actors SET actor_label = %s, actor_category = coalesce(%s, actor_category) "
                            "WHERE actor_label = %s", (new, categories.get(new), old))
            else:
                cur.execute(f"UPDATE {table} SET actor_label = %s WHERE actor_label = %s", (new, old))
            cur.execute(f"SELECT count(*) FROM {table} WHERE actor_label = %s", (old,))
            assert cur.fetchone()[0] == 0, (table, old)
    if args.apply:
        conn.commit()
        print("applied")
    else:
        conn.rollback()
        print("dry run: nothing written")


if __name__ == "__main__":
    main()
