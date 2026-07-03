---
session: Taxa QA Report (#15)
status: closed
opened: 2026-03-05
closed: 2026-03-05
outcome: success
summary: 'Built fractalaw taxa qa CLI command producing 4-section validation reports: Coverage Summary, Purpose Distribution
  with anomaly flags, Gate Analysis (skip_drrp sub-gates and descriptive_summary), and Anomaly Detection. Supports --laws
  and --family filters. Replaces ad-hoc Python/LanceDB validation that previously caught 3 real bugs.

  '
decisions:
- what: 4-section report structure with anomaly thresholds
  why: Ad-hoc validation during taxa refinement caught real bugs (Enactment FPs, Enforcement FPs, enrichment skip bug) but
    was not repeatable
  result: Anomaly thresholds at Enactment >10%, Enforcement >15%, 0 DRRP with >10 provisions, any purpose >2x corpus average
- what: Live re-run of parse_v2() per provision rather than reading stored taxa
  why: QA should validate current pipeline behaviour, not cached results
  result: Report reflects live classification including any recent regex changes
lessons:
- title: Ad-hoc validation should become first-class CLI features
  detail: Three bugs found via manual Python/LanceDB queries proved the value of repeatable QA. Automating it as taxa qa makes
    validation sustainable
  tag: tooling
metrics:
  report_sections: 4
  tests_passing: 337
  single_law_test_provisions: 93
  family_test_provisions: 9608
artifacts:
- crates/fractalaw-cli/src/main.rs
- crates/fractalaw-core/src/taxa/mod.rs
- crates/fractalaw-core/src/taxa/purpose.rs
depends_on: []
enables: []
---


# Session: Taxa QA Report (#15) (CLOSED)

**Date**: 2026-03-05
**Issue**: [#15 — Taxa QA report: purpose classification quality assurance command](https://github.com/fractalaw/fractalaw/issues/15)
**Objective**: Build `fractalaw taxa qa` CLI command that produces validation reports for purpose classification, DRRP coverage, and anomaly detection.
**Priority context**: See [priority-reviews.md](../../plans/priority-reviews.md) — rose to #1 after #17 gap investigation proved ad-hoc validation catches real bugs.

## Problem

During taxa refinement we repeatedly ran ad-hoc Python/LanceDB queries to validate purpose regex precision. These caught 3 real bugs (Enactment false positives, Enforcement false positives, enrichment skip bug). This QA process should be a first-class CLI feature.

## Key Files

- `crates/fractalaw-cli/src/main.rs` — CLI entry point, `cmd_taxa_qa()` (~250 lines)
- `crates/fractalaw-core/src/taxa/mod.rs` — `parse_v2()`, `should_skip_drrp()`, `is_descriptive_summary()` (made pub)
- `crates/fractalaw-core/src/taxa/purpose.rs` — `classify()` for live re-classification
- LanceDB `legislation_text` — per-provision taxa data (read-only)
- DuckDB `legislation` — law-level metadata for family filtering

## Implemented CLI

```bash
fractalaw taxa qa                                    # all enriched laws
fractalaw taxa qa --laws UK_uksi_1999_3242,...        # specific laws
fractalaw taxa qa --family "OH&S: Occupational"      # by classified family
```

## Report Sections (4 implemented)

1. **Coverage Summary** — per-law: provisions, Purpose%, DRRP%, Gated%, plus corpus totals
2. **Purpose Distribution** — 15-column table with per-law rates, anomaly flags at >2x corpus average
3. **Gate Analysis** — skip_drrp sub-gates (Interpretation-primary, Enactment-primary, Application+Scope, all-structural) + descriptive_summary, with trigger counts and percentages
4. **Anomaly Detection** — Enactment >10%, Enforcement >15%, 0 DRRP with >10 provisions, any purpose >2x corpus average

## Changes Made

### `crates/fractalaw-core/src/taxa/mod.rs`
- Made `should_skip_drrp()` and `is_descriptive_summary()` `pub` so QA command can report which gate fired per provision

### `crates/fractalaw-cli/src/main.rs`
- Added `Qa { laws, family }` variant to `TaxaAction` enum
- Implemented `cmd_taxa_qa()` with structs `ProvisionStats` and `LawStats`
- Processing: resolves laws (--laws, --family, or all LanceDB), runs `parse_v2()` live per provision, collects stats, prints 4 report sections
- Anomaly thresholds: Enactment >10%, Enforcement >15%, 0 DRRP with >10 provisions, any purpose >2x corpus average

## Test Results

- Single law: `--laws UK_uksi_1999_3242` → 93 provisions, all 4 sections render correctly
- Family filter: `--family "OH&S: Occupational / Personal Safety"` → 342 laws, 9,608 provisions
- All existing tests pass (337 total)

## Deferred to v2

- `--json` output mode (report structure is well-defined, easy to add later)
- Cross-purpose overlap matrix (issue #15 section 4)

## Progress

- [x] Add `Taxa::Qa` CLI variant with --laws, --family flags
- [x] Implement coverage summary report (provisions, purposes, DRRP per law)
- [x] Implement purpose distribution report with anomaly flags
- [x] Implement purpose gate analysis
- [x] Implement anomaly detection (missing labels, suspicious combos)
- [x] Terminal table output
- [ ] Optional --json output mode (deferred to v2)
- [ ] Cross-purpose overlap matrix (deferred to v2)

## Status: **Done**
