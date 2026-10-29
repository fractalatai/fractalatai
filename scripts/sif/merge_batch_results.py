#!/usr/bin/env python3
"""Merge a RunPod SIF batch's results into the full result files used by the export.

A batch directory (downloaded from /workspace/sif/batches/<date>/output) holds
zeroshot/predictions.json (Pass 1) and band_selection.json (Pass 2). Results replace any
existing entry with the same event_id (re-inference after a narrative change), otherwise append.
Afterwards, narrative_changed_at is cleared for the re-inferred events.

Usage:
    /usr/bin/python3 scripts/sif/merge_batch_results.py data/sif/batches/20261029 [--dry-run]
"""

import argparse
import json
import shutil
from datetime import date
from pathlib import Path

import duckdb

DUCKDB_PATH = Path("data/sif.duckdb")
TARGETS = {
    "zeroshot/predictions.json": Path("data/sif/benchmarks/qq_zeroshot_full.json"),
    "band_selection.json": Path("data/sif/benchmarks/band_selection_full.json"),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("batch_dir", type=Path)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    merged_ids = set()
    for rel, target in TARGETS.items():
        batch = json.load(open(args.batch_dir / "output" / rel))
        full = json.load(open(target))
        by_id = {e["event_id"]: e for e in full}
        replaced = sum(1 for e in batch if e["event_id"] in by_id)
        errors = sum(1 for e in batch if "error" in e.get("prediction", {}))
        for e in batch:
            by_id[e["event_id"]] = e
        merged_ids |= {e["event_id"] for e in batch}
        print(f"{target}: {len(full):,} + {len(batch) - replaced} new, {replaced} replaced "
              f"→ {len(by_id):,} ({errors} errors in batch)")
        if not args.dry_run:
            backup = target.with_name(f"{target.stem}_{date.today():%Y%m%d}_pre_merge.json")
            if not backup.exists():
                shutil.copy2(target, backup)
            tmp = target.with_suffix(".json.tmp")
            json.dump(list(by_id.values()), open(tmp, "w"), indent=2)
            tmp.replace(target)

    if args.dry_run:
        print("DRY RUN — no changes written")
        return
    con = duckdb.connect(str(DUCKDB_PATH))
    cleared = con.execute(
        "UPDATE events SET narrative_changed_at = NULL "
        "WHERE narrative_changed_at IS NOT NULL AND event_id IN (SELECT unnest(?))",
        [sorted(merged_ids)]).fetchone()[0]
    con.close()
    print(f"narrative_changed_at cleared on {cleared} events")


if __name__ == "__main__":
    main()
