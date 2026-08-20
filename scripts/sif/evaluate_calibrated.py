#!/usr/bin/env python3
"""Evaluate Qwen zero-shot results through calibration curves.

Takes Qwen's mechanism + source_properties extraction, maps through
the calibration curves, and computes calibrated P(SIF). Compares against
QQ human SIFp labels and Qwen's own severity estimates.

Usage:
    /usr/bin/python3 scripts/sif/evaluate_calibrated.py
"""

import json
import math
import re
from collections import Counter
from pathlib import Path

CAL_DIR = Path("data/sif/calibration")
QWEN_FILE = Path("data/sif/benchmarks/qq_zeroshot_full.json")

# Map non-standard Qwen mechanisms to our 18 labels
MECHANISM_MAP = {
    # Standard labels (pass through)
    "transport": "transport",
    "fall": "fall",
    "struck": "struck",
    "caught_in": "caught_in",
    "explosion": "explosion",
    "fire": "fire",
    "electrical": "electrical",
    "thermal": "thermal",
    "chemical": "chemical",
    "breathing": "breathing",
    "pressure": "pressure",
    "structural_collapse": "structural_collapse",
    "assault": "assault",
    "overexertion": "overexertion",
    "slip_no_fall": "slip_no_fall",
    "abrasion": "abrasion",
    "radiation_noise": "radiation_noise",
    "animal_insect": "animal_insect",
    # Non-standard Qwen outputs
    "collision": "transport",
    "cut": "struck",
    "cutting": "struck",
    "mechanical": "caught_in",
    "contact": "struck",
    "impact": "struck",
    "biological": "animal_insect",
    "respiratory": "breathing",
    "inhalation": "breathing",
    "exposure": "chemical",
    "unknown": "unknown",
    "communication_failure": "unknown",
    "communication": "unknown",
}

# Map mechanism to calibration file
MECHANISM_TO_FILE = {
    "transport": "motion_transport.json",
    "fall": "falls_gravity.json",
    "struck": "motion_struck.json",
    "caught_in": "caught_in.json",
    "explosion": "explosion.json",
    "fire": "fire.json",
    "electrical": "electrical.json",
    "thermal": "thermal.json",
    "chemical": "chemical.json",
    "breathing": "breathing.json",
    "pressure": "pressure.json",
    "structural_collapse": "structural_collapse.json",
    "assault": "assault.json",
    "overexertion": "overexertion.json",
    "slip_no_fall": "slip_no_fall.json",
    "abrasion": "abrasion.json",
    "radiation_noise": "radiation_noise.json",
    "animal_insect": "animal_insect.json",
}

# P(death) scale for Qwen's ordinal labels
P_DEATH = {
    "no_injury": 0.0001,
    "first_aid": 0.001,
    "medical_treatment": 0.005,
    "serious_injury": 0.10,
    "fatality": 0.999,
}


def logit(y):
    return math.log(y / (1.0 - y))


def bounded_transform(x, lb=0.0, ub=1.0):
    return math.log((x - lb) / (ub - x))


def spt_fit(p10, p50, p90):
    l10 = logit(0.1)
    l90 = logit(0.9)
    a1 = p50
    a2 = (p90 - p10) / (l90 - l10)
    a3 = (p90 + p10 - 2.0 * p50) / (2.0 * 0.4 * l90)
    return a1, a2, a3


def metalog_quantile(y, a1, a2, a3):
    l = logit(y)
    return a1 + a2 * l + a3 * (y - 0.5) * l


def p_sif_bounded(p10, p50, p90, threshold=0.10):
    if abs(p10 - p90) < 1e-6:
        return 1.0 if p50 >= threshold else 0.0
    z10 = bounded_transform(p10)
    z50 = bounded_transform(p50)
    z90 = bounded_transform(p90)
    a1, a2, a3 = spt_fit(z10, z50, z90)
    z_thresh = bounded_transform(threshold)
    if metalog_quantile(0.999, a1, a2, a3) < z_thresh:
        return 0.0
    if metalog_quantile(0.001, a1, a2, a3) >= z_thresh:
        return 1.0
    lo, hi = 0.001, 0.999
    for _ in range(50):
        mid = (lo + hi) / 2.0
        if metalog_quantile(mid, a1, a2, a3) < z_thresh:
            lo = mid
        else:
            hi = mid
    return 1.0 - (lo + hi) / 2.0


def extract_height_m(source_props):
    """Extract height in metres from source_properties text."""
    if not source_props:
        return None
    # Match "height: Xm" or "X metres" or "X m" patterns
    m = re.search(r'height[:\s]*(\d+\.?\d*)\s*m', source_props, re.I)
    if m:
        return float(m.group(1))
    m = re.search(r'(\d+\.?\d*)\s*(?:metres|meters|m)\b', source_props, re.I)
    if m:
        return float(m.group(1))
    # Feet to metres
    m = re.search(r'(\d+\.?\d*)\s*(?:feet|foot|ft)\b', source_props, re.I)
    if m:
        return float(m.group(1)) * 0.3048
    return None


def extract_voltage(source_props):
    """Extract voltage from source_properties text."""
    if not source_props:
        return None
    m = re.search(r'(\d+\.?\d*)\s*(?:kV|kv)', source_props, re.I)
    if m:
        return float(m.group(1)) * 1000
    m = re.search(r'(\d+\.?\d*)\s*[Vv](?:olts?)?', source_props, re.I)
    if m:
        return float(m.group(1))
    return None


def extract_speed_kmh(source_props):
    """Extract speed in km/h from source_properties text."""
    if not source_props:
        return None
    m = re.search(r'(\d+\.?\d*)\s*(?:km/?h|kph)', source_props, re.I)
    if m:
        return float(m.group(1))
    m = re.search(r'(\d+\.?\d*)\s*(?:mph)', source_props, re.I)
    if m:
        return float(m.group(1)) * 1.609
    return None


def select_band(mechanism, source_props, cal_data):
    """Select the appropriate calibration band from source_properties."""
    bands = cal_data["magnitude_bands"]

    if mechanism == "fall":
        h = extract_height_m(source_props)
        if h is not None:
            for b in bands:
                lo = b.get("height_m_low", 0)
                hi = b.get("height_m_high")
                if hi is None:
                    if h >= lo:
                        return b
                elif lo <= h < hi:
                    return b
        # Default: low band (1-3m) — conservative middle
        return next((b for b in bands if b["band"] == "low"), bands[1] if len(bands) > 1 else bands[0])

    if mechanism == "electrical":
        v = extract_voltage(source_props)
        if v is not None:
            for b in bands:
                lo = b.get("voltage_low", 0)
                hi = b.get("voltage_high")
                if hi is None:
                    if v >= lo:
                        return b
                elif lo <= v < hi:
                    return b
        # Default: low band (50-240V)
        return next((b for b in bands if b["band"] == "low"), bands[1] if len(bands) > 1 else bands[0])

    if mechanism == "transport":
        s = extract_speed_kmh(source_props)
        if s is not None:
            for b in bands:
                lo = b.get("speed_low", 0)
                hi = b.get("speed_high")
                if hi is None:
                    if s >= lo:
                        return b
                elif lo <= s < hi:
                    return b
        # Default: yard_speed
        return next((b for b in bands if b["band"] == "yard_speed"), bands[0])

    # For all other mechanisms, use the middle band as default
    # (index 1 if >=3 bands, else index 0)
    mid_idx = len(bands) // 2
    return bands[min(mid_idx, len(bands) - 1)]


def qwen_p_sif(prediction):
    """Compute P(SIF) from Qwen's own ordinal severity estimates."""
    p50_label = prediction.get("severity_p50", "no_injury")
    p50_val = P_DEATH.get(p50_label, 0.001)
    # Simple threshold: if Qwen's P50 >= serious_injury, it's SIF
    return 1.0 if p50_val >= 0.10 else 0.0


def qq_is_sif(qq_sifp):
    """Is the QQ human label SIF-potential?"""
    return qq_sifp in ("1 Fatal", "2 Massive", "3 Very High", "4 High")


def main():
    # Load calibration files
    cal_cache = {}
    for mech, fname in MECHANISM_TO_FILE.items():
        fpath = CAL_DIR / fname
        if fpath.exists():
            with open(fpath) as f:
                cal_cache[mech] = json.load(f)

    # Load Qwen results
    with open(QWEN_FILE) as f:
        events = json.load(f)

    print(f"Events: {len(events)}")
    print()

    # Process each event
    results = []
    unmapped_mechs = Counter()
    band_selected = Counter()
    magnitude_found = Counter()

    for evt in events:
        pred = evt["prediction"]
        raw_mech = pred.get("mechanism", "unknown")
        mech = MECHANISM_MAP.get(raw_mech)

        if mech is None or mech == "unknown":
            unmapped_mechs[raw_mech] += 1
            results.append({
                "event_id": evt["event_id"],
                "qq_sifp": evt["qq_sifp"],
                "mechanism": raw_mech,
                "mapped": None,
                "band": None,
                "cal_p_sif": None,
                "qwen_p_sif": qwen_p_sif(pred),
                "qq_is_sif": qq_is_sif(evt["qq_sifp"]),
            })
            continue

        cal = cal_cache.get(mech)
        if not cal:
            unmapped_mechs[f"no_cal:{mech}"] += 1
            continue

        source_props = pred.get("source_properties", "")
        band = select_band(mech, source_props, cal)
        band_name = band["band"]
        band_selected[f"{mech}:{band_name}"] += 1

        # Check if we found a specific magnitude
        if mech == "fall" and extract_height_m(source_props) is not None:
            magnitude_found["fall_height"] += 1
        elif mech == "electrical" and extract_voltage(source_props) is not None:
            magnitude_found["electrical_voltage"] += 1
        elif mech == "transport" and extract_speed_kmh(source_props) is not None:
            magnitude_found["transport_speed"] += 1
        else:
            magnitude_found["default_band"] += 1

        p10, p50, p90 = band["p10"], band["p50"], band["p90"]
        cal_psif = p_sif_bounded(p10, p50, p90)

        results.append({
            "event_id": evt["event_id"],
            "qq_sifp": evt["qq_sifp"],
            "mechanism": raw_mech,
            "mapped": mech,
            "band": band_name,
            "cal_p_sif": cal_psif,
            "qwen_p_sif": qwen_p_sif(pred),
            "qq_is_sif": qq_is_sif(evt["qq_sifp"]),
        })

    # --- Reporting ---
    valid = [r for r in results if r["cal_p_sif"] is not None]
    print(f"Calibrated: {len(valid)} / {len(results)} events")
    if unmapped_mechs:
        print(f"\nUnmapped mechanisms:")
        for m, n in unmapped_mechs.most_common():
            print(f"  {m:25s} {n:>5}")

    print(f"\nMagnitude extraction:")
    for k, n in magnitude_found.most_common():
        print(f"  {k:25s} {n:>5}")

    # Calibrated P(SIF) distribution
    print(f"\n{'=' * 70}")
    print("CALIBRATED P(SIF) DISTRIBUTION")
    print(f"{'=' * 70}")
    cal_sif = [r for r in valid if r["cal_p_sif"] >= 0.50]
    cal_elev = [r for r in valid if 0.10 <= r["cal_p_sif"] < 0.50]
    cal_non = [r for r in valid if r["cal_p_sif"] < 0.10]
    print(f"  SIF (≥0.50):      {len(cal_sif):>5}  ({100*len(cal_sif)/len(valid):.1f}%)")
    print(f"  ELEVATED (0.10-):  {len(cal_elev):>5}  ({100*len(cal_elev)/len(valid):.1f}%)")
    print(f"  NON_SIF (<0.10):  {len(cal_non):>5}  ({100*len(cal_non)/len(valid):.1f}%)")

    # Qwen's own P(SIF) distribution (from ordinal labels)
    print(f"\nQWEN OWN SEVERITY (P50 >= serious_injury)")
    qwen_sif = [r for r in valid if r["qwen_p_sif"] >= 0.50]
    print(f"  Qwen says SIF:    {len(qwen_sif):>5}  ({100*len(qwen_sif)/len(valid):.1f}%)")

    # QQ human labels
    qq_sif = [r for r in valid if r["qq_is_sif"]]
    print(f"\nQQ HUMAN LABELS")
    print(f"  Human says SIF:   {len(qq_sif):>5}  ({100*len(qq_sif)/len(valid):.1f}%)")

    # Cross-tab: calibrated vs QQ
    print(f"\n{'=' * 70}")
    print("CALIBRATED P(SIF) vs QQ HUMAN SIFp")
    print(f"{'=' * 70}")
    for sifp_label in ["1 Fatal", "2 Massive", "3 Very High", "4 High", "9 Not SIFp"]:
        subset = [r for r in valid if r["qq_sifp"] == sifp_label]
        if not subset:
            continue
        n = len(subset)
        n_sif = sum(1 for r in subset if r["cal_p_sif"] >= 0.50)
        n_elev = sum(1 for r in subset if 0.10 <= r["cal_p_sif"] < 0.50)
        n_non = sum(1 for r in subset if r["cal_p_sif"] < 0.10)
        print(f"  {sifp_label:15s}  n={n:>4}  SIF={n_sif:>4} ({100*n_sif/n:>5.1f}%)  ELEV={n_elev:>4} ({100*n_elev/n:>5.1f}%)  NON={n_non:>4} ({100*n_non/n:>5.1f}%)")

    # Cross-tab: calibrated vs Qwen own
    print(f"\n{'=' * 70}")
    print("CALIBRATED vs QWEN OWN SEVERITY")
    print(f"{'=' * 70}")
    both_sif = sum(1 for r in valid if r["cal_p_sif"] >= 0.50 and r["qwen_p_sif"] >= 0.50)
    cal_only = sum(1 for r in valid if r["cal_p_sif"] >= 0.50 and r["qwen_p_sif"] < 0.50)
    qwen_only = sum(1 for r in valid if r["cal_p_sif"] < 0.50 and r["qwen_p_sif"] >= 0.50)
    neither = sum(1 for r in valid if r["cal_p_sif"] < 0.50 and r["qwen_p_sif"] < 0.50)
    print(f"  Both SIF:         {both_sif:>5}")
    print(f"  Calibrated only:  {cal_only:>5}  (curve sees SIF, Qwen conservative)")
    print(f"  Qwen only:        {qwen_only:>5}  (Qwen sees SIF, curve doesn't)")
    print(f"  Neither SIF:      {neither:>5}")

    # Breakdown by mechanism
    print(f"\n{'=' * 70}")
    print("P(SIF) BY MECHANISM (calibrated)")
    print(f"{'=' * 70}")
    mech_counts = Counter(r["mapped"] for r in valid)
    for mech, n in mech_counts.most_common():
        subset = [r for r in valid if r["mapped"] == mech]
        n_sif = sum(1 for r in subset if r["cal_p_sif"] >= 0.50)
        avg_psif = sum(r["cal_p_sif"] for r in subset) / len(subset)
        bands = Counter(r["band"] for r in subset)
        top_band = bands.most_common(1)[0]
        print(f"  {mech:22s}  n={n:>4}  SIF={n_sif:>4} ({100*n_sif/n:>5.1f}%)  avg={avg_psif:.3f}  top_band={top_band[0]}({top_band[1]})")


if __name__ == "__main__":
    main()
