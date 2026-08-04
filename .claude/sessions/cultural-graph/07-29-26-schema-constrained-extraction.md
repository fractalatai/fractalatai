---
session: Schema-Constrained Extraction
status: closed
opened: 2026-07-29
closed: 2026-07-29
outcome: success

summary: >
  Ran schema-constrained extraction on all 300 positive observation narratives, producing 1,788 typed cultural edges
  across 13 edge types (10 original + 3 new). Exported training data as namespaced Parquet tables, built a 41-narrative
  stratified human review sample, and a 300-row cultural signal classifier training set.

decisions:
  - what: Keep emergence_pass.py and constrained_pass.py as separate scripts
    why: User identified that modifying emergence_pass.py would break reuse for future source types (hazards, near-misses). Each source type runs emergence first, then constrained.
    result: Two scripts, shared CSV loading and Gemini infrastructure, different prompts and output dirs
  - what: Add 3 new edge types to the schema (directs, cares-for, protects)
    why: Emergence pass verb mapping showed cultural dynamics not covered by the original 14 types. All three had meaningful frequency in the constrained pass (directs 115, protects 95, cares-for 34).
    result: 13 active cultural edge types + "operational" label. 4 original types (trusts, works-around, blames, silences) remain absent pending other source types.
  - what: Namespace all output files by source type
    why: User flagged that generic names (narratives.parquet, entities.parquet) would collide when hazard and near-miss data arrives. Future merging into combined-training-* tables.
    result: All files prefixed positive-observations-*
  - what: ONNX classifier for cultural signals, SLM for entity/edge extraction
    why: Two different model sizes for two different tasks. Cultural signal scoring is a simpler classification problem (6 continuous outputs from text) suitable for a lightweight ONNX model. Entity and relationship extraction requires a language model.
    result: Separate training sets exported for each model type

metrics:
  constrained_pass: { valid: 300, errors: 0, yield_pct: 100 }
  relationships: { total: 4978, cultural: 1788, operational: 3190, cultural_pct: 36 }
  cultural_edges_per_narrative: { mean: 6.0 }
  edge_type_coverage: { active_types: 13, absent_types: 4 }
  training_data: { narratives: 300, entities: 4516, relationships: 4978, cultural_edges: 1788 }
  review_sample: { narratives: 41, edge_types_covered: 13, sites_covered: 20 }
  signal_classifier: { rows: 300, signals: 6, best_coverage: "procedural_compliance 75%", worst_coverage: "workaround 7%" }

lessons:
  - title: maxOutputTokens 8192 eliminates JSON truncation errors
    detail: The emergence pass had 17/300 failures (5.7%) from JSON truncation at maxOutputTokens 4096. The constrained pass at 8192 had 0/300 failures. The thinking budget (2048) consumes tokens from the same pool, so effective output capacity at 4096 was only ~2048.
    tag: tooling
  - title: Discrimination rules in the prompt are essential for cultural/operational split
    detail: Without explicit rules like "Person→Process/Plant is almost always operational" and "followed/complied/carried out are operational not cultural", the model over-labels procedural actions as cultural edges. The constrained prompt's discrimination rules section drove the clean 36/64 cultural/operational split.
    tag: methodology
  - title: Separate scripts per extraction mode, not flags on one script
    detail: Temptation was to add --mode emergence/constrained to one script. User correctly identified this breaks reuse — emergence_pass.py will be run unchanged on hazard reports and near-misses. Each extraction mode has its own prompt, output directory, and validation logic.
    tag: architecture
  - title: Stratified sampling for human review must cover rare edge types explicitly
    detail: Random sampling of 41 from 300 would likely miss normalises (15 total), defers-to-by-rank (1 total), and cares-for (34 total). Explicit stratification by edge type, then filling with high-density and low-density narratives for contrast, produces a review sample that tests the full schema.
    tag: methodology

artifacts:
  - scripts/cultural-graph/constrained_pass.py
  - scripts/cultural-graph/prompts/constrained-system-v1.md
  - data/qq/cultural-graph/outputs/positive-observations-training-narratives.parquet
  - data/qq/cultural-graph/outputs/positive-observations-training-entities.parquet
  - data/qq/cultural-graph/outputs/positive-observations-training-relationships.parquet
  - data/qq/cultural-graph/outputs/positive-observations-training-cultural-edges.parquet
  - data/qq/cultural-graph/outputs/positive-observations-signal-classifier-training.parquet
  - data/qq/cultural-graph/outputs/positive-observations-review-sample.json
  - data/qq/cultural-graph/outputs/positive-observations-review-sample.md

depends_on:
  - 07-29-26-positive-observations-emergence.md

enables:
  - Hazard reports emergence + constrained passes (reuse emergence_pass.py, adapt constrained prompt)
  - Near-miss reports emergence + constrained passes
  - Combined training data merge across source types
  - SLM fine-tuning for positive observations edge extraction (RunPod)
  - ONNX cultural signal classifier training
  - Human review cycle with domain experts
---

# Session: Schema-Constrained Extraction (CLOSED)

## Problem

The emergence pass produced 4,021 relationships using 2,378 free-form verbs. 44% are operational (task actions), and the cultural verbs are inconsistently named (e.g., "followed", "follows", "complied with" all mean the same thing). To build SLM training data, we need a second extraction pass that forces relationships into the finalised cultural edge type schema — the 10 emerged types plus 3 new types proposed in the last session. This produces clean, consistent labels suitable for fine-tuning.

## Todo

- ✅ Finalise the edge type schema — 13 cultural types + "operational" label
- ✅ Design schema-constrained extraction prompt — `scripts/cultural-graph/prompts/constrained-system-v1.md`
- ✅ Run constrained pass on 5-row sample — 100% valid, clean cultural/operational split
- ✅ Review sample — PO-0003 correctly typed: 2x speaks-up-to, directs, responds-to-failure-of
- ✅ Run constrained pass on full 300 narratives — 300 valid (100%), 0 errors, 1,788 cultural edges
- ✅ Export training data to Parquet — 4 tables namespaced as positive-observations-training-*
- ✅ Design human review sample — 41 narratives stratified across all 13 edge types, 20 sites
- ✅ Build cultural signal ONNX classifier training set — 300 narratives with 6-dim signal scores

## Dependencies

- ✅ Emergence pass complete — 283 valid extractions (`07-29-26-positive-observations-emergence.md`)
- ✅ Schema mapping — 231 verbs mapped to edge types (`positive-observations-schema-mapping.json`)
- ✅ Gemini API access
- ⬜ Domain expert availability for human review sample (not blocking — can prepare the sample first)

## Edge Type Schema (Working)

**Original 14 (10 with verb matches in positive observations):**

| Edge type | Status | Top emerged verbs |
|-----------|--------|-------------------|
| shares-information-with | active | explained, informed, provided |
| monitors | active | observed, reviewed, identified |
| learns-from | active | understood, received |
| cooperates-with | active | agreed with, coordinates with |
| speaks-up-to | active | stopped, raised, suggested |
| recognises | active | demonstrated, thanked |
| adapts-to | active | improved, will update |
| responds-to-failure-of | active | deals with, agreed to address |
| normalises | active | ignored, leaves on |
| defers-to-by-rank | active | must follow |
| trusts | absent | (inferred state, not extractable as verb — may become computed property) |
| works-around | absent | (expected in hazard/near-miss data) |
| blames | absent | (expected in hazard/near-miss data) |
| silences | absent | (expected in hazard/near-miss data) |

**New types proposed from emergence pass:**

| Edge type | Emerged verbs | Rationale |
|-----------|--------------|-----------|
| directs | instructed, led | Authority/command — distinct from shares-information-with (informational) and defers-to-by-rank (receiver's yielding) |
| cares-for | offered to, helped hydrate | Welfare gestures — safety culture maturity indicator, distinct from cooperates-with (task coordination) |
| protects | afforded protection to, protects | Proactive safeguarding — distinct from monitors (passive) and responds-to-failure-of (reactive) |

## Constrained Pass Results

**300/300 valid (100%)** — bumping maxOutputTokens to 8192 eliminated the truncation errors from the emergence pass.

**4,978 relationships:** 1,788 cultural (36%), 3,190 operational (64%). Mean 6.0 cultural edges per narrative.

| Edge type | Count | % of cultural |
|-----------|-------|---------------|
| shares-information-with | 466 | 26% |
| cooperates-with | 243 | 14% |
| monitors | 239 | 13% |
| recognises | 221 | 12% |
| speaks-up-to | 149 | 8% |
| directs | 115 | 6% |
| protects | 95 | 5% |
| adapts-to | 83 | 5% |
| learns-from | 67 | 4% |
| responds-to-failure-of | 60 | 3% |
| cares-for | 34 | 2% |
| normalises | 15 | 1% |
| defers-to-by-rank | 1 | <1% |

## Training Data

Four Parquet tables in `data/qq/cultural-graph/outputs/`:

| File | Rows | Purpose |
|------|------|---------|
| positive-observations-training-narratives.parquet | 300 | Full text + signal scores + edge counts |
| positive-observations-training-entities.parquet | 4,516 | All 5P entities |
| positive-observations-training-relationships.parquet | 4,978 | All edges with typed labels |
| positive-observations-training-cultural-edges.parquet | 1,788 | Cultural edges only — SLM target |

Namespaced for future merging with hazard-reports-training-* and near-misses-training-* into combined-training-* tables.

## Human Review Sample

41 narratives selected via stratified sampling:
- All 13 edge types covered (including rare: normalises 4, defers-to-by-rank 1, cares-for 5)
- 5 dual-signal narratives (speaking-up + workaround)
- 5 low-cultural-density narratives for contrast
- 20 sites represented

Outputs:
- `positive-observations-review-sample.json` — machine-readable, includes review fields for corrections
- `positive-observations-review-sample.md` — human-readable with tables for marking correct/wrong/spurious/missing

## Signal Classifier Training Set

300 narratives with 6-dim cultural signal scores in `positive-observations-signal-classifier-training.parquet`.

Label coverage varies by signal:
- procedural_compliance: 75%, cooperation: 71% — strong coverage
- competence_recognition: 51%, improvement_orientation: 54% — moderate
- speaking_up: 34%, workaround: 7% — sparse (expected for positive observations, will improve with other source types)
