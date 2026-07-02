#!/usr/bin/env /usr/bin/python3
"""Migrate LanceDB: convert drrp_history from List<Struct> to JSON string.

Fixes Lance offset panics (issue #45) by replacing the nested List<Struct>
column with a simple Utf8 column containing JSON arrays.

Creates a Parquet backup before rebuilding.

Usage:
    /usr/bin/python3 scripts/migrate_drrp_history_json.py
    /usr/bin/python3 scripts/migrate_drrp_history_json.py --dry-run
"""

import argparse
import json
import os
from datetime import datetime, timezone

import lancedb
import pyarrow as pa

DB_PATH = "data/lancedb"
TABLE_NAME = "legislation_text"
BACKUP_PATH = "backups/legislation_text_pre_drrp_json.arrow"


def main():
    parser = argparse.ArgumentParser(description="Convert drrp_history List<Struct> → JSON string")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    db = lancedb.connect(DB_PATH)
    table = db.open_table(TABLE_NAME)
    schema = table.schema

    # Check current column type
    hist_field = None
    for f in schema:
        if f.name == "drrp_history":
            hist_field = f
            break

    if hist_field is None:
        print("drrp_history column not found. Nothing to migrate.")
        return

    if hist_field.type == pa.string() or hist_field.type == pa.utf8():
        print("drrp_history is already a string column. Nothing to do.")
        return

    print(f"Current drrp_history type: {hist_field.type}")
    rows = table.count_rows()
    print(f"Exporting {rows} rows to memory...")
    arrow = table.to_arrow()
    print(f"Loaded {arrow.num_rows} rows")

    if args.dry_run:
        # Show a sample
        hist_col = arrow.column("drrp_history")
        sample_count = 0
        for i in range(min(5, arrow.num_rows)):
            val = hist_col[i].as_py()
            if val is not None:
                print(f"  Row {i}: {json.dumps(val)}")
                sample_count += 1
                if sample_count >= 3:
                    break
        print("DRY RUN — would convert List<Struct> → JSON string")
        return

    # Backup — use Arrow IPC (Parquet fails on corrupted List<Struct> nulls)
    os.makedirs("backups", exist_ok=True)
    with pa.ipc.new_file(pa.OSFile(BACKUP_PATH, "wb"), arrow.schema) as writer:
        writer.write_table(arrow)
    backup_size = os.path.getsize(BACKUP_PATH)
    print(f"Backup: {BACKUP_PATH} ({backup_size / 1024 / 1024:.0f} MB)")

    # Convert List<Struct> → JSON string
    print("Converting drrp_history to JSON strings...")
    hist_col = arrow.column("drrp_history")
    json_values = []
    populated = 0

    for i in range(arrow.num_rows):
        val = hist_col[i].as_py()
        if val is not None:
            json_values.append(json.dumps(val))
            populated += 1
        else:
            json_values.append(None)

    print(f"  Converted: {populated}, Null: {arrow.num_rows - populated}")

    # Replace column
    col_idx = arrow.schema.get_field_index("drrp_history")
    new_col = pa.array(json_values, type=pa.string())
    new_table = arrow.set_column(col_idx, pa.field("drrp_history", pa.string(), nullable=True), new_col)

    print(f"Schema columns: {len(new_table.schema)}")

    # Rebuild
    db.drop_table(TABLE_NAME)
    print("Dropped old table")
    db.create_table(TABLE_NAME, data=new_table)
    new_rows = db.open_table(TABLE_NAME).count_rows()
    print(f"Recreated: {new_rows} rows")

    import subprocess
    r = subprocess.run(["du", "-sh", DB_PATH], capture_output=True, text=True)
    print(r.stdout.strip())
    r2 = subprocess.run(["df", "-h", "/var/home"], capture_output=True, text=True)
    print(r2.stdout.strip().split("\n")[-1])


if __name__ == "__main__":
    main()
