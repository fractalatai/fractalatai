---
description: Safe patterns for schema changes, data writes, and migrations across DuckDB, Postgres, and LanceDB
---

# Skill: Database Changes

## When This Applies

When modifying database schemas, adding columns, writing data, or migrating between stores. Covers all three data stores in the fractalaw pipeline.

## The Three Stores

| Store | Location | Role | Record Unit |
|-------|----------|------|-------------|
| DuckDB | `data/fractalaw.duckdb` | Law-level LRT metadata, publish tracking | 1 row per law |
| Postgres | `localhost:5433` | Provision-level hub — text, actors, embeddings, taxa | 1 row per provision (183K+) |
| LanceDB | `data/lancedb/` | Read-only outbound provisions + 384-dim embeddings | 1 row per provision (97K) |

**Data flows one way**: sertantai LAT -> LanceDB/Postgres -> DRRP pipeline -> DuckDB -> publish -> sertantai.

LanceDB is read-only for outbound data. Only DuckDB (LRT) gets published to Zenoh/sertantai.

## DuckDB: Schema Changes

DuckDB schema changes use idempotent `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`. This is the safest store to modify.

**Pattern** — add an `ensure_*_columns()` method in `crates/fractalaw-store/src/duck.rs`:

```rust
pub fn ensure_my_new_columns(&self) -> anyhow::Result<()> {
    self.execute(
        "ALTER TABLE legislation ADD COLUMN IF NOT EXISTS my_field VARCHAR"
    )?;
    self.execute(
        "ALTER TABLE legislation ADD COLUMN IF NOT EXISTS my_count INTEGER"
    )?;
    Ok(())
}
```

Then call it at the start of the CLI command that needs the columns.

**Existing examples**: `ensure_taxa_hash_columns()`, `ensure_fitness_columns()`, `ensure_triage_columns()`, `ensure_enrichment_queue_columns()`, `ensure_pipeline_status_columns()`.

**Rules**:
- Always use `IF NOT EXISTS` — the method may be called on every startup
- Call `ensure_*` at command entry, not deep inside business logic
- DuckDB supports `List<Struct>` columns — use `VARCHAR` with JSON for complex nested data, or native list types
- Initial population comes from Parquet import (`load_all`). Per-law updates use `upsert_legislation()` (delete + insert from temp Parquet)

## Postgres: Schema Changes

Postgres schema is declared upfront in `scripts/pg_schema.sql`. The Rust `PgStore` does not add columns at runtime.

**Pattern** — edit `scripts/pg_schema.sql`, then apply:

```bash
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c \
  "ALTER TABLE legislation_text ADD COLUMN IF NOT EXISTS my_field TEXT"
```

Then update the relevant Rust queries in `crates/fractalaw-store/src/pg.rs` to read/write the new column.

**Tables**:
- `legislation_text` — provisions (PK: `section_id`)
- `provision_actors` — per-provision x actor tier signals (PK: `(section_id, actor_label)`)
- `gold_benchmarks` — benchmark golden labels

**Write methods**: `upsert_lat()` (INSERT ... ON CONFLICT DO UPDATE), `update_taxa()`, `update_polished()`, `upsert_provision_actors()`.

**Rules**:
- Always use `IF NOT EXISTS` in ALTER statements
- Keep `pg_schema.sql` as the source of truth — don't add columns only in Rust
- Postgres is the hub for provision-level data. If you need per-provision writes, write here

## LanceDB: Schema Changes — DANGER ZONE

LanceDB has no `ALTER TABLE ADD COLUMN IF NOT EXISTS`. Schema changes require careful handling.

### Safe: Adding scalar columns

Use `add_columns()` with SQL expressions in `crates/fractalaw-store/src/lance.rs`:

```rust
// Check schema first
let schema = table.schema().await?;
if schema.field_with_name("my_field").is_err() {
    table.add_columns(
        NewColumnTransform::SqlExpressions(vec![
            ("my_field".into(), "CAST(NULL AS VARCHAR)".into())
        ]),
        None,
    ).await?;
}
```

### Unsafe: Adding complex columns (List<Struct>)

Complex columns cannot be added via `add_columns()`. They require a full table rebuild:

1. Export to Parquet backup first
2. Use `scripts/compact_lance.py` (export -> drop -> recreate with new schema)
3. Verify row counts match before and after

### NEVER do these

- **NEVER call `create_table_from_batches()` or `create_table_from_parquet()` on production** — both call `drop_table()` internally, destroying all data including embeddings (9 hours to recompute on CPU)
- **NEVER `--force` re-enrich the full corpus** — `merge_insert` creates ~25x write amplification (~8GB new fragments per full pass)
- **NEVER manually delete Lance data fragments** — binary manifest grep is unreliable and corrupts the table
- **NEVER run `fractalaw embed` on production without explicit confirmation** — embeddings take ~9 hours

### Before any bulk LanceDB operation

1. Back up to Parquet: `scripts/maintenance/backup_lancedb.py`
2. Verify backup row count matches source
3. Proceed with the operation
4. If it fails, restore from the Parquet backup (seconds vs 9 hours)

## When to Use Each Store

| Need | Store | Why |
|------|-------|-----|
| Add a law-level metadata field | DuckDB | One row per law, idempotent ALTER |
| Add a provision-level field | Postgres | Hub for per-provision data, proper ALTER support |
| Add embeddings or vector search | Postgres (pgvector) | Supports `<=>` similarity, no fragment bloat |
| Read provision text for outbound | LanceDB | Read-only for outbound data |
| Track publish state | DuckDB | `taxa_hash` / `published_hash` columns |
| Store pipeline timestamps | DuckDB | `ensure_pipeline_status_columns()` pattern |
| Store per-actor tier signals | Postgres | `provision_actors` table, per-tier columns |

## New Law Arrival Pattern

When a new law arrives via Zenoh sync watch:

1. Pull LRT record from sertantai (`query_lrt`) -> `upsert_legislation()` into DuckDB
2. Pull LAT provisions -> upsert into LanceDB/Postgres
3. Embed -> classify -> enrich
4. Backfill DuckDB from provision data
5. Publish via `fractalaw-sync-cli`

**Critical**: without the DuckDB LRT row first, enrichment UPDATE hits zero rows and publish finds nothing.
