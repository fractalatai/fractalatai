---
session: Energy Analyser
status: pending
opened: 2026-08-19
---

# Session: Energy Analyser (PENDING)

## Problem

Stage 2 of the SIF classifier: structured extraction from narratives to energy type, source/carrier/environment cues, body vulnerability, and severity quantiles (P10/P50/P90). Fine-tune Qwen 3 4B for JSON-structured output. Chains with Stage 1 mechanism classifier → metalog fit → P(SIF). Target: SIF recall >= 0.90, precision >= 0.80. Test against QQ SIFp benchmark (target: classifier-human agreement > 65% baseline).

## Todo

- ⬜ Annotate OSHA narratives with energy analysis labels (LLM-assisted + human validation)
- ⬜ Fine-tune Qwen 3 4B on RunPod — structured JSON output
- ⬜ Implement `fractalaw-ai::sif::energy_analyser` — Ollama inference
- ⬜ Build two-stage pipeline: Stage 1 → Stage 2 → metalog fit → P(SIF)
- ⬜ Evaluate end-to-end against gold benchmark (recall, precision, F1)
- ⬜ Evaluate against QQ SIFp benchmark (classifier vs human agreement)
- ⬜ CLI command `sif batch` — batch classification with DuckDB output
- ⬜ Handle missing data: Bayesian priors when magnitude cues absent

## Dependencies

- ⬜ S2: Training data + benchmark set
- ⬜ S4: Stage 1 mechanism classifier (feeds into Stage 2)
- ⬜ RunPod access for fine-tuning
