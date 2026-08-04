#!/usr/bin/env python3
"""Cultural graph data ingestion and QA.

Reads raw CSV from QQ, validates, profiles, cleans, and outputs
JSONL ready for RunPod inference.

Supports two CSV formats:
  - Headerless (Redactor_YYYY.csv): Id, Site, What, Type, AtWork, Action, FY, AP, Sector, SubSector
  - With headers (original samples): R_SiteCode, RE_What, RE_ReportType, IsAtWork, RE_ActionImmediate, Random

Usage:
    /usr/bin/python3 scripts/cultural-graph/ingest_qa.py --input data/qq/cultural-graph/qq-data/Redactor_2027.csv
    /usr/bin/python3 scripts/cultural-graph/ingest_qa.py --input data/qq/cultural-graph/qq-data/Redactor_2027.csv --check-dupes
"""

import argparse
import csv
import json
import statistics
import sys
from collections import Counter
from pathlib import Path

import duckdb

# Headerless Redactor format
REDACTOR_COLS = ["Id", "Site", "What", "Type", "AtWork", "Action", "FY", "AP", "Sector", "SubSector"]

# Header-format column mapping
HEADER_MAP = {
    "R_SiteCode": "Site",
    "RE_What": "What",
    "RE_ReportType": "Type",
    "IsAtWork": "AtWork",
    "RE_ActionImmediate": "Action",
    "ED_ActionImmediate": "Action",
}

DUCKDB_PATH = Path("data/cultural-graph.duckdb")


def detect_format(path):
    """Detect CSV format and encoding by trying to read 100 lines."""
    for encoding in ["utf-8-sig", "cp1252"]:
        try:
            with open(path, encoding=encoding) as f:
                # Read 100 lines to confirm encoding works
                for _ in range(100):
                    f.readline()
                f.seek(0)
                first_line = f.readline()
                first_field = first_line.split(",")[0].strip().strip('"')
                if first_field in ("R_SiteCode", "Id", "ID"):
                    return "headers", encoding
                if first_field.isdigit():
                    return "headerless", encoding
                return "headerless", encoding
        except UnicodeDecodeError:
            continue
    print("ERROR: Could not detect encoding", file=sys.stderr)
    sys.exit(1)


def load_csv(path, fmt, encoding):
    """Load CSV into list of normalised dicts."""
    records = []
    with open(path, encoding=encoding) as f:
        if fmt == "headers":
            reader = csv.DictReader(f)
            for row in reader:
                rec = {}
                for src, dst in HEADER_MAP.items():
                    if src in row:
                        rec[dst] = row[src]
                # Generate ID if not present
                rec.setdefault("Id", str(hash(row.get("RE_What", "")[:50])))
                rec.setdefault("FY", "")
                rec.setdefault("AP", "")
                rec.setdefault("Sector", "")
                rec.setdefault("SubSector", "")
                records.append(rec)
        else:
            reader = csv.reader(f)
            for row in reader:
                if len(row) < 6:
                    continue
                rec = dict(zip(REDACTOR_COLS, row + [""] * (len(REDACTOR_COLS) - len(row))))
                records.append(rec)

    # Combine What + Action into narrative_text
    for rec in records:
        text = rec.get("What", "")
        action = rec.get("Action", "").strip()
        if action:
            text = text.rstrip() + "\n\nImmediate action taken: " + action
        rec["narrative_text"] = text
        rec["word_count"] = len(text.split())

    return records


def profile(records, path):
    """Print QA profile of the data."""
    print(f"\n{'='*60}")
    print(f"DATA PROFILE: {path.name}")
    print(f"{'='*60}")
    print(f"Records: {len(records)}")

    # Report types
    types = Counter(r["Type"] for r in records)
    print(f"\nReport types:")
    for t, c in types.most_common():
        print(f"  {t}: {c}")

    # Sites
    sites = Counter(r["Site"] for r in records)
    print(f"\nSites: {len(sites)} unique")
    print(f"Top 5:")
    for s, c in sites.most_common(5):
        print(f"  {s}: {c}")

    # FY
    fys = Counter(r.get("FY", "") for r in records)
    if len(fys) > 1 or list(fys.keys()) != [""]:
        print(f"\nFinancial years:")
        for y, c in sorted(fys.items()):
            print(f"  {y}: {c}")

    # Sectors
    sectors = Counter(r.get("Sector", "") for r in records if r.get("Sector"))
    if sectors:
        print(f"\nSectors:")
        for s, c in sectors.most_common():
            print(f"  {s}: {c}")

    # Word counts
    words = [r["word_count"] for r in records]
    print(f"\nNarrative length (words):")
    print(f"  min={min(words)}, median={statistics.median(words):.0f}, "
          f"mean={statistics.mean(words):.0f}, max={max(words)}")

    # Short narratives
    short = [r for r in records if r["word_count"] < 10]
    print(f"\nVery short (<10 words): {len(short)} ({100*len(short)/len(records):.1f}%)")
    if short:
        for r in short[:3]:
            print(f"  [{r['word_count']}w] {r['narrative_text'][:60]}")

    # Empty Action field
    has_action = sum(1 for r in records if r.get("Action", "").strip())
    print(f"\nWith Action field: {has_action}/{len(records)} ({100*has_action/len(records):.0f}%)")

    # Encoding issues (check for replacement characters)
    encoding_issues = sum(1 for r in records if "\ufffd" in r.get("narrative_text", ""))
    if encoding_issues:
        print(f"\nWARNING: {encoding_issues} records with encoding errors")

    return types, sites


def check_dupes(records):
    """Check for duplicate IDs against existing DuckDB."""
    if not DUCKDB_PATH.exists():
        print("\nDuplicate check: DuckDB not found, skipping")
        return set()

    new_ids = {r["Id"] for r in records}
    con = duckdb.connect(str(DUCKDB_PATH), read_only=True)
    existing = set(con.execute("SELECT id FROM narratives").fetchdf()["id"].tolist())
    con.close()

    dupes = new_ids & existing
    if dupes:
        print(f"\nWARNING: {len(dupes)} duplicate IDs already in DuckDB")
        for d in list(dupes)[:5]:
            print(f"  {d}")
    else:
        print(f"\nDuplicate check: 0 duplicates (all {len(new_ids)} IDs are new)")

    return dupes


def write_jsonl(records, output_path, exclude_ids=None):
    """Write cleaned records as JSONL for RunPod."""
    exclude_ids = exclude_ids or set()
    written = 0
    with open(output_path, "w") as f:
        for rec in records:
            if rec["Id"] in exclude_ids:
                continue
            out = {
                "id": rec["Id"],
                "site": rec["Site"],
                "narrative_text": rec["narrative_text"],
                "report_type": rec["Type"],
                "fy": rec.get("FY", ""),
                "sector": rec.get("Sector", ""),
                "sub_sector": rec.get("SubSector", ""),
                "word_count": rec["word_count"],
            }
            f.write(json.dumps(out) + "\n")
            written += 1
    print(f"\nOutput: {output_path} ({written} records)")
    return written


def main():
    parser = argparse.ArgumentParser(description="Cultural graph data ingestion and QA")
    parser.add_argument("--input", required=True, help="Input CSV path")
    parser.add_argument("--output", help="Output JSONL path (default: auto-named)")
    parser.add_argument("--check-dupes", action="store_true", help="Check for duplicate IDs in DuckDB")
    parser.add_argument("--profile-only", action="store_true", help="Profile only, no output")
    args = parser.parse_args()

    input_path = Path(args.input)
    if not input_path.exists():
        print(f"ERROR: {input_path} not found", file=sys.stderr)
        sys.exit(1)

    # Detect format and load
    fmt, encoding = detect_format(input_path)
    print(f"Format: {fmt}, Encoding: {encoding}")
    records = load_csv(input_path, fmt, encoding)

    # Profile
    profile(records, input_path)

    # Duplicate check
    dupes = set()
    if args.check_dupes:
        dupes = check_dupes(records)

    if args.profile_only:
        return

    # Write output
    if args.output:
        output_path = Path(args.output)
    else:
        stem = input_path.stem
        output_path = Path("data/qq/cultural-graph/ingest") / f"{stem}-clean.jsonl"

    output_path.parent.mkdir(parents=True, exist_ok=True)
    write_jsonl(records, output_path, exclude_ids=dupes)


if __name__ == "__main__":
    main()
