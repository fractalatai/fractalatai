---
session: Mechanism Classifier
status: active
opened: 2026-08-19
---

# Session: Mechanism Classifier (ACTIVE)

## Problem

Stage 1 of the SIF classifier: single-label text classification from incident narratives to one of 20 mechanism labels. Fine-tune Qwen 3 0.6B with LoRA, export to ONNX for edge inference. Auto-non-SIF gate for low-energy mechanisms. Target macro F1 >= 0.85.

## Todo

- ✅ Finalise classifier label set: 20 labels (15 NEEDS_ASSESSMENT + 5 AUTO_NON_SIF)
- ❌ Extract stratified training data v1: 78K events at 5K cap — WRONG, over-sized and imbalanced
- ✅ Supplementary real data: MSHA (274K mine accidents) + OSHA SIR (79K severe injuries) — no synthetic needed
- ✅ Build test sets: OSHA held-out 1,940 + QQ correlation test 2,744
- ✅ Training data v2: 15,151 train + 1,683 val (stratified 90/10), MSHA merged, pre-processed, instruction format
- ⬜ Fine-tune Qwen 3 0.6B on RunPod — LoRA, weighted loss, instruction-tuning format. Inference on QQ data also on RunPod (bulk).
- ⬜ Export to ONNX
- ⬜ Evaluate against OSHA held-out (macro F1 by mechanism) + QQ correlation test
- ⬜ Implement `fractalaw-ai::sif::classifier` — ONNX inference wrapper
- ⬜ Add auto-non-SIF gate logic
- ⬜ CLI command `sif classify` — single event or batch
- ⬜ If 0.6B insufficient, try 1.7B and document trade-off

## Dependencies

- ✅ S2: OSHA 1.6M rows with OIICS event codes (training data), QQ 2,747 events with SIFp (correlation test only)
- ✅ S2: OIICS→mechanism mapping table, ICD-11 taxonomy
- ⬜ RunPod access for fine-tuning

## Training Data Strategy Evolution

### v1 (abandoned): Uniform 5K cap across all classes

Initial approach: reservoir-sample 5K events per class from OSHA = 78K total.

**Problems identified:**
- 25K events (32%) wasted on AUTO_NON_SIF classes that just need to be gated out
- Sparse SIF classes (breathing=40, pressure=250) drowned at 0.05-0.3% of training set
- 78K is overkill for LoRA fine-tuning a 0.6B model — diminishing returns beyond ~15K
- More data = more BLS autocoder noise memorised

### v2 (current): Differentiated caps + MSHA supplement + pre-processing

Informed by two Gemini reviews (`data/code-review/sif-training-plan-review.md`, `data/code-review/sif-training-data-strategy-review.md`).

**Target training set: ~14.5K events**

| Class type | Classes | Cap | Rationale |
|---|---|---|---|
| NEEDS_ASSESSMENT | 15 | 1,000 | These matter — model must discriminate between them |
| AUTO_NON_SIF | 5 | 500 | Just enough to learn "gate this out", not enough to dominate |

**MSHA supplement rules:**
- Don't dump all MSHA data in — cap MSHA contribution to avoid mining-domain dominance
- For each sparse class: use all OSHA examples + up to ~1,000 MSHA to fill the cap
- Example: explosion = 461 OSHA + ~539 MSHA = 1,000 (not 461 + 1,421 = 1,882)

**Pre-processing (apply to ALL sources):**
- Lower-case everything (MSHA is ALL CAPS)
- Strip `[REDACTED]` markers from OSHA
- Normalise whitespace
- No truncation — let the tokenizer handle sequence length

**Training approach (from Gemini review):**
- **LoRA** fine-tune, not full fine-tune — data-efficient, prevents overfitting
- **Weighted loss** (inverse frequency) — not oversampling, which overfits on duplicate sparse examples
- **Instruction-tuning format** — prepend "Classify the following workplace incident narrative. The mechanism of injury is:" to every input
- **Stratified 90/10 train/val split** — preserves class proportions for early stopping
- **Macro F1** not micro — essential given class imbalance

**Known limitations (accepted):**
- Training labels are BLS OIICS autocoder predictions (ML-on-ML). Model ceiling = autocoder accuracy. Can't avoid at scale — OIICS codes are the only mechanism labels on 1.6M OSHA records.
- MSHA narratives are mining-specific (384-char, jargon). Pre-processing helps but domain shift exists.
- No human-annotated gold test set. OSHA held-out shares the autocoder label source. QQ is a correlation test, not ground truth. A future domain-expert gold set would strengthen evaluation.

### Test data strategy

**Two test sets with different purposes — do not conflate:**

| Test Set | Size | Labels | Purpose | How to Use |
|---|---|---|---|---|
| OSHA held-out | 1,940 | OIICS autocoder | Mechanism classification accuracy | Macro F1, per-class P/R, confusion matrix. Measures how well the model replicates the OIICS classification. |
| QQ correlation | 2,744 | Human SIFp (subjective) | Classifier–human correlation | NOT scored as accuracy. Explore agreement and disagreement patterns. The classifier uses physics-based energy assessment; humans use subjective judgement. Differences are findings, not errors. |

**QQ correlation test is explicitly NOT gold standard** because:
- Human SIFp labels have ~65% inter-rater agreement (Hallowell & Spencer 2024)
- The whole point of the classifier is that humans are subjective
- Interesting findings will be in the disagreements: where the classifier flags SIF potential that humans missed (energy was present but outcome was lucky), or where humans flag SIF that the classifier doesn't see (humans picking up on context the narrative doesn't capture)

**Future enhancement:** A 1-2K event human-annotated gold set (50-100 per class, domain expert curated) would provide a true accuracy baseline. Currently not available.
