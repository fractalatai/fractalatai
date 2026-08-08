---
session: Report Type Adjustment
status: closed
opened: 2026-08-07
closed: 2026-08-08
outcome: success

summary: >
  Added report-type adjustment to the cultural graph reporting pipeline, controlling for
  the confound where a site's report type mix inflates/deflates composite rates (Growth
  differs 42x across report types). Adjusted rates are now the default for --template and
  --monthly-tracker; --raw available for unadjusted view; --report-type for per-type deep dives.

decisions:
  - what: Default to adjusted rates, not blended
    why: >
      Blended rates without adjustment are not a valid alternative view — they are confounded.
      A site with 45% positive observations (PEN) showed Growth 0.405 in blended view but only
      +0.19 after adjustment. Unadjusted aggregation across report types is a measurement error,
      not a design choice.
    result: >
      --template and --monthly-tracker default to adjusted. --raw flag for unadjusted.
      --report-type skips adjustment automatically (single type = no mix).
  - what: Use indirect standardisation (observed minus expected) for adjustment
    why: >
      Standard epidemiological approach. Compute org-wide baseline rate per report type,
      expected rate per site from its report type proportions, report residuals.
    result: >
      Validated against manual computation for 10 sites. PEN Growth drops from 0.405 to +0.19;
      ABE flips from above-average to genuine LOW (-0.05). Follow-on session will upgrade
      to SMR ratios (observed/expected) with Poisson CIs per research findings.
  - what: Composable filter architecture for --fy and --report-type
    why: >
      Both filters needed to combine freely (e.g. --fy 2027 --report-type "Hazard & Observations").
      Original code used a single if/else for --fy.
    result: Replaced with narr_conds/join_conds lists that compose any number of filters.

metrics:
  confound_magnitude: { growth_42x: "Growth rate 0.42 (PosObs) vs 0.01 (Injury)", pen_shift: "-0.323 Growth when filtering to H&O only" }
  validation: { sites_checked: 10, manual_match: "all within rounding tolerance" }
  report_types: { total: 9, major: "Hazard & Observations (5419), Positive Observations (2194), Near Miss (1829), Injury (781)" }

lessons:
  - title: DuckDB returns float32 — upcast before storing Python float64 residuals
    detail: >
      pandas raises LossySetitemError when writing a float64 residual (e.g. -0.05) back into a
      float32 column returned by DuckDB. Fix: dashboard[c] = dashboard[c].astype(float) before
      the adjustment loop.
    tag: data
  - title: Unadjusted aggregation across heterogeneous categories is a measurement error, not a view
    detail: >
      Initially framed as "blended (default) vs adjusted (special flag)". User correctly identified
      that blending without controlling for category mix is not defensible — adjustment is the
      correct default. Flipped the flag: --raw is the special case, adjusted is normal.
    tag: methodology
  - title: Temporal trajectory should not be adjusted by FY when showing trends
    detail: >
      The temporal query intentionally shows all FYs even when --fy filters the dashboard.
      Similarly, when --report-type filters, temporal shows that type across all FYs. But
      adjustment (which controls for report type mix) is only meaningful for cross-site
      comparison, not temporal trends — temporal trajectory always shows raw rates.
    tag: methodology

artifacts:
  - scripts/cultural-graph/generate_report.py
  - .claude/skills/cultural-graph-report/SKILL.md
  - data/qq/cultural-graph/outputs/briefs/site-cultural-profiles-brief.md
  - data/qq/cultural-graph/outputs/briefs/cultural-graph-executive-summary.md
  - .claude/plans/cultural-graph/monthly-tracker-options.md
  - .claude/sessions/cultural-graph/08-08-26-statistical-rigour.md

depends_on: []

enables:
  - 08-08-26-statistical-rigour.md (SMR ratios, funnel plots, negative binomial regression, FDR correction)
---

# Session: Report Type Adjustment (CLOSED)

## Problem

Blended composite rates (Voice/Leadership/Drift/Care/Growth per narrative) confound two signals: genuine cultural differences between sites and the report type distribution at each site. Growth differs 42x between positive observations and injury reports. A site with 45% positive observations (PEN) will naturally show high Growth regardless of culture. Cross-site comparison is misleading without controlling for report type mix.

Analysis and three candidate approaches documented in `.claude/plans/cultural-graph/monthly-tracker-options.md` (section "Report type confound — v0.3").

## Todo

- ✅ Add `--report-type` filter to `generate_report.py --template` (Approach 1 — per-report-type dashboards)
- ✅ Add `--report-type` filter to `generate_report.py --monthly-tracker`
- ✅ Add `--adjusted` flag to `generate_report.py --template` (Approach 2 — report-type-adjusted rates computed in Python)
- ✅ Generate per-report-type dashboards for Hazard & Observations and Positive Observations — validated: Growth ranking completely reshuffles (PEN drops from #1 to #2, -0.323 shift; ABE -0.102 shift). Confound confirmed and resolved
- ✅ Generate adjusted-rate dashboard — validated: PEN Growth drops from 0.405 to +0.19 (half was report mix); ABE flips from above-average to genuine LOW (-0.05); WFH confirmed genuine HIGH (+0.22). Report values match manual computation
- ✅ Update SKILL.md with `--report-type` and `--adjusted` usage and guidance on when to use each view
- ✅ Update briefs with a note on report type confound and how to interpret blended vs adjusted rates
- ✅ Decide default: adjusted is the default for both `--template` and `--monthly-tracker` (correct when aggregating across report types). `--raw` for unadjusted. `--report-type` skips adjustment automatically

## Dependencies

- ✅ Five-composite model (Voice/Leadership/Drift/Care/Growth) — implemented in `generate_report.py`
- ✅ Rates per narrative (not percentages) — implemented
- ✅ PowerBI CSV with per-site/year/report_type rows — already exported by `--csv`
- ✅ Report type confound analysis — documented in `monthly-tracker-options.md`
- ✅ Production data: 11,170 narratives, 9 report types, 68 sites in `cultural-graph.duckdb`
