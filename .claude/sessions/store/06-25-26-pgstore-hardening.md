# Session: PgStore Hardening & Remaining Wiring (PENDING)

## Context

PgStore is live — full pipeline (parse → embed → classify → validate) proven against 183K-row Postgres. 49 QQ corpus laws processed successfully. This session collects the remaining work to make PgStore the default hub store.

## Carried from PgStore Implementation session

### Remaining trait wiring

Commands that still open `LanceStore` internally (not reachable via `--pg`):
- `cmd_taxa_show`, `cmd_taxa_qa`, `cmd_taxa_eyeball`, `cmd_taxa_audit_fitness` — read-only diagnostic commands
- `misc.rs`: `cmd_text`, `cmd_embed`, `cmd_search`, `cmd_validate`, `cmd_export_training_data`
- `sync.rs`: `cmd_sync_publish_provisions`, `cmd_sync_pull_lat`, `cmd_sync_watch`

Fix: pass `&dyn ProvisionStore` from caller or thread `pg_url` through. Mechanical, same pattern as Phase 4.

### Postgres infrastructure

- ⬜ Enable quadlet on boot (`systemctl --user enable fractalaw-pg.service`)
- ⬜ Filtered query benchmarks (latency measurement vs LanceDB)
- ⬜ JSONB validation with `jsonb_pretty()` for actors/drrp_history
- ⬜ Batch upsert performance: UNNEST approach vs current row-by-row (Gemini recommended UNNEST)

## Carried from Law Status Tracker session (PENDING)

- ⬜ Build `law_pipeline_status` DuckDB table tracking per-law pipeline stage
- ⬜ `fractalaw taxa status` CLI command
- ⬜ Wire status updates into each pipeline stage (parse, embed, classify, validate, publish)
- See `store/06-24-26-law-status-tracker.md` for full design (Option D: DuckDB table + CLI command)

## Carried from QQ Corpus session

- ⬜ 20 QQ laws missing LAT from sertantai (parse failures on their side)
- ⬜ `UK_ukpga_2023_55` excluded (3,351 provisions) — needs confirmation before processing
- ⬜ ~4,191 corrections across 310+ audit logs awaiting `/human-review` adjudication

## New work

- ⬜ Default to `--pg` when `FRACTALAW_PG` env var is set (avoid typing URL every time)
- ⬜ Consider making PgStore the default (no flag needed) once all commands are wired
