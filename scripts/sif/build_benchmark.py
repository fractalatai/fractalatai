#!/usr/bin/env python3
"""Build benchmark set from OSHA held-out data + QQ test data.

Two components:
1. OSHA held-out: 100 events per class, different seed from training (seed=99 vs 42)
2. QQ test set: all 2,747 events with human SIFp labels (from data/sif.duckdb)

The benchmark is 100% real-world data. NEVER train on any of it.

Usage:
    /usr/bin/python3 scripts/sif/build_benchmark.py [--per-class 100] [--dry-run]
"""

import argparse
import csv
import codecs
import json
import random
import sys
import zipfile
from collections import Counter, defaultdict
from pathlib import Path

import duckdb

OSHA_DIR = Path("data/sif/sources/osha")
LABELS_FILE = Path("data/sif/taxonomy/classifier-labels.json")
DUCKDB_PATH = Path("data/sif.duckdb")
OUTPUT_DIR = Path("data/sif/benchmarks")

# Same column indices as extract_training_data.py
COL_NARRATIVE_WHAT = 33
COL_INCIDENT_DESC = 38
COL_NARRATIVE_INJURY = 36
COL_NARRATIVE_OBJECT = 37
COL_EVENT_CODE = 43
COL_EVENT_TITLE = 44
COL_SOURCE_TITLE = 46
COL_NATURE_TITLE = 40
COL_PART_TITLE = 42
COL_OUTCOME = 23
COL_INDUSTRY = 11
COL_JOB = 17

BENCHMARK_SEED = 99  # Different from training seed (42)


def load_labels():
    with open(LABELS_FILE) as f:
        data = json.load(f)
    oiics_to_label = {}
    label_info = {}
    for label in data["labels"]:
        label_info[label["id"]] = label
        for code in label["oiics"]:
            oiics_to_label[code] = label["id"]
    return oiics_to_label, label_info


def combine_narrative(row):
    parts = []
    what = row[COL_NARRATIVE_WHAT].strip() if len(row) > COL_NARRATIVE_WHAT else ""
    desc = row[COL_INCIDENT_DESC].strip() if len(row) > COL_INCIDENT_DESC else ""
    primary = what if len(what) >= len(desc) else desc
    if primary:
        parts.append(primary)
    injury = row[COL_NARRATIVE_INJURY].strip() if len(row) > COL_NARRATIVE_INJURY else ""
    if injury and injury not in primary:
        parts.append(injury)
    obj = row[COL_NARRATIVE_OBJECT].strip() if len(row) > COL_NARRATIVE_OBJECT else ""
    if obj and obj not in primary:
        parts.append(obj)
    return " ".join(parts)


def extract_osha_benchmark(oiics_to_label, per_class, label_info):
    """Extract a held-out benchmark from OSHA data using a different seed."""
    rng = random.Random(BENCHMARK_SEED)
    buckets = defaultdict(list)
    counts = Counter()

    zips = sorted(OSHA_DIR.glob("ITA_Case_Detail_*.zip"))
    for zip_path in zips:
        print(f"  Streaming {zip_path.name}...")
        with zipfile.ZipFile(zip_path) as zf:
            csv_name = [n for n in zf.namelist() if n.endswith(".csv")][0]
            with zf.open(csv_name) as f:
                reader = csv.reader(codecs.getreader("utf-8")(f, errors="replace"))
                next(reader)  # skip header
                for row in reader:
                    if len(row) <= COL_EVENT_CODE:
                        continue
                    event_code = row[COL_EVENT_CODE].strip()
                    label = oiics_to_label.get(event_code)
                    if not label:
                        continue
                    narrative = combine_narrative(row)
                    if not narrative or len(narrative) < 10:
                        continue

                    counts[label] += 1
                    n = counts[label]
                    if n <= per_class:
                        buckets[label].append({
                            "narrative": narrative,
                            "label": label,
                            "oiics_event_code": event_code,
                            "oiics_event_title": row[COL_EVENT_TITLE].strip() if len(row) > COL_EVENT_TITLE else "",
                            "outcome": row[COL_OUTCOME].strip() if len(row) > COL_OUTCOME else "",
                            "source": "osha_benchmark",
                        })
                    else:
                        j = rng.randint(0, n - 1)
                        if j < per_class:
                            buckets[label][j] = {
                                "narrative": narrative,
                                "label": label,
                                "oiics_event_code": event_code,
                                "oiics_event_title": row[COL_EVENT_TITLE].strip() if len(row) > COL_EVENT_TITLE else "",
                                "outcome": row[COL_OUTCOME].strip() if len(row) > COL_OUTCOME else "",
                                "source": "osha_benchmark",
                            }

    return dict(buckets)


def extract_qq_benchmark():
    """Extract QQ test set from DuckDB."""
    con = duckdb.connect(str(DUCKDB_PATH), read_only=True)
    rows = con.execute("""
        SELECT event_id, narrative, report_type, qq_sifp, hazard_category
        FROM events
        WHERE narrative IS NOT NULL AND LENGTH(narrative) > 10
    """).fetchall()
    con.close()

    records = []
    for row in rows:
        records.append({
            "event_id": row[0],
            "narrative": row[1],
            "report_type": row[2],
            "qq_sifp": row[3],
            "hazard_category": row[4],
            "source": "qq_benchmark",
        })
    return records


def main():
    parser = argparse.ArgumentParser(description="Build SIF benchmark set")
    parser.add_argument("--per-class", type=int, default=100, help="OSHA events per class (default: 100)")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    print("Loading classifier labels...")
    oiics_to_label, label_info = load_labels()

    # Component 1: OSHA held-out
    print(f"\nExtracting OSHA benchmark (seed={BENCHMARK_SEED}, {args.per_class}/class)...")
    osha_buckets = extract_osha_benchmark(oiics_to_label, args.per_class, label_info)

    osha_total = sum(len(v) for v in osha_buckets.values())
    print(f"\nOSHA benchmark: {osha_total:,} events")
    for label_id in sorted(osha_buckets.keys()):
        n = len(osha_buckets[label_id])
        gate = label_info[label_id]["gate"]
        print(f"  {label_id:25s}  {n:>4}  {gate}")

    # Component 2: QQ test set
    print(f"\nExtracting QQ benchmark from DuckDB...")
    qq_records = extract_qq_benchmark()
    print(f"QQ benchmark: {len(qq_records):,} events")

    sifp_dist = Counter(r["qq_sifp"] for r in qq_records)
    for k, v in sifp_dist.most_common():
        print(f"  {k:30s}  {v:>5}")

    print(f"\nTotal benchmark: {osha_total + len(qq_records):,} events (100% real)")

    if args.dry_run:
        print("\nDRY RUN — no output written")
        return

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Save OSHA benchmark
    osha_all = []
    for records in osha_buckets.values():
        osha_all.extend(records)
    random.Random(BENCHMARK_SEED).shuffle(osha_all)

    osha_path = OUTPUT_DIR / "osha_benchmark.json"
    with open(osha_path, "w") as f:
        json.dump(osha_all, f, indent=2)
    print(f"\n→ {osha_path} ({len(osha_all):,} events)")

    # Save QQ benchmark
    qq_path = OUTPUT_DIR / "qq_benchmark.json"
    with open(qq_path, "w") as f:
        json.dump(qq_records, f, indent=2)
    print(f"→ {qq_path} ({len(qq_records):,} events)")

    # Save summary
    summary = {
        "total": osha_total + len(qq_records),
        "osha_benchmark": {"events": osha_total, "per_class": args.per_class, "seed": BENCHMARK_SEED},
        "qq_benchmark": {"events": len(qq_records), "sifp_distribution": dict(sifp_dist)},
        "note": "100% real-world data. NEVER train on this.",
    }
    with open(OUTPUT_DIR / "benchmark_summary.json", "w") as f:
        json.dump(summary, f, indent=2)


if __name__ == "__main__":
    main()
