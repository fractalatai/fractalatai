---
session: QQ Corpus Controls Run
status: suspended
opened: 2026-07-11
---

# Session: QQ Corpus Controls Run (SUSPENDED)

## Problem

OH&S pilot validated the pipeline at 3.9:1 ratio with 1.1% flag rate. 380 of 428 QQ laws still need controls generated. Running family-by-family, largest first.

## Work

1. ⏸️ Batch by family — 160/428 QQ laws done (37%), 972 controls, 160 predicates. Resume with `--all --qq`.
2. ⬜ Corpus stats after completion
3. ⬜ Spot-check non-OH&S families for quality
4. ⬜ Final quality report

## Progress

| Family | Laws | Generated | Controls | Ratio | Flags |
|--------|:---:|:---:|:---:|:---:|:---:|
| OH&S | 47 | 35 | 263 | 3.9:1 | 14 |
| ENV PROTECTION | 37 | 20 | 74 | 2.5:1 | 5 |
| WASTE | 37 | 7 | 49 | 3.1:1 | 5 |
| WATER & WASTEWATER | 24 | 8 | 34 | 2.1:1 | 3 |
| TOWN & COUNTRY PLANNING | 23 | 7 | 31 | 2.6:1 | 1 |
| CLIMATE CHANGE | 20 | 10 | 23 | 1.7:1 | 1 |
| Remaining (~50 laws) | — | ~73 | ~498 | — | — |
| **Total in staging** | **160** | — | **972** | — | — |

## Dependencies

- ✅ Phase 2 pilot validated
- ✅ Pipeline script with --qq, skip-if-exists
- ✅ 48 QQ laws already in staging table
