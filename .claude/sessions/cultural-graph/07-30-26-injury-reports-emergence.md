---
session: Injury Reports Emergence Pass
status: closed
opened: 2026-07-30
closed: 2026-07-31
outcome: success

summary: >
  Processed 300 injury reports through constrained extraction. Cares-for surged to 145 (4x any other source) —
  welfare is the dominant cultural dynamic when harm occurs. Completes the 4-source reporter-authored corpus:
  1,199 narratives, 5,066 cultural edges across all 13 types.

decisions:
  - what: Skip emergence pass for injury reports — constrained pass only
    why: Schema validated as stable across 3 prior sources including near-misses (the first incident type). Injury is reporter-authored like the others, not a new author perspective. No schema discovery value.
    result: Faster processing, schema confirmed stable — no new types emerged

metrics:
  constrained: { valid: 300, invalid: 0, yield_pct: 100 }
  cultural_edges: { total: 745, cares_for: 145, speaks_up_to: 108, shares_information_with: 170, cooperates_with: 92, directs: 81 }
  dataset: { rows: 300, sites: 41, median_words: 114 }
  combined_corpus: { narratives: 1199, cultural_edges: 5066, source_types: 4 }

lessons:
  - title: Cares-for is the signature edge type of injury reports — 4x any other source
    detail: 145 cares-for edges vs 37 (near-miss), 36 (positive obs), 7 (hazard). When harm occurs, welfare gestures dominate. Each source type has a distinct peak signal — compliance (PO), speaking-up (hazard), intervention (near-miss), care (injury). No single source captures the full cultural picture.
    tag: data
  - title: Injury narratives are shorter and more operationally dense
    detail: Median 114 words vs 176 for hazards/near-misses. Only 23% of relationships are cultural (vs 36% for PO). The cultural signal concentrates in the response — who helped, who was informed — not in the incident description itself. The RE_ActionImmediate field is proportionally more important for injuries than other source types.
    tag: data
  - title: 41 sites in injury data — broadest geographic coverage of any source
    detail: More sites report injuries (41) than positive observations (24), hazards (26), or near-misses (30). Injury reporting may be more uniformly practised across the organisation than voluntary observation reporting.
    tag: data

artifacts:
  - data/qq/cultural-graph/outputs/training/injury-training-narratives.parquet
  - data/qq/cultural-graph/outputs/training/injury-training-entities.parquet
  - data/qq/cultural-graph/outputs/training/injury-training-relationships.parquet
  - data/qq/cultural-graph/outputs/training/injury-training-cultural-edges.parquet
  - data/qq/cultural-graph/outputs/briefs/injury-reports-phase1-brief.md

depends_on:
  - 07-30-26-near-miss-reports-emergence.md
  - 07-30-26-hazard-reports-emergence.md

enables:
  - Combined training set build (all 4 reporter-authored sources complete)
  - SLM fine-tuning with 5,066 cultural edges (2.9x positive-observations-only)
  - Resume of suspended Training Data Preparation session
---

# Session: Injury Reports Emergence Pass (CLOSED)

## Problem

Injury reports are the fourth reporter-authored source type. These narratives describe incidents where harm actually occurred — the most severe end of the reporting spectrum. Schema is stable (validated across 3 sources), so constrained pass only. Blames likely still absent (reporter-authored), but responds-to-failure-of and cares-for should be prominent.

## Todo

- ✅ Receive injury reports dataset — `Injury_strict_redacted(Sheet1).csv`
- ✅ Profile dataset — 300 rows, 41 sites, median 114 words, 99% have RE_ActionImmediate
- ✅ Run constrained pass — 300/300 valid, cares-for 145 (dominant signal), 745 cultural edges
- ✅ Export training data to Parquet — 300 narratives, 2,729 entities, 745 cultural edges
- ✅ Produce executive brief — combined phase 1+2, includes full 4-source corpus summary
- ✅ NAS backup

## Dependencies

- ✅ Extraction pipeline validated across 3 source types
- ✅ Scripts generalised with --input, --id-prefix, --source-type
- ✅ Edge type schema stable (13 cultural types + operational)
- ✅ Injury reports CSV received
