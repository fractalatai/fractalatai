# Session: 2026-06-05 — Gap C Phase 1A: Deterministic Parent Inheritance

## Context

**Meta-plan**: `.claude/plans/gap-c-tiered-resolution.md`
**Design doc**: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` v0.4
**Commit**: `39076f6`

## What was built

Tier 1 of the Gap C resolution pipeline: deterministic parent-clause inheritance. When a provision has no DRRP from the regex pipeline but has a duty-bearing purpose, walk up the document hierarchy (deepest-first) to find the nearest ancestor with actors and inherit them.

### Changes

- `fractalaw-core/src/taxa/mod.rs`: `is_duty_bearing_purpose()` function + 6 tests
- `fractalaw-core/src/schema.rs`: 3 new LAT columns (extraction_method, holder_inferred_from, ancestor_distance)
- `fractalaw-store/src/lance.rs`: `ensure_gap_c_columns()` schema migration
- `fractalaw-store/src/duck.rs`: `ensure_inherited_count_column()`
- `fractalaw-cli/src/main.rs`: `--gap-c` flag, hierarchy_path/depth extraction, Tier 1 inheritance pass, new batch columns

### Key design

- In-memory parent lookup: no extra LanceDB queries, all provisions already in batches
- Deepest-first walk: prevents override trap (intermediate child redefines actor)
- Only fills gaps: if regex found any actor, don't override
- `holder_inferred_from` stored as Utf8 (comma-joined) due to LanceDB SQL type limitation on List columns

## Results

### HSWA (single-law test)

98 provisions inherited, DRRP 32.9% → 35.8%. All at ancestor_distance=1.

### Customer corpus (299 laws, Milestone 1)

| Metric | Value |
|--------|-------|
| Regex-extracted | 63,260 |
| Tier 1 inherited | 8,648 |
| Total with DRRP | 71,908 |
| Tier 1 uplift | +13.7% |
| Laws with inheritance | 141 of 274 |

Ancestor distance: 74.1% at distance 1 (immediate parent). Confirms ChatGPT's prediction that nearest-parent resolves the vast majority.

## What's next

Phase 1B: Cross-reference resolver (Tier 2) — separate session.
