# Session: PgStore Implementation (PENDING)

Daughter session 2 of DB migration. Builds the Rust `PgStore` struct to replace `LanceStore` for hub operations.

## Goal

Implement `PgStore` in `crates/fractalaw-store/src/pg.rs` with the same method signatures as `LanceStore`, using sqlx. Wire into the pipeline so all commands (parse, embed, classify, validate, publish) work against Postgres+pgvector.

## Prerequisite

Postgres+pgvector running via podman quadlet on port 5433 with 183,509 rows migrated. See `store/06-24-26-pgvector-feasibility-spike.md` for connection details and schema.

## Carried from feasibility spike

- ⬜ Filtered query benchmarks (latency measurement)
- ⬜ JSONB validation with `jsonb_pretty()` for actors/drrp_history
- ⬜ Enable quadlet on boot (`systemctl --user enable fractalaw-pg.service`)

## Implementation plan

### 1. Add sqlx + pgvector to workspace

```toml
# Cargo.toml (workspace)
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "json", "chrono"] }

# crates/fractalaw-store/Cargo.toml
sqlx = { workspace = true, optional = true }
pgvector = { version = "0.4", optional = true }
```

Feature gate: `pg` feature on `fractalaw-store`, similar to `duckdb`/`lancedb`/`datafusion` gates.

### 2. PgStore struct

```rust
// crates/fractalaw-store/src/pg.rs
pub struct PgStore {
    pool: sqlx::PgPool,
}

impl PgStore {
    pub async fn connect(url: &str) -> Result<Self, StoreError>;

    // Read operations (matching LanceStore signatures)
    pub async fn query_legislation_text(&self, filter: &str, limit: usize, offset: usize) -> Result<Vec<RecordBatch>>;
    pub async fn query_provision_taxa(&self, law_name: &str) -> Result<Vec<RecordBatch>>;
    pub async fn search_text(&self, embedding: &[f32], law_name: Option<&str>, limit: usize) -> Result<Vec<RecordBatch>>;

    // Write operations (replacing merge_insert)
    pub async fn upsert_lat(&self, batch: &RecordBatch) -> Result<()>;
    pub async fn upsert_embeddings(&self, batch: &RecordBatch) -> Result<()>;
    pub async fn update_taxa(&self, batch: RecordBatch) -> Result<()>;

    // No compact() needed!
}
```

### 3. Arrow ↔ Postgres conversion

The pipeline uses Arrow RecordBatch throughout. PgStore needs:
- **RecordBatch → Postgres rows**: extract columns, convert types, executemany
- **Postgres rows → RecordBatch**: query results back to Arrow for pipeline consumption

This is the main implementation effort. Options:
- Manual column-by-column conversion (like migrate_to_pg.py does in Python)
- Use `arrow-odbc` or `connectorx` for automatic conversion
- Keep it simple: manual conversion in PgStore methods

### 4. CLI integration

Add `--store pg` flag or `FRACTALAW_STORE=pg` env var to CLI. When set, create PgStore instead of LanceStore. Most pipeline code takes `&LanceStore` — need to either:
- Create a trait `ProvisionStore` that both implement
- Or duplicate the store parameter (less clean but faster to ship)

Trait approach is cleaner and aligns with micro-apps architecture (edge uses LanceStore, hub uses PgStore).

### 5. Pipeline commands to wire up

| Command | LanceStore methods used | Notes |
|---------|------------------------|-------|
| `taxa parse` | `query_legislation_text`, `update_taxa`, `upsert_embeddings` | Core write path |
| `taxa embed` | `query_legislation_text`, `upsert_embeddings` | Embedding writes |
| `taxa classify` | `query_legislation_text`, `upsert_embeddings` | DRRP + position writes |
| `taxa validate` | `query_legislation_text` | Read-only (audit logs are files) |
| `taxa show` | `query_legislation_text` | Read-only |
| `sync pull-lat` | `upsert_lat` | LAT ingestion from sertantai |
| `sync publish` | `query_provision_taxa` | Read for zenoh publish |

### 6. Testing

- Run `taxa parse --laws UK_ukpga_1974_37 --force` against PgStore
- Verify same DRRP output as LanceStore
- Run `taxa embed --laws UK_ukpga_1974_37` — embeddings written to Postgres
- Run `taxa classify` — no disk exhaustion!
- Benchmark: full corpus parse time vs LanceStore

## Key files

- `crates/fractalaw-store/src/lance.rs` — LanceStore to mirror (method signatures)
- `crates/fractalaw-store/src/pg.rs` — new PgStore (to create)
- `crates/fractalaw-store/Cargo.toml` — add sqlx dependency
- `crates/fractalaw-cli/src/main.rs` — wire PgStore into commands
- `scripts/pg_schema.sql` — Postgres schema
- `scripts/migrate_to_pg.py` — data migration (already done)

## Gemini feedback (2026-06-24)

Full review: `data/code-review/pgstore-implementation.md`

### Arrow ↔ Postgres conversion

No magic crate — build manually. For reads: fetch `PgRow`s via sqlx, collect into `Vec<Option<T>>` per column, convert to Arrow arrays, build `RecordBatch`. For writes: iterate RecordBatch rows, extract values, bind to sqlx query.

**pgvector + sqlx**: the `pgvector` crate provides `sqlx::Type`/`Encode`/`Decode` for `pgvector::Vector`. Read: `row.try_get::<Option<Vector>, _>("embedding")` → access `.0` for `Vec<f32>`. Write: `pgvector::Vector::from(&[f32])` before binding.

### Trait abstraction: yes, do it

Use `async_trait` crate. Define `ProvisionStore` trait with all read/write methods. Both `LanceStore` and `PgStore` implement it. CLI uses `Box<dyn ProvisionStore>` via factory function. This aligns with micro-apps architecture (edge=LanceStore, hub=PgStore).

### Upsert: UNNEST for batch performance

For batch upserts, use `UNNEST($1::type[], $2::type[], ...)` instead of row-by-row `executemany`. Build the INSERT...ON CONFLICT query dynamically from the RecordBatch schema. Map Arrow DataType → Postgres type for UNNEST casts. This is the most performant approach without COPY.

### Feature gating

Standard pattern: `pg` feature on `fractalaw-store` with `sqlx` and `pgvector` as optional deps. CLI enables features it needs. Same as existing `duckdb`/`lancedb`/`datafusion` gates.

### Key risk: filter string safety

LanceStore passes raw SQL filter strings (e.g. `"law_name = 'UK_ukpga_1974_37'"`). PgStore must sanitise these or convert to parameterised queries. Don't pass raw strings to Postgres.

## Connection details

```
Host:     localhost
Port:     5433
Database: fractalaw
User:     fractalaw
Password: fractalaw
URL:      postgres://fractalaw:fractalaw@localhost:5433/fractalaw
```

Quadlet: `~/.config/containers/systemd/fractalaw-pg.container`
Start: `systemctl --user start fractalaw-pg.service`
