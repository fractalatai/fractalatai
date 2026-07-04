---
session: PgStore Hardening & Remaining Wiring
status: active
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

# Session: PgStore Hardening & Remaining Wiring (ACTIVE)

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

## Reframing: Two-Pass Architecture (2026-07-03)

The project has evolved from a single enrichment pipeline to a two-pass model:

**Pass 1 — Triage (fast, automated, event-driven)**
Regex scans provisions for actors + modals → determines if the law is "making" (sets obligations) vs non-making (amending, commencement, etc.) → publishes triage result back to sertantai. This is what `sync watch` should do. Light, reactive, near-real-time.

**Pass 2 — Deep parse (batch, manual, customer-scoped)**
SLM/LLM/human adjudication on a customer's specific law register. RunPod for SLM inference, 4-tier cascade (regex → classifier → SLM → LLM), QA, corrections. Triggered per-customer, not per-event. The `taxa parse → classify → reconcile → backfill → publish` pipeline.

**Why this matters for sync watch:**
The current `cmd_sync_watch` (now in `fractalaw-sync-cli`) was built during the LanceDB era when enrichment happened at the edge. It pulls LAT, queues for enrichment, and tries to run the full pipeline reactively. That made sense when LanceDB was the AI-at-the-edge store. With Postgres as the hub store, enrichment is a batch operation — standing up a pod, running SLM, QA. sync watch shouldn't trigger it.

**Sync watch should:**
1. Pull LAT from sertantai → store in Postgres
2. Ensure LRT exists in DuckDB
3. Run Pass 1 regex triage (quick — actors, modals, purpose classification)
4. Publish triage signal back to sertantai (is this a making law? what families?)
5. Ack receipt
6. Handle LAT deletion signals
7. **Stop.** No enrichment queue, no classifier, no SLM.

**What to remove from sync watch:**
- `enrichment_pending` queue logic
- `--pending` flag on publish
- Any LanceDB dependency (sync watch is Postgres + DuckDB only)
- The `enrich_single_law` call path

## Remaining work

### sync watch refactor
- ⬜ Strip enrichment queue logic from `cmd_sync_watch` (in `fractalaw-sync-cli`)
- ⬜ Wire to PgStore via `--pg` (swap `LanceStore::open` → `open_provision_store`)
- ⬜ Add Pass 1 regex triage: run `parse_v2()` on ingested provisions, publish making/non-making signal
- ⬜ Remove LanceDB dependency from sync-cli (it currently needs it for `upsert_lat`)
- ⬜ Decompose 250-line event handler into smaller functions

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
