#!/usr/bin/env python3
"""Batch RACI inference on JSP obligations via Ollama on RunPod.

Runs ON THE POD. Reads/writes fractalaw Postgres via reverse SSH tunnel.
Ollama calls are localhost (fast). DB writes go through the tunnel (small).

Setup on RunPod:
    # Reverse SSH tunnel (in a separate terminal on LOCAL machine):
    ssh -p <PORT> -i ~/.ssh/id_ed25519 -R 5433:localhost:5433 root@<IP> -N

    # On pod:
    ollama create gemma3-raci -f /workspace/raci/Modelfile
    python3 -u /workspace/raci/runpod_raci_batch.py --limit 10   # verify
    python3 -u /workspace/raci/runpod_raci_batch.py --workers 4  # full batch

Writes to: jsp_provisions.slm_raci (JSONB) via reverse tunnel to local PG.
"""

import argparse
import json
import sys
import time
import threading
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed

import psycopg2
import urllib.request

OLLAMA_URL = "http://localhost:11434/api/chat"
MODEL = "gemma3-raci"
PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

SYSTEM_PROMPT = """You are a RACI classifier for MoD Joint Service Publications (JSPs).

Given an obligation from a JSP, identify:
1. Which organisational role(s) are mentioned
2. Their RACI assignment: R (Responsible - does the work), A (Accountable - owns the outcome), C (Consulted - asked for input), I (Informed - notified)

Respond with a JSON array of assignments. If no role is identifiable, respond with an empty array [].

Example roles: MoD: Commanding Officer, MoD: Accountable Person, MoD: Senior Duty Holder, MoD: Defence Safety Authority, MoD: Contractor, MoD: User, MoD: Commander/Manager, MoD: Head of Establishment, MoD: Defence Organisation"""

write_lock = threading.Lock()


def classify_raci(text):
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
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = json.load(resp)

    content = data["message"]["content"].strip()
    if content.startswith("```"):
        content = content.split("```")[1]
        if content.startswith("json"):
            content = content[4:]
    return json.loads(content)


def main():
    parser = argparse.ArgumentParser(description="Batch RACI inference via Ollama (RunPod)")
    parser.add_argument("--source-id", help="Process a specific source only")
    parser.add_argument("--limit", type=int, help="Limit number of provisions to process")
    parser.add_argument("--workers", type=int, default=1, help="Parallel workers")
    args = parser.parse_args()

    # Preflight: Ollama
    print("Checking Ollama...", end=" ")
    try:
        resp = urllib.request.urlopen("http://localhost:11434/api/tags", timeout=5)
        models = json.load(resp)
        names = [m["name"] for m in models.get("models", [])]
        if MODEL in names or f"{MODEL}:latest" in names:
            print(f"OK ({MODEL} loaded)")
        else:
            print(f"FAIL: {MODEL} not found. Available: {names}")
            sys.exit(1)
    except Exception as e:
        print(f"FAIL ({e})")
        sys.exit(1)

    # Preflight: Postgres
    print("Checking Postgres...", end=" ")
    try:
        conn = psycopg2.connect(PG_DSN)
        cur = conn.cursor()
        cur.execute("SELECT count(*) FROM jsp_provisions WHERE text IS NOT NULL")
        total = cur.fetchone()[0]
        print(f"OK ({total} provisions)")
        cur.close()
        conn.close()
    except Exception as e:
        print(f"FAIL ({e})")
        print("Is the reverse SSH tunnel running? ssh -R 5433:localhost:5433 ...")
        sys.exit(1)

    # Load provisions needing RACI classification
    read_conn = psycopg2.connect(PG_DSN)
    read_cur = read_conn.cursor()

    where = "WHERE slm_raci IS NULL AND text IS NOT NULL"
    if args.source_id:
        where += f" AND source_id = '{args.source_id}'"
    limit = f"LIMIT {args.limit}" if args.limit else ""

    read_cur.execute(f"""
        SELECT section_id, source_id, text
        FROM jsp_provisions
        {where}
        ORDER BY source_id, position
        {limit}
    """)
    rows = read_cur.fetchall()
    read_cur.close()
    read_conn.close()

    print(f"Provisions to classify: {len(rows)}")
    if not rows:
        print("Nothing to classify.")
        return

    # Write connection (autocommit for save-as-you-go)
    write_conn = psycopg2.connect(PG_DSN)
    write_conn.autocommit = True
    write_cur = write_conn.cursor()

    processed = 0
    errors = 0
    t0 = time.time()

    def process_one(row):
        sid, src, text = row
        try:
            raci = classify_raci(text)
            with write_lock:
                write_cur.execute("""
                    UPDATE jsp_provisions
                    SET slm_raci = %s, slm_raci_at = NOW()
                    WHERE section_id = %s
                """, [json.dumps(raci), sid])
            return (sid, raci, None)
        except Exception as e:
            # Write empty array on parse failure so we don't retry
            with write_lock:
                write_cur.execute("""
                    UPDATE jsp_provisions
                    SET slm_raci = '[]'::jsonb, slm_raci_at = NOW()
                    WHERE section_id = %s
                """, [sid])
            return (sid, None, str(e))

    if args.workers > 1:
        with ThreadPoolExecutor(max_workers=args.workers) as executor:
            futures = {executor.submit(process_one, row): row for row in rows}
            for i, future in enumerate(as_completed(futures), 1):
                result = future.result()
                if result[2]:
                    errors += 1
                else:
                    processed += 1
                if i % 100 == 0:
                    elapsed = time.time() - t0
                    rate = i / elapsed
                    print(f"  {i}/{len(rows)} ({rate:.1f}/s, {errors} errors)")
    else:
        for i, row in enumerate(rows, 1):
            result = process_one(row)
            if result[2]:
                errors += 1
                if errors <= 3:
                    print(f"  Error: {result[2][:100]}")
            else:
                processed += 1
            if i % 100 == 0:
                elapsed = time.time() - t0
                rate = i / elapsed
                print(f"  {i}/{len(rows)} ({rate:.1f}/s, {errors} errors)")

    elapsed = time.time() - t0
    write_cur.close()
    write_conn.close()

    print(f"\nDone: {processed} classified, {errors} errors in {elapsed:.0f}s")

    # Stats
    conn = psycopg2.connect(PG_DSN)
    cur = conn.cursor()
    cur.execute("SELECT count(*) FROM jsp_provisions WHERE slm_raci IS NOT NULL")
    total_done = cur.fetchone()[0]
    cur.execute("SELECT count(*) FROM jsp_provisions WHERE slm_raci IS NULL AND text IS NOT NULL")
    remaining = cur.fetchone()[0]
    cur.close()
    conn.close()
    print(f"Total classified: {total_done}, Remaining: {remaining}")


if __name__ == "__main__":
    main()
