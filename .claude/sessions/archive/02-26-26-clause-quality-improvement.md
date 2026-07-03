---
session: Clause Quality Improvement
status: closed
opened: 2026-02-26
closed: 2026-02-26
outcome: success
summary: 'Improved regex clause_refined quality from 75.9% to 100% clean starts and 97.8% to 99.6% clean ends across 4 iterations
  on 7 safety laws. Replaced artificial window constants with full-text sentence scanning, added purpose-based filters for
  enactment/interpretation/descriptive text, and fixed epistemic "may" false positives.

  '
decisions:
- what: Remove all artificial window constants (SUBJECT_WINDOW, ACTION_WINDOW, MAX_CLAUSE_LEN)
  why: Fixed-size windows caused mid-sentence truncation; full-text sentence scan finds natural boundaries
  result: Clean start rate jumped from 75.9% to 98.7% in v2
- what: Run governed v2 (actor-anchored) before government v1/v2 (unanchored)
  why: Unanchored government patterns were misclassifying governed Responsibility as Duty
  result: Correct duty family assignment for actor-bearing provisions
- what: Filter epistemic "may" (may be/need/have/require)
  why: These mean "might", not "is permitted to" -- false Enabling classifications
  result: Eliminated spurious Enabling/Power detections
- what: Add is_descriptive_summary() filter
  why: Meta-regulatory text like "The Regulations impose duties on employers" describes duties without creating them
  result: v3 reached 100% clean starts
lessons:
- title: Sentence boundary detection beats fixed windows for clause extraction
  detail: Semicolons followed by sub-paragraph markers (a), (b) are list separators, not sentence ends
  tag: regex
- title: Application+Scope purpose is too broad for DRRP gating
  detail: 'Many false positive DRRP provisions have Application+Scope as primary purpose; logged as GH #20'
  tag: data-quality
metrics:
  iterations: 4
  clean_start_v1: 75.9%
  clean_start_v4: 100%
  clean_end_v1: 97.8%
  clean_end_v4: 99.6%
  drrp_count_v1: 324
  drrp_count_v4: 272
  tests_passing: 204
artifacts:
- crates/fractalaw-core/src/taxa/mod.rs
- crates/fractalaw-core/src/taxa/clause_refiner.rs
- crates/fractalaw-core/src/taxa/purpose.rs
- crates/fractalaw-core/src/taxa/duty_patterns_v2.rs
- crates/fractalaw-core/src/taxa/duty_type.rs
- crates/fractalaw-core/src/taxa/text_cleaner.rs
- crates/fractalaw-core/src/taxa/confidence.rs
depends_on:
- 02-26-26-phase-c-lancedb-polisher
enables:
- 02-26-26-v2-promotion-enrichment
---


# Session: 2026-02-26 — Clause Quality Improvement (CLOSED)

**Parent session**: [02-26-26-phase-c-lancedb-polisher.md](02-26-26-phase-c-lancedb-polisher.md)
**Status**: Complete

## Objective

Improve regex `clause_refined` quality for DRRP provisions. Baseline on 7 safety laws showed 75.9% clean starts and 97.8% clean ends.

## Results

| Version | DRRP | Clean Start | Clean End | Key Fixes |
|---------|------|-------------|-----------|-----------|
| v1 | 324 | 75.9% | 97.8% | baseline |
| v2 | 300 | 98.7% | 99.3% | full-text sentence scan, enactment filter, interp-primary, numbering strip |
| v3 | 276 | 100% | 99.6% | descriptive summary filter, editorial footnote+bare number |
| v4 | 272 | 100% | 99.6% | epistemic "may" filter, subordinate clause rejection |

204 taxa tests passing. Committed as `9d17422`.

## Changes Made

### Clause extraction (`mod.rs`, `clause_refiner.rs`)
- Removed all artificial window constants (`SUBJECT_WINDOW`, `ACTION_WINDOW`, `MAX_CLAUSE_LEN`)
- `extract_clause()` scans full text before actor for sentence start, full text after modal for sentence end
- `find_first_sentence_end()` skips semicolons followed by sub-paragraph markers `(a)`, `; and (b)` — these are list separators
- Removed dead code: `truncate_smart`, `ensure_clean_ending`, `find_sentence_start_progressive`

### Purpose-based filtering (`mod.rs`, `purpose.rs`)
- Expanded ENACTMENT pattern: `Statutory Instruments`, `Signed by authority of`
- Added interpretation-primary skip: if Interpretation is first purpose, skip DRRP (modal verbs inside definitions aren't duties)
- Added `is_descriptive_summary()`: filters "The Regulations impose duties on employers..." meta-regulatory text

### Pattern accuracy (`duty_patterns_v2.rs`, `duty_type.rs`)
- Reordered: governed v2 (actor-anchored) runs before government v1/v2 (unanchored) — fixes Responsibility→Duty misclassification
- Epistemic "may" filter: rejects `may be/need/have/require` as Enabling (these mean "might", not "is permitted to")
- Subordinate clause rejection: actor in `Where {actor} ..., {subject} shall` is in conditional clause, not main subject

### Text cleaning (`text_cleaner.rs`)
- Expanded `LEADING_NUMBERING` regex for UK legislation variants: `-(1)`, `- [F3 (1)`, `9.-(1)`, `5. - [F3 (1)`, `[F18 5`

### Confidence scoring (`confidence.rs`)
- Changed clean-ending signal from `!ends_with("...")` to `ends_with(['.', ';'])`
- Added doc comment: max is 0.85 (0.15 reserved for AI refinement)

## Known Issues

- **Application+Scope purpose too broad** — logged as [#20](https://github.com/fractalaw/fractalaw/issues/20). Many false positive DRRP provisions have Application+Scope as primary purpose. The regex needs tightening before it can be used as a DRRP gate.
- **COMAH schedule table** — tabular hazardous substances data produces a spurious Power match. Data quality issue, not a regex bug.

## Next Steps

- Tighten Application+Scope purpose classifier (#20) — the single biggest remaining improvement
- Re-enrich all 452 laws with improved pipeline
- Phase C: store taxa DRRP map in LanceDB alongside text
