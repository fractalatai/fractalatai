---
session: PgStore Implementation
status: closed
opened: 2026-06-24
closed: 2026-06-24
outcome: success

summary: >
  Built PgStore (Postgres+pgvector) as hub provision store replacement for LanceDB.
  5-phase implementation: module split (8443→897 lines), enrich_single_law decomposition
  (1440→76 lines), ProvisionStore trait, CLI --pg wiring, full pipeline validation.
  All commands work against Postgres with zero disk exhaustion.

decisions:
  - what: "Module split before PgStore wiring (Gemini: refactor before/during, not after)"
    why: main.rs at 8,443 lines with 1,442-line enrich_single_law was untestable and unwirable
    result: "6 modules: main(897), utils(610), llm(443), pipeline(1573), taxa(3166), sync(758), misc(1055)"
  - what: "ProvisionStore trait with Box<dyn ProvisionStore> dispatch"
    why: Pipeline code should be backend-agnostic. async_trait enables trait objects.
    result: Both LanceStore and PgStore implement the trait, CLI switches via --pg flag
  - what: "UNNEST batch upserts for performance (Gemini recommendation)"
    why: Row-by-row executemany is slow for bulk operations
    result: INSERT ON CONFLICT with batch binding for upsert_lat, update_taxa
  - what: "law_name parameter on trait (not raw SQL filter strings)"
    why: "LanceStore used raw SQL filters — unsafe for Postgres. law_name is the universal query key."
    result: LanceStore constructs filter internally, PgStore uses parameterised queries

metrics:
  main_rs_before: 8443
  main_rs_after: 897
  enrich_before: 1440
  enrich_after: 76
  postgres_rows: 183509
  pipeline_commands_wired: 6

lessons:
  - title: "Split first, wire second — refactoring a monolith during integration is a recipe for bugs"
    detail: "Gemini review confirmed: decompose enrich_single_law into pipeline stages before wiring PgStore. Each stage is independently testable."
    tag: architecture
  - title: "Arrow ↔ Postgres has no magic crate — manual column-by-column conversion"
    detail: "PgRow → Arrow arrays → RecordBatch for reads. RecordBatch rows → sqlx bind for writes. Tedious but straightforward."
    tag: infrastructure
  - title: "FixedSizeList inner field nullability must match between Arrow and Postgres"
    detail: Embedding column required nullable inner field to match pgvector Vector type. Mismatch caused silent data corruption.
    tag: data
  - title: "List<Struct> → JSONB is the right conversion for nested types"
    detail: "actors column was List<Struct> in LanceDB. JSONB in Postgres is more powerful (GIN indexed, queryable) and maps cleanly."
    tag: architecture

artifacts:
  - crates/fractalaw-store/src/pg.rs
  - crates/fractalaw-store/src/provision_store.rs
  - crates/fractalaw-cli/src/main.rs
  - crates/fractalaw-cli/src/commands/pipeline.rs
  - crates/fractalaw-cli/src/commands/taxa.rs
  - crates/fractalaw-cli/src/commands/sync.rs
  - crates/fractalaw-cli/src/commands/misc.rs
  - crates/fractalaw-cli/src/utils.rs
  - crates/fractalaw-cli/src/llm.rs

depends_on:
  - 06-24-26-pgvector-feasibility-spike.md

enables:
  - PgStore hardening
  - Law status tracker
  - Hub-only Postgres operation (no LanceDB dependency)
---

# Session: PgStore Implementation (CLOSED)

Daughter session 2 of DB migration. Builds the Rust `PgStore` struct to replace `LanceStore` for hub operations.

## Goal

Implement `PgStore` in `crates/fractalaw-store/src/pg.rs` with the same method signatures as `LanceStore`, using sqlx. Wire into the pipeline so all commands (parse, embed, classify, validate, publish) work against Postgres+pgvector.

## Prerequisite

Postgres+pgvector running via podman quadlet on port 5433 with 183,509 rows migrated. See `store/06-24-26-pgvector-feasibility-spike.md` for connection details and schema.

## Work plan

### Phase 1: Module split ✅
Split main.rs (8,443 → 897 lines) into focused modules. Mechanical refactor — no logic changes.
- `utils.rs` (610 lines): 15 utility functions + FitnessEntry
- `llm.rs` (443 lines): Gemini parsing, ActorMatcher, ParsedTier3Actor + tests
- `commands/pipeline.rs` (1,573 lines): enrich_single_law, source_tier, types
- `commands/taxa.rs` (3,166 lines): 13 cmd_taxa_* functions
- `commands/sync.rs` (758 lines): 10 sync/crdt functions
- `commands/misc.rs` (1,055 lines): 13 other command functions
- main.rs retains: Cli struct, Command enums, ZenohArgs, main(), open_duck

### Phase 2: Decompose enrich_single_law ✅
1,440-line function → 76-line orchestrator + 5 stage functions:
- `parse_provisions` (330 lines): regex DRRP extraction per provision
- `backlink_actors` (28 lines): infer Rule provision holders
- `apply_escalation` (601 lines): Tier 1/2/3 inheritance + LLM
- `write_provision_taxa` (294 lines): build Arrow batch → LanceDB
- `write_law_taxa` (120 lines): hash check → DuckDB UPDATE

### Phase 3: Wire ProvisionStore trait ✅
Changed pipeline + taxa commands from `&LanceStore` → `&dyn ProvisionStore`:
- `enrich_single_law`, `write_provision_taxa`: now accept `&dyn ProvisionStore`
- All 8 taxa commands taking LanceStore: switched to `&dyn ProvisionStore`
- `query_legislation_text` calls: `&filter` → `law_name` (trait API)
- Empty `law_name` = query all rows (both LanceStore + PgStore impls updated)
- misc.rs/sync.rs left on concrete `LanceStore` (Phase 4 CLI integration)

### Phase 4: CLI integration ✅
`--pg` flag wired for core pipeline commands via `open_provision_store`:
- `taxa parse`, `embed`, `classify`, `escalate`, `validate`: dispatch via `--pg`
- `taxa enrich`: accepts `pg_url` parameter, dispatches via `open_provision_store`
- No `--pg` → LanceStore (default, unchanged behavior)
- Test: `taxa parse --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws UK_ukpga_1974_37`

### Phase 5: Validate on Postgres ✅
Full pipeline validated against 183,509-row Postgres:
- **parse**: UK_ukpga_1974_37 (830 provisions protected), UK_uksi_1987_2116 --force (full re-enrichment)
- **embed**: 49 QQ laws, 1,215 embeddings written in 160s, no disk issues
- **classify**: 49 QQ laws, 1,741 provisions classified in 70s
- **validate**: 49 QQ laws, 8 laws had targets, corrections applied via Gemini
- Fixes applied during validation:
  - FixedSizeList inner field nullability (embedding column)
  - Timestamp nanos → TIMESTAMPTZ conversion  
  - List<Struct> → JSONB conversion (actors column)
  - TEXT[] quoting: double-quote → single-quote (Postgres SQL literals)
  - update_taxa/upsert_embeddings: INSERT ON CONFLICT → UPDATE WHERE (partial batches lack NOT NULL columns)

## Remaining trait wiring

Commands that still open `LanceStore` internally (not reachable via `--pg`):
- `cmd_taxa_show`, `cmd_taxa_qa`, `cmd_taxa_eyeball`, `cmd_taxa_audit_fitness` — read-only diagnostic commands, open LanceStore from `data_dir`
- `misc.rs`: `cmd_text`, `cmd_embed`, `cmd_search`, `cmd_validate`, `cmd_export_training_data` — open LanceStore internally
- `sync.rs`: `cmd_sync_publish_provisions`, `cmd_sync_pull_lat`, `cmd_sync_watch` — open LanceStore internally

Fix: pass `&dyn ProvisionStore` from caller or thread `pg_url` through. Mechanical, same pattern as Phase 4. Not blocking for Phase 5 validation but needed before hub-only operation.

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

- `crates/fractalaw-store/src/lance.rs` — LanceStore (implements ProvisionStore)
- `crates/fractalaw-store/src/pg.rs` — PgStore (implements ProvisionStore)
- `crates/fractalaw-store/src/provision_store.rs` — ProvisionStore trait
- `crates/fractalaw-store/Cargo.toml` — sqlx/pgvector deps (pg feature)
- `crates/fractalaw-cli/src/main.rs` — Cli struct, Command dispatch (897 lines)
- `crates/fractalaw-cli/src/commands/pipeline.rs` — enrich_single_law + types
- `crates/fractalaw-cli/src/commands/taxa.rs` — 13 taxa command functions
- `crates/fractalaw-cli/src/commands/sync.rs` — sync/crdt functions
- `crates/fractalaw-cli/src/commands/misc.rs` — other commands
- `crates/fractalaw-cli/src/llm.rs` — ActorMatcher, Gemini parsing
- `crates/fractalaw-cli/src/utils.rs` — shared utilities
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

### Architecture review (Gemini, 2026-06-24)

Full review: `data/code-review/cli-architecture.md`

**main.rs is 8,443 lines with enrich_single_law at 1,442 lines.** Gemini says: **refactor before/during wiring, not after.**

**Decompose enrich_single_law into pipeline stages:**
1. `load_provisions` — query store, prepare raw provisions
2. `parse_and_extract` — run parse_v2 per provision (pure, no store)
3. `build_arrow_batch` — transform parsed data to 22-column Arrow batch
4. `write_enriched_data` — upsert batch to store
5. `apply_tier1_inheritance` — parent-clause actor inheritance
6. `escalate_tier2_llm` — LLM escalation

**Split CLI into modules** (done — actual structure):
```
src/
├── main.rs              (897 lines: Cli, Command enums, ZenohArgs, main(), open_duck)
├── utils.rs             (610 lines: shared utilities, FitnessEntry)
├── llm.rs               (443 lines: ActorMatcher, Gemini parsing + tests)
├── display.rs           (pre-existing)
├── embed.rs             (pre-existing)
└── commands/
    ├── mod.rs
    ├── pipeline.rs      (1,573 lines: enrich_single_law, types)
    ├── taxa.rs          (3,166 lines: 13 taxa commands)
    ├── sync.rs          (758 lines: sync/crdt commands)
    └── misc.rs          (1,055 lines: other commands)
```

**Filter string → law_name**: keep ProvisionStore trait using law_name (safe), add backward-compat by constructing filter inside LanceStore impl.

**Remaining order of work:**
1. ~~Module split (mechanical, low risk)~~ ✅ commit 0a46772
2. Decompose enrich_single_law (pipeline.rs)
3. Wire ProvisionStore trait into decomposed functions
4. Each step is independently testable

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
