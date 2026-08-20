#!/usr/bin/env python3
"""Validate calibration curve data and check metalog feasibility.

Reads calibration JSON files and verifies:
1. P10 <= P50 <= P90 (monotonicity)
2. Values are on the P(death) scale [0, 1]
3. SPT metalog coefficients can be computed
4. The fitted metalog passes the SIF threshold check

Usage:
    /usr/bin/python3 scripts/sif/validate_calibration.py data/sif/calibration/falls_gravity.json
"""

import json
import math
import sys


def logit(y: float) -> float:
    """ln(y / (1-y))"""
    return math.log(y / (1.0 - y))


def spt_fit(p10: float, p50: float, p90: float) -> tuple[float, float, float]:
    """Fit a 3-term SPT metalog from P10/P50/P90 quantiles (unbounded).

    Returns (a1, a2, a3) coefficients.
    """
    l10 = logit(0.1)  # -2.197
    l90 = logit(0.9)  #  2.197 (= -l10)

    a1 = p50
    a2 = (p90 - p10) / (l90 - l10)
    a3 = (p90 + p10 - 2.0 * p50) / (2.0 * 0.4 * l90)

    return a1, a2, a3


def bounded_transform(x: float, lb: float = 0.0, ub: float = 1.0) -> float:
    """Transform bounded value to unbounded space: ln((x-lb)/(ub-x))"""
    return math.log((x - lb) / (ub - x))


def bounded_inverse(z: float, lb: float = 0.0, ub: float = 1.0) -> float:
    """Inverse transform: (lb + ub*exp(z)) / (1 + exp(z))"""
    e = math.exp(z)
    return (lb + ub * e) / (1.0 + e)


def spt_fit_bounded(p10: float, p50: float, p90: float, lb: float = 0.0, ub: float = 1.0) -> tuple[float, float, float]:
    """Fit a 3-term SPT metalog on bounded [lb, ub] values.

    Transforms to unbounded space via ln((x-lb)/(ub-x)), fits SPT, returns coefficients in transformed space.
    """
    z10 = bounded_transform(p10, lb, ub)
    z50 = bounded_transform(p50, lb, ub)
    z90 = bounded_transform(p90, lb, ub)
    return spt_fit(z10, z50, z90)


def metalog_quantile(y: float, a1: float, a2: float, a3: float) -> float:
    """Evaluate 3-term metalog quantile function at probability y."""
    l = logit(y)
    return a1 + a2 * l + a3 * (y - 0.5) * l


def check_feasibility(a1: float, a2: float, a3: float, n_points: int = 100) -> bool:
    """Check that the quantile function is monotonically increasing.

    For a 3-term metalog, the derivative is:
      dM/dy = a2 / (y(1-y)) + a3 * ((y-0.5)/(y(1-y)) + logit(y))
    This must be > 0 for all y in (0,1).
    """
    for i in range(1, n_points):
        y = i / n_points
        inv = 1.0 / (y * (1.0 - y))
        l = logit(y)
        d = y - 0.5
        deriv = a2 * inv + a3 * (d * inv + l)
        if deriv <= 0:
            return False
    return True


def sif_probability(a1: float, a2: float, a3: float, threshold: float = 0.10) -> float:
    """Estimate P(severity >= threshold) from the metalog.

    Binary search for the CDF value at the threshold.
    P(SIF) = 1 - CDF(threshold) = 1 - y where M(y) = threshold.
    """
    # If the entire distribution is below threshold, P(SIF) = 0
    if metalog_quantile(0.999, a1, a2, a3) < threshold:
        return 0.0
    # If the entire distribution is above threshold, P(SIF) = 1
    if metalog_quantile(0.001, a1, a2, a3) >= threshold:
        return 1.0

    # Binary search for y where M(y) = threshold
    lo, hi = 0.001, 0.999
    for _ in range(50):
        mid = (lo + hi) / 2.0
        if metalog_quantile(mid, a1, a2, a3) < threshold:
            lo = mid
        else:
            hi = mid

    return 1.0 - (lo + hi) / 2.0


def main():
    if len(sys.argv) < 2:
        print("Usage: validate_calibration.py <calibration_file.json>")
        sys.exit(1)

    with open(sys.argv[1]) as f:
        cal = json.load(f)

    print(f"Energy type: {cal['energy_type']}")
    print(f"Mechanism: {cal['mechanism']}")
    p_death_scale = cal.get('p_death_scale', {})
    print(f"SIF threshold: P(death) >= {p_death_scale.get('serious_injury', 0.10)}")
    print()

    sif_threshold = 0.10  # P(death) at which SIF begins
    bounds = cal.get('metalog_bounds', [0.0, 1.0])
    lb, ub = bounds[0], bounds[1]
    all_ok = True

    for band in cal['magnitude_bands']:
        p10, p50, p90 = band['p10'], band['p50'], band['p90']
        name = band['band']

        # Build label from available fields
        magnitude_key = [k for k in band if k.endswith('_low')][0] if any(k.endswith('_low') for k in band) else None
        if magnitude_key:
            unit = magnitude_key.rsplit('_', 2)[0].split('_', 1)[-1] if '_' in magnitude_key else ''
            lo = band[magnitude_key]
            hi_key = magnitude_key.replace('_low', '_high')
            hi = band.get(hi_key)
            hi_str = str(hi) if hi is not None else '∞'
            print(f"--- {name} ({lo}–{hi_str}) ---")
        else:
            print(f"--- {name} ---")

        print(f"  P10={p10}  P50={p50}  P90={p90}")

        # Check monotonicity
        if not (p10 <= p50 <= p90):
            print(f"  ❌ FAIL: P10 <= P50 <= P90 violated")
            all_ok = False
            continue

        # Check range
        if not (lb <= p10 <= ub and lb <= p50 <= ub and lb <= p90 <= ub):
            print(f"  ❌ FAIL: values outside [{lb}, {ub}]")
            all_ok = False
            continue

        # Handle near-degenerate cases (all same value)
        if abs(p10 - p90) < 1e-6:
            print(f"  Point mass at {p50} — no metalog needed")
            p_sif = 1.0 if p50 >= sif_threshold else 0.0
            print(f"  P(SIF) = {p_sif:.2f}")
            print()
            continue

        # Fit bounded metalog
        a1, a2, a3 = spt_fit_bounded(p10, p50, p90, lb, ub)
        print(f"  Bounded SPT coeffs: a1={a1:.4f}, a2={a2:.4f}, a3={a3:.4f}")

        # Check feasibility in transformed space
        feasible = check_feasibility(a1, a2, a3)
        if feasible:
            print(f"  ✓ Feasible (bounded [{lb}, {ub}])")
        else:
            print(f"  ❌ Infeasible even with bounded metalog")
            all_ok = False

        # Verify roundtrip through bounded transform
        q10_z = metalog_quantile(0.1, a1, a2, a3)
        q50_z = metalog_quantile(0.5, a1, a2, a3)
        q90_z = metalog_quantile(0.9, a1, a2, a3)
        q10 = bounded_inverse(q10_z, lb, ub)
        q50 = bounded_inverse(q50_z, lb, ub)
        q90 = bounded_inverse(q90_z, lb, ub)
        rt_ok = (abs(q10 - p10) < 1e-6 and abs(q50 - p50) < 1e-6 and abs(q90 - p90) < 1e-6)
        if rt_ok:
            print(f"  ✓ Roundtrip OK")
        else:
            print(f"  ❌ Roundtrip FAIL: q10={q10:.6f}, q50={q50:.6f}, q90={q90:.6f}")
            all_ok = False

        # P(SIF): find y where bounded_inverse(M(y)) = sif_threshold
        z_thresh = bounded_transform(sif_threshold, lb, ub)
        p_sif = sif_probability(a1, a2, a3, z_thresh)
        sif_flag = "SIF" if p_sif >= 0.50 else "ELEVATED" if p_sif >= 0.10 else "NON_SIF"
        print(f"  P(SIF) = {p_sif:.3f} → {sif_flag}")

        # Published mortality cross-check
        if 'published_mortality' in band:
            pm = band['published_mortality']
            z_fatal = bounded_transform(0.99, lb, ub)  # approximate P(death ≈ 1.0)
            p_fatal = sif_probability(a1, a2, a3, z_fatal)
            print(f"  Published mortality: {pm:.3f}, Metalog P(fatal): {p_fatal:.3f}")

        print()

    if all_ok:
        print("✓ All bands pass basic validation")
    else:
        print("❌ Some bands failed — review above")


if __name__ == "__main__":
    main()
