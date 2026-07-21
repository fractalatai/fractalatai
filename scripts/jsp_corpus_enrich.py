#!/usr/bin/env python3
"""
Batch enrichment of the full JSP corpus.

Orchestrates: pull → enrich → extract-refs → extract-obligations →
extract-artefacts → extract-terms → controls → publish

Usage:
    /usr/bin/python3 scripts/jsp_corpus_enrich.py --pull       # Pull all JSP chapters
    /usr/bin/python3 scripts/jsp_corpus_enrich.py --enrich     # Run all enrichment steps
    /usr/bin/python3 scripts/jsp_corpus_enrich.py --publish    # Publish all to sertantai
    /usr/bin/python3 scripts/jsp_corpus_enrich.py --all        # Pull + enrich + publish
    /usr/bin/python3 scripts/jsp_corpus_enrich.py --stats      # Show corpus stats
"""

import argparse
import subprocess
import sys
import json
import time

TENANT = "dev"
CONNECT = "tcp/localhost:7447"
SYNC_BIN = ["cargo", "run", "-p", "fractalaw-sync-cli", "--"]
CLI_BIN = ["cargo", "run", "-p", "fractalaw-cli", "--"]


def run(cmd, label=None, check=True):
    """Run a command, print label, return stdout."""
    if label:
        print(f"\n{'='*60}")
        print(f"  {label}")
        print(f"{'='*60}")
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    if result.returncode != 0 and check:
        # Print stderr but don't crash — some commands have warnings
        if "error" in result.stderr.lower() and "warning" not in result.stderr.lower():
            print(f"  ERROR: {result.stderr[:200]}")
            return None
    # Filter out compiler warnings
    stdout = "\n".join(
        line for line in result.stdout.split("\n")
        if not line.strip().startswith("warning") and "Compiling" not in line
        and "Finished" not in line and "Running" not in line
    )
    if stdout.strip():
        print(stdout.strip())
    return result.stdout


def get_jsp_source_ids():
    """Get JSP chapter source_ids from DuckDB (after pull)."""
    import duckdb
    conn = duckdb.connect("data/fractalaw.duckdb", read_only=True)
    try:
        rows = conn.execute(
            "SELECT DISTINCT source_id FROM jsp_provisions WHERE source_id != 'TEST' ORDER BY source_id"
        ).fetchall()
        return [r[0] for r in rows]
    except Exception:
        return []
    finally:
        conn.close()


def query_sertantai_sources():
    """Query sertantai for all JSP source_ids via Zenoh."""
    result = subprocess.run(
        SYNC_BIN + ["pull-secondary", "--source-id", "QUERY_SOURCES",
                     "--tenant", TENANT, "--connect", CONNECT],
        capture_output=True, text=True, timeout=30
    )
    # This will fail — we need a proper source list query.
    # For now, query DuckDB after first pull or use a known list.
    return []


def get_all_jsp_chapters():
    """Return all JSP chapter source_ids from sertantai via Zenoh."""
    result = subprocess.run(
        SYNC_BIN + ["list-secondary", "--source-type", "jsp", "--ids-only",
                     "--tenant", TENANT, "--connect", CONNECT],
        capture_output=True, text=True, timeout=30
    )
    if result.returncode != 0:
        print(f"Failed to query sertantai for JSP chapters: {result.stderr[:200]}")
        return []

    ids = [line.strip() for line in result.stdout.split("\n")
           if line.strip() and not line.startswith("Waiting") and not line.startswith("warning")]
    print(f"Discovered {len(ids)} JSP chapters from sertantai")
    return ids


def pull_all(source_ids=None):
    """Pull provisions for all JSP chapters from sertantai into DuckDB."""
    if source_ids is None:
        source_ids = get_all_jsp_chapters()
        if not source_ids:
            print("No JSP chapters discovered. Is sertantai running?")
            return 0

    total = len(source_ids)
    pulled = 0
    for i, sid in enumerate(source_ids, 1):
        print(f"\n[{i}/{total}] Pulling {sid}...")
        result = run(
            SYNC_BIN + ["pull-secondary", "--source-id", sid,
                        "--tenant", TENANT, "--connect", CONNECT],
            check=False
        )
        if result and "Staged" in (result or ""):
            pulled += 1
        time.sleep(0.5)  # Be gentle with sertantai

    print(f"\nPulled {pulled}/{total} sources")
    return pulled


def enrich_all():
    """Run all enrichment steps on provisions in DuckDB."""
    source_ids = get_jsp_source_ids()
    if not source_ids:
        print("No JSP provisions in DuckDB. Run --pull first.")
        return

    total = len(source_ids)
    print(f"Enriching {total} JSP sources...")

    for i, sid in enumerate(source_ids, 1):
        print(f"\n[{i}/{total}] {sid}")

        # Step 1: DRRP enrichment
        run(CLI_BIN + ["jsp", "enrich", sid], f"DRRP enrich: {sid}", check=False)

        # Step 2: Reference extraction
        run(CLI_BIN + ["jsp", "extract-refs", sid], f"Extract refs: {sid}", check=False)

        # Step 3: Obligation + RACI extraction
        run(CLI_BIN + ["jsp", "extract-obligations", sid], f"Extract obligations: {sid}", check=False)

        # Step 4: Mandated artefact extraction
        run(CLI_BIN + ["jsp", "extract-artefacts", sid], f"Extract artefacts: {sid}", check=False)

        # Step 5: Term extraction
        run(CLI_BIN + ["jsp", "extract-terms", sid], f"Extract terms: {sid}", check=False)

        # Step 6: Controls generation
        run(CLI_BIN + ["jsp", "controls", sid], f"Generate controls: {sid}", check=False)

    print(f"\nEnrichment complete for {total} sources")


def publish_all():
    """Publish all enriched sources to sertantai."""
    source_ids = get_jsp_source_ids()
    if not source_ids:
        print("No JSP provisions in DuckDB. Run --pull first.")
        return

    total = len(source_ids)
    published = 0
    for i, sid in enumerate(source_ids, 1):
        result = run(
            SYNC_BIN + ["publish-secondary", "--source-id", sid,
                        "--tenant", TENANT, "--connect", CONNECT],
            f"[{i}/{total}] Publish: {sid}",
            check=False
        )
        if result and "Published" in (result or ""):
            published += 1
        time.sleep(0.3)

    print(f"\nPublished {published}/{total} sources")


def show_stats():
    """Show corpus-level enrichment statistics."""
    import duckdb
    conn = duckdb.connect("data/fractalaw.duckdb", read_only=True)

    try:
        # Provisions
        prov = conn.execute("SELECT count(*), count(DISTINCT source_id) FROM jsp_provisions WHERE source_id != 'TEST'").fetchone()
        print(f"Provisions:   {prov[0]:>6} across {prov[1]} sources")
    except Exception:
        print("No jsp_provisions table")
        return

    try:
        enr = conn.execute("SELECT count(*), count(DISTINCT source_id) FROM jsp_enrichment").fetchone()
        print(f"Enriched:     {enr[0]:>6} across {enr[1]} sources")

        strength = conn.execute("""
            SELECT obligation_strength, count(*)
            FROM jsp_enrichment
            WHERE obligation_strength IS NOT NULL
            GROUP BY obligation_strength
            ORDER BY count(*) DESC
        """).fetchall()
        for s, n in strength:
            print(f"  {s}: {n}")
    except Exception:
        print("No jsp_enrichment table")

    try:
        refs = conn.execute("SELECT count(*), count(CASE WHEN resolved THEN 1 END) FROM jsp_references").fetchone()
        rate = refs[1] / refs[0] * 100 if refs[0] > 0 else 0
        print(f"References:   {refs[0]:>6} ({refs[1]} resolved, {rate:.0f}%)")

        by_type = conn.execute("""
            SELECT target_type, count(*) FROM jsp_references GROUP BY target_type ORDER BY count(*) DESC
        """).fetchall()
        for t, n in by_type:
            print(f"  {t}: {n}")
    except Exception:
        print("No jsp_references table")

    try:
        obs = conn.execute("SELECT count(*), count(DISTINCT source_id) FROM jsp_obligations").fetchone()
        print(f"Obligations:  {obs[0]:>6} across {obs[1]} sources")
    except Exception:
        print("No jsp_obligations table")

    try:
        raci = conn.execute("SELECT count(*) FROM jsp_raci").fetchone()
        print(f"RACI:         {raci[0]:>6}")

        by_role = conn.execute("""
            SELECT role_label, count(*) FROM jsp_raci GROUP BY role_label ORDER BY count(*) DESC LIMIT 10
        """).fetchall()
        for r, n in by_role:
            print(f"  {r}: {n}")
    except Exception:
        print("No jsp_raci table")

    try:
        arts = conn.execute("SELECT count(*) FROM jsp_mandated_artefacts").fetchone()
        print(f"Artefacts:    {arts[0]:>6}")

        by_type = conn.execute("""
            SELECT artefact_type, count(*) FROM jsp_mandated_artefacts GROUP BY artefact_type ORDER BY count(*) DESC
        """).fetchall()
        for t, n in by_type:
            print(f"  {t}: {n}")
    except Exception:
        print("No jsp_mandated_artefacts table")

    try:
        terms = conn.execute("SELECT count(*), count(acronym) FROM jsp_terms").fetchone()
        print(f"Terms:        {terms[0]:>6} ({terms[1]} acronyms)")
    except Exception:
        print("No jsp_terms table")

    try:
        controls = conn.execute("SELECT count(*) FROM suggested_controls WHERE source_id IS NOT NULL").fetchone()
        print(f"JSP Controls: {controls[0]:>6}")
    except Exception:
        pass

    conn.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Batch JSP corpus enrichment")
    parser.add_argument("--pull", action="store_true", help="Pull all JSP chapters from sertantai")
    parser.add_argument("--enrich", action="store_true", help="Run enrichment pipeline on all chapters")
    parser.add_argument("--publish", action="store_true", help="Publish all enrichment to sertantai")
    parser.add_argument("--all", action="store_true", help="Pull + enrich + publish")
    parser.add_argument("--stats", action="store_true", help="Show corpus-level stats")
    parser.add_argument("--source-ids", help="Comma-separated source_ids (override auto-discovery)")
    args = parser.parse_args()

    if not any([args.pull, args.enrich, args.publish, args.all, args.stats]):
        parser.print_help()
        sys.exit(1)

    source_ids = args.source_ids.split(",") if args.source_ids else None

    if args.stats:
        show_stats()
    elif args.all:
        pull_all(source_ids)
        enrich_all()
        publish_all()
    else:
        if args.pull:
            pull_all(source_ids)
        if args.enrich:
            enrich_all()
        if args.publish:
            publish_all()
