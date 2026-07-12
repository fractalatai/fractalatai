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
4. ✅ Uploaded to RunPod RTX 5090 (32GB). Training: 45 min, 2,940 steps, loss 0.56→0.25.
5. ✅ Eval: precision 93.3%, recall 91.7%, F1 92.5%, scope accuracy 100% (vs ~60% base model)
6. ✅ GGUF: BF16 + Q4_K_M exported. BF16 persisted on network volume. Q4 download interrupted (1.2/2.4GB) — re-quantise from BF16 on next pod attach.
7. ⬜ Download GGUF (re-quantise from BF16 on network volume, then scp)
8. ⬜ Load into Ollama, re-run extraction on gap provisions
9. ⬜ Measure entity quality improvement vs base model
10. ⬜ Re-propagate (Phase 3b) with cleaner entities

## Dependencies

- ✅ 7,325 mentions with curated regex_entities (training data)
- ✅ `/runpod-finetune` skill for LoRA training + GGUF export
- ✅ `/runpod-batch-inference` skill for re-extraction
- ✅ fitness_mentions per-tier columns (slm_entities independent of regex_entities)
- ⬜ RunPod pod with RTX 5090 (32GB for 16-bit training)
