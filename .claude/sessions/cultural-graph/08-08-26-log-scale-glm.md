---
session: Log-Scale GLM Framework
status: pending
opened: 2026-08-08
---

# Session: Log-Scale GLM Framework (PENDING)

## Problem

Gemini review (2026-08-08) identified three correctness issues in the statistical pipeline, all stemming from the same root cause: mixing asymmetric Poisson CIs with symmetric operations. The fix is to work on the log scale throughout, where the distribution is symmetric and quasi-Poisson corrections are well-defined.

Specific issues:
1. **Asymmetric CI inflation**: `smr_poisson_ci` computes exact Poisson CIs (asymmetric), then inflates symmetrically around SMR. At low counts this distorts the bounds.
2. **P-value derivation**: Converting asymmetric CIs to symmetric z-scores (`z = |SMR-1|/SE`) is inconsistent — feeds incorrect p-values into BH-FDR.
3. **Temporal trends**: OLS on raw rates assumes normal errors with constant variance — wrong for count data.

All three fix the same way: move to a quasi-Poisson GLM on the log scale.

## Todo

- ⬜ Replace `smr_poisson_ci` with log-scale approach: SE(log(SMR)) = sqrt(phi/O), CI = exp(log(SMR) +/- 1.96*SE), naturally asymmetric
- ⬜ Derive p-values on log scale: z = log(SMR) / SE(log(SMR)), two-sided normal p-value — consistent with quasi-Poisson assumption
- ⬜ Replace OLS temporal trends with quasi-Poisson GLM: `count ~ FY + offset(log(n_narratives))`, quasipoisson family (statsmodels)
- ⬜ Update funnel plot control limits to use log-scale CIs (exponentiated back)
- ⬜ Validate: compare flag counts and CI bounds against current pipeline — expect similar results with better statistical consistency
- ⬜ Update SKILL.md methodology section

## Dependencies

- ✅ SMR + quasi-Poisson + FDR pipeline implemented (session: Statistical Rigour)
- ✅ scipy 1.17.1, statsmodels 0.14.6 available
- Gemini review: `data/code-review/cultural-graph-statistical-pipeline.md`
