# Session: Reconciliation Engine (ACTIVE)

## Context

The `provision_actors` table has per-tier signal columns from 4 tiers:
- Regex: regex_drrp, regex_position (53.8% position accuracy)
- Classifier: cls_drrp, cls_position (65.2% position accuracy, v3 with dep features)
- Inferred: inferred_drrp, inferred_position (86.7%, correlative rules)
- LLM: llm_drrp, llm_position (not yet populated for provision_actors)

Reconciliation reads all tier signals and writes the final `drrp`, `position`, `extraction_method` — the output sertantai consumes.

## Work

1. ⬜ Rewrite `taxa reconcile` to read from provision_actors (current version reads legislation_text)
2. ⬜ Reconciliation rules for both DRRP and position per actor
3. ⬜ Write reconciled `drrp`, `position`, `extraction_method` to provision_actors
4. ⬜ Backfill `legislation_text.drrp_types` / `actors` from provision_actors for sertantai publish
5. ⬜ Test: reconcile benchmarks → benchmark QA on reconciled output
6. ⬜ Corpus-wide: re-parse + re-classify + reconcile full corpus

## Reconciliation rules (revised after Gemini review)

Per (section_id, actor_label):

**DRRP** (simplified — classifier excluded, inferred excluded):
1. LLM present → LLM wins (`extraction_method = 'llm'`, confidence = HIGH)
2. Else → use regex (`extraction_method = 'regex'`, confidence = HIGH)

Rationale: regex is 87.7% on DRRP, classifier is 66.7% (makes it worse), inferred is 41.7% (terrible). Don't let weaker signals corrupt a strong one.

**Position** (classifier + inferred participate, confidence-tiered):
1. LLM present → LLM wins (confidence = HIGHEST)
2. Inferred present → use inferred (confidence = HIGH, 86.7% accurate)
3. Regex + classifier agree → confirmed (confidence = HIGH, 79% accurate)
4. Disagree + classifier confidence ≥ 0.7 → use classifier (confidence = HIGH, 60-91% right)
5. Disagree + classifier confidence 0.5-0.7 → flag pending_llm, use regex interim (confidence = LOW)
6. Disagree + classifier confidence < 0.5 → use regex (confidence = MEDIUM, regex better below 0.5)
7. Only regex → use regex (confidence = LOW)

### Confidence validation (2026-06-27)

Classifier accuracy by confidence when disagreeing with regex:

| Confidence | Classifier right | Regex right |
|-----------|-----------------|-------------|
| ≥ 0.9 | **90.9%** | 4.0% |
| 0.7-0.9 | **60.4%** | 19.3% |
| 0.5-0.7 | 35.1% | 37.2% |
| < 0.5 | 21.8% | **43.5%** |

Confidence IS a valid signal. Crossover at ~0.6. Trust classifier above 0.7, trust regex below 0.5, flag LLM between.

**Output columns:**
- `drrp` — reconciled DRRP type
- `position` — reconciled position
- `extraction_method` — source tier (llm/inferred/reconciled_agree/reconciled_classifier/regex)
- `reconcile_confidence` — HIGH/MEDIUM/LOW

### Gemini review feedback (2026-06-27)

1. **Classifier excluded from DRRP** — 66.7% makes it worse than regex alone
2. **Inferred excluded from DRRP** — 41.7%, use inferred for position only
3. **Confidence score essential** — downstream systems need to know trust level
4. **review_flag for human review** — when all tiers low confidence or LLM contradicts consensus
5. **Validate confidence threshold** — check if classifier accuracy improves at ≥ 0.7 before using as signal (TODO)
6. **"Both wrong" (187 cases)** — only LLM or human review can catch these, reconcile can't detect them

## Dependencies

- ✅ provision_actors with all 4 tiers populated (benchmarks)
- ✅ Classifier at 65.2% position (v3 with dep features)
- ✅ Inferred tier at 86.7% position
- ✅ Benchmark QA infrastructure
