---
session: PgStore Hardening & Remaining Wiring
status: suspended
opened: 2026-06-25
closed:
outcome:

summary: >
  Hardened PgStore after initial implementation. Wired --pg for provisions publish
  and pull-lat. Fixed query_provision_taxa filter, upsert_embeddings, Zenoh selector
  params. Added is_benchmark column. Law status tracker implemented. 224 QQ laws published.
  sync watch wiring and remaining trait coverage still pending.

decisions:
  - what: "query_provision_taxa filter: extraction_method IS NOT NULL (not drrp_types)"
    why: drrp_types can be empty array for gated provisions, but extraction_method is always set when a provision has been through the pipeline
    result: Fixes provisions publish showing 0 rows for laws with gated provisions
  - what: Zenoh status queryable uses selector params (?laws=) not body payload
    why: Zenoh GET queries have no body in practice — sertantai sends params
    result: Both selector params and body payload supported as fallback

metrics:
  qq_laws_published: 224

artifacts:
  - crates/fractalaw-cli/src/commands/sync.rs
  - crates/fractalaw-cli/src/commands/taxa.rs
  - crates/fractalaw-store/src/pg.rs

depends_on:
  - 06-24-26-pgstore-implementation.md
---

# Session: PgStore Hardening & Remaining Wiring (SUSPENDED)

## Context

PgStore is live — full pipeline (parse → embed → classify → validate → publish) proven against Postgres. 224+ QQ corpus laws published to sertantai.

## Completed (2026-06-25)

- ✅ `cmd_sync_publish_provisions` wired to `--pg`
- ✅ `cmd_sync_pull_lat` wired to `--pg`
- ✅ PgStore `query_provision_taxa` filter fixed: `drrp_types IS NOT NULL` → `extraction_method IS NOT NULL`
- ✅ PgStore `upsert_embeddings` fixed: INSERT ON CONFLICT → UPDATE WHERE
- ✅ Zenoh status queryable: selector params `?laws=` support (Zenoh GET has no body)
- ✅ `is_benchmark` column added to DuckDB (20 laws flagged)
- ✅ Benchmark restoration from Arrow backup after accidental re-validation
- ✅ Law status tracker: schema, CLI, backfill, Zenoh queryable, `--customer` flag
- ✅ 224 QQ laws published (enrichment + provisions)
- ✅ Build warnings cleaned (zero warnings)

## Remaining (not blocking, carry forward as needed)

### sync watch → PgStore wiring

`cmd_sync_watch` is 250 lines (440-689) with 3 `lance.` calls:
- `lance.delete_law_lat(law_name)` — LAT deletion handler
- `lance.upsert_lat(batches)` — LAT ingestion (the main one)
- `lance.delete_law_annotations(law_name)` — annotation cleanup

The function also:
- Opens `LanceStore` at line 445 (hardcoded)
- Runs the DuckDB status queryable (already works)
- Handles Zenoh events in a `tokio::select!` loop

#### Assessment

The wiring change itself is simple — replace `LanceStore::open(...)` with `open_provision_store(data_dir, pg_url)` and change `lance` type. But this function has broader quality issues:

1. **250 lines in one function** — the event handler loop mixes LAT ingestion, LRT upsert, ack, enrichment queue, status queryable, and deletion. Similar to the old `enrich_single_law` before decomposition.
2. **`delete_law_annotations` not on the trait** — `ProvisionStore` doesn't have this method. Need to add it or handle differently.
3. **Event handler logic is inline** — each event type (lat, lrt, lat_deleted) is handled inline in the select loop rather than dispatched to functions.

#### Plan

1. Check if `delete_law_annotations` exists on PgStore / needs adding to trait
2. Add `pg_url` parameter, swap `LanceStore::open` → `open_provision_store`
3. Optionally: decompose the event handler into smaller functions (same pattern as enrich_single_law decomposition)
4. Get Gemini review on sync.rs shape before/after

### Remaining trait wiring (lower priority)
- `cmd_taxa_show`, `cmd_taxa_qa`, `cmd_taxa_eyeball`, `cmd_taxa_audit_fitness` — read-only diagnostic commands
- `misc.rs`: `cmd_text`, `cmd_embed`, `cmd_search`, `cmd_validate`, `cmd_export_training_data`

### Postgres infrastructure (quality-of-life)
- ⬜ Enable quadlet on boot (`systemctl --user enable fractalaw-pg.service`)
- ⬜ Batch upsert performance: UNNEST approach vs current row-by-row
- ⬜ Default to `--pg` via `FRACTALAW_PG` env var

### QQ corpus outstanding
- ⬜ `UK_ukpga_2023_55` excluded (3,351 provisions) — needs confirmation
- ⬜ ~41 QQ laws still missing LAT from sertantai (parse failures on their side)
- ⬜ ~4,191 corrections across audit logs awaiting `/human-review` adjudication
