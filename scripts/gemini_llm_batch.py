#!/usr/bin/env python3
"""Batch LLM classification of pending_llm actors via Gemini Flash.

Queries provision_actors for pending_llm actors, sends to Gemini,
writes llm_drrp/llm_position back.

Usage:
    GEMINI_API_KEY=... /usr/bin/python3 scripts/gemini_llm_batch.py
    GEMINI_API_KEY=... /usr/bin/python3 scripts/gemini_llm_batch.py --dry-run
    GEMINI_API_KEY=... /usr/bin/python3 scripts/gemini_llm_batch.py --limit 10

Position correction (#72): re-label counterparty/beneficiary actors on live
provisions with an active Obligation holder at the agreed rule (the SLM was
trained on the old split). Sample first, nothing written:
    /usr/bin/python3 scripts/gemini_llm_batch.py --position-correction \
        --exclude-law-file revoked.txt --limit 50 --sample-out data/audit/poscorr_sample.tsv
"""

import argparse
import csv
import hashlib
import json
import os
import sys
import time
from collections import Counter

import psycopg2
import requests

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "ml"))
try:  # enrichment provenance (#63); on a pod, copy fractalaw_provenance.py alongside
    import fractalaw_provenance as provenance
except ImportError:
    provenance = None
    print("warning: fractalaw_provenance.py not found; enrichment provenance will not be recorded")
TOUCHED_LAWS = set()

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
    "- counterparty: the recipient of the duty's act: the party the act is done to or withheld from (notified, informed, sent or supplied something, consulted, paid, given access, served, charged, or whose request the duty answers). 'Ensure that X is provided with ...' makes X a counterparty. For a power or right: the party subject to it.\n"
    "- beneficiary: the party whose interest the duty protects without receiving its act, e.g. 'ensure the health, safety and welfare of his employees', 'persons not in his employment are not exposed to risks'.\n"
    "- mentioned: referenced with no role in the relation, e.g. a regulator merely named in another party's duty.\n"
    "If an actor is both the recipient and the protected party, it is counterparty.\n\n"
    "Offences and penalties are not obligations: a provision that makes something an "
    "offence, or says a person guilty of an offence is liable to a fine or imprisonment, "
    "is none (the person is mentioned). A provision that only references, conditions, "
    "details, defines or exempts a duty or power created elsewhere is also none.\n\n"
    'Respond with ONLY a JSON object: {"drrp": "Obligation"|"Liberty"|"none", '
    '"position": "active"|"counterparty"|"beneficiary"|"mentioned"}'
)

VALID_POSITIONS = {"active", "counterparty", "beneficiary", "mentioned"}
# Per-row prompt version (llm_prompt_version): which labels the current prompt made
PROMPT_VERSION = "sha:" + hashlib.sha256(SYSTEM_PROMPT.encode()).hexdigest()[:12]
VALID_DRRP = {"Obligation", "Liberty", "none"}


def read_law_file(path):
    """Law names from a CSV line or one-per-line file, as a comma string."""
    with open(path) as f:
        return ",".join(n.strip() for n in f.read().replace("\n", ",").split(",") if n.strip())


def query_pending_llm(conn, limit=None, significance=None, max_confidence=None, law_file=None,
                      position_correction=False, exclude_law_file=None):
    """Rows of (section_id, actor_label, actor_category, regex_drrp, text, position)."""
    cur = conn.cursor()
    params = []

    if position_correction:
        # #72: non-active actors that would carry a correlative (claim_right /
        # protected) against an active Obligation holder, on live substantive
        # provisions; benchmark laws out; skip rows already at this prompt
        sql = """
            SELECT pa.section_id, pa.actor_label, pa.actor_category, pa.regex_drrp, lt.text, pa.position
            FROM provision_actors pa
            JOIN legislation_text lt ON pa.section_id = lt.section_id
            WHERE pa.position IN ('counterparty', 'beneficiary')
            AND lt.scope = 'substantive'
            AND coalesce(lt.status, '') NOT IN ('repealed', 'prospective')
            AND NOT unapplied_repealed(lt.law_name, lt.section_id)
            AND EXISTS (SELECT 1 FROM provision_actors h WHERE h.section_id = pa.section_id
                        AND h.position = 'active' AND h.drrp = 'Obligation')
            AND pa.llm_prompt_version IS DISTINCT FROM %s
            AND lt.law_name NOT IN (SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks)
        """
        params = [PROMPT_VERSION]
        if law_file:
            sql += " AND lt.law_name IN (SELECT unnest(string_to_array(%s, ',')))"
            params.append(read_law_file(law_file))
        if exclude_law_file:
            sql += " AND lt.law_name NOT IN (SELECT unnest(string_to_array(%s, ',')))"
            params.append(read_law_file(exclude_law_file))
        # Random order so a --limit sample spreads across laws
        sql += " ORDER BY md5(pa.section_id || pa.actor_label)"
    elif significance and max_confidence:
        # Target: SLM-classified actors on provisions with specific significance and low confidence
        sql = """
            SELECT pa.section_id, pa.actor_label, pa.actor_category, pa.regex_drrp, lt.text, pa.position
            FROM provision_actors pa
            JOIN legislation_text lt ON pa.section_id = lt.section_id
            WHERE lt.significance_overall = %s
            AND pa.slm_confidence < %s
            AND pa.slm_position IS NOT NULL
            AND pa.llm_position IS NULL
            AND lt.law_name NOT IN (SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks)
        """
        params = [significance, max_confidence]
        if law_file:
            sql += " AND lt.law_name IN (SELECT unnest(string_to_array(%s, ',')))"
            params.append(read_law_file(law_file))
        sql += " ORDER BY pa.slm_confidence, pa.section_id"
    else:
        # Default: pending_llm actors
        sql = """
            SELECT pa.section_id, pa.actor_label, pa.actor_category, pa.regex_drrp, lt.text, pa.position
            FROM provision_actors pa
            JOIN legislation_text lt ON pa.section_id = lt.section_id
            WHERE pa.extraction_method = 'pending_llm'
            AND pa.llm_position IS NULL
            AND lt.law_name NOT IN (SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks)
        """
        if law_file:
            sql += " AND lt.law_name IN (SELECT unnest(string_to_array(%s, ',')))"
            params.append(read_law_file(law_file))
        sql += " ORDER BY pa.section_id, pa.actor_label"

    if limit:
        sql += f" LIMIT {limit}"
    cur.execute(sql, params)
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
        # Concatenate all text parts (Gemini 2.5 may split across parts)
        parts = content.get("candidates", [{}])[0].get("content", {}).get("parts", [])
        text_resp = "".join(p.get("text", "") for p in parts).strip()

        if "```json" in text_resp:
            text_resp = text_resp.split("```json")[1].split("```")[0].strip()
        elif "```" in text_resp:
            text_resp = text_resp.split("```")[1].split("```")[0].strip()

        # Fallback: extract JSON object with regex
        import re
        if not text_resp.startswith("{"):
            m = re.search(r'\{[^}]+\}', text_resp)
            if m:
                text_resp = m.group()

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
            "UPDATE provision_actors SET llm_drrp = %s, llm_position = %s, llm_prompt_version = %s "
            "WHERE section_id = %s AND actor_label = %s",
            (drrp, position, PROMPT_VERSION, sid, label)
        )
        TOUCHED_LAWS.add(sid.split(":")[0])
    conn.commit()
    cur.close()


def main():
    parser = argparse.ArgumentParser(description="Gemini LLM batch classification")
    parser.add_argument("--limit", type=int, help="Limit number of actors")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--significance", choices=["HIGH", "MEDIUM", "LOW"],
                        help="Target provisions by significance level (requires --max-confidence)")
    parser.add_argument("--max-confidence", type=float, default=0.9,
                        help="SLM confidence threshold (default: 0.9)")
    parser.add_argument("--law-file", help="Law names to scope to (CSV line or one per line); applies to all modes")
    parser.add_argument("--position-correction", action="store_true",
                        help="#72: re-label counterparty/beneficiary actors under active Obligations at the agreed rule")
    parser.add_argument("--exclude-law-file", help="Law names to leave out (e.g. revoked laws), position correction only")
    parser.add_argument("--sample-out", help="Classify and write old → new to this TSV; nothing written to the DB")
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
    cur = conn.cursor()
    cur.execute("ALTER TABLE provision_actors ADD COLUMN IF NOT EXISTS llm_prompt_version text")
    conn.commit()
    cur.close()
    actors = query_pending_llm(conn, limit=args.limit,
                                significance=None if args.position_correction else args.significance,
                                max_confidence=args.max_confidence,
                                law_file=args.law_file,
                                position_correction=args.position_correction,
                                exclude_law_file=args.exclude_law_file)
    mode = ("position-correction" if args.position_correction
            else f"significance={args.significance} conf<{args.max_confidence}" if args.significance else "pending_llm")
    print(f"Loaded {len(actors):,} actors ({mode})")

    if args.dry_run:
        for sid, label, cat, drrp, text, _ in actors[:5]:
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
    sample = []

    for i, (sid, label, category, regex_drrp, text, old_position) in enumerate(actors):
        result = classify_actor(api_key, text, label)

        if result:
            drrp, position = result
            if args.sample_out:
                sample.append((sid, label, old_position, position, drrp, " ".join((text or "").split())[:400]))
            else:
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

    if args.sample_out:
        with open(args.sample_out, "w", newline="") as f:
            w = csv.writer(f, delimiter="\t")
            w.writerow(["section_id", "actor", "old_position", "new_position", "llm_drrp", "text"])
            w.writerows(sample)
        moved = Counter((o, n) for _, _, o, n, _, _ in sample)
        print(f"Sample written to {args.sample_out} (nothing written to the DB)")
        for (o, n), c in sorted(moved.items()):
            print(f"  {o} → {n}: {c}")

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

    if provenance and not args.sample_out:
        provenance.record(conn, TOUCHED_LAWS, "taxa", "llm", "llm", GEMINI_MODEL,
                          prompt_version=provenance.prompt_version(SYSTEM_PROMPT))
    conn.close()


if __name__ == "__main__":
    main()
