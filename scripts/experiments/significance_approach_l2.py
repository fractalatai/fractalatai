#!/usr/bin/env python3
"""Approach L2: HIGH count as burden indicator.

Instead of total provisions (L) or avg_sig (J), use HIGH count directly
as the primary signal — more HIGH provisions = more compliance work.
Score = high_count * (high_count / total) — rewards both volume and concentration.
Rating: percentile-based — top 20% -> HIGH, bottom 33% -> LOW, else MEDIUM.

Usage:
    /usr/bin/python3 scripts/significance_approach_l2.py
    /usr/bin/python3 scripts/significance_approach_l2.py --provision-approach b_gravity_weighted
"""

import argparse
import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

PROVISION_APPROACHES = {
    "b_gravity_weighted": "b",
    "e_gravity_scope_gate": "e",
    "f2_gravity_no_strength": "f2",
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--provision-approach", default="e_gravity_scope_gate")
    args = parser.parse_args()

    prov = args.provision_approach
    suffix = PROVISION_APPROACHES.get(prov, prov[:2])
    APPROACH = f"l2_high_burden_{suffix}"
    PROVISION_APPROACH = prov

    print(f"Approach: {APPROACH} (provision input: {PROVISION_APPROACH})")

    conn = psycopg2.connect(PG_DSN)
    cur = conn.cursor()

    cur.execute("""
        SELECT split_part(e.section_id, ':', 1) as law_name,
               e.overall_rating, count(*) as cnt
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

    scored = []
    for law_name, profile in laws.items():
        total = profile["total"]
        if total == 0:
            continue
        high = profile["HIGH"]
        proportion = high / total
        # HIGH count * proportion — rewards volume AND concentration
        score = high * proportion if high > 0 else 0.0
        scored.append((law_name, profile, score))

    scored.sort(key=lambda x: -x[2])
    n = len(scored)
    top_20 = int(n * 0.20)
    bottom_33 = int(n * 0.67)

    results = []
    for i, (law_name, profile, score) in enumerate(scored):
        if i < top_20:
            rating = "HIGH"
        elif i >= bottom_33:
            rating = "LOW"
        else:
            rating = "MEDIUM"
        results.append((
            law_name, APPROACH, rating, score,
            profile["HIGH"], profile["MEDIUM"], profile["LOW"], profile["total"]
        ))

    dist = {"HIGH": 0, "MEDIUM": 0, "LOW": 0}
    for r in results:
        dist[r[2]] += 1
    total_laws = len(results)
    print(f"\nLaws rated: {total_laws}")
    print(f"Distribution:")
    for level in ("HIGH", "MEDIUM", "LOW"):
        pct = dist[level] / total_laws * 100 if total_laws else 0
        print(f"  {level:6s}: {dist[level]:>4} ({pct:5.1f}%)")

    print(f"\nBenchmark laws:")
    bench = {r[0]: r for r in results}
    for law in ["UK_ukpga_1974_37", "UK_uksi_2015_51", "UK_uksi_1999_3242"]:
        if law in bench:
            r = bench[law]
            rank = [i for i, x in enumerate(results) if x[0] == law][0] + 1
            print(f"  {law:<30s} {r[2]:>7s}  rank:{rank:>3}/{total_laws}  (score:{r[3]:.1f}, H:{r[4]} M:{r[5]} L:{r[6]} total:{r[7]})")

    print(f"\nTop 5:")
    for r in results[:5]:
        print(f"  {r[0]:<40s} score:{r[3]:>7.1f}  (H:{r[4]} M:{r[5]} L:{r[6]} total:{r[7]})")
    print(f"\nBottom 5:")
    for r in results[-5:]:
        print(f"  {r[0]:<40s} score:{r[3]:>7.1f}  (H:{r[4]} M:{r[5]} L:{r[6]} total:{r[7]})")

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
