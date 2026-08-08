---
session: Statistical Rigour
status: pending
opened: 2026-08-08
---

# Session: Statistical Rigour (PENDING)

## Problem

The cultural graph reporting pipeline uses ad-hoc thresholds (±30% of org mean) for site flagging and simple residuals (observed − expected) for report-type adjustment. Standard epidemiological and count-data methods would strengthen the analysis: SMR-style ratios with confidence intervals, overdispersion-aware regression, multiple comparison correction, and funnel plots for C-suite communication.

## Todo

- ⬜ Switch from residuals to SMR ratios (observed / expected) with exact Poisson CIs — flag when CI excludes 1.0
- ⬜ Add funnel plots (matplotlib) — site rate vs volume with 95%/99.8% control limits
- ⬜ Fit negative binomial regression to handle overdispersion — validate Poisson CIs aren't over-flagging
- ⬜ Apply Benjamini-Hochberg FDR correction for 5×68 = 340 comparisons
- ⬜ Add temporal slope analysis — year-on-year linear regression per site to flag trajectory
- ⬜ Update SKILL.md and briefs with new methodology

## Dependencies

- ✅ Report-type adjustment implemented and defaulted (session: Report Type Adjustment)
- ✅ Five-composite model with rates per narrative
- ✅ Production data: 11,170 narratives, 68 sites, 6 FYs
- ⬜ scipy and statsmodels available in Python environment
- Design options and research documented in `.claude/plans/cultural-graph/monthly-tracker-options.md` (section "Statistical rigour — v0.4")
