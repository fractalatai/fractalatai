---
session: Empirical Bayes Shrinkage
status: closed
opened: 2026-08-08
closed: 2026-08-11
outcome: success

summary: >
  Implemented gamma-Poisson empirical Bayes shrinkage for SMR estimates, pulling noisy
  small-site values toward the org mean. Combined with log-scale GLM, reduced false flags
  by 48% total (71→37 flags, 31→18 sites). N≥10 filter retained.

decisions:
  - what: Use method-of-moments for gamma prior estimation (not MLE or MCMC)
    why: >
      Method of moments is simple, interpretable, and sufficient for 44 sites. MLE adds
      complexity without meaningful improvement. Full Bayesian (MCMC) is overkill for
      this use case.
    result: >
      Prior parameters: Voice α=4.63/β=4.86, Growth α=9.57/β=9.83. Prior means all
      near 1.0 as expected. Growth most regularised (fewest counts = most noise).
  - what: Keep N≥10 filter despite shrinkage
    why: >
      At N<10, data weight is <53% for Voice and worse for sparse composites — estimates
      are majority prior-driven. Including N<10 adds noise to EB estimation and inflates
      the FDR correction denominator.
    result: >
      At N=10 data weight is 69% (Voice), rising to 96% at N=100. Only 1 of 24 N<10
      sites would be flagged anyway. Filter remains the right default.
  - what: Recompute CIs around shrunken estimates using posterior SE = sqrt(phi/(O+alpha))
    why: >
      Using raw-SMR CIs with shrunken point estimates would be inconsistent. The
      posterior variance is smaller because the prior contributes alpha pseudo-observations.
    result: Shrunken CIs are tighter for small sites, consistent with the reduced uncertainty.
  - what: Defer mixed-effects quasi-Poisson GLM upgrade
    why: >
      EB method-of-moments gives equivalent shrinkage without computational cost or
      convergence issues. MixedLM only needed for site-level covariates or >100 sites.
    result: Documented clear revisit criteria in session doc and SKILL.md.

metrics:
  flags: { before_glm: 71, after_glm: 48, after_eb: 37, total_reduction_pct: 48 }
  flagged_sites: { before: 31, after: 18, reduction_pct: 42 }
  shrinkage_median: { voice: 0.014, leadership: 0.048, drift: 0.082, care: 0.050, growth: 0.257 }
  data_weight_at_n10: 0.69
  data_weight_at_n100: 0.96

lessons:
  - title: Subtract sampling variance before estimating prior variance
    detail: >
      Naive Var(O/E) overestimates the true between-site variance because it includes
      Poisson sampling noise. Must subtract mean(1/E) (average sampling variance) to get
      Var(theta). Without this correction the prior is too diffuse and shrinkage is weak.
    tag: methodology
  - title: Growth composite shrinks most because it has the fewest counts
    detail: >
      Growth baseline is ~0.12/narrative vs Voice ~1.09. At N=30 a site has ~4 expected
      Growth edges vs ~33 Voice. The gamma prior (alpha~10) dominates Growth estimates
      for small sites while Voice estimates remain data-driven. This is correct behaviour
      but worth noting for interpretation.
    tag: methodology
  - title: EB shrinkage and FDR interact — shrinkage reduces flags beyond what p-value correction alone achieves
    detail: >
      Log-scale GLM alone: 48 flags. Adding EB shrinkage: 37 flags. The 11 additional
      flags removed were on sites where the shrunken SMR moved closer to 1.0, widening
      the posterior CI enough to include 1.0. This is not double-counting — shrinkage
      reduces the point estimate while FDR controls the false discovery rate.
    tag: methodology

artifacts:
  - scripts/cultural-graph/generate_report.py
  - .claude/skills/cultural-graph-report/SKILL.md

depends_on:
  - 08-08-26-log-scale-glm.md
  - 08-08-26-statistical-rigour.md

enables:
  - Future: mixed-effects quasi-Poisson GLM if >100 sites or site-level covariates needed
---

# Session: Empirical Bayes Shrinkage (CLOSED)

## Problem

Small sites (N=10-30) have noisy SMR point estimates — a site with 15 narratives and SMR=2.5 is probably less extreme than it looks. Without shrinkage, C-suite consumers may overreact to extreme small-site SMRs. Gemini review (2026-08-08) identified this as "the most critical missing piece" for fair institutional comparison. The same approach is used in CMS Hospital Compare and NHS hospital profiling.

## Todo

- ✅ Implement gamma-Poisson empirical Bayes shrinkage: `SMR_shrunk = (O + alpha) / (E + beta)`, method-of-moments prior estimation
- ✅ Compare shrunken vs raw SMRs — small sites move 0.10-0.24 toward mean; large sites <0.002
- ✅ Evaluate N≥10 filter — keep it. At N<10 estimates are 60%+ prior-driven. Shrinkage at N≥10 is data-driven (69%+ data weight)
- ✅ Integrate shrunken SMRs into dashboard, funnel plots, and flag computation (both template and monthly tracker)
- ✅ Update FDR to use p-values from shrunken estimates (posterior SE = sqrt(phi/(O+alpha)))
- ⏸️ Evaluate upgrade path: empirical Bayes → mixed-effects quasi-Poisson GLM — EB is sufficient for current data, defer unless new requirements emerge
- ✅ Update session docs and SKILL.md

## Dependencies

- ✅ Log-scale GLM framework (session: Log-Scale GLM Framework) — 71→48 flags, consistent CI/p-value chain
- ✅ SMR + quasi-Poisson + FDR pipeline implemented (session: Statistical Rigour)
- ✅ statsmodels 0.14.6 available (needed for MixedLM if EB is insufficient)
- Gemini review: `data/code-review/cultural-graph-statistical-pipeline.md`

## Results

### Pipeline progression

| Stage | Flags | Flagged sites | Change |
|-------|-------|---------------|--------|
| Old (symmetric CIs, pre-GLM) | 71 | 31 | baseline |
| + Log-scale GLM | 48 | 25 | -32% flags |
| + EB shrinkage | 37 | 18 | -48% total |

### EB prior parameters (method of moments)

| Composite | alpha | beta | Prior mean | Interpretation |
|-----------|-------|------|------------|----------------|
| Voice | 4.63 | 4.86 | 0.95 | ~5 pseudo-observations at mean |
| Leadership | 2.79 | 2.86 | 0.98 | ~3 pseudo-obs |
| Drift | 3.29 | 3.18 | 1.04 | ~3 pseudo-obs |
| Care | 7.38 | 7.62 | 0.97 | ~7 pseudo-obs |
| Growth | 9.57 | 9.83 | 0.97 | ~10 pseudo-obs, most regularised |

### Shrinkage examples (Voice composite)

| Site | N | Raw SMR | Shrunk SMR | Movement |
|------|---|---------|------------|----------|
| 1.17 NWA | 22 | 0.17 | 0.40 | +0.23 |
| 6.42 RAF | 10 | 1.66 | 1.42 | -0.24 |
| 4.04 MGB | 37 | 0.10 | 0.23 | +0.13 |
| 6.46 SLF | 34 | 0.00 | 0.11 | +0.11 |
| 6.07 BCE | 1665 | 0.96 | 0.95 | <0.01 |
| 6.04 ASH | 875 | 0.56 | 0.56 | <0.01 |

Median shrinkage across all composites: Voice 0.014, Leadership 0.048, Drift 0.082, Care 0.050, Growth 0.257. Growth shrinks most because it has the fewest counts (most noise).

### FDR flag changes from shrinkage (log-scale → log-scale + EB)

- **Lost flags** (7 sites): 1.13 SYD_EVE (kept voice:LOW only), 3.09 MHA (lost growth:LOW, drift:LOW), 6.17 FHD (lost growth:HIGH), 6.27 LNC_4 (lost voice:LOW), 6.34 MAN (lost drift:HIGH), 6.46 SLF (lost care:LOW), 6.54 WIN (lost care:HIGH)
- **Net**: 48 → 37 flags, 25 → 18 flagged sites

### N≥10 filter decision

Keep. At N<10 the data weight is <53% for Voice (worse for sparse composites). Shrinkage at N≥10 provides 69%+ data weight — estimates are still informative. Including N<10 adds noise to EB estimation and FDR correction.

### Upgrade path

Mixed-effects quasi-Poisson GLM (site as random intercept) would be the full Bayesian approach. Not needed now — EB method-of-moments gives equivalent shrinkage without the computational cost or convergence issues. Revisit if: (a) >100 sites, (b) site-level covariates needed, or (c) temporal random slopes requested.
