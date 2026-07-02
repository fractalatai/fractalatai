#!/usr/bin/env python3
"""Approach G: Max provision — law significance = highest provision significance.

Uses provision-level results from Approach E (gravity+scope gate).

Usage:
    /usr/bin/python3 scripts/significance_approach_g.py
"""

import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
APPROACH = "g_max_provision"
PROVISION_APPROACH = "e_gravity_scope_gate"

RANK = {"HIGH": 3, "MEDIUM": 2, "LOW": 1}
REVERSE = {3: "HIGH", 2: "MEDIUM", 1: "LOW"}


def main():
    conn = psycopg2.connect(PG_DSN)
    cur = conn.cursor()

    # Get provision-level results grouped by law
    cur.execute("""
        SELECT split_part(e.section_id, ':', 1) as law_name,
               e.overall_rating,
               count(*) as cnt
        FROM significance_overall_experiments e
        WHERE e.approach = %s
        GROUP BY split_part(e.section_id, ':', 1), e.overall_rating
        ORDER BY 1, 2
    """, (PROVISION_APPROACH,))

    # Build per-law profiles
    laws = {}
    for law_name, rating, cnt in cur.fetchall():
        if law_name not in laws:
            laws[law_name] = {"HIGH": 0, "MEDIUM": 0, "LOW": 0, "total": 0}
        laws[law_name][rating] += cnt
        laws[law_name]["total"] += cnt

    results = []
    for law_name, profile in laws.items():
        max_rating = "LOW"
        for r in ("HIGH", "MEDIUM", "LOW"):
            if profile[r] > 0:
                max_rating = r
                break
        results.append((
            law_name, APPROACH, max_rating, float(RANK[max_rating]),
            profile["HIGH"], profile["MEDIUM"], profile["LOW"], profile["total"]
        ))

    # Distribution
    dist = {"HIGH": 0, "MEDIUM": 0, "LOW": 0}
    for r in results:
        dist[r[2]] += 1
    total = len(results)
    print(f"Laws rated: {total}")
    print(f"\nDistribution:")
    for level in ("HIGH", "MEDIUM", "LOW"):
        pct = dist[level] / total * 100 if total else 0
        print(f"  {level:6s}: {dist[level]:>4} ({pct:5.1f}%)")

    # Benchmark laws
    print(f"\nBenchmark laws:")
    bench = {r[0]: r for r in results}
    for law in ["UK_ukpga_1974_37", "UK_uksi_2015_51", "UK_uksi_1999_3242"]:
        if law in bench:
            r = bench[law]
            print(f"  {law:<30s} {r[2]:>7s}  (H:{r[4]} M:{r[5]} L:{r[6]} total:{r[7]})")

    # Persist
    cur.execute("DELETE FROM significance_law_experiments WHERE approach = %s", (APPROACH,))
    cur.executemany("""
        INSERT INTO significance_law_experiments
        (law_name, approach, overall_rating, score, high_count, medium_count, low_count, total_obligations)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
    """, results)
    conn.commit()
    print(f"\nPersisted {len(results)} law results")
    cur.close()
    conn.close()


if __name__ == "__main__":
    main()
