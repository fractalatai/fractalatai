# Session: Position Classifier Quality (PENDING)

## Problem

The position classifier is confidently wrong on benchmark provisions. HSWA s.2(1) — "It shall be the duty of every employer to ensure..." — has `Org: Employer` classified as `mentioned` instead of `active`. The classifier predicts `other@0.99` overriding the correct regex signal `active@0.30`.

This is visible in sertantai's governed filter: only 1 duty came through for HSWA (out of hundreds) because most actors are misclassified as `mentioned`.

## Root cause chain

1. Regex correctly identifies `Org: Employer` as `active` and `Ind: Employee` as `counterparty` — but at low confidence (0.30)
2. Position classifier (v1) predicts `other` with high confidence (0.92-0.99)
3. The cascade picks the classifier result over regex because classifier tier > regex tier
4. `other` maps to `mentioned` in the final output
5. Sertantai's governed filter correctly excludes `mentioned` actors from duty display

## Immediate issue: benchmark laws

HSWA (`UK_ukpga_1974_37`) is a benchmark law with `is_benchmark = true`. It should have LLM-validated positions (agentic tier), not classifier-overridden ones. The `extraction_method` shows `agentic` but the reason trail shows the classifier was the last to touch positions.

**Question**: how did the classifier override agentic-tier positions on a benchmark law? The source-tier protection should prevent this. Need to investigate whether the benchmark data was corrupted by the accidental re-validation earlier today.

## Scope

### 1. Benchmark restoration
- Check if the 20 benchmark laws have correct actor positions or if they were all corrupted
- Restore from gold standard if needed
- Verify source-tier protection actually prevents position classifier from overriding agentic

### 2. Position classifier v1 quality
- Evaluate false-`other` rate across the corpus
- The classifier was trained on a small dataset — may need retraining with better position labels
- Known issue: `other` is a catch-all that absorbs ambiguous cases

### 3. Cascade logic for positions
- Currently: regex position (low confidence) → classifier position (high confidence) → LLM position (agentic)
- The classifier shouldn't override regex when regex is correct — confidence scores are miscalibrated
- Consider: should position even go through the classifier, or jump straight from regex to LLM for uncertain cases?

### 4. Impact on sertantai
- Sertantai filters on `position = 'active'` for governed duties
- Any law where active actors are misclassified as `mentioned` will show 0 duties
- This is a systemic quality issue, not a one-off

## Key files

- `crates/fractalaw-core/src/taxa/` — regex actor extraction, position assignment
- `crates/fractalaw-ai/src/position_classifier.rs` — position classifier
- `scripts/retrain_drrp_classifier.py` — training pipeline
- `data/position_classifier_v1.pkl` — trained model weights
- `.claude/sessions/cascade/06-11-26-position-classifier.md` — original training session
