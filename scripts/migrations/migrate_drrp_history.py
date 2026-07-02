#!/usr/bin/env /usr/bin/python3
"""Migrate LanceDB: add drrp_history column to legislation_text.

Rebuilds the table with a new List<Struct> column populated from
existing extraction_method + drrp_types + taxa_confidence.

Creates an Arrow IPC backup before rebuilding.

Usage:
    /usr/bin/python3 scripts/migrate_drrp_history.py
    /usr/bin/python3 scripts/migrate_drrp_history.py --dry-run
"""

import argparse
import os
from datetime import datetime, timezone

import lancedb
import pyarrow as pa

DB_PATH = "data/lancedb"
TABLE_NAME = "legislation_text"
BACKUP_PATH = "backups/legislation_text_pre_drrp_history.arrow"


def main():
    parser = argparse.ArgumentParser(description="Add drrp_history column")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    db = lancedb.connect(DB_PATH)
    table = db.open_table(TABLE_NAME)

    # Check if column already exists
    schema = table.schema
    if "drrp_history" in [f.name for f in schema]:
        print("drrp_history column already exists. Nothing to do.")
        return

    rows = table.count_rows()
    print(f"Exporting {rows} rows to memory...")
    arrow = table.to_arrow()
    print(f"Loaded {arrow.num_rows} rows")

    if args.dry_run:
        print("DRY RUN — would rebuild table with drrp_history column")
        return

    # Backup
    os.makedirs("backups", exist_ok=True)
    with pa.ipc.new_file(pa.OSFile(BACKUP_PATH, "wb"), arrow.schema) as writer:
        writer.write_table(arrow)
    backup_size = os.path.getsize(BACKUP_PATH)
    print(f"Backup: {BACKUP_PATH} ({backup_size / 1024 / 1024:.0f} MB)")

    # Build drrp_history column from existing data
    print("Building drrp_history from existing extraction_method + drrp_types...")

    drrp_history_type = pa.list_(
        pa.struct([
            pa.field("tier", pa.string(), nullable=False),
            pa.field("drrp", pa.string(), nullable=False),
            pa.field("confidence", pa.float32(), nullable=True),
            pa.field("timestamp", pa.string(), nullable=True),
        ])
    )

    history_values = []
    now_iso = datetime.now(timezone.utc).isoformat()

    extraction_col = arrow.column("extraction_method") if "extraction_method" in arrow.schema.names else None
    drrp_col = arrow.column("drrp_types") if "drrp_types" in arrow.schema.names else None
    conf_col = arrow.column("taxa_confidence") if "taxa_confidence" in arrow.schema.names else None

    populated = 0
    for i in range(arrow.num_rows):
        method = extraction_col[i].as_py() if extraction_col is not None else None
        drrp = drrp_col[i].as_py() if drrp_col is not None else None
        conf = conf_col[i].as_py() if conf_col is not None else None

        if method and drrp:
            drrp_val = drrp[0] if drrp else "none"
            history_values.append([{
                "tier": method,
                "drrp": drrp_val,
                "confidence": float(conf) if conf is not None else None,
                "timestamp": now_iso,
            }])
            populated += 1
        elif method:
            history_values.append([{
                "tier": method,
                "drrp": "none",
                "confidence": float(conf) if conf is not None else None,
                "timestamp": now_iso,
            }])
            populated += 1
        else:
            history_values.append(None)

    print(f"  Populated: {populated}, Null: {arrow.num_rows - populated}")

    # Add column to table
    history_array = pa.array(history_values, type=drrp_history_type)
    new_table = arrow.append_column("drrp_history", history_array)

    print(f"New schema has {len(new_table.schema)} columns (was {len(arrow.schema)})")

    # Rebuild
    db.drop_table(TABLE_NAME)
    print("Dropped old table")
    db.create_table(TABLE_NAME, data=new_table)
    print(f"Recreated: {db.open_table(TABLE_NAME).count_rows()} rows")

    import subprocess
    r = subprocess.run(["du", "-sh", DB_PATH], capture_output=True, text=True)
    print(r.stdout.strip())
    r2 = subprocess.run(["df", "-h", "/var/home"], capture_output=True, text=True)
    print(r2.stdout.strip().split("\n")[-1])


if __name__ == "__main__":
    main()
