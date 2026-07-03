---
session: Purpose Classifier Investigation
status: closed
opened: 2026-03-07
closed: 2026-03-07
outcome: success

summary: >
  Investigated whether purpose classifier was OH&S-biased (it was not). CLIMATE CHANGE
  audit proved classifier works universally (150 provisions, 90% polarity). Zero results
  for FOOD/Maritime caused by missing LAT data, not classifier bias. #24 closed as not planned.

decisions:
  - what: "#24 closed as not planned — no code changes needed"
    why: Purpose classifier APPLICATION_SCOPE regex is universal (no family-specific logic). Missing results caused by missing LAT data.
    result: Real blocker for non-OH&S fitness expansion is LAT population from sertantai

metrics:
  climate_change_provisions: 150
  climate_change_polarity: 90.0
  climate_change_tagged: 20.0

lessons:
  - title: Always verify the root cause before assuming classifier bias
    detail: Zero results for FOOD/Maritime looked like classifier bias but was actually missing input data. CLIMATE CHANGE disproved the hypothesis immediately.
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/taxa/purpose.rs

depends_on:
  - 03-07-26-p-dimension-dictionary-expansion.md
---

# Session: Purpose Classifier Investigation (#24) (CLOSED)

**Date**: 2026-03-07
**Issue**: [#24 — Extend purpose classifier beyond OH&S for APPLICATION_SCOPE detection](https://github.com/fractalaw/fractalaw/issues/24)
**Depends on**: #23 (closed) — fitness dictionaries and audit tooling

## Problem (as originally understood)

During #23 Phase 4 validation, `taxa audit-fitness` against FOOD and TRANSPORT: Maritime Safety families returned zero APPLICATION_SCOPE provisions. This was interpreted as the purpose classifier being OH&S-biased.

## Investigation

### Finding: Purpose classifier is NOT OH&S-biased

The APPLICATION_SCOPE regex in `purpose.rs` is universal — no family-specific logic. It matches:

1. "Application" as heading
2. "these/this Regulations/Act/Order... shall apply to/in/where"
3. "regulation N shall not apply"
4. "shall apply to ... as they apply to"
5. "be under a like duty"
6. "does not apply to/where"
7. "shall have (no) effect / ceases to have effect"
8. "provisions of ... apply to/in"
9. "shall bind the Crown"

### Root cause: Missing LAT data, not classifier bias

| Family | Laws | LAT text in LanceDB? | APPLICATION_SCOPE |
|--------|------|----------------------|-------------------|
| OH&S: Occupational | 451 | Yes | 398 |
| CLIMATE CHANGE | 392 | Yes (59 enriched) | **150** (90% polarity) |
| FOOD | 373 | **No** | 0 |
| TRANSPORT: Maritime Safety | 280 | **No** | 0 |

CLIMATE CHANGE proves the classifier works for non-OH&S families. FOOD and Maritime have zero results because their provision text hasn't been synced to LanceDB from sertantai.

### CLIMATE CHANGE audit results

| Metric | Value |
|--------|-------|
| APPLICATION_SCOPE provisions | 150 |
| Polarity matched | 135 (90.0%) |
| At least one p-dimension tag | 30 (20.0%) |
| Gap provisions | 105 |
| Top candidates | participant, year, storage, permit |

The 20% tagged rate (vs 52.3% for OH&S) shows dictionary expansion for CLIMATE CHANGE would have high impact — but that's a dictionary issue (#23 architecture), not a classifier issue.

## Resolution

**#24 closed as "not planned"** — no code changes needed. The real blocker for non-OH&S fitness expansion is LAT population (syncing full-text law data from sertantai), not the purpose classifier.

## Key Files

- `crates/fractalaw-core/src/taxa/purpose.rs` — universal APPLICATION_SCOPE pattern (line 65-93)
- `docs/FITNESS-DICTIONARY-RUNBOOK.md` — workflow for expanding dictionaries once LAT arrives

## Status: **Closed** — investigation complete, #24 closed as not planned
