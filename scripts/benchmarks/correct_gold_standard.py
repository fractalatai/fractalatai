#!/usr/bin/env /usr/bin/python3
"""Correct golden benchmark Parquet files: 5-class → 3-class + offence removal.

Rewrites benchmark files on NAS:
1. Duty/Responsibility → Obligation
2. Right/Power → Liberty
3. Offence provisions (no modal, offence language) → none

Usage:
    # Dry run — show what would change
    /usr/bin/python3 scripts/correct_gold_standard.py --dry-run

    # Apply corrections to NAS
    /usr/bin/python3 scripts/correct_gold_standard.py

    # Apply to a local copy first
    /usr/bin/python3 scripts/correct_gold_standard.py --output-dir data/corrected-benchmarks
"""

import argparse
import glob
import os
import re
import sys
from collections import Counter

import pyarrow as pa
import pyarrow.parquet as pq

BENCHMARK_DIR = "/mnt/nas/sertantai-data/data/fractalaw-benchmarks"

# Offence detection: no obligation modal + offence language
HAS_OBLIGATION_MODAL = re.compile(r"(?i)\bshall\b|\bmust\b|\bis required to\b|\bhas a duty\b")
HAS_OFFENCE = re.compile(
    r"(?i)\bguilty of an offence\b|\bcommits an offence\b|\bis an offence\b"
    r"|\bliable on (?:summary )?conviction\b|\bliable to a fine\b"
    r"|\bshall be liable\b|\bpunishable\b"
)


def map_drrp(drrp_list, text):
    """Map 5-class DRRP to 3-class, with offence detection."""
    if not drrp_list:
        return [], "unchanged"

    original = drrp_list[0]

    # Check for offence provisions: gold says Duty but text is offence language
    # with no obligation modal
    if original in ("Duty", "Responsibility"):
        if HAS_OFFENCE.search(text) and not HAS_OBLIGATION_MODAL.search(text):
            return [], "offence→none"

    # Map to 3-class
    mapped = []
    for d in drrp_list:
        if d in ("Duty", "Responsibility"):
            mapped.append("Obligation")
        elif d in ("Right", "Power"):
            mapped.append("Liberty")
        else:
            mapped.append(d)

    if mapped == drrp_list:
        return mapped, "unchanged"

    return mapped, f"{original}→{mapped[0]}"


def process_file(filepath, output_dir=None, dry_run=False):
    """Process a single benchmark Parquet file."""
    table = pq.read_table(filepath)
    basename = os.path.basename(filepath)
    n_rows = table.num_rows

    changes = Counter()
    new_drrp_lists = []

    for i in range(n_rows):
        gold_drrp = table.column("gold_drrp_types")[i].as_py() or []
        text = table.column("text")[i].as_py() or ""

        new_drrp, change_type = map_drrp(gold_drrp, text)
        new_drrp_lists.append(new_drrp)
        changes[change_type] += 1

    # Report
    changed = sum(c for k, c in changes.items() if k != "unchanged")
    print(f"  {basename}: {n_rows} rows, {changed} changed")
    for change, count in sorted(changes.items()):
        if change != "unchanged":
            print(f"    {change}: {count}")

    if dry_run or changed == 0:
        return changes

    # Build new table with corrected drrp_types
    # Replace the gold_drrp_types column
    columns = {}
    for col_name in table.schema.names:
        if col_name == "gold_drrp_types":
            columns[col_name] = pa.array(
                [lst if lst else None for lst in new_drrp_lists],
                type=pa.list_(pa.string()),
            )
        else:
            columns[col_name] = table.column(col_name)

    new_table = pa.table(columns, schema=table.schema)

    # Write
    out_path = os.path.join(output_dir or os.path.dirname(filepath), basename)
    pq.write_table(new_table, out_path)
    print(f"    → wrote {out_path}")

    return changes


def main():
    parser = argparse.ArgumentParser(description="Correct gold standard benchmarks")
    parser.add_argument("--dry-run", action="store_true", help="Show changes without writing")
    parser.add_argument("--output-dir", help="Write to this directory instead of NAS")
    args = parser.parse_args()

    if not os.path.isdir(BENCHMARK_DIR):
        print(f"NAS not mounted at {BENCHMARK_DIR}")
        sys.exit(1)

    files = sorted(glob.glob(os.path.join(BENCHMARK_DIR, "tier2-*.parquet")))
    if not files:
        print(f"No benchmark files found in {BENCHMARK_DIR}")
        sys.exit(1)

    # NEVER write directly to NAS — block-padding corrupts Parquet files.
    # Always write locally first, then copy to NAS.
    output_dir = args.output_dir or "data/benchmarks"
    os.makedirs(output_dir, exist_ok=True)

    print(f"{'DRY RUN — ' if args.dry_run else ''}Correcting {len(files)} benchmark files")
    print(f"Output: {output_dir}\n")

    total_changes = Counter()
    total_rows = 0

    for f in files:
        changes = process_file(f, output_dir, args.dry_run)
        for k, v in changes.items():
            total_changes[k] += v
        total_rows += sum(changes.values())

    print(f"\n{'=' * 60}")
    print(f"Total: {total_rows} rows")
    for change, count in sorted(total_changes.items()):
        pct = 100 * count / total_rows if total_rows else 0
        print(f"  {change}: {count} ({pct:.1f}%)")

    changed = sum(c for k, c in total_changes.items() if k != "unchanged")
    print(f"\n  Changed: {changed} ({100*changed/total_rows:.1f}%)")
    print(f"  Unchanged: {total_changes['unchanged']} ({100*total_changes['unchanged']/total_rows:.1f}%)")


if __name__ == "__main__":
    main()
