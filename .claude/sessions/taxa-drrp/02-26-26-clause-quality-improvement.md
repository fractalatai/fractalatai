# Session: 2026-02-26 — Clause Quality Improvement

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
