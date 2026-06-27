# Session: Dependency Parsing Features (ACTIVE)

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

## Implementation approach

1. ✅ Prototype with spaCy en_core_web_sm — +3.0% accuracy
2. ⬜ Improve actor matching (phrase-level, not token)
3. ⬜ Try en_core_web_trf (transformer model) for better parses
4. ⬜ Retrain position classifier v3 with dep features
5. ⬜ Add section_type feature (carried from agree-wrong session)
6. ⬜ If significant: cache parse results in Postgres, integrate into pipeline

## Carried from classifier training + agree-wrong fixes

- Deep-dive on agree+wrong cases done in classifier training session — findings inform feature priorities here.
- ⬜ Add `section_type` as classifier feature (10 categories one-hot) — cheap win for ~24 errors where structural section types (sub_article, sub_section) correlate with mentioned/beneficiary positions. Requires classifier retrain.

## Dependencies

- ✅ provision_actors table with per-actor signals
- ✅ Classifier baseline: 60% (LR and GBT identical)
- Needs: spaCy model installed, Python feature extraction script
