---
session: Hazard Reports Emergence Pass
status: closed
opened: 2026-07-30
closed: 2026-07-30
outcome: success

summary: >
  Processed 300 hazard reports through emergence and constrained extraction passes. Produced 1,302 typed
  cultural edges — speaks-up-to (326), responds-to-failure-of (118), and normalises (68) fill the critical
  gaps in the positive observations training data. Scripts generalised to handle any source type.

decisions:
  - what: Combine RE_What + RE_ActionImmediate into a single narrative for extraction
    why: The action field describes who was contacted, what was escalated, what workaround was applied. Contains significant cultural signal (challenge-response dynamics) that RE_What alone misses.
    result: 98% of hazard narratives gain the action text, average length increases from ~160 to ~198 words
  - what: Generalise extraction scripts with --input, --id-prefix, --source-type arguments
    why: User flagged that modifying the emergence_pass.py script would break reuse. Each source type needs different input CSV, ID prefix, and source label, but the same extraction logic.
    result: Both emergence_pass.py and constrained_pass.py now accept any source type without code changes

metrics:
  dataset: { records: 300, sites: 26, median_words: 176 }
  emergence: { valid: 299, invalid: 1, yield_pct: 99.7 }
  constrained: { valid: 299, invalid: 1, yield_pct: 99.7 }
  cultural_edges: { total: 1302, speaks_up_to: 326, responds_to_failure_of: 118, normalises: 68, defers_to_by_rank: 6 }
  signal_comparison: { speaking_up_high_pct: 84, workaround_high_pct: 33, compliance_high_pct: 35, negative_polarity_pct: 25 }

lessons:
  - title: Hazard reports invert the positive observations signal profile as predicted
    detail: Speaking-up jumped from 39% to 84%, workarounds from 7% to 33%, compliance dropped from 79% to 35%. The two sources are complementary, not redundant — each dominates different edge types. Combined training data will cover all 13 types.
    tag: data
  - title: RE_ActionImmediate field is culturally rich and should be included for all source types that have it
    detail: The action field captures the response half of the challenge-response cycle. Without it, a hazard report says "X was broken" but not "Y contacted Z who escalated to W." The cultural signal is in the response as much as the observation.
    tag: data
  - title: maxOutputTokens 8192 should be the default for all extraction scripts
    detail: Positive observations had 17/300 truncation failures at 4096. After upgrading to 8192, both hazard passes had only 1/300 failures. The extra cost is negligible.
    tag: tooling

artifacts:
  - scripts/cultural-graph/emergence_pass.py (updated — --input, --id-prefix, --source-type args, RE_ActionImmediate combining)
  - scripts/cultural-graph/constrained_pass.py (updated — same args)
  - data/qq/cultural-graph/outputs/training/hazard-reports-training-narratives.parquet
  - data/qq/cultural-graph/outputs/training/hazard-reports-training-entities.parquet
  - data/qq/cultural-graph/outputs/training/hazard-reports-training-relationships.parquet
  - data/qq/cultural-graph/outputs/training/hazard-reports-training-cultural-edges.parquet
  - data/qq/cultural-graph/outputs/briefs/hazard-reports-phase1-brief.md
  - data/qq/cultural-graph/outputs/briefs/hazard-reports-phase2-brief.md

depends_on:
  - 07-29-26-positive-observations-emergence.md
  - 07-29-26-schema-constrained-extraction.md

enables:
  - Combined training set across all source types
  - Cross-source site-level cultural profiling
  - SLM fine-tuning with balanced edge type coverage
---

# Session: Hazard Reports Emergence Pass (CLOSED)

## Problem

The cultural graph extraction pipeline has been validated on positive observations. Hazard reports are the second source type — expected to show higher frequencies of works-around, monitors, and adapts-to edge types, filling gaps in the positive observations training data. New column `RE_ActionImmediate` provides additional narrative context about what was done in response.

## Todo

- ✅ Receive hazard reports dataset — 300 rows, 24+ sites, new RE_ActionImmediate column (98% populated)
- ✅ Update scripts — added --input, --id-prefix, --source-type args; RE_ActionImmediate auto-combined
- ✅ Profile dataset — 300 rows, 26 sites, median 176 words, 50% speaking-up keywords
- ✅ Run emergence pass — 299/300 valid, 84% speaking-up, 33% workaround, 25% negative polarity
- ✅ Produce phase 1 executive brief — `outputs/briefs/hazard-reports-phase1-brief.md`
- ✅ Run constrained pass — 299/300 valid, speaks-up-to 326, responds-to-failure-of 118, normalises 68
- ✅ Export training data to Parquet — 299 narratives, 1,302 cultural edges
- ✅ NAS backup

## Dependencies

- ✅ Extraction pipeline validated on positive observations
- ✅ Scripts ready: `emergence_pass.py`, `constrained_pass.py`, `export_training_data.py`
- ✅ Edge type schema finalised (13 cultural types + operational)
- ✅ Hazard reports CSV received (`Hazard & Observations_strict_redacted(Sheet1).csv`)
