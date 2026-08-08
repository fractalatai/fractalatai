#!/usr/bin/env python3
"""Generate cultural graph reports from DuckDB production data.

Four output modes:
  --template          Standard markdown narrative report (C-suite dashboard format)
  --csv               Flat CSV for PowerBI ingestion
  --monthly-tracker   Append-only monthly scorecard with flag change detection
  --bespoke           Ad-hoc query mode (interactive or with --query)

Composites (rates per narrative, five indicators covering all 12 edge types):
  Voice      = speaks-up-to + cooperates-with + shares-information-with
  Leadership = directs + monitors
  Drift      = normalises + adapts-to
  Care       = cares-for + responds-to-failure-of + protects
  Growth     = learns-from + recognises

Usage:
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --template
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --template --fy 2027
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --csv
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --csv --output data/qq/cultural-graph/exports/dashboard.csv
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --monthly-tracker
"""

import argparse
import json
import sys
from datetime import datetime
from pathlib import Path

import duckdb

DUCKDB_PATH = Path("data/cultural-graph.duckdb")
OUTPUT_DIR = Path("data/qq/cultural-graph/outputs/reports")
TRACKER_PATH = OUTPUT_DIR / "monthly-tracker.md"
TRACKER_STATE_PATH = OUTPUT_DIR / ".monthly-tracker-state.json"

EDGE_TYPES = [
    "shares-information-with", "speaks-up-to", "responds-to-failure-of",
    "directs", "cooperates-with", "protects", "monitors", "normalises",
    "adapts-to", "learns-from", "recognises", "cares-for",
]

# Five composites covering all 12 cultural edge types
VOICE = ("speaks-up-to", "cooperates-with", "shares-information-with")
LEADERSHIP = ("directs", "monitors")
DRIFT = ("normalises", "adapts-to")
CARE = ("cares-for", "responds-to-failure-of", "protects")
GROWTH = ("learns-from", "recognises")

COMPOSITE_NAMES = ["Voice", "Leadership", "Drift", "Care", "Growth"]

# Flag thresholds: LOW < avg × 0.7, HIGH > avg × 1.3 (30% deviation from population mean)
FLAG_LOW = 0.7
FLAG_HIGH = 1.3


def get_connection():
    return duckdb.connect(str(DUCKDB_PATH), read_only=True)


def generate_template(con, fy=None, report_type=None, raw=False):
    """Generate standard markdown report with rates per narrative.

    By default, rates are adjusted for report type mix (observed minus expected
    given each site's report type distribution). Use raw=True for unadjusted
    blended rates. When report_type is set, adjustment is unnecessary (single
    report type = no mix to control for) and is skipped automatically.
    """
    # Adjustment applies when aggregating across report types (no --report-type filter)
    adjusted = not raw and not report_type

    narr_conds, join_conds, title_parts = [], [], []
    if fy:
        narr_conds.append(f"fy = {fy}")
        join_conds.append(f"n.fy = {fy}")
        title_parts.append(f"FY{fy}")
    if report_type:
        safe_rt = report_type.replace("'", "''")
        narr_conds.append(f"report_type = '{safe_rt}'")
        join_conds.append(f"n.report_type = '{safe_rt}'")
        title_parts.append(report_type)
    if raw and not report_type:
        title_parts.append("Raw")

    narr_filter = ("WHERE " + " AND ".join(narr_conds)) if narr_conds else ""
    fy_filter = ("AND " + " AND ".join(join_conds)) if join_conds else ""
    title_suffix = (" — " + ", ".join(title_parts)) if title_parts else " — All Years"

    # Temporal query: filter by report_type but not fy (shows all FYs for context)
    if report_type:
        temporal_narr_filter = f"WHERE report_type = '{safe_rt}'"
        temporal_join_filter = f"AND n.report_type = '{safe_rt}'"
    else:
        temporal_narr_filter = ""
        temporal_join_filter = ""

    stats = con.execute(f"""
        SELECT COUNT(*) AS narratives, COUNT(DISTINCT site) AS sites,
               COUNT(DISTINCT report_type) AS report_types
        FROM narratives {narr_filter}
    """).fetchone()

    cultural_edges = con.execute(f"""
        SELECT COUNT(*) FROM edges e
        JOIN narratives n ON e.narrative_id = n.id
        WHERE e.is_cultural = true {fy_filter}
    """).fetchone()[0]

    # Site dashboard — rates per narrative (all narratives in denominator)
    dashboard = con.execute(f"""
        WITH site_narr AS (
            SELECT site, COUNT(*) AS n_narr FROM narratives {narr_filter} GROUP BY site
        ),
        site_edges AS (
            SELECT n.site,
                COUNT(*) FILTER (e.edge_type IN {VOICE}) AS voice_e,
                COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}) AS leadership_e,
                COUNT(*) FILTER (e.edge_type IN {DRIFT}) AS drift_e,
                COUNT(*) FILTER (e.edge_type IN {CARE}) AS care_e,
                COUNT(*) FILTER (e.edge_type IN {GROWTH}) AS growth_e
            FROM narratives n
            JOIN edges e ON e.narrative_id = n.id
            WHERE e.is_cultural = true {fy_filter}
            GROUP BY n.site
        )
        SELECT sn.site, sn.n_narr AS n,
            ROUND(COALESCE(se.voice_e, 0)::FLOAT / sn.n_narr, 2) AS voice,
            ROUND(COALESCE(se.leadership_e, 0)::FLOAT / sn.n_narr, 2) AS leadership,
            ROUND(COALESCE(se.drift_e, 0)::FLOAT / sn.n_narr, 2) AS drift,
            ROUND(COALESCE(se.care_e, 0)::FLOAT / sn.n_narr, 2) AS care,
            ROUND(COALESCE(se.growth_e, 0)::FLOAT / sn.n_narr, 2) AS growth
        FROM site_narr sn
        LEFT JOIN site_edges se ON sn.site = se.site
        WHERE sn.n_narr >= 10
        ORDER BY sn.n_narr DESC
    """).fetchdf()

    # Org averages — population mean (total edges / total narratives)
    total_narr = stats[0]
    org_avg = con.execute(f"""
        SELECT
            ROUND(COUNT(*) FILTER (e.edge_type IN {VOICE})::FLOAT / {total_narr}, 2),
            ROUND(COUNT(*) FILTER (e.edge_type IN {LEADERSHIP})::FLOAT / {total_narr}, 2),
            ROUND(COUNT(*) FILTER (e.edge_type IN {DRIFT})::FLOAT / {total_narr}, 2),
            ROUND(COUNT(*) FILTER (e.edge_type IN {CARE})::FLOAT / {total_narr}, 2),
            ROUND(COUNT(*) FILTER (e.edge_type IN {GROWTH})::FLOAT / {total_narr}, 2)
        FROM narratives n
        JOIN edges e ON e.narrative_id = n.id
        WHERE e.is_cultural = true {fy_filter}
    """).fetchone()
    avg_voice, avg_leadership, avg_drift, avg_care, avg_growth = org_avg
    avgs = dict(zip(["voice", "leadership", "drift", "care", "growth"], org_avg))
    composites = ["voice", "leadership", "drift", "care", "growth"]

    # Report-type adjustment: compute expected rate per site from report type mix,
    # then replace dashboard values with residuals (observed - expected)
    if adjusted:
        baselines = con.execute(f"""
            WITH rt_narr AS (
                SELECT report_type, COUNT(*) AS n_narr FROM narratives {narr_filter} GROUP BY report_type
            ),
            rt_edges AS (
                SELECT n.report_type,
                    COUNT(*) FILTER (e.edge_type IN {VOICE}) AS voice_e,
                    COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}) AS leadership_e,
                    COUNT(*) FILTER (e.edge_type IN {DRIFT}) AS drift_e,
                    COUNT(*) FILTER (e.edge_type IN {CARE}) AS care_e,
                    COUNT(*) FILTER (e.edge_type IN {GROWTH}) AS growth_e
                FROM narratives n
                JOIN edges e ON e.narrative_id = n.id
                WHERE e.is_cultural = true {fy_filter}
                GROUP BY n.report_type
            )
            SELECT rn.report_type,
                COALESCE(re.voice_e, 0)::FLOAT / rn.n_narr AS voice_bl,
                COALESCE(re.leadership_e, 0)::FLOAT / rn.n_narr AS leadership_bl,
                COALESCE(re.drift_e, 0)::FLOAT / rn.n_narr AS drift_bl,
                COALESCE(re.care_e, 0)::FLOAT / rn.n_narr AS care_bl,
                COALESCE(re.growth_e, 0)::FLOAT / rn.n_narr AS growth_bl
            FROM rt_narr rn
            LEFT JOIN rt_edges re ON rn.report_type = re.report_type
        """).fetchdf()

        site_rt = con.execute(f"""
            SELECT site, report_type, COUNT(*) AS n
            FROM narratives {narr_filter}
            GROUP BY site, report_type
        """).fetchdf()

        bl_lookup = {}
        for _, bl_row in baselines.iterrows():
            bl_lookup[bl_row["report_type"]] = {
                c: float(bl_row[f"{c}_bl"]) for c in composites
            }

        # Upcast to float64 so residuals can be stored back
        for c in composites:
            dashboard[c] = dashboard[c].astype(float)

        for idx, row in dashboard.iterrows():
            site_data = site_rt[site_rt["site"] == row["site"]]
            site_total = site_data["n"].sum()
            for c in composites:
                expected = sum(
                    (sr["n"] / site_total) * bl_lookup.get(sr["report_type"], {}).get(c, 0.0)
                    for _, sr in site_data.iterrows()
                )
                dashboard.at[idx, c] = round(row[c] - expected, 2)

    # Temporal trajectory — rates per narrative by FY (filtered by report_type but not fy)
    temporal = con.execute(f"""
        WITH fy_narr AS (
            SELECT fy, COUNT(*) AS n_narr FROM narratives {temporal_narr_filter} GROUP BY fy
        ),
        fy_edges AS (
            SELECT n.fy,
                COUNT(*) FILTER (e.edge_type IN {VOICE}) AS voice_e,
                COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}) AS leadership_e,
                COUNT(*) FILTER (e.edge_type IN {DRIFT}) AS drift_e,
                COUNT(*) FILTER (e.edge_type IN {CARE}) AS care_e,
                COUNT(*) FILTER (e.edge_type IN {GROWTH}) AS growth_e
            FROM narratives n
            JOIN edges e ON e.narrative_id = n.id
            WHERE e.is_cultural = true {temporal_join_filter}
            GROUP BY n.fy
        )
        SELECT fn.fy, fn.n_narr AS narratives,
            ROUND(COALESCE(fe.voice_e, 0)::FLOAT / fn.n_narr, 2) AS voice,
            ROUND(COALESCE(fe.leadership_e, 0)::FLOAT / fn.n_narr, 2) AS leadership,
            ROUND(COALESCE(fe.drift_e, 0)::FLOAT / fn.n_narr, 2) AS drift,
            ROUND(COALESCE(fe.care_e, 0)::FLOAT / fn.n_narr, 2) AS care,
            ROUND(COALESCE(fe.growth_e, 0)::FLOAT / fn.n_narr, 2) AS growth
        FROM fy_narr fn
        LEFT JOIN fy_edges fe ON fn.fy = fe.fy
        ORDER BY fn.fy
    """).fetchdf()

    # Flag outlier sites
    if adjusted:
        # For adjusted rates, flag based on magnitude of residual vs 30% of org avg
        def flag(row):
            flags = []
            for name in composites:
                threshold = avgs[name] * 0.3
                if threshold > 0:
                    if row[name] < -threshold:
                        flags.append(f"{name}:LOW")
                    elif row[name] > threshold:
                        flags.append(f"{name}:HIGH")
            return flags
    else:
        def flag(row):
            flags = []
            for name in composites:
                avg = avgs[name]
                if avg > 0:
                    if row[name] < avg * FLAG_LOW:
                        flags.append(f"{name}:LOW")
                    elif row[name] > avg * FLAG_HIGH:
                        flags.append(f"{name}:HIGH")
            return flags

    high_drift = dashboard[dashboard["drift"] > (avgs["drift"] * 0.3 if adjusted else avg_drift * FLAG_HIGH)].sort_values("drift", ascending=False)
    low_voice = dashboard[dashboard["voice"] < (-avgs["voice"] * 0.3 if adjusted else avg_voice * FLAG_LOW)].sort_values("voice")

    # Build report
    report = []
    report.append(f"# Cultural Graph Report{title_suffix}")
    report.append(f"\n*Generated {datetime.now().strftime('%Y-%m-%d %H:%M')}*")
    report.append("\n---")
    report.append("\n## Overview")
    report.append("\n| Metric | Value |")
    report.append("|--------|-------|")
    report.append(f"| Narratives analysed | {stats[0]:,} |")
    report.append(f"| Sites | {stats[1]} |")
    report.append(f"| Cultural relationships | {cultural_edges:,} |")
    report.append(f"| Report types | {stats[2]} |")

    report.append("\n## Executive Dashboard")
    if adjusted:
        report.append("\nReport-type-adjusted residuals per site (observed rate minus expected rate given report type mix).")
        report.append("Positive = more than expected, negative = less. Flags at ±30% of org average rate:")
        report.append(f"- **Voice** (baseline {avg_voice:.2f}) — communication signal vs expected")
        report.append(f"- **Leadership** (baseline {avg_leadership:.2f}) — directing/overseeing vs expected")
        report.append(f"- **Drift** (baseline {avg_drift:.2f}) — procedural bypass vs expected")
        report.append(f"- **Care** (baseline {avg_care:.2f}) — failure response vs expected")
        report.append(f"- **Growth** (baseline {avg_growth:.2f}) — learning signal vs expected")
    else:
        report.append("\nFive indicators per site — cultural edges per narrative, compared to org average:")
        report.append(f"- **Voice** ({avg_voice:.2f}) — are people communicating?")
        report.append(f"- **Leadership** ({avg_leadership:.2f}) — are people directing and overseeing?")
        report.append(f"- **Drift** ({avg_drift:.2f}) — are procedures being bypassed?")
        report.append(f"- **Care** ({avg_care:.2f}) — does the site respond when things go wrong?")
        report.append(f"- **Growth** ({avg_growth:.2f}) — is the site building on what it learns?")
    report.append("\n| Site | N | Voice | Leadership | Drift | Care | Growth |")
    report.append("|------|---|-------|------------|-------|------|--------|")
    fmt = "+.2f" if adjusted else ".2f"
    for _, row in dashboard.iterrows():
        site_flags = flag(row)
        flag_str = f" **{', '.join(site_flags)}**" if site_flags else ""
        report.append(
            f"| {row['site']} | {row['n']:.0f} | {row['voice']:{fmt}} | "
            f"{row['leadership']:{fmt}} | {row['drift']:{fmt}} | "
            f"{row['care']:{fmt}} | {row['growth']:{fmt}} |{flag_str}"
        )

    report.append("\n## Sites to Watch")
    watch_count = 0
    if len(high_drift) > 0:
        for _, row in high_drift.head(2).iterrows():
            watch_count += 1
            if adjusted:
                report.append(
                    f"\n{watch_count}. **{row['site']}** — Drift adjusted {row['drift']:+.2f} "
                    f"(genuine excess after controlling for report type mix)."
                )
            else:
                report.append(
                    f"\n{watch_count}. **{row['site']}** — Drift at {row['drift']:.2f} "
                    f"(org avg {avg_drift:.2f}). Narratives describe procedural deviations as routine."
                )
    if len(low_voice) > 0:
        for _, row in low_voice.head(1).iterrows():
            watch_count += 1
            if adjusted:
                report.append(
                    f"\n{watch_count}. **{row['site']}** — Voice adjusted {row['voice']:+.2f} "
                    f"(genuine deficit after controlling for report type mix)."
                )
            else:
                report.append(
                    f"\n{watch_count}. **{row['site']}** — Voice at {row['voice']:.2f} "
                    f"(org avg {avg_voice:.2f}). Low communication signal."
                )

    report.append("\n## Temporal Trajectory")
    report.append("\nRates per narrative by financial year:")
    report.append("\n| FY | N | Voice | Leadership | Drift | Care | Growth |")
    report.append("|---|---|-------|------------|-------|------|--------|")
    for _, row in temporal.iterrows():
        report.append(
            f"| {row['fy']:.0f} | {row['narratives']:,.0f} | {row['voice']:.2f} | "
            f"{row['leadership']:.2f} | {row['drift']:.2f} | "
            f"{row['care']:.2f} | {row['growth']:.2f} |"
        )

    report.append("\n---")
    if adjusted:
        report.append(f"\n*Report generated from {DUCKDB_PATH}. Values = adjusted residuals "
                      f"(observed rate minus expected rate given report type mix). "
                      f"Cultural relationships extracted by Qwen 3 8B fine-tuned model.*")
    else:
        report.append(f"\n*Report generated from {DUCKDB_PATH}. Rates = cultural edges per narrative. "
                      f"Cultural relationships extracted by Qwen 3 8B fine-tuned model.*")

    return "\n".join(report)


def generate_csv(con, output_path):
    """Generate flat CSV for PowerBI with per-type counts and composite rates."""
    df = con.execute(f"""
        WITH grp_narr AS (
            SELECT site, fy, report_type, sector, COUNT(*) AS n_narratives
            FROM narratives
            GROUP BY site, fy, report_type, sector
        ),
        grp_edges AS (
            SELECT n.site, n.fy, n.report_type, n.sector,
                COUNT(*) FILTER (e.is_cultural) AS cultural_edges,
                COUNT(*) FILTER (NOT e.is_cultural) AS operational_edges,
                COUNT(*) FILTER (e.edge_type = 'shares-information-with') AS shares_info,
                COUNT(*) FILTER (e.edge_type = 'speaks-up-to') AS speaks_up,
                COUNT(*) FILTER (e.edge_type = 'responds-to-failure-of') AS responds_failure,
                COUNT(*) FILTER (e.edge_type = 'directs') AS directs,
                COUNT(*) FILTER (e.edge_type = 'cooperates-with') AS cooperates,
                COUNT(*) FILTER (e.edge_type = 'protects') AS protects,
                COUNT(*) FILTER (e.edge_type = 'monitors') AS monitors,
                COUNT(*) FILTER (e.edge_type = 'normalises') AS normalises,
                COUNT(*) FILTER (e.edge_type = 'adapts-to') AS adapts_to,
                COUNT(*) FILTER (e.edge_type = 'learns-from') AS learns_from,
                COUNT(*) FILTER (e.edge_type = 'recognises') AS recognises,
                COUNT(*) FILTER (e.edge_type = 'cares-for') AS cares_for,
                COUNT(*) FILTER (e.edge_type IN {VOICE}) AS voice_edges,
                COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}) AS leadership_edges,
                COUNT(*) FILTER (e.edge_type IN {DRIFT}) AS drift_edges,
                COUNT(*) FILTER (e.edge_type IN {CARE}) AS care_edges,
                COUNT(*) FILTER (e.edge_type IN {GROWTH}) AS growth_edges
            FROM narratives n
            JOIN edges e ON e.narrative_id = n.id
            GROUP BY n.site, n.fy, n.report_type, n.sector
        )
        SELECT
            gn.site, gn.fy, gn.report_type, gn.sector,
            gn.n_narratives AS narratives,
            COALESCE(ge.cultural_edges, 0) AS cultural_edges,
            COALESCE(ge.operational_edges, 0) AS operational_edges,
            ROUND(COALESCE(ge.cultural_edges, 0)::FLOAT / gn.n_narratives, 2) AS avg_cultural_per_narrative,
            COALESCE(ge.shares_info, 0) AS shares_info,
            COALESCE(ge.speaks_up, 0) AS speaks_up,
            COALESCE(ge.responds_failure, 0) AS responds_failure,
            COALESCE(ge.directs, 0) AS directs,
            COALESCE(ge.cooperates, 0) AS cooperates,
            COALESCE(ge.protects, 0) AS protects,
            COALESCE(ge.monitors, 0) AS monitors,
            COALESCE(ge.normalises, 0) AS normalises,
            COALESCE(ge.adapts_to, 0) AS adapts_to,
            COALESCE(ge.learns_from, 0) AS learns_from,
            COALESCE(ge.recognises, 0) AS recognises,
            COALESCE(ge.cares_for, 0) AS cares_for,
            ROUND(COALESCE(ge.voice_edges, 0)::FLOAT / gn.n_narratives, 2) AS voice_rate,
            ROUND(COALESCE(ge.leadership_edges, 0)::FLOAT / gn.n_narratives, 2) AS leadership_rate,
            ROUND(COALESCE(ge.drift_edges, 0)::FLOAT / gn.n_narratives, 2) AS drift_rate,
            ROUND(COALESCE(ge.care_edges, 0)::FLOAT / gn.n_narratives, 2) AS care_rate,
            ROUND(COALESCE(ge.growth_edges, 0)::FLOAT / gn.n_narratives, 2) AS growth_rate
        FROM grp_narr gn
        LEFT JOIN grp_edges ge ON gn.site = ge.site AND gn.fy = ge.fy
            AND gn.report_type = ge.report_type AND gn.sector = ge.sector
        WHERE gn.n_narratives >= 3
        ORDER BY gn.site, gn.fy, gn.report_type
    """).fetchdf()

    df.to_csv(output_path, index=False)
    print(f"CSV export: {output_path} ({len(df)} rows)")
    return df


def generate_monthly_tracker(con, report_type=None, raw=False):
    """Generate monthly tracker entry with flag change detection.

    Appends a compact scorecard to monthly-tracker.md showing:
    - Batch volume vs cumulative totals
    - Five composite rates (population mean)
    - Temporal trajectory (all FYs)
    - Flag changes since last run (via JSON sidecar state file)
    - Currently active flags

    Flags are computed from adjusted rates by default (controlling for report
    type mix). Use raw=True for unadjusted flags. When report_type is set,
    adjustment is unnecessary and skipped.
    """
    adjusted = not raw and not report_type
    composites = ["voice", "leadership", "drift", "care", "growth"]
    now = datetime.now()

    # Build filters
    if report_type:
        safe_rt = report_type.replace("'", "''")
        narr_filter = f"WHERE report_type = '{safe_rt}'"
        narr_and = f"AND report_type = '{safe_rt}'"
        join_filter = f"AND n.report_type = '{safe_rt}'"
        rt_slug = report_type.lower().replace(" ", "-").replace("&", "and")
        state_path = OUTPUT_DIR / f".monthly-tracker-state-{rt_slug}.json"
        heading_suffix = f" ({report_type})"
    else:
        narr_filter = ""
        narr_and = ""
        join_filter = ""
        state_path = TRACKER_STATE_PATH
        heading_suffix = ""

    # Latest batch
    batch_detail = con.execute(f"""
        SELECT
            MAX(extracted_at)::DATE AS load_date,
            COUNT(*) AS narratives,
            COUNT(DISTINCT site) AS sites,
            SUM(cultural_edge_count) AS cultural_edges
        FROM narratives
        WHERE extracted_at::DATE = (SELECT MAX(extracted_at)::DATE FROM narratives {narr_filter})
            {narr_and}
    """).fetchone()

    # Cumulative totals
    cum = con.execute(f"""
        SELECT COUNT(*) AS narratives, COUNT(DISTINCT site) AS sites,
               SUM(cultural_edge_count) AS cultural_edges, COUNT(DISTINCT fy) AS years
        FROM narratives {narr_filter}
    """).fetchone()
    total_narr = cum[0]

    # Org averages — population mean
    org_avg = con.execute(f"""
        SELECT
            ROUND(COUNT(*) FILTER (e.edge_type IN {VOICE})::FLOAT / {total_narr}, 2),
            ROUND(COUNT(*) FILTER (e.edge_type IN {LEADERSHIP})::FLOAT / {total_narr}, 2),
            ROUND(COUNT(*) FILTER (e.edge_type IN {DRIFT})::FLOAT / {total_narr}, 2),
            ROUND(COUNT(*) FILTER (e.edge_type IN {CARE})::FLOAT / {total_narr}, 2),
            ROUND(COUNT(*) FILTER (e.edge_type IN {GROWTH})::FLOAT / {total_narr}, 2)
        FROM narratives n
        JOIN edges e ON e.narrative_id = n.id
        WHERE e.is_cultural = true {join_filter}
    """).fetchone()
    avg_voice, avg_leadership, avg_drift, avg_care, avg_growth = org_avg
    avgs = dict(zip(["voice", "leadership", "drift", "care", "growth"], org_avg))

    # Site dashboard — rates per narrative
    dashboard = con.execute(f"""
        WITH site_narr AS (
            SELECT site, COUNT(*) AS n_narr FROM narratives {narr_filter} GROUP BY site
        ),
        site_edges AS (
            SELECT n.site,
                COUNT(*) FILTER (e.edge_type IN {VOICE}) AS voice_e,
                COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}) AS leadership_e,
                COUNT(*) FILTER (e.edge_type IN {DRIFT}) AS drift_e,
                COUNT(*) FILTER (e.edge_type IN {CARE}) AS care_e,
                COUNT(*) FILTER (e.edge_type IN {GROWTH}) AS growth_e
            FROM narratives n
            JOIN edges e ON e.narrative_id = n.id
            WHERE e.is_cultural = true {join_filter}
            GROUP BY n.site
        )
        SELECT sn.site, sn.n_narr AS n,
            ROUND(COALESCE(se.voice_e, 0)::FLOAT / sn.n_narr, 2) AS voice,
            ROUND(COALESCE(se.leadership_e, 0)::FLOAT / sn.n_narr, 2) AS leadership,
            ROUND(COALESCE(se.drift_e, 0)::FLOAT / sn.n_narr, 2) AS drift,
            ROUND(COALESCE(se.care_e, 0)::FLOAT / sn.n_narr, 2) AS care,
            ROUND(COALESCE(se.growth_e, 0)::FLOAT / sn.n_narr, 2) AS growth
        FROM site_narr sn
        LEFT JOIN site_edges se ON sn.site = se.site
        WHERE sn.n_narr >= 10
        ORDER BY sn.n_narr DESC
    """).fetchdf()

    # Temporal trajectory — rates per narrative by FY
    temporal = con.execute(f"""
        WITH fy_narr AS (
            SELECT fy, COUNT(*) AS n_narr FROM narratives {narr_filter} GROUP BY fy
        ),
        fy_edges AS (
            SELECT n.fy,
                COUNT(*) FILTER (e.edge_type IN {VOICE}) AS voice_e,
                COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}) AS leadership_e,
                COUNT(*) FILTER (e.edge_type IN {DRIFT}) AS drift_e,
                COUNT(*) FILTER (e.edge_type IN {CARE}) AS care_e,
                COUNT(*) FILTER (e.edge_type IN {GROWTH}) AS growth_e
            FROM narratives n
            JOIN edges e ON e.narrative_id = n.id
            WHERE e.is_cultural = true {join_filter}
            GROUP BY n.fy
        )
        SELECT fn.fy, fn.n_narr AS narratives,
            ROUND(COALESCE(fe.voice_e, 0)::FLOAT / fn.n_narr, 2) AS voice,
            ROUND(COALESCE(fe.leadership_e, 0)::FLOAT / fn.n_narr, 2) AS leadership,
            ROUND(COALESCE(fe.drift_e, 0)::FLOAT / fn.n_narr, 2) AS drift,
            ROUND(COALESCE(fe.care_e, 0)::FLOAT / fn.n_narr, 2) AS care,
            ROUND(COALESCE(fe.growth_e, 0)::FLOAT / fn.n_narr, 2) AS growth
        FROM fy_narr fn
        LEFT JOIN fy_edges fe ON fn.fy = fe.fy
        ORDER BY fn.fy
    """).fetchdf()

    # Report-type adjustment for flags (default when aggregating across types)
    if adjusted:
        baselines = con.execute(f"""
            WITH rt_narr AS (
                SELECT report_type, COUNT(*) AS n_narr FROM narratives {narr_filter} GROUP BY report_type
            ),
            rt_edges AS (
                SELECT n.report_type,
                    COUNT(*) FILTER (e.edge_type IN {VOICE}) AS voice_e,
                    COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}) AS leadership_e,
                    COUNT(*) FILTER (e.edge_type IN {DRIFT}) AS drift_e,
                    COUNT(*) FILTER (e.edge_type IN {CARE}) AS care_e,
                    COUNT(*) FILTER (e.edge_type IN {GROWTH}) AS growth_e
                FROM narratives n
                JOIN edges e ON e.narrative_id = n.id
                WHERE e.is_cultural = true {join_filter}
                GROUP BY n.report_type
            )
            SELECT rn.report_type,
                COALESCE(re.voice_e, 0)::FLOAT / rn.n_narr AS voice_bl,
                COALESCE(re.leadership_e, 0)::FLOAT / rn.n_narr AS leadership_bl,
                COALESCE(re.drift_e, 0)::FLOAT / rn.n_narr AS drift_bl,
                COALESCE(re.care_e, 0)::FLOAT / rn.n_narr AS care_bl,
                COALESCE(re.growth_e, 0)::FLOAT / rn.n_narr AS growth_bl
            FROM rt_narr rn
            LEFT JOIN rt_edges re ON rn.report_type = re.report_type
        """).fetchdf()

        site_rt = con.execute(f"""
            SELECT site, report_type, COUNT(*) AS n
            FROM narratives {narr_filter}
            GROUP BY site, report_type
        """).fetchdf()

        bl_lookup = {}
        for _, bl_row in baselines.iterrows():
            bl_lookup[bl_row["report_type"]] = {
                c: float(bl_row[f"{c}_bl"]) for c in composites
            }

        for c in composites:
            dashboard[c] = dashboard[c].astype(float)

        for idx, row in dashboard.iterrows():
            site_data = site_rt[site_rt["site"] == row["site"]]
            site_total = site_data["n"].sum()
            for c in composites:
                expected = sum(
                    (sr["n"] / site_total) * bl_lookup.get(sr["report_type"], {}).get(c, 0.0)
                    for _, sr in site_data.iterrows()
                )
                dashboard.at[idx, c] = round(float(row[c]) - expected, 2)

    # Compute current flags
    current_flags = {}
    for _, row in dashboard.iterrows():
        flags = []
        if adjusted:
            for name in composites:
                threshold = avgs[name] * 0.3
                if threshold > 0:
                    if row[name] < -threshold:
                        flags.append(f"{name}:LOW")
                    elif row[name] > threshold:
                        flags.append(f"{name}:HIGH")
        else:
            for name in composites:
                avg = avgs[name]
                if avg > 0:
                    if row[name] < avg * FLAG_LOW:
                        flags.append(f"{name}:LOW")
                    elif row[name] > avg * FLAG_HIGH:
                        flags.append(f"{name}:HIGH")
        if flags:
            current_flags[row["site"]] = flags

    # Detect flag changes against previous state
    flag_changes = []
    prev_flags = {}
    if state_path.exists():
        prev_flags = json.loads(state_path.read_text())
        all_sites = set(list(current_flags.keys()) + list(prev_flags.keys()))
        for site in sorted(all_sites):
            prev = set(prev_flags.get(site, []))
            curr = set(current_flags.get(site, []))
            added = curr - prev
            removed = prev - curr
            if added:
                flag_changes.append(f"{site} gained {', '.join(sorted(added))}")
            if removed:
                flag_changes.append(f"{site} cleared {', '.join(sorted(removed))}")

    # Save current flags as state for next run
    state_path.write_text(json.dumps(current_flags, indent=2, sort_keys=True))

    # Build entry
    entry = []
    entry.append(f"## {now.strftime('%Y-%m-%d')}{heading_suffix}")
    entry.append("")
    entry.append(
        f"**Latest load**: {batch_detail[0]} | "
        f"**Batch**: {batch_detail[1]:,} narratives, {batch_detail[2]} sites"
    )
    entry.append("")
    entry.append("| | Batch | Cumulative |")
    entry.append("|---|---|---|")
    entry.append(f"| Narratives | {batch_detail[1]:,} | {cum[0]:,} |")
    entry.append(f"| Cultural edges | {batch_detail[3]:,} | {cum[2]:,} |")
    entry.append(f"| Sites | {batch_detail[2]} | {cum[1]} |")
    entry.append(f"| Years | | {cum[3]} |")
    entry.append("")
    entry.append(
        f"**Org averages** (edges per narrative): "
        f"Voice {avg_voice:.2f} | Leadership {avg_leadership:.2f} | "
        f"Drift {avg_drift:.2f} | Care {avg_care:.2f} | Growth {avg_growth:.2f}"
    )
    entry.append("")
    entry.append("### Temporal Trajectory")
    entry.append("")
    entry.append("| FY | N | Voice | Leadership | Drift | Care | Growth |")
    entry.append("|---|---|-------|------------|-------|------|--------|")
    for _, row in temporal.iterrows():
        entry.append(
            f"| {row['fy']:.0f} | {row['narratives']:,.0f} | "
            f"{row['voice']:.2f} | {row['leadership']:.2f} | "
            f"{row['drift']:.2f} | {row['care']:.2f} | {row['growth']:.2f} |"
        )
    entry.append("")

    if flag_changes:
        entry.append("### Flag Changes")
        entry.append("")
        for change in flag_changes:
            entry.append(f"- {change}")
        entry.append("")
    elif prev_flags:
        entry.append("**Flag changes**: None")
        entry.append("")

    if current_flags:
        active = []
        for site, flags in sorted(current_flags.items()):
            active.append(f"**{site}** ({', '.join(flags)})")
        entry.append(f"**Active flags**: {' | '.join(active)}")
    else:
        entry.append("**Active flags**: None")
    entry.append("")
    entry.append("---")
    entry.append("")

    return "\n".join(entry)


def write_monthly_tracker(entry, tracker_path=None):
    """Append tracker entry to monthly-tracker.md, creating with header if new."""
    tracker_path = tracker_path or TRACKER_PATH
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    if tracker_path.exists():
        content = tracker_path.read_text()
        content += entry
        tracker_path.write_text(content)
    else:
        header = (
            "# Cultural Graph Monthly Tracker\n\n"
            "Append-only log of monthly cultural graph processing. "
            "Each entry is generated after `/cultural-graph-load` updates DuckDB.\n\n"
            "Rates = cultural edges per narrative. Five composites cover all 12 edge types.\n\n"
            "---\n\n"
        )
        tracker_path.write_text(header + entry)
    print(f"Tracker updated: {tracker_path}")


def main():
    parser = argparse.ArgumentParser(description="Cultural graph reporting")
    parser.add_argument("--template", action="store_true", help="Generate standard markdown report")
    parser.add_argument("--csv", action="store_true", help="Generate PowerBI CSV export")
    parser.add_argument("--monthly-tracker", action="store_true", help="Append monthly scorecard to tracker")
    parser.add_argument("--fy", type=int, help="Filter to specific financial year")
    parser.add_argument("--report-type", help="Filter to specific report type (e.g. 'Hazard & Observations')")
    parser.add_argument("--raw", action="store_true",
                        help="Unadjusted blended rates (default adjusts for report type mix)")
    parser.add_argument("--output", help="Output file path (default: auto-named)")
    args = parser.parse_args()

    if not args.template and not args.csv and not args.monthly_tracker:
        parser.error("Specify --template, --csv, or --monthly-tracker")

    con = get_connection()

    if args.template:
        report = generate_template(con, fy=args.fy, report_type=args.report_type,
                                    raw=args.raw)
        OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
        if args.output:
            out_path = Path(args.output)
        else:
            parts = []
            if args.fy:
                parts.append(f"fy{args.fy}")
            if args.report_type:
                parts.append(args.report_type.lower().replace(" ", "-").replace("&", "and"))
            if args.raw and not args.report_type:
                parts.append("raw")
            suffix = "-" + "-".join(parts) if parts else "-all"
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

    if args.monthly_tracker:
        entry = generate_monthly_tracker(con, report_type=args.report_type, raw=args.raw)
        if args.report_type:
            rt_slug = args.report_type.lower().replace(" ", "-").replace("&", "and")
            tracker_path = OUTPUT_DIR / f"monthly-tracker-{rt_slug}.md"
        else:
            tracker_path = None
        write_monthly_tracker(entry, tracker_path=tracker_path)

    con.close()


if __name__ == "__main__":
    main()
