#!/usr/bin/env python3
"""Ingest QQ SIF data into data/sif.duckdb.

Joins SIF.csv (SIFp labels, hazard categories) with Redactor CSVs (narratives)
via RowSignatureII, and stores in the events table.

SIF.csv columns:
  RowSignatureII  — join key to Redactor data
  RE_When         — date of event (reporter)
  ED_SIFP         — SIFp assessment (triager adjusted)
  T_SIFP          — SIFp assessment (triager assigned)
  T_SIFP2         — SIFp secondary (triager)
  I_SIFA          — SIF actual (investigator)
  I_SIFP          — SIF potential (investigator)
  SIFP            — consensus label: I > T > ED cascade. USE THIS.
  RE_ReportType   — report type (reporter)
  ED_ReportType   — report type (triager adjusted)
  IsAtWork        — at work flag
  ED_EventTypeClass — event type classification
  ED_Hazards      — hazard category

Redactor CSV columns (headerless):
  Id, Site, What, Type, AtWork, Action, FY, AP, Sector, SubSector

Usage:
    /usr/bin/python3 scripts/sif/ingest_qq.py [--dry-run]
"""

import argparse
import csv
import sys
from pathlib import Path

import duckdb

SIF_CSV = Path("data/qq/sif/SIF.csv")
REDACTOR_DIR = Path("data/qq/cultural-graph/qq-data")
DUCKDB_PATH = Path("data/sif.duckdb")

REDACTOR_COLS = ["Id", "Site", "What", "Type", "AtWork", "Action", "FY", "AP", "Sector", "SubSector"]


def load_sif_labels() -> dict:
    """Load SIF.csv into a dict keyed by RowSignatureII."""
    records = {}
    with open(SIF_CSV, encoding="cp1252") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rid = row["RowSignatureII"].strip()
            records[rid] = {
                "event_date": row.get("RE_When", "").strip(),
                "qq_sifp": row.get("SIFP", "").strip().rstrip("\r"),
                "ed_sifp": row.get("ED_SIFP", "").strip(),
                "t_sifp": row.get("T_SIFP", "").strip(),
                "i_sifp": row.get("I_SIFP", "").strip(),
                "i_sifa": row.get("I_SIFA", "").strip(),
                "report_type": row.get("RE_ReportType", "").strip(),
                "ed_report_type": row.get("ED_ReportType", "").strip(),
                "event_type_class": row.get("ED_EventTypeClass", "").strip(),
                "hazard_category": row.get("ED_Hazards", "").strip().rstrip("\r"),
            }
    return records


def load_redactor_narratives(sif_ids: set) -> dict:
    """Load narratives from Redactor CSVs for matching IDs."""
    narratives = {}
    for path in sorted(REDACTOR_DIR.glob("Redactor_*.csv")):
        with open(path, encoding="cp1252") as f:
            reader = csv.reader(f)
            for row in reader:
                if len(row) < 7:
                    continue
                rid = row[0].strip()
                if rid in sif_ids:
                    narratives[rid] = {
                        "site": row[1].strip() if len(row) > 1 else "",
                        "narrative": row[2].strip() if len(row) > 2 else "",
                        "action": row[5].strip() if len(row) > 5 else "",
                        "fy": row[6].strip() if len(row) > 6 else "",
                        "sector": row[8].strip() if len(row) > 8 else "",
                        "sub_sector": row[9].strip().rstrip("\r") if len(row) > 9 else "",
                    }
    return narratives


def create_schema(con):
    """Create the SIF events table if it doesn't exist."""
    con.execute("""
        CREATE TABLE IF NOT EXISTS events (
            event_id          VARCHAR PRIMARY KEY,
            site              VARCHAR,
            narrative         VARCHAR,
            action            VARCHAR,
            report_type       VARCHAR,
            fy                VARCHAR,
            sector            VARCHAR,
            sub_sector        VARCHAR,
            event_date        VARCHAR,
            hazard_category   VARCHAR,
            event_type_class  VARCHAR,
            -- QQ SIFp labels (benchmark, not training)
            qq_sifp           VARCHAR,
            qq_ed_sifp        VARCHAR,
            qq_t_sifp         VARCHAR,
            qq_i_sifp         VARCHAR,
            qq_i_sifa         VARCHAR,
            ingested_at       TIMESTAMP DEFAULT current_timestamp
        )
    """)
    con.execute("""
        CREATE TABLE IF NOT EXISTS classifications (
            event_id          VARCHAR PRIMARY KEY REFERENCES events(event_id),
            mechanism_codes   VARCHAR[],
            mechanism_labels  VARCHAR[],
            object_codes      VARCHAR[],
            object_labels     VARCHAR[],
            sif_gate          VARCHAR,
            stage1_confidence FLOAT,
            energy_types      VARCHAR[],
            source_cues       VARCHAR[],
            carrier_cues      VARCHAR[],
            environment_cues  VARCHAR[],
            body_vulnerability VARCHAR,
            severity_p10      VARCHAR,
            severity_p50      VARCHAR,
            severity_p90      VARCHAR,
            energy_reasoning  VARCHAR,
            stage2_confidence FLOAT,
            p_sif             FLOAT,
            sif_class         VARCHAR,
            review_flag       BOOLEAN,
            classified_at     TIMESTAMP DEFAULT current_timestamp,
            model_version     VARCHAR
        )
    """)
    con.execute("""
        CREATE TABLE IF NOT EXISTS reviews (
            event_id          VARCHAR REFERENCES events(event_id),
            reviewer          VARCHAR,
            decision          VARCHAR,
            notes             VARCHAR,
            reviewed_at       TIMESTAMP DEFAULT current_timestamp,
            PRIMARY KEY (event_id, reviewer)
        )
    """)


def main():
    parser = argparse.ArgumentParser(description="Ingest QQ SIF data into DuckDB")
    parser.add_argument("--dry-run", action="store_true", help="Don't write to DuckDB")
    args = parser.parse_args()

    print("Loading SIF labels...")
    sif_records = load_sif_labels()
    print(f"  {len(sif_records):,} SIF records")

    print("Loading Redactor narratives...")
    narratives = load_redactor_narratives(set(sif_records.keys()))
    print(f"  {len(narratives):,} narratives matched")

    # Join
    rows = []
    unmatched = 0
    for rid, sif in sif_records.items():
        narr = narratives.get(rid)
        if not narr:
            unmatched += 1
            continue
        rows.append((
            rid,
            narr["site"],
            narr["narrative"],
            narr["action"],
            sif["report_type"],
            narr["fy"],
            narr["sector"],
            narr["sub_sector"],
            sif["event_date"],
            sif["hazard_category"],
            sif["event_type_class"],
            sif["qq_sifp"],
            sif["ed_sifp"],
            sif["t_sifp"],
            sif["i_sifp"],
            sif["i_sifa"],
        ))

    print(f"  {len(rows):,} joined rows, {unmatched} unmatched SIF records")

    if args.dry_run:
        print("\nDRY RUN — would insert:")
        print(f"  Events: {len(rows)}")
        # Show SIFP distribution
        from collections import Counter
        dist = Counter(r[11] for r in rows)
        print(f"  SIFP distribution:")
        for k, v in dist.most_common():
            print(f"    {k:30s} {v:>5,}")
        return

    con = duckdb.connect(str(DUCKDB_PATH))
    create_schema(con)

    # Check for duplicates
    existing = set(r[0] for r in con.execute("SELECT event_id FROM events").fetchall())
    new_rows = [r for r in rows if r[0] not in existing]
    dupes = len(rows) - len(new_rows)

    if dupes:
        print(f"  {dupes} duplicates skipped")

    if not new_rows:
        print("  No new rows to insert")
        con.close()
        return

    con.executemany("""
        INSERT INTO events (
            event_id, site, narrative, action, report_type, fy,
            sector, sub_sector, event_date, hazard_category, event_type_class,
            qq_sifp, qq_ed_sifp, qq_t_sifp, qq_i_sifp, qq_i_sifa
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, new_rows)

    count = con.execute("SELECT count(*) FROM events").fetchone()[0]
    print(f"\nDuckDB updated: {count:,} events in data/sif.duckdb")

    # Summary
    dist = con.execute("""
        SELECT qq_sifp, count(*) as n
        FROM events
        GROUP BY qq_sifp
        ORDER BY n DESC
    """).fetchall()
    print(f"SIFP distribution:")
    for label, n in dist:
        print(f"  {label:30s} {n:>5,}")

    con.close()


if __name__ == "__main__":
    main()
