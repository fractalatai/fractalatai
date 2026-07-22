#!/usr/bin/env python3
"""Batch control title generation via Ollama on RunPod.

Runs ON THE POD. Reads/writes fractalaw Postgres via reverse SSH tunnel.

Setup on RunPod:
    ollama create gemma3-control-titles -f /workspace/control-titles/Modelfile
    python3 -u /workspace/control-titles/runpod_control_titles_batch.py --limit 10
    python3 -u /workspace/control-titles/runpod_control_titles_batch.py --workers 4

Writes to: jsp_provisions.slm_control_title (JSONB)
"""

import argparse
import json
import sys
import time
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed

import psycopg2
import urllib.request

OLLAMA_URL = "http://localhost:11434/api/chat"
MODEL = "gemma3-control-titles"
PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

SYSTEM_PROMPT = """You are a compliance controls architect. Generate a control specification from a JSP obligation.

The control must be in indicative mood — a statement that is observably true or false.
Not "must ensure" but "is ensured." Not "shall provide" but "is provided."

Output JSON:
{"title": "indicative-mood statement verifiable on site", "description": "what reality this stands for", "what_it_checks": "what would look different if this control failed"}"""

write_lock = threading.Lock()


def generate_title(text):
    payload = json.dumps({
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": f"Obligation: {text}"},
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
    parser = argparse.ArgumentParser()
    parser.add_argument("--limit", type=int)
    parser.add_argument("--workers", type=int, default=1)
    args = parser.parse_args()

    # Preflight
    try:
        resp = urllib.request.urlopen("http://localhost:11434/api/tags", timeout=5)
        models = [m["name"] for m in json.load(resp).get("models", [])]
        assert MODEL in models or f"{MODEL}:latest" in models, f"{MODEL} not loaded"
        print(f"Ollama: {MODEL} ready")
    except Exception as e:
        print(f"Ollama FAIL: {e}"); sys.exit(1)

    # Load mandatory provisions needing control titles
    read_conn = psycopg2.connect(PG_DSN)
    cur = read_conn.cursor()

    where = "WHERE slm_control_title IS NULL AND text IS NOT NULL"
    # Only mandatory obligations get controls
    # We use the embedding + classifier result if available, else text heuristic
    artefact_keywords = "risk assessment|safety case|hazard log|permit to work|emergency plan|method statement|training|inspection|audit|occurrence"
    where += f" AND text ~* '({artefact_keywords})'"
    limit = f"LIMIT {args.limit}" if args.limit else ""

    cur.execute(f"SELECT section_id, source_id, text FROM jsp_provisions {where} ORDER BY source_id {limit}")
    rows = cur.fetchall()
    cur.close()
    read_conn.close()

    print(f"Provisions to generate titles for: {len(rows)}")
    if not rows:
        return

    write_conn = psycopg2.connect(PG_DSN)
    write_conn.autocommit = True
    write_cur = write_conn.cursor()

    processed = 0
    errors = 0
    t0 = time.time()

    def process_one(row):
        sid, src, text = row
        try:
            result = generate_title(text)
            with write_lock:
                write_cur.execute("""
                    UPDATE jsp_provisions
                    SET slm_control_title = %s, slm_control_title_at = NOW()
                    WHERE section_id = %s
                """, [json.dumps(result), sid])
            return (sid, result, None)
        except Exception as e:
            with write_lock:
                write_cur.execute("""
                    UPDATE jsp_provisions
                    SET slm_control_title = '{}'::jsonb, slm_control_title_at = NOW()
                    WHERE section_id = %s
                """, [sid])
            return (sid, None, str(e))

    if args.workers > 1:
        with ThreadPoolExecutor(max_workers=args.workers) as executor:
            futures = {executor.submit(process_one, row): row for row in rows}
            for i, future in enumerate(as_completed(futures), 1):
                r = future.result()
                if r[2]: errors += 1
                else: processed += 1
                if i % 100 == 0:
                    print(f"  {i}/{len(rows)} ({i/(time.time()-t0):.1f}/s, {errors} errors)")
    else:
        for i, row in enumerate(rows, 1):
            r = process_one(row)
            if r[2]: errors += 1
            else: processed += 1
            if i % 100 == 0:
                print(f"  {i}/{len(rows)} ({i/(time.time()-t0):.1f}/s, {errors} errors)")

    write_cur.close()
    write_conn.close()
    print(f"\nDone: {processed} generated, {errors} errors in {time.time()-t0:.0f}s")


if __name__ == "__main__":
    main()
