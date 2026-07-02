#!/usr/bin/env python3
"""Approach J: Weighted score — average provision significance per law.

Score = (3*HIGH + 2*MED + 1*LOW) / total. Threshold: >=2.5 -> HIGH, >=1.75 -> MEDIUM, else LOW.
Uses provision-level results from Approach E (gravity+scope gate).

Usage:
    /usr/bin/python3 scripts/significance_approach_j.py
"""

import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
APPROACH = "j_weighted_score"
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

    results = []
    for law_name, profile in laws.items():
        total = profile["total"]
        if total == 0:
            continue
        score = (3 * profile["HIGH"] + 2 * profile["MEDIUM"] + 1 * profile["LOW"]) / total
        if score >= 2.5:
            rating = "HIGH"
        elif score >= 1.75:
            rating = "MEDIUM"
        else:
            rating = "LOW"
        results.append((
            law_name, APPROACH, rating, score,
            profile["HIGH"], profile["MEDIUM"], profile["LOW"], total
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
            print(f"  {law:<30s} {r[2]:>7s}  (score:{r[3]:.2f}, H:{r[4]} M:{r[5]} L:{r[6]} total:{r[7]})")

    # Top 5 and bottom 5 by score
    ranked = sorted(results, key=lambda x: -x[3])
    print(f"\nTop 5 by weighted score:")
    for r in ranked[:5]:
        print(f"  {r[0]:<40s} score:{r[3]:.2f}  (H:{r[4]} M:{r[5]} L:{r[6]})")
    print(f"\nBottom 5 by weighted score:")
    for r in ranked[-5:]:
        print(f"  {r[0]:<40s} score:{r[3]:.2f}  (H:{r[4]} M:{r[5]} L:{r[6]})")

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
