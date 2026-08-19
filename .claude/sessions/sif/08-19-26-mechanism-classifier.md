---
session: Mechanism Classifier
status: active
opened: 2026-08-19
---

# Session: Mechanism Classifier (ACTIVE)

## Problem

Stage 1 of the SIF classifier: multi-label text classification from incident narratives to ICECI mechanism codes + object/substance codes. Fine-tune Qwen 3 0.6B with classification head, export to ONNX for edge inference. Auto-non-SIF gate for over-exertion/abrasion. Target F1 >= 0.85 for mechanism, >= 0.80 for object.

## Todo

- ✅ Finalise classifier label set: 20 labels (15 NEEDS_ASSESSMENT + 5 AUTO_NON_SIF)
- ✅ Extract stratified training data: 78,045 events in osha_training.parquet (5K cap, reservoir sampled)
- ✅ Supplementary real data: MSHA (274K mine accidents) + OSHA SIR (79K severe injuries) → breathing 412, pressure 1364, explosion 1882. No synthetic needed.
- ⬜ Build benchmark set (~2,000 events, 100% real-world from OSHA held-out + QQ test)
- ⬜ Fine-tune Qwen 3 0.6B on RunPod — multi-label classification head
- ⬜ Export to ONNX
- ⬜ Evaluate against benchmark (F1 by mechanism code, confusion matrix)
- ⬜ Implement `fractalaw-ai::sif::classifier` — ONNX inference wrapper
- ⬜ Add auto-non-SIF gate logic (over-exertion, abrasion → P(SIF)=0)
- ⬜ CLI command `sif classify` — single event or batch
- ⬜ If 0.6B insufficient, try 1.7B and document trade-off

## Dependencies

- ✅ S2: OSHA 1.6M rows with OIICS event codes (training data), QQ 2,747 events with SIFp (test only)
- ✅ S2: OIICS→mechanism mapping table, ICD-11 taxonomy
- ⬜ RunPod access for fine-tuning
