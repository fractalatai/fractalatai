#!/usr/bin/env python3
"""Batch significance rating of Obligation provisions via fine-tuned SLM on RunPod.

Connects to Postgres (via SSH tunnel), queries Obligation provisions,
classifies via local Ollama gemma3-significance, writes results back.

Prerequisites on RunPod:
    1. Ollama installed + serving
    2. Significance GGUF loaded: ollama create gemma3-significance -f Modelfile
    3. SSH reverse tunnel: ssh -R 5433:localhost:5433 -p <PORT> root@<IP> -N

Usage:
    python3 /workspace/runpod_significance_batch.py --test
    python3 /workspace/runpod_significance_batch.py --workers 4
"""

import argparse
import json
import math
import sys
import time
import threading
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed

import psycopg2
import requests

# ── Config ──────────────────────────────────────────────────────────────

OLLAMA_URL = "http://localhost:11434/api/chat"
MODEL = "gemma3-significance"
PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

SYSTEM_PROMPT = (
    "You are rating the significance of a UK statutory provision that creates a legal obligation.\n\n"
    "Rate on 4 dimensions, each HIGH / MEDIUM / LOW:\n\n"
    "1. scope_duty_bearer — breadth of who bears the duty.\n"
    "   HIGH: universal ('every employer', 'any person')\n"
    "   MEDIUM: categorical ('an employer who operates...', 'a competent person')\n"
    "   LOW: individual/specific ('the person', 'an inspector')\n\n"
    "2. scope_protected_class — breadth of who is protected.\n"
    "   HIGH: universal ('all employees', 'persons', 'the public')\n"
    "   MEDIUM: categorical ('employees in that workplace', 'young persons')\n"
    "   LOW: specific ('the document', 'the premises')\n\n"
    "3. gravity — what is at stake if breached.\n"
    "   HIGH: health, safety, life, welfare, serious environmental harm\n"
    "   MEDIUM: property, financial, moderate environmental impact\n"
    "   LOW: administrative, procedural, record-keeping, notification\n\n"
    "4. strength — how absolute is the obligation.\n"
    "   HIGH: absolute unqualified duty ('shall ensure' with no qualification)\n"
    "   MEDIUM: qualified ('SFARP', 'all reasonable steps', 'have regard to')\n"
    "   LOW: procedural ('shall notify', 'shall keep records', 'shall display')\n\n"
    'Respond with ONLY a JSON object:\n'
    '{"scope_duty_bearer": "HIGH"|"MEDIUM"|"LOW", "scope_protected_class": "HIGH"|"MEDIUM"|"LOW", '
    '"gravity": "HIGH"|"MEDIUM"|"LOW", "strength": "HIGH"|"MEDIUM"|"LOW"}'
)

DIMS = ["scope_duty_bearer", "scope_protected_class", "gravity", "strength"]
VALID = {"HIGH", "MEDIUM", "LOW"}

# ── Preflight ───────────────────────────────────────────────────────────

def preflight():
    print("=" * 60)
    print("PREFLIGHT CHECKS")
    print("=" * 60)
    ok = True

    print("[1/3] Ollama: ", end="", flush=True)
    try:
        resp = requests.get("http://localhost:11434/api/tags", timeout=5)
        models = [m["name"] for m in resp.json().get("models", [])]
        if any(m.startswith(MODEL) for m in models):
            print(f"OK ({MODEL} loaded)")
        else:
            print(f"FAIL ({MODEL} not found, available: {models})")
            ok = False
    except Exception as e:
        print(f"FAIL ({e})")
        ok = False

    print("[2/3] Postgres: ", end="", flush=True)
    try:
        conn = psycopg2.connect(PG_DSN)
        cur = conn.cursor()
        cur.execute(
            "SELECT count(DISTINCT pa.section_id) FROM provision_actors pa "
            "JOIN legislation_text lt ON pa.section_id = lt.section_id "
            "WHERE (pa.slm_drrp = 'Obligation' OR pa.regex_drrp = 'Obligation') "
            "AND lt.scope = 'substantive'"
        )
        count = cur.fetchone()[0]
        cur.close()
        conn.close()
        print(f"OK ({count:,} Obligation provisions)")
    except Exception as e:
        print(f"FAIL ({e})")
        ok = False

    print("[3/3] GPU: ", end="", flush=True)
    try:
        import torch
        if torch.cuda.is_available():
            print(f"OK ({torch.cuda.get_device_name(0)})")
        else:
            print("WARNING (no GPU)")
    except ImportError:
        print("SKIP (torch not installed)")

    print("=" * 60)
    if ok:
        print("ALL CHECKS PASSED")
    else:
        print("PREFLIGHT FAILED")
    print("=" * 60)
    return ok, count if ok else 0

# ── Query ───────────────────────────────────────────────────────────────

def query_obligation_provisions(conn, limit=None):
    """Fetch unique Obligation provisions needing significance rating."""
    sql = """
        SELECT DISTINCT ON (pa.section_id)
            pa.section_id, lt.text, lt.section_type
        FROM provision_actors pa
        JOIN legislation_text lt ON pa.section_id = lt.section_id
        WHERE (pa.slm_drrp = 'Obligation' OR pa.regex_drrp = 'Obligation')
        AND lt.scope = 'substantive'
        ORDER BY pa.section_id
    """
    if limit:
        sql = f"SELECT * FROM ({sql}) sub LIMIT {limit}"
    cur = conn.cursor()
    cur.execute(sql)
    rows = cur.fetchall()
    cur.close()
    return rows

# ── Classify ────────────────────────────────────────────────────────────

def rate_provision(text, section_id, section_type, timeout=60):
    user_msg = (
        f"Provision ({section_id}, type={section_type}): {text}\n\n"
        f"Rate the significance of this obligation."
    )
    body = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_msg},
        ],
        "stream": False,
        "logprobs": True,
        "top_logprobs": 1,
        "options": {"temperature": 0.0},
    }
    try:
        resp = requests.post(OLLAMA_URL, json=body, timeout=timeout)
        resp.raise_for_status()
        resp_json = resp.json()
        content = resp_json.get("message", {}).get("content", "").strip()

        # Confidence from logprobs
        token_logprobs = resp_json.get("logprobs", [])
        if token_logprobs:
            avg_logprob = sum(t.get("logprob", 0) for t in token_logprobs) / len(token_logprobs)
            confidence = math.exp(avg_logprob)
        else:
            confidence = None

        if "```json" in content:
            content = content.split("```json")[1].split("```")[0].strip()
        elif "```" in content:
            content = content.split("```")[1].split("```")[0].strip()

        parsed = json.loads(content)
        result = {}
        for d in DIMS:
            v = parsed.get(d, "").upper()
            if v not in VALID:
                return None
            result[d] = v
        result["confidence"] = confidence
        return result
    except (requests.RequestException, json.JSONDecodeError, KeyError):
        return None

# ── Worker ──────────────────────────────────────────────────────────────

lock = threading.Lock()
stats = {"classified": 0, "errors": 0, "done": 0}
dim_counts = {d: Counter() for d in DIMS}

def process_provision(prov):
    sid, text, stype = prov
    result = rate_provision(text, sid, stype)

    with lock:
        stats["done"] += 1
        if result:
            stats["classified"] += 1
            for d in DIMS:
                dim_counts[d][result[d]] += 1
            return (sid, result)
        else:
            stats["errors"] += 1
            return None

# ── Write back ──────────────────────────────────────────────────────────

def write_batch(conn, updates):
    if not updates:
        return
    cur = conn.cursor()
    for sid, rating in updates:
        cur.execute(
            "UPDATE legislation_text SET "
            "significance_scope_duty_bearer = %s, "
            "significance_scope_protected_class = %s, "
            "significance_gravity = %s, "
            "significance_strength = %s, "
            "significance_confidence = %s "
            "WHERE section_id = %s",
            (
                rating["scope_duty_bearer"],
                rating["scope_protected_class"],
                rating["gravity"],
                rating["strength"],
                rating.get("confidence"),
                sid,
            )
        )
    conn.commit()
    cur.close()

# ── Main ────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Batch significance rating on RunPod")
    parser.add_argument("--test", action="store_true", help="5%% sample")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--batch-size", type=int, default=100)
    args = parser.parse_args()

    ok, total_count = preflight()
    if not ok:
        sys.exit(1)

    limit = args.limit
    if args.test:
        limit = max(1, total_count // 20)
        print(f"\nTEST MODE: {limit:,} of {total_count:,} provisions (5%)")

    conn = psycopg2.connect(PG_DSN)
    provisions = query_obligation_provisions(conn, limit=limit)
    total = len(provisions)
    print(f"Loaded {total:,} Obligation provisions, {args.workers} workers\n", flush=True)

    if args.dry_run:
        for sid, text, stype in provisions[:5]:
            print(f"  {sid} ({stype}) | {text[:100]}...")
        if total > 5:
            print(f"  ... and {total - 5} more")
        conn.close()
        return

    t0 = time.time()
    pending_updates = []

    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = {executor.submit(process_provision, p): p for p in provisions}

        for future in as_completed(futures):
            result = future.result()
            if result:
                pending_updates.append(result)

            if len(pending_updates) >= args.batch_size:
                write_batch(conn, pending_updates)
                pending_updates = []

            done = stats["done"]
            if done % 100 == 0 and done > 0:
                elapsed = time.time() - t0
                rate = done / elapsed
                eta = (total - done) / rate if rate > 0 else 0
                print(f"  [{done:,}/{total:,}] {stats['classified']:,} rated, "
                      f"{stats['errors']} errors, {rate:.1f}/s, ETA {eta/60:.0f}m", flush=True)

    write_batch(conn, pending_updates)

    elapsed = time.time() - t0
    print(f"\n{'=' * 60}")
    print(f"Significance Rating Complete")
    print(f"{'=' * 60}")
    print(f"Total:      {total:,}")
    print(f"Rated:      {stats['classified']:,}")
    print(f"Errors:     {stats['errors']}")
    print(f"Workers:    {args.workers}")
    print(f"Time:       {elapsed/60:.1f} min ({total/elapsed:.1f}/s)")

    for d in DIMS:
        print(f"\n{d}:")
        for level in ["HIGH", "MEDIUM", "LOW"]:
            print(f"  {level:8s}: {dim_counts[d].get(level, 0):,}")

    conn.close()


if __name__ == "__main__":
    main()
