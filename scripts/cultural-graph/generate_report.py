#!/usr/bin/env python3
"""Generate cultural graph reports from DuckDB production data.

Three output modes:
  --template   Standard markdown narrative report (C-suite dashboard format)
  --csv        Flat CSV for PowerBI ingestion
  --bespoke    Ad-hoc query mode (interactive or with --query)

Usage:
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --template
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --template --fy 2027
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --csv
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --csv --output data/qq/cultural-graph/exports/dashboard.csv
"""

import argparse
import csv as csv_mod
import sys
from datetime import datetime
from pathlib import Path

import duckdb

DUCKDB_PATH = Path("data/cultural-graph.duckdb")
OUTPUT_DIR = Path("data/qq/cultural-graph/outputs/reports")

EDGE_TYPES = [
    "shares-information-with", "speaks-up-to", "responds-to-failure-of",
    "directs", "cooperates-with", "protects", "monitors", "normalises",
    "adapts-to", "learns-from", "recognises", "cares-for",
]


def get_connection():
    return duckdb.connect(str(DUCKDB_PATH), read_only=True)


def generate_template(con, fy=None):
    """Generate standard markdown report."""
    # Overall stats
    if fy:
        where = f"WHERE n.fy = {fy}"
        title_suffix = f" — FY{fy}"
    else:
        where = ""
        title_suffix = " — All Years"

    stats = con.execute(f"""
        SELECT
            count(DISTINCT n.id) AS narratives,
            count(DISTINCT n.site) AS sites,
            sum(n.cultural_edge_count) AS cultural_edges,
            sum(n.operational_edge_count) AS operational_edges,
            count(DISTINCT n.report_type) AS report_types
        FROM narratives n {where}
    """).fetchone()

    # Site dashboard
    dashboard = con.execute(f"""
        SELECT
            n.site,
            count(DISTINCT n.id) AS "N",
            ROUND(100.0 * count(*) FILTER (e.edge_type IN ('speaks-up-to', 'cooperates-with', 'shares-information-with'))
                / NULLIF(count(*) FILTER (e.is_cultural), 0), 0) AS "Voice%",
            ROUND(100.0 * count(*) FILTER (e.edge_type IN ('normalises', 'adapts-to'))
                / NULLIF(count(*) FILTER (e.is_cultural), 0), 0) AS "Drift%",
            ROUND(100.0 * count(*) FILTER (e.edge_type IN ('cares-for', 'responds-to-failure-of', 'protects'))
                / NULLIF(count(*) FILTER (e.is_cultural), 0), 0) AS "Care%"
        FROM narratives n
        JOIN edges e ON e.narrative_id = n.id
        {where}
        GROUP BY n.site
        HAVING count(DISTINCT n.id) >= 10
        ORDER BY count(DISTINCT n.id) DESC
    """).fetchdf()

    # Org averages
    avg_voice = dashboard["Voice%"].mean()
    avg_drift = dashboard["Drift%"].mean()
    avg_care = dashboard["Care%"].mean()

    # Temporal trajectory
    temporal = con.execute("""
        SELECT
            n.fy,
            count(DISTINCT n.id) AS narratives,
            ROUND(100.0 * count(*) FILTER (e.edge_type = 'speaks-up-to')
                / NULLIF(count(*) FILTER (e.is_cultural), 0), 1) AS "speak%",
            ROUND(100.0 * count(*) FILTER (e.edge_type = 'normalises')
                / NULLIF(count(*) FILTER (e.is_cultural), 0), 1) AS "norm%",
            ROUND(100.0 * count(*) FILTER (e.edge_type = 'cares-for')
                / NULLIF(count(*) FILTER (e.is_cultural), 0), 1) AS "care%"
        FROM narratives n
        JOIN edges e ON e.narrative_id = n.id
        GROUP BY n.fy
        ORDER BY n.fy
    """).fetchdf()

    # Find outlier sites
    high_drift = dashboard[dashboard["Drift%"] > avg_drift * 1.3].sort_values("Drift%", ascending=False)
    low_voice = dashboard[dashboard["Voice%"] < avg_voice * 0.85].sort_values("Voice%")
    high_care = dashboard[dashboard["Care%"] > avg_care * 1.3].sort_values("Care%", ascending=False)

    # Build report
    report = []
    report.append(f"# Cultural Graph Report{title_suffix}")
    report.append(f"\n*Generated {datetime.now().strftime('%Y-%m-%d %H:%M')}*")
    report.append(f"\n---")
    report.append(f"\n## Overview")
    report.append(f"\n| Metric | Value |")
    report.append(f"|--------|-------|")
    report.append(f"| Narratives analysed | {stats[0]:,} |")
    report.append(f"| Sites | {stats[1]} |")
    report.append(f"| Cultural relationships | {stats[2]:,} |")
    report.append(f"| Report types | {stats[4]} |")

    report.append(f"\n## Executive Dashboard")
    report.append(f"\nThree indicators per site compared to organisational average:")
    report.append(f"- **Voice** ({avg_voice:.0f}% avg) — are people communicating about safety?")
    report.append(f"- **Drift** ({avg_drift:.0f}% avg) — are procedures being bypassed?")
    report.append(f"- **Care** ({avg_care:.0f}% avg) — does the site respond when things go wrong?")
    report.append(f"\n| Site | N | Voice% | Drift% | Care% |")
    report.append(f"|------|---|--------|--------|-------|")
    for _, row in dashboard.iterrows():
        voice_flag = " **LOW**" if row["Voice%"] < avg_voice * 0.85 else (" **HIGH**" if row["Voice%"] > avg_voice * 1.15 else "")
        drift_flag = " **HIGH**" if row["Drift%"] > avg_drift * 1.3 else ""
        care_flag = " **HIGH**" if row["Care%"] > avg_care * 1.3 else ""
        report.append(f"| {row['site']} | {row['N']:.0f} | {row['Voice%']:.0f}{voice_flag} | {row['Drift%']:.0f}{drift_flag} | {row['Care%']:.0f}{care_flag} |")

    report.append(f"\n## Sites to Watch")
    watch_count = 0
    if len(high_drift) > 0:
        for _, row in high_drift.head(2).iterrows():
            watch_count += 1
            report.append(f"\n{watch_count}. **{row['site']}** — Drift at {row['Drift%']:.0f}% (org avg {avg_drift:.0f}%). Narratives describe procedural deviations as routine.")
    if len(low_voice) > 0:
        for _, row in low_voice.head(1).iterrows():
            watch_count += 1
            report.append(f"\n{watch_count}. **{row['site']}** — Voice at {row['Voice%']:.0f}% (org avg {avg_voice:.0f}%). Low communication signal — people are not describing challenge or information-sharing behaviours.")
    if len(high_care) > 0 and watch_count < 3:
        for _, row in high_care.head(1).iterrows():
            watch_count += 1
            report.append(f"\n{watch_count}. **{row['site']}** — Care at {row['Care%']:.0f}% (org avg {avg_care:.0f}%). Welfare response significantly above average — investigate whether this reflects significant incidents.")

    report.append(f"\n## Temporal Trajectory")
    report.append(f"\n| FY | Narratives | Speaks-up% | Normalises% | Cares-for% |")
    report.append(f"|----|-----------|-----------|------------|-----------|")
    for _, row in temporal.iterrows():
        report.append(f"| {row['fy']:.0f} | {row['narratives']:,.0f} | {row['speak%']:.1f} | {row['norm%']:.1f} | {row['care%']:.1f} |")

    report.append(f"\n---")
    report.append(f"\n*Report generated from {DUCKDB_PATH}. Cultural relationships extracted by Qwen 3 8B fine-tuned model.*")

    return "\n".join(report)


def generate_csv(con, output_path):
    """Generate flat CSV for PowerBI."""
    df = con.execute("""
        SELECT
            n.site,
            n.fy,
            n.report_type,
            n.sector,
            count(DISTINCT n.id) AS narratives,
            sum(n.cultural_edge_count) AS cultural_edges,
            sum(n.operational_edge_count) AS operational_edges,
            ROUND(AVG(n.cultural_edge_count), 2) AS avg_cultural_per_narrative,
            count(*) FILTER (e.edge_type = 'shares-information-with') AS shares_info,
            count(*) FILTER (e.edge_type = 'speaks-up-to') AS speaks_up,
            count(*) FILTER (e.edge_type = 'responds-to-failure-of') AS responds_failure,
            count(*) FILTER (e.edge_type = 'directs') AS directs,
            count(*) FILTER (e.edge_type = 'cooperates-with') AS cooperates,
            count(*) FILTER (e.edge_type = 'protects') AS protects,
            count(*) FILTER (e.edge_type = 'monitors') AS monitors,
            count(*) FILTER (e.edge_type = 'normalises') AS normalises,
            count(*) FILTER (e.edge_type = 'adapts-to') AS adapts_to,
            count(*) FILTER (e.edge_type = 'learns-from') AS learns_from,
            count(*) FILTER (e.edge_type = 'recognises') AS recognises,
            count(*) FILTER (e.edge_type = 'cares-for') AS cares_for,
            ROUND(100.0 * count(*) FILTER (e.edge_type IN ('speaks-up-to', 'cooperates-with', 'shares-information-with'))
                / NULLIF(count(*) FILTER (e.is_cultural), 0), 1) AS voice_pct,
            ROUND(100.0 * count(*) FILTER (e.edge_type IN ('normalises', 'adapts-to'))
                / NULLIF(count(*) FILTER (e.is_cultural), 0), 1) AS drift_pct,
            ROUND(100.0 * count(*) FILTER (e.edge_type IN ('cares-for', 'responds-to-failure-of', 'protects'))
                / NULLIF(count(*) FILTER (e.is_cultural), 0), 1) AS care_pct
        FROM narratives n
        JOIN edges e ON e.narrative_id = n.id
        GROUP BY n.site, n.fy, n.report_type, n.sector
        HAVING count(DISTINCT n.id) >= 3
        ORDER BY n.site, n.fy, n.report_type
    """).fetchdf()

    df.to_csv(output_path, index=False)
    print(f"CSV export: {output_path} ({len(df)} rows)")
    return df


def main():
    parser = argparse.ArgumentParser(description="Cultural graph reporting")
    parser.add_argument("--template", action="store_true", help="Generate standard markdown report")
    parser.add_argument("--csv", action="store_true", help="Generate PowerBI CSV export")
    parser.add_argument("--fy", type=int, help="Filter to specific financial year")
    parser.add_argument("--output", help="Output file path (default: auto-named)")
    args = parser.parse_args()

    if not args.template and not args.csv:
        parser.error("Specify --template or --csv")

    con = get_connection()

    if args.template:
        report = generate_template(con, fy=args.fy)
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        if args.output:
            out_path = Path(args.output)
        else:
            suffix = f"-fy{args.fy}" if args.fy else "-all"
            out_path = OUTPUT_DIR / f"cultural-graph-report{suffix}.md"
        out_path.write_text(report)
        print(f"Report: {out_path}")

    if args.csv:
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        if args.output:
            out_path = Path(args.output)
        else:
            out_path = OUTPUT_DIR / "cultural-graph-powerbi.csv"
        generate_csv(con, out_path)

    con.close()


if __name__ == "__main__":
    main()
