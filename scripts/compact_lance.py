#!/usr/bin/env python3
"""Compact LanceDB by rebuilding from in-memory Arrow table.

LanceDB compaction requires pylance which isn't installed.
This script reads the table into memory, drops it, and recreates
it — effectively a full compaction.
"""
import lancedb
import pyarrow.parquet as pq
import subprocess

DB_PATH = "data/lancedb"
TABLE_NAME = "legislation_text"

db = lancedb.connect(DB_PATH)
table = db.open_table(TABLE_NAME)
rows = table.count_rows()
print(f"Exporting {rows} rows to memory...")
arrow = table.to_arrow()

pq.write_table(arrow, "backups/legislation_text_mid_enrich.parquet")
print("Safety backup written to backups/legislation_text_mid_enrich.parquet")

db.drop_table(TABLE_NAME)
print("Dropped old table")

new_table = db.create_table(TABLE_NAME, data=arrow)
print(f"Recreated: {new_table.count_rows()} rows")

result = subprocess.run(["du", "-sh", "data/lancedb/"], capture_output=True, text=True)
print(result.stdout.strip())
result2 = subprocess.run(["df", "-h", "/var/home"], capture_output=True, text=True)
lines = result2.stdout.strip().split("\n")
print(lines[-1] if len(lines) > 1 else result2.stdout.strip())
