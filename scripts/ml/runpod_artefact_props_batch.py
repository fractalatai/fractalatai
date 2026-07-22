#!/usr/bin/env python3
"""Batch artefact property extraction via Ollama on RunPod.

Runs ON THE POD. Reads/writes fractalaw Postgres via reverse SSH tunnel.

Setup on RunPod:
    ollama create gemma3-artefact-props -f /workspace/artefact-props/Modelfile
    python3 -u /workspace/artefact-props/runpod_artefact_props_batch.py --limit 10
    python3 -u /workspace/artefact-props/runpod_artefact_props_batch.py --workers 4

Writes to: jsp_provisions.slm_artefact_props (JSONB)
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
MODEL = "gemma3-artefact-props"
PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

SYSTEM_PROMPT = """You are an artefact property extractor for MoD Joint Service Publications (JSPs).

Given an obligation text and the type of mandated artefact, extract structured properties:

{"owner_role": "who creates/maintains (or null)", "approver_role": "who approves (or null)", "reviewer_role": "who reviews (or null)", "review_frequency": "Annual/Quarterly/Monthly/Per-activity/On-change/Continuous (or null)", "required_content": ["what it must contain"], "acceptance_criterion": "test for adequacy (or null)", "scope": "what it covers (or null)"}

Extract only what is stated or clearly implied. Use null for properties not mentioned."""

write_lock = threading.Lock()


def extract_props(text, artefact_type):
    user_msg = f"Artefact type: {artefact_type}\n\nObligation: {text}"
    payload = json.dumps({
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_msg},
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

    # Load provisions with artefacts needing property extraction
    # Join with DuckDB artefact data? No — artefact types are in DuckDB, not PG.
    # We need artefact_type from DuckDB. Since we can't query DuckDB from the pod,
    # we match by text patterns (the artefact regex keywords).
    read_conn = psycopg2.connect(PG_DSN)
    cur = read_conn.cursor()

    where = "WHERE slm_artefact_props IS NULL AND text IS NOT NULL"
    # Only provisions that mention artefact keywords
    artefact_keywords = "risk assessment|safety case|hazard log|permit to work|emergency plan|method statement|training course|training record|inspection|audit|occurrence"
    where += f" AND text ~* '({artefact_keywords})'"
    limit = f"LIMIT {args.limit}" if args.limit else ""

    cur.execute(f"SELECT section_id, source_id, text FROM jsp_provisions {where} ORDER BY source_id {limit}")
    rows = cur.fetchall()
    cur.close()
    read_conn.close()

    print(f"Provisions to extract: {len(rows)}")
    if not rows:
        return

    # Detect artefact type from text
    import re
    artefact_patterns = [
        (re.compile(r'risk assessment', re.I), "Risk Assessment"),
        (re.compile(r'safety case', re.I), "Safety Case"),
        (re.compile(r'hazard log', re.I), "Hazard Log"),
        (re.compile(r'permit to work', re.I), "Permit"),
        (re.compile(r'emergency (?:plan|arrangement)', re.I), "Emergency Plan"),
        (re.compile(r'method statement|safe system of work', re.I), "Method Statement"),
        (re.compile(r'training (?:course|record)', re.I), "Training Record"),
        (re.compile(r'inspection', re.I), "Inspection Report"),
        (re.compile(r'audit', re.I), "Audit Report"),
        (re.compile(r'occurrence', re.I), "Occurrence Report"),
    ]

    def detect_type(text):
        for pat, atype in artefact_patterns:
            if pat.search(text):
                return atype
        return "Other"

    write_conn = psycopg2.connect(PG_DSN)
    write_conn.autocommit = True
    write_cur = write_conn.cursor()

    processed = 0
    errors = 0
    t0 = time.time()

    def process_one(row):
        sid, src, text = row
        atype = detect_type(text)
        try:
            props = extract_props(text, atype)
            props["artefact_type"] = atype
            with write_lock:
                write_cur.execute("""
                    UPDATE jsp_provisions
                    SET slm_artefact_props = %s, slm_artefact_props_at = NOW()
                    WHERE section_id = %s
                """, [json.dumps(props), sid])
            return (sid, props, None)
        except Exception as e:
            with write_lock:
                write_cur.execute("""
                    UPDATE jsp_provisions
                    SET slm_artefact_props = '{}'::jsonb, slm_artefact_props_at = NOW()
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
    print(f"\nDone: {processed} extracted, {errors} errors in {time.time()-t0:.0f}s")


if __name__ == "__main__":
    main()
