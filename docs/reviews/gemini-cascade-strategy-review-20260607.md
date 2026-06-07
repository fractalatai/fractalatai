# Gemini Review: Classification Cascade Strategy

**Date:** 2026-06-07
**Model:** Gemini 2.5 Flash
**Input:** `docs/CLASSIFICATION-CASCADE-STRATEGY.md`

---

This is a well-thought-out strategy, leveraging established patterns from production ML systems to tackle a common problem in NLP: balancing accuracy with cost. The tiered approach, combined with "done" stamping and customer prioritisation, provides a solid framework for cost-optimised DRRP extraction in legal texts.

## 1. Architecture Assessment

The 3-tier cascade is structurally sound. Key strengths: cost optimization, progressive complexity handling, "done" stamping as critical cost control, customer prioritization.

**Enhancements suggested:**
- Forward feedback loop from LLM to regex — use LLM insights to expand Tier 1 patterns
- Consistency of outputs — all tiers must produce DRRP type AND actor positions consistently

## 2. Confidence Calibration

The 0.7/0.5 thresholds are a starting point. Calibration requires:

1. **Golden dataset** — 500-1000 expertly annotated provisions as ground truth
2. **Cost-benefit curves** — plot cost savings vs F1 at various thresholds
3. **Precision priority for "Done"** — false positives (wrong "done" stamps) are permanent
4. **Recall priority for "Escalate"** — ensure hard cases reach higher tiers

Key metrics: overall F1, tier-specific coverage/accuracy, LLM cost per provision, inter-tier disagreement rate.

## 3. Tier 2 Design

Nearest-neighbour on embeddings is pragmatic but has limitations:
- Semantic similarity ≠ identical DRRP classification
- Hard to transfer actor positions via NN
- Non-linear decision boundaries may need a trainable model

**Recommendation:** Start with NN for DRRP type. If it struggles with positions, pivot to a small MLP on top of embeddings that outputs both DRRP type and position indicators.

## 4. Active Learning

Uncertainty sampling is correct. Pitfalls:
- **Cold start** — Tier 2 initially weak, need diverse seed data from Tier 3
- **Bias** — consistent misunderstanding leads to redundant LLM calls. Add diversity sampling (cluster uncertain points, select representatives)
- **Operationalization** — need tooling for extract → batch → LLM → ingest → retrain loop

## 5. What's Missing

- **Human-in-the-loop QA** — periodic expert review of LLM "done" classifications
- **Error analysis tooling** — categorise why cases fail, feed back to regex improvement
- **Versioning** — `classification_model_version` + `classification_config_version` for full lineage
- **Monitoring** — coverage by tier over time, confidence distributions, cost vs budget alerts
- **Handling correct "none"** — provisions that genuinely have no DRRP should get high confidence and early "done" stamp
- **Resilience** — LLM API retry with exponential backoff
- **Version-aware re-evaluation** — `--force-low-confidence` works on absolute threshold, but major model upgrades might warrant re-evaluating even "done" provisions

## 6. Quick Wins (ordered by impact/effort)

1. **`--force-low-confidence` + `classification_tier` column** — foundational, enables smart re-processing
2. **Customer priority routing (`--priority`)** — ~95% cost reduction by limiting expensive tiers to registered laws
3. **Tier 2 prototype (NN on embeddings)** — quick to build on existing infrastructure, catches "medium hard" cases
4. **Basic active learning harness** — script to identify uncertain Tier 2 predictions and queue for LLM
