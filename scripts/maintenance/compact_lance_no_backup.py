#!/usr/bin/env python3
"""Compact LanceDB without Parquet backup (emergency disk recovery)."""
import lancedb

db = lancedb.connect("data/lancedb")
table = db.open_table("legislation_text")
rows = table.count_rows()
print(f"Exporting {rows} rows to memory...")
arrow = table.to_arrow()
print("Loaded. Dropping old table...")
db.drop_table("legislation_text")
print("Dropped. Recreating...")
new_table = db.create_table("legislation_text", data=arrow)
print(f"Recreated: {new_table.count_rows()} rows")

import subprocess
result = subprocess.run(["du", "-sh", "data/lancedb/"], capture_output=True, text=True)
print(result.stdout.strip())
result2 = subprocess.run(["df", "-h", "/var/home"], capture_output=True, text=True)
lines = result2.stdout.strip().split("\n")
print(lines[-1])
