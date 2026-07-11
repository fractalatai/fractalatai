---
session: Fitness SLM Fine-Tune
status: active
opened: 2026-07-11
---

# Session: Fitness SLM Fine-Tune (ACTIVE)

## Problem

Phase 5 of FITNESS-STRATEGY.md: the base gemma3:4b produced ~40% entity noise during extraction — procedural terms ("claim", "payment", "instrument") alongside genuine applicability subjects ("employer", "marine conservation zone"). The rules engine can't distinguish good from noisy entities; false positives in Match nodes cause incorrect law-customer matches.

7,325 dictionary-extracted mentions provide clean training data — the dictionaries are curated, so their entities are known-correct. Fine-tuning on these should teach the model what a correct fitness entity looks like.

## Work

1. ✅ Enriched dictionaries first (feedback loop): added cross-domain terms (vehicle, building, land, dwelling, LLP, motor vehicle, body corporate, building work, licence) to core dictionaries
2. ✅ Re-ran regex extraction: training examples grew 7,325 → 8,706 (+19% diversity)
3. ✅ Exported training data as JSONL with scope dimensions from entity catalogue. Train: 7,835 / Test: 871
4. ✅ Uploaded to RunPod RTX 5090 (32GB). Training started — gemma3:4b 16-bit LoRA, 3 epochs.
5. ⬜ Training completes (~90 min) — check log, download GGUF
6. ⬜ Evaluate: precision + recall vs base model on held-out test
7. ⬜ Re-run extraction on gap provisions with fine-tuned model
8. ⬜ Measure entity quality improvement
9. ⬜ Re-propagate (Phase 3b) with cleaner entities

## Dependencies

- ✅ 7,325 mentions with curated regex_entities (training data)
- ✅ `/runpod-finetune` skill for LoRA training + GGUF export
- ✅ `/runpod-batch-inference` skill for re-extraction
- ✅ fitness_mentions per-tier columns (slm_entities independent of regex_entities)
- ⬜ RunPod pod with RTX 5090 (32GB for 16-bit training)
