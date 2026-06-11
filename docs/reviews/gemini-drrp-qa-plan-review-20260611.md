# Gemini Review: DRRP QA Plan

**Date:** 2026-06-11
**Model:** Gemini 2.5 Flash

## Summary

Comprehensive plan endorsed. Key feedback on order, missing methods, and where to start.

## Key Feedback

### 1. Execution order — mostly right, minor tweaks
- Create initial regression tests alongside golden benchmarks (not after)
- Run a preliminary coverage gap analysis early — informs benchmark selection and code review
- Suggested order: Code review → Benchmarks + initial regression tests → Coverage gaps → Classifier disagreements → Gemini spot-check → Human drill-through → Statistical anomalies → Full regression suite

### 2. Missing QA methods
- **Data drift monitoring** — post-deployment, monitor input distributions vs training data
- **Model explainability (SHAP/LIME)** — understand why classifier disagrees, debug misclassifications
- **Performance benchmarking** — measure end-to-end speed and memory for both pipelines
- **Error handling / resilience testing** — malformed input, extremely long provisions, edge-case characters

### 3. Golden benchmarks
- One Act + one SI per family is a good start, translates to dozens-hundreds of provisions
- Aim for 50-100 provisions per family with diverse coverage
- **Critical**: never train on the benchmark — strictly held out
- Periodically refresh/expand as new legislation types emerge
- Focus on generalising fixes to patterns, not specific benchmark text

### 4. 39% disagreement rate — concerning but expected for v1
- Nearly 40% is significant room for improvement
- Expected given domain complexity, limited training data, subtle nuance of "active actor"
- Prioritise high-confidence disagreements for human review (exactly the right approach)
- Categorise error types: misidentified counterparty vs missed implicit actor vs confused dual actors
- Position classifier will be the primary bottleneck — plan for rapid iteration

### 5. Code review scope — comprehensive, minor additions
- Consider reviewing actor label extraction (actors.rs) as a precursor to DRRP
- Check how provision text is prepared/cleaned before DRRP classification
- Review configuration/threshold constants (confidence scores, magic numbers)

### 6. Single most impactful first action
**Code review focusing on `duty_patterns_v2.rs` and `purpose.rs`**

Why: regex patterns and purpose gates are the first line of defence for both pipelines. Fixing fundamental gaps here prevents cascade errors downstream. High leverage — one regex fix can affect thousands of provisions.
