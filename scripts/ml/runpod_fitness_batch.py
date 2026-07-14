#!/usr/bin/env python3
"""Batch fitness entity extraction via SLM on RunPod (or local Ollama).

Connects to Postgres (via SSH tunnel on RunPod, or directly locally),
queries polarity-only provisions (no dictionary entities), extracts
applicability entities via gemma3:4b, writes to fitness_mentions.

Prerequisites on RunPod:
    1. Ollama installed + serving
    2. gemma3:4b loaded: ollama pull gemma3:4b
    3. SSH reverse tunnel: ssh -R 5433:localhost:5433 -p <PORT> root@<IP> -N
    4. OLLAMA_NUM_PARALLEL=4 set before ollama serve

Usage:
    python3 runpod_fitness_batch.py --test          # 5 provisions, verify output
    python3 runpod_fitness_batch.py --workers 4     # full batch
    python3 runpod_fitness_batch.py --limit 100     # first 100
"""

import argparse
import json
import sys
import time
import threading
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed

import psycopg2
import requests

# ── Config ──────────────────────────────────────────────────────────────

OLLAMA_URL = "http://localhost:11434/api/generate"
MODEL = "gemma3:4b"
PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

SYSTEM_PROMPT = """Extract applicability entities from this UK legislation provision. Return ONLY a JSON array.

The provision declares who or what the law applies to (or does not apply to). Extract the SUBJECT of the applicability as short canonical entities.

Classify each entity with one scope dimension:
- personal: who (actors, authorities, persons)
- material: what (activities, subjects, objects)
- territorial: where (places, zones, jurisdictions)
- conditional: qualifying criteria (thresholds, circumstances)

Return ONLY a JSON array like: [{"name": "employer", "scope": "personal"}, {"name": "construction work", "scope": "material"}]
If no clear applicability entities can be extracted, return: []"""

VALID_SCOPES = {"personal", "material", "territorial", "conditional", "temporal"}

# ── Stats ──────────────────────────────────────────────────────────────

stats_lock = threading.Lock()
stats = Counter()

# ── Preflight ──────────────────────────────────────────────────────────

def preflight():
    print("=" * 60)
    print("PREFLIGHT CHECKS — Fitness Entity Extraction")
    print("=" * 60)
    ok = True

    print("[1/3] Ollama: ", end="", flush=True)
    try:
        resp = requests.get("http://localhost:11434/api/tags", timeout=5)
        models = [m["name"] for m in resp.json().get("models", [])]
        if any(MODEL in m for m in models):
            print(f"OK ({MODEL} available)")
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
            "SELECT count(*) FROM fitness_mentions "
            "WHERE (ft_entities IS NULL OR ft_entities = '{}') "
        )
        count = cur.fetchone()[0]
        cur.close()
        conn.close()
        print(f"OK ({count:,} polarity-only mentions to process)")
    except Exception as e:
        print(f"FAIL ({e})")
        ok = False
        count = 0

    print("[3/3] GPU: ", end="", flush=True)
    try:
        import subprocess
        result = subprocess.run(
            ["nvidia-smi", "--query-gpu=name", "--format=csv,noheader"],
            capture_output=True, text=True, timeout=5,
        )
        if result.returncode == 0 and result.stdout.strip():
            print(f"OK ({result.stdout.strip()})")
        else:
            print("WARNING (no GPU — will be slow)")
    except Exception:
        print("SKIP (nvidia-smi not available)")

    print("=" * 60)
    if ok:
        print(f"ALL CHECKS PASSED — {count:,} provisions in queue")
    else:
        print("PREFLIGHT FAILED")
    print("=" * 60)
    return ok, count


# ── Query ──────────────────────────────────────────────────────────────

def query_gap_provisions(conn, limit=None, law_names=None):
    """Fetch mentions needing fine-tuned extraction (no ft_entities yet)."""
    sql = """
        SELECT fm.id, fm.section_id, fm.polarity, lt.text
        FROM fitness_mentions fm
        JOIN legislation_text lt ON fm.section_id = lt.section_id
        WHERE (fm.ft_entities IS NULL OR fm.ft_entities = '{}')
        AND fm.extraction_method != 'propagated'
        AND lt.text IS NOT NULL AND length(lt.text) > 20
    """
    if law_names:
        placeholders = ",".join(f"'{n}'" for n in law_names)
        sql += f" AND lt.law_name IN ({placeholders})"
    sql += " ORDER BY fm.id"
    if limit:
        sql += f" LIMIT {limit}"
    cur = conn.cursor()
    cur.execute(sql)
    rows = cur.fetchall()
    cur.close()
    return rows


# ── Extract ────────────────────────────────────────────────────────────

def extract_entities(mention_id, section_id, polarity, text, timeout=90):
    """Call SLM to extract fitness entities from a provision."""
    # Truncate very long provisions
    if len(text) > 1500:
        text = text[:1500] + "..."

    user_msg = f"Provision ({section_id}):\n\"{text}\"\nPolarity: {polarity}\n\nExtract applicability entities:"

    body = {
        "model": MODEL,
        "prompt": SYSTEM_PROMPT + "\n\n" + user_msg,
        "stream": False,
        "options": {"temperature": 0.1, "num_predict": 256},
    }

    try:
        resp = requests.post(OLLAMA_URL, json=body, timeout=timeout)
        resp.raise_for_status()
        resp_json = resp.json()
        content = resp_json.get("response", "").strip()

        # Parse JSON from response (handle markdown fences)
        if "```json" in content:
            content = content.split("```json")[1].split("```")[0].strip()
        elif "```" in content:
            content = content.split("```")[1].split("```")[0].strip()

        # Find the JSON array
        start = content.find("[")
        end = content.rfind("]")
        if start >= 0 and end > start:
            content = content[start:end + 1]

        parsed = json.loads(content)

        if not isinstance(parsed, list):
            with stats_lock:
                stats["parse_error"] += 1
            return mention_id, None, None, "not_array"

        # Validate and extract
        entities = []
        scopes = set()
        for item in parsed:
            if isinstance(item, dict) and "name" in item:
                name = item["name"].strip()
                scope = item.get("scope", "material").strip().lower()
                if scope not in VALID_SCOPES:
                    scope = "material"  # default
                if name and len(name) < 100:
                    entities.append(name)
                    scopes.add(scope)

        if not entities:
            with stats_lock:
                stats["empty_result"] += 1
            return mention_id, [], [], "empty"

        with stats_lock:
            stats["success"] += 1
        return mention_id, entities, sorted(scopes), "ok"

    except json.JSONDecodeError:
        with stats_lock:
            stats["json_error"] += 1
        return mention_id, None, None, "json_error"
    except requests.Timeout:
        with stats_lock:
            stats["timeout"] += 1
        return mention_id, None, None, "timeout"
    except Exception as e:
        with stats_lock:
            stats["error"] += 1
        return mention_id, None, None, f"error: {e}"


# ── Write ──────────────────────────────────────────────────────────────

def write_one(cur, mention_id, entities, scopes):
    """Write a single result to ft_* columns immediately."""
    cur.execute(
        """UPDATE fitness_mentions
           SET ft_entities = %s,
               ft_scope_dimensions = %s,
               ft_confidence = 0.85,
               source_detail = 'gemma3_fitness_finetuned',
               updated_at = now()
           WHERE id = %s""",
        (entities, scopes, mention_id),
    )


# ── Main ───────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Fitness entity extraction via SLM")
    parser.add_argument("--test", action="store_true", help="Process 5 provisions and show output")
    parser.add_argument("--limit", type=int, help="Max provisions to process")
    parser.add_argument("--workers", type=int, default=1, help="Parallel workers (match OLLAMA_NUM_PARALLEL)")
    parser.add_argument("--dry-run", action="store_true", help="Extract but don't write to DB")
    parser.add_argument("--laws", help="Comma-separated law names or path to a file (one per line) to scope extraction")
    args = parser.parse_args()

    if args.test:
        args.limit = 5
        args.workers = 1

    # Parse --laws: comma-separated string or file path (one per line)
    law_names = None
    if args.laws:
        import os
        if os.path.isfile(args.laws):
            with open(args.laws) as f:
                law_names = [line.strip() for line in f if line.strip()]
        else:
            law_names = [n.strip() for n in args.laws.split(",") if n.strip()]
        print(f"Scoped to {len(law_names)} laws")

    ok, total = preflight()
    if not ok:
        sys.exit(1)

    conn = psycopg2.connect(PG_DSN)
    rows = query_gap_provisions(conn, args.limit, law_names=law_names)
    print(f"\nLoaded {len(rows)} provisions for extraction")

    if not rows:
        print("Nothing to process.")
        return

    # Separate connection for writes (save-as-you-go)
    write_conn = psycopg2.connect(PG_DSN)
    write_conn.autocommit = True
    write_cur = write_conn.cursor()

    start_time = time.time()
    written = 0

    if args.workers <= 1:
        for i, (mid, sid, pol, text) in enumerate(rows):
            mention_id, entities, scopes, status = extract_entities(mid, sid, pol, text)
            if not args.dry_run and not args.test and entities and len(entities) > 0:
                write_one(write_cur, mention_id, entities, scopes)
                written += 1
            if args.test:
                print(f"\n  {sid} ({pol}):")
                print(f"    entities: {entities}")
                print(f"    scopes: {scopes}")
                print(f"    status: {status}")
            elif (i + 1) % 50 == 0:
                elapsed = time.time() - start_time
                rate = (i + 1) / elapsed
                eta = (len(rows) - i - 1) / rate if rate > 0 else 0
                print(f"  {i+1}/{len(rows)} ({rate:.1f}/s, ETA {eta/60:.0f}min) — {dict(stats)} written={written}", flush=True)
    else:
        write_lock = threading.Lock()
        with ThreadPoolExecutor(max_workers=args.workers) as executor:
            futures = {
                executor.submit(extract_entities, mid, sid, pol, text): mid
                for mid, sid, pol, text in rows
            }
            for i, future in enumerate(as_completed(futures)):
                mention_id, entities, scopes, status = future.result()
                if not args.dry_run and entities and len(entities) > 0:
                    with write_lock:
                        write_one(write_cur, mention_id, entities, scopes)
                        written += 1
                milestone = (i + 1) // 100
                if milestone > stats.get("_last_milestone", 0):
                    stats["_last_milestone"] = milestone
                    elapsed = time.time() - start_time
                    rate = (i + 1) / elapsed
                    eta = (len(rows) - i - 1) / rate if rate > 0 else 0
                    print(f"  {i+1}/{len(rows)} ({rate:.1f}/s, ETA {eta/60:.0f}min) — {dict(stats)} written={written}", flush=True)

    elapsed = time.time() - start_time
    print(f"\n{'=' * 60}")
    print(f"EXTRACTION COMPLETE")
    print(f"  Processed: {len(rows)}")
    print(f"  Written:   {written}")
    print(f"  Duration:  {elapsed:.0f}s ({len(rows)/elapsed:.1f} provisions/s)")
    print(f"  Stats:     {dict(stats)}")
    print(f"{'=' * 60}")

    write_cur.close()
    write_conn.close()
    conn.close()


if __name__ == "__main__":
    main()
