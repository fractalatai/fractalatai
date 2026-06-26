# Session: Position Classifier Quality (ACTIVE)

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

## Findings (2026-06-26)

### 1. Benchmark gold standard confirms the bug
HSWA s.2(1) gold: `Employer=active, Employee=counterparty`. Live Postgres: both `mentioned`. The benchmark data was correct — the live classification is wrong.

### 2. The cascade does NOT elevate position disagreements to LLM
In `cmd_taxa_classify` (taxa.rs:2858-2863):
```rust
// When classifier disagrees with "other", map to "mentioned"
let final_pos = if !agrees && cls_pos == "other" {
    "mentioned".to_string()
} else {
    regex_pos.clone()
};
```
When classifier says `other` and disagrees with regex, it **silently overrides to `mentioned`**. No LLM elevation. No flagging for review. Just overwrite.

### 3. The position classifier is systemically wrong — corpus-wide

| Metric | Count |
|--------|-------|
| Regex said `active`, classifier overrode to `mentioned` | **51,523** |
| Regex said `counterparty`, classifier overrode to `mentioned` | **18,073** |
| Total `mentioned` from classifier override | **72,187 / 72,312** (99.8%) |
| Correctly `active` (survived classifier) | 40,499 |

The position classifier v1 predicts `other` for nearly everything. It's worse than useless — it's actively destroying correct regex signals. Virtually every `mentioned` classification in the corpus is a false override.

### 4. Impact on sertantai
Sertantai filters duties by `position = 'active'` for governed actors. With 51K active actors misclassified as mentioned:
- HSWA shows 1 duty instead of hundreds
- The entire QQ corpus is underreporting duties by ~55%

### Fix options

**Option A: Remove the classifier override (immediate)**
Change the cascade logic: when classifier says `other`, keep the regex position instead of overriding to `mentioned`. This restores 51K+ correct positions immediately. No retraining needed.

**Option B: Flag disagreements for LLM review**
When regex and classifier disagree, flag as `pending_llm` instead of silently overriding. LLM adjudicates. More accurate but slower and costs API calls.

**Option C: Retrain the position classifier**
Fix the training data and retrain. The v1 model was trained on a small dataset with noisy labels. But this doesn't fix the 70K already-classified provisions.

**Recommended: A then C.** Remove the override now (fixes the corpus), retrain later (improves future classifications).

### 5. Position determines whether DRRP applies to an actor

The DRRP type (Obligation/Liberty) and actor position (active/counterparty/mentioned) are coupled — not independent:

| DRRP Type | Position | Legal meaning | Example |
|-----------|----------|---------------|---------|
| Obligation | **active** | Bears the duty | Employer *shall ensure* safety |
| Obligation | **counterparty** | Holds a claim against the duty | Employee *is owed* the duty |
| Liberty | **active** | Exercises the liberty/permission | Inspector *may enter* premises |
| Liberty | **counterparty** | Subject to the liberty | Occupier must *permit entry* |
| — | **beneficiary** | Benefits without a direct legal relation | Public benefits from safety duty |
| — | **mentioned** | **No legal relation — referenced only** | "as defined in the Act" |

Rights and Powers are not separate DRRP types — they emerge from the combination of actor type + position:
- Government actor + active Liberty = **Power** (e.g. HSE *may* prosecute)
- Non-government actor + counterparty Obligation = **Right** (claim against the duty-holder)

A `mentioned` actor has **no DRRP relationship**. When the position classifier overrides `active→mentioned`, it **nullifies the entire DRRP classification for that actor**. An Obligation with no active actor is an orphan duty with no holder.

The DRRP benchmark was passing on a technicality: the type field said `Obligation`, but the actor's position was set to `mentioned` — so the obligation has no holder. **The benchmark measured the DRRP type label, not the complete legal relation (type + actor + position).**

Sertantai correctly filters on `position = 'active'` because only active actors bear duties. The 51,523 overridden actors represent 51,523 duties that exist in name only — no holder assigned.

### 6. Why did benchmarks show >80% accuracy?

The benchmark report (`scripts/benchmark_report.py`) measures **DRRP type** accuracy (Obligation/Liberty/none) — NOT position accuracy. The >80% metric was for DRRP classification, which IS working. Position errors (active→mentioned) don't affect the DRRP metric at all. The benchmark never tested position quality.

The benchmark does compute a position confusion matrix (`pos_confusion`), but it was never the headline metric. If we had looked at the position confusion matrix, we'd have seen the massive active→mentioned drift.

### 6. What rules elevate to LLM?

DRRP classifier elevation rules (taxa.rs:2599-2641):
- **Gap fill** (regex=none, classifier=DRRP, confidence ≥ 0.7): classifier wins → `extraction_method = "classifier"`
- **Weak gap** (regex=none, classifier=DRRP, confidence < 0.7): flag → `pending_llm`
- **DRRP disagreement** (regex=X, classifier=Y, confidence ≥ 0.75): flag → `pending_llm`
- **Both modals** (obligation + enabling modal words): flag → `pending_llm`

**Position classifier has NO elevation rules.** When the position classifier disagrees with regex, it silently overrides to `mentioned` (taxa.rs:2858-2863). There's no `pending_llm` flagging, no LLM adjudication, no disagreement tracking for positions. This is the fundamental gap — DRRP has a principled disagreement→LLM path, positions don't.

## Key files

- `crates/fractalaw-cli/src/commands/taxa.rs:2858` — the override logic (the bug)
- `crates/fractalaw-core/src/taxa/` — regex actor extraction, position assignment
- `crates/fractalaw-ai/src/position_classifier.rs` — position classifier
- `scripts/retrain_drrp_classifier.py` — training pipeline
- `data/position_classifier_v1.pkl` — trained model weights
- `.claude/sessions/cascade/06-11-26-position-classifier.md` — original training session
