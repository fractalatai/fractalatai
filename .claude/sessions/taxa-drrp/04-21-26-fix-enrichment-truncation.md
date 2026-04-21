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

## Next Steps

- [ ] Fix the limit in `enrich_single_law()`
- [ ] Add warning log for large laws
- [ ] Investigate Part blob filtering safety
- [ ] Re-enrich PUBLIC family
- [ ] Resume PUBLIC gap analysis session

---

**Session status**: Open. Ready to implement.
