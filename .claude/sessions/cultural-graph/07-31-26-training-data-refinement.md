---
session: Training Data Refinement
status: closed
opened: 2026-07-31
closed: 2026-08-03
outcome: partial

summary: >
  Phase 1 data improvements (negatives + augmentation) completed and integrated into v2 training data.
  Phase 2 (per-model LoRA) overtaken — settled on Qwen 3 8B after testing 3 models. Phase 3 (eval improvements)
  still valid but not blocking production deployment.

lessons:
  - title: Training refinement was overtaken by model selection results
    detail: Per-model LoRA tuning became unnecessary once Qwen 3 8B was confirmed as the production model. The effort would have been spent optimising models we abandoned.
    tag: methodology

depends_on:
  - 07-30-26-training-data-preparation.md

enables:
  - Production inference on full QQ corpus
---

# Session: Training Data Refinement (CLOSED)

## Problem

Gemini review of the model testing plan identified three areas limiting extraction quality: lack of negative examples (model doesn't learn when *not* to extract), overfitting from simple oversampling of rare types (same 18 cares-for narratives repeated 8x), and a one-size-fits-all LoRA recipe that leaves per-model performance on the table. Evaluation metrics are also too coarse to diagnose specific failure modes.

## Todo

### Phase 1 — Data improvements
- ✅ Short narrative CSVs received (4 × ~300 rows, below original character threshold)
- ✅ Filter to zero-signal negatives — 200 selected (50 per source), keyword-filtered + word-count sorted, exported to `outputs/training/negative-examples.jsonl`
- ✅ LLM augmentation of rare-type narratives — 154 rephrased via Gemini, 0 failures. 55 normalises, 55 cares-for, 54 learns-from.
- ✅ Rebuild balanced combined training JSONL — completed in training-data-preparation session as combined-v2-train.jsonl

### Phase 2 — Per-model LoRA configuration
- ❌ Overtaken — settled on Qwen 3 8B after testing 3 models. Per-model tuning not needed.

### Phase 3 — Evaluation improvements
- ⏸️ Per-type F1 (deferred — valid but not blocking production)
- ⏸️ Error categorisation (deferred — valid but not blocking production)
- ⏸️ JSON schema adherence (deferred — valid but not blocking production)
- ⏸️ Inference latency per model (deferred — only Qwen 3 8B in production)

## Dependencies

- ✅ Combined training data from 4 sources (5,066 cultural edges)
- ✅ Gemini review identifying gaps (`data/code-review/cultural-graph-model-testing-plan.md`)
- ✅ Short narrative samples received and filtered (200 negative examples)
- ⬜ Balanced combined fine-tune results (current run completing)
