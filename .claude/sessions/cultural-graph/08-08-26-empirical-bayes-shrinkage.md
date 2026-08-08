---
session: Empirical Bayes Shrinkage
status: pending
opened: 2026-08-08
---

# Session: Empirical Bayes Shrinkage (PENDING)

## Problem

Small sites (N=10-30) have noisy SMR point estimates — a site with 15 narratives and SMR=2.5 is probably less extreme than it looks. Without shrinkage, C-suite consumers may overreact to extreme small-site SMRs. Gemini review (2026-08-08) identified this as "the most critical missing piece" for fair institutional comparison. The same approach is used in CMS Hospital Compare and NHS hospital profiling.

## Todo

- ⬜ Implement gamma-Poisson empirical Bayes shrinkage: `SMR_shrunk = (O + alpha) / (E + beta)`, estimate alpha/beta from org-wide data
- ⬜ Compare shrunken vs raw SMRs — quantify how much small sites move toward 1.0
- ⬜ Evaluate whether N≥10 filter is still needed with shrinkage (shrinkage may handle small sites better than exclusion)
- ⬜ Integrate shrunken SMRs into dashboard, funnel plots, and flag computation
- ⬜ Update FDR to use p-values from shrunken estimates
- ⬜ Evaluate upgrade path: empirical Bayes → mixed-effects quasi-Poisson GLM (`site` as random intercept) if EB is insufficient
- ⬜ Update SKILL.md and briefs

## Dependencies

- ⬜ Log-scale GLM framework (session: Log-Scale GLM Framework) — shrinkage should build on corrected CIs/p-values, not the current inconsistent chain
- ✅ SMR + quasi-Poisson + FDR pipeline implemented (session: Statistical Rigour)
- ✅ statsmodels 0.14.6 available (needed for MixedLM if EB is insufficient)
- Gemini review: `data/code-review/cultural-graph-statistical-pipeline.md`
