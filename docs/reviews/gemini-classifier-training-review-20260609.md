# Gemini Review: Tier 2 Classifier Training Plan

**Date:** 2026-06-09
**Model:** Gemini 2.5 Flash
**Input:** `.claude/sessions/cascade/06-09-26-tier2-classifier-training.md`

---

## Summary

Plan is well-structured. Data-centric phased approach is correct. Key additions:

1. **Phase order correct** — fix data before model
2. **Backfill embeddings now, but fix pipeline long-term** — enrichment/write-back should compute embeddings at write time to prevent future gaps
3. **Active learning for class balance** — train a temporary model to find likely Right/Responsibility provisions in unlabelled corpus, prioritise those for QA. Faster than blind targeted enrichment.
4. **Modal indicators will help** — complement embeddings with domain-specific signals. Not replicating regex — encoding the linguistic cues that differentiate DRRP types.
5. **Biggest risk: 80% accuracy target may be unreachable** with lightweight models on 384-dim embeddings. Legal language nuance may require larger embeddings or more powerful models, leading to protracted data collection and integration delays.
