#!/usr/bin/env python3
"""Prepare artefact property extraction training data for SLM fine-tuning.

Generates JSONL in chat format. The SLM learns to extract structured
properties from obligation text + artefact type.

Since we don't have ground-truth labels for properties, we bootstrap
from what the regex pipeline already captured:
- owner_role: from RACI (R assignment)
- competence: from obligation competence_requirements
- review_frequency: from common patterns in text

The rest (approver, reviewer, acceptance_criterion, required_content)
are left for the SLM to learn from the text directly.

Output: data/jsp-artefact-training/train.jsonl, val.jsonl

Usage:
    /usr/bin/python3 scripts/ml/prepare_artefact_training.py
"""

import json
import os
import random
from collections import defaultdict

import duckdb

SYSTEM_PROMPT = """You are an artefact property extractor for MoD Joint Service Publications (JSPs).

Given an obligation text and the type of mandated artefact, extract structured properties:

{
  "owner_role": "who creates/maintains this artefact (or null)",
  "approver_role": "who approves it (or null)",
  "reviewer_role": "who reviews it (or null)",
  "review_frequency": "Annual / Quarterly / Monthly / Per-activity / On-change / Continuous (or null)",
  "required_content": ["what it must contain or demonstrate"],
  "acceptance_criterion": "the test for adequacy (or null)",
  "scope": "what it covers (or null)"
}

Extract only what is stated or clearly implied in the text. Use null for properties not mentioned.

Roles should use canonical labels: MoD: Commanding Officer, MoD: Accountable Person, MoD: Senior Duty Holder, MoD: Defence Safety Authority, MoD: Contractor, MoD: User, MoD: Commander/Manager, MoD: Head of Establishment, MoD: Defence Organisation"""

OUTPUT_DIR = "data/jsp-artefact-training"


def extract_frequency(text):
    """Extract review frequency from obligation text."""
    t = text.lower()
    if "annual" in t or "yearly" in t or "each year" in t:
        return "Annual"
    if "quarter" in t:
        return "Quarterly"
    if "month" in t:
        return "Monthly"
    if "periodic" in t or "regular" in t:
        return "Per-activity"
    if "before" in t and ("entry" in t or "work" in t or "use" in t):
        return "Per-activity"
    if "change" in t or "modif" in t or "alter" in t:
        return "On-change"
    if "continu" in t:
        return "Continuous"
    return None


def main():
    conn = duckdb.connect("data/fractalaw.duckdb", read_only=True)

    # Load artefacts with obligation text and RACI
    rows = conn.execute("""
        SELECT a.artefact_id, a.artefact_type, a.matched_text,
               o.text AS obligation_text, o.competence_requirements,
               r.role_label, r.assignment_type
        FROM jsp_mandated_artefacts a
        JOIN jsp_obligations o ON a.obligation_id = o.obligation_id
        LEFT JOIN jsp_raci r ON r.obligation_id = a.obligation_id
        ORDER BY a.artefact_id
    """).fetchall()
    conn.close()

    # Group by artefact_id (an artefact may have multiple RACI rows)
    by_artefact = defaultdict(lambda: {
        "type": "", "obligation": "", "competence": None, "roles": []
    })
    for aid, atype, matched, otext, competence, role, rtype in rows:
        entry = by_artefact[aid]
        entry["type"] = atype
        entry["obligation"] = otext
        entry["competence"] = competence
        if role and rtype:
            entry["roles"].append({"role": role, "type": rtype})

    print(f"Artefacts: {len(by_artefact)}")

    # Build training examples
    examples = []
    for aid, data in by_artefact.items():
        # Bootstrap properties from what we know
        owner = None
        for r in data["roles"]:
            if r["type"] == "R":
                owner = r["role"]
                break

        frequency = extract_frequency(data["obligation"])
        competence = data["competence"]

        # Build the target response
        properties = {
            "owner_role": owner,
            "approver_role": None,
            "reviewer_role": None,
            "review_frequency": frequency,
            "required_content": [],
            "acceptance_criterion": None,
            "scope": None,
        }

        # Add competence to required_content if present
        if competence:
            properties["required_content"].append(f"competence: {competence}")

        user_msg = f"Artefact type: {data['type']}\n\nObligation: {data['obligation']}"

        examples.append({
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": user_msg},
                {"role": "assistant", "content": json.dumps(properties)},
            ]
        })

    print(f"Training examples: {len(examples)}")

    # Split 90/10
    random.seed(42)
    random.shuffle(examples)
    split = int(len(examples) * 0.9)
    train = examples[:split]
    val = examples[split:]

    os.makedirs(OUTPUT_DIR, exist_ok=True)
    with open(os.path.join(OUTPUT_DIR, "train.jsonl"), "w") as f:
        for ex in train:
            f.write(json.dumps(ex) + "\n")
    with open(os.path.join(OUTPUT_DIR, "val.jsonl"), "w") as f:
        for ex in val:
            f.write(json.dumps(ex) + "\n")

    print(f"\nWritten to {OUTPUT_DIR}/")
    print(f"  train.jsonl: {len(train)}")
    print(f"  val.jsonl: {len(val)}")

    # Stats
    with_owner = sum(1 for e in examples if json.loads(e["messages"][2]["content"])["owner_role"])
    with_freq = sum(1 for e in examples if json.loads(e["messages"][2]["content"])["review_frequency"])
    print(f"  With owner_role: {with_owner}")
    print(f"  With review_frequency: {with_freq}")


if __name__ == "__main__":
    main()
