---
session: Reconciliation Engine
status: closed
opened: 2026-06-26
closed: 2026-06-28
outcome: success

summary: >
  Built the reconciliation engine that reads all tier signals (regex, classifier,
  inferred, LLM) from provision_actors and writes the final drrp, position, and
  extraction_method. Achieved 66.5% position accuracy across 15 benchmark laws,
  +12.7% over regex-only. Added taxa backfill command for sertantai publish.

decisions:
  - what: Exclude classifier from DRRP reconciliation
    why: Classifier DRRP accuracy is 66.7% vs regex at 87.7% — adding it makes results worse
    result: DRRP stays at 94.1% (regex passthrough)

  - what: Exclude inferred tier from DRRP reconciliation
    why: Inferred DRRP accuracy is 41.7% — only useful for position
    result: Inferred contributes to position only (84.6% accurate there)

  - what: Confidence threshold at 0.7 for classifier override
    why: Validated empirically — classifier is 90.9% right at >=0.9, 60.4% at 0.7-0.9, crossover at ~0.6
    result: 376 actors resolved by confident classifier at 72.9% accuracy

  - what: Flag disagreements below 0.7 as pending_llm instead of falling back to regex
    why: User directive — "anything below 0.7 needs flagging for LLM, don't hand off to regex <0.5"
    result: 3,735 actors (34.6%) flagged for LLM resolution at LOW confidence

  - what: Separate backfill command (taxa backfill) rather than inline in reconcile
    why: Reconcile writes to provision_actors, sertantai reads from legislation_text — separate concerns
    result: Clean two-step flow — reconcile, backfill, publish

metrics:
  reconciled_overall: { matched: 1756, correct: 1168, accuracy: 66.5% }
  reconciled_agree: { matched: 895, correct: 707, accuracy: 79.0% }
  reconciled_classifier: { matched: 376, correct: 274, accuracy: 72.9% }
  pending_llm: { matched: 472, correct: 176, accuracy: 37.3% }
  inferred: { matched: 13, correct: 11, accuracy: 84.6% }
  regex_only: { matched: 1743, correct: 938, accuracy: 53.8% }
  drrp_regex: { accuracy: 94.1% }
  total_actors_reconciled: 10795
  laws_benchmarked: 15

lessons:
  - title: Don't let weaker tiers corrupt a strong one
    detail: Classifier at 66.7% DRRP and inferred at 41.7% DRRP both make results worse when combined with regex at 87.7%. Per-dimension accuracy matters — a tier can be good for position but bad for DRRP.
    tag: methodology

  - title: Confidence scores are meaningful when empirically validated
    detail: Classifier confidence correlates strongly with accuracy (91% at >=0.9, 22% at <0.5). But you must validate the threshold — don't assume 0.5 is the right cutoff. The crossover was at ~0.6.
    tag: methodology

  - title: Reconciliation confidence labels must reflect actual accuracy
    detail: HIGH (73-85%), MEDIUM (53.8%), LOW (37.3%) — the labels map to real accuracy bands. Downstream systems can trust HIGH and flag LOW for review.
    tag: architecture

  - title: Backfill is a separate step from reconciliation
    detail: provision_actors holds per-actor truth, legislation_text holds per-provision aggregates for sertantai. Keeping them separate means you can re-reconcile without re-parsing.
    tag: architecture

  - title: Both-wrong cases are invisible to reconciliation
    detail: When all tiers agree on the wrong answer (187 cases), reconcile can't detect it. Only LLM or human review catches these. Accept this limit.
    tag: methodology

artifacts:
  - crates/fractalaw-cli/src/commands/taxa.rs
  - crates/fractalaw-store/src/pg.rs
  - crates/fractalaw-store/src/provision_store.rs
  - scripts/pg_schema.sql

depends_on:
  - 06-26-26-dependency-parsing
  - 06-26-26-correlative-inference
  - 06-26-26-benchmark-qa

enables:
  - Local SLM tier (pending_llm resolution)
  - Corpus-wide re-process with reconciliation
  - Sertantai publish with reconciled actors
---

# Session: Reconciliation Engine (CLOSED)

## Context

The `provision_actors` table has per-tier signal columns from 4 tiers:
- Regex: regex_drrp, regex_position (53.8% position accuracy)
- Classifier: cls_drrp, cls_position (65.2% position accuracy, v3 with dep features)
- Inferred: inferred_drrp, inferred_position (86.7%, correlative rules)
- LLM: llm_drrp, llm_position (not yet populated for provision_actors)

Reconciliation reads all tier signals and writes the final `drrp`, `position`, `extraction_method` — the output sertantai consumes.

## Work

1. ✅ Rewrite `taxa reconcile` to read from provision_actors (query_all_actor_signals)
2. ✅ Reconciliation rules for both DRRP and position per actor
3. ✅ Write reconciled `drrp`, `position`, `extraction_method`, `reconcile_confidence` to provision_actors
4. ✅ Backfill `legislation_text.drrp_types` / `actors` from provision_actors (`taxa backfill`)
5. ✅ Test: reconcile benchmarks → benchmark QA on reconciled output
6. ⏸️ Corpus-wide: re-parse + re-classify + reconcile full corpus (deferred — operational task for future session)

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
5. Disagree + classifier confidence < 0.7 → flag pending_llm (confidence = LOW)
6. Only regex → use regex (confidence = MEDIUM)

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
5. **Validate confidence threshold** — ✅ confirmed crossover at ~0.6, threshold set at 0.7 (see confidence validation above)
6. **"Both wrong" (187 cases)** — only LLM or human review can catch these, reconcile can't detect them

## Benchmark results (2026-06-28)

### Position accuracy — all 15 benchmark laws (1,756 matched actors)

| Method | Matched | Correct | Accuracy |
|--------|---------|---------|----------|
| **Reconciled overall** | 1,756 | 1,168 | **66.5%** |
| Classifier only | 1,743 | 1,136 | 65.2% |
| Regex only | 1,743 | 938 | 53.8% |
| Inferred only | 13 | 11 | 84.6% |

### By reconciliation method

| Method | Confidence | Matched | Correct | Accuracy |
|--------|-----------|---------|---------|----------|
| reconciled_agree | HIGH | 895 | 707 | **79.0%** |
| inferred | HIGH | 13 | 11 | **84.6%** |
| reconciled_classifier | HIGH | 376 | 274 | **72.9%** |
| pending_llm | LOW | 472 | 176 | 37.3% |

### DRRP accuracy

Reconciled DRRP = regex passthrough. Regex DRRP accuracy: 94.1% (1,050/1,115 where gold_drrp not null).

### Distribution (10,795 actors across 15 laws)

| Method | Count | % |
|--------|-------|---|
| agree | 4,452 | 41.2% |
| pending_llm | 3,735 | 34.6% |
| cls_confident | 2,155 | 20.0% |
| inferred | 453 | 4.2% |

### Analysis

- Confidence labels are meaningful: HIGH = 73-85% accurate, LOW = 37%
- 34.6% of actors flagged for LLM — these are the marginal cases where regex and classifier disagree and classifier isn't confident
- The agree tier (79.0%) is the strongest signal — when both tiers agree they're usually right
- Reconciliation adds +12.7% over regex-only and +1.3% over classifier-only

## Dependencies

- ✅ provision_actors with all 4 tiers populated (benchmarks)
- ✅ Classifier at 65.2% position (v3 with dep features)
- ✅ Inferred tier at 86.7% position
- ✅ Benchmark QA infrastructure
