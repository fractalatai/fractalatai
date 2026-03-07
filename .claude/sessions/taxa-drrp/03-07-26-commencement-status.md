# Session: Denormalize Commencement Status onto LRT (#8)

**Date**: 2026-03-07
**Issue**: [#8 — Denormalize commencement status onto LRT hot path](https://github.com/fractalaw/fractalaw/issues/8)
**Depends on**: LAT schema cleanup (resolved 2026-02-19, session `02-19-26-LAT-schema.md`)

## Problem

The LRT schema has 4 annotation total columns (`total_text_amendments`, `total_modifications`, `total_commencements`, `total_extents`) that are always NULL. The data exists in `annotation_totals.parquet` (135 laws, 73 with commencement data, 3,999 total I-code commencement annotations) but is never merged into `legislation.parquet`.

The Feb 19 comment on #8 noted a blocker on LAT schema cleanup (`section_id` collision, annotation ID uniqueness). That was resolved the same day — citation-based identity eliminated all duplicates. No blocking dependencies remain.

## Phase 1: Backfill Annotation Totals

Merge `annotation_totals.parquet` into the LRT so all 4 columns are populated.

### Changes

Used Option A (SQL-level merge) — fix at source.

#### `data/export_legislation.sql`

- Replaced 4 hardcoded `NULL::INTEGER` columns with `ann.total_text_amendments`, `ann.total_modifications`, `ann.total_commencements`, `ann.total_extents`
- Added `LEFT JOIN read_parquet('data/annotation_totals.parquet') ann ON ann.name = r.name`
- Added annotation totals verification query to section 5
- Note: initial alias `at` caused parser error — `AT` is a SQL reserved keyword. Changed to `ann`.

#### `data/legislation.parquet`

Re-exported with populated annotation totals.

### Results

| Metric | Before | After |
|--------|--------|-------|
| Laws with annotation data | 0 | 128 |
| Total commencements | 0 | 3,996 |
| Max commencements (single law) | — | 498 (UK_ukpga_2009_23) |

Verified end-to-end:
- `duckdb` direct query: 128 laws with non-NULL annotation totals
- `fractalaw import`: reimported into persistent DuckDB
- `fractalaw law UK_ukpga_2007_19`: shows `total_commencements: 31` in Annotation Totals section

## Phase 2: Semantic Commencement Status

Derive a high-level `commencement_status` enum from edge-level `applied_status` data. This tells users at a glance whether a law is in force.

### Research findings

- `affect_type ILIKE 'coming into force'` is the main commencement signal (234K edges)
- `affect_type ILIKE 'Commencement Order'` adds 2,350 more
- `applied_status` values: `Yes`/`Y` (applied), `Not yet` (pending), `Not yet made to Welsh...` (partial), `See note` (ambiguous)
- Edge types are not commencement-specific; `affect_type` is the discriminant
- 2,267 laws in LRT have commencement edges as targets

### Changes

#### `data/export_legislation.sql`

Added `commencement_status` table (section 2b) derived from `law_edges.parquet`:
- Filter edges where `affect_type ILIKE 'coming into force'` or `'Commencement Order'`
- Per-law: count applied (`Yes`/`Y`) vs not-yet (`Not yet%`)
- Classify: `fully_commenced` (no not-yet), `not_commenced` (no applied), `partially_commenced` (mix)
- LEFT JOIN onto legislation export as `cs.commencement_status`
- Laws without commencement edges get NULL (no_commencement_data implied)

#### `crates/fractalaw-core/src/schema.rs`

- Added `commencement_status` (Utf8, nullable) field after section 1.9
- Updated doc comment: 78 -> 79 columns
- Updated field count test: 98 -> 99

#### `crates/fractalaw-store/src/duck.rs`

- Updated 2 column count assertions: 78 -> 79

#### `crates/fractalaw-cli/src/display.rs`

- Added `commencement_status` to STATUS section (displays alongside `status`)

### Results

| commencement_status | Laws |
|---------------------|------|
| fully_commenced | 1,855 |
| not_commenced | 329 |
| partially_commenced | 83 |
| NULL (no data) | 17,051 |

Verified:
- `fractalaw law UK_ukpga_2007_19`: shows `commencement_status: fully_commenced`
- Partially commenced laws correctly identified (e.g., UK_ukpga_2025_36)
- 361 tests pass

### Deferred

- `commencement_date` column: requires extracting dates from edges, lower priority
- `docs/SCHEMA.md` update: can be done in a follow-up

## Status: Phase 1 + Phase 2 complete
