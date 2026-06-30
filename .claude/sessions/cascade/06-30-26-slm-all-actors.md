# Session: SLM on All Actors — Pipeline Simplification (ACTIVE)

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

1. ⬜ Run SLM on all QQ corpus actors missing slm_position (~35,762 actors, ~32 min on RunPod)
2. ⬜ Measure SLM accuracy vs gold on ALL benchmark actors (not just pending_slm hard cases)
3. ⬜ Compare: SLM-only vs reconciled (regex+cls+SLM) on benchmarks
4. ⬜ Check DRRP: does regex Obligation/Liberty accuracy hold, or does SLM need to classify DRRP too?
5. ⬜ Extract SLM confidence via Ollama logprobs — add `"logprobs": true` to request, capture probability of predicted position token. This replaces per-class gating with a confidence threshold (same pattern as classifier at 0.7). Validate threshold against gold benchmarks.
6. ⬜ Retrain SLM with dual output (position + DRRP) — extend training data, retrain on RunPod
7. ⬜ Validate SLM DRRP accuracy vs regex DRRP (94.1% baseline)
8. ⬜ If SLM >= reconciled: propose simplified pipeline, review with Gemini
9. ⬜ If SLM < reconciled: keep current pipeline, close session

## Depends on

- ✅ SLM fine-tuned and deployed (gemma3-position in Ollama)
- ✅ RunPod batch script proven (18.8/s, concurrent)
- ✅ Gold benchmarks available (4,062 actors)
