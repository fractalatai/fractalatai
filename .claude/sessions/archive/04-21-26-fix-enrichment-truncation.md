---
session: Fix Enrichment Truncation (#33)
status: closed
opened: 2026-04-21
closed: 2026-04-24
outcome: success
summary: 'Fixed hardcoded limit=500 in LanceDB queries that silently truncated 80 laws (52,846 provisions). Raised all four
  affected call sites to 100,000 and added warning logs for laws with >2,000 provisions. PUBLIC family re-enriched with corrected
  data. Commit d72a702.

  '
decisions:
- what: Raise hardcoded limit from 500 to 100,000
  why: Largest law has ~4,200 provisions; 100,000 is safe and future-proof
  result: All four affected call sites fixed; three existing 200,000-limit sites were already fine
- what: Add tracing::warn for laws with >2,000 provisions
  why: Large laws should be visible in logs without manual checking
  result: Warning log added to enrich_single_law
- what: Defer Part blob filtering
  why: Safety of skipping Part/Chapter/Schedule rows depends on sertantai re-sync
  result: Deferred pending sertantai-legal#69 resolution
lessons:
- title: Stale assumptions become silent bugs
  detail: The limit=500 was pragmatic when the corpus was small OH&S laws; it was never revisited as laws like the Online
    Safety Act (4,181 provisions) entered
  tag: data-quality
- title: Always check for other instances of a hardcoded value
  detail: The initial fix targeted one call site but grep found three more with similar limits
  tag: process
metrics:
  laws_affected: 80
  provisions_affected: 52846
  call_sites_fixed: 4
  largest_law_provisions: ~4,200
artifacts:
- crates/fractalaw-cli/src/main.rs
depends_on: []
enables:
- taxa-gap-analysis/04-21-26-public-safety (resumed with corrected data)
---

# Session: Fix Enrichment Truncation (#33) (CLOSED)

## Context

**Issue**: fractalaw/fractalaw#33
**Related**: shotleybuilder/sertantai-legal#69 (Part blob duplication — separate fix)
**Blocked by this**: `.claude/sessions/taxa-drrp/taxa-gap-analysis/04-21-26-public-safety.md` (PUBLIC family gap analysis, suspended)

## Problem

`enrich_single_law()` in `crates/fractalaw-cli/src/main.rs` queries LanceDB with `limit=500`:

```rust
let batches = lance.query_legislation_text(&filter, 500, 0).await?;
```

Any law with >500 provisions is silently truncated. **80 laws, 52,846 provisions** affected corpus-wide.

## Fix Plan

### 1. Raise the limit

Replace the hardcoded 500 with a generous ceiling. The largest law in LanceDB is ~4,200 provisions. A limit of 100,000 is safe and future-proof.

```rust
let batches = lance.query_legislation_text(&filter, 100_000, 0).await?;
```

**Location**: `crates/fractalaw-cli/src/main.rs` line ~2859

### 2. Add a warning log

When provision count is high, log a warning so large laws are visible:

```rust
let row_count: usize = batches.iter().map(|b| b.num_rows()).sum();
if row_count > 2000 {
    tracing::warn!("{law_name}: {row_count} provisions — large law");
}
```

### 3. Filter Part/Chapter blobs (optional, pending sertantai-legal#69)

Skip Part/Chapter/Schedule-level rows during enrichment to avoid processing duplicate text. The enricher already skips `section_type == "heading"` — extend this to structural blobs:

```rust
if section_type == "heading"
    || (section_type == "part" || section_type == "chapter" || section_type == "schedule")
{
    continue;
}
```

**Caution**: Only do this if ALL laws have section-level provisions beneath their Part blobs. Some older laws might only have Part-level text. Verify before implementing.

### 4. Re-enrich affected families

After the fix:
```bash
cargo run -p fractalaw-cli -- taxa enrich --force
# Or per-family:
cargo run -p fractalaw-cli -- taxa enrich --family "PUBLIC" --force
```

### 5. Check other limit=500 sites

Search for other hardcoded limits in the enrichment/show/qa paths that might also truncate.

## Verification

After fix + re-enrichment:
1. Run LAT QA skill — Check 1 should find 0 truncated laws
2. Re-run PUBLIC family gap analysis (resume suspended session)
3. Compare confusion matrix before/after

## Investigation: Why Did the Limit Exist?

The `limit=500` was introduced in commit `a0af20e` ("Rework taxa enrichment to use existing LRT DRRP columns", 2026-02-24). It was a pragmatic default — at that time the corpus was mostly OH&S laws with <500 provisions per law. The limit was never revisited as larger laws (Online Safety Act at 4,181 provisions) entered the corpus. Not a deliberate safety mechanism, just a stale assumption.

## Implementation (2026-04-24)

### All affected sites found

| Line | Function | Old Limit | Fixed |
|------|----------|-----------|-------|
| ~2067 | `cmd_taxa_qa()` | 500 | 100,000 |
| ~2491 | `cmd_taxa_audit_fitness()` | 500 | 100,000 |
| ~2859 | `enrich_single_law()` | 500 | 100,000 + warning log |
| ~3733 | `cmd_export_training_data()` | 1,000 | 100,000 |

Three other call sites already used 200,000 (`cmd_taxa_qa` corpus-wide queries) — those were fine.

Added `tracing::warn!` in `enrich_single_law()` for laws with >2,000 provisions.

### Sertantai#69 status

OSA LanceDB data unchanged — Part blobs still present. The sertantai fix has been applied to the parser but affected laws haven't been re-synced to local LanceDB yet. The enrichment fix is independent.

## Next Steps

- [x] Fix the limit in `enrich_single_law()` and all other call sites
- [x] Add warning log for large laws
- [x] Re-enrich PUBLIC family after commit
- [x] Resume PUBLIC gap analysis session (reanalysis complete)
- [ ] Investigate Part blob filtering safety (deferred — depends on sertantai re-sync)

---

**Session closed**: 2026-04-24. Commit d72a702. PUBLIC gap analysis resumed with corrected data.
