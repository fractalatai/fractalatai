---
session: Extent Re-parse Sync
status: closed
opened: 2026-09-29
closed: 2026-09-29
outcome: success

summary: >
  Applied legal's 42-parent-Act extent re-parse to the hub. The dry run exposed a #62 bug: legal's rename-log timestamps are
  naive and failed RFC 3339 parsing, so every rename had been skipped, including in the first sync. Fixed; unparseable
  timestamps now fail loudly. Pilot + batch: 28 laws, 1,309 renames from legal's log, 25,410 actor rows carried, 0 held,
  0 gate failures. First live sync watch: the startup manifest pass found the hub fully in sync.

decisions:
  - what: Fail the law, rather than skip, when a rename timestamp won't parse
    why: A silently dropped rename archives tier data that should have been carried
    result: parse_legal_timestamp (RFC 3339 or naive UTC) + error
  - what: Recover the first sync's lost renames from lat_archive, not by recomputing
    why: Cheaper than re-running SLM work; legal's rename log maps old ids to new ids
    result: Added as the first step of the re-enrichment backlog session

metrics:
  sync: { laws: 28, legal_renames: 1309, actors_carried: 25410, text_changed: 85, archived: 40, held: 0, gate_failures: 0 }
  watch_live: { minutes: 15, in_sync: 691, applied: 0, events: 0, errors: 0 }

lessons:
  - title: A dry-run total of zero can be a bug, not a quiet day
    detail: renamed_map was 0 across 731 laws in the first sync and went unnoticed. Legal had logged thousands of renames. Check that expected inputs are actually consumed.
    tag: methodology

artifacts:
  - crates/fractalaw-sync-cli/src/lat_sync.rs
  - data/lat-sync/extent/apply.log
  - data/lat-sync/extent/watch_live.log

depends_on:
  - 09-26-26-robust-lat-sync.md
enables:
  - 09-29-26-reenrichment-backlog.md
---

# Session: Extent Re-parse Sync (CLOSED)

## Problem

Legal re-parsed the LAT of 42 parent Acts to fix provision extents: a provision without its own RestrictExtent had inherited the document root's extent instead of its nearest ancestor's. The hub now differs from legal's manifest:
- struct_hash changes on all 42 (extent_code);
- 1,462 section_id renames across 16 Acts on `lat-renames` (Environment Act 1995: 396, EPA 1990: 272, UK_ukpga_1968_73: 156, …);
- 25 rows with changed or dropped text in 4 laws.

This is the first large rename batch diff-apply will consume for real, and the first live `sync watch` run.

## Todo

- ✅ Backup: NAS `fractalaw-backups/20260929/` (pgdump 604M, 12 tables; DuckDB 443M, 19,493 laws), sha256 OK
- ✅ Dry run: **found a #62 bug.** Rename log timestamps are naive, the RFC 3339 parse failed, and every rename was skipped, including in the first sync. Fixed (commit below). Re-run: 48 planned (20 benchmarks report-only); 28 non-benchmark with 1,309 map renames, 0 held
- ✅ Pilot: Environment Act 1995 (390 map renames, 260/260 actors carried) + UK_anaw_2017_2 (808/808); both in_sync after
- ✅ Applied the other 26: 919 map renames, 24,342 actor rows carried, 84 text_changed, 40 archived, 0 held, 0 gate failures
- ✅ Handed 6 reparse_needed laws to the re-enrichment backlog (Food Safety Act 1990, UK_ukpga_1988_52, UK_ukpga_1993_11, UK_anaw_2017_2, CoP(A)A 1989, UK_uksi_2000_3184); rename recovery added there too
- ✅ First live `sync watch` (15 min, legal notified). Startup manifest pass: 691 in sync, 0 to apply, 20 benchmarks report-only, 4 delete candidates reported only; no events arrived; 0 errors. The per-event path is unobserved live (same `sync_law` code as `pull-lat`)
- ✅ Reported to legal; closed

## Dependencies

- ✅ #62 diff-apply + rename log consumption (`pull-lat`, `lat-sync` skill)
- ✅ Legal: extent re-parse applied; snapshot `lat_reparse_enabling_parents_b0`; report `backend/data/reports/lat-reparse/enabling_parents_b0.csv`
