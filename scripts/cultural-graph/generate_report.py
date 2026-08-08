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
from scipy.stats import chi2, linregress, norm

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


def smr_poisson_ci(observed, expected, phi=1.0, alpha=0.05):
    """Quasi-Poisson confidence interval for SMR = observed/expected.

    When phi > 1 (overdispersion), the Poisson CI is inflated by sqrt(phi).
    This is the standard quasi-Poisson correction used in epidemiology.
    """
    if expected <= 0:
        return (float('nan'), float('nan'), float('nan'))
    smr = observed / expected
    if observed == 0:
        p_lo = 0.0
        p_hi = chi2.ppf(1 - alpha / 2, 2) / (2 * expected)
    else:
        p_lo = chi2.ppf(alpha / 2, 2 * observed) / (2 * expected)
        p_hi = chi2.ppf(1 - alpha / 2, 2 * (observed + 1)) / (2 * expected)
    # Inflate CI by sqrt(phi) for quasi-Poisson correction
    if phi > 1.0:
        hw = (p_hi - p_lo) / 2 * (phi ** 0.5)
        lo = max(0, smr - hw)
        hi = smr + hw
    else:
        lo, hi = p_lo, p_hi
    return (smr, lo, hi)


def compute_overdispersion(con, composites_sql, narr_filter="", fy_filter=""):
    """Compute Pearson overdispersion factor (phi) per composite.

    phi = Pearson chi2 / df from a Poisson model with report_type as sole covariate.
    phi > 1 indicates overdispersion; Poisson CIs should be inflated by sqrt(phi).
    """
    import numpy as np

    comp_keys = ["voice", "leadership", "drift", "care", "growth"]
    comp_tuples = [VOICE, LEADERSHIP, DRIFT, CARE, GROWTH]
    phis = {}

    for key, types in zip(comp_keys, comp_tuples):
        cells = con.execute(f"""
            SELECT n.site, n.report_type, COUNT(*) AS n_narr,
                COUNT(*) FILTER (e.edge_type IN {types}) AS observed
            FROM narratives n
            LEFT JOIN edges e ON e.narrative_id = n.id AND e.is_cultural
            {"WHERE " + narr_filter.lstrip("WHERE ").lstrip("AND ") if narr_filter else ""}
            GROUP BY n.site, n.report_type
            HAVING COUNT(*) >= 3
        """).fetchdf()

        rt_rates = {}
        for rt, grp in cells.groupby("report_type"):
            total_obs = grp["observed"].sum()
            total_n = grp["n_narr"].sum()
            rt_rates[rt] = total_obs / total_n if total_n > 0 else 0

        chi2_val = 0.0
        for _, row in cells.iterrows():
            fitted = row["n_narr"] * rt_rates.get(row["report_type"], 0)
            if fitted > 0:
                chi2_val += (row["observed"] - fitted) ** 2 / fitted

        df = len(cells) - len(rt_rates)
        phis[key] = chi2_val / df if df > 0 else 1.0

    return phis


def apply_fdr(site_cis, composites, alpha=0.05):
    """Apply Benjamini-Hochberg FDR correction to SMR significance tests.

    Converts quasi-Poisson CIs to z-scores and p-values, then applies BH-FDR.
    Returns {site: [flag_strings]} for sites that remain significant.
    """
    import numpy as np

    tests = []
    for site, cis in site_cis.items():
        for c in composites:
            ci = cis.get(c)
            if ci:
                smr, lo, hi = ci[0], ci[1], ci[2]
                ci_width = hi - lo
                se = ci_width / (2 * 1.96) if ci_width > 0 else 0
                if se > 0 and not np.isnan(smr):
                    z = abs(smr - 1.0) / se
                    p = 2 * (1 - norm.cdf(z))
                else:
                    p = 1.0
                tests.append((site, c, smr, p))

    if not tests:
        return {}

    # Benjamini-Hochberg procedure
    pvals = np.array([t[3] for t in tests])
    n = len(pvals)
    sorted_idx = np.argsort(pvals)
    thresholds = (np.arange(1, n + 1) / n) * alpha
    rejected = np.zeros(n, dtype=bool)
    below = pvals[sorted_idx] <= thresholds
    if below.any():
        max_k = int(np.max(np.where(below)))
        rejected[sorted_idx[: max_k + 1]] = True

    fdr_flags = {}
    for i, (site, c, smr, p) in enumerate(tests):
        if rejected[i]:
            direction = "HIGH" if smr > 1.0 else "LOW"
            fdr_flags.setdefault(site, []).append(f"{c}:{direction}")

    return fdr_flags


def compute_temporal_trends(con, narr_filter="", min_years=3, min_n_per_year=5):
    """Compute per-site year-on-year trends via OLS, with BH-FDR correction.

    Returns list of significant trends: [(site, composite, slope, p_adj, n_years, direction)]
    sorted by absolute slope descending.
    """
    import numpy as np

    site_fy = con.execute(f"""
        WITH sfy AS (
            SELECT site, fy, COUNT(*) AS n_narr
            FROM narratives {narr_filter}
            GROUP BY site, fy
            HAVING COUNT(*) >= {min_n_per_year}
        ),
        sfy_edges AS (
            SELECT n.site, n.fy,
                COUNT(*) FILTER (e.edge_type IN {VOICE}) AS voice_e,
                COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}) AS leadership_e,
                COUNT(*) FILTER (e.edge_type IN {DRIFT}) AS drift_e,
                COUNT(*) FILTER (e.edge_type IN {CARE}) AS care_e,
                COUNT(*) FILTER (e.edge_type IN {GROWTH}) AS growth_e
            FROM narratives n
            JOIN edges e ON e.narrative_id = n.id
            WHERE e.is_cultural = true
            GROUP BY n.site, n.fy
        )
        SELECT s.site, s.fy, s.n_narr,
            COALESCE(se.voice_e, 0)::FLOAT / s.n_narr AS voice,
            COALESCE(se.leadership_e, 0)::FLOAT / s.n_narr AS leadership,
            COALESCE(se.drift_e, 0)::FLOAT / s.n_narr AS drift,
            COALESCE(se.care_e, 0)::FLOAT / s.n_narr AS care,
            COALESCE(se.growth_e, 0)::FLOAT / s.n_narr AS growth
        FROM sfy s
        LEFT JOIN sfy_edges se ON s.site = se.site AND s.fy = se.fy
        ORDER BY s.site, s.fy
    """).fetchdf()

    composites = ["voice", "leadership", "drift", "care", "growth"]
    all_trends = []

    for site, grp in site_fy.groupby("site"):
        if len(grp) < min_years:
            continue
        fys = grp["fy"].values.astype(float)
        for c in composites:
            rates = grp[c].values.astype(float)
            result = linregress(fys, rates)
            all_trends.append((site, c, result.slope, result.pvalue, len(grp)))

    if not all_trends:
        return []

    # BH-FDR correction
    pvals = np.array([t[3] for t in all_trends])
    n = len(pvals)
    sorted_idx = np.argsort(pvals)
    thresholds = (np.arange(1, n + 1) / n) * 0.05
    rejected = np.zeros(n, dtype=bool)
    below = pvals[sorted_idx] <= thresholds
    if below.any():
        max_k = int(np.max(np.where(below)))
        rejected[sorted_idx[: max_k + 1]] = True

    sig = []
    for i, (site, c, slope, p, n_years) in enumerate(all_trends):
        if rejected[i]:
            direction = "rising" if slope > 0 else "falling"
            sig.append((site, c, slope, p, n_years, direction))

    sig.sort(key=lambda x: abs(x[2]), reverse=True)
    return sig


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

    # Report-type adjustment: compute SMR (observed/expected) per site, with Poisson CIs.
    # SMR = 1.0 means site is exactly as expected given its report type mix.
    # site_cis stores {site: {composite: (smr, ci_lo, ci_hi)}} for flagging.
    site_cis = {}
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

        # Compute overdispersion factors (quasi-Poisson correction)
        phis = compute_overdispersion(con, composites, narr_filter, fy_filter)

        for c in composites:
            dashboard[c] = dashboard[c].astype(float)

        for idx, row in dashboard.iterrows():
            site = row["site"]
            n = float(row["n"])
            site_data = site_rt[site_rt["site"] == site]
            site_total = site_data["n"].sum()
            cis = {}
            for c in composites:
                observed = round(float(row[c]) * n)
                expected_rate = sum(
                    (sr["n"] / site_total) * bl_lookup.get(sr["report_type"], {}).get(c, 0.0)
                    for _, sr in site_data.iterrows()
                )
                expected = expected_rate * n
                smr, lo, hi = smr_poisson_ci(observed, expected, phi=phis.get(c, 1.0))
                dashboard.at[idx, c] = round(smr, 2)
                cis[c] = (smr, lo, hi, observed, expected)
            site_cis[site] = cis

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
        # BH-FDR correction across all site×composite tests
        fdr_flags = apply_fdr(site_cis, composites)

        def flag(row):
            return fdr_flags.get(row["site"], [])

        sig_high_drift = {s for s, flags in fdr_flags.items() if "drift:HIGH" in flags}
        sig_low_voice = {s for s, flags in fdr_flags.items() if "voice:LOW" in flags}
        high_drift = dashboard[dashboard["site"].isin(sig_high_drift)].sort_values("drift", ascending=False)
        low_voice = dashboard[dashboard["site"].isin(sig_low_voice)].sort_values("voice")
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

        high_drift = dashboard[dashboard["drift"] > avg_drift * FLAG_HIGH].sort_values("drift", ascending=False)
        low_voice = dashboard[dashboard["voice"] < avg_voice * FLAG_LOW].sort_values("voice")

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
        report.append("\nStandardised ratios per site (observed / expected given report type mix).")
        report.append("1.00 = as expected. Flagged at FDR<0.05 (Benjamini-Hochberg, quasi-Poisson):")
        report.append(f"- **Voice** (baseline {avg_voice:.2f}/narr) — communication signal vs expected")
        report.append(f"- **Leadership** (baseline {avg_leadership:.2f}/narr) — directing/overseeing vs expected")
        report.append(f"- **Drift** (baseline {avg_drift:.2f}/narr) — procedural bypass vs expected")
        report.append(f"- **Care** (baseline {avg_care:.2f}/narr) — failure response vs expected")
        report.append(f"- **Growth** (baseline {avg_growth:.2f}/narr) — learning signal vs expected")
    else:
        report.append("\nFive indicators per site — cultural edges per narrative, compared to org average:")
        report.append(f"- **Voice** ({avg_voice:.2f}) — are people communicating?")
        report.append(f"- **Leadership** ({avg_leadership:.2f}) — are people directing and overseeing?")
        report.append(f"- **Drift** ({avg_drift:.2f}) — are procedures being bypassed?")
        report.append(f"- **Care** ({avg_care:.2f}) — does the site respond when things go wrong?")
        report.append(f"- **Growth** ({avg_growth:.2f}) — is the site building on what it learns?")
    report.append("\n| Site | N | Voice | Leadership | Drift | Care | Growth |")
    report.append("|------|---|-------|------------|-------|------|--------|")
    for _, row in dashboard.iterrows():
        site_flags = flag(row)
        flag_str = f" **{', '.join(site_flags)}**" if site_flags else ""
        report.append(
            f"| {row['site']} | {row['n']:.0f} | {row['voice']:.2f} | "
            f"{row['leadership']:.2f} | {row['drift']:.2f} | "
            f"{row['care']:.2f} | {row['growth']:.2f} |{flag_str}"
        )

    report.append("\n## Sites to Watch")
    watch_count = 0
    if len(high_drift) > 0:
        for _, row in high_drift.head(2).iterrows():
            watch_count += 1
            if adjusted:
                ci = site_cis.get(row["site"], {}).get("drift", (0, 0, 0))
                report.append(
                    f"\n{watch_count}. **{row['site']}** — Drift SMR {row['drift']:.2f} "
                    f"(95% CI: {ci[1]:.2f}–{ci[2]:.2f}). "
                    f"Statistically significant excess after controlling for report type mix."
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
                ci = site_cis.get(row["site"], {}).get("voice", (0, 0, 0))
                report.append(
                    f"\n{watch_count}. **{row['site']}** — Voice SMR {row['voice']:.2f} "
                    f"(95% CI: {ci[1]:.2f}–{ci[2]:.2f}). "
                    f"Statistically significant deficit after controlling for report type mix."
                )
            else:
                report.append(
                    f"\n{watch_count}. **{row['site']}** — Voice at {row['voice']:.2f} "
                    f"(org avg {avg_voice:.2f}). Low communication signal."
                )

    # Temporal trends — per-site OLS slopes (only when not filtering to single FY)
    if not fy:
        trends = compute_temporal_trends(con, narr_filter)
        if trends:
            report.append("\n## Temporal Trends")
            report.append("\nSites with statistically significant year-on-year trends "
                          "(OLS slope, FDR<0.05):")
            report.append("\n| Site | Indicator | Slope/yr | Years | Direction |")
            report.append("|------|-----------|----------|-------|-----------|")
            for site, c, slope, p, n_years, direction in trends:
                arrow = "\u2191" if direction == "rising" else "\u2193"
                report.append(
                    f"| {site} | {c.capitalize()} | {slope:+.3f} | {n_years} | "
                    f"{arrow} {direction} |"
                )
        else:
            report.append("\n## Temporal Trends")
            report.append("\nNo statistically significant site-level trends detected (FDR<0.05).")

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
        report.append(f"\n*Report generated from {DUCKDB_PATH}. Values = standardised ratios "
                      f"(observed / expected given report type mix). Flagged at FDR<0.05 "
                      f"(Benjamini-Hochberg, quasi-Poisson CIs). "
                      f"Cultural relationships extracted by Qwen 3 8B fine-tuned model.*")
    else:
        report.append(f"\n*Report generated from {DUCKDB_PATH}. Rates = cultural edges per narrative. "
                      f"Cultural relationships extracted by Qwen 3 8B fine-tuned model.*")

    return "\n".join(report), site_cis


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


def generate_funnel(site_cis, output_dir, phis=None):
    """Generate funnel plots — SMR vs expected count with 95%/99.8% control limits.

    Standard NHS-style funnel: sites within the funnel are consistent with chance
    variation; sites outside have statistically significant deviation from expected.
    When phis is provided, control limits are widened by sqrt(phi) per composite.
    """
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np

    composites = ["voice", "leadership", "drift", "care", "growth"]
    titles = ["Voice", "Leadership", "Drift", "Care", "Growth"]
    if phis is None:
        phis = {c: 1.0 for c in composites}

    fig, axes = plt.subplots(2, 3, figsize=(15, 10))
    axes = axes.flatten()

    for i, (c, title) in enumerate(zip(composites, titles)):
        ax = axes[i]
        phi = phis.get(c, 1.0)
        phi_sqrt = phi ** 0.5
        es, smrs, labels = [], [], []
        for site, cis in site_cis.items():
            ci = cis.get(c)
            if ci and ci[4] > 0 and not np.isnan(ci[0]):
                es.append(ci[4])       # expected count
                smrs.append(ci[0])     # SMR
                labels.append(site)

        es = np.array(es)
        smrs = np.array(smrs)

        # Funnel curves (quasi-Poisson: SMR ≈ 1 ± z·√φ/√E)
        e_range = np.linspace(max(1, es.min() * 0.8), es.max() * 1.1, 300)
        upper_95 = 1 + 1.96 * phi_sqrt / np.sqrt(e_range)
        lower_95 = np.maximum(0, 1 - 1.96 * phi_sqrt / np.sqrt(e_range))
        upper_998 = 1 + 3.09 * phi_sqrt / np.sqrt(e_range)
        lower_998 = np.maximum(0, 1 - 3.09 * phi_sqrt / np.sqrt(e_range))

        ax.fill_between(e_range, lower_998, upper_998, alpha=0.08, color="steelblue")
        ax.fill_between(e_range, lower_95, upper_95, alpha=0.15, color="steelblue")
        ax.plot(e_range, upper_95, color="steelblue", linewidth=0.8, linestyle="--")
        ax.plot(e_range, lower_95, color="steelblue", linewidth=0.8, linestyle="--")
        ax.plot(e_range, upper_998, color="steelblue", linewidth=0.5, linestyle=":")
        ax.plot(e_range, lower_998, color="steelblue", linewidth=0.5, linestyle=":")
        ax.axhline(y=1.0, color="grey", linestyle="-", linewidth=0.8)

        # Plot sites — red if outside 95% funnel, blue if inside
        for j in range(len(es)):
            ul = 1 + 1.96 * phi_sqrt / np.sqrt(es[j])
            ll = max(0, 1 - 1.96 * phi_sqrt / np.sqrt(es[j]))
            outside = smrs[j] > ul or smrs[j] < ll
            color = "red" if outside else "steelblue"
            alpha = 0.9 if outside else 0.5
            ax.scatter(es[j], smrs[j], c=color, s=18, alpha=alpha, zorder=5, edgecolors="none")
            if outside:
                short = labels[j].split(" ")[-1] if " " in labels[j] else labels[j]
                ax.annotate(short, (es[j], smrs[j]), fontsize=5.5,
                            ha="left", va="bottom", xytext=(3, 3),
                            textcoords="offset points", color="red")

        phi_label = f" (φ={phi:.1f})" if phi > 1.05 else ""
        ax.set_title(f"{title}{phi_label}", fontweight="bold", fontsize=11)
        ax.set_xlabel("Expected count", fontsize=9)
        ax.set_ylabel("SMR", fontsize=9)
        ax.set_ylim(bottom=0)
        ax.tick_params(labelsize=8)

    # Hide 6th subplot
    ax6 = axes[5]
    ax6.set_visible(False)

    fig.suptitle("Cultural Graph — Funnel Plots (SMR vs Expected Count, quasi-Poisson)",
                 fontweight="bold", fontsize=13)
    fig.tight_layout(rect=[0, 0, 1, 0.95])

    output_dir.mkdir(parents=True, exist_ok=True)
    out_path = output_dir / "funnel-plots.png"
    fig.savefig(out_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"Funnel plots: {out_path}")
    return out_path


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
    # Uses SMR (observed/expected) with Poisson CIs — flag when CI excludes 1.0
    site_cis = {}
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

        phis = compute_overdispersion(con, composites, narr_filter, join_filter)

        for c in composites:
            dashboard[c] = dashboard[c].astype(float)

        for idx, row in dashboard.iterrows():
            site = row["site"]
            n = float(row["n"])
            site_data = site_rt[site_rt["site"] == site]
            site_total = site_data["n"].sum()
            cis = {}
            for c in composites:
                observed = round(float(row[c]) * n)
                expected_rate = sum(
                    (sr["n"] / site_total) * bl_lookup.get(sr["report_type"], {}).get(c, 0.0)
                    for _, sr in site_data.iterrows()
                )
                expected = expected_rate * n
                smr, lo, hi = smr_poisson_ci(observed, expected, phi=phis.get(c, 1.0))
                dashboard.at[idx, c] = round(smr, 2)
                cis[c] = (smr, lo, hi, observed, expected)
            site_cis[site] = cis

    # Compute current flags
    current_flags = {}
    if adjusted:
        fdr_flags = apply_fdr(site_cis, composites)
        for site, flags in fdr_flags.items():
            current_flags[site] = flags
    else:
        for _, row in dashboard.iterrows():
            flags = []
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
    parser.add_argument("--funnel", action="store_true",
                        help="Generate funnel plots (PNG) from adjusted SMR data")
    parser.add_argument("--output", help="Output file path (default: auto-named)")
    args = parser.parse_args()

    if not args.template and not args.csv and not args.monthly_tracker and not args.funnel:
        parser.error("Specify --template, --csv, --monthly-tracker, or --funnel")

    if args.funnel and args.raw:
        parser.error("--funnel requires adjusted SMR data (incompatible with --raw)")

    con = get_connection()

    # Generate template first if needed (funnel needs its site_cis output)
    site_cis = {}
    if args.template or args.funnel:
        report, site_cis = generate_template(con, fy=args.fy, report_type=args.report_type,
                                              raw=args.raw)
    if args.template:
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

    if args.funnel:
        if not site_cis:
            parser.error("--funnel requires adjusted SMR data (no site_cis computed — check flags)")
        phis = compute_overdispersion(con, ["voice", "leadership", "drift", "care", "growth"])
        generate_funnel(site_cis, OUTPUT_DIR, phis=phis)

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
