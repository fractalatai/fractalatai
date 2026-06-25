# Session: PgStore Hardening & Remaining Wiring (CLOSED)

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

### Trait wiring (read-only diagnostic commands)
- `cmd_taxa_show`, `cmd_taxa_qa`, `cmd_taxa_eyeball`, `cmd_taxa_audit_fitness`
- `misc.rs`: `cmd_text`, `cmd_embed`, `cmd_search`, `cmd_validate`, `cmd_export_training_data`
- `cmd_sync_watch` (still opens LanceStore internally for enrichment queue)

### Postgres infrastructure (quality-of-life)
- ⬜ Enable quadlet on boot (`systemctl --user enable fractalaw-pg.service`)
- ⬜ Batch upsert performance: UNNEST approach vs current row-by-row
- ⬜ Default to `--pg` via `FRACTALAW_PG` env var

### QQ corpus outstanding
- ⬜ `UK_ukpga_2023_55` excluded (3,351 provisions) — needs confirmation
- ⬜ ~41 QQ laws still missing LAT from sertantai (parse failures on their side)
- ⬜ ~4,191 corrections across audit logs awaiting `/human-review` adjudication
