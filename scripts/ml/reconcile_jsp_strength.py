#!/usr/bin/env python3
"""Reconcile JSP obligation strength from PG classifier into DuckDB.

Reads cls_strength from PG (inference store), writes reconciled
obligation_strength to DuckDB (publish store).

Reconciliation: classifier > regex (classifier has embedding context).

Usage:
    /usr/bin/python3 scripts/ml/reconcile_jsp_strength.py
"""

import duckdb
import psycopg2

def main():
    # Read classifier results from PG
    pg = psycopg2.connect(
        host="localhost", port=5433, dbname="fractalaw",
        user="fractalaw", password="fractalaw"
    )
    cur = pg.cursor()
    cur.execute("SELECT section_id, cls_strength FROM jsp_provisions WHERE cls_strength IS NOT NULL")
    cls_map = {r[0]: r[1] for r in cur.fetchall()}
    cur.close()
    pg.close()
    print(f"Classifier results from PG: {len(cls_map)}")

    # Update DuckDB enrichment with reconciled strength
    duck = duckdb.connect("data/fractalaw.duckdb")

    updated = 0
    for sid, cls_strength in cls_map.items():
        duck.execute("""
            UPDATE jsp_enrichment
            SET obligation_strength = ?
            WHERE section_id = ?
        """, [cls_strength, sid])
        updated += 1

    duck.close()
    print(f"Reconciled {updated} rows in DuckDB (classifier > regex)")


if __name__ == "__main__":
    main()
