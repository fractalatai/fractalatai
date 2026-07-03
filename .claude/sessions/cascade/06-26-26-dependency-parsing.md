---
session: "Dependency Parsing Features"
status: closed
opened: 2026-06-26
closed: 2026-06-27
outcome: success

summary: >
  Added 7 dependency parsing features per (provision, actor) pair using spaCy. Position
  classifier v3 (428 dims) improved from 61.0% to 64.8% CV accuracy. Counterparty F1
  got biggest boost (+0.068) from subject/object distinction. Live benchmark: classifier
  position 57.4% to 65.2%, agree+wrong errors 243 to 188. Gemini recommended batch
  Python job with precomputed features stored in provision_actors.

decisions:
  - what: "Python batch job for dep features, Rust reads precomputed"
    why: "No mature Rust dep parser with spaCy quality. Clean separation: Python for NLP, Rust for classification."
    result: "Batch script parses all provisions with spaCy, stores 7 features in provision_actors. Incremental via text_hash."
  - what: "Use en_core_web_md over en_core_web_sm"
    why: "Medium model gave +0.9% additional accuracy over small model. en_core_web_trf deferred to GPU availability."
    result: "64.8% CV with md model vs 63.9% with sm. v3 classifier exported."
  - what: "section_type feature included but no additional gain"
    why: "Dependency parsing already captures the structural information that section_type would provide."
    result: "10 one-hot features included in v3 weights for completeness but no measurable accuracy improvement."

lessons:
  - title: "Per-actor features break the embedding ceiling"
    detail: "Embedding is per-provision (same for all actors). Dep features are per-actor (subject vs object). This is why counterparty F1 improved most."
    tag: pipeline
  - title: "Dep parsing can also indicate DRRP severity"
    detail: "Verb strength (ensure vs allow), subject breadth (every employer vs an inspector), and qualification presence could rank duties within a law."
    tag: domain
---

# Session: Dependency Parsing Features (CLOSED)

## Problem

Position classifier is stuck at 60% regardless of model (LR and GBT identical). Embedding dominates (79.9% feature importance) but is per-provision, not per-actor. Need per-actor structural features to distinguish active from counterparty.

## What dependency parsing provides

Per (provision, actor) pair:
- `is_subject` — actor is grammatical subject of main verb
- `is_object` — actor is direct/indirect object
- `is_agent_of_passive` — "ensured by the employer" → employer is agent
- `verb_distance` — hops from actor to main verb in parse tree
- `voice` — active/passive voice detection
- `verb_modal` — shall/must/may/entitled

These directly map to Hohfeldian positions — subject of "shall" = active, object = counterparty.

## DRRP importance/severity signal

Dependency parsing could also indicate the **weight/severity** of a DRRP. Not all duties are equal:

- s.2(1) HSWA "duty of every employer to ensure health, safety and welfare" → **general duty**, broad scope, high severity
- "duty to allow an inspector access" → **procedural duty**, narrow scope, lower severity

Structural indicators of importance:
- **Scope of the subject**: "every employer" vs "the person" vs "an inspector" — breadth of who's affected
- **Scope of the object/complement**: "health, safety and welfare at work of all his employees" (broad) vs "access to premises" (narrow)
- **Verb strength**: "ensure" (absolute) vs "allow" (permissive) vs "have regard to" (discretionary)
- **Qualifications**: "so far as is reasonably practicable" weakens; unqualified = stronger
- **Section position**: Part I general duties vs Part IV misc provisions — structural importance

A high/medium/low severity tag per DRRP could be derived from:
- Verb strength × subject breadth × qualification presence
- This is a new dimension beyond DRRP type — it ranks duties within the same law

## Relationship to Legal-BERT

Independent — additive, not either/or:
- **Legal-BERT**: better embedding, better semantic understanding (model swap)
- **Dep parsing**: new per-actor features from sentence structure (new features)

Both improve the classifier but through different mechanisms. Dep parsing is higher impact for position since it's per-actor.

## Results (2026-06-27)

### 7 dep features, spaCy en_core_web_sm

| Metric | Without dep | With dep | Change |
|--------|-------------|----------|--------|
| CV accuracy | 61.0% | **63.9%** | **+3.0%** |
| Active F1 | 0.625 | 0.666 | +0.041 |
| Counterparty F1 | 0.439 | 0.507 | +0.068 |
| Beneficiary F1 | 0.429 | 0.446 | +0.017 |

Counterparty got biggest boost — subject/object distinction working.

### Features extracted per (provision, actor)
- `is_subject` — actor in nsubj subtree of ROOT verb
- `is_object` — actor in dobj/pobj subtree
- `is_agent_of_passive` — passive agent
- `is_in_attr_subtree` — "duty of every employer" pattern
- `voice_passive` — passive voice detection
- `has_modal` — shall/must/may auxiliary
- `verb_distance` — hops from actor token to ROOT (normalised)

### Known limitations
- Actor matching is token-level (misses multi-word "Secretary of State")
- `en_core_web_sm` is small model — `en_core_web_trf` would give better parses
- Only checks ROOT verb — nested clauses with secondary verbs not captured

## Implementation progress

1. ✅ Prototype with spaCy en_core_web_sm — +3.0% accuracy
2. ✅ Improve actor matching (phrase-level via char_span) — +0.9% more
3. ✅ Try en_core_web_md — 64.8% total (+3.8% over baseline)
4. ✅ Retrain position classifier v3 with dep + section_type features (428 dims)
5. ✅ Section_type feature added — no additional gain on top of dep parsing (dep already captures it)
6. ✅ Integrate v3 into pipeline — batch script + Rust wiring + live benchmark: position 57→65%

### Gemini architecture review (2026-06-27)

**Recommended: Option A (Python batch job) + incremental updates.**

- Batch Python script parses all provisions with spaCy, stores 7 dep features in `provision_actors`
- Rust classifier reads precomputed features — no Python at classify time
- Incremental: only re-parse provisions where text changed (text_hash column)
- Use `en_core_web_trf` for best quality (GPU recommended for 550K provisions)
- `nlp.pipe()` batching + multiprocessing for throughput
- Clean separation: Python for NLP, Rust for classification

Why not other options:
- Microservice (B): network overhead for 550K calls, service management
- Rust-native (C): no mature Rust dep parser with spaCy quality
- Lazy cache (D): Python subprocess from Rust is messy, first run = 300hrs
- ONNX export (E): tree postprocessing in Rust is massive engineering

Future: fine-tune transformer on legal dependency treebank for best legal parse quality.

v3 exported: `docs/position_classifier_v3.json` (64.8% CV, 428 features)

### Comparison table

| Config | CV Accuracy | Active F1 | Counterparty F1 |
|--------|------------|-----------|-----------------|
| Baseline (no dep) | 61.0% | 0.625 | 0.439 |
| + dep (sm, token) | 63.9% | 0.666 | 0.507 |
| + dep (md, phrase) | **64.8%** | **0.676** | **0.510** |

### Live benchmark (v3 with dep features from Postgres)

| Metric | v2 (no dep) | v3 (with dep) | Change |
|--------|-------------|---------------|--------|
| Classifier position | 57.4% | **65.2%** | **+7.8%** |
| Agree+wrong | 243 (13.9%) | **188 (10.8%)** | -55 errors |
| Disagree, cls right | 356 (20.4%) | **429 (24.6%)** | +73 more correct |

## Carried from classifier training + agree-wrong fixes

- Deep-dive on agree+wrong cases done in classifier training session — findings inform feature priorities here.
- ✅ `section_type` included in v3 training (10 one-hot features, no additional gain over dep parsing but included in weights)

## Dependencies

- ✅ provision_actors table with per-actor signals
- ✅ Classifier baseline: 60% (LR and GBT identical)
- Needs: spaCy model installed, Python feature extraction script
