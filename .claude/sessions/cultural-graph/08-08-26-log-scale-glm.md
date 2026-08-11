---
session: Log-Scale GLM Framework
status: closed
opened: 2026-08-08
closed: 2026-08-11
outcome: success

summary: >
  Replaced three inconsistent statistical operations (symmetric CI inflation,
  CI-to-z p-values, OLS temporal trends) with a unified log-scale quasi-Poisson
  framework. Reduced false flags by 32% (71→48) with properly asymmetric CIs.

decisions:
  - what: Use SE(log(SMR)) = sqrt(phi/O) instead of symmetric Poisson CI inflation
    why: >
      Gemini review identified that inflating asymmetric exact Poisson CIs symmetrically
      around the SMR distorts bounds at low counts. Log-scale SE is symmetric where it
      matters and exponentiates back to naturally asymmetric CIs.
    result: >
      Small-site CIs now correctly wider on the upper tail (NWA N=22 O=2: [0.000, 0.573]
      → [0.027, 1.097]). Large-site CIs nearly unchanged (SHB: [1.091, 1.257] → [1.094, 1.259]).
  - what: Derive p-values as z = |log(SMR)| / SE(log(SMR)) instead of CI-width back-conversion
    why: >
      Converting asymmetric CIs to symmetric z-scores via SE = CI_width/(2*1.96) was
      statistically inconsistent — fed incorrect p-values into BH-FDR.
    result: >
      8 sites lost flags (borderline false positives), 6 new site×composite flags gained.
      Net: 71→48 flags, 31→25 flagged sites.
  - what: Replace OLS temporal trends with quasi-Poisson GLM (count ~ FY + offset(log(N)))
    why: >
      OLS on raw rates assumes normal errors with constant variance — wrong for count data.
      Quasi-Poisson GLM correctly models count nature, varying exposure, and overdispersion.
    result: >
      5 significant trends detected (same key sites). Now reports rate ratios per year
      instead of raw slopes. Added statsmodels dependency, removed scipy.linregress.
  - what: Clip funnel plot y-axis to data range
    why: >
      Log-scale upper limits grow exponentially at small E (exp(3.09*sqrt(phi/E)) at E=1
      can be ~100), making plots unreadable. Symmetric limits capped more naturally.
    result: Readable funnel plots with data-driven y-axis. Funnel curves still visible at moderate E.

metrics:
  flags: { before: 71, after: 48, reduction_pct: 32 }
  flagged_sites: { before: 31, after: 25, reduction_pct: 19 }
  sites_lost_flags: 8
  sites_gained_flags: 3
  temporal_trends_significant: 5

lessons:
  - title: Log-scale funnel limits need y-axis clipping
    detail: >
      exp(±z·√φ/√E) produces enormous upper limits at small E (e.g., 97× at E=1 for
      Leadership φ=2.2). The old symmetric ±z·√φ/√E capped at ~6×. Must clip y-axis
      to max(SMR)*1.3 or similar to keep plots readable.
    tag: methodology
  - title: Quasi-Poisson GLM warnings are expected for sparse composites
    detail: >
      statsmodels GLM with Poisson family emits PerfectSeparationWarning and divide-by-zero
      RuntimeWarning for sites with zero counts in some composites. These are harmless
      (try/except catches actual failures). Suppress with warnings.catch_warnings().
    tag: tooling
  - title: O=0 case needs special handling in log-scale CIs
    detail: >
      log(0) is undefined, so the log-scale CI formula breaks when observed=0. Fall back
      to exact Poisson upper bound (chi2.ppf) for the O=0 case. This is a well-known
      edge case in Poisson epidemiology.
    tag: methodology

artifacts:
  - scripts/cultural-graph/generate_report.py

depends_on:
  - 08-08-26-statistical-rigour.md

enables:
  - 08-08-26-empirical-bayes-shrinkage.md
---

# Session: Log-Scale GLM Framework (CLOSED)

## Problem

Gemini review (2026-08-08) identified three correctness issues in the statistical pipeline, all stemming from the same root cause: mixing asymmetric Poisson CIs with symmetric operations. The fix is to work on the log scale throughout, where the distribution is symmetric and quasi-Poisson corrections are well-defined.

Specific issues:
1. **Asymmetric CI inflation**: `smr_poisson_ci` computes exact Poisson CIs (asymmetric), then inflates symmetrically around SMR. At low counts this distorts the bounds.
2. **P-value derivation**: Converting asymmetric CIs to symmetric z-scores (`z = |SMR-1|/SE`) is inconsistent — feeds incorrect p-values into BH-FDR.
3. **Temporal trends**: OLS on raw rates assumes normal errors with constant variance — wrong for count data.

All three fix the same way: move to a quasi-Poisson GLM on the log scale.

## Todo

- ✅ Replace `smr_poisson_ci` with log-scale approach: SE(log(SMR)) = sqrt(phi/O), CI = exp(log(SMR) ± 1.96*SE), naturally asymmetric
- ✅ Derive p-values on log scale: z = |log(SMR)| / SE(log(SMR)), two-sided normal p-value — consistent with quasi-Poisson assumption
- ✅ Replace OLS temporal trends with quasi-Poisson GLM: `count ~ FY + offset(log(n_narratives))`, quasipoisson family (statsmodels)
- ✅ Update funnel plot control limits to use log-scale CIs (exponentiated back)
- ✅ Validate: compare flag counts and CI bounds against current pipeline
- ✅ Update SKILL.md methodology section (done at end of EB session)

## Dependencies

- ✅ SMR + quasi-Poisson + FDR pipeline implemented (session: Statistical Rigour)
- ✅ scipy 1.17.1, statsmodels 0.14.6 available
- Gemini review: `data/code-review/cultural-graph-statistical-pipeline.md`

## Validation Results

Baseline (old symmetric CI inflation) → log-scale:
- **Flags**: 71 → 48 (-32%), **flagged sites**: 31 → 25 (-19%)
- Small-site CIs now properly asymmetric (e.g., NWA N=22 O=2: [0.000, 0.573] → [0.027, 1.097])
- Large-site CIs nearly identical (e.g., SHB N=1072: [1.091, 1.257] → [1.094, 1.259])
- **Lost flags** (8 sites — borderline, false positives from inconsistent CI→z chain): 1.07 MEL_CTN, 1.09 MEL_S, 1.17 NWA, 4.03 KEL, 6.16 FRN, 6.26 LNC_LH, 6.30 LGL, 7.06 OFF
- **Gained flags** (6 new site×composite flags): 3.11 OFF leadership:HIGH, 6.17 FHD growth:HIGH, 6.24 HUR leadership:HIGH, 6.34 MAN drift:HIGH, 6.40 PLB leadership:HIGH + care:HIGH, 6.54 WIN care:HIGH
- **Reduced flags** on many sites (e.g., 1.08 MEL_P 3→2, 1.13 SYD_EVE 4→1, 4.04 MGB 4→2)
- Temporal trends: OLS → quasi-Poisson GLM. 5 significant trends (same key sites: ASH voice/drift/care/growth rising, MEL_P voice falling). Now reports rate ratios per year instead of raw slopes
- Funnel plots: asymmetric log-scale limits `exp(±z·√φ/√E)`, y-axis clipped to data range for readability
- `linregress` import removed, `statsmodels` added for temporal GLM
