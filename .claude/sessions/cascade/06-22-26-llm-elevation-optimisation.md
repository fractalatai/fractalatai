# Session: LLM Elevation Optimisation (PENDING)

## Problem

The classifier flags 398 provisions as `pending_llm` across the 16 benchmark laws. But:

- **303 are already correct** — LLM call is wasted cost and a regression risk
- **95 are genuine mismatches** that LLM would fix (assuming perfect LLM → 89.8%)
- **229 errors are NOT flagged** for LLM — the elevation logic misses them entirely

This is a false-positive / false-negative problem in the LLM elevation decision.

## False positives: 303 unnecessary LLM elevations

These provisions are `pending_llm` but the current pipeline answer already matches gold. The classifier flagged them because:
- Disagreement with regex at ≥0.9 confidence (classifier says X, regex says Y, but regex was right)
- Both-modals detected (ambiguity signal fired, but the regex answer was correct)
- Low-confidence gap fill (classifier filled a gap but with <0.7 confidence, so flagged — but the fill was correct)

**Question**: Can the elevation threshold be tuned to reduce unnecessary LLM calls? E.g.:
- Raise disagreement threshold from 0.9 → 0.95?
- Skip both-modals flag when regex confidence is high?
- Accept low-confidence gap fills when they agree with the regex tier?

## False negatives: 229 unflagged errors

These provisions have wrong DRRP classification but are NOT flagged for LLM. Breakdown from benchmark:

| Error type | Count | Why unflagged |
|---|---|---|
| none→Liberty FP (classifier) | ~51 | Classifier confidently gap-filled above 0.7 threshold |
| none→Obligation FP (classifier) | ~36 | Same — confident but wrong |
| Liberty→Obligation | ~43 | Regex confident, classifier agrees or doesn't trigger disagreement |
| Liberty→none | ~52 | No signal at all, or purpose-gated — nothing to flag |
| Obligation→none | ~47 | Similar — no regex signal, classifier didn't fill |

**Question**: What new signals could flag these for LLM?
- Classifier confidence in a narrow band (0.7-0.8) → flag as uncertain?
- Provisions with zero actors but DRRP classification → suspicious, flag?
- Provisions where regex and classifier agree on the same wrong answer → need external signal

## Benchmark reference

| Metric | Value |
|---|---|
| Actual (regex + classifier) | 85.6% (1,925/2,250) |
| With perfect LLM on pending_llm | 89.8% (2,021/2,250) |
| LLM-elevated provisions | 398 |
| Already correct (wasted LLM) | 303 (76% false positive rate) |
| Errors not flagged | 229 |

## Investigation plan

1. Profile the 303 false positives — which elevation trigger fired (disagreement vs both-modals vs low-confidence)?
2. Test threshold adjustments on the 398: would raising thresholds reduce false positives without losing the 95 genuine fixes?
3. Profile the 229 false negatives — what do they have in common? Can the SignalSet/DecisionTrail reveal a pattern?
4. Prototype a confidence-band flag: classifier confidence 0.7-0.8 → `pending_llm` instead of accepting

## Key files

- `fractalaw-cli/src/main.rs:5066-5114` — classifier transition rules, thresholds
- `fractalaw-core/src/taxa/decision.rs` — DecisionTrail, SignalSet
- `data/benchmark_trace.json` — full trace for investigation
- `scripts/benchmark_report.py` — benchmark runner

## Prior sessions

- `06-22-26-liberty-false-positives.md` (CLOSED) — 85.6% benchmark, regex ceiling reached
- `06-22-26-pipeline-traceability.md` (CLOSED) — signal/decision separation enabling this analysis
