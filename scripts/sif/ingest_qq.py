#!/usr/bin/env python3
"""Ingest a QQ SIF import file (ECO_ETL export) into data/sif.duckdb.

File contract: .claude/plans/sif/SIF-IMPORT-SCHEMA.md — one UTF-8 CSV, 19 columns, one row per
report, keyed on row_sig (Id x 100 + report-type code). Files land in data/qq/sif/incoming/.

Behaviour (delta or cumulative files both work):
  - new row_sig                      → insert
  - known row_sig, date_updated newer → update all fields; if narrative/action changed, set
                                        narrative_changed_at (model results are stale)
  - known row_sig, not newer          → leave alone
  - stored row dated inside the file's event_date range but absent from the file
                                      → set vanished_at (ECO_ETL merged it, ETL-020); never deleted

Usage:
    /usr/bin/python3 scripts/sif/ingest_qq.py data/qq/sif/incoming/SIF_20261029.csv [--dry-run]
"""

import argparse
import csv
import re
import sys
from collections import Counter
from datetime import datetime
from pathlib import Path

import duckdb

DUCKDB_PATH = Path("data/sif.duckdb")
SCHEMA_SQL = Path("scripts/sif/sif_schema.sql")

FILE_COLUMNS = [
    "row_sig", "id", "event_date", "date_updated", "fy", "site", "sector", "sub_sector",
    "report_type", "ed_report_type", "event_type_class", "hazard_category",
    "sifp", "ed_sifp", "t_sifp", "i_sifp", "i_sifa", "narrative", "action",
]
REQUIRED = ["row_sig", "id", "event_date", "date_updated", "fy", "site", "sector",
            "report_type", "ed_report_type", "sifp", "narrative"]
# file column → events column (where the name differs)
RENAME = {"row_sig": "event_id", "id": "case_id", "sifp": "qq_sifp", "ed_sifp": "qq_ed_sifp",
          "t_sifp": "qq_t_sifp", "i_sifp": "qq_i_sifp", "i_sifa": "qq_i_sifa"}
DB_COLUMNS = [RENAME.get(c, c) for c in FILE_COLUMNS]


def norm(s):
    return " ".join((s or "").split())


def parse_ts(s):
    return datetime.fromisoformat(s.strip()) if s and s.strip() else None


def load_file(path):
    """Read and validate the import file. Exits on contract errors; returns (rows, warnings)."""
    with open(path, encoding="utf-8-sig", newline="") as f:
        reader = csv.DictReader(f)
        if reader.fieldnames != FILE_COLUMNS:
            sys.exit(f"header mismatch:\n  got      {reader.fieldnames}\n  expected {FILE_COLUMNS}")
        rows = list(reader)

    errors, warnings = [], []
    if any(None in r or None in r.values() for r in rows):
        errors.append("ragged rows (wrong field count)")
    dupes = [k for k, n in Counter(r["row_sig"] for r in rows).items() if n > 1]
    if dupes:
        errors.append(f"{len(dupes)} duplicate row_sig, e.g. {dupes[:5]}")
    for r in rows:
        rs, cid = r["row_sig"], r["id"]
        if not (rs.isdigit() and cid.isdigit() and int(rs) // 100 == int(cid) and 1 <= int(rs) % 100 <= 9):
            errors.append(f"row_sig {rs} is not id {cid} x 100 + type code")
        if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", r["event_date"]):
            errors.append(f"row_sig {rs}: event_date {r['event_date']!r} not YYYY-MM-DD")
        try:
            parse_ts(r["date_updated"])
        except ValueError:
            errors.append(f"row_sig {rs}: date_updated {r['date_updated']!r} not ISO")
        empty = [k for k in REQUIRED if not r[k].strip()]
        if empty:
            warnings.append(f"row_sig {rs}: empty required {empty}")
    if errors:
        sys.exit("import file rejected:\n  " + "\n  ".join(errors[:20]))
    return rows, warnings


def main():
    parser = argparse.ArgumentParser(description="Ingest a QQ SIF import file into DuckDB")
    parser.add_argument("file", type=Path)
    parser.add_argument("--dry-run", action="store_true", help="Report what would change; write nothing")
    args = parser.parse_args()

    rows, warnings = load_file(args.file)
    print(f"{args.file}: {len(rows):,} rows, valid")
    for w in warnings:
        print(f"  WARN {w}")

    con = duckdb.connect(str(DUCKDB_PATH), read_only=args.dry_run)
    if not args.dry_run:
        con.execute(SCHEMA_SQL.read_text())
    stored = {
        r[0]: {"date_updated": r[1], "narrative": r[2], "action": r[3], "vanished_at": r[4]}
        for r in con.execute(
            "SELECT event_id, date_updated, narrative, action, vanished_at FROM events").fetchall()
    }

    inserts, updates, text_changed, unchanged = [], [], [], 0
    for r in rows:
        values = [r[c] for c in FILE_COLUMNS]
        values[FILE_COLUMNS.index("id")] = int(r["id"])
        values[FILE_COLUMNS.index("date_updated")] = parse_ts(r["date_updated"])
        old = stored.get(r["row_sig"])
        if old is None:
            inserts.append(values)
        elif old["date_updated"] is None or parse_ts(r["date_updated"]) > old["date_updated"]:
            changed = (norm(old["narrative"]) != norm(r["narrative"])
                       or norm(old["action"]) != norm(r["action"]))
            updates.append((values, changed))
            if changed:
                text_changed.append(r["row_sig"])
        else:
            unchanged += 1

    lo, hi = min(r["event_date"] for r in rows), max(r["event_date"] for r in rows)
    in_file = {r["row_sig"] for r in rows}
    vanished = [eid for (eid,) in con.execute(
        "SELECT event_id FROM events WHERE event_date BETWEEN ? AND ? AND vanished_at IS NULL",
        [lo, hi]).fetchall() if eid not in in_file]
    reappeared = [k for k in in_file if stored.get(k, {}).get("vanished_at")]

    print(f"  event_date range {lo} → {hi}")
    print(f"  insert {len(inserts):,} | update {len(updates):,} (text changed {len(text_changed)}) "
          f"| unchanged {unchanged:,} | vanished {len(vanished)} | reappeared {len(reappeared)}")
    by_type = Counter(v[FILE_COLUMNS.index("ed_report_type")] for v in inserts)
    print("  new by ed_report_type: " + ", ".join(f"{k} {n}" for k, n in by_type.most_common()))
    if vanished:
        print(f"  vanished (merged in ECO_ETL?): {vanished[:20]}{' …' if len(vanished) > 20 else ''}")

    if args.dry_run:
        print("DRY RUN — no changes written")
        return

    src = args.file.name
    placeholders = ", ".join("?" for _ in DB_COLUMNS)
    con.execute("BEGIN")
    con.executemany(
        f"INSERT INTO events ({', '.join(DB_COLUMNS)}, source_file) VALUES ({placeholders}, ?)",
        [v + [src] for v in inserts])
    set_clause = ", ".join(f"{c} = ?" for c in DB_COLUMNS[1:])
    for values, changed in updates:
        con.execute(
            f"UPDATE events SET {set_clause}, source_file = ?, vanished_at = NULL"
            + (", narrative_changed_at = current_timestamp" if changed else "")
            + " WHERE event_id = ?",
            values[1:] + [src, values[0]])
    if reappeared:
        con.executemany("UPDATE events SET vanished_at = NULL WHERE event_id = ?", [[k] for k in reappeared])
    if vanished:
        con.executemany("UPDATE events SET vanished_at = current_timestamp WHERE event_id = ?",
                        [[k] for k in vanished])
    con.execute("COMMIT")

    total = con.execute("SELECT count(*) FROM events").fetchone()[0]
    print(f"\nDuckDB updated: {total:,} events in {DUCKDB_PATH}")
    con.close()


if __name__ == "__main__":
    main()
