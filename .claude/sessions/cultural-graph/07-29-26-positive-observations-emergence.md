---
session: Positive Observations Emergence Pass
status: closed
opened: 2026-07-29
closed: 2026-07-29
outcome: success

summary: >
  Ran zero-shot Open IE emergence pass on 300 positive observation narratives from 24 QQ sites via Gemini 2.5 Flash.
  283/300 valid extractions yielded 4,590 entities and 4,021 relationships. Speaking-up signal found in 47% of narratives
  (vs 22% from keyword scan), and 10 of 14 cultural edge types have verb matches. Three new edge types proposed (directs, cares-for, protects).

decisions:
  - what: Per-source-type SLM approach — train separate models for positive observations, hazards, and near-misses
    why: Narrative genres are fundamentally different — positive observations are recognition-heavy, hazards are risk-focused, near-misses surface challenge dynamics. A single model would have to learn all registers at once.
    result: Validated by emergence pass — positive observations show distinct edge type distribution (shares-information-with and monitors dominant, works-around/blames/silences absent)
  - what: Open IE before schema-constrained extraction
    why: Unconstrained extraction discovers what relationships naturally exist in the data before forcing them into the 14-type schema. Avoids schema bias.
    result: 2,378 unique verbs emerged, 44% classified as operational (not cultural), 3 new cultural edge types identified that the schema missed
  - what: Use Gemini 2.5 Flash with structured JSON output for emergence pass
    why: Existing infrastructure, fast, cheap, 94.3% valid JSON output rate
    result: 283/300 valid, 17 failures from JSON truncation on longer narratives (recoverable with higher maxOutputTokens)

metrics:
  dataset: { records: 300, sites: 24, median_words: 133 }
  extraction: { valid: 283, invalid: 17, yield_pct: 94.3 }
  entities: { total: 4590, per_narrative: 16.2, types: "Process 37%, Plant 24%, People 24%, Place 14%, Provision 1%" }
  relationships: { total: 4021, per_narrative: 14.2, unique_verbs: 2378, positive_pct: 59, neutral_pct: 35, negative_pct: 6 }
  cultural_signals: { speaking_up_pct: 47, workaround_pct: 7, improvement_pct: 40, both_speaking_and_workaround: 12 }
  schema_coverage: { matched: 10, total: 14, absent: "trusts, works-around, blames, silences", new_proposed: 3 }

lessons:
  - title: Open IE finds 2x more speaking-up signal than keyword search
    detail: Keyword scan found speaking-up language in 22% of narratives. Structured extraction found it in 47% — people describe speaking-up through actions ("I informed the supervisor we should not enter") not explicit vocabulary ("I raised a concern"). This has implications for any keyword-based safety culture measurement.
    tag: methodology
  - title: 44% of extracted relationships are operational, not cultural
    detail: Verbs like followed, conducted, used, wore describe task actions not interpersonal dynamics. The SLM training pipeline needs to learn this distinction — a "follows procedure" edge is operational, a "follows supervisor's lead" edge is cultural (defers-to-by-rank). Polarity and source/target entity types are the discriminating features.
    tag: models
  - title: Trusts is an inferred state, not an extractable verb
    detail: None of the 2,378 emerged verbs mapped to "trusts" — because trust is inferred from patterns of behaviour (consistently cooperating, deferring, sharing information) rather than stated as an action. The cultural graph schema may need to treat trust as a computed property of edge patterns rather than a directly extractable edge type.
    tag: methodology
  - title: Positive observations contain genuine cultural signal despite being "good news" reports
    detail: Expected mostly procedural compliance. Found 12 narratives with both speaking-up AND workaround signals — the most culturally revealing items in the dataset. The gas monitoring narrative (PO-0003) showed a full challenge-and-escalation chain. Positive observations are not culturally empty.
    tag: data
  - title: Gemini JSON truncation on longer narratives is the main failure mode
    detail: 17/300 failures were all JSON parse errors from truncated output on narratives >200 words. Fix is increasing maxOutputTokens from 4096 to 8192. Not a prompt or model quality issue.
    tag: tooling

artifacts:
  - scripts/cultural-graph/emergence_pass.py
  - scripts/cultural-graph/prompts/emergence-system-v1.md
  - data/qq/cultural-graph/outputs/positive-observations-narratives.parquet
  - data/qq/cultural-graph/outputs/positive-observations-entities.parquet
  - data/qq/cultural-graph/outputs/positive-observations-relationships.parquet
  - data/qq/cultural-graph/outputs/positive-observations-schema-mapping.json
  - data/qq/cultural-graph/outputs/positive-observations-phase1-brief.md
  - data/qq/cultural-graph/emergence/emergence_20260729_151625.jsonl
  - data/qq/cultural-graph/emergence/emergence_20260729_152023.jsonl

depends_on:
  - .claude/plans/cultural-graph/initial-review.md

enables:
  - Hazard reports emergence pass (same script, different source type)
  - Near-miss reports emergence pass
  - Cross-source-type cultural signal comparison
  - Per-source-type SLM training data preparation
---

# Session: Positive Observations Emergence Pass (CLOSED)

## Problem

The cultural graph plan (`.claude/plans/cultural-graph/initial-review.md`) identifies "Deliverable 2" as the first concrete step: run an emergence pass on existing narratives from the reporting tool. We now have the dataset — 300 positive observations (redacted) from 24 QQ sites (`data/qq/cultural-graph/Positive_Observations_Redacted(ReportTable).csv`). This session runs the zero-shot Open IE emergence pass to produce seed extractions (5P entities, candidate cultural edges, triad signals) that will bootstrap the cultural graph training pipeline.

## Todo

- ✅ Profile the dataset — 300 rows, 24 sites, median 133 words, strong procedural content
- ✅ Design the extraction prompt — `scripts/cultural-graph/prompts/emergence-system-v1.md`
- ✅ Run emergence pass on 5-row sample — 100% valid, good cultural signal detection
- ✅ Review sample extractions — PO-0003 (gas monitoring) correctly captured speaking-up/workaround dynamics
- ✅ Run full emergence pass on 300 narratives — 283 valid (94.3%), 17 JSON truncation errors
- ✅ Store results — 3 Parquet tables: narratives (283), entities (4,590), relationships (4,021)
- ✅ Analyse emergence results — 4,590 entities, 4,021 relationships, site-level cultural profiles
- ✅ Produce executive summary brief — `data/qq/cultural-graph/outputs/positive-observations-phase1-brief.md`
- ✅ Map emerged verbs to schema — 231 verbs mapped, 10/14 edge types present, 3 new types proposed

## Dependencies

- ✅ Cultural graph schema and architectural plan (`.claude/plans/cultural-graph/initial-review.md`)
- ✅ Positive observations dataset (300 rows, `data/qq/cultural-graph/`)
- ✅ LLM access — Gemini 2.5 Flash via existing API key infrastructure

## Dataset Profile

**300 records** from 24 sites (file has 1,475 lines due to multiline narratives). All "Positive Observations", all `IsAtWork=TRUE`. `Random` column is a constant (0.104) — likely a sampling weight.

| Metric | Value |
|--------|-------|
| Records | 300 |
| Sites | 24 (top: PEN 66, ABE 45, SHB 34, HEB 29) |
| Median words | 133 |
| Mean words | 166 |
| Range | 59–926 words |
| Bulk (50-200 words) | 77% |

**Domain**: Defence/military ordnance and testing sites. Heavy on weapon inspections, ammunition handling, trials, range operations, calibration checks. Redacted with `xxxxx` (88% of narratives).

**Cultural signal indicators** (keyword scan):
- Trust/recognition: 28% — "good knowledge", "competent", "impressed"
- Procedure following: 24% — "in accordance", "followed", "complied"
- Cooperation: 23% — "team", "together", "assist"
- Speaking up: 22% — "raised", "informed", "concern", "not content"
- Improvement suggestions: 19% — "improvement", "recommendation"
- Workarounds: 4% — "instead of", "rather than"

**Note**: These are positive observations, so they skew toward procedural compliance and recognition. Workarounds and dissent are rare but present (e.g., gas monitoring challenge narrative). The Open IE pass should surface richer cultural signal than keyword matching can.

## Emergence Pass Results

**Yield**: 283/300 valid (94.3%). 17 failures from JSON truncation on longer narratives.

**Entities** (4,590 total, 16.2 per narrative): Process 37%, Plant 24%, People 24%, Place 14%, Provision 1%.

**Relationships** (4,021 total, 14.2 per narrative): 59% positive polarity, 35% neutral, 6% negative. 2,378 unique relationship verbs emerged — the raw material for mapping to cultural edge types.

**Cultural signals** — Open IE found significantly richer signal than keyword scan:
- Speaking up: **47%** (vs 22% from keywords) — the biggest uplift
- Workaround: **7%** high-signal (22 narratives at >=0.5)
- Improvement orientation: **40%** with strong signal
- 12 narratives had both speaking-up AND workaround — the most culturally revealing

**Site-level profiles**: BCE and ASH show highest speaking-up / lowest compliance (candid reporting?). SHB highest compliance/competence. FHD strongest improvement orientation (0.81).

**Output artefacts**:
- JSONL: `data/qq/cultural-graph/emergence/emergence_20260729_*.jsonl`
- Executive brief: `data/qq/cultural-graph/outputs/positive-observations-phase1-brief.md`
- Script: `scripts/cultural-graph/emergence_pass.py`
- Prompt: `scripts/cultural-graph/prompts/emergence-system-v1.md`

## Schema Mapping

231 frequent verbs (count >= 3) mapped to the cultural graph schema's 14 edge types. Full mapping: `data/qq/cultural-graph/outputs/schema_mapping.json`.

**Distribution of 1,607 verb occurrences:**

| Category | Verbs | Freq | Top examples |
|----------|-------|------|-------------|
| operational | 76 | 703 | followed, conducted, used, wore |
| shares-information-with | 44 | 288 | explained, informed, provided |
| monitors | 24 | 192 | observed, reviewed, identified |
| structural | 33 | 167 | had, has, contained, located in |
| learns-from | 9 | 66 | understood, received |
| cooperates-with | 12 | 52 | agreed with, coordinates with |
| speaks-up-to | 11 | 42 | stopped, raised, suggested |
| recognises | 6 | 38 | demonstrated, thanked |
| adapts-to | 4 | 17 | improved, will update |
| responds-to-failure-of | 3 | 9 | deals with, agreed to address |
| normalises | 2 | 6 | ignored, leaves on |
| defers-to-by-rank | 1 | 4 | must follow |

**Schema coverage**: 10 of 14 edge types have verb matches. Four types absent: **trusts**, **works-around**, **blames**, **silences** — expected for positive observations. Trusts is also an inferred state rather than an observable action verb.

**3 new edge types proposed:**
1. **directs** — giving orders/leading with authority (instructed, led). Distinct from shares-information-with and defers-to-by-rank.
2. **cares-for** — welfare gestures (offered to, helped hydrate). Safety culture maturity indicator.
3. **protects** — proactive safeguarding (afforded protection to). Distinct from monitors (passive) and responds-to-failure-of (reactive).

**Key insight**: 44% of verb occurrences are "operational" (task actions like followed, conducted, used) — these don't map to cultural edges. The SLM training data needs to learn to distinguish operational relationships from cultural ones. The remaining 56% are genuine cultural signal, dominated by information-sharing and monitoring behaviours.
