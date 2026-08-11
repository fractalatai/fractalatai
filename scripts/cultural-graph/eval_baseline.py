#!/usr/bin/env python3
"""Evaluate zero-shot and fine-tuned SLM on cultural graph extraction.

Runs a model via Ollama on test narratives, compares output against
Gemini silver labels. Reports precision/recall per edge type.

Usage:
    /usr/bin/python3 scripts/cultural-graph/eval_baseline.py --model qwen3:8b --sample 5
    /usr/bin/python3 scripts/cultural-graph/eval_baseline.py --model qwen3:8b --all
    /usr/bin/python3 scripts/cultural-graph/eval_baseline.py --model cultural-graph-po:latest --all
"""

import argparse
import json
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path

import requests

OLLAMA_URL = "http://localhost:11434/api/chat"
TEST_PATH = Path("data/qq/cultural-graph/training/positive-observations-slm-test.jsonl")

CULTURAL_TYPES = {
    "shares-information-with", "monitors", "learns-from", "cooperates-with",
    "speaks-up-to", "recognises", "adapts-to", "responds-to-failure-of",
    "normalises", "directs", "cares-for", "protects",
}


def load_test_examples(path, sample=None):
    """Load test examples from JSONL."""
    examples = []
    with open(path) as f:
        for line in f:
            examples.append(json.loads(line))
    if sample:
        examples = examples[:sample]
    return examples


def call_ollama(model, system_prompt, user_prompt):
    """Call Ollama chat API."""
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
        "stream": False,
        "options": {"temperature": 0.1, "num_predict": 4096},
        "format": "json",
    }
    resp = requests.post(OLLAMA_URL, json=payload, timeout=600)
    resp.raise_for_status()
    data = resp.json()
    content = data["message"]["content"]
    try:
        return json.loads(content), data.get("eval_duration", 0) / 1e9
    except json.JSONDecodeError:
        return {"error": "JSON parse failed", "raw": content[:500]}, 0


def extract_edge_set(relationships):
    """Convert relationships list to a set of (source, target, edge_type) tuples.
    Normalise source/target to lowercase for comparison."""
    edges = set()
    for r in relationships:
        if r.get("edge_type") in CULTURAL_TYPES:
            edges.add((
                r["source"].lower().strip(),
                r["target"].lower().strip(),
                r["edge_type"],
            ))
    return edges


def edge_type_only_set(relationships):
    """Just the edge types present, for coarser comparison."""
    return Counter(
        r["edge_type"] for r in relationships
        if r.get("edge_type") in CULTURAL_TYPES
    )


def evaluate(examples, model):
    """Run model on examples and compare against silver labels."""
    results = []
    total_time = 0

    for i, ex in enumerate(examples):
        system = ex["messages"][0]["content"]
        user = ex["messages"][1]["content"]
        gold_output = json.loads(ex["messages"][2]["content"])
        gold_edges = extract_edge_set(gold_output.get("relationships", []))
        gold_types = edge_type_only_set(gold_output.get("relationships", []))

        print(f"[{i+1}/{len(examples)}] ", end="", flush=True)

        pred_output, duration = call_ollama(model, system, user)
        total_time += duration

        if "error" in pred_output:
            print(f"ERROR: {pred_output['error']}")
            results.append({"valid": False, "error": pred_output["error"]})
            continue

        pred_edges = extract_edge_set(pred_output.get("relationships", []))
        pred_types = edge_type_only_set(pred_output.get("relationships", []))

        # Exact match (source, target, edge_type)
        tp = len(gold_edges & pred_edges)
        fp = len(pred_edges - gold_edges)
        fn = len(gold_edges - pred_edges)

        # Type-level comparison (did it find the right edge types, regardless of source/target?)
        type_tp = sum((gold_types & pred_types).values())
        type_total_gold = sum(gold_types.values())
        type_total_pred = sum(pred_types.values())

        prec = tp / (tp + fp) if (tp + fp) > 0 else 0
        rec = tp / (tp + fn) if (tp + fn) > 0 else 0

        type_prec = type_tp / type_total_pred if type_total_pred > 0 else 0
        type_rec = type_tp / type_total_gold if type_total_gold > 0 else 0

        print(f"exact P={prec:.2f} R={rec:.2f} | type P={type_prec:.2f} R={type_rec:.2f} | "
              f"{len(pred_edges)} pred, {len(gold_edges)} gold | {duration:.1f}s")

        results.append({
            "valid": True,
            "exact": {"tp": tp, "fp": fp, "fn": fn, "precision": prec, "recall": rec},
            "type_level": {"tp": type_tp, "gold": type_total_gold, "pred": type_total_pred,
                           "precision": type_prec, "recall": type_rec},
            "pred_types": dict(pred_types),
            "gold_types": dict(gold_types),
            "duration": duration,
        })
        time.sleep(0.5)

    # Aggregate
    valid = [r for r in results if r["valid"]]
    if not valid:
        print("No valid results")
        return

    print(f"\n{'='*60}")
    print(f"Model: {model}")
    print(f"Examples: {len(valid)}/{len(examples)} valid")
    print(f"Total inference time: {total_time:.1f}s ({total_time/len(valid):.1f}s/example)")

    # Exact match aggregate
    total_tp = sum(r["exact"]["tp"] for r in valid)
    total_fp = sum(r["exact"]["fp"] for r in valid)
    total_fn = sum(r["exact"]["fn"] for r in valid)
    agg_prec = total_tp / (total_tp + total_fp) if (total_tp + total_fp) > 0 else 0
    agg_rec = total_tp / (total_tp + total_fn) if (total_tp + total_fn) > 0 else 0
    agg_f1 = 2 * agg_prec * agg_rec / (agg_prec + agg_rec) if (agg_prec + agg_rec) > 0 else 0
    print(f"\nExact match (source+target+type):")
    print(f"  Precision: {agg_prec:.3f}")
    print(f"  Recall:    {agg_rec:.3f}")
    print(f"  F1:        {agg_f1:.3f}")

    # Type-level aggregate
    type_tp = sum(r["type_level"]["tp"] for r in valid)
    type_gold = sum(r["type_level"]["gold"] for r in valid)
    type_pred = sum(r["type_level"]["pred"] for r in valid)
    type_prec = type_tp / type_pred if type_pred > 0 else 0
    type_rec = type_tp / type_gold if type_gold > 0 else 0
    type_f1 = 2 * type_prec * type_rec / (type_prec + type_rec) if (type_prec + type_rec) > 0 else 0
    print(f"\nType-level match (edge type counts):")
    print(f"  Precision: {type_prec:.3f}")
    print(f"  Recall:    {type_rec:.3f}")
    print(f"  F1:        {type_f1:.3f}")

    # Per-type breakdown
    print(f"\nPer-type (predicted / gold):")
    all_pred_types = Counter()
    all_gold_types = Counter()
    for r in valid:
        all_pred_types.update(r["pred_types"])
        all_gold_types.update(r["gold_types"])
    all_types = sorted(set(all_pred_types.keys()) | set(all_gold_types.keys()))
    for t in all_types:
        p = all_pred_types.get(t, 0)
        g = all_gold_types.get(t, 0)
        print(f"  {t:<28} pred={p:>4}  gold={g:>4}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", default="qwen3:8b", help="Ollama model name")
    parser.add_argument("--sample", type=int, help="Run on first N test examples")
    parser.add_argument("--all", action="store_true", help="Run on all test examples")
    args = parser.parse_args()

    if not args.sample and not args.all:
        parser.error("Specify --sample N or --all")

    examples = load_test_examples(TEST_PATH, sample=args.sample if not args.all else None)
    print(f"Loaded {len(examples)} test examples")
    print(f"Model: {args.model}")
    print()

    evaluate(examples, args.model)


if __name__ == "__main__":
    main()
