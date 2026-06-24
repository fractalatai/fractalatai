# Session: Law Status Tracker (PENDING)

## Problem

We repeatedly run ad-hoc Python queries to determine where each law is in the pipeline. With 274 QQ laws across multiple stages (LAT pull → embed → parse → classify → validate → adjudicate → publish), it's hard to know at a glance what's done, what's pending, and what's blocked.

Every moving part — sync watch, manual parse, classify, validate, embed — updates different columns in LanceDB/DuckDB but there's no unified view of per-law pipeline status.

## What we need

A per-law status tracker that answers:
- Has LAT been pulled? (provisions in LanceDB)
- Are embeddings computed? (embedding column populated)
- Has it been parsed? (extraction_method set)
- Has it been classified? (classifier tier in drrp_history)
- Has it been validated? (audit log exists)
- Has it been adjudicated? (adjudication log exists)
- Has it been published? (published_hash matches taxa_hash in DuckDB)
- Is it blocked? (no LAT from sertantai, excluded, etc.)

## Options

### Option A: DuckDB status view

Add a `pipeline_status` view or table to DuckDB that joins LanceDB stats (via periodic refresh) with DuckDB metadata. Query with `fractalaw query "SELECT * FROM pipeline_status WHERE customer = 'QQ'"`.

Pros: SQL-queryable, joins naturally with existing DuckDB data, can filter by family/customer.
Cons: Requires periodic refresh from LanceDB (not real-time), another table to maintain.

### Option B: CLI status command

`fractalaw taxa status --laws <csv>` or `fractalaw taxa status --customer QQ` that queries both LanceDB and DuckDB on the fly and prints a summary table.

Pros: Always live, no sync needed, single command.
Cons: Slower (queries LanceDB every time), output only (not queryable from other tools).

### Option C: Status JSON file

Write `data/pipeline-status.json` after each pipeline operation (parse, classify, validate). Sync watch updates it too. A simple JSON file with per-law status.

Pros: Fast to read, can be synced to NAS, consumed by other tools.
Cons: Can get stale if operations don't update it, another file to manage.

### Option D: DuckDB table + CLI command

Maintain a `law_pipeline_status` table in DuckDB with columns: `law_name, has_lat, has_embeddings, parsed_at, classified_at, validated_at, adjudicated_at, published_at, status`. Update it from each pipeline stage. CLI command reads and displays it.

Pros: Persistent, SQL-queryable, CLI-accessible, updated by each stage.
Cons: Need to wire updates into every pipeline command.

## Recommendation

**Option D** — a DuckDB table is the natural home. It's already where law metadata lives. The CLI commands already open DuckDB. Adding a status update after each stage is minimal wiring. The `fractalaw taxa status` command reads it.

The table could also track customer/register membership (QQ, etc.) so filtering by customer is built in.

## Key files

- `crates/fractalaw-cli/src/main.rs` — all pipeline commands (parse, classify, validate, publish)
- `crates/fractalaw-store/src/duck.rs` — DuckDB operations
- `data/qq-applicable-laws.csv` — current customer law list

## Integration points

Each pipeline stage would update the status table:
- `sync watch` → sets `has_lat = true, lat_pulled_at = now`
- `taxa embed` → sets `has_embeddings = true, embedded_at = now`
- `taxa parse` → sets `parsed_at = now`
- `taxa classify` → sets `classified_at = now`
- `taxa validate` → sets `validated_at = now`
- `/human-review` apply → sets `adjudicated_at = now`
- `sync publish` → sets `published_at = now`
