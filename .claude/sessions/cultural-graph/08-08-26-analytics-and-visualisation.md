---
session: Extended Analytics and Visualisation
status: closed
opened: 2026-08-08
closed: 2026-08-11
outcome: success

summary: >
  Added five analytical extensions to the cultural graph report: caterpillar plots,
  power analysis, sensitivity analysis, sector confound surface, and batch drift
  monitoring. Surfaced a major sector confound (AUS rates 2.4× lower than UKD).

decisions:
  - what: Surface sector confound in report but do not auto-adjust for it
    why: >
      AUS Voice rate is 0.50 vs UKD 1.21 per narrative (2.4× difference). Under
      sector×RT baselines, AUS "outliers" normalise (ASH voice 0.56 → 1.26). But
      whether to adjust depends on the business question: org-wide comparison
      (current) vs within-sector comparison. This is a stakeholder decision, not
      a statistical one.
    result: >
      Sector Analysis section added to report showing rates per narrative by sector.
      Note directs users to consider --sector-adjust if within-sector comparison needed.
  - what: Embed power analysis and sensitivity analysis directly in the template report
    why: >
      These are C-suite-facing documents. Power analysis manages expectations ("at N=50
      we can only detect SMRs outside 0.71–1.41"). Sensitivity shows flags are robust
      (83-100% stable under moderate perturbations). Both build trust in the methodology.
    result: >
      Two new sections auto-generated in every adjusted report. No additional CLI flags
      needed — always included.
  - what: Use 2σ threshold for batch drift alerting, activate at ≥3 batches
    why: >
      With only 2 batches, standard deviation is meaningless. At ≥3 batches the z-score
      becomes interpretable. 2σ is conservative enough to avoid false alerts while catching
      genuine model degradation.
    result: Infrastructure ready. Currently 2 batches (2.25, 2.47 edges/narr), no alerts.

metrics:
  sensitivity_stability:
    phi_20pct: "83-100%"
    baseline_5pct: "83-100%"
    baseline_10pct: "69-94%"
    n_threshold: "89-100%"
  sector_confound:
    ukd_voice_rate: 1.208
    aus_voice_rate: 0.498
    ratio: 2.42
  power_n50_voice: "0.71-1.41"
  power_n100_voice: "0.78-1.28"
  power_n1000_voice: "0.92-1.08"

lessons:
  - title: Sector confound is the largest unmeasured confounder in the pipeline
    detail: >
      AUS sites have ~50% lower edge extraction rates than UKD across all composites.
      This is larger than the report-type confound for many composites. Under RT-only
      baselines (dominated by 81% UKD data), every AUS site looks like a systematic
      underperformer. The pipeline currently flags 6 AUS sites — most would lose flags
      under sector-adjusted baselines. This should be discussed with stakeholders before
      the next report cycle.
    tag: methodology
  - title: Sensitivity analysis reveals baseline perturbation matters more than phi or N threshold
    detail: >
      phi ±20% and N threshold changes barely affect flags (83-100% stable). But
      baseline ±10% can shift 11-19 flags (69-94% stable). This makes sense — baseline
      directly determines expected counts, which drive SMR. The sector confound is
      essentially a 50%+ baseline perturbation for AUS sites, explaining why it
      dominates.
    tag: methodology
  - title: EB shrinkage makes N threshold changes irrelevant
    detail: >
      Raising N from 10 to 15 or 20 changes zero flags. Lowering to 5 actually reduces
      flags (more tests = stricter FDR). Shrinkage already handles marginal sites by
      pulling their estimates toward the mean — the N threshold is now just a data-quality
      gate, not a statistical one.
    tag: methodology

artifacts:
  - scripts/cultural-graph/generate_report.py
  - .claude/skills/cultural-graph-report/SKILL.md
  - data/qq/cultural-graph/outputs/reports/caterpillar-plots.png

depends_on:
  - 08-08-26-log-scale-glm.md
  - 08-08-26-empirical-bayes-shrinkage.md
  - 08-08-26-statistical-rigour.md

enables:
  - Sector-adjusted SMR baselines (--sector-adjust flag) if stakeholders want within-sector comparison
  - Batch drift alerting (activates automatically at ≥3 extraction batches)
---

# Session: Extended Analytics and Visualisation (CLOSED)

## Problem

The pipeline produces SMR tables and funnel plots but lacks several standard analytical tools for institutional comparison: caterpillar plots for ordered site comparison, power analysis to set expectations, sensitivity analysis for robustness, confounder investigation beyond report type, and upstream model drift monitoring. These were identified in Gemini review (2026-08-08) as valuable extensions.

## Todo

- ✅ Caterpillar plots — `--caterpillar` flag, 5-panel PNG ordered by SMR with 95% CIs, red where CI excludes 1.0
- ✅ Power analysis — "Detectable Effect Sizes" section in report. Table of min detectable SMR per N per composite
- ✅ Sensitivity analysis — "Sensitivity Analysis" section in report. Tests phi ±20%, baseline ±5%/±10%, N threshold. Flags 83-100% stable under moderate perturbations
- ✅ Additional confounders — "Sector Analysis" section in report. Surfaced major sector confound: AUS rates ~50% of UKD. Documented, not auto-adjusted (business decision)
- ✅ AI model drift monitoring — "Extraction Quality" section in report. Per-batch edge rates with 2σ drift alerting (activates at ≥3 batches)
- ✅ Update SKILL.md with new visualisations and analytics

## Results

### Caterpillar plots
Five-panel PNG, one per composite. Sites ordered by SMR ascending, horizontal CIs. Red where CI excludes 1.0. Clear visual ranking with precision visible from CI width. Accessible via `--caterpillar` CLI flag.

### Power analysis
Key findings: at N=50, Voice SMRs must be below 0.71 or above 1.41 to be detectable. Leadership and Drift need wider deviations (0.52-1.93 and 0.48-2.08 at N=50) due to higher overdispersion and lower baseline rates. At N=1000, Voice detectable range narrows to 0.92-1.08.

### Sensitivity analysis
Flags are robust: 83-100% stable under phi ±20%, 83-100% stable under baseline ±5%. Baseline ±10% is the most impactful perturbation (69-94% stable). N threshold changes (5, 15, 20) have minimal effect — EB shrinkage already handles marginal sites.

### Sector confound
**Major finding**: AUS Voice rate 0.50 vs UKD 1.21 per narrative (2.4× difference). Current baselines are org-wide (81% UKD by volume), so AUS sites are compared against UKD-inflated expectations. Under sector×RT baselines, most AUS "outlier" sites normalise (e.g., ASH voice SMR 0.56 → 1.26). Not auto-adjusted — whether to control for sector depends on business question (org-wide vs within-sector comparison). Documented in report Sector Analysis section.

### Drift monitoring
Infrastructure in place. Per-batch edge rates shown in report. With only 2 batches (2.25 and 2.47 edges/narr), no alerts yet. Drift alerting activates at ≥3 batches using 2σ threshold.

## Dependencies

- ✅ Log-scale GLM framework (session: Log-Scale GLM Framework) — 71→48 flags, consistent CI/p-value chain
- ✅ Empirical Bayes shrinkage (session: Empirical Bayes Shrinkage) — 48→37 flags, shrunken estimates integrated
- ✅ Funnel plots and temporal trends implemented (session: Statistical Rigour)
- ✅ matplotlib 3.11.1 available
- Gemini review: `data/code-review/cultural-graph-statistical-pipeline.md`
