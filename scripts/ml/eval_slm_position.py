#!/usr/bin/env python3
"""Evaluate gemma3:4b on pending_llm benchmark actors.

Queries provision_actors for pending_llm actors that have gold benchmark labels,
sends each (provision_text, actor_label) to Ollama, compares predicted position
against gold_position.

Usage:
    python3 scripts/eval_slm_position.py [--limit N] [--dry-run]
"""

import argparse
import json
import sys
import time
from collections import Counter

import psycopg2
import requests

OLLAMA_URL = "http://localhost:11434/api/generate"
MODEL = "gemma3-position"

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

SYSTEM_PROMPT = """\
You are a UK statutory law classifier. Given a provision from UK legislation and \
an actor mentioned in it, classify the actor's Hohfeldian legal position.

Positions:
- active: The actor bears the duty, obligation, or exercises the power/liberty. \
They are the one who "shall", "must", or "may" do something.
- counterparty: The actor to whom the duty is owed, or against whom the right is held. \
They are the correlative — if an employer has a duty, employees are the counterparty.
- beneficiary: The actor who benefits from the provision but is neither the duty-bearer \
nor the direct correlative. Often "persons", "the public", "employees" in safety provisions.
- mentioned: The actor is referenced but has no active legal role in this provision. \
Definitional, procedural, or merely cited.

Respond with ONLY a JSON object: {"position": "active"|"counterparty"|"beneficiary"|"mentioned"}
No explanation, no markdown, no other text."""

USER_TEMPLATE = """\
Provision: {text}

Actor: {actor_label}

What is this actor's Hohfeldian position in this provision?"""


def query_eval_set(conn, limit=None):
    """Fetch pending_llm actors with gold labels + provision text."""
    sql = """
        SELECT pa.section_id, pa.actor_label, pa.regex_position, pa.cls_position,
               gb.gold_position, lt.text
        FROM provision_actors pa
        JOIN gold_benchmarks gb ON pa.section_id = gb.section_id
            AND pa.actor_label = gb.actor_label
        JOIN legislation_text lt ON pa.section_id = lt.section_id
        WHERE pa.extraction_method = 'pending_llm'
        ORDER BY pa.section_id, pa.actor_label
    """
    if limit:
        sql += f" LIMIT {limit}"
    cur = conn.cursor()
    cur.execute(sql)
    rows = cur.fetchall()
    cur.close()
    return rows


def classify_actor(text, actor_label, timeout=60):
    """Send a single (provision, actor) to Ollama and parse the response."""
    user_msg = USER_TEMPLATE.format(text=text, actor_label=actor_label)
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
        resp = requests.post(OLLAMA_URL.replace("/api/generate", "/api/chat"), json=body, timeout=timeout)
        resp.raise_for_status()
        content = resp.json().get("message", {}).get("content", "").strip()

        # Strip markdown fences if present
        if "```json" in content:
            content = content.split("```json")[1].split("```")[0].strip()
        elif "```" in content:
            content = content.split("```")[1].split("```")[0].strip()

        parsed = json.loads(content)
        position = parsed.get("position", "").lower().strip()
        if position in ("active", "counterparty", "beneficiary", "mentioned"):
            return position
        return None
    except (requests.RequestException, json.JSONDecodeError, KeyError, IndexError) as e:
        print(f"  ERROR: {e}", file=sys.stderr)
        return None


def main():
    parser = argparse.ArgumentParser(description="Evaluate gemma3:4b on pending_llm actors")
    parser.add_argument("--limit", type=int, help="Limit number of actors to evaluate")
    parser.add_argument("--dry-run", action="store_true", help="Show queries without calling Ollama")
    args = parser.parse_args()

    conn = psycopg2.connect(PG_DSN)
    rows = query_eval_set(conn, limit=args.limit)
    conn.close()

    print(f"Evaluation set: {len(rows)} pending_llm actors with gold labels\n")

    if args.dry_run:
        for sid, actor, regex_pos, cls_pos, gold_pos, text in rows[:5]:
            print(f"  {sid} | {actor}")
            print(f"    regex={regex_pos} cls={cls_pos} gold={gold_pos}")
            print(f"    text={text[:120]}...")
            print()
        print(f"... and {len(rows) - 5} more" if len(rows) > 5 else "")
        return

    correct = 0
    total = 0
    errors = 0
    results = []
    method_counts = Counter()
    confusion = Counter()  # (predicted, gold)
    t0 = time.time()

    for i, (sid, actor, regex_pos, cls_pos, gold_pos, text) in enumerate(rows):
        predicted = classify_actor(text, actor)
        total += 1

        if predicted is None:
            errors += 1
            print(f"  [{i+1}/{len(rows)}] {sid} | {actor} → PARSE ERROR")
            continue

        is_correct = predicted == gold_pos
        if is_correct:
            correct += 1

        confusion[(predicted, gold_pos)] += 1
        results.append({
            "section_id": sid,
            "actor_label": actor,
            "gold": gold_pos,
            "predicted": predicted,
            "regex": regex_pos,
            "cls": cls_pos,
            "correct": is_correct,
        })

        marker = "OK" if is_correct else "WRONG"
        if (i + 1) % 25 == 0 or not is_correct:
            print(f"  [{i+1}/{len(rows)}] {sid} | {actor} → {predicted} (gold={gold_pos}) {marker}")

        # Progress summary every 50
        if (i + 1) % 50 == 0:
            elapsed = time.time() - t0
            rate = (i + 1) / elapsed
            eta = (len(rows) - i - 1) / rate if rate > 0 else 0
            acc = 100.0 * correct / total if total > 0 else 0
            print(f"  --- Progress: {correct}/{total} = {acc:.1f}% | "
                  f"{rate:.1f}/s | ETA {eta:.0f}s ---")

    elapsed = time.time() - t0
    acc = 100.0 * correct / total if total > 0 else 0

    print(f"\n{'='*60}")
    print(f"SLM Evaluation: gemma3:4b on pending_llm actors")
    print(f"{'='*60}")
    print(f"Total:    {total}")
    print(f"Correct:  {correct}")
    print(f"Errors:   {errors}")
    print(f"Accuracy: {acc:.1f}%")
    print(f"Baseline: 37.3% (reconciled regex fallback)")
    print(f"Time:     {elapsed:.1f}s ({total/elapsed:.1f} actors/s)")

    # Per-position accuracy
    print(f"\nPer-position accuracy:")
    positions = ["active", "counterparty", "beneficiary", "mentioned"]
    for pos in positions:
        golds = [r for r in results if r["gold"] == pos]
        if golds:
            pos_correct = sum(1 for r in golds if r["correct"])
            print(f"  {pos:15s}: {pos_correct}/{len(golds)} = {100*pos_correct/len(golds):.1f}%")

    # Confusion matrix
    print(f"\nConfusion matrix (rows=predicted, cols=gold):")
    print(f"  {'':15s} " + " ".join(f"{p:>12s}" for p in positions))
    for pred in positions:
        counts = [confusion.get((pred, gold), 0) for gold in positions]
        print(f"  {pred:15s} " + " ".join(f"{c:12d}" for c in counts))

    # Comparison: SLM vs regex vs classifier on these same actors
    print(f"\nComparison on these {total} actors:")
    regex_correct = sum(1 for r in results if r["regex"] == r["gold"])
    cls_correct = sum(1 for r in results if r["cls"] == r["gold"])
    print(f"  Regex:      {regex_correct}/{total} = {100*regex_correct/total:.1f}%")
    print(f"  Classifier: {cls_correct}/{total} = {100*cls_correct/total:.1f}%")
    print(f"  SLM:        {correct}/{total} = {acc:.1f}%")

    # Save detailed results
    out_path = "data/slm_eval_results.json"
    with open(out_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nDetailed results saved to {out_path}")


if __name__ == "__main__":
    main()
