# Session: Prune LAT for Non-Making Laws

**Date**: 2026-04-14

## Problem

Laws that produce no duties or responsibilities (non-making laws) still have their full provision text (LAT) stored in LanceDB after enrichment. Neither fractalaw nor sertantai should store legal text for laws that don't create at least one duty or responsibility — the record volume is too high otherwise.

**Clarification**: The DuckDB (LRT) metadata is always kept and always published to sertantai — it contains useful metadata regardless. The issue is strictly about LAT rows (per-provision text) in LanceDB accumulating for non-making laws.

Previous sessions (02-26-26-taxa-refinement, 02-27-26-application-scope-tightening) implemented **provision-level skip gates** (`should_skip_drrp()`) which correctly prevent false DRRP parsing. But LAT rows were never pruned after enrichment determined a law was non-making.

## Root Cause

After `enrich_single_law()` finishes, the LAT rows that were pulled into LanceDB remain regardless of outcome. There was no step to delete them when the law turns out to have zero duties and zero responsibilities.

## Changes

### `enrich_single_law` return type (main.rs)

Changed from `Result<bool>` to `Result<EnrichResult>` with a three-variant enum:

```rust
enum EnrichResult {
    Making,    // has duties or responsibilities — keep LAT
    NonMaking, // has other taxa (rights, powers, fitness) but no duties/responsibilities
    NoTaxa,    // no taxa signal at all
}
```

Making check: `!taxa.duties.is_empty() || !taxa.responsibilities.is_empty()`

### `cmd_sync_watch` — Step 3b: prune LAT (main.rs)

After enrichment, if `NonMaking` or `NoTaxa`, calls `lance.delete_law_lat(law_name)` to remove the provision text from LanceDB. The DuckDB LRT row and publish to sertantai are unaffected.

### `cmd_taxa_enrich` — bulk prune (main.rs)

Same logic in the bulk enrichment loop. Reports total pruned laws/rows at the end.

## What's NOT changed

- **DuckDB (LRT)** — all metadata always kept, always published
- **Publish pipeline** — `sync publish` modes are untouched; they publish LRT from DuckDB
- **Provision-level skip gates** — `should_skip_drrp()` unchanged
- **`delete_law_lat()`** — already existed in lance.rs, just newly called from enrichment paths

## Validation

Tested against OH&S family — 5 enriched laws with zero duties and zero responsibilities.

### Test 1: Single law (`UK_nisi_1987_1280`)

```
$ cargo run -p fractalaw-cli -- taxa enrich --force --laws UK_nisi_1987_1280
Processed 1 laws. LRT now has 3342 laws with DRRP taxa data.
Pruned 23 LAT rows from 1 non-making laws.
```

### Test 2: Remaining 4 non-making laws

```
$ cargo run -p fractalaw-cli -- taxa enrich --force --laws UK_nisr_2020_330,UK_uksi_2013_240,UK_uksi_2025_1331,UK_uksi_2026_15
Processed 4 laws. LRT now has 3342 laws with DRRP taxa data.
Pruned 306 LAT rows from 4 non-making laws.
```

### DuckDB metadata verified intact post-prune

```
+-------------------+--------+--------+--------+----------+
| name              | dtypes | rights | powers | enriched |
+-------------------+--------+--------+--------+----------+
| UK_nisi_1987_1280 | 1      |        | 1      | true     |
| UK_nisr_2020_330  |        |        |        | true     |
| UK_uksi_2013_240  |        |        |        | true     |
| UK_uksi_2025_1331 |        |        |        | true     |
| UK_uksi_2026_15   | 1      | 1      |        | true     |
+-------------------+--------+--------+--------+----------+
```

LRT rows retained with taxa_hash set, duty_types/rights/powers preserved. 329 LAT rows pruned total.

## Next Steps

- [x] Run `taxa enrich --force` on a test family to validate pruning counts
- [x] Verify DuckDB LRT metadata intact for pruned non-making laws
- [ ] Verify sertantai still receives full LRT metadata for non-making laws (needs live publish test)
- [ ] Bulk prune non-making LAT across all families ([#32](https://github.com/fractalaw/fractalaw/issues/32))

## Status: CLOSED
