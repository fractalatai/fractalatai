#!/usr/bin/env python3
"""Export gold benchmark data as JSONL for Google AI Studio fine-tuning.

Produces train/test split with stratified sampling by position class.
Output format matches Google AI Studio's supervised tuning format.

Usage:
    python3 scripts/export_slm_training_data.py [--test-fraction 0.15]
"""

import argparse
import json
import random
from collections import defaultdict

import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

VALID_POSITIONS = {"active", "counterparty", "beneficiary", "mentioned"}
VALID_DRRP = {"Obligation", "Liberty", "none"}

SYSTEM_INSTRUCTION = (
    "You are a UK statutory law classifier. Given a provision from UK legislation "
    "and an actor mentioned in it, classify:\n\n"
    "1. The DRRP type of the provision for this actor:\n"
    "- Obligation: The provision imposes a duty or prohibition on the actor. "
    "Language: 'shall', 'shall not', 'must', 'must not', 'is required to', "
    "'has a duty', 'ensure', 'responsible for', 'so far as is reasonably practicable'.\n"
    "- Liberty: The provision grants a power, permission, or entitlement to the actor. "
    "Language: 'may', 'may not' (limiting a power), 'power to', 'entitled to', "
    "'authorise', 'enable', 'has the right'.\n"
    "- none: The provision does not create an obligation or liberty for this actor. "
    "The actor may be referenced in a definition, amendment, offence consequence, "
    "or cross-reference without a new legal relation being created.\n\n"
    "2. The actor's Hohfeldian legal position in this provision:\n"
    "- active: The actor bears the duty or exercises the power/liberty. "
    "They are the subject of the obligation or enabling language.\n"
    "- counterparty: The actor to whom the duty is owed or who is subject to the power. "
    "The correlative — if an employer has a duty, employees are the counterparty.\n"
    "- beneficiary: The actor benefits from the provision but is neither the duty-bearer "
    "nor the direct correlative. Often 'the public', 'persons' in safety provisions.\n"
    "- mentioned: The actor is referenced but has no active legal role in this provision. "
    "Common in definitions, cross-references, amendments, and offence consequences.\n\n"
    "Note: An actor can be 'active' with drrp 'none' — e.g. in an offence provision "
    "('A person who contravenes... is guilty') the actor is active but no new duty is created.\n\n"
    'Respond with ONLY a JSON object: {"drrp": "Obligation"|"Liberty"|"none", '
    '"position": "active"|"counterparty"|"beneficiary"|"mentioned"}'
)


def fetch_data(conn):
    """Fetch gold benchmarks joined with provision text."""
    cur = conn.cursor()
    cur.execute("""
        SELECT gb.section_id, gb.actor_label, gb.gold_position,
               COALESCE(gb.gold_drrp, 'none') as gold_drrp, lt.text
        FROM gold_benchmarks gb
        JOIN legislation_text lt ON gb.section_id = lt.section_id
        WHERE lt.text IS NOT NULL AND length(lt.text) > 10
          AND gb.gold_position IN ('active', 'counterparty', 'beneficiary', 'mentioned')
        ORDER BY gb.section_id, gb.actor_label
    """)
    rows = cur.fetchall()
    cur.close()
    return rows


def format_example(section_id, actor_label, position, drrp, text, fmt="huggingface"):
    """Format a single example for fine-tuning.

    fmt="huggingface" — messages/role/content (Unsloth, HF TRL, Colab)
    fmt="google"      — contents/role/parts (Vertex AI)
    """
    user_msg = f"Provision ({section_id}): {text}\n\nActor: {actor_label}\n\nClassify this actor's DRRP type and Hohfeldian position in this provision."
    model_response = json.dumps({"drrp": drrp, "position": position})

    if fmt == "google":
        return {
            "contents": [
                {"role": "user", "parts": [{"text": user_msg}]},
                {"role": "model", "parts": [{"text": model_response}]},
            ]
        }
    else:
        # HuggingFace / Unsloth chat format
        return {
            "messages": [
                {"role": "system", "content": SYSTEM_INSTRUCTION},
                {"role": "user", "content": user_msg},
                {"role": "assistant", "content": model_response},
            ]
        }


def stratified_split(rows, test_fraction=0.15, seed=42):
    """Split rows into train/test with stratified sampling by position."""
    random.seed(seed)
    by_class = defaultdict(list)
    for row in rows:
        by_class[row[2]].append(row)

    train, test = [], []
    for cls, items in by_class.items():
        random.shuffle(items)
        n_test = max(1, int(len(items) * test_fraction))
        test.extend(items[:n_test])
        train.extend(items[n_test:])

    random.shuffle(train)
    random.shuffle(test)
    return train, test


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--test-fraction", type=float, default=0.15)
    parser.add_argument("--format", choices=["huggingface", "google"], default="huggingface",
                        help="Output format: huggingface (Unsloth/TRL/Colab) or google (Vertex AI)")
    args = parser.parse_args()

    conn = psycopg2.connect(PG_DSN)
    rows = fetch_data(conn)
    conn.close()

    print(f"Total examples: {len(rows)}")
    print(f"Format: {args.format}")

    train_rows, test_rows = stratified_split(rows, args.test_fraction)

    # Write training JSONL
    train_path = "data/slm_train.jsonl"
    with open(train_path, "w") as f:
        for sid, actor, position, drrp, text in train_rows:
            example = format_example(sid, actor, position, drrp, text, fmt=args.format)
            f.write(json.dumps(example) + "\n")

    # Write test JSONL
    test_path = "data/slm_test.jsonl"
    with open(test_path, "w") as f:
        for sid, actor, position, drrp, text in test_rows:
            example = format_example(sid, actor, position, drrp, text, fmt=args.format)
            f.write(json.dumps(example) + "\n")

    # Write system instruction separately
    system_path = "data/slm_system_instruction.txt"
    with open(system_path, "w") as f:
        f.write(SYSTEM_INSTRUCTION)

    # Summary
    from collections import Counter
    train_pos = Counter(r[2] for r in train_rows)
    test_pos = Counter(r[2] for r in test_rows)
    train_drrp = Counter(r[3] for r in train_rows)
    test_drrp = Counter(r[3] for r in test_rows)

    print(f"\nTrain: {len(train_rows)} examples → {train_path}")
    print("  Position:", dict(sorted(train_pos.items())))
    print("  DRRP:    ", dict(sorted(train_drrp.items())))

    print(f"\nTest: {len(test_rows)} examples → {test_path}")
    print("  Position:", dict(sorted(test_pos.items())))
    print("  DRRP:    ", dict(sorted(test_drrp.items())))

    print(f"\nSystem instruction → {system_path}")

    # Also write a simple CSV for manual inspection
    csv_path = "data/slm_training_summary.csv"
    with open(csv_path, "w") as f:
        f.write("split,section_id,actor_label,position,drrp,text_length\n")
        for sid, actor, position, drrp, text in train_rows:
            f.write(f"train,{sid},{actor},{position},{drrp},{len(text)}\n")
        for sid, actor, position, drrp, text in test_rows:
            f.write(f"test,{sid},{actor},{position},{drrp},{len(text)}\n")
    print(f"Summary CSV → {csv_path}")


if __name__ == "__main__":
    main()
