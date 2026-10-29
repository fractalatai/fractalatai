#!/usr/bin/env python3
"""One-off: re-key data/sif.duckdb events (and the Qwen/band result JSON) from the legacy
RowSignatureII hash to ECO_ETL's row_sig (Id x 100 + report-type code).

Mapping comes from data/sif/legacy_rowsig_to_id.csv (legacy hash inverted by brute force,
see .claude/plans/sif/SIF-IMPORT-SCHEMA.md). After this, event_id = row_sig everywhere and the
old key is kept in legacy_event_id.

Also: event_date -> ISO 'YYYY-MM-DD', adds case_id, ed_report_type (from SIF.csv), date_updated,
source_file, narrative_changed_at, vanished_at. classifications/reviews are empty and recreated.

Idempotent: exits if events already has legacy_event_id.

Usage:
    /usr/bin/python3 scripts/migrations/sif_rekey_rowsig.py [--dry-run]
"""

import argparse
import csv
import json
import sys
from pathlib import Path

import duckdb

DUCKDB_PATH = Path("data/sif.duckdb")
MAPPING = Path("data/sif/legacy_rowsig_to_id.csv")
SIF_CSV = Path("data/qq/sif/SIF.csv")
RESULT_FILES = [
    Path("data/sif/benchmarks/qq_zeroshot_full.json"),
    Path("data/sif/benchmarks/band_selection_full.json"),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    mapping = {r["legacy_row_sig"]: r for r in csv.DictReader(open(MAPPING))}
    with open(SIF_CSV, encoding="cp1252", newline="") as f:
        ed_types = {r["RowSignatureII"].strip(): r["ED_ReportType"].strip() for r in csv.DictReader(f)}

    con = duckdb.connect(str(DUCKDB_PATH), read_only=args.dry_run)
    cols = [r[0] for r in con.execute("DESCRIBE events").fetchall()]
    if "legacy_event_id" in cols:
        print("events already re-keyed — nothing to do")
        return

    for t in ("classifications", "reviews"):
        n = con.execute(f"SELECT count(*) FROM {t}").fetchone()[0]
        if n:
            sys.exit(f"{t} has {n} rows — this migration assumes it is empty")

    legacy_ids = [r[0] for r in con.execute("SELECT event_id FROM events").fetchall()]
    missing = [e for e in legacy_ids if e not in mapping]
    if missing:
        sys.exit(f"{len(missing)} events have no row_sig mapping, e.g. {missing[:5]}")
    print(f"events: {len(legacy_ids):,}, all mapped")

    # Result JSON remap check
    remapped = {}
    for p in RESULT_FILES:
        data = json.load(open(p))
        unmapped = [e["event_id"] for e in data if str(e["event_id"]) not in mapping]
        if unmapped:
            sys.exit(f"{p}: {len(unmapped)} unmapped event_ids, e.g. {unmapped[:5]}")
        for e in data:
            e["legacy_event_id"] = str(e["event_id"])
            e["event_id"] = mapping[str(e["event_id"])]["row_sig"]
        remapped[p] = data
        print(f"{p}: {len(data):,} results remapped")

    if args.dry_run:
        print("DRY RUN — no changes written")
        return

    map_rows = [(k, v["row_sig"], int(v["id"]), ed_types.get(k)) for k, v in mapping.items()]
    con.execute("BEGIN")
    con.execute("CREATE TEMP TABLE m (legacy VARCHAR, row_sig VARCHAR, case_id BIGINT, ed_report_type VARCHAR)")
    con.executemany("INSERT INTO m VALUES (?, ?, ?, ?)", map_rows)
    con.execute("""
        CREATE TABLE events_new AS
        SELECT m.row_sig                 AS event_id,
               m.case_id                 AS case_id,
               e.event_id                AS legacy_event_id,
               e.site, e.narrative, e.action, e.report_type,
               m.ed_report_type          AS ed_report_type,
               e.fy, e.sector, e.sub_sector,
               strftime(strptime(e.event_date, '%d/%m/%Y %H:%M'), '%Y-%m-%d') AS event_date,
               e.hazard_category, e.event_type_class,
               e.qq_sifp, e.qq_ed_sifp, e.qq_t_sifp, e.qq_i_sifp, e.qq_i_sifa,
               e.ingested_at,
               CAST(NULL AS TIMESTAMP)   AS date_updated,
               'SIF.csv'                 AS source_file,
               CAST(NULL AS TIMESTAMP)   AS narrative_changed_at,
               CAST(NULL AS TIMESTAMP)   AS vanished_at
        FROM events e JOIN m ON m.legacy = e.event_id
    """)
    n_new = con.execute("SELECT count(*), count(DISTINCT event_id) FROM events_new").fetchone()
    if n_new != (len(legacy_ids), len(legacy_ids)):
        con.execute("ROLLBACK")
        sys.exit(f"row count mismatch: {n_new} vs {len(legacy_ids)}")

    schema_sql = Path("scripts/sif/sif_schema.sql").read_text()
    con.execute("DROP TABLE reviews")
    con.execute("DROP TABLE classifications")
    con.execute("DROP TABLE events")
    con.execute(schema_sql)
    con.execute("INSERT INTO events BY NAME SELECT * FROM events_new")
    con.execute("DROP TABLE events_new")
    final = con.execute("SELECT count(*) FROM events").fetchone()[0]
    if final != len(legacy_ids):
        con.execute("ROLLBACK")
        sys.exit(f"final count {final} != {len(legacy_ids)}")
    con.execute("COMMIT")
    con.close()
    print(f"events re-keyed: {final:,}")

    for p, data in remapped.items():
        tmp = p.with_suffix(".json.tmp")
        json.dump(data, open(tmp, "w"), indent=2)
        tmp.replace(p)
        print(f"wrote {p}")


if __name__ == "__main__":
    main()
