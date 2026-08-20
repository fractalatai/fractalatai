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
- ✅ Fine-tune run 1: Qwen 3 0.6B / 20 labels — macro F1 = 0.742. Fall and struck pairs confused.
- ✅ Fine-tune run 2: Qwen 3 1.7B / 18 labels (merged fall, struck) — macro F1 = 0.810. Gate accuracy 92.5%.
- ⬜ Fine-tune run 3: single-model approach (mechanism + energy in one pass) — Gemini suggestion, compare to two-stage
- ⬜ Export best model to ONNX
- ⬜ Evaluate against QQ correlation test (bulk inference on RunPod)
- ⬜ Implement `fractalaw-ai::sif::classifier` — ONNX inference wrapper
- ⬜ Add auto-non-SIF gate logic
- ⬜ CLI command `sif classify` — single event or batch

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

## Training Run Results

### Run 1: Qwen 3 0.6B / 20 labels

- Model: Qwen/Qwen3-0.6B, LoRA r=16 α=32, 3 epochs, lr=2e-4, batch 16, bf16
- Labels: 20 (15 NEEDS_ASSESSMENT + 5 AUTO_NON_SIF) with fall_height and fall_same_level separate, struck_by and struck_against separate
- Training: 15,151 train + 1,683 val
- **Macro F1: 0.742** | Micro F1: 0.740
- Strong: breathing 0.985, radiation_noise 0.970, thermal 0.966, electrical 0.965
- Weak: abrasion 0.330, fall_height 0.389, struck_by 0.459, fall_same_level 0.544
- **Decision:** Merge confusable pairs (fall_height + fall_same_level → fall, struck_by + struck_against → struck), try 1.7B

### Run 2: Qwen 3 1.7B / 18 labels (merged)

- Model: Qwen/Qwen3-1.7B, LoRA r=16 α=32, 3 epochs, lr=2e-4, batch 8 + grad_accum 2, bf16
- Labels: 18 (13 NEEDS_ASSESSMENT + 5 AUTO_NON_SIF) with merged fall and struck
- Training: 13,351 train + 1,483 val
- **Macro F1: 0.810** | Micro F1: 0.815
- Strong: electrical 0.975, chemical 0.971, breathing 0.970, radiation_noise 0.969
- Weak: abrasion 0.384, struck 0.449, caught_in 0.585
- Note: 0.6B OOM'd at batch 16 on 4090 24GB; batch 8 + grad_accum 2 fixed it (16GB VRAM)

**SIF gate analysis (the metric that matters for safety):**
- Gate accuracy: 92.5%
- False negatives (SIF→non-SIF): 4.2% (63/1483) — struck→abrasion (19), fall→slip (13) are main leaks
- False positives (non-SIF→SIF): 3.2% (48/1483) — harmless, just unnecessary Stage 2 analysis
- Within-NEEDS_ASSESSMENT confusion: fire↔explosion (36), struck↔caught_in↔structural_collapse — all still go to Stage 2, so no safety impact

### Run 3: Qwen 3 1.7B / 18 labels + labelled structured context

- Same as run 2 but with `Hazard category:` and `Event type:` prepended as labelled fields
- **Macro F1: 0.990** on OSHA val — but illusory
- **QQ inference: 95.2% predicted as "breathing"** — model learned the OIICS field label format as a shortcut. Without it, collapses to one class.
- **Verdict: BROKEN.** Structured context as labelled fields doesn't generalise.

### Run 4: Qwen 3 1.7B / 18 labels + appended text (no labels)

- Same as run 3 but OIICS text appended as plain text, no `Hazard category:` prefix
- **Macro F1: 0.998** on OSHA val — still illusory
- **QQ inference: 95.2% predicted as "breathing"** — same failure. The appended OIICS text (event title, source, nature, body part) is too strong a signal. Model learns it instead of narrative.
- **Verdict: BROKEN.** Any OIICS text in training dominates — model can't function without it.

### Run 5: Qwen 3 1.7B / 18 labels / narrative only (revert to run 2 approach)

- Narrative only, no structured context. Namespaced as `mechanism-run5` on workspace.
- Batch 16 on RTX 5090 (32GB).
- **Macro F1: 0.799** on OSHA val
- **QQ inference results:**
  - Gate: 88.2% NEEDS_ASSESSMENT, 11.8% AUTO_NON_SIF
  - Mechanism distribution is diverse and plausible (no single-class collapse)
  - SIFp events sent to Stage 2: 90.6% (37/395 missed = 9.4% false negative)
  - Not-SIFp events gated out: 12.3% (288/2349)
  - **Random baseline comparison:** random would miss ~46 SIFp vs actual 37 — gate is only marginally better than random
- **Verdict: WORKS but marginal value.** The gate filters 12% of events, mostly overexertion and slips. But 88% of non-SIF events still pass through. The 9.4% false negative rate on SIFp is a safety concern.

### Key Finding: Domain Gap Kills Stage 1

The fundamental problem is not the model architecture or the label set — it's the training data. OSHA narratives are US workplace English. QQ narratives are UK defence/industrial English. The model learned to classify American incident reports, not British ones.

Evidence:
- QQ "Manual Handling" (123 events) maps to OSHA "Overexertion" — but the narratives read differently. "I was carrying radio equipment 500 metres" vs "employee was lifting boxes and felt a pop".
- The model correctly classifies OSHA-style text (0.80 F1 on held-out OSHA) but doesn't transfer to QQ.
- The gate's marginal-over-random performance confirms the model isn't learning generalisable mechanism patterns — it's learning OSHA narrative style.

**For Stage 1 to add value, the training data needs a thorough mix of narrative styles**: US/UK/AU, multiple industries (construction, mining, defence, manufacturing, logistics), multiple reporting cultures. Single-source training produces single-domain performance.

### What Works and Should Be Retained

1. **SIPmath engine** — solid, independently valuable, no changes needed
2. **Label set** (18 labels, merged pairs) — the right granularity
3. **Training pipeline** — extraction, MSHA supplements, stratified sampling, LoRA fine-tuning all work
4. **Model architecture** — 1.7B LoRA fits on edge (16GB VRAM), trains in ~20 min
5. **Taxonomy** — ICD-11 Chapter 23, ICECI mapping, severity scale, OIICS mapping all valid
6. **Supplementary data sources** — MSHA, PHMSA, NFIRS, etc. documented and ready
7. **QQ ingest** — DuckDB schema, join pipeline, SIFp labels
8. **P(death) severity scale** — AIS-based, principled
9. **Chance P vs Outcome P** — foundational conceptual frame

### What Needs to Change

- **Training data diversity** — need UK (RIDDOR narratives if accessible), Australian (Safe Work Australia), multi-industry sources
- **Or skip Stage 1** — go straight to single-model approach (mechanism + energy in one pass), where the model can be trained on a different task that may generalise better
- **Customer-specific fine-tuning** — the base model handles one domain; each customer needs adaptation

### RunPod workspace (namespaced)

All artefacts persist on `/workspace/sif/` (network volume, survives pod stop):
- `/workspace/sif/data/` — training parquet, val parquet, labels, benchmarks, qq_benchmark
- `/workspace/sif/models/mechanism-run5/` — run 5 LoRA adapter (production candidate)
- `/workspace/sif/models/mechanism-run5-merged/` — merged model for inference
- `/workspace/sif/models/mechanism-classifier/` — run 4 (broken, can clean up)
- `/workspace/sif/models/mechanism-classifier-merged/` — run 4 merged (broken, can clean up)
- `/workspace/sif/output/run5/` — QQ inference results
- `/workspace/sif/scripts/` — finetune_mechanism.py, export_and_infer.py
