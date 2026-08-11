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
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --csv --output data/qq/cultural-graph/reports/dashboard.csv
    /usr/bin/python3 scripts/cultural-graph/generate_report.py --monthly-tracker
"""

import argparse
import json
import sys
from datetime import datetime
from pathlib import Path

import duckdb
from scipy.stats import chi2, norm

DUCKDB_PATH = Path("data/cultural-graph.duckdb")
OUTPUT_DIR = Path("data/qq/cultural-graph/reports")
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


def compute_standardised_averages(con, narr_filter="", fy_filter="", min_rt_n=50):
    """Compute report-type-standardised org averages as prevalence + intensity.

    Prevalence = proportion of narratives with any cultural edges (per RT, then
    equal-weighted across RTs).
    Intensity = edges per signal-bearing narrative (per RT, then equal-weighted).

    Returns (prevalence, intensity_dict, rt_baselines_df) where:
      - prevalence: float (standardised proportion, e.g. 0.57)
      - intensity_dict: {composite: standardised rate among signal-bearing narratives}
      - rt_baselines_df: per-report-type detail (n_narr, n_signal, prevalence,
        and intensity per composite)
    Only report types with >= min_rt_n narratives contribute to the averages.
    """
    composites = ["voice", "leadership", "drift", "care", "growth"]
    rt_df = con.execute(f"""
        WITH rt_narr AS (
            SELECT report_type,
                COUNT(*) AS n_narr,
                COUNT(*) FILTER (cultural_edge_count > 0) AS n_signal
            FROM narratives {narr_filter} GROUP BY report_type
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
            WHERE e.is_cultural = true AND n.cultural_edge_count > 0 {fy_filter}
            GROUP BY n.report_type
        )
        SELECT rn.report_type, rn.n_narr, rn.n_signal,
            ROUND(rn.n_signal::FLOAT / rn.n_narr, 3) AS prevalence,
            CASE WHEN rn.n_signal > 0
                THEN COALESCE(re.voice_e, 0)::FLOAT / rn.n_signal ELSE 0 END AS voice,
            CASE WHEN rn.n_signal > 0
                THEN COALESCE(re.leadership_e, 0)::FLOAT / rn.n_signal ELSE 0 END AS leadership,
            CASE WHEN rn.n_signal > 0
                THEN COALESCE(re.drift_e, 0)::FLOAT / rn.n_signal ELSE 0 END AS drift,
            CASE WHEN rn.n_signal > 0
                THEN COALESCE(re.care_e, 0)::FLOAT / rn.n_signal ELSE 0 END AS care,
            CASE WHEN rn.n_signal > 0
                THEN COALESCE(re.growth_e, 0)::FLOAT / rn.n_signal ELSE 0 END AS growth
        FROM rt_narr rn
        LEFT JOIN rt_edges re ON rn.report_type = re.report_type
        ORDER BY rn.n_narr DESC
    """).fetchdf()

    eligible = rt_df[rt_df["n_narr"] >= min_rt_n]
    prevalence = round(float(eligible["prevalence"].mean()), 2) if len(eligible) > 0 else 0.0
    intensity = {}
    for c in composites:
        intensity[c] = round(float(eligible[c].mean()), 2) if len(eligible) > 0 else 0.0

    # Bootstrap 95% CIs across report-type estimates
    import numpy as np
    cis = {}
    n_boot = 2000
    rng = np.random.default_rng(42)
    if len(eligible) >= 3:
        prev_vals = eligible["prevalence"].values.astype(float)
        boot_prev = [np.mean(rng.choice(prev_vals, len(prev_vals))) for _ in range(n_boot)]
        cis["prevalence"] = (round(float(np.percentile(boot_prev, 2.5)), 2),
                             round(float(np.percentile(boot_prev, 97.5)), 2))
        for c in composites:
            c_vals = eligible[c].values.astype(float)
            boot_c = [np.mean(rng.choice(c_vals, len(c_vals))) for _ in range(n_boot)]
            cis[c] = (round(float(np.percentile(boot_c, 2.5)), 2),
                      round(float(np.percentile(boot_c, 97.5)), 2))

    return prevalence, intensity, rt_df, cis


def smr_poisson_ci(observed, expected, phi=1.0, alpha=0.05):
    """Log-scale quasi-Poisson confidence interval for SMR = observed/expected.

    Works on the log scale where the distribution is symmetric:
      SE(log(SMR)) = sqrt(phi / O)
      CI = exp(log(SMR) ± z * SE)

    This naturally produces asymmetric CIs on the SMR scale without the
    inconsistency of inflating exact Poisson CIs symmetrically.

    For O=0, falls back to exact Poisson upper bound (log undefined).
    """
    import math

    if expected <= 0:
        return (float('nan'), float('nan'), float('nan'))
    smr = observed / expected
    z = norm.ppf(1 - alpha / 2)  # 1.96 for alpha=0.05

    if observed == 0:
        # O=0: log undefined — use exact Poisson upper bound
        lo = 0.0
        hi = chi2.ppf(1 - alpha / 2, 2) / (2 * expected)
        return (smr, lo, hi)

    log_smr = math.log(smr)
    se_log = math.sqrt(phi / observed)
    lo = math.exp(log_smr - z * se_log)
    hi = math.exp(log_smr + z * se_log)
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


def empirical_bayes_shrinkage(site_cis, composites):
    """Gamma-Poisson empirical Bayes shrinkage for SMRs.

    Models observed counts as Poisson(E·θ) where θ ~ Gamma(α, β).
    Estimates α, β per composite via method of moments on (O, E) pairs.
    Posterior mean: θ_shrunk = (O + α) / (E + β).

    Returns {site: {composite: (smr_shrunk, smr_raw, O, E, shrinkage_pct)}}.
    """
    import numpy as np

    eb_results = {}
    eb_params = {}

    for c in composites:
        # Collect (O, E) pairs for all sites with E > 0
        pairs = []
        for site, cis in site_cis.items():
            ci = cis.get(c)
            if ci and ci[4] > 0:  # expected > 0
                pairs.append((ci[3], ci[4], site))  # (observed, expected, site)

        if len(pairs) < 3:
            eb_params[c] = (1.0, 1.0)
            continue

        Os = np.array([p[0] for p in pairs], dtype=float)
        Es = np.array([p[1] for p in pairs], dtype=float)
        smrs = Os / Es

        # Method of moments for gamma prior on θ (the true SMR):
        # E[θ] = α/β, Var[θ] = α/β²
        # But observed SMR variance includes sampling variance:
        # Var(O/E) = Var(θ) + E[1/E_i] (Poisson sampling var)
        # So: Var(θ) = Var(O/E) - mean(1/E_i)
        mean_smr = np.mean(smrs)
        var_smr = np.var(smrs, ddof=1)
        sampling_var = np.mean(1.0 / Es)  # average Poisson sampling variance

        var_theta = max(var_smr - sampling_var, 0.01 * var_smr)

        # Gamma params from moments: α = μ²/σ², β = μ/σ²
        alpha_prior = mean_smr ** 2 / var_theta
        beta_prior = mean_smr / var_theta
        eb_params[c] = (alpha_prior, beta_prior)

        # Compute shrunken estimates
        for O, E, site in pairs:
            smr_raw = O / E
            smr_shrunk = (O + alpha_prior) / (E + beta_prior)
            shrinkage_pct = abs(smr_shrunk - smr_raw) / max(abs(smr_raw - 1.0), 0.001) * 100
            if site not in eb_results:
                eb_results[site] = {}
            eb_results[site][c] = (smr_shrunk, smr_raw, O, E, shrinkage_pct)

    return eb_results, eb_params


def apply_fdr(site_cis, composites, phis=None, alpha=0.05):
    """Apply Benjamini-Hochberg FDR correction to SMR significance tests.

    Derives p-values on the log scale (consistent with quasi-Poisson):
      z = |log(SMR)| / SE(log(SMR)),  SE = sqrt(phi/O)
    Then applies BH-FDR.
    Returns {site: [flag_strings]} for sites that remain significant.
    """
    import math
    import numpy as np

    if phis is None:
        phis = {}

    tests = []
    for site, cis in site_cis.items():
        for c in composites:
            ci = cis.get(c)
            if ci:
                smr, lo, hi, observed, expected = ci[0], ci[1], ci[2], ci[3], ci[4]
                if observed > 0 and expected > 0 and not np.isnan(smr):
                    phi = phis.get(c, 1.0)
                    log_smr = math.log(smr)
                    se_log = math.sqrt(phi / observed)
                    z = abs(log_smr) / se_log
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
    """Compute per-site year-on-year trends via quasi-Poisson GLM, with BH-FDR.

    Models count ~ FY + offset(log(n_narratives)) with Poisson family, then
    applies quasi-Poisson correction (scale p-values by Pearson phi).
    Returns list of significant trends: [(site, composite, slope, p_adj, n_years, direction)]
    sorted by absolute slope descending.
    """
    import math
    import warnings
    import numpy as np
    import statsmodels.api as sm

    site_fy = con.execute(f"""
        WITH sfy AS (
            SELECT site, fy, COUNT(*) AS n_narr
            FROM narratives {narr_filter}
            GROUP BY site, fy
            HAVING COUNT(*) >= {min_n_per_year}
        ),
        sfy_edges AS (
            SELECT n.site, n.fy,
                COALESCE(COUNT(*) FILTER (e.edge_type IN {VOICE}), 0) AS voice_ct,
                COALESCE(COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}), 0) AS leadership_ct,
                COALESCE(COUNT(*) FILTER (e.edge_type IN {DRIFT}), 0) AS drift_ct,
                COALESCE(COUNT(*) FILTER (e.edge_type IN {CARE}), 0) AS care_ct,
                COALESCE(COUNT(*) FILTER (e.edge_type IN {GROWTH}), 0) AS growth_ct
            FROM narratives n
            JOIN edges e ON e.narrative_id = n.id
            WHERE e.is_cultural = true
            GROUP BY n.site, n.fy
        )
        SELECT s.site, s.fy, s.n_narr,
            COALESCE(se.voice_ct, 0) AS voice,
            COALESCE(se.leadership_ct, 0) AS leadership,
            COALESCE(se.drift_ct, 0) AS drift,
            COALESCE(se.care_ct, 0) AS care,
            COALESCE(se.growth_ct, 0) AS growth
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
        # Centre FY for numerical stability
        fy_centred = fys - fys.mean()
        log_exposure = np.log(grp["n_narr"].values.astype(float))
        X = sm.add_constant(fy_centred)
        for c in composites:
            counts = grp[c].values.astype(float)
            try:
                with warnings.catch_warnings():
                    warnings.simplefilter("ignore")
                    model = sm.GLM(counts, X, family=sm.families.Poisson(),
                                   offset=log_exposure)
                    result = model.fit()
                # Quasi-Poisson: scale deviance by Pearson chi2/df
                phi = result.pearson_chi2 / result.df_resid if result.df_resid > 0 else 1.0
                phi = max(phi, 1.0)  # don't under-disperse
                # FY coefficient is index 1; quasi-Poisson t-test
                beta = result.params[1]
                se = result.bse[1] * math.sqrt(phi)
                z = abs(beta) / se if se > 0 else 0
                p = 2 * (1 - norm.cdf(z))
                all_trends.append((site, c, beta, p, len(grp)))
            except Exception:
                # Convergence failure (rare) — skip this site×composite
                continue

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

    # Org averages — standardised prevalence + intensity (equal-weighted across report types, N≥50)
    composites = ["voice", "leadership", "drift", "care", "growth"]
    prevalence, intensity, rt_baselines, org_cis = compute_standardised_averages(con, narr_filter, fy_filter)
    avgs = intensity  # intensity is the per-composite dict used downstream
    avg_voice = intensity["voice"]
    avg_leadership = intensity["leadership"]
    avg_drift = intensity["drift"]
    avg_care = intensity["care"]
    avg_growth = intensity["growth"]

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

        # Empirical Bayes shrinkage — pull noisy small-site SMRs toward the mean
        import math
        eb_results, eb_params = empirical_bayes_shrinkage(site_cis, composites)
        for idx, row in dashboard.iterrows():
            site = row["site"]
            if site in eb_results:
                for c in composites:
                    if c in eb_results[site]:
                        smr_shrunk = eb_results[site][c][0]
                        old = site_cis[site][c]
                        dashboard.at[idx, c] = round(smr_shrunk, 2)
                        # Recompute CI around shrunken estimate on log scale
                        O, E = old[3], old[4]
                        phi = phis.get(c, 1.0)
                        alpha_p, beta_p = eb_params[c]
                        if O > 0:
                            se_log = math.sqrt(phi / (O + alpha_p))
                            z_val = norm.ppf(0.975)
                            lo = smr_shrunk * math.exp(-z_val * se_log)
                            hi = smr_shrunk * math.exp(z_val * se_log)
                        else:
                            lo, hi = old[1], old[2]
                        site_cis[site][c] = (smr_shrunk, lo, hi, O, E)

    # Temporal trajectory — prevalence + intensity by FY (filtered by report_type but not fy)
    temporal = con.execute(f"""
        WITH fy_narr AS (
            SELECT fy,
                COUNT(*) AS n_narr,
                COUNT(*) FILTER (cultural_edge_count > 0) AS n_signal
            FROM narratives {temporal_narr_filter} GROUP BY fy
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
            WHERE e.is_cultural = true AND n.cultural_edge_count > 0 {temporal_join_filter}
            GROUP BY n.fy
        )
        SELECT fn.fy, fn.n_narr AS narratives,
            ROUND(fn.n_signal::FLOAT / fn.n_narr * 100, 0) AS signal_pct,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.voice_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS voice,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.leadership_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS leadership,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.drift_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS drift,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.care_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS care,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.growth_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS growth
        FROM fy_narr fn
        LEFT JOIN fy_edges fe ON fn.fy = fe.fy
        ORDER BY fn.fy
    """).fetchdf()

    # Flag outlier sites
    if adjusted:
        # BH-FDR correction across all site×composite tests
        fdr_flags = apply_fdr(site_cis, composites, phis=phis)

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

    report.append("\n## Org Averages")
    report.append("\nStandardised across report types (equal-weighted, removing mix confound)."
                  " 95% CIs from bootstrap across report types.")
    if "prevalence" in org_cis:
        prev_ci = org_cis["prevalence"]
        report.append(f"\n**Prevalence**: {prevalence:.0%} of narratives contain cultural signal "
                      f"(95% CI: {prev_ci[0]:.0%}–{prev_ci[1]:.0%})")
    else:
        report.append(f"\n**Prevalence**: {prevalence:.0%} of narratives contain cultural signal")
    report.append(f"\n**Intensity** (edges per signal-bearing narrative):")
    if org_cis:
        ci_parts = []
        for c, label in zip(composites, COMPOSITE_NAMES):
            val = intensity[c]
            if c in org_cis:
                lo, hi = org_cis[c]
                ci_parts.append(f"**{label} {val:.2f}** ({lo:.2f}–{hi:.2f})")
            else:
                ci_parts.append(f"**{label} {val:.2f}**")
        report.append(f"\n> {' | '.join(ci_parts)}")
    else:
        report.append(f"\n> **Voice {avg_voice:.2f}** | **Leadership {avg_leadership:.2f}** "
                      f"| **Drift {avg_drift:.2f}** | **Care {avg_care:.2f}** "
                      f"| **Growth {avg_growth:.2f}**")
    report.append("\nPer-report-type baselines:")
    report.append("\n| Report Type | N | Signal % | Voice | Leadership | Drift | Care | Growth |")
    report.append("|-------------|---|---------|-------|------------|-------|------|--------|")
    for _, rt_row in rt_baselines.iterrows():
        rt_name = rt_row["report_type"] if rt_row["report_type"] else "Unknown"
        pct = rt_row["prevalence"] * 100
        report.append(
            f"| {rt_name} | {rt_row['n_narr']:,.0f} | {pct:.0f}% | "
            f"{rt_row['voice']:.2f} | {rt_row['leadership']:.2f} | "
            f"{rt_row['drift']:.2f} | {rt_row['care']:.2f} | {rt_row['growth']:.2f} |"
        )

    report.append("\n## Executive Dashboard")
    if adjusted:
        report.append("\nStandardised ratios per site (observed / expected given report type mix).")
        report.append("Empirical Bayes shrinkage applied — small-site estimates pulled toward the org mean.")
        report.append("1.00 = as expected. Flagged at FDR<0.05 (Benjamini-Hochberg, quasi-Poisson).")
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

    # Temporal trends — per-site quasi-Poisson GLM (only when not filtering to single FY)
    if not fy:
        trends = compute_temporal_trends(con, narr_filter)
        if trends:
            report.append("\n## Temporal Trends")
            report.append("\nSites with statistically significant year-on-year trends "
                          "(quasi-Poisson GLM, FDR<0.05). RR = rate ratio per year.")
            report.append("\n| Site | Indicator | RR/yr | Years | Direction |")
            report.append("|------|-----------|-------|-------|-----------|")
            import math
            for site, c, beta, p, n_years, direction in trends:
                rr = math.exp(beta)
                arrow = "\u2191" if direction == "rising" else "\u2193"
                report.append(
                    f"| {site} | {c.capitalize()} | {rr:.2f} | {n_years} | "
                    f"{arrow} {direction} |"
                )
        else:
            report.append("\n## Temporal Trends")
            report.append("\nNo statistically significant site-level trends detected (FDR<0.05).")

    report.append("\n## Temporal Trajectory")
    report.append("\nPrevalence and intensity (edges per signal-bearing narrative) by financial year:")
    report.append("\n| FY | N | Signal % | Voice | Leadership | Drift | Care | Growth |")
    report.append("|---|---|---------|-------|------------|-------|------|--------|")
    for _, row in temporal.iterrows():
        report.append(
            f"| {row['fy']:.0f} | {row['narratives']:,.0f} | {row['signal_pct']:.0f}% | "
            f"{row['voice']:.2f} | {row['leadership']:.2f} | {row['drift']:.2f} | "
            f"{row['care']:.2f} | {row['growth']:.2f} |"
        )

    # Power analysis — minimum detectable SMR per site size (adjusted view only)
    if adjusted:
        import math
        # Power analysis needs all-narrative baselines (SMR denominator), not signal-only intensity
        baselines_avg = {}
        for c in composites:
            eligible = rt_baselines[rt_baselines["n_narr"] >= 50]
            if len(eligible) > 0:
                # all-narrative rate = prevalence × intensity
                baselines_avg[c] = float(eligible["prevalence"].mean() * eligible[c].mean())
            else:
                baselines_avg[c] = 0.5
        power = compute_power_analysis(phis, eb_params, baselines_avg)
        report.append("\n## Detectable Effect Sizes")
        report.append("\nMinimum SMR deviation detectable at each site size "
                      "(95% CI excludes 1.0, quasi-Poisson + EB shrinkage):")
        report.append("\n| N | Voice | Leadership | Drift | Care | Growth |")
        report.append("|---|-------|------------|-------|------|--------|")
        for N, row in power:
            cols = " | ".join(f"{row[c][0]:.2f}–{row[c][1]:.2f}" for c in composites)
            report.append(f"| {N} | {cols} |")
        report.append("\n*Read: at N=50, only Voice SMRs below "
                      f"{power[3][1]['voice'][0]:.2f} or above "
                      f"{power[3][1]['voice'][1]:.2f} are detectable.*")

        # Sector confound analysis — standardised (prevalence + intensity per sector)
        sector_srt = con.execute(f"""
            WITH srt_narr AS (
                SELECT sector, report_type,
                    COUNT(*) AS n_narr,
                    COUNT(*) FILTER (cultural_edge_count > 0) AS n_signal
                FROM narratives GROUP BY sector, report_type
            ),
            srt_edges AS (
                SELECT n.sector, n.report_type,
                    COUNT(*) FILTER (e.edge_type IN {VOICE}) AS voice_e,
                    COUNT(*) FILTER (e.edge_type IN {LEADERSHIP}) AS leadership_e,
                    COUNT(*) FILTER (e.edge_type IN {DRIFT}) AS drift_e,
                    COUNT(*) FILTER (e.edge_type IN {CARE}) AS care_e,
                    COUNT(*) FILTER (e.edge_type IN {GROWTH}) AS growth_e
                FROM narratives n JOIN edges e ON e.narrative_id = n.id
                WHERE e.is_cultural = true AND n.cultural_edge_count > 0
                GROUP BY n.sector, n.report_type
            )
            SELECT sn.sector, sn.report_type, sn.n_narr, sn.n_signal,
                sn.n_signal::FLOAT / sn.n_narr AS prevalence,
                CASE WHEN sn.n_signal > 0
                    THEN COALESCE(se.voice_e, 0)::FLOAT / sn.n_signal ELSE 0 END AS voice,
                CASE WHEN sn.n_signal > 0
                    THEN COALESCE(se.leadership_e, 0)::FLOAT / sn.n_signal ELSE 0 END AS leadership,
                CASE WHEN sn.n_signal > 0
                    THEN COALESCE(se.drift_e, 0)::FLOAT / sn.n_signal ELSE 0 END AS drift,
                CASE WHEN sn.n_signal > 0
                    THEN COALESCE(se.care_e, 0)::FLOAT / sn.n_signal ELSE 0 END AS care,
                CASE WHEN sn.n_signal > 0
                    THEN COALESCE(se.growth_e, 0)::FLOAT / sn.n_signal ELSE 0 END AS growth
            FROM srt_narr sn
            LEFT JOIN srt_edges se ON sn.sector = se.sector AND sn.report_type = se.report_type
        """).fetchdf()

        sectors = [s for s in sector_srt["sector"].unique() if s is not None]
        if len(sectors) > 1:
            sector_rows = []
            for sector in sectors:
                s_data = sector_srt[sector_srt["sector"] == sector]
                s_total = int(s_data["n_narr"].sum())
                # Equal-weight across report types with N≥20 within this sector
                eligible = s_data[s_data["n_narr"] >= 20]
                if len(eligible) == 0:
                    continue
                s_prev = round(float(eligible["prevalence"].mean()) * 100)
                s_int = {c: round(float(eligible[c].mean()), 2) for c in composites}
                sector_rows.append((sector, s_total, s_prev, s_int))

            if sector_rows:
                report.append("\n## Sector Analysis")
                report.append("\nStandardised prevalence + intensity by sector "
                              "(equal-weighted across report types within each sector):")
                report.append("\n| Sector | N | Signal % | Voice | Leadership "
                              "| Drift | Care | Growth |")
                report.append("|--------|---|---------|-------|------------|"
                              "-------|------|--------|")
                for sector, n, prev, si in sorted(sector_rows, key=lambda x: -x[1]):
                    report.append(
                        f"| {sector} | {n:,} | {prev}% | "
                        f"{si['voice']:.2f} | {si['leadership']:.2f} | "
                        f"{si['drift']:.2f} | {si['care']:.2f} | {si['growth']:.2f} |"
                    )
                report.append("\n*Note: current SMRs use org-wide report-type baselines "
                              "(dominated by largest sector). Sector-level baselines "
                              "differ substantially — consider `--sector-adjust` if "
                              "within-sector comparison is needed.*")

        # Sensitivity analysis
        sens, n_base, s_base = compute_sensitivity(con, site_cis, composites, phis, eb_params)
        report.append("\n## Sensitivity Analysis")
        report.append(f"\nBaseline: {n_base} flags on {s_base} sites. "
                      "Stability under perturbations:")
        report.append("\n| Perturbation | Flags | Gained | Lost | Stable |")
        report.append("|-------------|-------|--------|------|--------|")
        for name, nf, ns, ng, nl, pct in sens:
            report.append(f"| {name} | {nf} | +{ng} | −{nl} | {pct:.0f}% |")

    # Batch drift monitoring
    batch_df, drift_alerts = compute_batch_drift(con)
    if len(batch_df) > 0:
        report.append("\n## Extraction Quality")
        report.append("\nCultural edges per narrative by extraction batch:")
        report.append("\n| Batch | Narratives | Edges/narr | Sites |")
        report.append("|-------|------------|------------|-------|")
        for _, br in batch_df.iterrows():
            bd = str(br['batch_date'])[:10]
            report.append(
                f"| {bd} | {br['n_narr']:,} | "
                f"{br['edges_per_narr']:.2f} | {br['sites']} |"
            )
        if drift_alerts:
            report.append("\n**Drift alerts:**")
            for alert in drift_alerts:
                report.append(f"- {alert}")
        elif len(batch_df) >= 3:
            report.append("\nNo extraction drift detected (all batches within 2σ).")

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

        # Funnel curves on log scale: SE(log(SMR)) = √(φ/E), limits = exp(±z·SE)
        e_range = np.linspace(max(1, es.min() * 0.8), es.max() * 1.1, 300)
        se_log = phi_sqrt / np.sqrt(e_range)
        upper_95 = np.exp(1.96 * se_log)
        lower_95 = np.exp(-1.96 * se_log)
        upper_998 = np.exp(3.09 * se_log)
        lower_998 = np.exp(-3.09 * se_log)

        ax.fill_between(e_range, lower_998, upper_998, alpha=0.08, color="steelblue")
        ax.fill_between(e_range, lower_95, upper_95, alpha=0.15, color="steelblue")
        ax.plot(e_range, upper_95, color="steelblue", linewidth=0.8, linestyle="--")
        ax.plot(e_range, lower_95, color="steelblue", linewidth=0.8, linestyle="--")
        ax.plot(e_range, upper_998, color="steelblue", linewidth=0.5, linestyle=":")
        ax.plot(e_range, lower_998, color="steelblue", linewidth=0.5, linestyle=":")
        ax.axhline(y=1.0, color="grey", linestyle="-", linewidth=0.8)

        # Plot sites — red if outside 95% funnel, blue if inside
        for j in range(len(es)):
            se_j = phi_sqrt / np.sqrt(es[j])
            ul = np.exp(1.96 * se_j)
            ll = np.exp(-1.96 * se_j)
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
        # Clip y-axis to data range (log-scale limits blow up at small E)
        if len(smrs) > 0:
            ymax = min(max(smrs) * 1.3, upper_95[0] * 1.1)
            ax.set_ylim(bottom=0, top=max(ymax, 2.5))
        else:
            ax.set_ylim(bottom=0)
        ax.tick_params(labelsize=8)

    # Hide 6th subplot
    ax6 = axes[5]
    ax6.set_visible(False)

    fig.suptitle("Cultural Graph — Funnel Plots (SMR vs Expected Count, log-scale quasi-Poisson)",
                 fontweight="bold", fontsize=13)
    fig.tight_layout(rect=[0, 0, 1, 0.95])

    output_dir.mkdir(parents=True, exist_ok=True)
    out_path = output_dir / "funnel-plots.png"
    fig.savefig(out_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"Funnel plots: {out_path}")
    return out_path


def compute_batch_drift(con, z_threshold=2.0):
    """Monitor AI model drift across extraction batches.

    Compares per-batch cultural edge rates against the historical mean.
    Flags batches where the rate deviates by more than z_threshold standard
    deviations. Returns (batch_df, alerts) where alerts is a list of strings.
    """
    import numpy as np

    batch_df = con.execute("""
        SELECT extracted_at::DATE AS batch_date,
            COUNT(*) AS n_narr,
            SUM(cultural_edge_count) AS total_edges,
            ROUND(SUM(cultural_edge_count)::FLOAT / COUNT(*), 3) AS edges_per_narr,
            COUNT(DISTINCT site) AS sites
        FROM narratives
        GROUP BY extracted_at::DATE
        ORDER BY batch_date
    """).fetchdf()

    alerts = []
    if len(batch_df) < 2:
        return batch_df, alerts

    rates = batch_df["edges_per_narr"].values.astype(float)
    mean_rate = np.mean(rates)
    std_rate = np.std(rates, ddof=1) if len(rates) > 2 else np.std(rates)

    if std_rate > 0:
        for _, row in batch_df.iterrows():
            z = abs(row["edges_per_narr"] - mean_rate) / std_rate
            if z > z_threshold:
                direction = "above" if row["edges_per_narr"] > mean_rate else "below"
                alerts.append(
                    f"Batch {row['batch_date']}: {row['edges_per_narr']:.2f} edges/narr "
                    f"({z:.1f}σ {direction} mean {mean_rate:.2f})"
                )

    return batch_df, alerts


def compute_sensitivity(con, site_cis, composites, phis, eb_params):
    """Test flag robustness under perturbations.

    Runs the full pipeline under altered assumptions and reports how many
    flags are stable vs gained/lost. Tests: phi scaling (±20%), baseline
    perturbation (±5%, ±10%), N threshold changes (5, 15, 20).
    Returns list of (name, n_flags, n_sites, n_gained, n_lost, stable_pct).
    """
    import math
    import numpy as np
    from scipy.stats import norm as _norm

    def _flag_set_from_cis(site_cis_in, composites_in, phis_in, eb_scale=1.0):
        """Compute FDR flags and return as set of (site, flag) tuples.
        eb_scale multiplies the EB prior alpha/beta (>1 = stronger shrinkage)."""
        eb_r, eb_p = empirical_bayes_shrinkage(site_cis_in, composites_in)
        # Scale EB priors
        if eb_scale != 1.0:
            eb_p = {c: (a * eb_scale, b * eb_scale) for c, (a, b) in eb_p.items()}
            # Recompute shrunken estimates with scaled priors
            eb_r = {}
            for site, cis_site in site_cis_in.items():
                for c in composites_in:
                    ci = cis_site.get(c)
                    if ci and ci[4] > 0:
                        O, E = ci[3], ci[4]
                        a_p, b_p = eb_p[c]
                        smr_s = (O + a_p) / (E + b_p)
                        if site not in eb_r:
                            eb_r[site] = {}
                        eb_r[site][c] = (smr_s, ci[0], O, E, 0)
        cis_copy = {}
        for site in site_cis_in:
            cis_copy[site] = dict(site_cis_in[site])
        for site in eb_r:
            for c in composites_in:
                if c in eb_r[site]:
                    smr_s = eb_r[site][c][0]
                    old = cis_copy[site][c]
                    O, E = old[3], old[4]
                    phi = phis_in.get(c, 1.0)
                    a_p = eb_p[c][0]
                    if O > 0:
                        se = math.sqrt(phi / (O + a_p))
                        z_v = _norm.ppf(0.975)
                        lo = smr_s * math.exp(-z_v * se)
                        hi = smr_s * math.exp(z_v * se)
                    else:
                        lo, hi = old[1], old[2]
                    cis_copy[site][c] = (smr_s, lo, hi, O, E)
        fdr = apply_fdr(cis_copy, composites_in, phis=phis_in)
        fs = set()
        for s, fl in fdr.items():
            for f in fl:
                fs.add((s, f))
        return fs, len(fdr)

    def _rebuild_cis(phi_scale=1.0, baseline_perturb=0.0, min_n=10):
        baselines_df = con.execute("""
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
        site_rt = con.execute(
            "SELECT site, report_type, COUNT(*) AS n FROM narratives GROUP BY site, report_type"
        ).fetchdf()
        bl_lookup = {}
        for _, r in baselines_df.iterrows():
            bl_lookup[r["report_type"]] = {
                c: float(r[f"{c}_bl"]) * (1.0 + baseline_perturb) for c in composites
            }
        dashboard = con.execute(f"""
            WITH site_narr AS (SELECT site, COUNT(*) AS n_narr FROM narratives GROUP BY site),
            site_edges AS (
                SELECT n.site,
                    COUNT(*) FILTER (e.edge_type IN ('speaks-up-to', 'cooperates-with', 'shares-information-with')) AS voice_e,
                    COUNT(*) FILTER (e.edge_type IN ('directs', 'monitors')) AS leadership_e,
                    COUNT(*) FILTER (e.edge_type IN ('normalises', 'adapts-to')) AS drift_e,
                    COUNT(*) FILTER (e.edge_type IN ('cares-for', 'responds-to-failure-of', 'protects')) AS care_e,
                    COUNT(*) FILTER (e.edge_type IN ('learns-from', 'recognises')) AS growth_e
                FROM narratives n JOIN edges e ON e.narrative_id = n.id WHERE e.is_cultural = true GROUP BY n.site)
            SELECT sn.site, sn.n_narr AS n,
                COALESCE(se.voice_e, 0)::FLOAT / sn.n_narr AS voice,
                COALESCE(se.leadership_e, 0)::FLOAT / sn.n_narr AS leadership,
                COALESCE(se.drift_e, 0)::FLOAT / sn.n_narr AS drift,
                COALESCE(se.care_e, 0)::FLOAT / sn.n_narr AS care,
                COALESCE(se.growth_e, 0)::FLOAT / sn.n_narr AS growth
            FROM site_narr sn LEFT JOIN site_edges se ON sn.site = se.site
            WHERE sn.n_narr >= {min_n} ORDER BY sn.n_narr DESC
        """).fetchdf()
        scaled = {k: v * phi_scale for k, v in phis.items()}
        cis = {}
        for idx, r in dashboard.iterrows():
            site = r["site"]
            n = float(r["n"])
            sd = site_rt[site_rt["site"] == site]
            st = sd["n"].sum()
            c_dict = {}
            for c in composites:
                obs = round(float(r[c]) * n)
                er = sum(
                    (sr["n"] / st) * bl_lookup.get(sr["report_type"], {}).get(c, 0.0)
                    for _, sr in sd.iterrows()
                )
                exp = er * n
                smr, lo, hi = smr_poisson_ci(obs, exp, phi=scaled.get(c, 1.0))
                c_dict[c] = (smr, lo, hi, obs, exp)
            cis[site] = c_dict
        return cis, scaled

    # Baseline
    base_flags, base_sites = _flag_set_from_cis(site_cis, composites, phis)

    # Tests: (name, rebuild_kwargs, eb_scale)
    tests = [
        ("phi ×0.8",      {"phi_scale": 0.8},    1.0),
        ("phi ×1.2",      {"phi_scale": 1.2},    1.0),
        ("baseline +5%",  {"baseline_perturb": 0.05}, 1.0),
        ("baseline −5%",  {"baseline_perturb": -0.05}, 1.0),
        ("baseline +10%", {"baseline_perturb": 0.10}, 1.0),
        ("baseline −10%", {"baseline_perturb": -0.10}, 1.0),
        ("N≥5",           {"min_n": 5},          1.0),
        ("N≥20",          {"min_n": 20},         1.0),
        ("EB ×0.5",       {},                    0.5),
        ("EB ×2.0",       {},                    2.0),
    ]
    results = []
    for name, kwargs, eb_s in tests:
        if kwargs:
            cis_t, phis_t = _rebuild_cis(**kwargs)
        else:
            cis_t, phis_t = site_cis, phis
        flags_t, sites_t = _flag_set_from_cis(cis_t, composites, phis_t, eb_scale=eb_s)
        gained = len(flags_t - base_flags)
        lost = len(base_flags - flags_t)
        stable = len(base_flags & flags_t)
        pct = stable / len(base_flags) * 100 if base_flags else 100
        results.append((name, len(flags_t), sites_t, gained, lost, round(pct, 1)))
    return results, len(base_flags), base_sites


def compute_power_analysis(phis, eb_params, baselines_avg,
                           site_sizes=(10, 20, 30, 50, 100, 200, 500, 1000)):
    """Compute minimum detectable SMR at various site sizes.

    For quasi-Poisson with EB shrinkage:
      SE(log(SMR)) = sqrt(phi / (E + alpha))
      Detectable when |log(SMR)| > z * SE
      => SMR must be outside [exp(-z*SE), exp(z*SE)]

    Returns list of (N, {composite: (smr_lo, smr_hi)}).
    """
    import math

    z = norm.ppf(0.975)
    composites = ["voice", "leadership", "drift", "care", "growth"]
    results = []
    for N in site_sizes:
        row = {}
        for c in composites:
            phi = phis.get(c, 1.0)
            alpha_p = eb_params[c][0] if eb_params else 0
            bl = baselines_avg.get(c, 0.5)
            E = bl * N
            se = math.sqrt(phi / (E + alpha_p))
            row[c] = (math.exp(-z * se), math.exp(z * se))
        results.append((N, row))
    return results


def generate_caterpillar(site_cis, output_dir, phis=None, eb_params=None):
    """Generate caterpillar plots — SMRs with CIs ordered by value.

    One subplot per composite. Sites ordered by SMR (ascending), horizontal
    error bars showing 95% CI. Sites where CI excludes 1.0 are coloured red.
    Complements funnel plots: funnel shows the volume effect, caterpillar
    shows the ranking and precision of each site.
    """
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np

    composites = ["voice", "leadership", "drift", "care", "growth"]
    titles = ["Voice", "Leadership", "Drift", "Care", "Growth"]
    if phis is None:
        phis = {c: 1.0 for c in composites}

    fig, axes = plt.subplots(1, 5, figsize=(22, 10), sharey=False)

    for i, (c, title) in enumerate(zip(composites, titles)):
        ax = axes[i]
        entries = []
        for site, cis in site_cis.items():
            ci = cis.get(c)
            if ci and not np.isnan(ci[0]) and ci[4] > 0:
                smr, lo, hi = ci[0], ci[1], ci[2]
                short = site.split(" ")[-1] if " " in site else site
                entries.append((smr, lo, hi, short))

        if not entries:
            ax.set_visible(False)
            continue

        # Sort by SMR ascending
        entries.sort(key=lambda x: x[0])
        smrs = [e[0] for e in entries]
        los = [e[1] for e in entries]
        his = [e[2] for e in entries]
        labels = [e[3] for e in entries]
        ys = np.arange(len(entries))

        for j in range(len(entries)):
            # Red if CI excludes 1.0
            sig = his[j] < 1.0 or los[j] > 1.0
            color = "firebrick" if sig else "steelblue"
            ax.plot([los[j], his[j]], [ys[j], ys[j]], color=color,
                    linewidth=1.2, alpha=0.7, solid_capstyle="round")
            ax.plot(smrs[j], ys[j], "o", color=color, markersize=3.5,
                    zorder=5)

        ax.axvline(x=1.0, color="grey", linestyle="-", linewidth=0.8)
        ax.set_yticks(ys)
        ax.set_yticklabels(labels, fontsize=6)
        ax.set_xlabel("SMR", fontsize=9)
        phi = phis.get(c, 1.0)
        phi_label = f" (φ={phi:.1f})" if phi > 1.05 else ""
        ax.set_title(f"{title}{phi_label}", fontweight="bold", fontsize=11)
        ax.tick_params(axis="x", labelsize=8)
        # Clip x-axis to reasonable range
        xmax = min(max(his) * 1.1, 5.0)
        ax.set_xlim(left=0, right=max(xmax, 2.5))

    fig.suptitle("Cultural Graph — Caterpillar Plots (SMR with 95% CI, ordered by value)",
                 fontweight="bold", fontsize=13)
    fig.tight_layout(rect=[0, 0, 1, 0.95])

    output_dir.mkdir(parents=True, exist_ok=True)
    out_path = output_dir / "caterpillar-plots.png"
    fig.savefig(out_path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"Caterpillar plots: {out_path}")
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

    # Org averages — standardised prevalence + intensity (equal-weighted across report types, N≥50)
    prevalence, intensity, _rt_bl, _cis = compute_standardised_averages(con, narr_filter, join_filter)
    avgs = intensity
    avg_voice = intensity["voice"]
    avg_leadership = intensity["leadership"]
    avg_drift = intensity["drift"]
    avg_care = intensity["care"]
    avg_growth = intensity["growth"]

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

    # Temporal trajectory — prevalence + intensity by FY
    temporal = con.execute(f"""
        WITH fy_narr AS (
            SELECT fy,
                COUNT(*) AS n_narr,
                COUNT(*) FILTER (cultural_edge_count > 0) AS n_signal
            FROM narratives {narr_filter} GROUP BY fy
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
            WHERE e.is_cultural = true AND n.cultural_edge_count > 0 {join_filter}
            GROUP BY n.fy
        )
        SELECT fn.fy, fn.n_narr AS narratives,
            ROUND(fn.n_signal::FLOAT / fn.n_narr * 100, 0) AS signal_pct,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.voice_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS voice,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.leadership_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS leadership,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.drift_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS drift,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.care_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS care,
            CASE WHEN fn.n_signal > 0
                THEN ROUND(COALESCE(fe.growth_e, 0)::FLOAT / fn.n_signal, 2) ELSE 0 END AS growth
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

        # Empirical Bayes shrinkage
        import math
        eb_results, eb_params = empirical_bayes_shrinkage(site_cis, composites)
        for idx, row in dashboard.iterrows():
            site = row["site"]
            if site in eb_results:
                for c in composites:
                    if c in eb_results[site]:
                        smr_shrunk = eb_results[site][c][0]
                        old = site_cis[site][c]
                        dashboard.at[idx, c] = round(smr_shrunk, 2)
                        O, E = old[3], old[4]
                        phi = phis.get(c, 1.0)
                        alpha_p, beta_p = eb_params[c]
                        if O > 0:
                            se_log = math.sqrt(phi / (O + alpha_p))
                            z_val = norm.ppf(0.975)
                            lo = smr_shrunk * math.exp(-z_val * se_log)
                            hi = smr_shrunk * math.exp(z_val * se_log)
                        else:
                            lo, hi = old[1], old[2]
                        site_cis[site][c] = (smr_shrunk, lo, hi, O, E)

    # Compute current flags
    current_flags = {}
    if adjusted:
        fdr_flags = apply_fdr(site_cis, composites, phis=phis)
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
    entry.append(f"**Prevalence**: {prevalence:.0%} of narratives contain cultural signal")
    entry.append("")
    entry.append(
        f"**Intensity** (standardised, edges per signal-bearing narrative): "
        f"Voice {avg_voice:.2f} | Leadership {avg_leadership:.2f} | "
        f"Drift {avg_drift:.2f} | Care {avg_care:.2f} | Growth {avg_growth:.2f}"
    )
    entry.append("")
    entry.append("### Temporal Trajectory")
    entry.append("")
    entry.append("| FY | N | Signal % | Voice | Leadership | Drift | Care | Growth |")
    entry.append("|---|---|---------|-------|------------|-------|------|--------|")
    for _, row in temporal.iterrows():
        entry.append(
            f"| {row['fy']:.0f} | {row['narratives']:,.0f} | {row['signal_pct']:.0f}% | "
            f"{row['voice']:.2f} | {row['leadership']:.2f} | {row['drift']:.2f} | "
            f"{row['care']:.2f} | {row['growth']:.2f} |"
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
    parser.add_argument("--caterpillar", action="store_true",
                        help="Generate caterpillar plots (PNG) — SMRs with CIs ordered by value")
    parser.add_argument("--output", help="Output file path (default: auto-named)")
    args = parser.parse_args()

    if not args.template and not args.csv and not args.monthly_tracker and not args.funnel and not args.caterpillar:
        parser.error("Specify --template, --csv, --monthly-tracker, --funnel, or --caterpillar")

    if (args.funnel or args.caterpillar) and args.raw:
        parser.error("--funnel/--caterpillar require adjusted SMR data (incompatible with --raw)")

    con = get_connection()

    # Generate template first if needed (funnel needs its site_cis output)
    site_cis = {}
    if args.template or args.funnel or args.caterpillar:
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

    if args.funnel or args.caterpillar:
        if not site_cis:
            parser.error("--funnel/--caterpillar require adjusted SMR data (no site_cis computed)")
        phis = compute_overdispersion(con, ["voice", "leadership", "drift", "care", "growth"])
        if args.funnel:
            generate_funnel(site_cis, OUTPUT_DIR, phis=phis)
        if args.caterpillar:
            generate_caterpillar(site_cis, OUTPUT_DIR, phis=phis)

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
