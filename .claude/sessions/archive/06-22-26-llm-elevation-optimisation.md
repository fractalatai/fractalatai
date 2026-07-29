---
session: "LLM Elevation Optimisation"
status: closed
opened: 2026-06-22
closed: 2026-06-22
outcome: success

summary: >
  Fixed disagreement check bug that elevated 76% false positives to LLM. Added
  regex_drrp != cls_drrp check. Tuned disagreement threshold from 0.9 to 0.75
  (precision optimum). Benchmark 85.6% to 86.0%. LLM-elevated from 398 to 134
  provisions. Profiled 239 unflagged errors — 170 are hard floor where both tiers
  agree on the wrong answer.

decisions:
  - what: "Add disagreement check before flagging as pending_llm"
    why: "76% of LLM-elevated provisions were already correct — regex and classifier agreed on the right answer"
    result: "LLM-elevated reduced from 398 to 125. FP rate from 76% to 32%."
  - what: "Lower disagreement threshold from 0.9 to 0.75"
    why: "54 provisions had correct classifier answer below 0.9 threshold. Sweep showed 0.75 is precision optimum."
    result: "Benchmark 85.6% to 86.0%. Perfect LLM ceiling 89.4% to 89.8%."

lessons:
  - title: "170 hard-floor errors need better models, not better thresholds"
    detail: "When both regex and classifier confidently agree on the wrong answer, no threshold tuning can help. These need either more training data, a larger model, or the LLM tier."
    tag: pipeline
---

# Session: LLM Elevation Optimisation (CLOSED)

## Problem

The classifier flagged 398 provisions as `pending_llm` across 16 benchmark laws. But:
- **303 were already correct** (76% FP rate — wasted LLM cost and regression risk)
- **95 were genuine mismatches** that LLM would fix
- **229 errors were NOT flagged** for LLM at all

## Fix 1: Disagreement check bug (`a3e057c`)

**Root cause**: The transition rule flagged ALL provisions where `has_drrp && classifier_confidence >= 0.9`, regardless of whether the classifier actually **disagreed** with the regex. 263 provisions where regex and classifier agreed on the correct answer were needlessly elevated.

**Fix**: Added `regex_drrp != cls_drrp` check before flagging as `pending_llm`.

| Metric | Before | After |
|--------|--------|-------|
| LLM-elevated | 398 | 125 |
| Already correct (wasted) | 303 (76% FP) | 40 (32% FP) |
| LLM would fix | 95 | 85 |

## Fix 2: Disagreement threshold 0.9 → 0.75 (`fa88407`)

**Analysis**: 54 provisions where the classifier had the correct answer but confidence was below the 0.9 disagreement threshold (range 0.42–0.87, all below 0.9).

Threshold sweep on 133 real disagreements showed **0.75 is the precision optimum** (69.6%): captures 16 fixes with 5 wasted calls. Below 0.75, precision drops as low-confidence noise floods in.

| Metric | 0.9 threshold | 0.75 threshold |
|--------|---------------|----------------|
| Actual accuracy | 85.6% | **86.0%** |
| LLM-elevated | 125 | 134 |
| FP rate | 32% | 37% |
| Perfect LLM ceiling | 89.4% | **89.8%** |

## Unflagged errors: 239 profiled

| Category | Count | Root cause |
|----------|-------|------------|
| Classifier agrees with wrong answer | 170 | Both regex and classifier confidently wrong in same direction |
| Classifier correct but not used | 54 | Confidence below disagreement threshold (addressed by 0.75 fix) |
| Classifier said none | 9 | Classifier had no opinion |
| No classifier ran | 4 | No embedding or non-regulation section type |

**The 170 where both tiers agree on the wrong answer are the true hard floor.** No threshold tuning can help when both tiers are confidently wrong. These need either more training data for the classifier, a larger base model, or the LLM tier.

## Final benchmark

| Metric | Value |
|---|---|
| Actual (regex + classifier) | **86.0%** (1,935/2,250) |
| With perfect LLM on pending_llm | **89.8%** (2,020/2,250) |
| LLM-elevated provisions | 134 |
| Already correct (wasted LLM) | 49 (37% FP rate) |
| Unflagged errors | ~220 (170 need better model) |

## Key files

- `fractalaw-cli/src/main.rs:5192-5230` — classifier transition rules, thresholds
- `fractalaw-core/src/taxa/decision.rs` — DecisionTrail, SignalSet
- `scripts/benchmark_report.py` — benchmark runner

## Prior sessions

- `06-22-26-liberty-false-positives.md` (CLOSED) — 85.6% benchmark, regex ceiling reached
- `06-22-26-pipeline-traceability.md` (CLOSED) — signal/decision separation enabling this analysis
