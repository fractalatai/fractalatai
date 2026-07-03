---
session: APPLICATION_SCOPE Priority Bug
status: closed
opened: 2026-03-07
closed: 2026-03-07
outcome: success

summary: >
  Fixed bug where fitness extraction only ran when APPLICATION_SCOPE was the primary
  purpose. Changed purposes.first() to purposes.contains() in both early-return and
  DRRP paths. Polarity match 81.4% to 99.0%, 95 provisions recovered.

decisions:
  - what: "Use purposes.contains() instead of purposes.first() for fitness gate"
    why: Compound-purpose provisions like Interpretation+Application_Scope were skipped entirely
    result: "Polarity 81.4%→99.0%, Tagged 58.5%→79.6%, 95 provisions recovered"

metrics:
  polarity_before: 81.4
  polarity_after: 99.0
  tagged_before: 58.5
  tagged_after: 79.6
  provisions_recovered: 95
  tests_passing: 349

lessons:
  - title: "Per-provision vs per-rule counting matters for compound provisions"
    detail: polarity_matched was incremented per-rule, so compound provisions (split into 2 rules) counted twice, inflating Polarity% above 100%. Fixed to per-provision counting.
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/taxa/mod.rs
  - crates/fractalaw-cli/src/main.rs

depends_on:
  - 03-07-26-cross-reference-resolution.md
---

# Session: APPLICATION_SCOPE Priority Bug (#26) (CLOSED)

**Date**: 2026-03-07
**Issue**: [#26 — Fitness: APPLICATION_SCOPE as secondary purpose skips fitness extraction](https://github.com/fractalaw/fractalaw/issues/26)
**Discovered during**: #22 (cross-reference resolution) investigation

## Problem

`parse_v2()` only calls `fitness::extract()` when APPLICATION_SCOPE is `purposes.first()`. Two affected code paths:

1. **Early return** (line 108-121): `should_skip_drrp()` triggers for INTERPRETATION/ENACTMENT/APPLICATION_SCOPE-primary, but fitness only runs when `purposes.first() == APPLICATION_SCOPE`. Provisions like `["INTERPRETATION", "APPLICATION_SCOPE"]` get skipped entirely.
2. **DRRP path** (line 146-163): Provisions like `["PROCESS_RULE", "APPLICATION_SCOPE"]` go through DRRP extraction but `fitness_rules` was hardcoded to `vec![]`.

## Root Cause

Both code paths used `purposes.first()` instead of `purposes.contains()` to decide whether to run `fitness::extract()`.

## Fix

### `crates/fractalaw-core/src/taxa/mod.rs`

**Two changes**: both the early return (line 110) and the DRRP path (line 157) now use `purposes.contains(&APPLICATION_SCOPE)` instead of `purposes.first() == Some(&APPLICATION_SCOPE)`.

**New test**: `secondary_application_scope_gets_fitness_rules` — verifies compound provision "shall not apply to the master or crew of a ship" gets fitness_rules when APPLICATION_SCOPE is secondary.

### `crates/fractalaw-cli/src/main.rs`

**Bug fix**: `polarity_matched` was incremented per-rule (compound provisions counted twice → Polarity% >100%). Changed to per-provision counting.

## Results

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Polarity% | 81.4% | 99.0% | +17.6pp |
| Tagged% | 58.5% | 79.6% | +21.1pp |
| No-polarity | 99 | 4 | -95 |
| Gaps | 10 | 12 | +2 (newly detected) |
| Cross-Refs | 57 | 66 | +9 (newly detected) |

95 provisions that previously got zero fitness_rules now get proper polarity detection and p-dimension tagging.

## Key Files

- `crates/fractalaw-core/src/taxa/mod.rs` — `parse_v2()` lines 110 and 150 (`purposes.contains()`)
- `crates/fractalaw-cli/src/main.rs` — `polarity_matched` counting fix

## Status: **Complete** — 349 tests pass, audit verified
