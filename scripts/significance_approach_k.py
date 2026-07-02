#!/usr/bin/env python3
"""Approach K: Distribution profile — publish the triple {high, medium, low}.

No single rating. Sort by HIGH count, then MEDIUM as tiebreaker.

Usage:
    /usr/bin/python3 scripts/significance_approach_k.py
    /usr/bin/python3 scripts/significance_approach_k.py --provision-approach b_gravity_weighted
"""

import argparse
import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

PROVISION_APPROACHES = {
    "b_gravity_weighted": "b",
    "e_gravity_scope_gate": "e",
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--provision-approach", default="e_gravity_scope_gate",
                        help="Provision-level approach key in significance_overall_experiments")
    args = parser.parse_args()

    prov = args.provision_approach
    suffix = PROVISION_APPROACHES.get(prov, prov[:1])
    APPROACH = f"k_distribution_{suffix}"
    PROVISION_APPROACH = prov

    print(f"Approach: {APPROACH} (provision input: {PROVISION_APPROACH})")

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

    # Sort by HIGH count desc, then MEDIUM count desc as tiebreaker
    ranked = sorted(laws.items(), key=lambda x: (x[1]["HIGH"], x[1]["MEDIUM"]), reverse=True)

    results = []
    for law_name, profile in ranked:
        # No overall_rating for this approach — it's the full profile
        results.append((
            law_name, APPROACH, None, None,
            profile["HIGH"], profile["MEDIUM"], profile["LOW"], profile["total"]
        ))

    print(f"Laws rated: {len(results)}")

    print(f"\nTop 10 laws (sorted HIGH desc, MED desc):")
    for r in results[:10]:
        print(f"  {r[0]:<40s} H:{r[4]:>3} M:{r[5]:>3} L:{r[6]:>3}  (total:{r[7]})")

    print(f"\nBottom 10 laws:")
    for r in results[-10:]:
        print(f"  {r[0]:<40s} H:{r[4]:>3} M:{r[5]:>3} L:{r[6]:>3}  (total:{r[7]})")

    print(f"\nBenchmark laws:")
    bench = {r[0]: r for r in results}
    for law in ["UK_ukpga_1974_37", "UK_uksi_2015_51", "UK_uksi_1999_3242"]:
        if law in bench:
            r = bench[law]
            rank = [i for i, x in enumerate(results) if x[0] == law][0] + 1
            print(f"  {law:<30s} rank:{rank:>3}/{len(results)}  (H:{r[4]} M:{r[5]} L:{r[6]} total:{r[7]})")

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
