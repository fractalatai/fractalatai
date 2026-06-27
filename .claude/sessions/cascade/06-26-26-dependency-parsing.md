# Session: Dependency Parsing Features (PENDING)

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

## Implementation approach

1. Use spaCy (`en_core_web_trf` or `en_core_web_lg`) for dependency parsing
2. Extract features at training time from benchmark texts
3. Test impact on classifier accuracy before full pipeline integration
4. If significant: add spaCy as a Python preprocessing step, cache parse results in Postgres

## Carried from classifier training + agree-wrong fixes

- Deep-dive on agree+wrong cases done in classifier training session — findings inform feature priorities here.
- ⬜ Add `section_type` as classifier feature (10 categories one-hot) — cheap win for ~24 errors where structural section types (sub_article, sub_section) correlate with mentioned/beneficiary positions. Requires classifier retrain.

## Dependencies

- ✅ provision_actors table with per-actor signals
- ✅ Classifier baseline: 60% (LR and GBT identical)
- Needs: spaCy model installed, Python feature extraction script
