#!/usr/bin/env python3
"""Approach A: Equal-weight sum across 5 significance dimensions.

Score: HIGH=3, MEDIUM=2, LOW=1 per dimension, average all 5.
Thresholds: >=2.5 -> HIGH, >=1.75 -> MEDIUM, else LOW.

Usage:
    /usr/bin/python3 scripts/significance_approach_a.py
    /usr/bin/python3 scripts/significance_approach_a.py --dry-run
"""

import argparse
import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
APPROACH = "a_equal_weight"

BENCHMARKS = [
    "UK_ukpga_1974_37:s.2(1)",      # HSWA general duty -> expect HIGH
    "UK_ukpga_1974_37:s.9",          # HSWA safety policy -> expect MEDIUM
    "UK_uksi_1999_3242:reg.3(1)",    # MHSW risk assessment -> expect HIGH
    "UK_uksi_2015_51:reg.4(1)",      # CDM client duties -> expect HIGH
]

SCORE_MAP = {"HIGH": 3, "MEDIUM": 2, "LOW": 1}


def score_to_rating(score):
    if score >= 2.5:
        return "HIGH"
    elif score >= 1.75:
        return "MEDIUM"
    else:
        return "LOW"


def compute(row):
    """Compute overall score from 5 dimension values."""
    sdb, spc, grav, strength, hier = row
    vals = [SCORE_MAP.get(v, 0) for v in (sdb, spc, grav, strength, hier)]
    if 0 in vals:
        return None, None  # missing dimension
    avg = sum(vals) / len(vals)
    return avg, score_to_rating(avg)


def main():
    parser = argparse.ArgumentParser(description="Approach A: Equal-weight significance")
    parser.add_argument("--dry-run", action="store_true", help="Print results without writing")
    args = parser.parse_args()

    conn = psycopg2.connect(PG_DSN)
    cur = conn.cursor()

    # Fetch all rated provisions
    cur.execute("""
        SELECT section_id,
               significance_scope_duty_bearer,
               significance_scope_protected_class,
               significance_gravity,
               significance_strength,
               significance_hierarchy
        FROM legislation_text
        WHERE significance_gravity IS NOT NULL
          AND significance_hierarchy IS NOT NULL
    """)
    rows = cur.fetchall()
    print(f"Rated provisions: {len(rows):,}")

    results = []
    for section_id, *dims in rows:
        score, rating = compute(dims)
        if rating:
            results.append((section_id, APPROACH, rating, score))

    print(f"Scored provisions: {len(results):,}")

    # Distribution
    dist = {"HIGH": 0, "MEDIUM": 0, "LOW": 0}
    for _, _, rating, _ in results:
        dist[rating] += 1
    total = len(results)
    print(f"\nCorpus distribution:")
    for level in ("HIGH", "MEDIUM", "LOW"):
        pct = dist[level] / total * 100 if total else 0
        print(f"  {level:6s}: {dist[level]:>6,} ({pct:5.1f}%)")

    # Benchmark provisions
    print(f"\nBenchmark provisions:")
    print(f"  {'section_id':<40s} {'score':>5s} {'overall':>7s}  dims (sdb/spc/grav/str/hier)")
    bench_lookup = {r[0]: r for r in results}
    for sid in BENCHMARKS:
        if sid in bench_lookup:
            _, _, rating, score = bench_lookup[sid]
            # Get raw dims
            cur.execute("""
                SELECT significance_scope_duty_bearer, significance_scope_protected_class,
                       significance_gravity, significance_strength, significance_hierarchy
                FROM legislation_text WHERE section_id = %s
            """, (sid,))
            dims = cur.fetchone()
            print(f"  {sid:<40s} {score:>5.2f} {rating:>7s}  {'/'.join(dims)}")
        else:
            print(f"  {sid:<40s}   n/a     n/a  (not rated or missing hierarchy)")

    if not args.dry_run:
        # Clear previous results for this approach
        cur.execute("DELETE FROM significance_overall_experiments WHERE approach = %s", (APPROACH,))
        # Insert
        cur.executemany(
            "INSERT INTO significance_overall_experiments (section_id, approach, overall_rating, score) VALUES (%s, %s, %s, %s)",
            results,
        )
        conn.commit()
        print(f"\nPersisted {len(results):,} results to significance_overall_experiments")
    else:
        print(f"\nDry run — nothing written")

    cur.close()
    conn.close()


if __name__ == "__main__":
    main()
