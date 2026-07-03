#!/usr/bin/env python3
"""Export significance benchmark data as JSONL for SLM fine-tuning.

Joins significance ratings with provision text from Postgres.
Produces train/test split in HuggingFace chat format.

Usage:
    /usr/bin/python3 scripts/export_significance_training.py
"""

import json
import random
from collections import Counter, defaultdict

import psycopg2

PG_DSN = "host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw"

SYSTEM_INSTRUCTION = (
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


def main():
    # Load significance ratings
    ratings = json.load(open("data/significance_benchmark.json"))
    ratings_map = {r["section_id"]: r for r in ratings}
    print(f"Loaded {len(ratings)} significance ratings")

    # Fetch provision text from Postgres
    conn = psycopg2.connect(PG_DSN)
    cur = conn.cursor()
    sids = list(ratings_map.keys())
    cur.execute(
        "SELECT section_id, text, section_type FROM legislation_text WHERE section_id = ANY(%s)",
        (sids,)
    )
    texts = {row[0]: (row[1], row[2]) for row in cur.fetchall()}
    cur.close()
    conn.close()
    print(f"Fetched text for {len(texts)} provisions")

    # Build examples
    examples = []
    for sid, rating in ratings_map.items():
        if sid not in texts:
            continue
        text, stype = texts[sid]
        if not text or len(text) < 20:
            continue

        user_msg = (
            f"Provision ({sid}, type={stype}): {text}\n\n"
            f"Rate the significance of this obligation."
        )
        model_response = json.dumps({
            "scope_duty_bearer": rating["scope_duty_bearer"],
            "scope_protected_class": rating["scope_protected_class"],
            "gravity": rating["gravity"],
            "strength": rating["strength"],
        })

        examples.append({
            "messages": [
                {"role": "system", "content": SYSTEM_INSTRUCTION},
                {"role": "user", "content": user_msg},
                {"role": "assistant", "content": model_response},
            ],
            "section_id": sid,
        })

    print(f"Built {len(examples)} training examples")

    # Stratified split by gravity (most balanced dimension)
    random.seed(42)
    by_gravity = defaultdict(list)
    for ex in examples:
        label = json.loads(ex["messages"][2]["content"])["gravity"]
        by_gravity[label].append(ex)

    train, test = [], []
    for label, items in by_gravity.items():
        random.shuffle(items)
        n_test = max(1, int(len(items) * 0.15))
        test.extend(items[:n_test])
        train.extend(items[n_test:])

    random.shuffle(train)
    random.shuffle(test)

    # Write JSONL
    train_path = "data/ml/significance_train.jsonl"
    with open(train_path, "w") as f:
        for ex in train:
            out = {"messages": ex["messages"]}
            f.write(json.dumps(out) + "\n")

    test_path = "data/ml/significance_test.jsonl"
    with open(test_path, "w") as f:
        for ex in test:
            out = {"messages": ex["messages"]}
            f.write(json.dumps(out) + "\n")

    # Summary
    train_dims = {d: Counter() for d in ["scope_duty_bearer", "scope_protected_class", "gravity", "strength"]}
    for ex in train:
        labels = json.loads(ex["messages"][2]["content"])
        for d, v in labels.items():
            train_dims[d][v] += 1

    print(f"\nTrain: {len(train)} → {train_path}")
    print(f"Test:  {len(test)} → {test_path}")
    for d in ["scope_duty_bearer", "scope_protected_class", "gravity", "strength"]:
        print(f"  {d}: {dict(sorted(train_dims[d].items()))}")


if __name__ == "__main__":
    main()
