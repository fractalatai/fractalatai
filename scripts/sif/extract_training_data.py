#!/usr/bin/env python3
"""Extract stratified training data from OSHA + MSHA sources.

v2: Differentiated caps (1K for NEEDS_ASSESSMENT, 500 for AUTO_NON_SIF),
MSHA supplement for sparse classes, pre-processing, instruction-tuning format.

Training data ONLY — QQ is correlation test, NEVER mixed in here.

Usage:
    /usr/bin/python3 scripts/sif/extract_training_data.py [--dry-run]
"""

import argparse
import csv
import codecs
import json
import random
import re
import sys
import zipfile
from collections import Counter, defaultdict
from pathlib import Path

OSHA_DIR = Path("data/sif/sources/osha")
MSHA_DIR = Path("data/sif/sources/msha")
LABELS_FILE = Path("data/sif/taxonomy/classifier-labels.json")
OUTPUT_DIR = Path("data/sif/training")

SEED = 42

# Per-gate caps
CAP_NEEDS_ASSESSMENT = 1000
CAP_AUTO_NON_SIF = 500

# Max MSHA contribution per class (avoid mining-domain dominance)
MSHA_MAX_PER_CLASS = 1000

# Instruction prefix for every training example
INSTRUCTION = "Classify the following workplace incident narrative. The mechanism of injury is:"

# OSHA CSV column indices
COL_NARRATIVE_WHAT = 33
COL_INCIDENT_DESC = 38
COL_NARRATIVE_INJURY = 36
COL_NARRATIVE_OBJECT = 37
COL_EVENT_CODE = 43
COL_EVENT_TITLE = 44
COL_OUTCOME = 23


def load_labels():
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


def preprocess(text: str) -> str:
    """Normalise a narrative for training.

    - Lower-case (MSHA is ALL CAPS)
    - Strip [REDACTED] markers from OSHA
    - Collapse whitespace
    - Strip leading/trailing whitespace
    """
    text = text.lower()
    text = re.sub(r"\[redacted\]", "", text)
    text = re.sub(r"\s+", " ", text)
    return text.strip()


def combine_osha_narrative(row: list) -> str:
    """Combine OSHA narrative fields into a single text."""
    what = row[COL_NARRATIVE_WHAT].strip() if len(row) > COL_NARRATIVE_WHAT else ""
    desc = row[COL_INCIDENT_DESC].strip() if len(row) > COL_INCIDENT_DESC else ""
    primary = what if len(what) >= len(desc) else desc
    parts = [primary] if primary else []

    injury = row[COL_NARRATIVE_INJURY].strip() if len(row) > COL_NARRATIVE_INJURY else ""
    if injury and injury not in primary:
        parts.append(injury)

    obj = row[COL_NARRATIVE_OBJECT].strip() if len(row) > COL_NARRATIVE_OBJECT else ""
    if obj and obj not in primary:
        parts.append(obj)

    return " ".join(parts)


def get_cap(label_id: str, label_info: dict) -> int:
    """Return the per-class cap based on SIF gate."""
    gate = label_info[label_id]["gate"]
    return CAP_NEEDS_ASSESSMENT if gate == "NEEDS_ASSESSMENT" else CAP_AUTO_NON_SIF


def extract_osha(oiics_to_label: dict, label_info: dict) -> tuple[dict, dict]:
    """Reservoir-sample from OSHA zip files with per-gate caps."""
    rng = random.Random(SEED)
    buckets: dict[str, list] = defaultdict(list)
    counts: dict[str, int] = Counter()

    zips = sorted(OSHA_DIR.glob("ITA_Case_Detail_*.zip"))
    for zip_path in zips:
        print(f"  OSHA: {zip_path.name}...")
        with zipfile.ZipFile(zip_path) as zf:
            csv_name = [n for n in zf.namelist() if n.endswith(".csv")][0]
            with zf.open(csv_name) as f:
                reader = csv.reader(codecs.getreader("utf-8")(f, errors="replace"))
                next(reader)
                for row in reader:
                    if len(row) <= COL_EVENT_CODE:
                        continue
                    event_code = row[COL_EVENT_CODE].strip()
                    label = oiics_to_label.get(event_code)
                    if not label:
                        continue
                    narrative = combine_osha_narrative(row)
                    if not narrative or len(narrative) < 10:
                        continue

                    cap = get_cap(label, label_info)
                    counts[label] += 1
                    n = counts[label]

                    record = {
                        "narrative": preprocess(narrative),
                        "label": label,
                        "source": "osha",
                    }

                    if n <= cap:
                        buckets[label].append(record)
                    else:
                        j = rng.randint(0, n - 1)
                        if j < cap:
                            buckets[label][j] = record

    return dict(buckets), dict(counts)


# MSHA label → JSON file mapping
MSHA_LABEL_MAP = {
    "breathing": "msha_breathing.json",
    "pressure": "msha_pressure.json",
    "explosion": "msha_explosion.json",
    "fire": "msha_fire.json",
    "electrical": "msha_electrical.json",
}


def load_msha_supplements(
    osha_buckets: dict, label_info: dict
) -> dict[str, list]:
    """Load MSHA supplement data for sparse classes.

    Fills each class up to its cap, using at most MSHA_MAX_PER_CLASS
    MSHA events to avoid mining-domain dominance.
    """
    rng = random.Random(SEED + 1)
    supplements: dict[str, list] = {}

    for label_id, filename in MSHA_LABEL_MAP.items():
        path = MSHA_DIR / filename
        if not path.exists():
            continue

        cap = get_cap(label_id, label_info)
        osha_count = len(osha_buckets.get(label_id, []))
        room = cap - osha_count
        if room <= 0:
            print(f"  MSHA {label_id}: OSHA already at cap ({osha_count}), skipping")
            continue

        with open(path) as f:
            msha_records = json.load(f)

        # Shuffle and take up to min(room, MSHA_MAX_PER_CLASS)
        rng.shuffle(msha_records)
        take = min(room, MSHA_MAX_PER_CLASS, len(msha_records))

        records = []
        for rec in msha_records[:take]:
            narrative = preprocess(rec["narrative"])
            if narrative and len(narrative) >= 10:
                records.append({
                    "narrative": narrative,
                    "label": label_id,
                    "source": "msha",
                })

        supplements[label_id] = records
        print(
            f"  MSHA {label_id}: +{len(records)} (OSHA {osha_count} + MSHA {len(records)} = {osha_count + len(records)}, "
            f"cap {cap}, MSHA pool {len(msha_records)})"
        )

    return supplements


def build_training_set(
    osha_buckets: dict, msha_supplements: dict, label_info: dict, rng: random.Random
) -> list[dict]:
    """Merge OSHA + MSHA, add instruction prefix, shuffle."""
    all_records = []

    for label_id in sorted(label_info.keys()):
        osha = osha_buckets.get(label_id, [])
        msha = msha_supplements.get(label_id, [])
        combined = osha + msha
        all_records.extend(combined)

    # Add instruction prefix to every narrative
    for rec in all_records:
        rec["text"] = f"{INSTRUCTION}\n{rec['narrative']}"

    rng.shuffle(all_records)
    return all_records


def write_parquet(records: list[dict], path: Path):
    """Write training records to Parquet."""
    import pyarrow as pa
    import pyarrow.parquet as pq

    schema = pa.schema([
        ("text", pa.utf8()),         # instruction + narrative (model input)
        ("label", pa.utf8()),        # mechanism label (model target)
        ("narrative", pa.utf8()),    # raw narrative (for debugging)
        ("source", pa.utf8()),       # osha or msha
    ])

    arrays = {
        field.name: pa.array([r[field.name] for r in records], type=pa.utf8())
        for field in schema
    }
    table = pa.table(arrays, schema=schema)

    path.parent.mkdir(parents=True, exist_ok=True)
    pq.write_table(table, path)
    return table.num_rows, path.stat().st_size


def write_splits(records: list[dict], rng: random.Random, val_fraction: float = 0.10):
    """Write train/val Parquet files with stratified split."""
    # Group by label
    by_label = defaultdict(list)
    for rec in records:
        by_label[rec["label"]].append(rec)

    train_records = []
    val_records = []

    for label_id, recs in by_label.items():
        rng.shuffle(recs)
        n_val = max(1, int(len(recs) * val_fraction))
        val_records.extend(recs[:n_val])
        train_records.extend(recs[n_val:])

    rng.shuffle(train_records)
    rng.shuffle(val_records)

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    train_path = OUTPUT_DIR / "train.parquet"
    val_path = OUTPUT_DIR / "val.parquet"

    train_n, train_size = write_parquet(train_records, train_path)
    val_n, val_size = write_parquet(val_records, val_path)

    return train_n, val_n, train_path, val_path


def main():
    parser = argparse.ArgumentParser(description="Extract SIF training data v2")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    print("Loading classifier labels...")
    oiics_to_label, label_info = load_labels()
    print(f"  {len(label_info)} labels")
    print(f"  NEEDS_ASSESSMENT cap: {CAP_NEEDS_ASSESSMENT}")
    print(f"  AUTO_NON_SIF cap: {CAP_AUTO_NON_SIF}")

    # Step 1: Extract from OSHA
    print(f"\nStep 1: OSHA extraction (seed={SEED})...")
    osha_buckets, osha_counts = extract_osha(oiics_to_label, label_info)

    # Step 2: MSHA supplements for sparse classes
    print(f"\nStep 2: MSHA supplements (max {MSHA_MAX_PER_CLASS}/class)...")
    msha_supplements = load_msha_supplements(osha_buckets, label_info)

    # Step 3: Report
    print(f"\nClass distribution:")
    print(f"{'Label':25s}  {'OSHA':>6s}  {'MSHA':>6s}  {'Total':>6s}  {'Cap':>5s}  {'Gate'}")
    print("-" * 80)
    total = 0
    for label_id in sorted(label_info.keys(), key=lambda k: len(osha_buckets.get(k, [])), reverse=True):
        osha_n = len(osha_buckets.get(label_id, []))
        msha_n = len(msha_supplements.get(label_id, []))
        cap = get_cap(label_id, label_info)
        gate = label_info[label_id]["gate"]
        t = osha_n + msha_n
        total += t
        msha_str = str(msha_n) if msha_n > 0 else "-"
        print(f"{label_id:25s}  {osha_n:>6,}  {msha_str:>6s}  {t:>6,}  {cap:>5,}  {gate}")

    print(f"\nTotal: {total:,} training events")

    if args.dry_run:
        print("\nDRY RUN — no output written")
        return

    # Step 4: Merge, add instruction prefix, stratified split
    rng = random.Random(SEED)
    print(f"\nStep 4: Building training set...")
    all_records = build_training_set(osha_buckets, msha_supplements, label_info, rng)

    print(f"Step 5: Stratified train/val split (90/10)...")
    train_n, val_n, train_path, val_path = write_splits(all_records, random.Random(SEED))

    print(f"\nOutput:")
    print(f"  {train_path}: {train_n:,} events ({train_path.stat().st_size / 1024 / 1024:.1f} MB)")
    print(f"  {val_path}: {val_n:,} events ({val_path.stat().st_size / 1024 / 1024:.1f} MB)")

    # Step 6: Save summary
    summary = {
        "version": "v2",
        "seed": SEED,
        "caps": {"needs_assessment": CAP_NEEDS_ASSESSMENT, "auto_non_sif": CAP_AUTO_NON_SIF},
        "msha_max_per_class": MSHA_MAX_PER_CLASS,
        "instruction_prefix": INSTRUCTION,
        "preprocessing": ["lowercase", "strip_redacted", "normalise_whitespace"],
        "train_events": train_n,
        "val_events": val_n,
        "class_distribution": {
            label_id: {
                "osha": len(osha_buckets.get(label_id, [])),
                "msha": len(msha_supplements.get(label_id, [])),
                "total": len(osha_buckets.get(label_id, [])) + len(msha_supplements.get(label_id, [])),
                "gate": label_info[label_id]["gate"],
            }
            for label_id in sorted(label_info.keys())
        },
    }
    summary_path = OUTPUT_DIR / "training_summary_v2.json"
    with open(summary_path, "w") as f:
        json.dump(summary, f, indent=2)
    print(f"  {summary_path}")

    # Sample check
    print(f"\nSample training records:")
    for rec in all_records[:3]:
        print(f"  [{rec['label']}] ({rec['source']}) {rec['text'][:120]}...")


if __name__ == "__main__":
    main()
