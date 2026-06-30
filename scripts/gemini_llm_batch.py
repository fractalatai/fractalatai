#!/usr/bin/env python3
"""Batch LLM classification of pending_llm actors via Gemini Flash.

Queries provision_actors for pending_llm actors, sends to Gemini,
writes llm_drrp/llm_position back.

Usage:
    GEMINI_API_KEY=... /usr/bin/python3 scripts/gemini_llm_batch.py
    GEMINI_API_KEY=... /usr/bin/python3 scripts/gemini_llm_batch.py --dry-run
    GEMINI_API_KEY=... /usr/bin/python3 scripts/gemini_llm_batch.py --limit 10
"""

import argparse
import json
import os
import sys
import time
from collections import Counter

import psycopg2
import requests

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"
GEMINI_MODEL = "gemini-2.5-flash"

SYSTEM_PROMPT = (
    "You are a UK statutory law classifier. Given a provision from UK legislation "
    "and an actor mentioned in it, classify:\n\n"
    "1. The DRRP type of the provision for this actor:\n"
    "- Obligation: The provision imposes a duty or prohibition.\n"
    "- Liberty: The provision grants a power, permission, or entitlement.\n"
    "- none: No obligation or liberty is created for this actor.\n\n"
    "2. The actor's Hohfeldian legal position:\n"
    "- active: Bears the duty or exercises the power/liberty.\n"
    "- counterparty: To whom the duty is owed or subject to the power.\n"
    "- beneficiary: Benefits but is neither duty-bearer nor direct correlative.\n"
    "- mentioned: Referenced but no active legal role.\n\n"
    'Respond with ONLY a JSON object: {"drrp": "Obligation"|"Liberty"|"none", '
    '"position": "active"|"counterparty"|"beneficiary"|"mentioned"}'
)

VALID_POSITIONS = {"active", "counterparty", "beneficiary", "mentioned"}
VALID_DRRP = {"Obligation", "Liberty", "none"}


def query_pending_llm(conn, limit=None):
    cur = conn.cursor()
    sql = """
        SELECT pa.section_id, pa.actor_label, pa.actor_category, pa.regex_drrp, lt.text
        FROM provision_actors pa
        JOIN legislation_text lt ON pa.section_id = lt.section_id
        WHERE pa.extraction_method = 'pending_llm'
        AND pa.llm_position IS NULL
        AND lt.law_name NOT IN (SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks)
        ORDER BY pa.section_id, pa.actor_label
    """
    if limit:
        sql += f" LIMIT {limit}"
    cur.execute(sql)
    rows = cur.fetchall()
    cur.close()
    return rows


def classify_actor(api_key, text, actor_label):
    url = f"https://generativelanguage.googleapis.com/v1beta/models/{GEMINI_MODEL}:generateContent?key={api_key}"
    user_msg = (
        f"Provision: {text}\n\n"
        f"Actor: {actor_label}\n\n"
        f"Classify this actor's DRRP type and Hohfeldian position."
    )
    body = {
        "contents": [
            {"role": "user", "parts": [{"text": SYSTEM_PROMPT + "\n\n" + user_msg}]}
        ],
        "generationConfig": {
            "temperature": 0.1,
            "maxOutputTokens": 100,
            "thinkingConfig": {"thinkingBudget": 0}
        }
    }
    try:
        resp = requests.post(url, json=body, timeout=30)
        resp.raise_for_status()
        content = resp.json()
        text_resp = content.get("candidates", [{}])[0].get("content", {}).get("parts", [{}])[0].get("text", "").strip()

        if "```json" in text_resp:
            text_resp = text_resp.split("```json")[1].split("```")[0].strip()
        elif "```" in text_resp:
            text_resp = text_resp.split("```")[1].split("```")[0].strip()

        parsed = json.loads(text_resp)
        position = parsed.get("position", "").lower().strip()
        drrp = parsed.get("drrp", "none").strip()

        if drrp.lower() == "obligation":
            drrp = "Obligation"
        elif drrp.lower() == "liberty":
            drrp = "Liberty"
        else:
            drrp = "none"

        if position in VALID_POSITIONS:
            return (drrp, position)
        return None
    except json.JSONDecodeError as e:
        print(f"  PARSE ERROR: {e} — response: [{text_resp[:200]}]", file=sys.stderr)
        return None
    except Exception as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return None


def write_batch(conn, updates):
    if not updates:
        return
    cur = conn.cursor()
    for sid, label, drrp, position in updates:
        cur.execute(
            "UPDATE provision_actors SET llm_drrp = %s, llm_position = %s "
            "WHERE section_id = %s AND actor_label = %s",
            (drrp, position, sid, label)
        )
    conn.commit()
    cur.close()


def main():
    parser = argparse.ArgumentParser(description="Gemini LLM batch classification")
    parser.add_argument("--limit", type=int, help="Limit number of actors")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    api_key = os.environ.get("GEMINI_API_KEY") or ""
    if not api_key:
        # Try .bashrc
        import subprocess
        result = subprocess.run(["bash", "-c", "source ~/.bashrc && echo $GEMINI_API_KEY"],
                                capture_output=True, text=True)
        api_key = result.stdout.strip()
    if not api_key:
        print("ERROR: GEMINI_API_KEY not set")
        sys.exit(1)

    conn = psycopg2.connect(PG_DSN)
    actors = query_pending_llm(conn, limit=args.limit)
    print(f"Loaded {len(actors):,} pending_llm actors")

    if args.dry_run:
        for sid, label, cat, drrp, text in actors[:5]:
            print(f"  {sid} | {label} | {text[:100]}...")
        if len(actors) > 5:
            print(f"  ... and {len(actors) - 5} more")
        conn.close()
        return

    classified = 0
    errors = 0
    updates = []
    pos_counts = Counter()
    drrp_counts = Counter()
    t0 = time.time()

    for i, (sid, label, category, regex_drrp, text) in enumerate(actors):
        result = classify_actor(api_key, text, label)

        if result:
            drrp, position = result
            updates.append((sid, label, drrp, position))
            classified += 1
            pos_counts[position] += 1
            drrp_counts[drrp] += 1
        else:
            errors += 1

        # Write every 50
        if len(updates) >= 50:
            write_batch(conn, updates)
            updates = []

        if (i + 1) % 50 == 0:
            elapsed = time.time() - t0
            rate = (i + 1) / elapsed
            eta = (len(actors) - i - 1) / rate if rate > 0 else 0
            print(f"  [{i+1:,}/{len(actors):,}] {classified:,} classified, "
                  f"{errors} errors, {rate:.1f}/s, ETA {eta/60:.0f}m")

        # Rate limit: ~10 requests/s for Flash
        time.sleep(0.1)

    if updates:
        write_batch(conn, updates)

    elapsed = time.time() - t0
    print(f"\n{'=' * 60}")
    print(f"Gemini LLM Batch Complete")
    print(f"{'=' * 60}")
    print(f"Total:      {len(actors):,}")
    print(f"Classified: {classified:,}")
    print(f"Errors:     {errors}")
    print(f"Time:       {elapsed/60:.1f} min")
    print(f"\nPer-position:")
    for pos in ["active", "counterparty", "beneficiary", "mentioned"]:
        print(f"  {pos:15s}: {pos_counts.get(pos, 0):,}")
    print(f"\nPer-DRRP:")
    for d in ["Obligation", "Liberty", "none"]:
        print(f"  {d:15s}: {drrp_counts.get(d, 0):,}")

    conn.close()


if __name__ == "__main__":
    main()
