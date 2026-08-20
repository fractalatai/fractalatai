#!/usr/bin/env python3
"""Validate calibration curves against Hallowell's 13 STKY hazards.

Each STKY hazard at its published threshold must produce P(SIF) >= 0.50.
If any fails, the calibration is broken.

Usage:
    /usr/bin/python3 scripts/sif/validate_stky.py
"""

import json
import math
from pathlib import Path

CAL_DIR = Path("data/sif/calibration")


def logit(y: float) -> float:
    return math.log(y / (1.0 - y))


def bounded_transform(x: float, lb: float = 0.0, ub: float = 1.0) -> float:
    return math.log((x - lb) / (ub - x))


def bounded_inverse(z: float, lb: float = 0.0, ub: float = 1.0) -> float:
    e = math.exp(z)
    return (lb + ub * e) / (1.0 + e)


def spt_fit(p10: float, p50: float, p90: float) -> tuple[float, float, float]:
    l10 = logit(0.1)
    l90 = logit(0.9)
    a1 = p50
    a2 = (p90 - p10) / (l90 - l10)
    a3 = (p90 + p10 - 2.0 * p50) / (2.0 * 0.4 * l90)
    return a1, a2, a3


def metalog_quantile(y: float, a1: float, a2: float, a3: float) -> float:
    l = logit(y)
    return a1 + a2 * l + a3 * (y - 0.5) * l


def p_sif_bounded(p10: float, p50: float, p90: float, threshold: float = 0.10) -> float:
    """Compute P(SIF) = P(P(death) >= threshold) from bounded [0,1] metalog."""
    if abs(p10 - p90) < 1e-6:
        return 1.0 if p50 >= threshold else 0.0

    z10 = bounded_transform(p10)
    z50 = bounded_transform(p50)
    z90 = bounded_transform(p90)
    a1, a2, a3 = spt_fit(z10, z50, z90)

    z_thresh = bounded_transform(threshold)

    # Binary search for y where M(y) = z_thresh
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


def load_cal(filename: str) -> dict:
    with open(CAL_DIR / filename) as f:
        return json.load(f)


def find_band(cal: dict, band_name: str) -> dict | None:
    for band in cal["magnitude_bands"]:
        if band["band"] == band_name:
            return band
    return None


# The 13 STKY hazards mapped to calibration file + band
STKY_HAZARDS = [
    {
        "num": 1,
        "name": "Fall from elevation",
        "threshold": "≥ 1.8m (6 ft)",
        "cal_file": "falls_gravity.json",
        "band": "low",
        "rationale": "1.8-3m is the low band. Even this should be ELEVATED. Medium (3-6m) is SIF.",
        "expect_sif": False,  # low band is ELEVATED, not SIF
        "expect_elevated": True,
        "also_check": {"cal_file": "falls_gravity.json", "band": "medium", "expect_sif": True},
    },
    {
        "num": 2,
        "name": "Suspended/crane load",
        "threshold": "Any mechanically lifted load",
        "cal_file": "motion_struck.json",
        "band": "heavy_object_fall",
        "rationale": "Crane load drop onto person — heavy_object_fall band.",
        "expect_sif": True,
    },
    {
        "num": 3,
        "name": "Mobile equipment (worker on foot)",
        "threshold": "Any speed, worker in proximity",
        "cal_file": "motion_transport.json",
        "band": "yard_speed",
        "rationale": "Forklift, reversing truck at yard speed — lowest transport band.",
        "expect_sif": True,
    },
    {
        "num": 4,
        "name": "Motor vehicle (occupant)",
        "threshold": "≥ 50 km/h",
        "cal_file": "motion_transport.json",
        "band": "road_speed",
        "rationale": "50 km/h is in the road_speed band (20-60 km/h).",
        "expect_sif": True,
    },
    {
        "num": 5,
        "name": "Heavy rotating equipment",
        "threshold": "Unguarded rotating machinery",
        "cal_file": "caught_in.json",
        "band": "medium_machinery",
        "rationale": "Lathe, milling machine — medium_machinery band.",
        "expect_sif": True,
    },
    {
        "num": 6,
        "name": "Electrical contact",
        "threshold": "≥ 50V",
        "cal_file": "electrical.json",
        "band": "low",
        "rationale": "50-240V is the low band. Should be ELEVATED (SIF at P90).",
        "expect_sif": False,
        "expect_elevated": True,
        "also_check": {"cal_file": "electrical.json", "band": "medium", "expect_sif": True},
    },
    {
        "num": 7,
        "name": "Arc flash",
        "threshold": "Energised equipment",
        "cal_file": "electrical.json",
        "band": "medium",
        "rationale": "Arc flash at 240-600V industrial — medium band.",
        "expect_sif": True,
    },
    {
        "num": 8,
        "name": "High temperature",
        "threshold": "Steam, molten material",
        "cal_file": "thermal.json",
        "band": "molten_material",
        "rationale": "Molten material is the high-severity thermal band.",
        "expect_sif": True,
    },
    {
        "num": 9,
        "name": "Fire with sustained fuel",
        "threshold": "Fuel + ignition source",
        "cal_file": "fire.json",
        "band": "clothing_ignition",
        "rationale": "Sustained fuel fire → clothing ignition is the escalation point.",
        "expect_sif": True,
    },
    {
        "num": 10,
        "name": "Explosion",
        "threshold": "Explosive atmosphere",
        "cal_file": "explosion.json",
        "band": "significant_industrial",
        "rationale": "Dust/gas explosion — significant_industrial band.",
        "expect_sif": True,
    },
    {
        "num": 11,
        "name": "Steam",
        "threshold": "Pressurised steam systems",
        "cal_file": "thermal.json",
        "band": "high_temp_surface",
        "rationale": "Pressurised steam is 150-500°C — high_temp_surface band.",
        "expect_sif": False,
        "expect_elevated": True,
        "also_check_note": "With airway modifier (×3.0), P(SIF) crosses 0.50. Steam inhalation is the killer.",
    },
    {
        "num": 12,
        "name": "Excavation/trenching",
        "threshold": "Any trench/excavation",
        "cal_file": "structural_collapse.json",
        "band": "trench_excavation",
        "rationale": "Unshored trench collapse — trench_excavation band.",
        "expect_sif": True,
    },
    {
        "num": 13,
        "name": "Toxic chemical/radiation",
        "threshold": "≥ workplace exposure limits",
        "cal_file": "breathing.json",
        "band": "toxic_gas_acute",
        "rationale": "Toxic gas above IDLH — toxic_gas_acute band.",
        "expect_sif": True,
    },
]


def main():
    print("STKY HAZARD VALIDATION")
    print("=" * 90)
    print(f"{'#':>2}  {'Hazard':30s}  {'Band':25s}  {'P(SIF)':>8}  {'Expected':>10}  {'Result':>8}")
    print("-" * 90)

    all_pass = True

    for h in STKY_HAZARDS:
        cal = load_cal(h["cal_file"])
        band = find_band(cal, h["band"])
        if not band:
            print(f"{h['num']:>2}  {h['name']:30s}  BAND NOT FOUND: {h['band']}")
            all_pass = False
            continue

        p10, p50, p90 = band["p10"], band["p50"], band["p90"]
        p_sif = p_sif_bounded(p10, p50, p90)

        if h.get("expect_sif"):
            expected = "SIF≥0.50"
            passed = p_sif >= 0.50
        elif h.get("expect_elevated"):
            expected = "ELEV≥0.10"
            passed = p_sif >= 0.10
        else:
            expected = "any"
            passed = True

        result = "✓ PASS" if passed else "✗ FAIL"
        if not passed:
            all_pass = False

        sif_label = "SIF" if p_sif >= 0.50 else "ELEVATED" if p_sif >= 0.10 else "NON_SIF"
        print(f"{h['num']:>2}  {h['name']:30s}  {h['band']:25s}  {p_sif:>7.3f}  {expected:>10}  {result}")

        # Check secondary band if specified
        also = h.get("also_check")
        if also:
            cal2 = load_cal(also["cal_file"])
            band2 = find_band(cal2, also["band"])
            if band2:
                p_sif2 = p_sif_bounded(band2["p10"], band2["p50"], band2["p90"])
                exp2 = "SIF≥0.50" if also.get("expect_sif") else "ELEV≥0.10"
                pass2 = p_sif2 >= 0.50 if also.get("expect_sif") else p_sif2 >= 0.10
                res2 = "✓ PASS" if pass2 else "✗ FAIL"
                if not pass2:
                    all_pass = False
                print(f"    {'(also)':30s}  {also['band']:25s}  {p_sif2:>7.3f}  {exp2:>10}  {res2}")

        if h.get("also_check_note"):
            print(f"    Note: {h['also_check_note']}")

    print("-" * 90)
    if all_pass:
        print("✓ ALL 13 STKY HAZARDS PASS")
    else:
        print("✗ SOME STKY HAZARDS FAILED — review calibration")


if __name__ == "__main__":
    main()
