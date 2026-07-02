#!/usr/bin/env python3
"""Approach B: Gravity-dominant weighted sum.

Weights: gravity=0.35, scope_duty_bearer=0.20, scope_protected_class=0.20, strength=0.15, hierarchy=0.10
Score: HIGH=3, MEDIUM=2, LOW=1. Weighted sum thresholded into H/M/L.

Usage:
    /usr/bin/python3 scripts/significance_approach_b.py
"""

import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
APPROACH = "b_gravity_weighted"

BENCHMARKS = [
    "UK_ukpga_1974_37:s.2(1)",
    "UK_ukpga_1974_37:s.9",
    "UK_uksi_1999_3242:reg.3(1)",
    "UK_uksi_2015_51:reg.4(1)",
]

SCORE_MAP = {"HIGH": 3, "MEDIUM": 2, "LOW": 1}
WEIGHTS = {
    "gravity": 0.35,
    "scope_duty_bearer": 0.20,
    "scope_protected_class": 0.20,
    "strength": 0.15,
    "hierarchy": 0.10,
}


def score_to_rating(score):
    if score >= 2.5:
        return "HIGH"
    elif score >= 1.75:
        return "MEDIUM"
    else:
        return "LOW"


def compute(sdb, spc, grav, strength, hier):
    vals = {"scope_duty_bearer": sdb, "scope_protected_class": spc, "gravity": grav, "strength": strength, "hierarchy": hier}
    nums = {k: SCORE_MAP.get(v, 0) for k, v in vals.items()}
    if 0 in nums.values():
        return None, None
    weighted = sum(nums[k] * WEIGHTS[k] for k in WEIGHTS)
    return weighted, score_to_rating(weighted)


def main():
    conn = psycopg2.connect(PG_DSN)
    cur = conn.cursor()

    cur.execute("""
        SELECT section_id, significance_scope_duty_bearer, significance_scope_protected_class,
               significance_gravity, significance_strength, significance_hierarchy
        FROM legislation_text WHERE significance_gravity IS NOT NULL AND significance_hierarchy IS NOT NULL
    """)
    rows = cur.fetchall()
    print(f"Rated provisions: {len(rows):,}")

    results = []
    for section_id, sdb, spc, grav, strength, hier in rows:
        score, rating = compute(sdb, spc, grav, strength, hier)
        if rating:
            results.append((section_id, APPROACH, rating, score))

    dist = {"HIGH": 0, "MEDIUM": 0, "LOW": 0}
    for _, _, rating, _ in results:
        dist[rating] += 1
    total = len(results)
    print(f"\nCorpus distribution:")
    for level in ("HIGH", "MEDIUM", "LOW"):
        pct = dist[level] / total * 100 if total else 0
        print(f"  {level:6s}: {dist[level]:>6,} ({pct:5.1f}%)")

    print(f"\nBenchmark provisions:")
    print(f"  {'section_id':<40s} {'score':>5s} {'overall':>7s}  dims (sdb/spc/grav/str/hier)")
    bench_lookup = {r[0]: r for r in results}
    for sid in BENCHMARKS:
        if sid in bench_lookup:
            _, _, rating, score = bench_lookup[sid]
            cur.execute("""
                SELECT significance_scope_duty_bearer, significance_scope_protected_class,
                       significance_gravity, significance_strength, significance_hierarchy
                FROM legislation_text WHERE section_id = %s
            """, (sid,))
            dims = cur.fetchone()
            print(f"  {sid:<40s} {score:>5.2f} {rating:>7s}  {'/'.join(dims)}")
        else:
            print(f"  {sid:<40s}   n/a     n/a")

    cur.execute("DELETE FROM significance_overall_experiments WHERE approach = %s", (APPROACH,))
    cur.executemany(
        "INSERT INTO significance_overall_experiments (section_id, approach, overall_rating, score) VALUES (%s, %s, %s, %s)",
        results,
    )
    conn.commit()
    print(f"\nPersisted {len(results):,} results")
    cur.close()
    conn.close()


if __name__ == "__main__":
    main()
