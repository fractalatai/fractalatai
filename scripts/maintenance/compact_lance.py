#!/usr/bin/env python3
"""Compact LanceDB by rebuilding from in-memory Arrow table.

LanceDB compaction requires pylance which isn't installed.
This script reads the table into memory, drops it, and recreates
it — effectively a full compaction.

Backup is Arrow IPC (not Parquet) because Parquet doesn't support
null components inside list fields (actors struct has nullable reason).
"""
import os
import lancedb
import pyarrow as pa
import subprocess

DB_PATH = "data/lancedb"
TABLE_NAME = "legislation_text"
BACKUP_PATH = "backups/legislation_text_backup.arrow"

os.makedirs("backups", exist_ok=True)

db = lancedb.connect(DB_PATH)
table = db.open_table(TABLE_NAME)
rows = table.count_rows()
print(f"Exporting {rows} rows to memory...")
arrow = table.to_arrow()

# Arrow IPC backup — handles nulls in nested structs (Parquet doesn't)
with pa.ipc.new_file(pa.OSFile(BACKUP_PATH, "wb"), arrow.schema) as writer:
    writer.write_table(arrow)
backup_size = os.path.getsize(BACKUP_PATH)
print(f"Backup: {BACKUP_PATH} ({backup_size / 1024 / 1024:.0f} MB)")

db.drop_table(TABLE_NAME)
print("Dropped old table")

new_table = db.create_table(TABLE_NAME, data=arrow)
print(f"Recreated: {new_table.count_rows()} rows")

result = subprocess.run(["du", "-sh", "data/lancedb/"], capture_output=True, text=True)
print(result.stdout.strip())
result2 = subprocess.run(["df", "-h", "/var/home"], capture_output=True, text=True)
lines = result2.stdout.strip().split("\n")
print(lines[-1] if len(lines) > 1 else result2.stdout.strip())
