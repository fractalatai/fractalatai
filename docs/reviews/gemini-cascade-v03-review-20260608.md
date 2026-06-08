# Gemini Review: Classification Cascade Strategy v0.3

**Date:** 2026-06-08
**Model:** Gemini 2.5 Flash
**Input:** `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`

---

## Summary

v0.3 is a necessary and well-justified pivot. The data unequivocally supports demoting regex from classifier to sieve.

## Key points

1. **Regex demotion justified** — confidence scores negatively correlated with correctness in complex cases. Data-driven decision.

2. **Actor-count routing correct** — simple, interpretable, directly aligned with observed performance. A more sophisticated "complexity score" could be built later but actor count is the pragmatic choice now.

3. **21-hour CPU cost acceptable** — one-time investment for foundational data quality. Can be parallelised across cores/machines if needed. Ongoing cost ~10 min/law is manageable.

4. **New risks introduced:**
   - LLM-specific errors harder to debug than regex failures
   - Ongoing CPU cost for new laws needs monitoring
   - Interpretability: debugging Gemma misclassifications is harder than regex
   - Hierarchy path fragments pushed to Gemma without full context

5. **Build first:** Revise Tier 2 filter to fire on multi-actor OR (single-actor + DRRP=none with actors). This is the core control flow change — everything else depends on it.
