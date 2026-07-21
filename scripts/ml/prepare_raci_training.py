#!/usr/bin/env python3
"""Prepare RACI training data for SLM fine-tuning.

Generates JSONL in Ollama/HuggingFace chat format from DuckDB
jsp_obligations + jsp_raci tables.

Output: data/jsp-raci-training/train.jsonl, val.jsonl

Usage:
    /usr/bin/python3 scripts/ml/prepare_raci_training.py
"""

import json
import os
import random
from collections import defaultdict

import duckdb

SYSTEM_PROMPT = """You are a RACI classifier for MoD Joint Service Publications (JSPs).

Given an obligation from a JSP, identify:
1. Which organisational role(s) are mentioned
2. Their RACI assignment: R (Responsible - does the work), A (Accountable - owns the outcome), C (Consulted - asked for input), I (Informed - notified)

Respond with a JSON array of assignments. If no role is identifiable, respond with an empty array [].

Example roles: MoD: Commanding Officer, MoD: Accountable Person, MoD: Senior Duty Holder, MoD: Defence Safety Authority, MoD: Contractor, MoD: User, MoD: Commander/Manager, MoD: Head of Establishment, MoD: Defence Organisation

Key patterns:
- "X must/shall/will [do something]" → X is R (Responsible)
- "X is accountable for" → X is A (Accountable)
- "in consultation with X" → X is C (Consulted)
- "X shall be informed/notified" → X is I (Informed)
- Passive voice "shall be conducted" → look for the actor in context"""

OUTPUT_DIR = "data/jsp-raci-training"


def main():
    conn = duckdb.connect("data/fractalaw.duckdb", read_only=True)

    # Load positive examples: obligations with RACI assignments
    rows = conn.execute("""
        SELECT o.obligation_id, o.text, o.strength,
               r.role_label, r.assignment_type
        FROM jsp_obligations o
        JOIN jsp_raci r ON o.obligation_id = r.obligation_id
        ORDER BY o.obligation_id
    """).fetchall()

    # Group RACI assignments by obligation
    by_obligation = defaultdict(lambda: {"text": "", "strength": "", "raci": []})
    for oid, text, strength, role, atype in rows:
        by_obligation[oid]["text"] = text
        by_obligation[oid]["strength"] = strength
        by_obligation[oid]["raci"].append({"role": role, "type": atype})

    print(f"Positive examples: {len(by_obligation)} obligations with RACI")

    # Load negative examples: mandatory obligations WITHOUT RACI
    # (these are the hardest — mandatory but no actor identified)
    neg_rows = conn.execute("""
        SELECT o.obligation_id, o.text, o.strength
        FROM jsp_obligations o
        LEFT JOIN jsp_raci r ON o.obligation_id = r.obligation_id
        WHERE r.raci_id IS NULL AND o.strength = 'Mandatory'
        LIMIT 500
    """).fetchall()

    print(f"Negative examples: {len(neg_rows)} mandatory obligations without RACI")
    conn.close()

    # Build training examples
    examples = []

    # Positive: obligation → RACI assignments
    for oid, data in by_obligation.items():
        response = json.dumps(data["raci"], indent=None)
        examples.append({
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": data["text"]},
                {"role": "assistant", "content": response},
            ]
        })

    # Negative: obligation → empty array
    for oid, text, strength in neg_rows:
        examples.append({
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": text},
                {"role": "assistant", "content": "[]"},
            ]
        })

    print(f"Total examples: {len(examples)}")

    # Shuffle and split 90/10
    random.seed(42)
    random.shuffle(examples)
    split = int(len(examples) * 0.9)
    train = examples[:split]
    val = examples[split:]

    print(f"Train: {len(train)}, Val: {len(val)}")

    # Write JSONL
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    with open(os.path.join(OUTPUT_DIR, "train.jsonl"), "w") as f:
        for ex in train:
            f.write(json.dumps(ex) + "\n")
    with open(os.path.join(OUTPUT_DIR, "val.jsonl"), "w") as f:
        for ex in val:
            f.write(json.dumps(ex) + "\n")

    print(f"\nWritten to {OUTPUT_DIR}/")
    print(f"  train.jsonl: {len(train)} examples")
    print(f"  val.jsonl: {len(val)} examples")

    # Stats
    pos_count = len(by_obligation)
    neg_count = len(neg_rows)
    print(f"\n  Positive (has RACI): {pos_count}")
    print(f"  Negative (no RACI): {neg_count}")
    print(f"  Ratio: {pos_count/(pos_count+neg_count):.0%} positive")


if __name__ == "__main__":
    main()
