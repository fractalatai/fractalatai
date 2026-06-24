# Session: pgvector Feasibility Spike (ACTIVE)

## Goal

Prove pgvector can replace LanceDB on the hub. Stand up Postgres+pgvector in a container, map the 59-column schema, migrate data, and prototype the key operations (upsert, vector search, metadata filter).

## Steps

### 1. Container setup

```bash
# pgvector official image — Postgres 17 + pgvector 0.8
podman run -d --name fractalaw-pg \
  -e POSTGRES_DB=fractalaw \
  -e POSTGRES_USER=fractalaw \
  -e POSTGRES_PASSWORD=fractalaw \
  -p 5433:5432 \
  -v fractalaw-pgdata:/var/lib/postgresql/data \
  pgvector/pgvector:pg17

# Verify
psql -h localhost -p 5433 -U fractalaw -c "CREATE EXTENSION IF NOT EXISTS vector;"
```

Port 5433 to avoid conflict with any existing Postgres.

### 2. Schema mapping

Map the 59 LanceDB columns to Postgres types:

| LanceDB type | Postgres type | Notes |
|---|---|---|
| Utf8 | TEXT | Direct |
| Int32 | INTEGER | Direct |
| Float32 | REAL | Direct |
| Timestamp(ns, UTC) | TIMESTAMPTZ | Direct |
| FixedSizeList<Float32, 384> | vector(384) | pgvector native |
| List<Utf8> | TEXT[] | Postgres arrays |
| List<Struct{...}> | JSONB | actors struct → JSONB array |
| List<UInt32> | INTEGER[] | token_ids |

Key design decisions:
- `section_id` as PRIMARY KEY (currently the merge_insert key)
- `law_name` indexed for per-law queries
- `actors` as JSONB (rich querying with GIN index)
- `drrp_history` as JSONB (already JSON string in LanceDB)
- `embedding` as `vector(384)` with HNSW index

### 3. Data migration

Export from LanceDB → load into Postgres:
- Export via pyarrow to Parquet or Arrow IPC
- Load via psycopg/sqlx bulk insert
- Verify: row counts, embedding checksums, JSONB integrity

### 4. Prototype key operations

Implement in Python first (fast iteration), then Rust:

a. **Upsert provisions** (the merge_insert replacement):
```sql
INSERT INTO legislation_text (section_id, law_name, text, drrp_types, ...)
VALUES ($1, $2, $3, $4, ...)
ON CONFLICT (section_id) DO UPDATE SET text = EXCLUDED.text, ...;
```

b. **Vector similarity search** (the search_text replacement):
```sql
SELECT section_id, text, 1 - (embedding <=> $1) AS similarity
FROM legislation_text
WHERE law_name = $2
ORDER BY embedding <=> $1
LIMIT 10;
```

c. **Filtered query** (the query_legislation_text replacement):
```sql
SELECT * FROM legislation_text
WHERE law_name = $1
ORDER BY sort_key
LIMIT $2 OFFSET $3;
```

d. **Provision taxa query** (the query_provision_taxa replacement):
```sql
SELECT section_id, drrp_types, actors, extraction_method, ...
FROM legislation_text
WHERE law_name = $1 AND drrp_types IS NOT NULL;
```

### 5. Benchmarks

Compare against LanceDB for our actual workload:
- Upsert 10K provisions (single law parse): time + disk usage
- Upsert 167K provisions (full corpus parse): time + disk usage
- Vector search (384-dim, top-10, filtered by law_name): latency
- No compaction needed — measure disk usage stays stable

### 6. Evaluate

- Does it eliminate disk exhaustion? (no fragment bloat)
- Is upsert performance acceptable? (< 1 minute for full corpus)
- Is query performance comparable? (< 100ms for filtered queries)
- Does JSONB work for actors struct + drrp_history?
- Is the container approach operationally simple?

## Environment

- Container runtime: podman 5.8.2 (also docker 29.4.3)
- Host: Fedora Bluefin DX
- Rust: sqlx crate for async Postgres
- Python: psycopg for prototyping
- Disk: 116GB total, ~26GB free after LanceDB compact

## Key files

- `crates/fractalaw-core/src/schema.rs` — 59-column schema definition
- `crates/fractalaw-store/src/lance.rs` — LanceStore methods to replicate
- `scripts/compact_lance.py` — current data export (reusable for migration)
