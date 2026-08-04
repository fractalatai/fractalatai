---
session: Training Data Preparation
status: closed
opened: 2026-07-30
closed: 2026-07-31
outcome: success

summary: >
  Built combined training corpus (1,199 narratives, 5,066 cultural edges, 4 sources), tested 3 base models
  (Qwen 3 8B, Qwen 2.5 7B, Llama 3.1 8B), achieved F1=0.612 with Qwen 3 8B balanced. Exported working
  4.7GB GGUF running locally via Ollama. Both edge and cloud deployment paths validated.

decisions:
  - what: Single combined model instead of per-source-type models
    why: Combined 4-source training outperforms any single-source model. Edge types are the same across sources — speaks-up-to is speaks-up-to whether in a PO or near-miss. Source type is metadata, not a model boundary.
    result: F1=0.612 (combined) vs F1=0.558 (PO-only)
  - what: Qwen 3 8B as the production model
    why: Tested 3 models — Qwen 3 8B (F1=0.612), Llama 3.1 8B (F1=0.608), Qwen 2.5 7B (F1=0.572). Qwen 3 8B wins on aggregate F1 and per-type calibration. Qwen 2.5's "designed for JSON" advantage didn't materialise.
    result: Qwen 3 8B selected for both edge (GGUF) and cloud (4-bit adapter)
  - what: Shelve defers-to-by-rank from active schema
    why: Military-specific, only 8 instances across 1,200 contractor narratives. QQ is a defence contractor, not military. Retain in schema library for military client deployments.
    result: 12 active cultural types + operational
  - what: 16-bit retrain for GGUF export
    why: 4-bit LoRA weights are lost during GGUF quantisation merge. 4-bit adapters work fine for cloud inference (Path B) but edge (Path A) needs GGUF with baked-in weights.
    result: 16-bit retrain on 100GB container disk pod, successful Q4_K_M export (4.7GB)
  - what: Two deployment architectures (edge and cloud)
    why: IT may block Ollama on managed devices. Self-hosted Baserow + RunPod preserves data sovereignty while avoiding IT blockers. Both use the same model.
    result: Both paths validated — edge via Ollama GGUF, cloud via 4-bit adapter on RunPod

metrics:
  best_model: { name: "Qwen 3 8B balanced combined v1", f1: 0.612, precision: 0.611, recall: 0.613, valid_json_pct: 100 }
  deployed_model: { name: "Qwen 3 8B 16-bit v2 GGUF", f1: 0.581, precision: 0.607, recall: 0.557, gguf_size_gb: 4.7 }
  training_data: { narratives: 1199, cultural_edges: 5066, balanced_examples: 7231, negatives: 200, augmented: 154 }
  models_tested: { qwen3_8b: 0.612, llama31_8b: 0.608, qwen25_7b: 0.572 }
  edge_inference: { first_call_seconds: 101, estimated_warm_seconds: "30-60", batch_300_hours: "2.5-5" }

lessons:
  - title: Class balancing via oversampling matters more than data volume
    detail: Unbalanced combined (1,060 ex, 4 sources) scored F1=0.484. Same data balanced to 6,292 examples scored F1=0.612. Class imbalance collapses rare edge types regardless of total data volume. This was the single biggest training insight.
    tag: models
  - title: 4-bit LoRA training produces working GPU adapters but broken GGUFs
    detail: 4-bit QLoRA works perfectly for inference on GPU (RunPod). But during GGUF merge, the small LoRA corrections are rounded away by quantisation. Edge deployment (Ollama GGUF) requires 16-bit retrain. This is documented in the runpod-finetune skill but we missed it. Cost us an extra GPU hour.
    tag: models
  - title: RunPod container disk must be 100GB for 8B model GGUF export
    detail: GGUF export downloads full 16-bit base model (~16GB), merges adapter, writes BF16 intermediate (~16GB), then quantises to Q4 (~5GB). Default 50GB container disk is not enough. Also network volume breaks llama-quantize — write to /tmp (container-local) then copy to /workspace.
    tag: infrastructure
  - title: Benchmark JSON claims do not predict fine-tuning quality
    detail: Qwen 2.5 7B was recommended as trained specifically for structured JSON output. It scored F1=0.572 vs Qwen 3 8B at 0.612. Benchmark JSON compliance does not translate to structured extraction quality after LoRA fine-tuning.
    tag: models
  - title: Qwen 3 thinking tags resolve with sufficient balanced training data
    detail: Early runs had 0% valid JSON from Qwen 3's <think> tags. The balanced combined v1 run achieved 100% valid JSON — the model learned to skip thinking when trained on enough examples. The v2 run (different data mix) brought them back. Inconsistent but manageable with tag stripping.
    tag: models
  - title: One model beats per-source-type models
    detail: Original plan called for separate SLMs per report type. Testing showed combined training consistently outperforms single-source. Each source provides examples of rare edge types the others lack. Per-source models are worse, not better.
    tag: architecture
  - title: LLM augmentation of rare types via Gemini rephrasing works
    detail: 154 rare-type narratives rephrased by Gemini (normalises, learns-from, cares-for) in 4 minutes, 0 failures. Preserves cultural signal while varying phrasing. Better than simple oversampling which repeats identical examples.
    tag: data

artifacts:
  - data/cultural-graph-models/qwen3-8b-cultural-graph-v2-q4.gguf
  - data/cultural-graph-models/Modelfile
  - data/qq/cultural-graph/outputs/training/combined-v2-train.jsonl
  - data/qq/cultural-graph/outputs/training/combined-v2-test.jsonl
  - data/qq/cultural-graph/outputs/training/negative-examples.jsonl
  - data/qq/cultural-graph/outputs/training/augmented-rare-types.jsonl
  - data/qq/cultural-graph/outputs/briefs/cultural-graph-model-deployment-brief.md
  - data/code-review/cultural-graph-model-testing-plan.md
  - scripts/cultural-graph/finetune_runpod.py
  - scripts/cultural-graph/eval_baseline.py
  - scripts/cultural-graph/eval_finetuned.py
  - scripts/cultural-graph/export_training_data.py
  - .claude/plans/cultural-graph/initial-review.md (updated with Section 10 lessons)

depends_on:
  - 07-29-26-positive-observations-emergence.md
  - 07-29-26-schema-constrained-extraction.md
  - 07-30-26-hazard-reports-emergence.md
  - 07-30-26-near-miss-reports-emergence.md
  - 07-30-26-injury-reports-emergence.md

enables:
  - Edge deployment of cultural graph extraction (Ollama GGUF)
  - Cloud deployment via RunPod + Baserow
  - Human review cycle with domain experts (41-narrative sample ready)
  - ONNX cultural signal classifier training
  - Cross-source site profiling
  - Investigation reports processing (blames/silences)
---

# Session: Training Data Preparation (CLOSED)

## Problem

The outputs directory has 14 files from two sessions in a flat structure — emergence Parquet, constrained training data, review samples, classifier data, briefs, and schema mappings all mixed together. As hazard and near-miss sources arrive, this becomes unmanageable. Need to reorganise into a clear directory structure, produce the phase 2 executive brief, and continue preparing training data for the positive observations SLM.

## Todo

- ✅ Reorganise outputs/ into subdirectories — briefs/, training/, review/, analysis/
- ✅ Produce phase 2 executive brief — `outputs/briefs/positive-observations-phase2-brief.md`
- ✅ Update NAS backup to match new directory structure
- ✅ Retry 17 failed emergence pass narratives — 17/17 recovered at maxOutputTokens 8192
- ✅ Run constrained pass on retried 17 — 17/17 valid
- ✅ Rebuild all training Parquet with full 300/300 coverage — 1,775 cultural edges, 4,936 total relationships
- ✅ Convert training Parquet to instruction-tuning JSONL — HuggingFace chat format
- ✅ Create train/test split — 263 train / 37 test, stratified by cultural edge density
- ✅ Select base model — Qwen 3 8B (32GB edge device confirmed, 20-50 tok/s at Q4)
- ✅ Build LoRA fine-tuning script — `scripts/cultural-graph/finetune_runpod.py`
- ✅ Build evaluation script — `scripts/cultural-graph/eval_baseline.py`
- ✅ Run Qwen 3 8B zero-shot baseline — type-level F1=0.468, exact F1=0.041 (2.2s/example on RTX 5090)
- ✅ Fine-tune SLM on RunPod (unbalanced) — F1=0.424, worse than zero-shot due to class imbalance
- ✅ Fine-tune SLM on RunPod (balanced) — F1=0.558, +19% over zero-shot baseline
- ✅ Build combined instruction-tuning JSONL — 1,060 train / 139 test, 4,477 cultural edges, ~1.4M tokens
- ✅ Upload to RunPod and fine-tune (unbalanced combined) — F1=0.484, rare types collapsed again
- ✅ Build balanced combined training set — 6,292 oversampled examples
- ✅ Fine-tune balanced combined — **F1=0.612**, 100% valid JSON, all types predicted
- ✅ Shelve defers-to-by-rank from active schema — military-specific, 8/1200 instances, retained in schema library
- ✅ Fine-tune Llama 3.1 8B (r=32) — F1=0.608, high precision (0.707) but low recall (0.533). Slower to train, no improvement over Qwen.
- ✅ LLM augmentation of rare-type narratives — 154 rephrased via Gemini
- ✅ Negative examples selected — 200 zero-signal short narratives (50 per source)
- ❌ Fine-tune Qwen 2.5 7B — tested, F1=0.572, underperformed Qwen 3 8B. Not pursuing.
- ✅ Export GGUF for Ollama — 16-bit retrain + Q4_K_M export, 4.7GB. Downloaded locally + NAS backup.
- ✅ Local edge test — model runs via Ollama, valid JSON output, 101s first inference on CPU
- ⏸️ Train ONNX cultural signal classifier (deferred — next session)
- ⏸️ Run models on 41 human review narratives (deferred — next session)

## Fine-tuning Results

| Model | Train data | Examples | Valid JSON | Type F1 |
|-------|-----------|----------|-----------|---------|
| Qwen 3 8B zero-shot | — | — | 100% | 0.468 |
| PO-only unbalanced | positive obs | 263 | 95% | 0.424 |
| PO-only balanced | positive obs (oversampled) | 1,879 | 97% | **0.558** |
| Combined unbalanced | all 4 sources | 1,060 | 93% | 0.484 |
| **Combined balanced** | **all 4 sources (oversampled)** | **6,292** | **100%** | **0.612** |
| Llama 3.1 8B balanced | all 4 sources (oversampled) | 6,292 | ? | 0.608 |

**Key findings**:
1. Class balancing via oversampling is consistently more important than data volume. Unbalanced runs collapse rare edge types regardless of how much data they have.
2. 4-bit LoRA training produces valid adapters for GPU inference (RunPod Path B) but LoRA weights are lost during GGUF merge. Edge deployment (Path A) requires 16-bit retrain + 100GB container disk.
3. Qwen 3 8B consistently outperforms Qwen 2.5 7B and Llama 3.1 8B on this task.

## Dependencies

- ✅ All 4 reporter-authored sources processed (1,199 narratives, 5,066 cultural edges)
- ✅ Combined instruction-tuning JSONL built (1,060 train / 139 test)
- ✅ Balanced combined JSONL built (6,292 oversampled train)
- ✅ RunPod workspace at `/workspace/cultural-graph/` with all adapters and scripts

## Appendix: Alternative SLM Candidates

| Model | Params | Q4 size | CPU tok/s | Think tags | JSON quality | Notes |
|-------|--------|---------|-----------|-----------|-------------|-------|
| **Qwen 3 8B** (current) | 8.2B | 5.2 GB | 8–12 | **Yes** | Good | Think tags complicate parsing; can disable with `/no_think` |
| **Qwen 2.5 7B** | 7.6B | 4.7 GB | 8–14 | No | Excellent | Top recommendation — trained for structured/JSON output, cleaner than Qwen 3 |
| **Phi-4 Mini** | 3.8B | 2.4 GB | 25–40 | No | Good | Half the size, 2-3x faster. Strong reasoning. Use instruct variant not reasoning. |
| **NuExtract-1.5** | 3.8B | 2.4 GB | 20–35 | No | Purpose-built | Fine-tuned specifically for text-to-JSON extraction. Worth a zero-shot baseline. |
| **Gemma 3 4B** | 4B | 2.6 GB | 20–35 | No | Good | Already used elsewhere in project. Clean instruction following. |
| **Llama 3.1/3.3 8B** | 8B | 4.9 GB | 8–13 | No | Good | Solid all-rounder, good fine-tune ecosystem. |

**Avoid**: Gemma 4 12B, Phi-4 14B, Mistral NeMo 12B — all push 32GB RAM ceiling.

**Quick win**: Qwen 3 8B thinking can be disabled via `enable_thinking=False` or `/no_think` system prompt prefix — may improve JSON validity without model change.

**Deployment architecture options:**
- **Path A — Edge (company laptop):** 32GB ceiling, potential IT blockers. Target ≤8B models via Ollama.
- **Path B — Cloud (RunPod + self-hosted Baserow):** Safety team uploads CSV to self-hosted Baserow, RunPod pod pulls via API, processes with larger model, pushes results back. No size constraint. Data sovereignty preserved (Baserow is self-hosted). Pragmatic stepping stone while edge deployment is worked out with IT.

**Recommended evaluation order (edge, ≤8B):**
1. Qwen 3 8B + thinking disabled (zero-code fix)
2. Qwen 2.5 7B fine-tune (same LoRA recipe)
3. NuExtract-1.5 zero-shot (no fine-tuning needed)
4. Phi-4 Mini fine-tune (if edge speed matters more than raw accuracy)

**Cloud candidates (RunPod, no size constraint):**
- DeepSeek-R1 14B distill — chain-of-thought reasoning may help harder edge type discriminations
- Llama 3.3 70B — ceiling test, what's the best extraction quality achievable?
- If cloud consistently beats edge by a large margin, Path B becomes the operational architecture

**Evaluation plan (in order):**

| # | Model | Size | Where | Purpose |
|---|-------|------|-------|---------|
| 1 | Qwen 3 8B balanced combined | 8B | RunPod | Current run — baseline for combined data |
| 2 | Qwen 2.5 7B | 7B | RunPod | No thinking tags, trained for JSON output |
| 3 | Llama 3.3 8B | 8B | RunPod | Stability pick, clean fine-tune canvas |
| 4 | DeepSeek-R1 14B distill | 14B | RunPod | Chain-of-thought for harder discriminations |
| 5 | Llama 3.3 70B | 70B | RunPod | Ceiling test — best possible quality |

All use the same balanced combined training set (6,292 examples) and test set (139 examples) for direct comparison. Fine-tune with same LoRA recipe (r=16, alpha=32, lr=5e-5) unless model-specific adjustments are needed.

**Note on base vs instruct:** Gemini recommends fine-tuning base models not instruct. Disagree for our task — we need reliable JSON output, which requires instruction-following behaviour. Training out RLHF alignment would be counterproductive. Use instruct variants throughout.

## Gemini Review Feedback (2026-07-31)

Raw review: `data/code-review/cultural-graph-model-testing-plan.md`

**Actionable items:**

1. **Model selection** — Add Mistral 7B Instruct to the eval plan. Gemini calls its omission "glaring" for structured extraction. Also consider function-calling optimised model variants.

2. **LoRA recipe per model** — Don't use the same hyperparameters for all models. r, alpha, lr should be tuned per model architecture. At minimum, use community-recommended values for each base model rather than a single recipe.

3. **Data augmentation for rare types** — Instead of just oversampling (repeating same examples), apply text transformations (synonym replacement, rephrasing) to rare-type narratives before oversampling. Reduces overfitting on specific phrasings.

4. **Negative examples** — Add narratives with no cultural relationships to the training set. Teaches the model when *not* to extract.

5. **Evaluation granularity** — Current type-level F1 is too coarse. Need per-type F1, error categorisation (hallucinations, misses, type confusion, span errors), and JSON schema adherence beyond just "valid JSON."

6. **Edge latency** — 25-50 seconds per narrative on CPU may be too slow. Define whether this is batch (acceptable) or interactive (unacceptable). Consider hybrid: fast edge model for simple cases, cloud for complex ones.

7. **IT engagement** — Start the security/approval process for edge deployment NOW. Longest lead time item.

**Items we disagree with or already handle:**
- "Pivot immediately from Qwen 3 to Qwen 2.5" — we're finishing the current run to establish a baseline, then testing Qwen 2.5 next. Not a misstep, just sequencing.
- Edge latency concern assumes interactive use — our use case is monthly batch processing of 300 narratives. 25-50s × 300 = 2-4 hours, acceptable.
