# Cultural Graph Monthly Tracker — Design Options

*August 2026*

Given monthly narrative ingestion via the 4-skill workflow, the static prose briefs (executive summary, site profiles) need a companion that updates automatically. Three options explored, from lightest to heaviest:

---

## Option A: Append-only monthly scorecard (recommended — built)

A single file (`monthly-tracker.md`) where each month gets a new section appended automatically by `generate_report.py --monthly-tracker`. The prose briefs become static "explainer" documents, updated only when something structural changes (new schema version, new site category).

Each entry contains:
- **Volume**: batch size vs cumulative totals
- **Org averages**: Voice/Drift/Care current state
- **Temporal trajectory**: full FY table with latest data point
- **Flag changes**: sites that gained or lost HIGH/LOW flags since last run
- **Active flags**: all currently flagged sites

Flag change detection uses a JSON sidecar file (`.monthly-tracker-state.json`) that stores the previous run's flags and diffs against the current computation.

**Pros**: simple, git-diffable, the whole history is in one file. Fully automated via `generate_report.py --monthly-tracker`.
**Cons**: grows indefinitely (though slowly — ~25 lines per entry).

---

## Option B: Monthly snapshot files + index

Each month gets its own file (`tracker/2026-08.md`, `tracker/2026-09.md`). An auto-generated index file links them all and shows the cumulative trend line.

```
data/qq/cultural-graph/outputs/tracker/
  index.md              ← auto-generated, cumulative trend table
  2026-08.md            ← this month's scorecard
  2026-09.md            ← next month
```

The index carries the temporal trajectory table and auto-extends each month. Individual month files carry the site-level detail.

**Pros**: clean separation, git diffs show exactly what's new. Each month is self-contained — hand someone a single file.
**Cons**: more files. Need a script to regenerate the index.

---

## Option C: DuckDB materialised snapshots + auto-report

Add a `monthly_snapshots` table in `cultural-graph.duckdb`:

```sql
CREATE TABLE monthly_snapshots (
    month       DATE,
    site        VARCHAR,
    narratives  INTEGER,
    voice_pct   FLOAT,
    drift_pct   FLOAT,
    care_pct    FLOAT,
    flags       VARCHAR[]
);
```

Each `/cultural-graph-load` run appends the month's snapshot. Then `/cultural-graph-report --tracker` queries the snapshot table.

**Pros**: the tracker is just a SQL query. Flag changes are `WHERE this_month.flags != last_month.flags`. Trend analysis is trivial. PowerBI can connect directly.
**Cons**: most engineering work. The data already exists in edges/narratives tables — this is a pre-computed view on top.

---

## Decision

**Option A built first** — `generate_report.py --monthly-tracker` appends to `monthly-tracker.md` after each `/cultural-graph-load`. Prose briefs stay as static explainer documents, updated only on structural changes.

Option C is the right long-term answer for PowerBI dashboards but is premature until someone is actively consuming monthly trends in a BI tool.

---

## Composite indicators — v0.2

### Five composites covering all 12 edge types

The original three composites (Voice/Drift/Care) covered only 80% of cultural edges. Four edge types — directs, monitors, learns-from, recognises — were unassigned. This creates a gap that a C-suite audience will notice: "what's the other 20%?"

Two new composites close the gap:

| Composite | Components | Question it answers | ~Rate |
|---|---|---|---|
| **Voice** | speaks-up + cooperates + shares-info | Are people communicating? | 48% |
| **Leadership** | directs + monitors | Are people directing and overseeing? | 15% |
| **Drift** | normalises + adapts-to | Are procedures being bypassed? | 8% |
| **Care** | cares-for + responds-to-failure + protects | Does the site respond when things go wrong? | 24% |
| **Growth** | learns-from + recognises | Is the site building on what it learns? | 5% |

Five composites, 12 edge types, 100% coverage.

### Rates not percentages

Because the five composites sum to 100%, they are compositional data — a zero-sum game. If Voice goes up, one or more of the others must go down by the same amount. This makes trending misleading: an increase in Voice could mean people are speaking up more, or it could mean Leadership dropped (fewer people directing). You can't tell which.

**Solution: express composites as rates per narrative** (edges per narrative), not as percentages of total edges. Each composite becomes independent — Voice can go up without anything else going down. The question "are people communicating more?" is answered by "Voice edges per narrative rose from 1.1 to 1.3" — a real increase, not a compositional artifact.

### Headcount normalisation

Rates per narrative measure report quality (how rich is each report in cultural signal). A complementary rate — **edges per headcount** — measures workforce participation: how much cultural signal is the workforce generating per capita?

| Rate | What it answers |
|---|---|
| Edges per narrative | How rich are reports? (report quality) |
| Edges per headcount | How much signal is the workforce generating? (participation + quality) |

A site could have excellent edges-per-narrative (rich reports) but poor edges-per-headcount (hardly anyone reports). The headcount rate captures both dimensions.

Per-site headcount is not maintained in the cultural graph pipeline. Headcount data is ingested alongside narratives and the per-capita rate is computed downstream in PowerBI. The pipeline exports the raw counts (narratives, edge counts per composite, per site, per year) in the PowerBI CSV; PowerBI joins headcount and computes the rate.

### Summary of changes (implemented)

1. **Add Leadership and Growth composites** to `generate_report.py` (template, tracker, and CSV modes) — done
2. **Switch from percentages to rates per narrative** in the tracker and template reports — done
3. **Ensure the PowerBI CSV exports raw edge counts per composite** — done (five rate columns plus 12 per-type raw counts)
4. **Update briefs** to use the five-composite model and rate-based language — done

---

## Report type confound — v0.3

### The problem

Blended rates per narrative confound two signals: (1) genuine cultural differences between sites and (2) the report type distribution at each site.

Different report types produce dramatically different composite rates:

| Report type | N | Voice | Leadership | Drift | Care | Growth |
|---|---|---|---|---|---|---|
| Positive Observations | 2,194 | 1.30 | 0.50 | 0.16 | 0.45 | **0.42** |
| Hazard & Observations | 5,419 | 1.09 | 0.30 | 0.22 | 0.54 | **0.05** |
| Near Miss | 1,829 | 1.07 | 0.33 | 0.23 | 0.63 | 0.06 |
| Injury | 781 | 0.47 | 0.15 | 0.07 | 0.42 | **0.01** |

Growth differs **42x** between positive observations and injury reports. A site with 45% positive observations (PEN) will naturally show high Growth regardless of culture. A site with 5% positive observations (MAL) will show near-zero Growth.

The report type mix varies dramatically across sites:
- **PEN**: 45% positive observations → inflates Voice and Growth
- **MAL**: 5% positive observations → deflates Growth
- **FRN**: 7% positive observations → deflates Growth

Within a single report type, site differentiation is **much sharper**. For Hazard & Observations: ASH Voice 0.49 vs PEN 1.98 vs MHA 0.27 — a 7x range compared to ~3x in the blended view.

### Three approaches (all valid, different use cases)

#### Approach 1: Per-report-type dashboards

Show the dashboard separately for each major report type. Most honest — each table compares like with like.

**Use case**: site-level deep dives. A site leader wants to know "how do our hazard reports compare to other sites' hazard reports?" Not confounded by report mix.

**Implementation**: `generate_report.py --template --report-type "Positive Observations"`. Filter both the dashboard and the temporal trajectory to a single report type. The CSV already exports per-report-type rows.

#### Approach 2: Report-type-adjusted rates

Compute the expected rate for each site given its report type mix, then report the residual (observed minus expected). A site that scores Voice 1.5 but is expected at 1.3 given its report mix has an adjusted Voice of +0.2. A site at Voice 1.5 but expected at 1.5 has adjusted Voice of 0.0 — its score is entirely explained by report mix.

**Use case**: organisational comparison. "Which sites genuinely differ from what we'd expect given their report types?" Statistically the most defensible approach for cross-site ranking.

**Implementation**: `generate_report.py --template --adjusted`. Computed in Python/DuckDB, not downstream in PowerBI — the adjustment should be available in the standard report template so anyone can read it without BI tooling. Steps:
1. Compute org-wide baseline rate per report type (e.g. Voice = 1.30 for Positive Observations, 1.09 for Hazard & Observations, etc.)
2. For each site, compute expected rate = sum(baseline_rate × site's proportion of that report type)
3. Adjusted rate = observed rate − expected rate. Positive = site produces more than expected given its report mix. Negative = less.

#### Approach 3: Single report type as benchmark

Pick one report type (Hazard & Observations or Positive Observations) and only profile sites on that type. Simplest to explain: "we compare how sites describe hazards."

**Use case**: C-suite simplicity. One table, one comparison, no confounds. The trade-off is discarding 50–80% of data per site.

**Implementation**: filter in `generate_report.py --template --report-type "Hazard & Observations"`. Already possible once Approach 1 is built.

### Recommendation

Build **Approach 1** first (`--report-type` filter) — small extension, enables Approach 3 for free. Then build **Approach 2** (`--adjusted` flag) — compute adjusted rates in Python so they appear in the standard report template. All three approaches live in `generate_report.py`, not downstream in PowerBI.

### Decision (implemented August 2026)

All three approaches built. The key insight: blended rates without adjustment are not a valid alternative view — they are confounded. Adjusted rates are now the **default** for `--template` and `--monthly-tracker`. `--raw` flag available for unadjusted view. `--report-type` skips adjustment automatically (single type = no mix to control for).

---

## Statistical rigour — v0.4

### Current approach

Indirect standardisation via residuals: for each site, compute expected rate given its report type mix, report observed − expected. Flag at ±30% of org average. This is valid but has several areas where standard statistical methods would strengthen the analysis.

### 1. SMR-style ratios instead of residuals (high priority)

The standard epidemiological form of indirect standardisation is the **Standardised Morbidity Ratio (SMR)**: `observed / expected`, not `observed - expected`.

**Why ratios are better:**
- Scale-invariant: a residual of +0.5 means different things for a composite with baseline 0.2 vs 5.0. The ratio (2.5x vs 1.1x) communicates magnitude correctly.
- Directly interpretable for C-suite: "Site X produces 1.4x the expected Voice given its report mix" is clearer than "+0.3 residual."
- Confidence intervals are well-established: for count data, the exact Poisson CI on the SMR uses chi-squared bounds, implementable in Python with `scipy.stats.chi2.ppf`. A site is flagged only when the CI excludes 1.0 — replacing the arbitrary 30% threshold with statistical significance.

**Implementation**: change division to ratio in `generate_template`, add ~5 lines of scipy for CIs.

### 2. Funnel plots (high communication value)

The standard visualisation for comparing institutional rates against a population mean. Used by NHS for hospital comparison and CQC for care home monitoring — directly analogous to this use case.

Sites plotted with rate on y-axis, volume (narrative count) on x-axis, with 95%/99.8% control limits drawn as a funnel. Small sites naturally get wider limits, visually explaining why their outlier flags may be noise.

**Implementation**: ~30 lines of matplotlib/plotly. Exceptionally effective for C-suite audiences.

### 3. Negative binomial regression (rigorous foundation)

Edge counts per narrative are likely **overdispersed** (variance exceeds mean) because narratives cluster by author, site culture, and incident severity. A Poisson model assumes variance equals mean and will produce artificially narrow CIs, over-flagging sites.

Negative binomial regression (`statsmodels.GLM` with `NegativeBinomial` family) handles this. The model: `edges ~ report_type + site` with `log(n_narratives)` as exposure offset. Gives:
- Rate ratio per site vs org mean with proper p-values
- CIs that account for overdispersion
- A single model replacing ad-hoc rate arithmetic

Runs in <1 second at 11K rows via `statsmodels`.

### 4. FDR correction for multiple comparisons (medium priority)

5 composites × 68 sites = 340 comparisons. At alpha=0.05, ~17 false flags expected by chance. **Benjamini-Hochberg FDR** correction (`scipy.stats.false_discovery_control()` or `statsmodels.stats.multitest.multipletests(method='fdr_bh')`) controls the expected proportion of false flags — appropriate for a screening/flagging use case. Less conservative than Bonferroni, which would be too aggressive for correlated composites.

### 5. Temporal slope analysis (trajectory insight)

With 6 financial years, a site's trajectory matters more than its point estimate. A simple year-on-year slope per site (linear regression on annual rates) flags improving/deteriorating sites. High C-suite value: "Site X is trending up on Drift" is more actionable than "Site X has high Drift."

### 6. Lower priority considerations

- **Random effects / multilevel models**: treating site as a random effect (via `statsmodels.MixedLM` or Bayesian shrinkage) pulls small-site estimates toward the org mean, reducing false flags from small samples. Adds complexity.
- **Zero-inflation**: if many narratives produce zero edges for some composites, a zero-inflated negative binomial model may fit better. Only pursue if excess zeros are observed beyond what negative binomial predicts.

### Recommendation

Priority order for implementation:
1. **SMR ratios with Poisson CIs** — immediate improvement, minimal code change
2. **Funnel plots** — strongest communication tool for C-suite
3. **Negative binomial regression** — proper statistical foundation
4. **FDR correction** — eliminates false flags from multiple testing
5. **Temporal slopes** — trajectory analysis
