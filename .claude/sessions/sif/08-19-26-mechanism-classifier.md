---
session: Mechanism Classifier
status: pending
opened: 2026-08-19
---

# Session: Mechanism Classifier (PENDING)

## Problem

Stage 1 of the SIF classifier: multi-label text classification from incident narratives to ICECI mechanism codes + object/substance codes. Fine-tune Qwen 3 0.6B with classification head, export to ONNX for edge inference. Auto-non-SIF gate for over-exertion/abrasion. Target F1 >= 0.85 for mechanism, >= 0.80 for object.

## Todo

- ⬜ Prepare training data: OSHA narratives + ICD-11 labels + synthetic balance data
- ⬜ Fine-tune Qwen 3 0.6B on RunPod — multi-label classification head
- ⬜ Export to ONNX
- ⬜ Evaluate against benchmark (F1 by mechanism code, confusion matrix)
- ⬜ Implement `fractalaw-ai::sif::classifier` — ONNX inference wrapper
- ⬜ Add auto-non-SIF gate logic (over-exertion, abrasion → P(SIF)=0)
- ⬜ CLI command `sif classify` — single event or batch
- ⬜ If 0.6B insufficient, try 1.7B and document trade-off

## Dependencies

- ⬜ S2: Training data + benchmark set
- ⬜ RunPod access for fine-tuning
