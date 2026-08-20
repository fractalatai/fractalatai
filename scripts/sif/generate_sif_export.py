#!/usr/bin/env python3
"""Generate SIF analysis export for PowerBI and standard reports.

Joins Qwen mechanism extraction + band selection + calibrated P(SIF)
back to original QQ event records. Produces:
  --csv      Flat CSV for PowerBI ingestion (one row per event)
  --report   Markdown summary report with statistics

Usage:
    /usr/bin/python3 scripts/sif/generate_sif_export.py --csv
    /usr/bin/python3 scripts/sif/generate_sif_export.py --report
    /usr/bin/python3 scripts/sif/generate_sif_export.py --csv --report
"""

import argparse
import csv
import json
import math
from collections import Counter
from datetime import datetime
from pathlib import Path

import duckdb

DUCKDB_PATH = Path("data/sif.duckdb")
QWEN_FILE = Path("data/sif/benchmarks/qq_zeroshot_full.json")
BAND_FILE = Path("data/sif/benchmarks/band_selection_full.json")
CAL_DIR = Path("data/sif/calibration")
SIF_CSV = Path("data/qq/sif/SIF.csv")
OUTPUT_DIR = Path("data/qq/sif")
REPORT_DIR = OUTPUT_DIR / "reports"

MECHANISM_TO_FILE = {
    "transport": "motion_transport.json", "fall": "falls_gravity.json",
    "struck": "motion_struck.json", "caught_in": "caught_in.json",
    "explosion": "explosion.json", "fire": "fire.json",
    "electrical": "electrical.json", "thermal": "thermal.json",
    "chemical": "chemical.json", "breathing": "breathing.json",
    "pressure": "pressure.json", "structural_collapse": "structural_collapse.json",
    "assault": "assault.json", "overexertion": "overexertion.json",
    "slip_no_fall": "slip_no_fall.json", "abrasion": "abrasion.json",
    "radiation_noise": "radiation_noise.json", "animal_insect": "animal_insect.json",
}

MECHANISM_MAP = {
    "transport": "transport", "fall": "fall", "struck": "struck",
    "caught_in": "caught_in", "explosion": "explosion", "fire": "fire",
    "electrical": "electrical", "thermal": "thermal", "chemical": "chemical",
    "breathing": "breathing", "pressure": "pressure",
    "structural_collapse": "structural_collapse", "assault": "assault",
    "overexertion": "overexertion", "slip_no_fall": "slip_no_fall",
    "abrasion": "abrasion", "radiation_noise": "radiation_noise",
    "animal_insect": "animal_insect",
    "collision": "transport", "cut": "struck", "cutting": "struck",
    "mechanical": "caught_in", "contact": "struck", "impact": "struck",
    "biological": "animal_insect", "respiratory": "breathing",
    "inhalation": "breathing", "exposure": "chemical",
}


# --- Metalog P(SIF) computation ---

def logit(y):
    return math.log(y / (1.0 - y))


def bounded_transform(x, lb=0.0, ub=1.0):
    return math.log((x - lb) / (ub - x))


def spt_fit(p10, p50, p90):
    l90 = logit(0.9)
    return p50, (p90 - p10) / (2 * l90), (p90 + p10 - 2.0 * p50) / (2.0 * 0.4 * l90)


def metalog_q(y, a1, a2, a3):
    l = logit(y)
    return a1 + a2 * l + a3 * (y - 0.5) * l


def p_sif_bounded(p10, p50, p90, threshold=0.10):
    if abs(p10 - p90) < 1e-6:
        return 1.0 if p50 >= threshold else 0.0
    z10, z50, z90 = bounded_transform(p10), bounded_transform(p50), bounded_transform(p90)
    a1, a2, a3 = spt_fit(z10, z50, z90)
    z_thresh = bounded_transform(threshold)
    if metalog_q(0.999, a1, a2, a3) < z_thresh:
        return 0.0
    if metalog_q(0.001, a1, a2, a3) >= z_thresh:
        return 1.0
    lo, hi = 0.001, 0.999
    for _ in range(50):
        mid = (lo + hi) / 2.0
        if metalog_q(mid, a1, a2, a3) < z_thresh:
            lo = mid
        else:
            hi = mid
    return 1.0 - (lo + hi) / 2.0


def sif_label(p_sif):
    if p_sif >= 0.50:
        return "SIF"
    elif p_sif >= 0.10:
        return "ELEVATED"
    else:
        return "NON_SIF"


# --- Data loading ---

def load_calibrations():
    cal_cache = {}
    for mech, fname in MECHANISM_TO_FILE.items():
        fpath = CAL_DIR / fname
        if fpath.exists():
            with open(fpath) as f:
                cal_cache[mech] = json.load(f)
    return cal_cache


def load_events():
    con = duckdb.connect(str(DUCKDB_PATH), read_only=True)
    rows = con.execute("""
        SELECT event_id, site, narrative, action, report_type, fy, sector, sub_sector,
               event_date, hazard_category, event_type_class, qq_sifp,
               qq_ed_sifp, qq_t_sifp, qq_i_sifp, qq_i_sifa
        FROM events
    """).fetchall()
    cols = ["event_id", "site", "narrative", "action", "report_type", "fy", "sector",
            "sub_sector", "event_date", "hazard_category", "event_type_class", "qq_sifp",
            "qq_ed_sifp", "qq_t_sifp", "qq_i_sifp", "qq_i_sifa"]
    con.close()
    return {str(r[0]): dict(zip(cols, r)) for r in rows}


def load_ed_report_types():
    """Load triager (ED) report types from SIF.csv source."""
    ed_map = {}
    if SIF_CSV.exists():
        with open(SIF_CSV, encoding="latin-1") as f:
            reader = csv.DictReader(f)
            for row in reader:
                eid = row.get("RowSignatureII", "").strip()
                ed_rt = row.get("ED_ReportType", "").strip()
                if eid and ed_rt:
                    ed_map[eid] = ed_rt
    return ed_map


def build_joined_records():
    """Join DuckDB events + Qwen Pass 1 + Band Selection + Calibrated P(SIF)."""
    events = load_events()
    cal_cache = load_calibrations()
    ed_report_types = load_ed_report_types()

    # Load Qwen zero-shot
    with open(QWEN_FILE) as f:
        qwen_data = {str(e["event_id"]): e for e in json.load(f)}

    # Load band selection
    with open(BAND_FILE) as f:
        band_data = {str(e["event_id"]): e for e in json.load(f)}

    records = []
    for event_id, evt in events.items():
        rec = {
            # Original QQ fields
            "event_id": evt["event_id"],
            "event_date": evt["event_date"],
            "fy": evt["fy"],
            "site": evt["site"],
            "sector": evt["sector"],
            "sub_sector": evt["sub_sector"],
            "report_type": evt["report_type"],
            "ed_report_type": ed_report_types.get(str(evt["event_id"]), evt["report_type"]),
            "hazard_category": evt["hazard_category"],
            "event_type_class": evt["event_type_class"],
            "narrative": evt["narrative"],
            "action": evt["action"],
            # QQ human SIF labels
            "qq_sifp": evt["qq_sifp"],
            "qq_ed_sifp": evt["qq_ed_sifp"],
            "qq_t_sifp": evt["qq_t_sifp"],
            "qq_i_sifp": evt["qq_i_sifp"],
            "qq_i_sifa": evt["qq_i_sifa"],
        }

        # Qwen Pass 1: mechanism + energy
        qw = qwen_data.get(event_id)
        if qw:
            pred = qw.get("prediction", {})
            raw_mech = pred.get("mechanism", "")
            mapped_mech = MECHANISM_MAP.get(raw_mech, "")
            rec["sif_mechanism_raw"] = raw_mech
            rec["sif_mechanism"] = mapped_mech
            rec["sif_energy_types"] = "|".join(pred.get("energy_types", []))
            rec["sif_source_properties"] = pred.get("source_properties", "")
            rec["sif_body_vulnerability"] = pred.get("body_vulnerability", "")
            rec["sif_qwen_p50"] = pred.get("severity_p50", "")
            rec["sif_reasoning_pass1"] = pred.get("reasoning", "")
        else:
            rec.update({
                "sif_mechanism_raw": "", "sif_mechanism": "",
                "sif_energy_types": "", "sif_source_properties": "",
                "sif_body_vulnerability": "", "sif_qwen_p50": "",
                "sif_reasoning_pass1": "",
            })

        # Band selection (Pass 2)
        bs = band_data.get(event_id)
        if bs:
            rec["sif_band"] = bs.get("band_selected", "")
            rec["sif_band_confidence"] = bs.get("band_confidence", "")
            rec["sif_reasoning_pass2"] = bs.get("reasoning", "")
            ev = bs.get("extracted_values", {})
            rec["sif_height_m"] = ev.get("height_m")
            rec["sif_voltage_v"] = ev.get("voltage_v")
            rec["sif_speed_kmh"] = ev.get("speed_kmh")
            rec["sif_temperature_c"] = ev.get("temperature_c")
            rec["sif_mass_kg"] = ev.get("mass_kg")
        else:
            rec.update({
                "sif_band": "", "sif_band_confidence": "",
                "sif_reasoning_pass2": "",
                "sif_height_m": None, "sif_voltage_v": None,
                "sif_speed_kmh": None, "sif_temperature_c": None,
                "sif_mass_kg": None,
            })

        # Calibrated P(SIF)
        mech = rec.get("sif_mechanism", "")
        band_name = rec.get("sif_band", "")
        cal = cal_cache.get(mech)
        if cal and band_name:
            band = None
            for b in cal["magnitude_bands"]:
                if b["band"] == band_name:
                    band = b
                    break
            if not band:
                band = cal["magnitude_bands"][0]
            p10, p50, p90 = band["p10"], band["p50"], band["p90"]
            psif = p_sif_bounded(p10, p50, p90)
            rec["sif_p_sif"] = round(psif, 3)
            rec["sif_classification"] = sif_label(psif)
            rec["sif_cal_p10"] = p10
            rec["sif_cal_p50"] = p50
            rec["sif_cal_p90"] = p90
        else:
            rec["sif_p_sif"] = None
            rec["sif_classification"] = ""
            rec["sif_cal_p10"] = None
            rec["sif_cal_p50"] = None
            rec["sif_cal_p90"] = None

        records.append(rec)

    return records


# --- CSV export ---

CSV_COLUMNS = [
    # Original QQ
    "event_id", "event_date", "fy", "site", "sector", "sub_sector",
    "report_type", "ed_report_type", "hazard_category", "event_type_class",
    # QQ human SIF
    "qq_sifp", "qq_ed_sifp", "qq_t_sifp",
    # SIF analysis
    "sif_mechanism", "sif_energy_types", "sif_band", "sif_band_confidence",
    "sif_p_sif", "sif_classification",
    # Extracted magnitudes
    "sif_height_m", "sif_voltage_v", "sif_speed_kmh", "sif_temperature_c", "sif_mass_kg",
    # Context
    "sif_source_properties", "sif_body_vulnerability", "sif_qwen_p50",
    # Calibration detail
    "sif_cal_p10", "sif_cal_p50", "sif_cal_p90",
    # Reasoning (for audit)
    "sif_reasoning_pass1", "sif_reasoning_pass2",
    # Full text (last — long columns at end for PowerBI usability)
    "narrative", "action",
]


def export_csv(records, output_path):
    with open(output_path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=CSV_COLUMNS, extrasaction="ignore")
        writer.writeheader()
        for rec in records:
            # Clean for CSV: None → empty, strip newlines from text fields
            row = {}
            for k, v in rec.items():
                if v is None:
                    row[k] = ""
                elif isinstance(v, str):
                    row[k] = v.replace("\n", " ").replace("\r", " ")
                else:
                    row[k] = v
            writer.writerow(row)
    print(f"CSV: {len(records)} rows → {output_path}")

    # Compressed copy for upload
    import gzip
    import shutil
    gz_path = Path(str(output_path) + ".gz")
    with open(output_path, "rb") as f_in, gzip.open(gz_path, "wb") as f_out:
        shutil.copyfileobj(f_in, f_out)
    csv_size = output_path.stat().st_size
    gz_size = gz_path.stat().st_size
    ratio = 100 * (1 - gz_size / csv_size)
    print(f"  gz: {gz_size/1024:.0f} KB ({ratio:.0f}% smaller) → {gz_path}")


# --- Report generation ---

def generate_report(records, output_path):
    """Generate markdown summary report."""
    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    analysed = [r for r in records if r.get("sif_classification")]
    total = len(records)
    n_analysed = len(analysed)

    sif = [r for r in analysed if r["sif_classification"] == "SIF"]
    elev = [r for r in analysed if r["sif_classification"] == "ELEVATED"]
    non = [r for r in analysed if r["sif_classification"] == "NON_SIF"]

    lines = []
    lines.append(f"# SIF Analysis Report")
    lines.append(f"")
    lines.append(f"Generated: {now}")
    lines.append(f"")
    lines.append(f"## Summary")
    lines.append(f"")
    lines.append(f"| Metric | Value |")
    lines.append(f"|--------|-------|")
    lines.append(f"| Total events | {total:,} |")
    lines.append(f"| Analysed | {n_analysed:,} |")
    lines.append(f"| SIF (P(SIF) >= 0.50) | {len(sif):,} ({100*len(sif)/n_analysed:.1f}%) |")
    lines.append(f"| ELEVATED (0.10 - 0.50) | {len(elev):,} ({100*len(elev)/n_analysed:.1f}%) |")
    lines.append(f"| NON_SIF (< 0.10) | {len(non):,} ({100*len(non)/n_analysed:.1f}%) |")
    lines.append(f"")

    # Comparison with QQ human labels
    qq_sif = [r for r in analysed if r["qq_sifp"] in ("1 Fatal", "2 Massive", "3 Very High", "4 High")]
    lines.append(f"| QQ human SIF-potential | {len(qq_sif):,} ({100*len(qq_sif)/n_analysed:.1f}%) |")
    lines.append(f"")

    # By mechanism
    lines.append(f"## By Mechanism")
    lines.append(f"")
    lines.append(f"| Mechanism | Events | SIF | ELEVATED | NON_SIF | SIF % |")
    lines.append(f"|-----------|--------|-----|----------|---------|-------|")
    mech_counts = Counter(r["sif_mechanism"] for r in analysed)
    for mech, n in mech_counts.most_common():
        subset = [r for r in analysed if r["sif_mechanism"] == mech]
        n_sif = sum(1 for r in subset if r["sif_classification"] == "SIF")
        n_elev = sum(1 for r in subset if r["sif_classification"] == "ELEVATED")
        n_non = sum(1 for r in subset if r["sif_classification"] == "NON_SIF")
        pct = 100 * n_sif / n if n > 0 else 0
        lines.append(f"| {mech} | {n} | {n_sif} | {n_elev} | {n_non} | {pct:.0f}% |")
    lines.append(f"")

    # By sector
    lines.append(f"## By Sector")
    lines.append(f"")
    lines.append(f"| Sector | Events | SIF | SIF % | ELEVATED | NON_SIF |")
    lines.append(f"|--------|--------|-----|-------|----------|---------|")
    sector_counts = Counter(r.get("sector", "") for r in analysed)
    for sector, n in sector_counts.most_common():
        subset = [r for r in analysed if r.get("sector", "") == sector]
        n_sif = sum(1 for r in subset if r["sif_classification"] == "SIF")
        n_elev = sum(1 for r in subset if r["sif_classification"] == "ELEVATED")
        n_non = sum(1 for r in subset if r["sif_classification"] == "NON_SIF")
        pct = 100 * n_sif / n if n > 0 else 0
        lines.append(f"| {sector} | {n} | {n_sif} | {pct:.0f}% | {n_elev} | {n_non} |")
    lines.append(f"")

    # By report type (triager's ED classification)
    lines.append(f"## By Report Type (Triager)")
    lines.append(f"")
    lines.append(f"| Report Type | Events | SIF | SIF % |")
    lines.append(f"|-------------|--------|-----|-------|")
    rt_counts = Counter(r.get("ed_report_type", "") for r in analysed)
    for rt, n in rt_counts.most_common():
        subset = [r for r in analysed if r.get("ed_report_type", "") == rt]
        n_sif = sum(1 for r in subset if r["sif_classification"] == "SIF")
        pct = 100 * n_sif / n if n > 0 else 0
        lines.append(f"| {rt} | {n} | {n_sif} | {pct:.0f}% |")
    lines.append(f"")

    # Calibrated vs QQ human cross-tab
    lines.append(f"## Calibrated vs QQ Human SIFp")
    lines.append(f"")
    lines.append(f"| QQ SIFp | n | SIF | ELEVATED | NON_SIF | SIF % |")
    lines.append(f"|---------|---|-----|----------|---------|-------|")
    for label in ["1 Fatal", "2 Massive", "3 Very High", "4 High", "9 Not SIFp"]:
        subset = [r for r in analysed if r["qq_sifp"] == label]
        if not subset:
            continue
        n = len(subset)
        n_sif = sum(1 for r in subset if r["sif_classification"] == "SIF")
        n_elev = sum(1 for r in subset if r["sif_classification"] == "ELEVATED")
        n_non = sum(1 for r in subset if r["sif_classification"] == "NON_SIF")
        pct = 100 * n_sif / n if n > 0 else 0
        lines.append(f"| {label} | {n} | {n_sif} | {n_elev} | {n_non} | {pct:.0f}% |")
    lines.append(f"")

    # Top SIF events (highest P(SIF) with details)
    lines.append(f"## Top 20 SIF Events")
    lines.append(f"")
    lines.append(f"| Site | Mechanism | Band | P(SIF) | QQ SIFp | Narrative (first 100 chars) |")
    lines.append(f"|------|-----------|------|--------|---------|---------------------------|")
    sif_sorted = sorted(analysed, key=lambda r: r.get("sif_p_sif", 0) or 0, reverse=True)
    for r in sif_sorted[:20]:
        narr = (r.get("narrative", "") or "")[:100].replace("|", "/").replace("\n", " ")
        lines.append(f"| {r.get('site', '')} | {r['sif_mechanism']} | {r.get('sif_band', '')} | {r.get('sif_p_sif', 0):.2f} | {r['qq_sifp']} | {narr} |")
    lines.append(f"")

    # Band confidence
    lines.append(f"## Band Selection Confidence")
    lines.append(f"")
    conf_counts = Counter(r.get("sif_band_confidence", "") for r in analysed)
    for conf, n in conf_counts.most_common():
        lines.append(f"- **{conf}**: {n} ({100*n/n_analysed:.1f}%)")
    lines.append(f"")

    # Methodology note
    lines.append(f"## Methodology")
    lines.append(f"")
    lines.append(f"- **Pass 1**: Qwen 3 8B zero-shot mechanism extraction from narrative text")
    lines.append(f"- **Pass 2**: Qwen 3 8B band selection using calibration-derived prompts")
    lines.append(f"- **Calibration**: Literature-based dose-response curves (18 mechanisms, 87 bands)")
    lines.append(f"- **P(SIF)**: Probability of Serious Injury or Fatality from bounded metalog distribution")
    lines.append(f"- **SIF threshold**: P(death) >= 0.10 (AIS 3-4 severity, ~10% mortality)")
    lines.append(f"- **Default rule**: When narrative lacks magnitude detail, least-severe band is used (conservative)")
    lines.append(f"")

    report = "\n".join(lines)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        f.write(report)
    print(f"Report: {output_path}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--csv", action="store_true", help="Export PowerBI CSV")
    parser.add_argument("--report", action="store_true", help="Generate markdown report")
    parser.add_argument("--output-csv", default=None, help="CSV output path")
    parser.add_argument("--output-report", default=None, help="Report output path")
    args = parser.parse_args()

    if not args.csv and not args.report:
        args.csv = True
        args.report = True

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    REPORT_DIR.mkdir(parents=True, exist_ok=True)

    print("Building joined records...")
    records = build_joined_records()
    print(f"  {len(records)} events joined")

    if args.csv:
        csv_path = Path(args.output_csv) if args.output_csv else OUTPUT_DIR / "sif-powerbi.csv"
        export_csv(records, csv_path)

    if args.report:
        report_path = Path(args.output_report) if args.output_report else REPORT_DIR / "sif-report.md"
        generate_report(records, report_path)


if __name__ == "__main__":
    main()
