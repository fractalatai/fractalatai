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

### Approach

Aggregate `applied_status` from `law_edges` where `edge_type` involves commencement:

```
fully_commenced    — all commencement edges have applied_status = 'Yes'
partially_commenced — mix of 'Yes' and 'Not yet'
not_commenced      — all edges have applied_status = 'Not yet'
no_commencement_data — no commencement edges exist (most SIs)
```

### New LRT columns

| Column | Arrow Type | Nullable | Description |
|--------|-----------|----------|-------------|
| `commencement_status` | Utf8 | yes | Enum: `fully_commenced`, `partially_commenced`, `not_commenced`, `no_commencement_data` |
| `commencement_date` | Date32 | yes | Date of full commencement (null if partial/not commenced) |

### Research needed

- Survey `law_edges` to count how many laws have commencement-related edges
- Determine whether `edge_type` values distinguish commencement from other relationships
- Check if `applied_status = 'Not yet'` reliably maps to "not commenced" vs "amendment not applied"

### Files to modify

| File | Change |
|------|--------|
| `crates/fractalaw-core/src/schema.rs` | Add `commencement_status` and `commencement_date` columns |
| `data/export_legislation.sql` | Derive status from edge aggregation |
| `docs/SCHEMA.md` | Document new columns |
| `crates/fractalaw-cli/src/display.rs` | Add to law card display |

## Status: Phase 1 complete, Phase 2 pending research
