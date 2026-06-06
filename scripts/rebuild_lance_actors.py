#!/usr/bin/env python3
"""Rebuild LanceDB legislation_text table with native Arrow actors struct.

Converts the `actors` column from JSON string (Utf8) to a native Arrow
List<Struct(label: Utf8, role: Utf8, recipient_type: Utf8)>.

Steps:
  1. Export current table to Parquet backup
  2. Read back, convert actors column
  3. Drop and recreate table with new schema

Usage:
  /usr/bin/python3 scripts/rebuild_lance_actors.py [--dry-run]
"""
import json
import sys
from datetime import datetime
from pathlib import Path

import lancedb
import pyarrow as pa
import pyarrow.parquet as pq

DB_PATH = "data/lancedb"
TABLE_NAME = "legislation_text"
BACKUP_DIR = Path("backups")

# Target Arrow type for actors column
ACTORS_TYPE = pa.list_(pa.struct([
    pa.field("label", pa.string(), nullable=False),
    pa.field("role", pa.string(), nullable=False),
    pa.field("recipient_type", pa.string(), nullable=True),
    pa.field("label_source", pa.string(), nullable=False),
    pa.field("reason", pa.string(), nullable=True),
]))


def convert_actors_column(table: pa.Table) -> pa.Table:
    """Convert actors column from JSON string to native Arrow struct."""
    actors_idx = table.schema.get_field_index("actors")
    actors_col = table.column(actors_idx)

    # Parse JSON strings into Python lists of dicts, then build Arrow array
    entries = []
    for val in actors_col:
        if val is None or not val.is_valid:
            entries.append(None)
        else:
            text = val.as_py()
            if not text:
                entries.append(None)
            else:
                try:
                    parsed = json.loads(text)
                    entries.append([
                        {
                            "label": a.get("label", ""),
                            "role": a.get("role", ""),
                            "recipient_type": a.get("recipient_type"),
                        }
                        for a in parsed
                    ])
                except (json.JSONDecodeError, TypeError):
                    entries.append(None)

    new_actors = pa.array(entries, type=ACTORS_TYPE)

    # Replace the column
    columns = list(range(table.num_columns))
    columns.remove(actors_idx)
    new_table = table.select(columns)

    # Insert at original position
    new_table = new_table.add_column(actors_idx, pa.field("actors", ACTORS_TYPE, nullable=True), new_actors)
    return new_table


def main():
    dry_run = "--dry-run" in sys.argv

    db = lancedb.connect(DB_PATH)
    table = db.open_table(TABLE_NAME)

    row_count = table.count_rows()
    schema = table.schema
    print(f"Current table: {row_count:,} rows, {len(schema)} columns")

    # Check current actors type
    actors_field = schema.field("actors")
    print(f"Current actors type: {actors_field.type}")
    if actors_field.type != pa.string():
        print("ERROR: actors column is not string type — already migrated?")
        sys.exit(1)

    # Step 1: Export to Parquet
    BACKUP_DIR.mkdir(exist_ok=True)
    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    backup_path = BACKUP_DIR / f"legislation_text_pre_rebuild_{stamp}.parquet"

    print(f"\nStep 1: Exporting to {backup_path} ...")
    arrow_table = table.to_arrow()
    pq.write_table(arrow_table, str(backup_path))
    backup_rows = pq.read_metadata(str(backup_path)).num_rows
    print(f"  Exported {backup_rows:,} rows ({backup_path.stat().st_size / 1024 / 1024:.1f} MB)")

    assert backup_rows == row_count, f"Row count mismatch: {backup_rows} vs {row_count}"

    # Step 2: Convert actors column
    print("\nStep 2: Converting actors column to native Arrow struct ...")
    converted = convert_actors_column(arrow_table)
    new_actors_field = converted.schema.field("actors")
    print(f"  New actors type: {new_actors_field.type}")

    # Count non-null actors
    non_null = sum(1 for v in converted.column("actors") if v.is_valid)
    print(f"  Non-null actors entries: {non_null:,}")

    if dry_run:
        print("\n[DRY RUN] Would drop and recreate table. Exiting.")
        return

    # Step 3: Drop and recreate
    print(f"\nStep 3: Dropping and recreating {TABLE_NAME} ...")
    db.drop_table(TABLE_NAME)
    new_table = db.create_table(TABLE_NAME, data=converted)

    # Step 4: Verify
    verify_count = new_table.count_rows()
    verify_schema = new_table.schema
    verify_actors = verify_schema.field("actors")

    print(f"\nVerification:")
    print(f"  Rows: {verify_count:,} (expected {row_count:,})")
    print(f"  Columns: {len(verify_schema)} (expected {len(schema)})")
    print(f"  Actors type: {verify_actors.type}")

    # Check embeddings survived
    sample = new_table.search().limit(1).to_arrow()
    emb_col = sample.column("embedding")
    has_emb = emb_col[0].is_valid if len(emb_col) > 0 else False
    print(f"  Embeddings present: {has_emb}")

    assert verify_count == row_count, f"Row count mismatch after rebuild!"
    assert verify_actors.type == ACTORS_TYPE, f"Actors type mismatch!"
    print("\nRebuild complete.")


if __name__ == "__main__":
    main()
