#!/usr/bin/env python3
"""Part/Chapter breakdown — treat large Acts as a series of "regulations".

For Acts (ukpga, asp, anaw, apni), extract section number from section_id and
group by Part (using known Part boundaries from the Act structure).
Report significance distribution per Part.

This analysis explores whether publishing sub-law significance (per Part/Chapter)
gives compliance officers better signal for large foundational Acts.

Usage:
    /usr/bin/python3 scripts/significance_part_breakdown.py
    /usr/bin/python3 scripts/significance_part_breakdown.py --provision-approach b_gravity_weighted
"""

import argparse
import math
import re
import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

PROVISION_APPROACHES = {
    "b_gravity_weighted": "b",
    "e_gravity_scope_gate": "e",
    "f2_gravity_no_strength": "f2",
}


def extract_section_num(section_id):
    """Extract primary section number from section_id like 'UK_...:s.2(1)'."""
    m = re.search(r':s\.(\d+)', section_id)
    if m:
        return int(m.group(1))
    m = re.search(r':reg\.(\d+)', section_id)
    if m:
        return int(m.group(1))
    return None


def get_part_boundaries(cur, law_name):
    """Get Part boundaries from structural rows in legislation_text."""
    cur.execute("""
        SELECT section_id, section_type
        FROM legislation_text
        WHERE law_name = %s AND section_type = 'part'
        ORDER BY section_id
    """, (law_name,))
    parts = cur.fetchall()
    if not parts:
        return None
    return parts


def assign_part_by_section_range(section_num, part_ranges):
    """Assign a section number to a Part based on ranges."""
    for part_name, (start, end) in part_ranges.items():
        if start <= section_num <= end:
            return part_name
    return "Other"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--provision-approach", default="e_gravity_scope_gate")
    args = parser.parse_args()

    prov = args.provision_approach
    suffix = PROVISION_APPROACHES.get(prov, prov[:2])
    PROVISION_APPROACH = prov

    print(f"Part/Chapter breakdown (provision input: {PROVISION_APPROACH})")

    conn = psycopg2.connect(PG_DSN)
    cur = conn.cursor()

    # Find Acts with many rated provisions (>50) — candidates for Part breakdown
    cur.execute("""
        SELECT split_part(e.section_id, ':', 1) as law_name, count(*) as cnt
        FROM significance_overall_experiments e
        WHERE e.approach = %s
        AND split_part(e.section_id, ':', 1) ~ '(ukpga|asp|anaw|apni)'
        GROUP BY 1
        HAVING count(*) > 50
        ORDER BY 2 DESC
    """, (PROVISION_APPROACH,))
    large_acts = cur.fetchall()
    print(f"\nLarge Acts (>50 rated provisions): {len(large_acts)}")

    for law_name, total_count in large_acts:
        # Get all rated provisions with their overall rating
        cur.execute("""
            SELECT e.section_id, e.overall_rating
            FROM significance_overall_experiments e
            WHERE e.approach = %s
            AND split_part(e.section_id, ':', 1) = %s
        """, (PROVISION_APPROACH, law_name))
        provisions = cur.fetchall()

        # Check if this Act has Part structural rows
        parts = get_part_boundaries(cur, law_name)

        # Group by section number ranges
        # Extract section numbers and sort
        by_section = []
        for sid, rating in provisions:
            num = extract_section_num(sid)
            if num is not None:
                by_section.append((num, rating))

        if not by_section:
            continue

        by_section.sort(key=lambda x: x[0])
        min_s, max_s = by_section[0][0], by_section[-1][0]

        # Simple quartile split if no Part boundaries known
        # Split into roughly equal ranges by section number
        range_size = (max_s - min_s + 1)
        if range_size < 4:
            continue

        quarter = range_size // 4
        ranges = {
            f"ss.{min_s}-{min_s+quarter-1}": (min_s, min_s + quarter - 1),
            f"ss.{min_s+quarter}-{min_s+2*quarter-1}": (min_s + quarter, min_s + 2 * quarter - 1),
            f"ss.{min_s+2*quarter}-{min_s+3*quarter-1}": (min_s + 2 * quarter, min_s + 3 * quarter - 1),
            f"ss.{min_s+3*quarter}-{max_s}": (min_s + 3 * quarter, max_s),
        }

        print(f"\n{'='*70}")
        print(f"{law_name} ({total_count} provisions, ss.{min_s}-{max_s})")
        if parts:
            print(f"  Parts found: {', '.join(p[0].split(':')[1] for p in parts)}")
        print(f"  {'Range':<25s} {'HIGH':>5s} {'MED':>5s} {'LOW':>5s} {'Total':>5s} {'%HIGH':>6s}")

        for range_name, (start, end) in ranges.items():
            h = m = l = 0
            for num, rating in by_section:
                if start <= num <= end:
                    if rating == "HIGH":
                        h += 1
                    elif rating == "MEDIUM":
                        m += 1
                    else:
                        l += 1
            total = h + m + l
            pct = h / total * 100 if total > 0 else 0
            print(f"  {range_name:<25s} {h:>5d} {m:>5d} {l:>5d} {total:>5d} {pct:>5.1f}%")

        # Also show L-score if this were treated as separate "laws"
        print(f"\n  If each range were a separate law (L-score = avg_sig * log2(n+1)):")
        for range_name, (start, end) in ranges.items():
            h = m = l = 0
            for num, rating in by_section:
                if start <= num <= end:
                    if rating == "HIGH":
                        h += 1
                    elif rating == "MEDIUM":
                        m += 1
                    else:
                        l += 1
            total = h + m + l
            if total == 0:
                continue
            avg_sig = (3 * h + 2 * m + 1 * l) / total
            l_score = avg_sig * math.log2(total + 1)
            print(f"    {range_name:<25s} avg:{avg_sig:.2f}  L-score:{l_score:.2f}")

    cur.close()
    conn.close()


if __name__ == "__main__":
    main()
