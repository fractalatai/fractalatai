#!/usr/bin/env python3
"""Export cultural graph training data as instruction-tuning JSONL.

Reads constrained extraction JSONL, produces train/test split in
HuggingFace chat format (system/user/assistant messages) for SLM fine-tuning.

Usage:
    /usr/bin/python3 scripts/cultural-graph/export_training_data.py
    /usr/bin/python3 scripts/cultural-graph/export_training_data.py --test-fraction 0.15
"""

import argparse
import csv
import json
import random
from collections import defaultdict
from pathlib import Path

DATA_DIR = Path("data/qq/cultural-graph")
CONSTRAINED_DIR = DATA_DIR / "constrained"
INPUT_CSV = DATA_DIR / "Positive_Observations_Redacted(ReportTable).csv"
OUTPUT_DIR = DATA_DIR / "outputs/training"

SYSTEM_INSTRUCTION = (
    "You are a safety culture analyst. Given a workplace safety narrative, "
    "extract entities and cultural relationships.\n\n"
    "Entity types: People, Plant, Process, Place, Provision.\n\n"
    "Cultural relationship types:\n"
    "- shares-information-with: briefing, explaining, informing another person\n"
    "- monitors: watching, checking, reviewing another's work\n"
    "- learns-from: acquiring knowledge from another person or experience\n"
    "- cooperates-with: working together, coordinating, jointly participating\n"
    "- speaks-up-to: raising concerns, challenging decisions, stopping work, suggesting improvements\n"
    "- recognises: acknowledging competence, effort, or good practice\n"
    "- adapts-to: adjusting behaviour or improving a process in response to conditions\n"
    "- responds-to-failure-of: reacting when something goes wrong or is found deficient\n"
    "- normalises: treating a deviation from procedure as acceptable or routine\n"
    "- directs: giving orders or leading activities with authority\n"
    "- cares-for: welfare gestures — looking after someone's wellbeing\n"
    "- protects: proactive safeguarding — designing or maintaining controls that prevent harm\n"
    "- operational: a person performing a task — not an interpersonal cultural relationship\n\n"
    "Return a JSON object with entities and relationships. "
    "Each relationship has source, target, edge_type, and detail fields."
)


def load_narratives():
    """Load narrative text from CSV."""
    with open(INPUT_CSV, encoding="utf-8-sig") as f:
        rows = list(csv.DictReader(f))
    texts = {}
    for i, row in enumerate(rows):
        texts[f"PO-{i+1:04d}"] = row["RE_What"]
    return texts


def load_extractions():
    """Load all valid constrained extractions."""
    records = {}
    for path in sorted(CONSTRAINED_DIR.glob("constrained_*.jsonl")):
        with open(path) as f:
            for line in f:
                r = json.loads(line)
                if r["valid"]:
                    records[r["narrative_id"]] = r
    return records


def format_example(narrative_text, extraction):
    """Format a single training example in HuggingFace chat format."""
    user_msg = f"Extract entities and cultural relationships from this narrative:\n\n\"{narrative_text}\""

    # Build the target output — entities + cultural edges only (skip operational)
    entities = extraction.get("entities", [])
    relationships = extraction.get("relationships", [])

    output = {
        "entities": [{"text": e["text"], "type": e["type"]} for e in entities],
        "relationships": [
            {
                "source": r["source"],
                "target": r["target"],
                "edge_type": r.get("edge_type", "unknown"),
                "detail": r.get("detail", ""),
            }
            for r in relationships
        ],
    }

    return {
        "messages": [
            {"role": "system", "content": SYSTEM_INSTRUCTION},
            {"role": "user", "content": user_msg},
            {"role": "assistant", "content": json.dumps(output)},
        ]
    }


def stratified_split(records, test_fraction=0.13, seed=42):
    """Split by cultural edge density to ensure test set has varied complexity."""
    random.seed(seed)

    # Bucket by cultural edge count: 0-2 (low), 3-6 (medium), 7+ (high)
    buckets = defaultdict(list)
    for nid, r in records.items():
        rels = r["extraction"].get("relationships", [])
        cultural = len([x for x in rels if x.get("edge_type") != "operational"])
        if cultural <= 2:
            buckets["low"].append(nid)
        elif cultural <= 6:
            buckets["medium"].append(nid)
        else:
            buckets["high"].append(nid)

    train_ids, test_ids = [], []
    for bucket, ids in buckets.items():
        random.shuffle(ids)
        n_test = max(1, int(len(ids) * test_fraction))
        test_ids.extend(ids[:n_test])
        train_ids.extend(ids[n_test:])

    random.shuffle(train_ids)
    random.shuffle(test_ids)
    return train_ids, test_ids


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--test-fraction", type=float, default=0.13,
                        help="Fraction for test set (default 0.13 ≈ 40 narratives)")
    args = parser.parse_args()

    texts = load_narratives()
    records = load_extractions()
    print(f"Loaded {len(records)} valid extractions")

    train_ids, test_ids = stratified_split(records, args.test_fraction)
    print(f"Train: {len(train_ids)}, Test: {len(test_ids)}")

    # Write train JSONL
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    train_path = OUTPUT_DIR / "positive-observations-slm-train.jsonl"
    with open(train_path, "w") as f:
        for nid in train_ids:
            example = format_example(texts[nid], records[nid]["extraction"])
            f.write(json.dumps(example) + "\n")

    # Write test JSONL
    test_path = OUTPUT_DIR / "positive-observations-slm-test.jsonl"
    with open(test_path, "w") as f:
        for nid in test_ids:
            example = format_example(texts[nid], records[nid]["extraction"])
            f.write(json.dumps(example) + "\n")

    # Stats
    train_cultural = 0
    test_cultural = 0
    for nid in train_ids:
        rels = records[nid]["extraction"].get("relationships", [])
        train_cultural += len([x for x in rels if x.get("edge_type") != "operational"])
    for nid in test_ids:
        rels = records[nid]["extraction"].get("relationships", [])
        test_cultural += len([x for x in rels if x.get("edge_type") != "operational"])

    print(f"\nTrain: {train_path}")
    print(f"  {len(train_ids)} narratives, {train_cultural} cultural edges")
    print(f"Test: {test_path}")
    print(f"  {len(test_ids)} narratives, {test_cultural} cultural edges")

    # Check token budget (rough estimate: 4 chars ≈ 1 token)
    train_chars = sum(
        len(json.dumps(format_example(texts[nid], records[nid]["extraction"])))
        for nid in train_ids
    )
    test_chars = sum(
        len(json.dumps(format_example(texts[nid], records[nid]["extraction"])))
        for nid in test_ids
    )
    print(f"\nEstimated tokens (4 chars/token):")
    print(f"  Train: ~{train_chars // 4:,} tokens")
    print(f"  Test:  ~{test_chars // 4:,} tokens")
    print(f"  Mean per example: ~{train_chars // 4 // len(train_ids):,} tokens")


if __name__ == "__main__":
    main()
