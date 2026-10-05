#!/usr/bin/python3
"""Remove the output of the retired correlative rule "Gvt: Authority: Enforcement active → inferred Ind: Public
beneficiary" (correlative-rules.yaml rule 3; Jason 2026-10-05: not required).

Those rows are actor_label 'Ind: Public' with extraction_method 'inferred'. They were created by the rule, and the
old position classifier later scored some of them (cls_position only). No other tier wrote them. Run this
BEFORE rename_actor_labels_20261005.py renames "Public" → "Ind: Public", so the rule's rows don't merge into
the real ones (13 provisions carry both).

  retire_public_correlative_20261005.py            # dry run
  retire_public_correlative_20261005.py --apply
"""

import argparse

import psycopg2

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
WHERE = ("actor_label = 'Ind: Public' AND extraction_method = 'inferred' AND regex_position IS NULL AND regex_drrp IS NULL "
         "AND llm_position IS NULL AND llm_drrp IS NULL AND slm_position IS NULL AND slm_drrp IS NULL AND adj_position IS NULL")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()
    conn = psycopg2.connect(PG)
    cur = conn.cursor()
    cur.execute(f"SELECT count(*), count(DISTINCT split_part(section_id, ':', 1)), count(cls_position) FROM provision_actors WHERE {WHERE}")
    n, laws, cls = cur.fetchone()
    cur.execute("SELECT count(*) FROM provision_actors WHERE actor_label = 'Ind: Public'")
    total = cur.fetchone()[0]
    print(f"retired-rule rows: {n} in {laws} laws ({cls} scored by the classifier); Ind: Public rows in total: {total}")
    if total != n:
        print(f"  {total - n} Ind: Public rows have other tier data and are kept")
    if args.apply:
        cur.execute(f"DELETE FROM provision_actors WHERE {WHERE}")
        assert cur.rowcount == n
        conn.commit()
        print(f"deleted {n}")
    else:
        conn.rollback()
        print("dry run: nothing written")


if __name__ == "__main__":
    main()
