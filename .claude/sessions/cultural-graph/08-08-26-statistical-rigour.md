---
session: Statistical Rigour
status: closed
opened: 2026-08-08
closed: 2026-08-08
outcome: success

summary: >
  Replaced ad-hoc ±30% flagging thresholds with three layers of epidemiological
  statistical correction: SMR ratios with quasi-Poisson CIs, overdispersion correction,
  and Benjamini-Hochberg FDR. Reduced false flags by 49% (140→72). Added NHS-style
  funnel plots and temporal trend analysis.

decisions:
  - what: Use quasi-Poisson correction instead of full negative binomial regression
    why: >
      NB regression with statsmodels NegativeBinomial(alpha=1.0) over-corrected (0/52
      significant sites). Quasi-Poisson (scale Poisson CI by sqrt(phi)) is the standard
      epidemiological approach and phi can be computed without statsmodels at runtime.
    result: >
      16 additional false flags eliminated (83→72 with FDR). phi computed from Pearson
      chi2 in DuckDB — no statsmodels runtime dependency.
  - what: Compute overdispersion factor from Pearson chi2/df, not simple variance/mean
    why: >
      Naive V/M ratio at narrative level (2.42 for Voice) doesn't match the Poisson GLM
      chi2/df (1.79) because it doesn't condition on the fitted report-type model. The
      Pearson chi2 from fitted residuals is the correct measure.
    result: >
      Implemented Pearson chi2/df computation in Python using DuckDB cell-level data.
      Validated against statsmodels — exact match (1.79 vs 1.79).
  - what: Apply BH-FDR via p-values derived from quasi-Poisson CIs
    why: >
      Can't apply FDR directly to CI bounds. Converted CIs to z-scores and two-sided
      p-values, then applied BH procedure. Manual BH implementation avoids dependency
      on specific scipy version.
    result: 72 flags after FDR (down from 83 quasi-Poisson without FDR).
  - what: Use raw rates (not SMR) for temporal trend analysis
    why: >
      Temporal trends compare a site to itself over time, not across sites. SMR
      (which compares to org average) is wrong for this — a site improving while
      the org also improves would show flat SMR.
    result: >
      OLS on raw rates per FY, BH-FDR corrected. 1 significant trend: BRS Voice
      falling -0.167/yr.

metrics:
  flag_progression:
    raw_30pct: 140
    poisson_ci: 99
    quasi_poisson_ci: 83
    quasi_poisson_fdr: 72
    total_reduction_pct: 49
  overdispersion_phi:
    voice: 1.79
    leadership: 2.19
    drift: 1.78
    care: 1.22
    growth: 1.67
  temporal_trends:
    tests: "~220 site×composite"
    significant_after_fdr: 1
    top_trend: "BRS Voice -0.167/yr over 3 years"

lessons:
  - title: Quasi-Poisson beats full negative binomial for overdispersed count data in screening applications
    detail: >
      NegativeBinomial(alpha=1.0) in statsmodels produced 0 significant sites — the fixed
      alpha over-corrected. Quasi-Poisson (multiply CI width by sqrt(phi)) is simpler, has
      no alpha to tune, and is the standard in epidemiological SMR analysis. The key insight:
      we're doing screening, not model fitting — we want slightly wider CIs, not a different
      distributional assumption.
    tag: methodology
  - title: Pearson chi2/df must be computed from model residuals, not raw variance/mean
    detail: >
      Naive per-narrative V/M ratio gave phi=2.42 for Voice. Pearson chi2/df from the
      report-type-fitted Poisson model gave 1.79. The difference is that the model residuals
      remove between-report-type variance (which is signal, not noise). Computing phi from
      raw V/M would over-inflate the CIs.
    tag: methodology
  - title: DuckDB float32 returns need explicit upcast before storing float64 results
    detail: >
      pandas raises LossySetitemError when writing Python float64 into DuckDB-returned
      float32 columns. Apply dashboard[c] = dashboard[c].astype(float) before the
      computation loop. Same issue hit in the previous session with residuals.
    tag: data
  - title: BH-FDR can be implemented in 6 lines without scipy version dependency
    detail: >
      scipy.stats.false_discovery_control exists but the API varies across versions.
      Manual BH (sort p-values, compare to k/n*alpha thresholds, find max k) is trivial
      and portable. Used in both apply_fdr and compute_temporal_trends.
    tag: tooling
  - title: Funnel plot phi label per subplot communicates overdispersion to technical audiences
    detail: >
      Adding "(φ=1.8)" to each subplot title makes the quasi-Poisson correction visible
      without cluttering the plot. Reviewers can immediately see that Leadership (φ=2.2)
      has wider funnels than Care (φ=1.2).
    tag: methodology

artifacts:
  - scripts/cultural-graph/generate_report.py
  - .claude/skills/cultural-graph-report/SKILL.md
  - data/qq/cultural-graph/outputs/briefs/site-cultural-profiles-brief.md
  - data/qq/cultural-graph/outputs/briefs/cultural-graph-executive-summary.md
  - data/qq/cultural-graph/outputs/reports/funnel-plots.png

depends_on:
  - 08-07-26-report-type-adjustment.md

enables:
  - Random effects / multilevel models for small-site shrinkage (future)
  - Zero-inflated models if excess zeros observed (future)
---

# Session: Statistical Rigour (CLOSED)

## Problem

The cultural graph reporting pipeline uses ad-hoc thresholds (±30% of org mean) for site flagging and simple residuals (observed − expected) for report-type adjustment. Standard epidemiological and count-data methods would strengthen the analysis: SMR-style ratios with confidence intervals, overdispersion-aware regression, multiple comparison correction, and funnel plots for C-suite communication.

## Todo

- ✅ Switch from residuals to SMR ratios (observed / expected) with exact Poisson CIs — flag when CI excludes 1.0. 29% fewer flags (99 vs 140) — CI-based flagging eliminates noise from small sites
- ✅ Add funnel plots (matplotlib) — site SMR vs expected count with 95%/99.8% control limits. NHS-style 2x3 grid, red outliers labeled. `--funnel` flag, standalone or combined with `--template`
- ✅ Overdispersion validated (V/M 1.22–2.19) and corrected via quasi-Poisson (sqrt(phi) CI inflation). 83 flags vs 99 Poisson / 140 raw — 16 false positives from overdispersion eliminated. No statsmodels runtime dependency (phi computed from Pearson chi2 in DuckDB)
- ✅ Apply Benjamini-Hochberg FDR correction — 72 flags (down from 83 quasi-Poisson, 99 Poisson, 140 raw). 49% total reduction from original ad-hoc thresholds
- ✅ Add temporal slope analysis — OLS per site×composite, BH-FDR corrected. 1 significant trend detected (BRS Voice -0.167/yr). Skipped when --fy filters to single year
- ✅ Update SKILL.md and briefs with new methodology (SMR, quasi-Poisson, BH-FDR, funnel plots, temporal trends)

## Dependencies

- ✅ Report-type adjustment implemented and defaulted (session: Report Type Adjustment)
- ✅ Five-composite model with rates per narrative
- ✅ Production data: 11,170 narratives, 68 sites, 6 FYs
- ✅ scipy 1.17.1 available
- ✅ statsmodels 0.14.6 installed (used for validation only — runtime uses Pearson chi2 computed in DuckDB)
- ✅ scipy 1.17.1 available
- Design options and research documented in `.claude/plans/cultural-graph/monthly-tracker-options.md` (section "Statistical rigour — v0.4")
