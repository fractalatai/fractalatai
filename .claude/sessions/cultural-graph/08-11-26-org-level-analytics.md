---
session: Org-Level Analytics
status: closed
opened: 2026-08-11
closed: 2026-08-11
outcome: success

summary: >
  Replaced raw blended org averages with standardised prevalence (56%) + intensity
  (Voice 1.81, Care 0.95) decomposition, removing report-type mix confound. Added
  bootstrap CIs, standardised sector rates, EB sensitivity tests, directory
  reorganisation, and Gemini review. Revealed AUS-UKD gap is primarily prevalence
  (34% vs 63%), not intensity.

decisions:
  - what: Decompose org-level rates into prevalence + intensity instead of single blended rate
    why: >
      Raw blended Voice (1.09) was diluted by 43% zero-signal narratives and dominated
      by Hazard & Observations volume (49%). C-suite consumers could not compare
      composites fairly. User identified that the binary cultural/non-cultural signal
      is itself informative and should be separated from edge density.
    result: >
      Prevalence 56% (49%–62% CI), Intensity Voice 1.81 (1.58–2.08). The 66% increase
      from blended to intensity accurately reflects cultural signal density without
      dilution.
  - what: Equal-weight across report types for standardisation
    why: >
      Volume-weighting lets Hazard & Observations (49% of data) dominate. Equal-weighting
      answers "what is the rate across report types?" without one type drowning out others.
      Gemini prefers inverse-variance weighting but acknowledged equal-weighting is
      defensible if the business question is stated explicitly.
    result: >
      7 report types with N≥50 contribute equally. Parked IV-weighting as a future option
      pending stakeholder input on which question to answer.
  - what: Standardise sector rates using same prevalence + intensity method as org level
    why: >
      Gemini review identified raw blended sector rates as inconsistent with org-level
      standardisation. Without same-method standardisation, sector differences are still
      confounded by within-sector report-type mix.
    result: >
      Revealed raw 2.4× Voice gap decomposes into prevalence gap (UKD 63% vs AUS 34%)
      and smaller intensity gap (1.90 vs 1.46, 1.3×). Confound is primarily narrative
      quality/style, not cultural signal density.
  - what: Reorganise data/qq/cultural-graph directory
    why: >
      outputs/ mixed training data, briefs, generated reports, analysis parquets, review
      samples, and an orphan comms plan. No README. Root CSVs cluttered alongside directories.
    result: >
      Eliminated outputs/. New structure: raw/, reports/, training/, analysis/, review/,
      docs/ (with briefs), archive/. README indexes at root, reports/, and docs/.
      7 scripts and 1 skill updated.

metrics:
  prevalence: { value: 0.56, ci_lo: 0.49, ci_hi: 0.62 }
  intensity_voice: { value: 1.81, ci_lo: 1.58, ci_hi: 2.08 }
  intensity_growth: { value: 0.16, ci_lo: 0.04, ci_hi: 0.37 }
  sector_prevalence: { ukd: 0.63, aus: 0.34 }
  sector_voice_intensity: { ukd: 1.90, aus: 1.46 }
  eb_sensitivity: { eb_0_5_stable_pct: 100, eb_2_0_stable_pct: 89 }
  old_blended_voice: 1.09
  new_intensity_voice: 1.81
  increase_pct: 66

lessons:
  - title: Raw blended rates hide two distinct signals — prevalence and intensity
    detail: >
      43% of narratives have zero cultural edges. Blending them with signal-bearing
      narratives dilutes intensity by ~40%. The user correctly identified that the
      binary signal (cultural vs non-cultural) is itself informative and should be
      separated. This decomposition is standard in epidemiology (e.g., disease
      prevalence vs severity given infection) but was not initially applied here.
    tag: methodology
  - title: Sector confound is primarily a prevalence difference, not intensity
    detail: >
      The raw 2.4× Voice gap (UKD 1.21 vs AUS 0.50) looked like a massive cultural
      difference. After decomposition, the intensity gap is only 1.3× (1.90 vs 1.46).
      The real difference is prevalence: AUS narratives carry cultural signal 34% of
      the time vs UKD 63%. This suggests the difference is in narrative style/quality
      (shorter, more factual AUS reports) rather than genuine cultural divergence.
      Without the decomposition, this distinction was invisible.
    tag: methodology
  - title: Growth CI is 10× wider than Care CI because one report type is an outlier
    detail: >
      Growth intensity CI is 0.04–0.37 vs Care 0.88–1.01. This is because Positive
      Observations has Growth intensity 0.80 while all other types are <0.10. Equal-
      weighting gives this outlier full influence. This is a genuine feature of the
      data (positive observations are where recognition happens) but makes the Growth
      aggregate unstable. Flagged as a case for IV-weighting consideration.
    tag: methodology
  - title: Gemini correctly identified that sector rates must use the same standardisation as org rates
    detail: >
      We initially showed raw blended sector rates alongside standardised org rates —
      an inconsistency that made the sector comparison invalid. The fix was straightforward
      (same prevalence + intensity computation, scoped to sector) but the inconsistency
      was not obvious until Gemini pointed it out.
    tag: methodology

artifacts:
  - scripts/cultural-graph/generate_report.py
  - .claude/skills/cultural-graph-report/SKILL.md
  - data/qq/cultural-graph/README.md
  - data/qq/cultural-graph/reports/README.md
  - data/qq/cultural-graph/docs/README.md
  - data/code-review/cultural-graph-org-level-analytics.md

depends_on:
  - 08-08-26-log-scale-glm.md
  - 08-08-26-empirical-bayes-shrinkage.md
  - 08-08-26-analytics-and-visualisation.md

enables:
  - Inverse-variance weighting option (parked, needs stakeholder input)
  - Sector-adjusted SMR baselines (--sector-adjust flag)
  - EWMA/CUSUM drift monitoring (when ≥5 batches available)
---

# Session: Org-Level Analytics (CLOSED)

## Problem

Org-level headline numbers (Voice 1.09, Leadership 0.33, etc.) were raw blended rates — confounded by report type mix (49% Hazard & Observations) and diluted by 43% of narratives with zero cultural edges. C-suite consumers could not compare composites fairly because the numbers reflected which report types the org submits most, not genuine cultural signal strength. Gemini review (2026-08-11) confirmed the fix and identified further gaps.

## Todo

- ✅ Replace raw blended org averages with report-type-standardised prevalence + intensity decomposition
- ✅ Show per-report-type baselines in template report (N, signal %, intensity per composite)
- ✅ Update monthly tracker to use standardised prevalence + intensity
- ✅ Reset monthly tracker with clean baseline (purge development-era entries)
- ✅ Regenerate all stale reports with current pipeline
- ✅ Reorganise data/qq/cultural-graph directory (outputs/ eliminated, README indexes added)
- ✅ Gemini review of org-level analytics methodology
- ✅ Standardise sector rates — prevalence + intensity per sector, equal-weighted by RT within each sector (N≥20 per sector×RT cell)
- ✅ Add 95% CIs to aggregated prevalence and intensity — bootstrap across 7 report-type estimates (2000 resamples)
- ✅ Add EB shrinkage perturbation to sensitivity analysis — EB ×0.5 and EB ×2.0, flags 89-100% stable
- ✅ Update temporal trajectory to prevalence + intensity (was raw blended rates — inconsistent with org averages)
- 🅿️ Equal-weighting vs inverse-variance weighting for report-type standardisation — Gemini prefers IV weighting; equal-weighting answers "rate across types" without volume dominance. Needs stakeholder input on which question to answer
- 🅿️ N≥50 threshold excludes Occupational Illness (N=49, highest prevalence 80%) — consider lowering to 30 or pooling small-N types into "Other"
- 🅿️ Drift monitoring: EWMA/CUSUM instead of simple z-score — correct but premature with only 2 batches
- ⏸️ Case-mix adjustment (site-level covariates beyond sector and report type) — deferred, data doesn't exist yet

## Dependencies

- ✅ Log-scale GLM + EB shrinkage pipeline (sessions: Log-Scale GLM, Empirical Bayes Shrinkage)
- ✅ Extended analytics (session: Extended Analytics and Visualisation) — caterpillar plots, power/sensitivity, sector confound surfaced
- Gemini review: `data/code-review/cultural-graph-org-level-analytics.md`

## Results

### Prevalence + intensity decomposition

Replaced single blended rate with two standardised metrics:
- **Prevalence**: 56% of narratives contain cultural signal (standardised across report types)
- **Intensity** (edges per signal-bearing narrative): Voice 1.81, Leadership 0.54, Drift 0.26, Care 0.95, Growth 0.16

The confound is significant: blended Voice was 1.09 (diluted by 43% zero-signal narratives and dominated by Hazard & Observations volume). Intensity among signal-bearing narratives is 1.81 — a 66% higher figure that more accurately reflects cultural signal density.

### Directory reorganisation

Eliminated `outputs/` grab-bag. New structure: `raw/`, `reports/`, `training/`, `analysis/`, `review/`, `docs/` (with briefs), `archive/`. README.md indexes at root and in `reports/` and `docs/`. All script and skill paths updated.

### Gemini review findings (2026-08-11)

Key actionable items:
1. **Sector rates inconsistently standardised** — shown raw blended while org rates are standardised. Must use same method.
2. **No CIs on aggregated metrics** — prevalence and intensity reported without uncertainty.
3. **EB shrinkage not in sensitivity analysis** — should perturb alpha/beta to test robustness.

Parked items (valid but not urgent):
4. Equal-weighting vs inverse-variance — needs business question clarity
5. N≥50 threshold impact — quantify before deciding
6. EWMA/CUSUM for drift — premature with 2 batches

### Standardised sector rates

Replaced raw blended sector rates with prevalence + intensity per sector, equal-weighted across report types within each sector (N≥20 per cell). Key finding: the raw 2.4× Voice gap (UKD 1.21 vs AUS 0.50) decomposes into:
- **Prevalence gap**: UKD 63% vs AUS 34% — AUS narratives carry cultural signal half as often
- **Intensity gap**: UKD 1.90 vs AUS 1.46 — much smaller (1.3×), closer than expected

The confound is primarily a prevalence difference (narrative quality/style), not just intensity.

### CIs on org averages

Bootstrap 95% CIs (2000 resamples across 7 eligible report types):
- Prevalence: 56% (49%–62%)
- Voice: 1.81 (1.58–2.08)
- Leadership: 0.54 (0.41–0.69)
- Drift: 0.26 (0.20–0.32)
- Care: 0.95 (0.88–1.01)
- Growth: 0.16 (0.04–0.37) — widest CI, driven by Positive Observations outlier (0.80 vs <0.10 for other types)

### EB sensitivity

Flags 89-100% stable under EB ×0.5 / ×2.0:
- EB ×0.5 (half shrinkage): 40 flags (+4, −0) — weaker shrinkage lets marginal sites through
- EB ×2.0 (double shrinkage): 32 flags (+0, −4) — stronger shrinkage absorbs more small-site noise

### Temporal trajectory

Updated template and monthly tracker temporal trajectory from raw blended rates to prevalence + intensity by FY. Signal % column added (ranges 54%–63% across FYs). Intensity numbers are now scoped to signal-bearing narratives, consistent with org-level headline metrics. All reports and both tracker variants (default + Hazard & Observations) regenerated.
