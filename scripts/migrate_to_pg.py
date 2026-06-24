#!/usr/bin/env /usr/bin/python3
"""Migrate LanceDB legislation_text to PostgreSQL+pgvector.

Usage:
    /usr/bin/python3 scripts/migrate_to_pg.py
"""

import json
import time
from datetime import datetime, timezone

import lancedb
import psycopg


def arrow_val(val, col_name, field_type):
    """Convert Arrow scalar to Python value for Postgres."""
    if val is None:
        return None

    ft = str(field_type)

    # Nanosecond timestamps overflow Python datetime
    if "timestamp" in ft:
        try:
            return val.as_py()
        except (ValueError, OverflowError):
            if hasattr(val, "value") and val.value is not None:
                try:
                    return datetime.fromtimestamp(val.value / 1e9, tz=timezone.utc)
                except (ValueError, OSError):
                    return None
            return None

    py = val.as_py()

    if col_name == "embedding" and py is not None:
        return str(py)
    elif col_name == "actors":
        if py is None:
            return None
        if isinstance(py, list):
            return json.dumps(py)
        return None
    else:
        return py


def main():
    print("Loading from LanceDB...")
    start = time.time()
    db = lancedb.connect("data/lancedb")
    tbl = db.open_table("legislation_text")
    arrow = tbl.to_arrow()
    print(f"Loaded {arrow.num_rows:,} rows in {time.time()-start:.1f}s")

    cols = [f.name for f in arrow.schema]
    placeholders = ", ".join(["%s"] * len(cols))
    col_names = ", ".join(cols)
    upsert_set = ", ".join(
        f"{c} = EXCLUDED.{c}" for c in cols if c != "section_id"
    )
    insert_sql = (
        f"INSERT INTO legislation_text ({col_names}) "
        f"VALUES ({placeholders}) "
        f"ON CONFLICT (section_id) DO UPDATE SET {upsert_set}"
    )

    conn = psycopg.connect(
        "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
    )

    BATCH = 1000
    inserted = 0
    batch_start = time.time()

    with conn.cursor() as cur:
        batch = []
        for i in range(arrow.num_rows):
            row = []
            for j, col in enumerate(cols):
                val = arrow.column(col)[i]
                py = arrow_val(val, col, arrow.schema.field(j).type)
                row.append(py)
            batch.append(tuple(row))

            if len(batch) >= BATCH:
                cur.executemany(insert_sql, batch)
                conn.commit()
                inserted += len(batch)
                batch = []
                elapsed = time.time() - batch_start
                rate = inserted / elapsed
                eta = (arrow.num_rows - inserted) / rate if rate > 0 else 0
                print(
                    f"  {inserted:,}/{arrow.num_rows:,} "
                    f"({rate:.0f} rows/s, ETA {eta:.0f}s)"
                )

        if batch:
            cur.executemany(insert_sql, batch)
            conn.commit()
            inserted += len(batch)

    elapsed = time.time() - batch_start
    print(f"Done: {inserted:,} rows in {elapsed:.1f}s ({inserted/elapsed:.0f} rows/s)")

    with conn.cursor() as cur:
        cur.execute("SELECT COUNT(*) FROM legislation_text")
        count = cur.fetchone()[0]
        cur.execute(
            "SELECT COUNT(*) FROM legislation_text WHERE embedding IS NOT NULL"
        )
        emb_count = cur.fetchone()[0]
        print(f"Postgres: {count:,} rows, {emb_count:,} with embeddings")

    conn.close()


if __name__ == "__main__":
    main()
