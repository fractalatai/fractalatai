---
session: QQ Corpus Controls Run
status: suspended
opened: 2026-07-11
---

# Session: QQ Corpus Controls Run (SUSPENDED)

## Problem

OH&S pilot validated the pipeline at 3.9:1 ratio with 1.1% flag rate. 380 of 428 QQ laws still need controls generated. Running family-by-family, largest first.

## Work

1. ✅ Batch complete — 220/428 QQ laws with controls (208 skipped, no provisions)
2. ✅ Corpus stats: 1,341 controls, 222 predicates, 92 flagged
3. ✅ Spot-checked 12 diverse non-OH&S laws (86 controls) — zero deontic leaks, zero paperwork referents
4. ✅ Final quality report below

## Final Report

| Metric | Value |
|--------|-------|
| Laws with controls | 220 / 428 QQ (208 have no governed provisions) |
| Controls | 1,341 |
| Policy predicates | 222 |
| Validated | 1,471 (94.1%) |
| Flagged | 92 (5.9%) |
| Controls per law | min 1, median 6, max 18, avg 6.1 |

### Flag breakdown

| Flag | Count | Notes |
|------|-------|-------|
| JUDGEMENT_MISSING | 73 | Soft — judgement term in text but not in field |
| INVALID_REF | 36 | Provision ref not in input set (filtered provisions) |
| PAPERWORK | 4 | Description references document existence |
| DEONTIC | 2 | "must" or "shall" in title |

### Spot-check (12 laws, 86 controls)

Families covered: Wildlife & Countryside, Pollution, Lead at Work, Working Time, Water Industry, Transport (operators), Rabbit control, Environmental Damage, Driver Hours, REACH Chemicals, Occupiers' Liability. Zero deontic leaks, zero paperwork referents. Load-bearing judgement well-flagged throughout.

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
