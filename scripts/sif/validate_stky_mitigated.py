#!/usr/bin/env python3
"""Validate that mitigations reduce P(SIF) for STKY hazards.

For each STKY hazard:
1. Compute unmitigated P(SIF) from calibration curve
2. Apply relevant mitigations (Swiss cheese chain)
3. Check residual P(SIF) drops — the mitigation should make a difference

Uses Monte Carlo (SIPmath style) to properly compose gated effectiveness.

Usage:
    /usr/bin/python3 scripts/sif/validate_stky_mitigated.py
"""

import json
import math
import random
from pathlib import Path

CAL_DIR = Path("data/sif/calibration")
MIT_DIR = Path("data/sif/calibration/mitigations")

N_TRIALS = 10000
random.seed(42)

MECHANISM_TO_FILE = {
    "fall": "falls_gravity.json",
    "electrical": "electrical.json",
    "struck": "motion_struck.json",
    "transport": "motion_transport.json",
    "caught_in": "caught_in.json",
    "fire": "fire.json",
    "thermal": "thermal.json",
    "structural_collapse": "structural_collapse.json",
    "breathing": "breathing.json",
    "chemical": "chemical.json",
    "explosion": "explosion.json",
}

MECHANISM_TO_MIT_FILE = {
    "fall": "fall_protection.json",
    "electrical": "electrical_protection.json",
    "struck": "struck_protection.json",
    "transport": "transport_protection.json",
    "caught_in": "caught_in_protection.json",
    "fire": "fire_protection.json",
    "thermal": "thermal_protection.json",
    "structural_collapse": "structural_collapse_protection.json",
    "breathing": "confined_space.json",
    "chemical": "chemical_protection.json",
    "explosion": "explosion_protection.json",
}


# --- Metalog ---

def logit(y):
    return math.log(y / (1.0 - y))

def bounded_transform(x, lb=0.0, ub=1.0):
    return math.log((x - lb) / (ub - x))

def bounded_inverse(z, lb=0.0, ub=1.0):
    e = math.exp(z)
    return (lb + ub * e) / (1.0 + e)

def spt_fit(p10, p50, p90):
    l90 = logit(0.9)
    return p50, (p90 - p10) / (2 * l90), (p90 + p10 - 2.0 * p50) / (2.0 * 0.4 * l90)

def metalog_q(y, a1, a2, a3):
    l = logit(y)
    return a1 + a2 * l + a3 * (y - 0.5) * l


def generate_sip(p10, p50, p90, lb=0.0, ub=1.0, n=N_TRIALS):
    """Generate N Monte Carlo trials from a bounded metalog."""
    if abs(p10 - p90) < 1e-6:
        return [p50] * n

    z10 = bounded_transform(p10, lb, ub)
    z50 = bounded_transform(p50, lb, ub)
    z90 = bounded_transform(p90, lb, ub)
    a1, a2, a3 = spt_fit(z10, z50, z90)

    trials = []
    for _ in range(n):
        u = random.random()
        u = max(0.001, min(0.999, u))
        z = metalog_q(u, a1, a2, a3)
        x = bounded_inverse(z, lb, ub)
        trials.append(x)
    return trials


def gated_effectiveness(p_active, eff_p10, eff_p50, eff_p90, n=N_TRIALS):
    """Generate gated effectiveness SIP — Bernoulli gate × effectiveness metalog."""
    eff_sip = generate_sip(eff_p10, eff_p50, eff_p90, lb=0.0, ub=1.0, n=n)
    result = []
    for i in range(n):
        if random.random() < p_active:
            result.append(eff_sip[i])
        else:
            result.append(0.0)  # barrier failed — no effect
    return result


def chain_mitigations(severity_sip, mitigation_sips):
    """Swiss cheese chain: residual[i] = severity[i] × ∏(1 - eff_j[i])"""
    n = len(severity_sip)
    residual = list(severity_sip)
    for eff in mitigation_sips:
        for i in range(n):
            residual[i] *= (1.0 - eff[i])
    return residual


def exceedance(trials, threshold):
    """P(X >= threshold)"""
    return sum(1 for x in trials if x >= threshold) / len(trials)


def load_cal_band(mechanism, band_name):
    fname = MECHANISM_TO_FILE.get(mechanism)
    if not fname:
        return None
    with open(CAL_DIR / fname) as f:
        cal = json.load(f)
    for b in cal["magnitude_bands"]:
        if b["band"] == band_name:
            return b
    return None


def load_mitigations(mechanism):
    fname = MECHANISM_TO_MIT_FILE.get(mechanism)
    if not fname:
        return []
    fpath = MIT_DIR / fname
    if not fpath.exists():
        return []
    with open(fpath) as f:
        data = json.load(f)
    return data.get("mitigations", [])


# --- STKY scenarios with specific mitigations to apply ---

STKY_SCENARIOS = [
    {
        "name": "6m fall with harness",
        "mechanism": "fall",
        "band": "medium",
        "mitigations": ["harness_lanyard"],
        "expect": "SIF → ELEVATED or NON_SIF",
    },
    {
        "name": "6m fall with guardrail + harness",
        "mechanism": "fall",
        "band": "medium",
        "mitigations": ["guardrail", "harness_lanyard"],
        "expect": "SIF → NON_SIF (layered defence)",
    },
    {
        "name": "480V contact with LOTO",
        "mechanism": "electrical",
        "band": "medium",
        "mitigations": ["loto"],
        "expect": "SIF → NON_SIF",
    },
    {
        "name": "480V contact with insulated gloves + GFCI",
        "mechanism": "electrical",
        "band": "medium",
        "mitigations": ["insulated_tools_gloves", "gfci_rcd"],
        "expect": "SIF → ELEVATED or NON_SIF",
    },
    {
        "name": "Forklift strike with segregation",
        "mechanism": "transport",
        "band": "yard_speed",
        "mitigations": ["segregation_design"],
        "expect": "SIF → NON_SIF",
    },
    {
        "name": "Dropped scaffold tube with hard hat",
        "mechanism": "struck",
        "band": "heavy_dropped",
        "mitigations": ["hard_hat"],
        "expect": "SIF → ELEVATED (hard hat reduces but doesn't eliminate)",
    },
    {
        "name": "Dropped scaffold tube with toe boards + hard hat",
        "mechanism": "struck",
        "band": "heavy_dropped",
        "mitigations": ["toe_board_netting", "hard_hat"],
        "expect": "SIF → NON_SIF (layered)",
    },
    {
        "name": "Lathe entanglement with interlocked guard",
        "mechanism": "caught_in",
        "band": "medium_machinery",
        "mitigations": ["interlocked_guard"],
        "expect": "SIF → NON_SIF",
    },
    {
        "name": "Flash fire with FR clothing",
        "mechanism": "fire",
        "band": "clothing_ignition",
        "mitigations": ["fr_clothing"],
        "expect": "SIF → ELEVATED (FR prevents escalation)",
    },
    {
        "name": "Structural fire with sprinklers + escape",
        "mechanism": "fire",
        "band": "structural_fire",
        "mitigations": ["sprinkler", "escape_route"],
        "expect": "SIF → ELEVATED or NON_SIF",
    },
    {
        "name": "Confined space with SCBA + rescue team",
        "mechanism": "breathing",
        "band": "oxygen_deficient",
        "mitigations": ["scba", "standby_rescue_team"],
        "expect": "SIF → ELEVATED or NON_SIF",
    },
    {
        "name": "Trench collapse with shoring",
        "mechanism": "structural_collapse",
        "band": "trench_excavation",
        "mitigations": ["trench_box"],
        "expect": "SIF → NON_SIF",
    },
    {
        "name": "Dust explosion with suppression + blast design",
        "mechanism": "explosion",
        "band": "significant_industrial",
        "mitigations": ["explosion_suppression", "blast_resistant_design"],
        "expect": "SIF → ELEVATED or NON_SIF",
    },
]


def main():
    print("STKY HAZARD MITIGATION VALIDATION")
    print("=" * 100)
    print(f"{'Scenario':45s}  {'Unmit':>8}  {'Mitigated':>9}  {'Reduction':>10}  {'Result'}")
    print("-" * 100)

    all_pass = True

    for scenario in STKY_SCENARIOS:
        mech = scenario["mechanism"]
        band_name = scenario["band"]
        mit_names = scenario["mitigations"]

        # Load calibration band
        band = load_cal_band(mech, band_name)
        if not band:
            print(f"  SKIP: {scenario['name']} — band {band_name} not found")
            continue

        # Generate unmitigated severity SIP
        severity_sip = generate_sip(band["p10"], band["p50"], band["p90"], lb=0.0, ub=1.0)
        unmit_psif = exceedance(severity_sip, 0.10)

        # Load and apply mitigations
        all_mits = load_mitigations(mech)
        mit_sips = []
        mit_labels = []
        for mit_name in mit_names:
            mit = next((m for m in all_mits if m["name"] == mit_name), None)
            if not mit:
                # Try generic controls
                generic_path = MIT_DIR / "generic_controls.json"
                if generic_path.exists():
                    with open(generic_path) as f:
                        generic = json.load(f)
                    mit = next((m for m in generic["mitigations"] if m["name"] == mit_name), None)
            if mit:
                eff_sip = gated_effectiveness(
                    mit["p_active"],
                    mit["effectiveness_p10"],
                    mit["effectiveness_p50"],
                    mit["effectiveness_p90"],
                )
                mit_sips.append(eff_sip)
                mit_labels.append(mit["display"])

        # Apply Swiss cheese chain
        if mit_sips:
            residual = chain_mitigations(severity_sip, mit_sips)
        else:
            residual = severity_sip

        mit_psif = exceedance(residual, 0.10)
        reduction = unmit_psif - mit_psif
        pct_reduction = 100 * reduction / unmit_psif if unmit_psif > 0 else 0

        # Did it reduce?
        reduced = mit_psif < unmit_psif
        label_unmit = "SIF" if unmit_psif >= 0.50 else "ELEV" if unmit_psif >= 0.10 else "NON"
        label_mit = "SIF" if mit_psif >= 0.50 else "ELEV" if mit_psif >= 0.10 else "NON"
        result = f"{label_unmit}→{label_mit}"

        passed = reduced
        symbol = "✓" if passed else "✗"
        if not passed:
            all_pass = False

        print(f"  {symbol} {scenario['name']:43s}  {unmit_psif:>7.3f}  {mit_psif:>9.3f}  {pct_reduction:>8.0f}%  {result}")

    print("-" * 100)
    if all_pass:
        print("✓ ALL SCENARIOS: mitigations reduce P(SIF)")
    else:
        print("✗ SOME SCENARIOS FAILED — mitigations did not reduce P(SIF)")


if __name__ == "__main__":
    main()
