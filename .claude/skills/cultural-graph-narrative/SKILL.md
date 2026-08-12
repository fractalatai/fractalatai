---
description: Generate AI-curated prose narrative reports for cultural graph sites. Produces site data cards (L3 in the documentation suite) with statistical context, trend commentary, and actionable insights. Use when the user asks for a site-specific narrative, a site data card, or a prose summary of cultural graph results.
---

# Cultural Graph: Narrative Report

## When This Applies

After the reporting pipeline has run (`/cultural-graph-report`). This skill generates prose insights that go beyond the tabular reports — interpreting the numbers, highlighting what's changed, and suggesting what to investigate.

Use cases:
- **Site data card**: 1-page prose summary for a specific site (for site safety leads)
- **Monthly narrative**: prose summary of the latest monthly tracker entry (for leadership meetings)
- **Sector briefing**: comparative narrative across sites in a sector

## How It Works

Query DuckDB directly, compute the same statistics as the pipeline, then write prose. All data comes from `data/cultural-graph.duckdb`.

### Site data card

For a specific site, generate a markdown document covering:

1. **Header**: site name, sector, narrative count, years of data
2. **Prevalence**: what % of this site's narratives carry cultural signal, vs org average
3. **SRR profile**: each composite's SRR with interpretation (above/below/as expected)
4. **Flags**: which composites are flagged and what that means in plain language
5. **Trend**: if 3+ years of data, is any composite trending up or down?
6. **Report type mix**: what types of reports this site submits, and how that affects expectations
7. **Context**: how this site compares to its sector peers
8. **Suggested actions**: what to investigate based on the flags and trends

### Data extraction

Use this Python pattern to extract site data:

```python
import sys, math
sys.path.insert(0, 'scripts/cultural-graph')
from generate_report import *
from scipy.stats import norm

con = get_connection()
site = '<SITE_NAME>'  # e.g., '6.14 ESK'
composites = ['voice', 'leadership', 'drift', 'care', 'growth']
COMP_NAMES = ['Voice', 'Leadership', 'Drift', 'Care', 'Growth']

# 1. Basic stats
stats = con.execute(f"""
    SELECT COUNT(*) AS n, COUNT(DISTINCT fy) AS years,
        COUNT(*) FILTER (cultural_edge_count > 0) AS with_signal,
        ROUND(COUNT(*) FILTER (cultural_edge_count > 0)::FLOAT / COUNT(*) * 100, 0) AS signal_pct,
        MIN(fy) AS first_fy, MAX(fy) AS last_fy, sector
    FROM narratives WHERE site = '{site}' GROUP BY sector
""").fetchone()

# 2. Org-level standardised averages
prevalence_org, intensity_org, rt_baselines, org_cis = compute_standardised_averages(con)

# 3. Report type mix
rt_mix = con.execute(f"""
    SELECT report_type, COUNT(*) AS n,
        ROUND(COUNT(*)::FLOAT / (SELECT COUNT(*) FROM narratives WHERE site = '{site}') * 100, 0) AS pct
    FROM narratives WHERE site = '{site}' GROUP BY report_type ORDER BY n DESC
""").fetchdf()

# 4. Site-level edge rates
n = float(stats[0])
site_edges = con.execute(f"""
    SELECT
        COUNT(*) FILTER (e.edge_type IN ('speaks-up-to', 'cooperates-with', 'shares-information-with'))::FLOAT / {n} AS voice,
        COUNT(*) FILTER (e.edge_type IN ('directs', 'monitors'))::FLOAT / {n} AS leadership,
        COUNT(*) FILTER (e.edge_type IN ('normalises', 'adapts-to'))::FLOAT / {n} AS drift,
        COUNT(*) FILTER (e.edge_type IN ('cares-for', 'responds-to-failure-of', 'protects'))::FLOAT / {n} AS care,
        COUNT(*) FILTER (e.edge_type IN ('learns-from', 'recognises'))::FLOAT / {n} AS growth
    FROM narratives nn JOIN edges e ON e.narrative_id = nn.id
    WHERE nn.site = '{site}' AND e.is_cultural = true
""").fetchone()

# 5. Compute SRR per composite
baselines = con.execute("""
    WITH rt_narr AS (SELECT report_type, COUNT(*) AS n_narr FROM narratives GROUP BY report_type),
    rt_edges AS (
        SELECT n.report_type,
            COUNT(*) FILTER (e.edge_type IN ('speaks-up-to', 'cooperates-with', 'shares-information-with')) AS voice_e,
            COUNT(*) FILTER (e.edge_type IN ('directs', 'monitors')) AS leadership_e,
            COUNT(*) FILTER (e.edge_type IN ('normalises', 'adapts-to')) AS drift_e,
            COUNT(*) FILTER (e.edge_type IN ('cares-for', 'responds-to-failure-of', 'protects')) AS care_e,
            COUNT(*) FILTER (e.edge_type IN ('learns-from', 'recognises')) AS growth_e
        FROM narratives n JOIN edges e ON e.narrative_id = n.id WHERE e.is_cultural = true GROUP BY n.report_type)
    SELECT rn.report_type,
        COALESCE(re.voice_e, 0)::FLOAT / rn.n_narr AS voice_bl,
        COALESCE(re.leadership_e, 0)::FLOAT / rn.n_narr AS leadership_bl,
        COALESCE(re.drift_e, 0)::FLOAT / rn.n_narr AS drift_bl,
        COALESCE(re.care_e, 0)::FLOAT / rn.n_narr AS care_bl,
        COALESCE(re.growth_e, 0)::FLOAT / rn.n_narr AS growth_bl
    FROM rt_narr rn LEFT JOIN rt_edges re ON rn.report_type = re.report_type
""").fetchdf()
bl_lookup = {}
for _, bl_row in baselines.iterrows():
    bl_lookup[bl_row['report_type']] = {c: float(bl_row[f'{c}_bl']) for c in composites}

site_rt = con.execute(f"SELECT report_type, COUNT(*) AS n FROM narratives WHERE site = '{site}' GROUP BY report_type").fetchdf()
site_total = site_rt['n'].sum()

phis = compute_overdispersion(con, composites)

smrs = {}
for i, c in enumerate(composites):
    observed = round(site_edges[i] * n)
    expected_rate = sum(
        (sr['n'] / site_total) * bl_lookup.get(sr['report_type'], {}).get(c, 0.0)
        for _, sr in site_rt.iterrows()
    )
    expected = expected_rate * n
    smr, lo, hi = smr_poisson_ci(observed, expected, phi=phis.get(c, 1.0))
    smrs[c] = {'smr': smr, 'lo': lo, 'hi': hi, 'O': observed, 'E': expected}
```

### Output format

Write the data card as markdown, structured for PPTX copy/paste (short sections, bold key numbers, max 6 bullets per section). Save to `data/qq/cultural-graph/reports/site-cards/`.

Example output structure:

```markdown
# Site Data Card: ESK

**Sector:** UKD | **Narratives:** 197 | **Years:** FY2022–FY2027 | **Generated:** 2026-08-12

---

## Cultural Signal

**76% of this site's narratives contain cultural signal** (org average: 56%).
ESK's workforce writes reflectively — narratives describe not just what happened
but who was involved and how they interacted.

## SRR Profile

| Composite | SRR | Status | Interpretation |
|-----------|-----|--------|---------------|
| **Voice** | 1.64 | HIGH | 64% more communication signal than expected |
| **Leadership** | 2.17 | HIGH | More than double the expected direction/oversight |
| **Drift** | 0.88 | — | As expected |
| **Care** | 1.33 | HIGH | 33% more failure response than expected |
| **Growth** | 2.06 | HIGH | Double the expected learning/recognition signal |

## What This Means

ESK has the richest cultural signal of any site in the dataset...
[AI-generated prose interpreting the specific pattern]

## Suggested Actions

- Validate with site leads: is the high signal genuine or a narrative style effect?
- ...
```

## Guidelines

- **Use the pipeline functions** (`smr_poisson_ci`, `compute_overdispersion`, `empirical_bayes_shrinkage`, `compute_standardised_averages`) — don't recompute statistics from scratch
- **Write for a site safety lead**, not a statistician. "64% more communication signal than expected" not "SRR = 1.64, p < 0.05"
- **Be specific about what to investigate**, not just "investigate further"
- **Note limitations**: small sites have wide CIs; EB shrinkage may have pulled the estimate; short time series limit trend detection
- **PPTX-friendly**: sections sized to one slide, tables max 5 columns, bold key numbers
- **Save site cards** to `data/qq/cultural-graph/reports/site-cards/<site-slug>.md`
- **Always include the Spongl footer**: end every site card with `*The Cultural Graph(TM) is a trademark of Spongl Ltd. All rights reserved. (C) Spongl Ltd 2026.*`

## File Locations

| File | Purpose |
|------|---------|
| `scripts/cultural-graph/generate_report.py` | Pipeline functions (import and reuse) |
| `data/cultural-graph.duckdb` | Source data |
| `data/qq/cultural-graph/reports/site-cards/` | Output directory for site data cards |
