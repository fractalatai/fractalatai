#!/usr/bin/env python3
"""Derive significance_hierarchy from metadata (law type + depth).

Runs after significance SLM batch. Sets hierarchy based on law-type-relative
depth thresholds — Acts are deeper than SIs/EU Directives.

Usage:
    /usr/bin/python3 .claude/skills/customer-batch-parse/scripts/derive_hierarchy.py
    /usr/bin/python3 .claude/skills/customer-batch-parse/scripts/derive_hierarchy.py --law-file data/qq-applicable-laws.csv
    /usr/bin/python3 .claude/skills/customer-batch-parse/scripts/derive_hierarchy.py --laws UK_ukpga_1974_37
"""

import argparse
import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"


def main():
    parser = argparse.ArgumentParser(description="Derive significance hierarchy from metadata")
    parser.add_argument("--laws", help="Comma-separated law names")
    parser.add_argument("--law-file", help="CSV file with law names")
    args = parser.parse_args()

    law_filter = ""
    if args.laws:
        names = [f"'{n.strip()}'" for n in args.laws.split(",")]
        law_filter = f"AND lt.law_name IN ({','.join(names)})"
    elif args.law_file:
        with open(args.law_file) as f:
            csv_laws = [n.strip() for n in f.read().split(",") if n.strip()]
        names = [f"'{n}'" for n in csv_laws]
        law_filter = f"AND lt.law_name IN ({','.join(names)})"

    conn = psycopg2.connect(PG_DSN)
    cur = conn.cursor()

    sql = f"""
        UPDATE legislation_text lt SET significance_hierarchy =
          CASE
            -- Acts (ukpga, asp, anaw, apni): sub_section at depth 4 is primary duty level
            WHEN lt.law_name ~ '(ukpga|asp|anaw|apni)' THEN
              CASE
                WHEN lt.depth <= 4 THEN 'HIGH'
                WHEN lt.depth = 5 THEN 'MEDIUM'
                ELSE 'LOW'
              END
            -- SIs (uksi, wsi): flatter structure
            WHEN lt.law_name ~ '(uksi|wsi)' THEN
              CASE
                WHEN lt.depth <= 2 THEN 'HIGH'
                WHEN lt.depth <= 3 THEN 'MEDIUM'
                ELSE 'LOW'
              END
            -- EU Directives (eudr): flat
            WHEN lt.law_name ~ 'eudr' THEN
              CASE
                WHEN lt.depth <= 2 THEN 'HIGH'
                WHEN lt.depth <= 3 THEN 'MEDIUM'
                ELSE 'LOW'
              END
            -- EU Regulations / other
            ELSE
              CASE
                WHEN lt.depth <= 2 THEN 'HIGH'
                WHEN lt.depth <= 3 THEN 'MEDIUM'
                ELSE 'LOW'
              END
          END
        WHERE lt.significance_gravity IS NOT NULL
        {law_filter}
    """

    cur.execute(sql)
    count = cur.rowcount
    conn.commit()
    cur.close()
    conn.close()

    print(f"Derived hierarchy for {count:,} provisions")


if __name__ == "__main__":
    main()
