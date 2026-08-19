#!/usr/bin/env python3
"""Extract stratified training data from OSHA ITA Case Detail files.

Streams through OSHA zip files, maps OIICS event codes to classifier labels,
randomly samples up to CAP per class, and outputs a training-ready Parquet file.

The OSHA data is TRAINING data. QQ data is TEST/BENCHMARK ONLY — never mixed in here.

Usage:
    /usr/bin/python3 scripts/sif/extract_training_data.py [--cap 5000] [--seed 42] [--dry-run]
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

OSHA_DIR = Path("data/sif/sources/osha")
LABELS_FILE = Path("data/sif/taxonomy/classifier-labels.json")
OUTPUT_DIR = Path("data/sif/training")

# OSHA CSV column indices
COL_NARRATIVE_WHAT = 33       # NEW_NAR_WHAT_HAPPENED
COL_NARRATIVE_BEFORE = 34     # NEW_NAR_BEFORE_INCIDENT
COL_INCIDENT_LOCATION = 35   # NEW_INCIDENT_LOCATION
COL_NARRATIVE_INJURY = 36    # NEW_NAR_INJURY_ILLNESS
COL_NARRATIVE_OBJECT = 37    # NEW_NAR_OBJECT_SUBSTANCE
COL_INCIDENT_DESC = 38       # NEW_INCIDENT_DESCRIPTION
COL_EVENT_CODE = 43          # event_code_pred
COL_EVENT_TITLE = 44         # event_title_pred
COL_SOURCE_TITLE = 46        # source_title_pred
COL_NATURE_TITLE = 40        # nature_title_pred
COL_PART_TITLE = 42          # part_title_pred
COL_OUTCOME = 23             # incident_outcome (1=death, 2=DAFW, 3=DJTR, 4=other)
COL_INDUSTRY = 11            # industry_description
COL_JOB = 17                 # job_description


def load_labels() -> tuple[dict, dict]:
    """Load classifier labels. Returns (oiics_to_label, label_info) mappings."""
    with open(LABELS_FILE) as f:
        data = json.load(f)

    oiics_to_label = {}
    label_info = {}
    for label in data["labels"]:
        label_info[label["id"]] = label
        for code in label["oiics"]:
            oiics_to_label[code] = label["id"]

    return oiics_to_label, label_info


def combine_narrative(row: list) -> str:
    """Combine OSHA narrative fields into a single text.

    Prioritises NEW_NAR_WHAT_HAPPENED (Form 301 — most detailed),
    falls back to NEW_INCIDENT_DESCRIPTION (Form 300 — shorter).
    Appends injury and object fields if present.
    """
    parts = []

    # Primary narrative: what happened
    what = row[COL_NARRATIVE_WHAT].strip() if len(row) > COL_NARRATIVE_WHAT else ""
    desc = row[COL_INCIDENT_DESC].strip() if len(row) > COL_INCIDENT_DESC else ""

    # Use the longer of what_happened vs incident_description
    primary = what if len(what) >= len(desc) else desc
    if primary:
        parts.append(primary)

    # Append injury description if it adds info
    injury = row[COL_NARRATIVE_INJURY].strip() if len(row) > COL_NARRATIVE_INJURY else ""
    if injury and injury not in primary:
        parts.append(injury)

    # Append object/substance if it adds info
    obj = row[COL_NARRATIVE_OBJECT].strip() if len(row) > COL_NARRATIVE_OBJECT else ""
    if obj and obj not in primary:
        parts.append(obj)

    return " ".join(parts)


def stream_osha_zip(zip_path: Path):
    """Stream rows from an OSHA zip file, yielding parsed CSV rows."""
    with zipfile.ZipFile(zip_path) as zf:
        names = zf.namelist()
        csv_name = [n for n in names if n.endswith(".csv")][0]
        with zf.open(csv_name) as f:
            reader = csv.reader(codecs.getreader("utf-8")(f, errors="replace"))
            header = next(reader)  # skip header
            for row in reader:
                yield row


def extract_record(row: list, oiics_to_label: dict) -> dict | None:
    """Extract a training record from an OSHA row. Returns None if no match."""
    if len(row) <= COL_EVENT_CODE:
        return None

    event_code = row[COL_EVENT_CODE].strip()
    if not event_code:
        return None

    label = oiics_to_label.get(event_code)
    if not label:
        return None

    narrative = combine_narrative(row)
    if not narrative or len(narrative) < 10:
        return None

    outcome = row[COL_OUTCOME].strip() if len(row) > COL_OUTCOME else ""
    event_title = row[COL_EVENT_TITLE].strip() if len(row) > COL_EVENT_TITLE else ""
    source_title = row[COL_SOURCE_TITLE].strip() if len(row) > COL_SOURCE_TITLE else ""
    nature_title = row[COL_NATURE_TITLE].strip() if len(row) > COL_NATURE_TITLE else ""
    part_title = row[COL_PART_TITLE].strip() if len(row) > COL_PART_TITLE else ""
    industry = row[COL_INDUSTRY].strip() if len(row) > COL_INDUSTRY else ""
    job = row[COL_JOB].strip() if len(row) > COL_JOB else ""

    return {
        "narrative": narrative,
        "label": label,
        "oiics_event_code": event_code,
        "oiics_event_title": event_title,
        "oiics_source": source_title,
        "oiics_nature": nature_title,
        "oiics_body_part": part_title,
        "outcome": outcome,
        "industry": industry,
        "job": job,
    }


def reservoir_sample(streams: list[Path], oiics_to_label: dict, cap: int, seed: int) -> dict[str, list]:
    """Reservoir sampling: stream through all files, keep up to CAP per class.

    For classes with more than CAP examples, uses reservoir sampling so every
    record has an equal probability of being selected without loading all data
    into memory.
    """
    rng = random.Random(seed)
    buckets: dict[str, list] = defaultdict(list)
    counts: dict[str, int] = Counter()

    for zip_path in streams:
        print(f"  Streaming {zip_path.name}...")
        for row in stream_osha_zip(zip_path):
            record = extract_record(row, oiics_to_label)
            if record is None:
                continue

            label = record["label"]
            counts[label] += 1
            n = counts[label]

            if n <= cap:
                buckets[label].append(record)
            else:
                # Reservoir sampling: replace with probability cap/n
                j = rng.randint(0, n - 1)
                if j < cap:
                    buckets[label][j] = record

    return dict(buckets), dict(counts)


def main():
    parser = argparse.ArgumentParser(description="Extract stratified OSHA training data")
    parser.add_argument("--cap", type=int, default=5000, help="Max examples per class (default: 5000)")
    parser.add_argument("--seed", type=int, default=42, help="Random seed for sampling")
    parser.add_argument("--dry-run", action="store_true", help="Count only, don't write output")
    args = parser.parse_args()

    print("Loading classifier labels...")
    oiics_to_label, label_info = load_labels()
    print(f"  {len(label_info)} labels, {len(oiics_to_label)} OIICS code mappings")

    # Find OSHA zip files
    zips = sorted(OSHA_DIR.glob("ITA_Case_Detail_*.zip"))
    if not zips:
        print(f"ERROR: No OSHA zip files found in {OSHA_DIR}")
        sys.exit(1)
    print(f"\nStreaming {len(zips)} OSHA files (cap={args.cap}, seed={args.seed})...")

    buckets, total_counts = reservoir_sample(zips, oiics_to_label, args.cap, args.seed)

    # Report
    print(f"\nExtraction results:")
    print(f"{'Label':25s}  {'Total':>8s}  {'Sampled':>8s}  {'Gate':20s}")
    print("-" * 70)
    total_sampled = 0
    for label_id in sorted(label_info.keys(), key=lambda k: total_counts.get(k, 0), reverse=True):
        info = label_info[label_id]
        total = total_counts.get(label_id, 0)
        sampled = len(buckets.get(label_id, []))
        total_sampled += sampled
        print(f"{label_id:25s}  {total:>8,}  {sampled:>8,}  {info['gate']}")

    print(f"\nTotal: {sum(total_counts.values()):,} matched → {total_sampled:,} sampled")

    if args.dry_run:
        print("\nDRY RUN — no output written")
        return

    # Write to Parquet
    try:
        import pyarrow as pa
        import pyarrow.parquet as pq
    except ImportError:
        print("\nERROR: pyarrow not installed. Install with: pip install pyarrow")
        print("Falling back to JSONL output...")
        output_path = OUTPUT_DIR / "osha_training.jsonl"
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        with open(output_path, "w") as f:
            for label_id, records in buckets.items():
                for r in records:
                    f.write(json.dumps(r) + "\n")
        print(f"Wrote {total_sampled:,} records to {output_path}")
        return

    # Build Arrow table
    all_records = []
    for label_id, records in buckets.items():
        all_records.extend(records)

    # Shuffle to mix classes
    rng = random.Random(args.seed)
    rng.shuffle(all_records)

    schema = pa.schema([
        ("narrative", pa.utf8()),
        ("label", pa.utf8()),
        ("oiics_event_code", pa.utf8()),
        ("oiics_event_title", pa.utf8()),
        ("oiics_source", pa.utf8()),
        ("oiics_nature", pa.utf8()),
        ("oiics_body_part", pa.utf8()),
        ("outcome", pa.utf8()),
        ("industry", pa.utf8()),
        ("job", pa.utf8()),
    ])

    arrays = {field.name: pa.array([r[field.name] for r in all_records], type=pa.utf8()) for field in schema}
    table = pa.table(arrays, schema=schema)

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    output_path = OUTPUT_DIR / "osha_training.parquet"
    pq.write_table(table, output_path)
    print(f"\nWrote {len(all_records):,} records to {output_path}")
    print(f"  File size: {output_path.stat().st_size / 1024 / 1024:.1f} MB")

    # Also write label distribution summary
    summary = {
        "total_records": len(all_records),
        "cap_per_class": args.cap,
        "seed": args.seed,
        "source_files": [z.name for z in zips],
        "class_distribution": {
            label_id: {"total_in_osha": total_counts.get(label_id, 0), "sampled": len(buckets.get(label_id, []))}
            for label_id in sorted(label_info.keys())
        },
    }
    summary_path = OUTPUT_DIR / "training_summary.json"
    with open(summary_path, "w") as f:
        json.dump(summary, f, indent=2)
    print(f"  Summary: {summary_path}")


if __name__ == "__main__":
    main()
