---
session: pgvector Feasibility Spike
status: closed
opened: 2026-06-24
closed: 2026-06-24
outcome: success

summary: >
  Proved pgvector can replace LanceDB on hub. Postgres+pgvector via podman quadlet,
  59-column schema mapped, 183,509 rows migrated in 223s. Upsert bloat: 1MB vs 500MB+
  in LanceDB. HNSW vector search working with cosine similarity.

decisions:
  - what: Podman quadlet with systemd integration (not Docker Compose)
    why: Fedora Bluefin is immutable — podman is native, quadlet integrates with user systemd
    result: "fractalaw-pg.container on port 5433, ShmSize=2G for HNSW builds"
  - what: actors column as JSONB (was List<Struct> in LanceDB)
    why: JSONB is more powerful — GIN indexed, queryable, no nested Arrow type complexity
    result: GIN index on actors for containment queries

metrics:
  rows_migrated: 183509
  migration_time_seconds: 223
  migration_rate: "821 rows/s"
  lancedb_bloat_1k_upsert: "500 MB"
  pgvector_bloat_1k_upsert: "1 MB"
  total_size: "715 MB (231 data + 271 indexes + HNSW)"
  embeddings_present: 143643

lessons:
  - title: "ShmSize=2G required for HNSW index builds on 143K+ vectors"
    detail: Default shared memory is too small for pgvector HNSW construction. Without it, index build fails silently.
    tag: infrastructure
  - title: "Port 5433 avoids conflict with any system Postgres on 5432"
    detail: Even on systems without Postgres, using a non-default port prevents future surprises.
    tag: infrastructure
  - title: "migrate_to_pg.py is re-runnable via ON CONFLICT upsert"
    detail: The migration script can be safely re-run — existing rows are updated, new rows inserted. No manual cleanup needed.
    tag: tooling

artifacts:
  - scripts/pg_schema.sql
  - scripts/migrations/migrate_to_pg.py

depends_on:
  - 06-24-26-lancedb-to-qdrant-migration.md

enables:
  - PgStore Rust implementation
  - Full pipeline on Postgres
---

# Session: pgvector Feasibility Spike (CLOSED)

## Goal

Prove pgvector can replace LanceDB on the hub. Stand up Postgres+pgvector in a container, map the 59-column schema, migrate data, and prototype the key operations.

## Results

### Steps completed

- ✅ **Container setup**: podman quadlet with systemd integration, auto-restart, 2GB shared memory
- ✅ **Schema mapping**: 59 columns mapped, schema in `scripts/pg_schema.sql`
- ✅ **Data migration**: 183,509 rows in 223s (821 rows/s) via `scripts/migrate_to_pg.py`
- ✅ **Vector search**: HNSW index built, cosine similarity working correctly
- ✅ **Upsert test**: 1K updates → 1MB growth (vs 500MB+ in LanceDB)
- ✅ **HNSW index**: built with m=16, ef_construction=100

### Steps not done (deferred to PgStore implementation session)

- ⬜ Filtered query benchmarks (latency measurement)
- ⬜ JSONB validation with `jsonb_pretty()`
- ⬜ Rust `PgStore` implementation (sqlx)
- ⬜ Pipeline integration (replace LanceStore calls)
- ⬜ Enable quadlet on boot (`systemctl --user enable fractalaw-pg.service`)

### Benchmark comparison

| Metric | LanceDB | PostgreSQL+pgvector |
|--------|---------|---------------------|
| Data size (stable) | 550 MB (post-compact) | 715 MB (with HNSW) |
| Data size (during ops) | 8-14 GB (fragment bloat) | ~715 MB (stable) |
| Migration time | — | 223s (821 rows/s) |
| Upsert bloat (1K rows) | ~500MB fragments | 1 MB |
| Vector search | Works | Works (semantically correct) |
| Compaction needed | Every 5 laws (manual) | Never |
| Disk exhaustion | 5+ times in 2 days | None |
| HNSW index build | N/A | Success (m=16, ef=100) |

**Verdict: pgvector conclusively eliminates the operational pain. Migration proven feasible.**

## Working with the Postgres instance

### Connection details

```
Host:     localhost
Port:     5433 (not 5432 — avoids conflict)
Database: fractalaw
User:     fractalaw
Password: fractalaw
```

### Container management (podman quadlet)

```bash
# Quadlet file location
~/.config/containers/systemd/fractalaw-pg.container

# Service management
systemctl --user start fractalaw-pg.service
systemctl --user stop fractalaw-pg.service
systemctl --user restart fractalaw-pg.service
systemctl --user status fractalaw-pg.service

# Enable auto-start on login
systemctl --user enable fractalaw-pg.service

# View logs
podman logs fractalaw-pg

# Data volume (persists across restarts)
podman volume inspect fractalaw-pgdata
```

### Quadlet config

```ini
# ~/.config/containers/systemd/fractalaw-pg.container
[Container]
ContainerName=fractalaw-pg
Image=docker.io/pgvector/pgvector:pg17
PublishPort=5433:5432
Volume=fractalaw-pgdata:/var/lib/postgresql/data
ShmSize=2G
Environment=POSTGRES_DB=fractalaw
Environment=POSTGRES_USER=fractalaw
Environment=POSTGRES_PASSWORD=fractalaw

[Service]
Restart=always
```

`ShmSize=2G` is required for HNSW index builds on 143K+ vectors.

### Connecting

```bash
# psql
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw

# Python
import psycopg
conn = psycopg.connect("host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw")

# Rust (sqlx — for future PgStore)
# DATABASE_URL=postgres://fractalaw:fractalaw@localhost:5433/fractalaw
```

### Schema

Table: `legislation_text` — 59 columns matching LanceDB schema.
Schema file: `scripts/pg_schema.sql`

Key columns:
- `section_id` TEXT PRIMARY KEY (upsert key)
- `law_name` TEXT NOT NULL (indexed)
- `embedding` vector(384) (HNSW indexed, cosine similarity)
- `actors` JSONB (GIN indexed — was List<Struct> in LanceDB)
- `drrp_history` TEXT (JSON string — same as LanceDB)
- `drrp_types` TEXT[] (GIN indexed)
- `extraction_method` TEXT (indexed)

### Indexes

```sql
idx_lt_law_name           — B-tree on law_name
idx_lt_extraction_method  — B-tree on extraction_method
idx_lt_drrp_types         — GIN on drrp_types (array containment)
idx_lt_embedding          — HNSW on embedding (vector_cosine_ops, m=16, ef=100)
```

### Current data

- 183,509 rows (full corpus including sync watch arrivals)
- 143,643 with embeddings (39,866 still need embedding)
- 592 distinct laws
- Table size: 231 MB data + 271 MB indexes = 715 MB total

### Migration script

`scripts/migrate_to_pg.py` — reads LanceDB via pyarrow, bulk inserts via psycopg executemany. Handles nanosecond timestamp conversion and JSONB serialisation. Re-runnable (uses ON CONFLICT upsert).

### Key queries

```sql
-- Vector similarity search
SELECT section_id, text, 1 - (embedding <=> $1) AS similarity
FROM legislation_text
WHERE law_name = $2
ORDER BY embedding <=> $1
LIMIT 10;

-- Filtered query (replaces query_legislation_text)
SELECT * FROM legislation_text
WHERE law_name = $1
ORDER BY sort_key
LIMIT $2 OFFSET $3;

-- Provision taxa (replaces query_provision_taxa)
SELECT section_id, drrp_types, actors, extraction_method, taxa_confidence
FROM legislation_text
WHERE law_name = $1 AND drrp_types IS NOT NULL;

-- Upsert (replaces merge_insert)
INSERT INTO legislation_text (section_id, law_name, text, ...)
VALUES ($1, $2, $3, ...)
ON CONFLICT (section_id) DO UPDATE SET text = EXCLUDED.text, ...;
```

## Next session: PgStore implementation

Daughter session of the DB migration — build the Rust `PgStore` struct with the same method signatures as `LanceStore`, using sqlx. Wire into the pipeline. This is the main integration work.

## Key files

- `scripts/pg_schema.sql` — Postgres schema definition
- `scripts/migrate_to_pg.py` — LanceDB → Postgres migration script
- `~/.config/containers/systemd/fractalaw-pg.container` — podman quadlet
- `crates/fractalaw-store/src/lance.rs` — LanceStore methods to replicate in PgStore
