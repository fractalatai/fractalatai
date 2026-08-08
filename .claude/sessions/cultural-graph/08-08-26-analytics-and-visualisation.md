---
session: Extended Analytics and Visualisation
status: pending
opened: 2026-08-08
---

# Session: Extended Analytics and Visualisation (PENDING)

## Problem

The pipeline produces SMR tables and funnel plots but lacks several standard analytical tools for institutional comparison: caterpillar plots for ordered site comparison, power analysis to set expectations, sensitivity analysis for robustness, confounder investigation beyond report type, and upstream model drift monitoring. These were identified in Gemini review (2026-08-08) as valuable extensions.

## Todo

- ⬜ Caterpillar plots — SMRs with CIs ordered by value, all sites on one axis. Complement to funnel plots: funnel shows volume effect, caterpillar shows ranking
- ⬜ Power analysis — minimum detectable SMR per site size per composite. "At N=50 narratives, we can detect SMR deviations of X or more." Manages C-suite expectations
- ⬜ Sensitivity analysis — how robust are flags to: different phi estimation methods, baseline rate perturbation, N≥10 threshold changes
- ⬜ Additional confounders — test sector (AUS vs UKD) as a covariate in the GLM. If sector explains significant variance, the SMRs should control for it alongside report type
- ⬜ AI model drift monitoring — compare extraction rates (edges per narrative) across monthly batches. Flag if batch-level rates deviate from historical baseline. Upstream quality gate before reporting
- ⬜ Update SKILL.md with new visualisations and analytics

## Dependencies

- ⬜ Log-scale GLM framework (session: Log-Scale GLM Framework) — sensitivity analysis should test the corrected pipeline
- ⬜ Empirical Bayes shrinkage (session: Empirical Bayes Shrinkage) — caterpillar plots are most valuable with shrunken estimates
- ✅ Funnel plots and temporal trends implemented (session: Statistical Rigour)
- ✅ matplotlib 3.11.1 available
- Gemini review: `data/code-review/cultural-graph-statistical-pipeline.md`
