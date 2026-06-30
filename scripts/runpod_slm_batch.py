#!/usr/bin/env python3
"""Batch SLM classification on RunPod GPU via Ollama (concurrent).

Connects to Postgres (via SSH tunnel), queries pending_slm actors,
classifies via local Ollama gemma3-position with concurrent workers,
writes slm_drrp/slm_position back.

Checkpoint/resume: queries only actors with slm_position IS NULL.
If the script crashes, re-run picks up where it left off.

Prerequisites on RunPod:
    1. Ollama installed: curl -fsSL https://ollama.com/install.sh | sh
    2. GGUF model loaded: ollama create gemma3-position -f Modelfile
    3. SSH tunnel from LOCAL machine:
       ssh -R 5433:localhost:5433 -p <PORT> -i ~/.ssh/id_ed25519 root@<IP>
       (reverse tunnel: pod's localhost:5433 → local Postgres)

Usage:
    python3 /workspace/runpod_slm_batch.py --test          # 5% sample (~1,275 actors)
    python3 /workspace/runpod_slm_batch.py                 # full run
    python3 /workspace/runpod_slm_batch.py --workers 8     # more concurrency
    python3 /workspace/runpod_slm_batch.py --dry-run       # query only
"""

import argparse
import json
import sys
import time
import threading
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed

import psycopg2
import psycopg2.pool
import requests

# ── Config ──────────────────────────────────────────────────────────────

OLLAMA_URL = "http://localhost:11434/api/chat"
MODEL = "gemma3-position"
PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

SYSTEM_PROMPT = (
    "You are a UK statutory law classifier. Given a provision from UK legislation "
    "and an actor mentioned in it, classify:\n\n"
    "1. The DRRP type of the provision for this actor:\n"
    "- Obligation: The provision imposes a duty or prohibition on the actor. "
    "Language: 'shall', 'shall not', 'must', 'must not', 'is required to', "
    "'has a duty', 'ensure', 'responsible for', 'so far as is reasonably practicable'.\n"
    "- Liberty: The provision grants a power, permission, or entitlement to the actor. "
    "Language: 'may', 'may not' (limiting a power), 'power to', 'entitled to', "
    "'authorise', 'enable', 'has the right'.\n"
    "- none: The provision does not create an obligation or liberty for this actor.\n\n"
    "2. The actor's Hohfeldian legal position in this provision:\n"
    "- active: The actor bears the duty or exercises the power/liberty.\n"
    "- counterparty: The actor to whom the duty is owed or who is subject to the power.\n"
    "- beneficiary: The actor benefits but is neither duty-bearer nor direct correlative.\n"
    "- mentioned: The actor is referenced but has no active legal role.\n\n"
    "Note: An actor can be 'active' with drrp 'none' — e.g. in an offence provision "
    "('A person who contravenes... is guilty') the actor is active but no new duty is created.\n\n"
    'Respond with ONLY a JSON object: {"drrp": "Obligation"|"Liberty"|"none", '
    '"position": "active"|"counterparty"|"beneficiary"|"mentioned"}'
)

VALID_POSITIONS = {"active", "counterparty", "beneficiary", "mentioned"}
VALID_DRRP = {"Obligation", "Liberty", "none"}

# ── Preflight ───────────────────────────────────────────────────────────

def preflight():
    print("=" * 60)
    print("PREFLIGHT CHECKS")
    print("=" * 60)
    ok = True

    print("[1/3] Ollama: ", end="")
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

    print("[2/3] Postgres: ", end="")
    try:
        conn = psycopg2.connect(PG_DSN)
        cur = conn.cursor()
        cur.execute(
            "SELECT count(*) FROM provision_actors pa "
            "JOIN legislation_text lt ON pa.section_id = lt.section_id "
            "WHERE pa.slm_position IS NULL AND pa.regex_position IS NOT NULL "
            "AND lt.scope = 'substantive'"
        )
        count = cur.fetchone()[0]
        cur.close()
        conn.close()
        print(f"OK ({count:,} pending_slm actors remaining)")
    except Exception as e:
        print(f"FAIL ({e})")
        ok = False

    print("[3/3] GPU: ", end="")
    try:
        import torch
        if torch.cuda.is_available():
            print(f"OK ({torch.cuda.get_device_name(0)})")
        else:
            print("WARNING (no GPU — Ollama will use CPU)")
    except ImportError:
        print("SKIP (torch not installed — Ollama handles GPU)")

    print("=" * 60)
    if ok:
        print("ALL CHECKS PASSED")
    else:
        print("PREFLIGHT FAILED")
    print("=" * 60)
    return ok, count if ok else 0

# ── Query ───────────────────────────────────────────────────────────────

def query_pending_slm(conn, limit=None):
    """Fetch actors needing SLM classification (all actors without slm_position on substantive provisions)."""
    sql = """
        SELECT pa.section_id, pa.actor_label, pa.actor_category, pa.regex_drrp, lt.text
        FROM provision_actors pa
        JOIN legislation_text lt ON pa.section_id = lt.section_id
        WHERE pa.slm_position IS NULL
        AND pa.regex_position IS NOT NULL
        AND lt.scope = 'substantive'
        ORDER BY pa.section_id, pa.actor_label
    """
    if limit:
        sql += f" LIMIT {limit}"
    cur = conn.cursor()
    cur.execute(sql)
    rows = cur.fetchall()
    cur.close()
    return rows

# ── Classify ────────────────────────────────────────────────────────────

def classify_actor(text, actor_label, timeout=60):
    """Send a single (provision, actor) to Ollama and parse the dual DRRP+position response."""
    user_msg = (
        f"Provision: {text}\n\n"
        f"Actor: {actor_label}\n\n"
        f"Classify this actor's DRRP type and Hohfeldian position in this provision."
    )
    body = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_msg},
        ],
        "stream": False,
        "options": {"temperature": 0.0},
    }
    try:
        resp = requests.post(OLLAMA_URL, json=body, timeout=timeout)
        resp.raise_for_status()
        content = resp.json().get("message", {}).get("content", "").strip()

        if "```json" in content:
            content = content.split("```json")[1].split("```")[0].strip()
        elif "```" in content:
            content = content.split("```")[1].split("```")[0].strip()

        parsed = json.loads(content)
        position = parsed.get("position", "").lower().strip()
        drrp = parsed.get("drrp", "none").strip()
        # Normalise DRRP capitalisation
        if drrp.lower() == "obligation":
            drrp = "Obligation"
        elif drrp.lower() == "liberty":
            drrp = "Liberty"
        else:
            drrp = "none"
        if position in VALID_POSITIONS:
            return (drrp, position)
        return None
    except (requests.RequestException, json.JSONDecodeError, KeyError, IndexError):
        return None

# ── Worker ──────────────────────────────────────────────────────────────

# Thread-safe counters
lock = threading.Lock()
stats = {"classified": 0, "errors": 0, "done": 0}
class_counts = Counter()
drrp_counts = Counter()

def process_actor(actor):
    """Classify one actor and return the update tuple (or None on error)."""
    sid, label, category, regex_drrp, text = actor
    result = classify_actor(text, label)

    with lock:
        stats["done"] += 1
        if result:
            drrp, position = result
            stats["classified"] += 1
            class_counts[position] += 1
            drrp_counts[drrp] += 1
            return (sid, label, drrp, position)
        else:
            stats["errors"] += 1
            return None

# ── Write back ──────────────────────────────────────────────────────────

def write_batch(conn, updates):
    """Write slm_drrp and slm_position to provision_actors."""
    if not updates:
        return
    cur = conn.cursor()
    for sid, label, drrp, position in updates:
        cur.execute(
            "UPDATE provision_actors SET slm_drrp = %s, slm_position = %s "
            "WHERE section_id = %s AND actor_label = %s "
            "AND (slm_position IS NULL OR slm_position != %s OR slm_drrp IS NULL OR slm_drrp != %s)",
            (drrp, position, sid, label, position, drrp)
        )
    conn.commit()
    cur.close()

# ── Main ────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Batch SLM classification on RunPod")
    parser.add_argument("--test", action="store_true",
                        help="Test mode: process 5%% sample (~1,275 actors)")
    parser.add_argument("--limit", type=int, help="Limit number of actors")
    parser.add_argument("--dry-run", action="store_true", help="Query only, don't classify")
    parser.add_argument("--workers", type=int, default=4,
                        help="Number of concurrent Ollama workers (default: 4)")
    parser.add_argument("--batch-size", type=int, default=100,
                        help="Write to Postgres every N results (default: 100)")
    args = parser.parse_args()

    ok, pending_count = preflight()
    if not ok:
        sys.exit(1)

    limit = args.limit
    if args.test:
        limit = max(1, pending_count // 20)  # 5%
        print(f"\nTEST MODE: processing {limit:,} of {pending_count:,} actors (5%)")

    conn = psycopg2.connect(PG_DSN)
    actors = query_pending_slm(conn, limit=limit)
    total = len(actors)
    print(f"Loaded {total:,} pending_slm actors, {args.workers} workers\n")

    if args.dry_run:
        for sid, label, cat, drrp, text in actors[:5]:
            print(f"  {sid} | {label} | {text[:100]}...")
        if total > 5:
            print(f"  ... and {total - 5} more")
        conn.close()
        return

    t0 = time.time()
    pending_updates = []

    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = {executor.submit(process_actor, actor): actor for actor in actors}

        for future in as_completed(futures):
            result = future.result()
            if result:
                pending_updates.append(result)

            # Write in batches
            if len(pending_updates) >= args.batch_size:
                write_batch(conn, pending_updates)
                pending_updates = []

            # Progress every 100
            done = stats["done"]
            if done % 100 == 0 and done > 0:
                elapsed = time.time() - t0
                rate = done / elapsed
                eta = (total - done) / rate if rate > 0 else 0
                print(f"  [{done:,}/{total:,}] {stats['classified']:,} classified, "
                      f"{stats['errors']} errors, {rate:.1f}/s, ETA {eta/60:.0f}m")

    # Write remaining
    write_batch(conn, pending_updates)

    elapsed = time.time() - t0
    print(f"\n{'=' * 60}")
    print(f"SLM Batch Complete")
    print(f"{'=' * 60}")
    print(f"Total:      {total:,}")
    print(f"Classified: {stats['classified']:,}")
    print(f"Errors:     {stats['errors']}")
    print(f"Workers:    {args.workers}")
    print(f"Time:       {elapsed/60:.1f} min ({total/elapsed:.1f}/s)")
    print(f"\nPer-position:")
    for pos in ["active", "counterparty", "beneficiary", "mentioned"]:
        print(f"  {pos:15s}: {class_counts.get(pos, 0):,}")
    print(f"\nPer-DRRP:")
    for d in ["Obligation", "Liberty", "none"]:
        print(f"  {d:15s}: {drrp_counts.get(d, 0):,}")

    conn.close()


if __name__ == "__main__":
    main()
