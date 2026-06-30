---
session: SLM on All Actors — Pipeline Simplification
status: closed
opened: 2026-06-30
closed: 2026-06-30
outcome: success

summary: >
  Retrained SLM with dual DRRP+position output (96.2% DRRP, 80.3% position).
  Ran on all 113,833 actors with confidence scores. Confidence-based gating
  (<0.9 → pending_llm) replaces per-class gating, flagging only 1.4% for LLM.
  Classifier tier effectively replaced by SLM.

decisions:
  - what: Confidence threshold at 0.9 for SLM→LLM elevation
    why: Benchmark data shows 94.9% accuracy at ≥0.99, 34.8% at 0.90-0.95, 14.3% at <0.90. Threshold at 0.9 flags 1,640 actors (1.4%) vs 19,451 at 0.95.
    result: 1.4% pending_llm — manageable LLM budget, human-triggered

  - what: SLM replaces classifier for position classification
    why: SLM 79.7% vs classifier 59.9% on benchmarks. When regex and SLM disagree, SLM is right 82% vs regex 9%. Classifier adds no value.
    result: Removes dep features (spaCy), classifier weights, reconciliation complexity from hub pipeline

  - what: SLM classifies DRRP as well as position in single call
    why: SLM DRRP 92.5% vs regex 87.7%. One inference call replaces both regex DRRP and position classification.
    result: Dual output {"drrp": "...", "position": "..."} with confidence score

  - what: DRRP and position are independent classifications
    why: Gold benchmarks show none+active (56 actors — offence provisions) and Obligation+mentioned (302 actors). Neither derives from the other.
    result: SLM must predict both independently, not derive one from the other

metrics:
  slm_position: { accuracy: 79.7%, active: 94.3%, counterparty: 82.8%, beneficiary: 56.6%, mentioned: 53.8% }
  slm_drrp: { accuracy: 92.5%, obligation: 99.0%, liberty: 95.5%, none: 92.1% }
  regex_position: { accuracy: 53.8% }
  regex_drrp: { accuracy: 87.7% }
  classifier_position: { accuracy: 59.9% }
  confidence_bands: { gte_099: 42.2%, gte_095: 82.9%, gte_090: 98.6%, lt_090: 1.4% }
  confidence_accuracy: { gte_099: 94.9%, lt_090: 14.3% }
  total_classified: 113833
  errors: 0
  time: 181.8min
  cost: ~$3

lessons:
  - title: SLM confidence via logprobs is a top-level API parameter, not in options
    detail: Ollama chat API takes logprobs=True and top_logprobs=N as top-level request fields, not inside the options object. Returns token-level log probabilities in the response.
    tag: models

  - title: Confidence correlates perfectly with accuracy — use it for elevation
    detail: 94.9% at ≥0.99, 62.8% at 0.95-0.99, 34.8% at 0.90-0.95, 14.3% at <0.90. Per-class gating was a proxy for confidence — the real signal is the model's own certainty.
    tag: methodology

  - title: DRRP and position are independent — offence provisions prove it
    detail: "A person who contravenes... is guilty" has drrp=none but position=active. The actor is active in the offence consequence but no new duty is created. Gold labelling is correct — the provision is a consequence, not an obligation.
    tag: data

  - title: Clearing and re-running SLM is cheap enough to not worry about
    detail: Full corpus (113K actors) takes 3 hours at $3. Better to clear and re-run with confidence than to patch incomplete data. The checkpoint/resume (slm_position IS NULL) makes partial runs safe.
    tag: methodology

artifacts:
  - crates/fractalaw-cli/src/commands/taxa.rs
  - crates/fractalaw-store/src/pg.rs
  - crates/fractalaw-store/src/provision_store.rs
  - scripts/runpod_slm_batch.py
  - scripts/export_slm_training_data.py
  - scripts/finetune_runpod_16bit.py
  - scripts/pg_schema.sql

depends_on:
  - 06-27-26-local-llm-tier
  - 06-29-26-tier3-slm

enables:
  - Simplified hub pipeline (regex actor extraction → SLM classification → LLM human-triggered)
  - Confidence-based LLM elevation
  - Future SLM retraining with LLM results as training data
---

# Session: SLM on All Actors — Pipeline Simplification (CLOSED)

## Problem

The current pipeline has 4 tiers (regex → classifier → SLM → LLM) but benchmark data shows SLM outperforms regex decisively:

| Signal | Benchmark actors | SLM accuracy | Regex accuracy |
|--------|-----------------|-------------|---------------|
| Agree | 190 | 78.9% | 78.9% |
| Disagree | 282 | **82.3%** | **9.2%** |

When regex and SLM disagree, SLM is right 82% vs regex 9%. Regex is losing utility. The classifier (65.2%) adds marginal value between regex (53.8%) and SLM (77-82%).

Meanwhile:
- SLM runs at 18.8/s on RunPod ($0.40 for 25K actors)
- Classifier requires dep features (Python/spaCy batch job) + embeddings — heavy dependencies
- Per-class gating (don't trust mentioned/beneficiary) creates 9,368 pending_llm actors that are mostly correct

## Hypothesis

Replace regex → classifier → reconcile → SLM with:

```
Regex (DRRP + actor extraction) → SLM (position for ALL actors) → LLM (disagreements only)
```

Regex still needed for:
- **DRRP type** (Obligation/Liberty/none) — 94.1% accurate, SLM not trained on this
- **Actor extraction** — finding who's mentioned in the text
- **Purpose classification** — structural/substantive gating

But regex **position** (active/counterparty/beneficiary/mentioned) can be replaced by SLM entirely.

## What needs measuring

### Position accuracy
- SLM on ALL actors (not just pending_slm hard cases) — does 82% hold on easy cases too?
- Per-position: active, counterparty, beneficiary, mentioned
- Compare vs regex, classifier, reconciled

### DRRP accuracy
- Does SLM need to classify DRRP as well as position?
- Or is regex DRRP (94.1%) sufficient?
- Obligation vs Liberty split matters — Liberty detection is weaker in regex

### Cost/time
- Full corpus SLM: ~69K actors at 18.8/s = ~62 min on RunPod (~$1)
- vs current: dep features (10 min) + embed (20 min) + classify (30 min) + reconcile + SLM on subset

### What the simplified pipeline eliminates
- Dep features (spaCy Python batch job)
- Embeddings (for classification — still needed for edge/search)
- Position classifier (sklearn LR, v3 weights)
- Reconciliation logic between regex/classifier/SLM
- Per-class gating rules

### What it keeps
- Regex (DRRP + actor extraction + purpose classification)
- SLM (position classification for all actors)
- LLM (human-triggered, for disagreements between regex DRRP and SLM)
- Embeddings (for edge sync / vector search — NOT for classification)

## DRRP × Position matrix

| | active | counterparty | beneficiary | mentioned |
|--|--------|-------------|------------|-----------|
| **Obligation** | ✅ duty-bearer | ✅ claim-holder | ✅ benefits | — |
| **Liberty** | ✅ exercises power | ✅ subject to power | ✅ benefits | — |
| **none** | — | — | — | ✅ just referenced |

Key insight: **"mentioned" falls out automatically once DRRP is correct.** If no Obligation or Liberty is detected in the provision, actors are mentioned — there's no legal relation to position them in. The SLM doesn't need to predict "mentioned" as a class. It needs to answer:

1. Is there an Obligation or Liberty in this provision?
2. If yes, what position does this actor hold in that relation?

If no DRRP → mentioned. If DRRP but actor isn't part of the relation → beneficiary.

## Retraining approach: DRRP-first classification

Current training: `{"position": "active"|"counterparty"|"beneficiary"|"mentioned"}` — 4-class position only.

Proposed training: `{"drrp": "Obligation"|"Liberty"|"none", "position": "active"|"counterparty"|"beneficiary"|"mentioned"}` — dual output.

But the real shift is conceptual: **DRRP detection is the primary classification, position follows from it.** The SLM should learn:

- "The employer shall ensure..." → DRRP=Obligation, this actor is active
- "...health and safety of employees" → DRRP=Obligation, this actor is counterparty
- "In this regulation, 'employer' means..." → DRRP=none, this actor is mentioned

### Corrected matrix (from gold benchmarks)

| | active | counterparty | beneficiary | mentioned |
|--|--------|-------------|------------|-----------|
| **Obligation** | 820 | 533 | 268 | 302 |
| **Liberty** | 396 | 288 | 68 | 127 |
| **none** | 56 | 51 | 18 | 972 |

`mentioned` is valid at ALL DRRP levels — an actor can be mentioned in a provision that has Obligations (e.g. "The employer shall notify the **HSE**" — HSE is mentioned, not active/counterparty). The earlier 1:1 simplification (none=mentioned) was wrong.

The 56 `none+active` and 51 `none+counterparty` are mostly **offence/liability provisions** ("It is an offence to...", "A person who contravenes... is guilty"). These create duties but without "shall/must" modal language. Gemini gold labelling classified DRRP as none — a gold labelling gap. These should probably be Obligation.

**Implication for training**: DRRP and position are independent classifications. The SLM must predict both. `mentioned` doesn't fall out from `drrp=none` — it's a position that can occur with any DRRP type.

Training data changes:
- Add `gold_drrp` to training examples (already in gold_benchmarks table)
- Update export script to include DRRP in the assistant response
- Update prompt to ask for both DRRP and position
- Retrain on RunPod (~$2, ~90 min)
- Validate: DRRP accuracy vs regex (94.1%), position accuracy vs current (77-82%)

## Constraint

This is a **hub pipeline** change only. The edge architecture (LanceDB, ONNX embeddings, local-first inference) is unchanged. Embeddings are still computed for edge vector search.

## Work

1. ✅ Run SLM on ALL 113,833 actors (3hrs on RTX 5090, $3, zero errors)
2. ✅ SLM position accuracy on benchmarks: 79.7% (vs regex 53.8%, classifier 59.9%)
3. ✅ SLM DRRP accuracy: 92.5% (vs regex 87.7%) — SLM beats regex on DRRP
4. ✅ Confidence via logprobs: slm_confidence column, threshold validated against gold
5. ✅ Retrained with dual output (DRRP+position): 96.2% DRRP, 80.3% position on test set
6. ✅ SLM >= reconciled: confidence-based gating replaces per-class gating and classifier tier
7. ✅ Updated reconciliation: SLM ≥0.9 confidence → accept, <0.9 → pending_llm (1.4% of actors)

## Depends on

- ✅ SLM fine-tuned and deployed (gemma3-position in Ollama)
- ✅ RunPod batch script proven (18.8/s, concurrent)
- ✅ Gold benchmarks available (4,062 actors)
