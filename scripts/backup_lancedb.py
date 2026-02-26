#!/usr/bin/env python3
"""
Backup LanceDB table to Parquet.

Exports the legislation_text table (with embeddings) to a timestamped Parquet file.
This preserves the 9+ hours of embedding computation work.

Usage:
    python scripts/backup_lancedb.py
    python scripts/backup_lancedb.py --output backups/custom_name.parquet
"""

import argparse
from datetime import datetime
from pathlib import Path

import lance
import pyarrow.dataset as pa_dataset


def backup_lancedb(lance_path: Path, output_path: Path):
    """Export LanceDB table to Parquet."""
    print(f"Reading from: {lance_path}")

    if not lance_path.exists():
        print(f"Error: LanceDB table not found at {lance_path}")
        return 1

    # Open Lance dataset
    ds = lance.dataset(str(lance_path))

    # Get schema and row count
    schema = ds.schema
    count = ds.count_rows()

    print(f"Schema: {len(schema)} columns")
    print(f"Rows: {count:,}")

    # Check for embeddings
    has_embeddings = "embedding" in schema.names
    has_tokens = "token_ids" in schema.names

    if has_embeddings:
        print("✓ Contains embeddings (384-dim)")
    else:
        print("⚠ No embeddings found")

    if has_tokens:
        print("✓ Contains token_ids")

    # Create output directory if needed
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Export to Parquet
    print(f"\nExporting to: {output_path}")
    pa_dataset.write_dataset(
        ds.scanner().to_reader(),
        str(output_path),
        format="parquet",
    )

    # Verify backup
    backup_size = output_path.stat().st_size / (1024 * 1024)  # MB
    print(f"✓ Backup complete: {backup_size:.1f} MB")

    return 0


def main():
    parser = argparse.ArgumentParser(description="Backup LanceDB to Parquet")
    parser.add_argument(
        "--lance-path",
        type=Path,
        default=Path("data/lancedb/legislation_text.lance"),
        help="Path to LanceDB table (default: data/lancedb/legislation_text.lance)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="Output Parquet file path (default: backups/legislation_text_YYYYMMDD_HHMMSS.parquet)",
    )

    args = parser.parse_args()

    # Generate default output path with timestamp
    if args.output is None:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        args.output = Path(f"backups/legislation_text_{timestamp}.parquet")

    return backup_lancedb(args.lance_path, args.output)


if __name__ == "__main__":
    exit(main())
