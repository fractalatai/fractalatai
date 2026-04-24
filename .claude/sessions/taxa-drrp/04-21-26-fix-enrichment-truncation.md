# Session: 2026-04-21 — Fix Enrichment Truncation (#33)

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
