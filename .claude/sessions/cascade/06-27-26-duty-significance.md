# Session: Duty Significance / Importance Rating (ACTIVE)

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

## Work

1. ⬜ Define significance taxonomy (HIGH/MEDIUM/LOW or numeric)
2. ⬜ Build rule-based scorer using dep features + verb analysis + hierarchy
3. ⬜ Score benchmark provisions and compare against human judgement
4. ⬜ Add `significance` column to provision_actors
5. ⬜ Publish significance signal to sertantai

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
