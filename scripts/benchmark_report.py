#!/usr/bin/env /usr/bin/python3
"""Per-tier benchmark report from provision_actors + gold_benchmarks tables.

Compares regex, classifier, and reconciled signals against gold standard.
Both tables must be populated in Postgres before running.

Usage:
    /usr/bin/python3 scripts/benchmark_report.py
    /usr/bin/python3 scripts/benchmark_report.py --category Org
"""

import argparse
import psycopg2

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"


def run_report(category_filter=None):
    conn = psycopg2.connect(PG)
    cur = conn.cursor()

    where = ""
    if category_filter:
        where = f"AND pa.actor_category = '{category_filter}'"

    # Per-tier accuracy
    cur.execute(f"""
        SELECT
            count(*) as total,
            count(*) FILTER (WHERE pa.regex_position = g.gold_position) as regex_pos_ok,
            count(*) FILTER (WHERE pa.cls_position = g.gold_position) as cls_pos_ok,
            count(*) FILTER (WHERE pa.inferred_position = g.gold_position) as inf_pos_ok,
            count(*) FILTER (WHERE pa.regex_drrp = g.gold_drrp) as regex_drrp_ok,
            count(*) FILTER (WHERE pa.cls_drrp = g.gold_drrp) as cls_drrp_ok,
            count(*) FILTER (WHERE pa.inferred_drrp = g.gold_drrp) as inf_drrp_ok,
            count(pa.regex_position) as regex_pos_total,
            count(pa.cls_position) as cls_pos_total,
            count(pa.inferred_position) as inf_pos_total,
            count(pa.regex_drrp) as regex_drrp_total,
            count(pa.cls_drrp) as cls_drrp_total,
            count(pa.inferred_drrp) as inf_drrp_total
        FROM gold_benchmarks g
        JOIN provision_actors pa USING (section_id, actor_label)
        WHERE 1=1 {where}
    """)
    r = cur.fetchone()
    total = r[0]
    rp_ok, cp_ok, ip_ok = r[1], r[2], r[3]
    rd_ok, cd_ok, id_ok = r[4], r[5], r[6]
    rp_t, cp_t, ip_t = r[7], r[8], r[9]
    rd_t, cd_t, id_t = r[10], r[11], r[12]

    pct = lambda ok, t: f"{100*ok/t:.1f}%" if t > 0 else "n/a"

    print(f"=== Benchmark Report ===")
    if category_filter:
        print(f"  Category filter: {category_filter}")
    print(f"  Matched: {total} gold actors in provision_actors\n")

    print(f"  {'Metric':<20} {'Regex':>12} {'Classifier':>12} {'Inferred':>12}")
    print(f"  {'DRRP':<20} {pct(rd_ok,rd_t):>12} {pct(cd_ok,cd_t):>12} {pct(id_ok,id_t):>12}")
    print(f"  {'Position':<20} {pct(rp_ok,rp_t):>12} {pct(cp_ok,cp_t):>12} {pct(ip_ok,ip_t):>12}")

    # Disagreement analysis
    cur.execute(f"""
        SELECT
            count(*) FILTER (WHERE pa.regex_position = pa.cls_position AND pa.regex_position = g.gold_position) as agree_correct,
            count(*) FILTER (WHERE pa.regex_position = pa.cls_position AND pa.regex_position != g.gold_position) as agree_wrong,
            count(*) FILTER (WHERE pa.regex_position != pa.cls_position AND pa.regex_position = g.gold_position) as regex_right,
            count(*) FILTER (WHERE pa.regex_position != pa.cls_position AND pa.cls_position = g.gold_position) as cls_right,
            count(*) FILTER (WHERE pa.regex_position != pa.cls_position AND pa.regex_position != g.gold_position AND pa.cls_position != g.gold_position) as both_wrong
        FROM gold_benchmarks g
        JOIN provision_actors pa USING (section_id, actor_label)
        WHERE pa.regex_position IS NOT NULL AND pa.cls_position IS NOT NULL {where}
    """)
    ac, aw, rr, cr, bw = cur.fetchone()
    dis_total = ac + aw + rr + cr + bw

    print(f"\n  === Disagreement Analysis ({dis_total} actors) ===")
    print(f"  {'Agree + correct':<25} {ac:>6} ({pct(ac,dis_total)})")
    print(f"  {'Agree + wrong':<25} {aw:>6} ({pct(aw,dis_total)})")
    print(f"  {'Disagree, regex right':<25} {rr:>6} ({pct(rr,dis_total)})")
    print(f"  {'Disagree, cls right':<25} {cr:>6} ({pct(cr,dis_total)})")
    print(f"  {'Disagree, both wrong':<25} {bw:>6} ({pct(bw,dis_total)})")

    # Per-category breakdown
    cur.execute(f"""
        SELECT
            pa.actor_category,
            count(*) as total,
            count(*) FILTER (WHERE pa.regex_position = g.gold_position) as regex_ok,
            count(*) FILTER (WHERE pa.cls_position = g.gold_position) as cls_ok,
            count(*) FILTER (WHERE pa.inferred_position = g.gold_position) as inf_ok,
            count(pa.inferred_position) as inf_total
        FROM gold_benchmarks g
        JOIN provision_actors pa USING (section_id, actor_label)
        WHERE 1=1 {where}
        GROUP BY pa.actor_category
        ORDER BY total DESC
    """)

    print(f"\n  === Per-Category Position Accuracy ===")
    print(f"  {'Category':<12} {'Total':>6} {'Regex':>10} {'Classifier':>12} {'Inferred':>12}")
    for cat, t, rok, cok, iok, it in cur.fetchall():
        print(f"  {cat or 'unknown':<12} {t:>6} {pct(rok,t):>10} {pct(cok,t):>12} {pct(iok,it):>12}")

    # Position confusion matrices
    for tier, col in [("Regex", "regex_position"), ("Classifier", "cls_position"), ("Inferred", "inferred_position")]:
        cur.execute(f"""
            SELECT g.gold_position, pa.{col}, count(*)
            FROM gold_benchmarks g
            JOIN provision_actors pa USING (section_id, actor_label)
            WHERE pa.{col} IS NOT NULL {where}
            GROUP BY g.gold_position, pa.{col}
            ORDER BY g.gold_position, pa.{col}
        """)
        rows = cur.fetchall()
        if not rows:
            continue
        positions = sorted(set(r[0] for r in rows) | set(r[1] for r in rows))
        matrix = {}
        for gold, pred, cnt in rows:
            matrix[(gold, pred)] = cnt

        print(f"\n  === {tier} Position Confusion Matrix ===")
        print(f"  {'gold↓ pipe→':>15}", end='')
        for p in positions:
            print(f"{p:>14}", end='')
        print()
        for g in positions:
            print(f"  {g:>15}", end='')
            for p in positions:
                print(f"{matrix.get((g,p), 0):>14}", end='')
            print()

    cur.close()
    conn.close()


def main():
    parser = argparse.ArgumentParser(description="Benchmark report from provision_actors")
    parser.add_argument("--category", help="Filter by actor category (Org, Ind, Gvt, etc.)")
    args = parser.parse_args()

    run_report(args.category)


if __name__ == "__main__":
    main()
