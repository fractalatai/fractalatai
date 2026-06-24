# Session: Vector DB Migration — pgvector hub + LanceDB edge (ACTIVE)

## Meta-plan — daughter sessions for investigation + implementation

## Motivation

LanceDB fragment bloat is a recurring operational pain:
- `merge_insert` creates ~25x write amplification (167K rows × update = 14GB fragments)
- Disk fills to 0% during corpus-wide operations (happened 3 times in 2 days)
- `compact()` rebuilds from scratch (export → drop → recreate) — not native compaction
- `pylance` not installed, so no incremental compaction available
- Schema changes require table rebuild (export → add column → drop → recreate)
- Binary fragment files cannot be manually inspected or repaired

## Current LanceDB usage

`LanceStore` in `crates/fractalaw-store/src/lance.rs` wraps all LanceDB access:

**Read operations:**
- `search_text()` — vector similarity search (384-dim embeddings)
- `query_legislation_text()` — SQL filter with limit/offset
- `query_provision_taxa()` — provision taxa projection for zenoh publish
- `query_unpolished()` — find unenriched provisions

**Write operations (fragment-creating):**
- `upsert_lat()` — merge_insert LAT from sertantai
- `upsert_embeddings()` — merge_insert embeddings
- `update_taxa()` — merge_insert DRRP classification results
- `update_polished()` — merge_insert AI refinements

**Maintenance:**
- `compact()` — full rebuild (the nuclear option)
- `ensure_gap_c_columns()` — schema migration via add_columns

**Schema:** 59 columns on `legislation_text` table, including nested types (List<Struct> for actors, FixedSizeList<Float32, 384> for embeddings, JSON strings for drrp_history).

## Qdrant considerations

### Potential advantages
- **No fragment bloat** — point-based architecture, updates are in-place
- **Native compaction** — WAL-based, no export/drop/recreate cycle
- **Better concurrency** — multiple readers/writers without lock conflicts
- **Filtering** — payload-based filtering alongside vector search
- **Snapshots** — built-in backup/restore without manual Parquet export
- **gRPC API** — Rust client available (`qdrant-client` crate)

### Potential challenges
- **Schema mapping** — 59 columns including nested structs need mapping to Qdrant payloads
- **No SQL** — LanceDB supports SQL queries via DataFusion; Qdrant has its own filter language
- **Arrow integration** — current pipeline uses Arrow RecordBatch throughout; Qdrant uses its own types
- **Bulk operations** — merge_insert-by-key pattern needs reimplementation as upsert-by-id
- **Migration** — 167K points with 384-dim vectors + rich payloads need data migration
- **Local-first** — LanceDB is embedded (no server); Qdrant needs a running server process
- **Cost of change** — LanceStore is deeply integrated; every pipeline stage uses it

## Daughter sessions (when ready to proceed)

### 1. Qdrant feasibility spike
- Stand up Qdrant locally (Docker or native)
- Map the 59-column schema to Qdrant collection + payload fields
- Prototype: create collection, upsert 1K points, query, filter
- Measure: insert speed, query latency, disk usage vs LanceDB

### 2. QdrantStore abstraction
- Design a `QdrantStore` with the same method signatures as `LanceStore`
- Trait-based abstraction so pipeline code doesn't need to know which backend is used
- Handle the Arrow ↔ Qdrant type conversion

### 3. Data migration
- Export LanceDB → Qdrant migration script
- Verify row counts, embedding integrity, payload completeness
- Dual-read validation (query both, compare results)

### 4. Pipeline integration
- Wire `QdrantStore` into all pipeline stages
- Test: parse → classify → validate → publish cycle on Qdrant
- Performance comparison vs LanceDB

### 5. Cutover
- Final data sync
- Switch default store
- Decommission LanceDB

## Decision criteria

- Does Qdrant eliminate the disk exhaustion problem?
- Is query performance comparable or better?
- Can we preserve the Arrow-based data flow or does the pipeline need significant rework?
- Is the operational overhead of running a Qdrant server acceptable for a local-first architecture?
- Can we migrate without data loss?

## Gemini review (2026-06-24)

Full review: `data/code-review/vector-db-migration.md`

### Recommendation: Migrate to Qdrant

Gemini's strong recommendation is Qdrant — it directly solves all pain points (fragment bloat, disk exhaustion, compaction, concurrency). The local-first compromise (server process) is the main trade-off.

### Alternative: PostgreSQL + pgvector

If the server process is acceptable anyway, pgvector offers:
- Full SQL (we'd keep DataFusion-like querying)
- `JSONB` for nested payloads (actors struct, drrp_history) — more powerful than Qdrant's payload filtering
- Mature Rust clients (`sqlx`, `tokio-postgres`)
- Robust transactional guarantees
- No fragment bloat, efficient upsert via `INSERT ... ON CONFLICT`

### Other alternatives considered

| DB | Embedding search | Metadata filtering | Upsert perf | Rust client | Local-first |
|---|---|---|---|---|---|
| **Qdrant** | Excellent (HNSW) | Excellent (nested payloads) | Very high | Official crate | Server process |
| **PostgreSQL + pgvector** | Good (HNSW) | Excellent (SQL + JSONB) | Very high | Excellent (sqlx) | Server process |
| **ChromaDB** | Good | Limited (flat KV) | Decent | Community crate | Server process |
| **LanceDB (current)** | Good | Excellent (SQL/DataFusion) | Poor (25x bloat) | Official crate | Embedded |

### Can we fix LanceDB instead?

- **pylance** could help with incremental compaction but adds Python dependency and doesn't fix merge_insert write amplification
- Newer LanceDB versions may have better native compaction — worth checking
- Different write patterns (larger batches, append-only) could reduce bloat but not eliminate it
- **Verdict**: fixing LanceDB is patching a fundamental architectural mismatch for our update-heavy workload

### Critical context: micro-apps architecture (from .claude/plans/micro-apps.md)

Fractalaw is not just a parsing pipeline — it's building toward WASM micro-apps running on **hub AND edge**:
- **Hub**: full corpus, heavy AI models, batch processing — server-based DB is fine
- **Edge**: offline tablets/laptops with synced data slices, ONNX embeddings, semantic search — **needs embedded DB, no server**

The Field Research Tool (edge micro-app) searches a local LanceDB partition via `fractal:data/query`. This is the killer use case for LanceDB's embedded architecture.

**This means:**
- **Qdrant/pgvector can't replace LanceDB on edge** — both need a server
- **LanceDB's embedded nature is a feature for edge**, even though it's painful on hub
- Sertantai already uses PostgreSQL for LAT — pgvector would align the hub stack
- The answer may be **hybrid**: pgvector on hub (write-heavy, rich queries), LanceDB on edge (read-heavy, embedded, synced slices)

### Decision criteria updated

| Criterion | Qdrant | pgvector | LanceDB (current) | Hybrid (pgvector hub + LanceDB edge) |
|---|---|---|---|---|
| Hub write perf | Excellent | Excellent | Poor (fragment bloat) | Excellent (pgvector) |
| Edge embedded | No (server) | No (server) | Yes | Yes (LanceDB edge) |
| SQL queries | No | Yes | Yes (DataFusion) | Yes (both) |
| Sertantai alignment | No | Yes (already Postgres) | No | Yes |
| Migration complexity | High | High | None | Medium (hub only) |
| Micro-apps edge story | Broken | Broken | Works | Works |

## Key files

- `crates/fractalaw-store/src/lance.rs` — current LanceStore (the migration surface)
- `crates/fractalaw-core/src/schema.rs` — schema definition (59 columns)
- `scripts/compact_lance.py` — current compaction workaround
