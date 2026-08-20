---
session: Mechanism Classifier
status: closed
opened: 2026-08-19
closed: 2026-08-20
outcome: partial

summary: >
  Built and validated the Stage 1 mechanism classifier across 5 training runs (Qwen 0.6B→1.7B,
  20→18 labels, with/without structured context). Achieved 0.80 macro F1 on OSHA held-out data
  but discovered a fundamental domain gap: US OSHA narratives don't transfer to UK QQ defence
  narratives. The auto-non-SIF gate filters only 12% of events and is marginal over random on
  cross-domain data. Key finding: single-source training data produces single-domain performance.

decisions:
  - what: Merge fall_height + fall_same_level → fall, struck_by + struck_against → struck (20 → 18 labels)
    why: The model couldn't distinguish these semantically similar pairs from narrative text alone (F1 0.39/0.54 and 0.46/0.68). Stage 2 energy analysis is the right place for height/energy discrimination.
    result: Merged labels improved overall macro F1 from 0.742 to 0.810
  - what: Narrative only — no structured context in training
    why: Runs 3+4 showed that including OIICS event titles (as labelled fields or appended text) achieves 0.99 F1 on OSHA but collapses to single-class prediction on QQ. The model learns the OIICS text as a shortcut rather than learning from the narrative.
    result: Run 5 (narrative only) is the only model that generalises to QQ, even though F1 is lower (0.80)
  - what: Differentiated training caps — 1K for NEEDS_ASSESSMENT, 500 for AUTO_NON_SIF
    why: Original 5K cap was overkill for 0.6B LoRA (78K events). Gemini review confirmed 500-1500/class is the sweet spot. Auto-non-SIF classes don't need 5K each to learn "gate this out".
    result: Training set reduced from 78K to 14.8K. Training time ~20 min on RTX 5090.
  - what: Domain gap is the blocker, not model architecture
    why: Run 5 QQ gate analysis showed 88.2% NEEDS_ASSESSMENT rate — gate filters only 12%. SIFp false negative rate (9.4%) is marginally better than random (would be ~11.6%). The model learned OSHA narrative style, not generalisable mechanism patterns.
    result: Stage 1 two-stage approach suspended. Moving to single-model zero-shot experiment.
  - what: Weighted loss (inverse frequency), not oversampling
    why: Gemini review recommended against oversampling — duplicating sparse class examples causes overfitting. Weighted loss penalises rare-class errors more without altering data distribution.
    result: Breathing class (334 examples) achieved 0.97 F1 with weighted loss despite being the smallest class.

metrics:
  run1: { model: "Qwen3-0.6B", labels: 20, macro_f1: 0.742, micro_f1: 0.740, gpu: "RTX 4090", vram_gb: 18.3 }
  run2: { model: "Qwen3-1.7B", labels: 18, macro_f1: 0.810, micro_f1: 0.815, gpu: "RTX 4090", vram_gb: 16, gate_accuracy: 0.925, false_neg_pct: 4.2, false_pos_pct: 3.2 }
  run3: { model: "Qwen3-1.7B", labels: 18, macro_f1: 0.990, note: "BROKEN — labelled context shortcut, 95% breathing on QQ" }
  run4: { model: "Qwen3-1.7B", labels: 18, macro_f1: 0.998, note: "BROKEN — appended text same failure as run3" }
  run5: { model: "Qwen3-1.7B", labels: 18, macro_f1: 0.799, micro_f1: 0.803, gpu: "RTX 5090", vram_gb: 16, qq_gate_needs_assessment_pct: 88.2, qq_sifp_captured_pct: 90.6, qq_sifp_missed_pct: 9.4, qq_gate_marginal_over_random: 9 }
  training_data: { train: 13351, val: 1483, total_labels: 18, osha_events: 1579596, msha_supplements: 2102 }
  training_time: { run5_minutes: 20, gpu: "RTX 5090 32GB", batch_size: 16 }

lessons:
  - title: Structured context in training creates format-dependent shortcuts
    detail: >
      Including OIICS event titles as labelled fields (Hazard category: ...) or as appended plain text
      both produced 0.99+ F1 on OSHA validation but collapsed to single-class prediction (95% breathing)
      on QQ data. The model learned to read the OIICS text — which is effectively the answer — rather than
      learning mechanism patterns from the narrative. Any supplementary text that correlates strongly with
      the label will be exploited as a shortcut.
    tag: models
  - title: Single-source training data produces single-domain performance
    detail: >
      Training exclusively on US OSHA narratives (even with MSHA mining supplements) doesn't transfer to
      UK defence/industrial narratives. Different vocabulary ("manual handling" vs "overexertion"),
      different reporting culture, different incident mix. The auto-non-SIF gate's marginal-over-random
      performance on QQ data confirms the model learned OSHA style, not generalisable patterns.
    tag: data
  - title: 78K training events is overkill for LoRA fine-tuning a 0.6B model
    detail: >
      Original 5K/class cap produced 78K events. Gemini review identified 500-1500/class as the sweet spot
      for LoRA. Reduced to 14.8K with no loss of accuracy on OSHA held-out (0.80 vs 0.81). More data just
      means longer training and more autocoder noise memorised.
    tag: models
  - title: Qwen 1.7B OOMs at batch 16 on RTX 4090 (24GB) but fits on RTX 5090 (32GB)
    detail: >
      LoRA fine-tune of Qwen3-1.7B for sequence classification used 18.3GB on 0.6B and OOM'd at batch 16
      on 1.7B (24GB 4090). Batch 8 + grad_accum 2 fixed it (16GB). RTX 5090 (32GB) runs batch 16 comfortably.
      Always confirm GPU VRAM before setting batch size.
    tag: infrastructure
  - title: RunPod pip packages are ephemeral — reinstall on every pod start
    detail: >
      System Python packages installed via pip3 --break-system-packages are lost when the pod stops/restarts.
      Always verify packages before running, not just GPU. The workspace (/workspace) persists but the
      system environment doesn't.
    tag: infrastructure
  - title: Always namespace model runs on /workspace
    detail: >
      Runs 3 and 4 overwrote /workspace/sif/models/mechanism-classifier/, destroying run 2's model.
      Fixed by adding --run argument to scripts: mechanism-run5/ instead of mechanism-classifier/.
      Each run gets its own directory. Never overwrite.
    tag: tooling
  - title: OIICS event_code_pred labels are BLS autocoder predictions, not human annotations
    detail: >
      The training labels come from another ML model (BLS SOII autocoder). This creates an inherent
      ceiling — our model can't exceed the autocoder's accuracy. Documented limitation, can't avoid
      at 1.6M scale. The QQ correlation test provides a different signal from a different labelling process.
    tag: data
  - title: bf16 not fp16 for modern GPUs
    detail: >
      fp16 training caused NotImplementedError on RTX 4090/5090 ("_amp_foreach_non_finite_check_and_unscale_cuda
      not implemented for BFloat16"). These GPUs natively use bfloat16. Always set bf16=True.
    tag: infrastructure

artifacts:
  - scripts/sif/finetune_mechanism.py
  - scripts/sif/export_and_infer.py
  - scripts/sif/extract_training_data.py
  - scripts/sif/build_benchmark.py
  - data/sif/training/train.parquet
  - data/sif/training/val.parquet
  - data/sif/taxonomy/classifier-labels.json
  - data/sif/taxonomy/oiics-to-mechanism-mapping.json
  - data/sif/benchmarks/osha_benchmark.json
  - data/sif/benchmarks/qq_benchmark.json
  - data/sif/models/mechanism-classifier-v1/training_meta.json
  - data/sif/models/mechanism-classifier-v1/classification_report.txt
  - data/code-review/sif-training-plan-review.md
  - data/code-review/sif-training-data-strategy-review.md

depends_on:
  - 08-19-26-taxonomy-and-data.md
  - 08-19-26-sipmath-engine.md

enables:
  - Zero-shot single-model experiment (mechanism + energy in one pass)
  - Future multi-source training data acquisition (RIDDOR, Safe Work Australia)
  - Customer-specific fine-tuning workflow
---

# Session: Mechanism Classifier (CLOSED)

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
- ⏸️ Single-model approach — moved to new zero-shot experiment session
- ✅ QQ correlation test — run 5 completed, gate marginal over random (domain gap)
- ⏸️ ONNX export — deferred until model is production-worthy
- ⏸️ Rust implementation + CLI — deferred until model is production-worthy

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
