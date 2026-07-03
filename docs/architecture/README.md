# Architecture Documentation

Specifications and contracts for the fractalaw data model, sync protocol, and service boundaries.

## Schema Reference

`SCHEMA-REFERENCE.md` is **auto-generated** from live databases — do not edit manually.

### Regenerating

```bash
# Requires Postgres running (systemctl --user start fractalaw-pg.service)
/usr/bin/python3 scripts/maintenance/schema_docs.py

# Postgres only (skip DuckDB)
/usr/bin/python3 scripts/maintenance/schema_docs.py --no-duckdb

# DuckDB only (skip Postgres)
/usr/bin/python3 scripts/maintenance/schema_docs.py --no-pg
```

The script queries `information_schema.columns` and `pg_indexes` for Postgres, and `DESCRIBE` for DuckDB. Output overwrites `SCHEMA-REFERENCE.md` with the current column definitions, types, nullability, and indexes.

### When to regenerate

- After adding columns to `scripts/pg_schema.sql`
- After running `ALTER TABLE` on Postgres or DuckDB
- Before publishing schema changes to sertantai
- Before starting a new session that depends on schema knowledge

### Historical schema docs

The original hand-written schema documents (SCHEMA.md v0.8, SCHEMA-DIAGRAM.md v0.6, LAT-SCHEMA-FOR-SERTANTAI.md, LAT-TRANSFORMS-FOR-SERTANTAI.md) are archived in `.claude/sessions/archive/`. They contain design rationale and migration history that the auto-generated reference does not capture.

## Contents

| File | Description |
|------|-------------|
| `SCHEMA-REFERENCE.md` | Auto-generated schema for Postgres + DuckDB tables |
| `ZENOH-SYNC.md` | Zenoh sync spec — data flow, published schema, CLI reference |
| `ZENOH-LAT-ARROW-IPC.md` | Arrow IPC wire format for LAT over Zenoh |
| `ZENOH-LAT-DELETION-SIGNAL.md` | LAT deletion signal specification |
| `sertantai-zenoh-subscriber.md` | Design handoff for sertantai Zenoh integration |
