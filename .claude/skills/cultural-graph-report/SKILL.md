---
description: Generate cultural graph reports and dashboards from DuckDB. Monthly tracker with flag change detection, templated markdown reports (blended, per-report-type, or report-type-adjusted), PowerBI CSV exports, and per-year or all-years analysis. Step 4 of the monthly cultural graph workflow.
---

# Cultural Graph: Reporting

## When This Applies

After `/cultural-graph-load` has updated DuckDB with new extraction results. This is step 4 of the monthly workflow:

1. `/cultural-graph-ingest` → cleaned JSONL
2. `/cultural-graph-runpod` → inference results
3. `/cultural-graph-load` → DuckDB updated
4. **Report** (this skill) → markdown report + PowerBI CSV

## Usage

### Part A: Monthly tracker (run after each load)

Append-only scorecard showing batch volume, cumulative totals, org averages, temporal trajectory, and flag changes. Run this after every `/cultural-graph-load`.

```bash
# Default — adjusted for report type mix
/usr/bin/python3 scripts/cultural-graph/generate_report.py --monthly-tracker

# Per report type (separate tracker file and state sidecar)
/usr/bin/python3 scripts/cultural-graph/generate_report.py --monthly-tracker --report-type "Hazard & Observations"

# Raw unadjusted (not recommended for cross-site comparison)
/usr/bin/python3 scripts/cultural-graph/generate_report.py --monthly-tracker --raw
```

Output: `data/qq/cultural-graph/outputs/reports/monthly-tracker.md` (appended each run), or `monthly-tracker-{slug}.md` when filtered by report type. Each report type gets its own state sidecar for independent flag change detection.

Each entry shows:
- **Volume**: batch narratives/edges vs cumulative totals
- **Org averages**: Voice/Drift/Care percentages
- **Temporal trajectory**: speaks-up, normalises, cares-for by FY (full history)
- **Flag changes**: sites that gained or lost HIGH/LOW flags since last run
- **Active flags**: all currently flagged sites

Flag change detection uses a JSON sidecar (`.monthly-tracker-state.json`) that stores the previous run's flags and diffs against the current state. First run has no previous state so shows active flags only.

The tracker replaces the need to manually update the prose briefs (`site-cultural-profiles-brief.md`, `cultural-graph-executive-summary.md`) each month. Those briefs are now static explainer documents, updated only on structural changes (schema version, new edge types).

### Part B: Templated narrative report

Standard Voice/Drift/Care dashboard, "Sites to Watch", temporal trajectory.

```bash
# Default — adjusted for report type mix (correct for cross-site comparison)
/usr/bin/python3 scripts/cultural-graph/generate_report.py --template

# Specific financial year
/usr/bin/python3 scripts/cultural-graph/generate_report.py --template --fy 2027

# Per report type — compares like with like within a single report type
/usr/bin/python3 scripts/cultural-graph/generate_report.py --template --report-type "Hazard & Observations"
/usr/bin/python3 scripts/cultural-graph/generate_report.py --template --report-type "Positive Observations"

# Raw unadjusted — blended rates without controlling for report type mix
/usr/bin/python3 scripts/cultural-graph/generate_report.py --template --raw

# Funnel plots — NHS-style SMR vs expected count with control limits (PNG)
/usr/bin/python3 scripts/cultural-graph/generate_report.py --funnel

# Template + funnel in one run
/usr/bin/python3 scripts/cultural-graph/generate_report.py --template --funnel
```

Output: `data/qq/cultural-graph/outputs/reports/cultural-graph-report-{suffix}.md` and optionally `funnel-plots.png`

The report auto-generates:
- Overview stats (narratives, sites, cultural edges)
- Executive dashboard — SMR per site, flagged at FDR<0.05 (quasi-Poisson CIs, Benjamini-Hochberg)
- "Sites to Watch" — top outliers with SMR and 95% CI
- Temporal trends — sites with significant year-on-year slope (OLS, FDR-corrected)
- Temporal trajectory — org-wide composite rates by FY

#### Which view to use

| View | Flag | Use case |
|------|------|----------|
| **Adjusted** | (default) | Cross-site comparison. SMR (observed/expected) controlling for report type mix. Flagged at FDR<0.05 with quasi-Poisson CIs. |
| **Per report type** | `--report-type "X"` | Site-level deep dives. Compares like with like. No adjustment needed — single type = no mix. |
| **Raw** | `--raw` | Unadjusted blended rates with 30% threshold flags. Not recommended for cross-site comparison — confounded by report type mix. |
| **Funnel** | `--funnel` | NHS-style funnel plot (PNG). Shows SMR vs expected count with 95%/99.8% control limits. Visually explains why small sites aren't flagged. |

When `--report-type` is set, adjustment is skipped automatically. `--funnel` requires adjusted data (incompatible with `--raw`).

### Part C: Bespoke analysis

For ad-hoc deep dives into specific sites, sectors, or time periods, query DuckDB directly:

```bash
/usr/bin/python3 -c "
import duckdb
con = duckdb.connect('data/cultural-graph.duckdb', read_only=True)
print(con.execute('''
    SELECT site, edge_type, count(*) AS n
    FROM narratives n JOIN edges e ON e.narrative_id = n.id
    WHERE n.site = '3.09 MHA' AND e.is_cultural
    GROUP BY site, edge_type ORDER BY n DESC
''').fetchdf().to_string(index=False))
con.close()
"
```

Common bespoke queries:
- Single site deep dive (edge type breakdown, temporal trajectory, report type comparison)
- Sector comparison (AUS vs UKD cultural profiles)
- New vs historical comparison (FY2027 vs FY2022-2026 for a specific site)
- Report type analysis (which report types produce the most cultural signal at a site)

### Part D: PowerBI CSV export

Flat CSV with per-site, per-year, per-report-type breakdowns — designed for corporate BI tool ingestion.

```bash
/usr/bin/python3 scripts/cultural-graph/generate_report.py --csv
```

Output: `data/qq/cultural-graph/outputs/reports/cultural-graph-powerbi.csv`

Columns: site, fy, report_type, sector, narratives, cultural_edges, operational_edges, avg_cultural_per_narrative, [12 edge type counts], voice_rate, leadership_rate, drift_rate, care_rate, growth_rate

Rates are edges per narrative. Raw per-type counts are included for downstream flexibility (e.g. joining headcount data to compute per-capita rates in PowerBI).

## Report Indicators

Five composites covering all 12 cultural edge types, expressed as **rates per narrative** (not percentages — percentages are compositional and produce misleading trends).

| Indicator | Components | Org average | Question |
|-----------|-----------|-------------|----------|
| **Voice** | speaks-up + cooperates + shares-info | ~1.09 | Are people communicating? |
| **Leadership** | directs + monitors | ~0.33 | Are people directing and overseeing? |
| **Drift** | normalises + adapts-to | ~0.19 | Are procedures being bypassed? |
| **Care** | cares-for + responds-to-failure + protects | ~0.53 | Does the site respond when things go wrong? |
| **Growth** | learns-from + recognises | ~0.12 | Is the site building on what it learns? |

Org average = population mean (total edges / total narratives).

### Statistical methodology (default adjusted view)

The default report uses **Standardised Morbidity Ratios (SMR)**: observed edge count / expected count given the site's report type mix. SMR = 1.0 means as expected; >1.0 = more; <1.0 = less.

Four layers of statistical correction:
1. **Indirect standardisation** — controls for report type mix (Growth differs 42x across report types)
2. **Log-scale quasi-Poisson CIs** — SE(log(SMR)) = √(φ/O), CI = exp(log(SMR) ± z·SE). Naturally asymmetric; corrects for overdispersion (φ ranges 1.2–2.2 across composites)
3. **Empirical Bayes shrinkage** — gamma-Poisson prior estimated via method of moments. SMR_shrunk = (O + α) / (E + β). Small sites pulled toward org mean; large sites barely change. Prevents C-suite overreaction to noisy small-site estimates (same approach as CMS Hospital Compare, NHS hospital profiling)
4. **Benjamini-Hochberg FDR** — corrects for multiple comparisons (5 composites × ~44 sites ≈ 220 tests). P-values derived on log scale: z = |log(SMR)| / SE(log(SMR))

A site is flagged HIGH/LOW only when its FDR-adjusted p-value < 0.05. Sites with N < 10 narratives are excluded (estimates would be >60% prior-driven).

**Temporal trend analysis**: per-site quasi-Poisson GLM (count ~ FY + offset(log(N)), quasipoisson family), FDR-corrected. Results shown as rate ratios per year (RR > 1 = rising, < 1 = falling).

Per-capita rates (edges per headcount) are computed downstream in PowerBI by joining headcount data against the CSV export.

## File Locations

| File | Purpose |
|------|---------|
| `scripts/cultural-graph/generate_report.py` | Report generation script |
| `data/qq/cultural-graph/outputs/reports/monthly-tracker.md` | Append-only monthly scorecard |
| `data/qq/cultural-graph/outputs/reports/.monthly-tracker-state.json` | Flag state for change detection |
| `data/qq/cultural-graph/outputs/reports/funnel-plots.png` | Funnel plots (generated by `--funnel`) |
| `data/qq/cultural-graph/outputs/reports/` | Output directory for reports, CSVs, and PNGs |
| `data/cultural-graph.duckdb` | Source data |

## Notes

- Reports are generated from DuckDB, not from raw JSONL — always run `/cultural-graph-load` first
- **Monthly workflow**: after each `/cultural-graph-load`, run `--monthly-tracker` to append the latest scorecard. This replaces manually updating the prose briefs each month.
- The prose briefs (`site-cultural-profiles-brief.md`, `cultural-graph-executive-summary.md`) are static explainer documents — update only on structural changes (schema version, new edge types, methodology changes)
- The templated report is designed for C-suite consumption — SMR ratios per site, statistically rigorous flagging, funnel plots for visual communication
- The PowerBI CSV is one row per site-year-report_type combination — pivot/filter in PowerBI
- Bespoke analysis uses DuckDB SQL directly — no script needed, just query
- **Report type confound**: blended rates confound genuine cultural differences with report type mix (Growth differs 42x between Positive Observations and Injury reports). The default view controls for this via SMR (indirect standardisation). Use `--report-type` for per-type deep dives, `--raw` for unadjusted view
- **Funnel plots** (`--funnel`): the standard NHS/CQC visualisation for comparing institutional rates. Visually explains why small sites aren't flagged — the funnel is wide at low expected counts. Log-scale quasi-Poisson control limits: exp(±z·√φ/√E), naturally asymmetric (φ shown per composite)
- Design options and statistical research documented in `.claude/plans/cultural-graph/monthly-tracker-options.md`
