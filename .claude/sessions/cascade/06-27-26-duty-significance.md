---
session: Duty Significance / Importance Rating
status: closed
opened: 2026-06-27
closed: 2026-07-01
outcome: success

summary: >
  Built and deployed a novel duty significance rating system for UK statutory Obligation provisions.
  40,468 provisions rated across 4 SLM dimensions (scope_duty_bearer, scope_protected_class, gravity, strength)
  plus metadata-derived hierarchy, using a fine-tuned gemma-3-4b-it on RunPod (~$5 total cost).
  No equivalent model exists — this fills a gap between obligation extraction and compliance risk assessment.

decisions:
  - what: 4 decomposed SLM dimensions + 1 metadata dimension (not single overall rating)
    why: Sertantai can re-weight per customer without re-running the model. Gemini review confirmed right decomposition.
    result: 40,468 provisions rated with per-dimension granularity
  - what: Dedicated significance SLM, not extending position/DRRP model
    why: Different task (how important vs who bears it), different training data, independent retraining
    result: gemma3-significance-q4.gguf (2.4GB), ~65 min full corpus on RunPod
  - what: Hierarchy derived from metadata, not SLM-rated
    why: Gemini review found 1% HIGH — structural metadata, not semantic. SLM adds no value here.
    result: v2.1 combined weighted scoring (section number + depth + section_type) with percentile thresholds
  - what: Strength refined — SFARP-qualified → MEDIUM, absolute unqualified → HIGH
    why: "shall ensure" is standard legislative drafting (63% HIGH was meaningless). HIGH reserved for truly absolute duties.
    result: Training data improved to 39% HIGH, but SLM still predicts 71% HIGH (distillation bias — deferred)
  - what: Scope split into duty_bearer and protected_class sub-dimensions
    why: AND condition for single scope too strict (5% HIGH). Both matter independently.
    result: Better distribution, separate signals for filtering

metrics:
  corpus_rated: { provisions: 40468, time_minutes: 65, cost_usd: 1 }
  training_data: { provisions: 2592, source: "gemini-2.5-flash", versions: 2 }
  training_cost: { runpod_usd: 2, gemini_usd: 2, total_usd: 5 }
  hierarchy_v2_1: { high_pct: 33, medium_pct: 35, low_pct: 31 }
  strength_skew: { training_high_pct: 39, slm_high_pct: 71, note: "distillation bias" }

lessons:
  - title: Nobody rates inherent duty significance at provision level
    detail: >
      Web research found compliance risk frameworks (likelihood × impact), obligation extraction,
      and contract scoring — but nobody rates the statutory duty itself. This is genuinely novel.
      Closest is Sirion's contract obligation criticality scoring.
    tag: methodology
  - title: Gemini self-reported confidence is useless
    detail: >
      Gemini 2.5 Flash returns 0.9-1.0 confidence on everything — not discriminating.
      Logprobs only available on Vertex AI, not AI Studio. SLM logprobs (proven at 0.9 threshold)
      provide the quality signal instead. Gemini generates training labels, not production predictions.
    tag: models
  - title: Depth doesn't correlate with importance in UK legislation
    detail: >
      General duties (s.2) and procedural duties (s.28) live at the same depth in HSWA.
      Relative approaches (per-law average, depth-thirds) normalise away useful signal.
      Section number extraction from section_id was the missing signal for hierarchy.
    tag: data
  - title: Distillation bias amplifies skewed training data
    detail: >
      Strength dimension had 39% HIGH in Gemini training data but SLM predicts 71% HIGH.
      The model amplifies the majority class. Needs either more balanced training data or
      post-hoc calibration. Future SLM retrain should address this.
    tag: models
  - title: Per-provision significance, not per-actor
    detail: >
      Significance lives on legislation_text (the provision), not provision_actors (the actor).
      A provision's gravity and scope are properties of the duty itself, not the actor bearing it.
      Different from DRRP/position which are per-actor.
    tag: architecture

artifacts:
  - scripts/gemini_significance.py
  - scripts/export_significance_training.py
  - scripts/runpod_significance_batch.py
  - .claude/skills/customer-batch-parse/scripts/derive_hierarchy.py
  - data/significance_benchmark.json
  - data/significance_v02_benchmark.json

depends_on:
  - 06-24-26-slm-finetune.md

enables:
  - 07-01-26-significance-aggregation.md
  - 07-01-26-significance-publish.md
---

# Session: Duty Significance / Importance Rating (CLOSED)

## Problem

Not all duties are equal. HSWA s.2(1) "duty of every employer to ensure health, safety and welfare" is fundamentally more significant than "duty to allow an inspector access to premises". A customer viewing their compliance register needs to know which duties are critical vs procedural.

Currently all Obligations are equal — no ranking, no severity, no significance signal.

## Research findings (2026-06-27)

### Linguistic signals that correlate with significance

Analysed high-significance vs low-significance duties across HSWA and MHSW benchmarks:

| Signal | High significance | Low significance |
|--------|-----------------|-----------------|
| **Subject breadth** | "every employer", "every self-employed person" | "the person", "an inspector", "the authority" |
| **Verb strength** | ensure, maintain, provide, protect | allow, notify, inform, keep (records) |
| **Object breadth** | "health, safety and welfare at work of all employees" | "access to premises", "a copy of the document" |
| **Universality** | "every" / "any" quantifier on subject | Specific/definite article "the" |
| **Qualification** | "so far as is reasonably practicable" (SFARP) weakens but indicates a general duty | Unqualified specific duties |
| **Hierarchy depth** | Part I general duties (depth 2-4) | Part IV schedules, sub-paragraphs (depth 5+) |
| **Section position** | Early numbered sections (s.2, s.3, s.4) | Later sections, schedules, annexed provisions |

### Proposed significance dimensions

A duty could be rated on 3-5 dimensions:

**1. Scope** (who's affected)
- Universal: "every employer", "all employees" → HIGH
- Categorical: "an employer who..." → MEDIUM  
- Individual: "the person", "an inspector" → LOW

**2. Gravity** (what's at stake)
- Safety/health/life: "health, safety and welfare" → HIGH
- Property/environment: "prevent damage to" → MEDIUM
- Administrative: "keep records", "notify", "display notice" → LOW

**3. Obligation strength** (verb + qualification)
- Absolute: "shall ensure" (no qualification) → HIGH
- Qualified: "shall ensure so far as is reasonably practicable" → HIGH but qualified
- Discretionary: "shall have regard to", "shall consider" → MEDIUM
- Procedural: "shall notify", "shall keep", "shall produce" → LOW

**4. Hierarchy position**
- Part I / General Duties → HIGH
- Named regulations (reg.3, reg.4) → MEDIUM
- Sub-articles, schedules, transitional → LOW

### Where this fits in the pipeline

This is a NEW output dimension, not a modification of DRRP or position. Each (provision, actor) with DRRP=Obligation would additionally get:
- `significance: HIGH/MEDIUM/LOW`
- Or a numeric score 0.0-1.0

### Implementation approaches

**Option A: Rule-based (deterministic)**
Score from linguistic signals:
- Universal quantifier → +0.3
- Strong verb (ensure, protect, maintain) → +0.3
- Safety/health object terms → +0.2
- Low depth (general duty) → +0.1
- SFARP qualification → +0.1 (indicates importance despite being qualified)

Pros: Explainable, no training needed, fast.
Cons: Misses nuance, hard to tune.

**Option B: Classifier (ML)**
Train on manually labelled significance ratings (gold benchmarks + human annotation).

Pros: Learns complex patterns.
Cons: Needs labelled training data we don't have.

**Option C: LLM-based**
Ask Gemini to rate significance for each Obligation provision.

Pros: Best quality, understands legal context.
Cons: Expensive for full corpus.

**Option D: Hybrid — rules + LLM validation**
Rule-based score for all provisions. LLM validates/corrects a sample. Iterate rules based on LLM feedback.

### Dependency parsing contribution

The dep features we just built provide some of the signals:
- `dep_is_subject` + subject token analysis → scope detection
- `dep_has_modal` + verb lemma → obligation strength
- `dep_voice_passive` → passive duties often procedural
- `dep_verb_distance` → structural complexity correlates with procedural

### What the customer sees

In sertantai, the compliance register could show:
```
🔴 HIGH  s.2(1)  Employer shall ensure health, safety and welfare (SFARP)
🟡 MED   s.9(1)  Employer shall prepare and revise safety policy
🟢 LOW   s.20(2) Duty to allow inspector access
```

This helps prioritise compliance effort.

## Revised model (post Gemini review, 2026-07-01)

### Decisions

- **Strength refined**: HIGH reserved for absolute duties with no qualification. SFARP-qualified → MEDIUM.
- **Hierarchy dropped as SLM dimension**: derive from metadata (section_type + depth). Not an LLM classification.
- **Scope split**: `scope_duty_bearer` and `scope_protected_class` as separate sub-dimensions
- **Penalty NOT in SLM**: sertantai models this separately by extracting Offence provisions from the schema
- **3-point scale retained**: HIGH/MEDIUM/LOW — well understood by users
- **Weighted combination deferred**: run across corpus first, then tune weights (ML as future experiment)

### Final dimensions

**SLM-rated (4 dimensions, single inference call):**

1. **Scope: Duty Bearer**
   - HIGH: universal ("every employer", "any person")
   - MEDIUM: categorical ("an employer who operates...", "a competent person")
   - LOW: individual/specific ("the person", "an inspector")

2. **Scope: Protected Class**
   - HIGH: universal ("all employees", "persons", "the public")
   - MEDIUM: categorical ("employees in that workplace", "young persons")
   - LOW: specific ("the document", "the premises")

3. **Gravity**
   - HIGH: health, safety, life, welfare, serious environmental harm
   - MEDIUM: property, financial loss, moderate environmental impact
   - LOW: administrative, procedural, record-keeping, notification

4. **Strength** (refined)
   - HIGH: absolute duty, no qualification ("shall ensure", "must provide" — unqualified)
   - MEDIUM: qualified ("shall ensure SFARP", "shall have regard to", "all reasonable steps")
   - LOW: procedural ("shall notify", "shall keep records", "shall display")

**Metadata-derived (not SLM):**

5. **Hierarchy** — from section_type + depth columns
6. **Penalty** — future, sertantai extracts from Offence provisions

### SLM output

```json
{"scope_duty_bearer": "HIGH"|"MEDIUM"|"LOW", "scope_protected_class": "HIGH"|"MEDIUM"|"LOW", "gravity": "HIGH"|"MEDIUM"|"LOW", "strength": "HIGH"|"MEDIUM"|"LOW"}
```

## Work (revised)

1. ✅ Gemini benchmark ratings generated (2,592 provisions — original 4-dimension model)
2. ✅ Web research (2026-07-01): no direct equivalent found. See below.
3. ✅ Re-run Gemini with revised dimensions — v0.2: 2,592 rated, strength 63%→39% HIGH, scope split working
4. ⏸️ Human review 10-20% sample (deferred — POC user feedback will provide this)
5. ✅ Fine-tuned significance SLM on RunPod (gemma3-significance-q4.gguf, 2.4GB)
6. ✅ Added significance columns to legislation_text (per-provision, not per-actor)
7. ✅ Run across full corpus — 40,468 Obligation provisions rated on RunPod (~65 min, $1)
8. ✅ Derive hierarchy signal from metadata — v1 (absolute depth) was too coarse (HSWA s.2(1) = LOW). Revised to **law-type-relative thresholds**: Acts have deeper structure than SIs/EU Directives, so depth thresholds differ. Distribution improved from 4%/36%/60% to 14%/43%/43%. HSWA s.2(1) now MEDIUM (not perfect but defensible).

   Law-type-relative depth mapping (v2 — Acts include sub_section as primary duty level):
   - **Acts** (ukpga, asp, anaw): depth ≤4 = HIGH, =5 = MEDIUM, >5 = LOW
     (In Acts, sub_section at depth 4 IS the primary duty — s.2(1) not s.2)
   - **SIs** (uksi, wsi): depth ≤2 = HIGH, ≤3 = MEDIUM, >3 = LOW
   - **EU Directives** (eudr): depth ≤2 = HIGH, ≤3 = MEDIUM, >3 = LOW
   - **EU Regulations / other**: depth ≤2 = HIGH, ≤3 = MEDIUM, >3 = LOW
   
   Distribution: HIGH 33%, MEDIUM 35%, LOW 31%. HSWA s.2(1) now correctly HIGH.
   
   **v3 explored and rejected**: tried per-law average depth baseline and per-law depth-thirds. Both failed — relative approaches normalise away useful signal (MEDIUM swallows 75%), and HSWA s.2(1) drops back to LOW/MEDIUM because its depth ≈ law average. The fundamental problem: depth doesn't correlate with importance in UK legislation. General duties (s.2) and procedural duties (s.28) live at the same depth. Section number within a Part matters but isn't in the metadata.
   
   **v2.1 (Combined Weighted Scoring — current)**: Gemini suggested extracting primary section number from section_id. Composite score:
   - 0.4 × (1 - section_number / max_section_in_law) — lower sections score higher
   - 0.3 × (1 / depth) — shallower provisions score higher
   - 0.3 × section_type_weight — section/article=1.0, sub_section=0.8, paragraph=0.4, sub_paragraph=0.2
   
   Percentile-based thresholds: top 33% = HIGH, middle 34% = MEDIUM, bottom 33% = LOW.
   
   Results: HSWA s.2(1)=HIGH ✅, s.3(1)=HIGH ✅, s.7=HIGH ✅, s.2(2)(c)=MEDIUM ✅, s.59(1)(b)=LOW ✅. Section number extraction was the missing signal.
   - Map section_type + depth to HIGH/MEDIUM/LOW
   - HIGH: section_type in (section, article) AND depth <= 3 (general duties, Part I)
   - MEDIUM: section_type in (section, sub_section, article, sub_article, regulation) AND depth 4-6
   - LOW: section_type in (paragraph, sub_paragraph, schedule_paragraph) OR depth > 6
   - Store as `significance_hierarchy` on provision_actors or legislation_text
   - Validate against benchmark laws (HSWA Part I = HIGH, Schedule provisions = LOW)
9. ⏸️ Experiment: overall significance measure per provision (deferred — 07-01-26-significance-aggregation session)
10. ⏸️ Aggregation models for compliance officer use (deferred — 07-01-26-significance-aggregation session)
11. ⏸️ Investigate strength skew — SLM 71% HIGH vs training 39% (deferred — future SLM retrain)
12. ⏸️ Publish significance signal to sertantai (deferred — 07-01-26-significance-publish session)

## Option E: Dedicated fine-tuned SLM (2026-06-30)

Given the success of the position+DRRP SLM (80.3% position, 96.2% DRRP, 18.8/s on RunPod), a dedicated significance SLM is the natural approach.

### Why dedicated, not extending the existing SLM

- Significance is a different task — *how important* vs *who bears it*
- Only runs on Obligation provisions (not all actors)
- Training data would be purpose-built (Gemini rates significance)
- Adding dimensions to the position/DRRP model dilutes the training signal
- Separate model can be retrained independently

### Dimension storage: separate vs combined

Store each dimension separately — combining is a display/business decision:
- `significance_scope: HIGH/MEDIUM/LOW` — breadth of who's affected
- `significance_gravity: HIGH/MEDIUM/LOW` — what's at stake (health/property/admin)
- `significance_strength: HIGH/MEDIUM/LOW` — verb strength + qualification
- `significance_hierarchy: HIGH/MEDIUM/LOW` — structural position in the law
- `significance_overall: HIGH/MEDIUM/LOW` — algorithm-derived from dimensions

Sertantai can re-weight dimensions per customer without re-running the model.

### Open question: decomposed dimensions vs single overall rating

Option 1: Train SLM on 4 decomposed dimensions → combine algorithmically
- More granular, explainable
- Needs 4× the labels in training data
- Each dimension needs clear definition boundaries

Option 2: Train SLM on single overall significance (HIGH/MEDIUM/LOW)
- Simpler, one label per training example
- Gemini can rate overall significance in one call
- Loses granularity — can't re-weight later
- But: is the customer really going to re-weight? Or do they just want "which duties matter?"

Option 3: Gemini rates all 4 dimensions → store separately → train SLM on each
- Best of both — decomposed storage, single training pipeline
- 4 separate SLM models or one multi-output model
- More expensive training data generation (Gemini rates 4 fields per provision)

### Training data pipeline

1. Query all Obligation provisions from benchmark laws
2. Send to Gemini with significance prompt → get ratings per dimension
3. Human validates/corrects a sample
4. Export as JSONL training data
5. Fine-tune dedicated gemma-3-4b-it on RunPod (~$2, 90 min)
6. Run on corpus Obligation provisions

### Cost estimate

- Gemini training data: ~2,000 Obligation provisions × $0.001 = ~$2
- RunPod training: ~$2
- RunPod inference (full corpus): ~60K Obligation actors at 18.8/s = ~53 min, ~$1
- Total: ~$5

## Dependencies

- ✅ Dep parsing features available (v3 classifier)
- ✅ provision_actors table with per-actor signals
- ✅ RunPod fine-tuning pipeline proven (gemma-3-4b-it, LoRA, GGUF export)
- ✅ SLM batch inference proven (18.8/s with concurrent workers)
- Would benefit from verb lemma extraction (extend dep features)

## Web research: equivalent models (2026-07-01)

No direct equivalent found. Nobody rates inherent duty significance at the provision level.

**Closest models from different angles:**

1. **Compliance risk frameworks** (Adherent, Secureframe) — likelihood × impact matrix. Rates *organisational risk of breach*, not the duty's inherent weight. Depends on company controls.
2. **RegTech obligation extraction** (Ascent RegTech, FinregE) — extract obligations as objects, classify by topic/jurisdiction. No published importance rating.
3. **Contract obligation scoring** (Sirion) — criticality = financial impact × regulatory exposure × relationship importance. Closest to ours but for contracts, not statutes.
4. **EU AI Act extraction** (ScienceDirect 2025) — LLMs + knowledge graphs, 93% precision. Classifies type/addressee/predicate but no severity rating.
5. **Hohfeldian analysis in AI** — academic work models responsibility, not severity. Deontic vs potestative maps to our Obligation/Liberty.

**The gap we fill:** everyone extracts and classifies obligations. Nobody rates their *inherent significance*. Compliance frameworks rate breach risk (organisational). We rate the duty itself (universal).

### Gemini confidence (2026-07-01) — skipped

Gemini 2.5 Flash does not support logprobs on the AI Studio API (Vertex AI only). Self-reported confidence tested but poorly calibrated — returns 0.9-1.0 on everything, not discriminating. Skipped for training data generation.

When the SLM is trained on this data, its own logprobs (proven to work at 0.9 threshold for position/DRRP) will provide the quality signal. Gemini is generating training labels, not production predictions.

Sources: [Adherent](https://www.adherent.com/blog/compliance-risk-assessment-a-step-by-step-framework-for-regulatory-teams/), [Ascent RegTech](https://www.ascentregtech.com/our-difference/change-management/), [FinregE](https://finreg-e.com/compliance-services/regulatory-obligations/), [Sirion](https://www.sirion.ai/library/contract-obligations/contract-obligation-compliance-management/), [ScienceDirect](https://www.sciencedirect.com/science/article/pii/S2212473X25001026)

# Gemini Review: Duty Significance Model (2026-07-01)

## Context

Reviewed the 4-dimension significance rating model for UK statutory Obligation provisions. 2,592 benchmark provisions rated by Gemini Flash. Dimensions: scope, gravity, strength, hierarchy.

## Distribution

| Dimension | HIGH | MEDIUM | LOW |
|-----------|------|--------|-----|
| Scope | 136 (5%) | 1,123 (43%) | 1,333 (51%) |
| Gravity | 632 (24%) | 948 (37%) | 1,012 (39%) |
| Strength | 1,638 (63%) | 679 (26%) | 275 (11%) |
| Hierarchy | 30 (1%) | 1,833 (71%) | 729 (28%) |

## Feedback

### 1. Dimensions — right decomposition?

- **Right decomposition** overall. Covers key aspects.
- **Missing: Enforceability/Penalty** — severity of penalty for breach (criminal, civil, admin fine). High-gravity duty with low penalty is treated differently.
- **Missing: Clarity/Ambiguity** — how clearly defined. Ambiguous duties harder to comply with.
- **Missing: Frequency/Recurrence** — one-off vs ongoing obligation.
- **Redundant: Hierarchy** — more about structural metadata than semantic significance. 1% HIGH confirms this.

### 2. Strength skews 63% HIGH — too broad

- "Shall ensure" and "must provide" are standard legislative drafting. Nearly every Obligation uses them.
- **HIGH should be reserved for truly absolute duties** with no explicit or implicit qualification.
- **MEDIUM should encompass majority** of shall/must duties qualified by SFARP, "all reasonable steps", "due diligence".
- **LOW remains** for procedural, discretionary, weak obligations.

### 3. Hierarchy — 1% HIGH, limited usefulness

- Primarily reflects location within statute, not semantic significance.
- **Highly derivable from metadata** (section number, Part, schedule).
- Recommendation: remove as LLM-rated dimension. Use as metadata weighting factor instead.

### 4. Scope — duty-bearer vs protected-class

- Current AND condition for HIGH too strict (5%).
- **Both matter** — duty-bearer breadth AND protected-class breadth.
- Recommendation: split into `Scope_DutyBearer` and `Scope_ProtectedClass`, or relax AND to weighted combination.

### 5. Combining dimensions into overall score

- No single right way — depends on intended use.
- **Weighted sum**: LOW=1, MEDIUM=2, HIGH=3, apply weights per dimension.
- **Rule-based**: "If Gravity=HIGH, Overall=HIGH regardless".
- **ML post-hoc**: train decision tree on expert-assigned overall scores.
- Start with weighted sum, review weights with legal experts.

### 6. Granularity — 3-level vs 5-level

- **3-level good for initial training** — simpler, more consistent LLM output.
- Explore 5-level or numeric later if human-annotated data supports it.

### 7. Concerns about Gemini-generated labels

- **Hallucination risk** — plausible but incorrect labels.
- **Bias propagation** — Gemini biases transfer to SLM.
- **Inconsistency** — may not apply definitions uniformly.
- **Mitigation**: human review 10-20% of labels, measure inter-annotator agreement, iterative refinement.

## Actions

- Refine Strength dimension — SFARP-qualified → MEDIUM, not HIGH
- Drop Hierarchy as LLM dimension — derive from metadata
- Split or relax Scope AND condition
- Consider Enforceability/Penalty as 4th dimension (replacing Hierarchy)
- Human review sample before SLM training
