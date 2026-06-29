# Session: SLM Pipeline Wiring (ACTIVE)

## Problem

The fine-tuned gemma3-position model (77.1% local accuracy) is loaded in Ollama but incorrectly wired — it was writing to `llm_drrp`/`llm_position`, conflating SLM (local, free, 77%) with LLM (Gemini, paid, ~95%). These are distinct tiers with different accuracy, cost, and quality signals. The cascade must be:

```
Regex (Tier 1) → Classifier (Tier 2) → SLM (Tier 3, local) → LLM (Tier 4, Gemini) → Human (Tier 5)
```

Each tier needs its own columns and the reconciliation engine must know when to elevate to the next tier vs accept the current result.

## Proposed schema change

Add SLM tier columns to `provision_actors`:

```sql
ALTER TABLE provision_actors ADD COLUMN IF NOT EXISTS slm_drrp TEXT;
ALTER TABLE provision_actors ADD COLUMN IF NOT EXISTS slm_position TEXT;
```

Full tier column map:

| Tier | DRRP column | Position column | Confidence column | Accuracy (position) |
|------|------------|----------------|-------------------|-------------------|
| 1. Regex | regex_drrp | regex_position | — | 53.8% |
| 2. Classifier | cls_drrp | cls_position | cls_confidence | 65.2% |
| 3. SLM | slm_drrp | slm_position | — | 77.1% (on pending cases) |
| 4. LLM (Gemini) | llm_drrp | llm_position | — | ~95% (gold standard) |
| — Inferred | inferred_drrp | inferred_position | — | 86.7% (correlative rules) |

## Proposed reconciliation rules (4-tier)

### Position reconciliation (revised after Gemini review)

1. **LLM present** → LLM wins (confidence = HIGHEST, ~95%)
2. **Inferred present** → use inferred (confidence = HIGH, 86.7%)
3. **SLM present** → SLM wins (confidence = HIGH, 77.1%)
4. **Regex + classifier agree** → confirmed (confidence = HIGH, 79%)
5. **Disagree, classifier ≥ 0.7** → use classifier (confidence = HIGH, 72.9%)
6. **Disagree, classifier < 0.7** → flag `pending_slm` (confidence = LOW)
7. **Only regex** → use regex (confidence = MEDIUM)

Note: Inferred ranks above SLM (86.7% > 77.1%) but in practice they don't compete — inferred creates NEW actors from correlative rules, while SLM classifies existing actors flagged as pending. They operate on different actor sets.

### Elevation signals

| Elevation | Signal | Rationale |
|-----------|--------|-----------|
| Regex → Classifier | Always (classifier runs on all actors) | Free and fast |
| Classifier → SLM | Regex and classifier **disagree** AND classifier confidence **< 0.7** | Below 0.7, classifier is 35-60% right — these are the hard cases |
| SLM → pending_llm | SLM predicts **beneficiary** or **mentioned** | 31% and 58% accurate — not trusted. Accumulates for human-triggered LLM batch |
| pending_llm → LLM | **Human-triggered** | Manual decision to run Gemini. Results become training data for next SLM cycle |

### SLM → LLM elevation (decided: per-class gating, human-triggered)

**SLM is terminal for active and counterparty** — 91.7% and 87.4% accurate. Accepted without review.

**SLM flags beneficiary and mentioned as `pending_llm`** — 31.2% and 58.3% accurate. Not trusted. These accumulate until a human triggers an LLM batch.

**LLM tier is NOT automated.** The human decides when to run Gemini on the pending_llm backlog. This is a deliberate cost/quality control point — same as the human review tier.

**SLM improvement cycle:** LLM results feed back as training data for the next SLM fine-tuning round. Each iteration improves the SLM on its weak classes, shrinking the pending_llm backlog over time. Retraining is cheap (~$2, ~90 min on RunPod).

### DRRP reconciliation (unchanged)

1. LLM present → LLM wins
2. Else → regex wins (94.1% accurate)

SLM currently classifies position only, not DRRP. `slm_drrp` carries regex DRRP forward. Could train a DRRP SLM later if needed.

## Work

1. ✅ Full local eval: 77.1% on 472 pending_llm actors (active 91.7%, counterparty 87.4%, beneficiary 31.2%, mentioned 58.3%)
2. ✅ Build `taxa slm` command (initial version — wrote to llm_* columns, needs fix)
3. ✅ Add slm_drrp/slm_position columns to provision_actors + schema
4. ✅ Update `taxa slm` to write tier="slm", query pending_slm actors
5. ✅ Update upsert_provision_actors to handle tier="slm"
6. ✅ Update reconciliation: 4-tier cascade with per-class SLM gating (active/counterparty accepted, beneficiary/mentioned → pending_llm)
7. ✅ Cleared llm_* data, re-ran reconcile → SLM → reconcile. HSWA: 183 SLM-accepted (79.4%), 202 pending_llm (95.6% against gold)
8. ✅ Per-class gating working: active/counterparty trusted, beneficiary/mentioned flagged for LLM
9. ✅ Run across all 15 benchmark laws: 3,249 classified, 101 errors. Overall 78.2% (+11.7% over reconcile-only)
10. ✅ Run `taxa backfill` — 6,244 provisions across 15 benchmark laws updated in legislation_text
11. ⬜ Corpus-wide: classify all pending actors

## Local eval results (2026-06-29)

472 pending_slm benchmark actors — the hardest cases where regex and classifier disagree and classifier confidence < 0.7. (Previously labelled `pending_llm` before SLM tier was separated.)

| Source | Accuracy |
|--------|----------|
| Regex | 37.3% |
| Classifier | 32.8% |
| **SLM (gemma3-position)** | **77.1%** |

| Position | Gold count | SLM accuracy |
|----------|-----------|-------------|
| active | 181 | **91.7%** |
| counterparty | 127 | **87.4%** |
| beneficiary | 32 | 31.2% |
| mentioned | 132 | 58.3% |

## Dependencies

- ✅ gemma3-position loaded in Ollama (2.4GB Q4_K_M GGUF, from 16-bit training)
- ✅ provision_actors with per-tier columns
- ✅ Reconciliation engine (needs update for 4-tier)
- ✅ taxa backfill aggregates provision_actors → legislation_text
- ✅ eval script uses /api/chat format matching training template
