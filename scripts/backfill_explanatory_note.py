#!/usr/bin/env python3
"""Backfill explanatory_note into DuckDB from sertantai LRT via zenoh.

Queries the sertantai zenoh queryable for LRT records and extracts
the explanatory_note field into the DuckDB legislation table.

Prerequisites:
  - sertantai-legal running as zenoh peer on the LAN
  - pip install eclipse-zenoh  (or use the Rust CLI alternative below)

Usage:
    /usr/bin/python3 scripts/backfill_explanatory_note.py
    /usr/bin/python3 scripts/backfill_explanatory_note.py --law UK_uksi_1997_1713
    /usr/bin/python3 scripts/backfill_explanatory_note.py --qq-only
    /usr/bin/python3 scripts/backfill_explanatory_note.py --from-json data/lrt-dump.json

Alternative (no zenoh Python required):
    1. Export LRT from sertantai:
       curl http://sertantai-host/api/lrt > data/lrt-dump.json
       OR: on sertantai machine, run:
       mix run -e 'IO.puts Jason.encode!(SertantaiLegal.Repo.all(SertantaiLegal.LegalRegister))'
    2. Import the JSON:
       /usr/bin/python3 scripts/backfill_explanatory_note.py --from-json data/lrt-dump.json
"""

import argparse
import json
import sys
from pathlib import Path

import duckdb

DUCKDB_PATH = "data/fractalaw.duckdb"
QQ_LAWS_PATH = "data/sertantai/qq-applicable-laws.csv"


def load_qq_laws():
    """Load QQ applicable law names."""
    text = Path(QQ_LAWS_PATH).read_text().strip()
    return [name.strip() for name in text.split(",") if name.strip()]


def backfill_from_json(duck_conn, json_path, qq_only=False):
    """Backfill from a JSON file containing LRT records."""
    data = json.loads(Path(json_path).read_text())
    if isinstance(data, dict):
        data = [data]

    qq_laws = set(load_qq_laws()) if qq_only else None

    updated = 0
    skipped = 0
    no_note = 0
    for record in data:
        name = record.get("name")
        if not name:
            continue
        if qq_only and name not in qq_laws:
            skipped += 1
            continue

        note = record.get("explanatory_note")
        if not note:
            no_note += 1
            continue

        # Truncate at 10K (sertantai's limit, but belt and braces)
        if len(note) > 10000:
            note = note[:10000]

        duck_conn.execute(
            "UPDATE legislation SET explanatory_note = ? WHERE name = ?",
            [note, name]
        )
        updated += 1

    print(f"Updated: {updated}")
    print(f"No note: {no_note}")
    if qq_only:
        print(f"Skipped (not QQ): {skipped}")
    return updated


def backfill_from_zenoh(duck_conn, tenant="dev", law_name=None, qq_only=False):
    """Backfill by querying sertantai's zenoh LRT queryable."""
    try:
        import zenoh
    except ImportError:
        print("ERROR: zenoh Python package not installed.", file=sys.stderr)
        print("Install with: pip install eclipse-zenoh", file=sys.stderr)
        print("Or use --from-json with an exported LRT dump.", file=sys.stderr)
        sys.exit(1)

    session = zenoh.open(zenoh.Config())

    if law_name:
        key = f"fractalaw/@{tenant}/data/legislation/lrt/{law_name}"
    else:
        key = f"fractalaw/@{tenant}/data/legislation/lrt"

    print(f"Querying: {key}")
    replies = session.get(key, timeout=30.0)

    qq_laws = set(load_qq_laws()) if qq_only else None
    updated = 0
    no_note = 0

    for reply in replies:
        try:
            payload = reply.ok.payload.to_string()
            data = json.loads(payload)
        except Exception as e:
            print(f"  Parse error: {e}", file=sys.stderr)
            continue

        records = data if isinstance(data, list) else [data]
        for record in records:
            name = record.get("name")
            if not name:
                continue
            if qq_only and name not in qq_laws:
                continue

            note = record.get("explanatory_note")
            if not note:
                no_note += 1
                continue

            if len(note) > 10000:
                note = note[:10000]

            duck_conn.execute(
                "UPDATE legislation SET explanatory_note = ? WHERE name = ?",
                [note, name]
            )
            updated += 1

    session.close()
    print(f"Updated: {updated}")
    print(f"No note: {no_note}")
    return updated


def main():
    parser = argparse.ArgumentParser(description="Backfill explanatory_note into DuckDB")
    parser.add_argument("--from-json", help="Import from a JSON file instead of querying zenoh")
    parser.add_argument("--law", help="Backfill a single law")
    parser.add_argument("--qq-only", action="store_true", help="Only backfill QQ applicable laws (~428)")
    parser.add_argument("--tenant", default="dev", help="Zenoh tenant namespace")
    args = parser.parse_args()

    duck_conn = duckdb.connect(DUCKDB_PATH)

    # Ensure column exists
    try:
        duck_conn.execute("ALTER TABLE legislation ADD COLUMN explanatory_note TEXT")
        print("Added explanatory_note column")
    except Exception:
        pass  # Already exists

    if args.from_json:
        count = backfill_from_json(duck_conn, args.from_json, qq_only=args.qq_only)
    else:
        count = backfill_from_zenoh(duck_conn, tenant=args.tenant,
                                     law_name=args.law, qq_only=args.qq_only)

    # Report
    total = duck_conn.execute(
        "SELECT count(*) FROM legislation WHERE explanatory_note IS NOT NULL"
    ).fetchone()[0]
    print(f"\nTotal laws with explanatory_note: {total}")

    duck_conn.close()


if __name__ == "__main__":
    main()
