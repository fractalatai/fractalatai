#!/usr/bin/env python3
"""Batch RACI inference on JSP obligations via Ollama (RunPod or local).

Queries DuckDB for obligations without RACI, sends each to the
gemma3-raci model via Ollama, writes results to DuckDB.

IMPORTANT: Namespaced to /workspace/raci/ on RunPod.

Setup:
    # On RunPod (after fine-tuning):
    ollama create gemma3-raci -f /workspace/raci/Modelfile
    # Or locally after transferring the GGUF:
    ollama create gemma3-raci -f Modelfile

    # Reverse SSH tunnel for Postgres (if running on RunPod):
    ssh -R 5433:localhost:5433 root@pod-ip

Usage:
    /usr/bin/python3 scripts/ml/runpod_raci_batch.py
    /usr/bin/python3 scripts/ml/runpod_raci_batch.py --source-id JSP-375-CH23
    /usr/bin/python3 scripts/ml/runpod_raci_batch.py --limit 10  # test run
    /usr/bin/python3 scripts/ml/runpod_raci_batch.py --workers 4  # parallel
"""

import argparse
import json
import sys
import time
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed

import duckdb
import urllib.request

OLLAMA_URL = "http://localhost:11434/api/chat"
MODEL = "gemma3-raci"

SYSTEM_PROMPT = """You are a RACI classifier for MoD Joint Service Publications (JSPs).

Given an obligation from a JSP, identify:
1. Which organisational role(s) are mentioned
2. Their RACI assignment: R (Responsible - does the work), A (Accountable - owns the outcome), C (Consulted - asked for input), I (Informed - notified)

Respond with a JSON array of assignments. If no role is identifiable, respond with an empty array [].

Example roles: MoD: Commanding Officer, MoD: Accountable Person, MoD: Senior Duty Holder, MoD: Defence Safety Authority, MoD: Contractor, MoD: User, MoD: Commander/Manager, MoD: Head of Establishment, MoD: Defence Organisation"""


def classify_raci(text):
    """Send obligation text to Ollama and get RACI classification."""
    payload = json.dumps({
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": text},
        ],
        "stream": False,
        "options": {"temperature": 0.0},
    }).encode()

    req = urllib.request.Request(OLLAMA_URL, data=payload, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.load(resp)

    content = data["message"]["content"].strip()

    # Parse JSON array from response
    # Handle markdown code fences if present
    if content.startswith("```"):
        content = content.split("```")[1]
        if content.startswith("json"):
            content = content[4:]

    return json.loads(content)


def main():
    parser = argparse.ArgumentParser(description="Batch RACI inference via Ollama")
    parser.add_argument("--source-id", help="Process a specific source only")
    parser.add_argument("--limit", type=int, help="Limit number of obligations to process")
    parser.add_argument("--workers", type=int, default=1, help="Parallel workers")
    parser.add_argument("--db", default="data/fractalaw.duckdb")
    args = parser.parse_args()

    # Preflight: check Ollama
    print("Checking Ollama...", end=" ")
    try:
        resp = urllib.request.urlopen("http://localhost:11434/api/tags", timeout=5)
        models = json.load(resp)
        model_names = [m["name"] for m in models.get("models", [])]
        if MODEL in model_names or f"{MODEL}:latest" in model_names:
            print(f"OK ({MODEL} loaded)")
        else:
            print(f"WARNING: {MODEL} not found. Available: {model_names}")
            print(f"Run: ollama create {MODEL} -f Modelfile")
            sys.exit(1)
    except Exception as e:
        print(f"FAIL ({e})")
        sys.exit(1)

    # Load obligations without SLM RACI
    conn = duckdb.connect(args.db, read_only=True)

    where_clauses = ["r.raci_id IS NULL", "o.strength = 'Mandatory'"]
    if args.source_id:
        where_clauses.append(f"o.source_id = '{args.source_id}'")

    where = " AND ".join(where_clauses)
    limit = f"LIMIT {args.limit}" if args.limit else ""

    rows = conn.execute(f"""
        SELECT o.obligation_id, o.section_id, o.source_id, o.text
        FROM jsp_obligations o
        LEFT JOIN jsp_raci r ON o.obligation_id = r.obligation_id
        WHERE {where}
        ORDER BY o.source_id, o.section_id
        {limit}
    """).fetchall()
    conn.close()

    print(f"Obligations to classify: {len(rows)}")
    if not rows:
        print("Nothing to classify.")
        return

    # Classify
    results = []
    errors = 0
    t0 = time.time()

    def process_one(row):
        oid, sid, src, text = row
        try:
            raci = classify_raci(text)
            return (oid, sid, src, raci, None)
        except Exception as e:
            return (oid, sid, src, None, str(e))

    if args.workers > 1:
        with ThreadPoolExecutor(max_workers=args.workers) as executor:
            futures = {executor.submit(process_one, row): row for row in rows}
            for i, future in enumerate(as_completed(futures), 1):
                result = future.result()
                if result[4]:  # error
                    errors += 1
                else:
                    results.append(result)
                if i % 50 == 0:
                    elapsed = time.time() - t0
                    print(f"  {i}/{len(rows)} ({elapsed:.0f}s, {errors} errors)")
    else:
        for i, row in enumerate(rows, 1):
            result = process_one(row)
            if result[4]:
                errors += 1
                if errors <= 3:
                    print(f"  Error: {result[4][:100]}")
            else:
                results.append(result)
            if i % 50 == 0:
                elapsed = time.time() - t0
                print(f"  {i}/{len(rows)} ({elapsed:.0f}s, {errors} errors)")

    elapsed = time.time() - t0
    print(f"\nClassified {len(results)}/{len(rows)} in {elapsed:.0f}s ({errors} errors)")

    # Stats
    total_assignments = sum(len(r[3]) for r in results)
    print(f"Total RACI assignments: {total_assignments}")

    role_dist = Counter()
    type_dist = Counter()
    for _, _, _, raci, _ in results:
        for a in raci:
            role_dist[a.get("role", "unknown")] += 1
            type_dist[a.get("type", "unknown")] += 1

    print("\nBy type:")
    for t, n in type_dist.most_common():
        print(f"  {t}: {n}")
    print("\nTop roles:")
    for r, n in role_dist.most_common(10):
        print(f"  {r}: {n}")

    # Write to DuckDB
    conn = duckdb.connect(args.db)

    # Add slm columns if missing
    cols = conn.execute("SELECT column_name FROM information_schema.columns WHERE table_name = 'jsp_raci'").fetchall()
    col_names = [c[0] for c in cols]
    if "assignment_source" not in col_names:
        # Table might not have the column — it should from Phase 3
        pass

    inserted = 0
    for oid, sid, src, raci, _ in results:
        for ri, a in enumerate(raci):
            role = a.get("role", "").replace("'", "''")
            atype = a.get("type", "R")
            raci_id = f"{oid}:slm.{ri}"

            conn.execute(f"""
                INSERT OR REPLACE INTO jsp_raci (raci_id, obligation_id, section_id, source_id, role_label, assignment_type, assignment_source)
                VALUES ('{raci_id.replace("'", "''")}', '{oid.replace("'", "''")}', '{sid.replace("'", "''")}', '{src.replace("'", "''")}', '{role}', '{atype}', 'slm')
            """)
            inserted += 1

    conn.close()
    print(f"\nInserted {inserted} SLM RACI assignments into DuckDB")


if __name__ == "__main__":
    main()
