#!/usr/bin/env python3
"""Corpus pipeline coverage statistics.

Reports provision scope (OUT/STRUCTURAL/SUBSTANTIVE) and per-tier coverage
against the base case. Used as QA gate for each pipeline tier.

Usage:
    /usr/bin/python3 scripts/corpus_stats.py
    /usr/bin/python3 scripts/corpus_stats.py --laws UK_ukpga_1974_37
    /usr/bin/python3 scripts/corpus_stats.py --benchmarks-only
"""

import argparse
import psycopg2

PG = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

OUT_SECTION_TYPES = (
    "heading", "part", "chapter", "signed",
    "title", "table", "commencement", "note",
    "schedule",
)

STRUCTURAL_PURPOSES = (
    "Enactment+Citation+Commencement",
    "Interpretation+Definition",
    "Amendment",
    "Repeal+Revocation",
    "Extent",
    "Transitional Arrangement",
    "Unclassified",
)

DRRP_MODALS = (
    " shall ", " must ", " ensure ", " required to ", " responsible for ",
    " entitled to ", " has the right ", " have the right ",
    " may ", " has the power ", " have the power ",
)


def main():
    parser = argparse.ArgumentParser(description="Corpus pipeline coverage stats")
    parser.add_argument("--laws", help="Comma-separated law names")
    parser.add_argument("--law-file", help="CSV file with law names (e.g. data/qq-applicable-laws.csv)")
    parser.add_argument("--benchmarks-only", action="store_true")
    parser.add_argument("--all", action="store_true", help="All laws (no filter)")
    args = parser.parse_args()

    conn = psycopg2.connect(PG)
    cur = conn.cursor()

    law_filter = ""
    label = ""
    if args.laws:
        names = [f"'{n.strip()}'" for n in args.laws.split(",")]
        law_filter = f"AND lt.law_name IN ({','.join(names)})"
        label = f"{len(names)} specified laws"
    elif args.law_file:
        import os
        with open(args.law_file) as f:
            csv_laws = [n.strip() for n in f.read().split(",") if n.strip()]
        names = [f"'{n}'" for n in csv_laws]
        law_filter = f"AND lt.law_name IN ({','.join(names)})"
        label = f"{len(csv_laws)} laws from {os.path.basename(args.law_file)}"
    elif args.benchmarks_only:
        law_filter = "AND lt.law_name IN (SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks)"
        label = "benchmark laws"
    elif args.all:
        law_filter = ""
        label = "all laws"
    else:
        law_filter = "AND lt.law_name NOT IN (SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks)"
        label = "non-benchmark laws"

    print(f"Corpus: {label}\n")

    # Tier 0: Base case
    print("=" * 70)
    print("TIER 0: BASE CASE — Provision Scope")
    print("=" * 70)

    cur.execute(f"""
        SELECT
            count(*) as total,
            count(*) FILTER (WHERE section_type IN %s) as out_section_type,
            count(*) FILTER (WHERE section_type NOT IN %s AND length(text) < 20) as out_short,
            count(*) FILTER (WHERE section_type NOT IN %s AND length(text) >= 20) as in_scope
        FROM legislation_text lt
        WHERE text IS NOT NULL {law_filter}
    """, (OUT_SECTION_TYPES, OUT_SECTION_TYPES, OUT_SECTION_TYPES))
    total, out_st, out_short, in_scope = cur.fetchone()
    out_total = out_st + out_short

    print(f"  Total provisions:       {total:>8,}")
    print(f"  OUT (section_type):     {out_st:>8,}  ({100*out_st/total:.1f}%)")
    print(f"  OUT (short text):       {out_short:>8,}  ({100*out_short/total:.1f}%)")
    print(f"  IN SCOPE:               {in_scope:>8,}  ({100*in_scope/total:.1f}%)")

    # Orphan actors on OUT provisions
    cur.execute(f"""
        SELECT count(*)
        FROM provision_actors pa
        JOIN legislation_text lt ON pa.section_id = lt.section_id
        WHERE (lt.section_type IN %s OR length(lt.text) < 20)
        {law_filter}
    """, (OUT_SECTION_TYPES,))
    orphans = cur.fetchone()[0]
    status = "PASS" if orphans == 0 else f"FAIL ({orphans} orphan actors)"
    print(f"  QA: No actors on OUT:   {status}")

    # Tier 1: Regex + Embed
    print(f"\n{'=' * 70}")
    print("TIER 1: REGEX + EMBED — In-scope provisions")
    print("=" * 70)

    cur.execute(f"""
        SELECT
            count(*) as in_scope,
            count(*) FILTER (WHERE embedding IS NOT NULL) as has_embedding,
            count(*) FILTER (WHERE lt.extraction_method IS NOT NULL) as has_extraction
        FROM legislation_text lt
        WHERE lt.scope = 'substantive'
        {law_filter}
    """)
    in_scope, has_emb, has_ext = cur.fetchone()
    emb_gap = in_scope - has_emb

    print(f"  In-scope provisions:    {in_scope:>8,}")
    print(f"  Has embedding:          {has_emb:>8,}  ({100*has_emb/in_scope:.1f}%)")
    print(f"  Embedding gap:          {emb_gap:>8,}  ({100*emb_gap/in_scope:.1f}%)")
    print(f"  Has extraction_method:  {has_ext:>8,}  ({100*has_ext/in_scope:.1f}%)")

    # Laws with embedding gaps
    cur.execute(f"""
        SELECT lt.law_name, count(*) as missing
        FROM legislation_text lt
        WHERE embedding IS NULL
        AND lt.scope = 'substantive'
        {law_filter}
        GROUP BY lt.law_name
        ORDER BY count(*) DESC
    """)
    gap_laws = cur.fetchall()
    if gap_laws:
        print(f"  Laws with gaps:         {len(gap_laws):>8,}")
        for name, cnt in gap_laws[:5]:
            print(f"    {name}: {cnt} provisions")
        if len(gap_laws) > 5:
            print(f"    ... and {len(gap_laws) - 5} more")

    # Actor coverage
    cur.execute(f"""
        SELECT
            count(DISTINCT pa.section_id) as provisions_with_actors,
            count(*) as total_actors,
            count(*) FILTER (WHERE pa.regex_position IS NOT NULL) as has_regex_pos
        FROM provision_actors pa
        JOIN legislation_text lt ON pa.section_id = lt.section_id
        WHERE lt.scope = 'substantive'
        {law_filter}
    """)
    prov_with_actors, total_actors, has_regex = cur.fetchone()
    print(f"  Provisions with actors: {prov_with_actors:>8,}  ({100*prov_with_actors/in_scope:.1f}%)")
    print(f"  Total actors:           {total_actors:>8,}")
    print(f"  Has regex_position:     {has_regex:>8,}  ({100*has_regex/total_actors:.1f}%)" if total_actors > 0 else "")
    emb_status = "PASS" if emb_gap == 0 else f"FAIL ({emb_gap} in-scope provisions without embedding)"
    print(f"  QA: Embedding coverage: {emb_status}")

    # Tier 2: Classifier
    print(f"\n{'=' * 70}")
    print("TIER 2: CLASSIFIER — Actors on in-scope provisions")
    print("=" * 70)

    cur.execute(f"""
        SELECT
            count(*) as total_actors,
            count(*) FILTER (WHERE pa.regex_position IS NOT NULL) as has_regex,
            count(*) FILTER (WHERE pa.dep_is_subject IS NOT NULL) as has_dep,
            count(*) FILTER (WHERE pa.cls_position IS NOT NULL) as has_cls,
            count(*) FILTER (WHERE pa.cls_confidence IS NOT NULL) as has_conf,
            count(*) FILTER (WHERE lt.embedding IS NOT NULL AND pa.dep_is_subject IS NOT NULL) as eligible,
            count(*) FILTER (WHERE lt.embedding IS NOT NULL AND pa.dep_is_subject IS NOT NULL AND pa.cls_position IS NOT NULL) as classified
        FROM provision_actors pa
        JOIN legislation_text lt ON pa.section_id = lt.section_id
        WHERE lt.scope = 'substantive'
        {law_filter}
    """)
    total_a, has_regex, has_dep, has_cls, has_conf, eligible, classified = cur.fetchone()

    if total_a == 0:
        print("  No actors found")
    else:
        print(f"  Total actors (in-scope):{total_a:>8,}")
        print(f"  Has regex position:     {has_regex:>8,}  ({100*has_regex/total_a:.1f}%)")
        print(f"  Has dep features:       {has_dep:>8,}  ({100*has_dep/total_a:.1f}%)")
        print(f"  Eligible (emb + dep):   {eligible:>8,}  ({100*eligible/total_a:.1f}%)")
        print(f"  Has cls position:       {has_cls:>8,}  ({100*has_cls/total_a:.1f}%)")
        gap = eligible - classified
        not_eligible = total_a - eligible
        status = "PASS" if gap == 0 else f"FAIL ({gap} eligible actors without cls_position)"
        print(f"  Not eligible (no emb):  {not_eligible:>8,}  ({100*not_eligible/total_a:.1f}%)")
        print(f"  QA: Classifier gap:     {status}")

    # Tier 3: Reconciliation + SLM
    print(f"\n{'=' * 70}")
    print("TIER 3: RECONCILIATION + SLM")
    print("=" * 70)

    cur.execute(f"""
        SELECT
            pa.extraction_method,
            count(*) as cnt
        FROM provision_actors pa
        JOIN legislation_text lt ON pa.section_id = lt.section_id
        WHERE lt.scope = 'substantive'
        AND pa.extraction_method IS NOT NULL
        {law_filter}
        GROUP BY pa.extraction_method
        ORDER BY count(*) DESC
    """)
    methods = cur.fetchall()
    reconciled = sum(c for _, c in methods)

    print(f"  Reconciled actors:      {reconciled:>8,}")
    for method, cnt in methods:
        print(f"    {method:25s} {cnt:>8,}  ({100*cnt/reconciled:.1f}%)")

    pending_slm = sum(c for m, c in methods if m == "pending_slm")
    pending_llm = sum(c for m, c in methods if m == "pending_llm")
    print(f"  QA: pending_slm:        {'PASS' if pending_slm == 0 else f'{pending_slm} remaining'}")
    print(f"  QA: pending_llm:        {pending_llm} (human-triggered)")

    print(f"\n{'=' * 70}")
    cur.close()
    conn.close()


if __name__ == "__main__":
    main()
