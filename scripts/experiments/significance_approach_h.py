#!/usr/bin/env python3
"""Approach H: HIGH count — law significance = count of HIGH provisions, ranked.

Top 20% by HIGH count -> HIGH law, bottom 33% -> LOW, else MEDIUM.
Uses provision-level results from Approach E (gravity+scope gate).

Usage:
    /usr/bin/python3 scripts/significance_approach_h.py
"""

import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
APPROACH = "h_high_count"
PROVISION_APPROACH = "e_gravity_scope_gate"


def main():
    conn = psycopg2.connect(PG_DSN)
    cur = conn.cursor()

    cur.execute("""
        SELECT split_part(e.section_id, ':', 1) as law_name,
               e.overall_rating,
               count(*) as cnt
        FROM significance_overall_experiments e
        WHERE e.approach = %s
        GROUP BY split_part(e.section_id, ':', 1), e.overall_rating
        ORDER BY 1, 2
    """, (PROVISION_APPROACH,))

    laws = {}
    for law_name, rating, cnt in cur.fetchall():
        if law_name not in laws:
            laws[law_name] = {"HIGH": 0, "MEDIUM": 0, "LOW": 0, "total": 0}
        laws[law_name][rating] += cnt
        laws[law_name]["total"] += cnt

    # Rank by HIGH count
    ranked = sorted(laws.items(), key=lambda x: x[1]["HIGH"], reverse=True)
    n = len(ranked)
    top_20 = int(n * 0.20)
    bottom_33 = int(n * 0.67)  # top 67% cutoff

    results = []
    for i, (law_name, profile) in enumerate(ranked):
        if i < top_20:
            rating = "HIGH"
        elif i >= bottom_33:
            rating = "LOW"
        else:
            rating = "MEDIUM"
        score = float(profile["HIGH"])
        results.append((
            law_name, APPROACH, rating, score,
            profile["HIGH"], profile["MEDIUM"], profile["LOW"], profile["total"]
        ))

    dist = {"HIGH": 0, "MEDIUM": 0, "LOW": 0}
    for r in results:
        dist[r[2]] += 1
    total = len(results)
    print(f"Laws rated: {total}")
    print(f"\nDistribution:")
    for level in ("HIGH", "MEDIUM", "LOW"):
        pct = dist[level] / total * 100 if total else 0
        print(f"  {level:6s}: {dist[level]:>4} ({pct:5.1f}%)")

    print(f"\nBenchmark laws:")
    bench = {r[0]: r for r in results}
    for law in ["UK_ukpga_1974_37", "UK_uksi_2015_51", "UK_uksi_1999_3242"]:
        if law in bench:
            r = bench[law]
            print(f"  {law:<30s} {r[2]:>7s}  (H:{r[4]} M:{r[5]} L:{r[6]} total:{r[7]})")

    # Top 5 and bottom 5
    print(f"\nTop 5 by HIGH count:")
    for law_name, profile in ranked[:5]:
        print(f"  {law_name:<40s} HIGH:{profile['HIGH']:>3} MED:{profile['MEDIUM']:>3} LOW:{profile['LOW']:>3}")
    print(f"\nBottom 5 by HIGH count:")
    for law_name, profile in ranked[-5:]:
        print(f"  {law_name:<40s} HIGH:{profile['HIGH']:>3} MED:{profile['MEDIUM']:>3} LOW:{profile['LOW']:>3}")

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
