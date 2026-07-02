#!/usr/bin/env python3
"""Approach D: Max-of-dimensions.

Overall = max(all 5 dimensions). A provision is as significant as its most significant dimension.

Usage:
    /usr/bin/python3 scripts/significance_approach_d.py
"""

import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
APPROACH = "d_max"

BENCHMARKS = [
    "UK_ukpga_1974_37:s.2(1)",
    "UK_ukpga_1974_37:s.9",
    "UK_uksi_1999_3242:reg.3(1)",
    "UK_uksi_2015_51:reg.4(1)",
]

RANK = {"HIGH": 3, "MEDIUM": 2, "LOW": 1}
REVERSE = {3: "HIGH", 2: "MEDIUM", 1: "LOW"}


def compute(sdb, spc, grav, strength, hier):
    vals = [sdb, spc, grav, strength, hier]
    if not all(vals):
        return None, None
    max_val = max(RANK.get(v, 0) for v in vals)
    return float(max_val), REVERSE[max_val]


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
            print(f"  {sid:<40s} {score:>5.1f} {rating:>7s}  {'/'.join(dims)}")
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
