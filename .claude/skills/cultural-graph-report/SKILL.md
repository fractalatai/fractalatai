---
description: Generate cultural graph reports and dashboards from DuckDB. Templated markdown reports, PowerBI CSV exports, and per-year or all-years analysis. Step 4 of the monthly cultural graph workflow.
---

# Cultural Graph: Reporting

## When This Applies

After `/cultural-graph-load` has updated DuckDB with new extraction results. This is step 4 of the monthly workflow:

1. `/cultural-graph-ingest` → cleaned JSONL
2. `/cultural-graph-runpod` → inference results
3. `/cultural-graph-load` → DuckDB updated
4. **Report** (this skill) → markdown report + PowerBI CSV

## Usage

### Part A: Templated narrative report

Standard Voice/Drift/Care dashboard, "Sites to Watch", temporal trajectory.

```bash
# All years
/usr/bin/python3 scripts/cultural-graph/generate_report.py --template

# Specific financial year
/usr/bin/python3 scripts/cultural-graph/generate_report.py --template --fy 2027
```

Output: `data/qq/cultural-graph/outputs/reports/cultural-graph-report-{all|fyNNNN}.md`

The report auto-generates:
- Overview stats (narratives, sites, cultural edges)
- Executive dashboard with Voice/Drift/Care per site, outliers flagged as **HIGH** or **LOW**
- "Sites to Watch" — top outliers with one-sentence explanations
- Temporal trajectory — speaks-up, normalises, cares-for trends by FY

### Part B: Bespoke analysis

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

### Part C: PowerBI CSV export

Flat CSV with per-site, per-year, per-report-type breakdowns — designed for corporate BI tool ingestion.

```bash
/usr/bin/python3 scripts/cultural-graph/generate_report.py --csv
```

Output: `data/qq/cultural-graph/outputs/reports/cultural-graph-powerbi.csv`

Columns: site, fy, report_type, sector, narratives, cultural_edges, operational_edges, avg_cultural_per_narrative, [12 edge type counts], voice_pct, drift_pct, care_pct

## Report Indicators

| Indicator | Components | Org average | Flag threshold |
|-----------|-----------|-------------|----------------|
| **Voice** | speaks-up + cooperates + shares-info | ~47% | LOW <40%, HIGH >54% |
| **Drift** | normalises + adapts-to | ~9% | HIGH >12% |
| **Care** | cares-for + responds-to-failure + protects | ~24% | HIGH >31% |

## File Locations

| File | Purpose |
|------|---------|
| `scripts/cultural-graph/generate_report.py` | Report generation script |
| `data/qq/cultural-graph/outputs/reports/` | Output directory for reports and CSVs |
| `data/cultural-graph.duckdb` | Source data |

## Notes

- Reports are generated from DuckDB, not from raw JSONL — always run `/cultural-graph-load` first
- The templated report is designed for C-suite consumption — three numbers per site, traffic-light outlier flagging, one-sentence explanations
- The PowerBI CSV is one row per site-year-report_type combination — pivot/filter in PowerBI
- Bespoke analysis uses DuckDB SQL directly — no script needed, just query
