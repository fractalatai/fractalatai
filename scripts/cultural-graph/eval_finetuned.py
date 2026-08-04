#!/usr/bin/env python3
"""Evaluate fine-tuned cultural graph model via Unsloth on GPU.

Run on RunPod after fine-tuning. Loads the adapter from OUTPUT_DIR.

Usage:
    python3 /workspace/cultural-graph/eval_finetuned.py
"""

import json
import re
import time
from collections import Counter

ADAPTER_DIR = "/workspace/cultural-graph/output/cultural-graph-po-lora"
TEST_FILE = "/workspace/cultural-graph/positive-observations-slm-test.jsonl"

CULTURAL_TYPES = {
    "shares-information-with", "monitors", "learns-from", "cooperates-with",
    "speaks-up-to", "recognises", "adapts-to", "responds-to-failure-of",
    "normalises", "directs", "cares-for", "protects",
}


def main():
    import torch
    from unsloth import FastModel

    model, tokenizer = FastModel.from_pretrained(
        ADAPTER_DIR, max_seq_length=2048, load_in_4bit=True,
    )

    with open(TEST_FILE) as f:
        examples = [json.loads(line) for line in f]

    print(f"Evaluating {len(examples)} test examples")
    total = 0
    valid_json = 0
    type_matches = 0
    type_total = 0
    per_type_pred = Counter()
    per_type_gold = Counter()
    t0 = time.time()

    for i, ex in enumerate(examples):
        msgs = ex["messages"]
        gold_output = json.loads(msgs[2]["content"])
        gold_rels = [r for r in gold_output.get("relationships", [])
                     if r.get("edge_type") in CULTURAL_TYPES]
        gold_types = Counter(r["edge_type"] for r in gold_rels)

        input_msgs = [
            {"role": msgs[0]["role"], "content": msgs[0]["content"]},
            {"role": msgs[1]["role"], "content": msgs[1]["content"]},
        ]
        input_ids = tokenizer.apply_chat_template(
            input_msgs, tokenize=True, add_generation_prompt=True,
            return_tensors="pt"
        ).to(model.device)

        with torch.no_grad():
            output_ids = model.generate(
                input_ids, max_new_tokens=2048, temperature=0.1,
                do_sample=True, pad_token_id=tokenizer.pad_token_id,
            )

        new_tokens = output_ids[0][input_ids.shape[1]:]
        response = tokenizer.decode(new_tokens, skip_special_tokens=True)
        # Strip Qwen3 thinking tags
        response = re.sub(r"<think>[\s\S]*?</think>", "", response).strip()

        total += 1
        try:
            pred = json.loads(response)
            valid_json += 1
            pred_rels = [r for r in pred.get("relationships", [])
                         if r.get("edge_type") in CULTURAL_TYPES]
            pred_types = Counter(r["edge_type"] for r in pred_rels)
            per_type_pred.update(pred_types)
            per_type_gold.update(gold_types)
            matches = sum((pred_types & gold_types).values())
            type_matches += matches
            type_total += sum(gold_types.values())
        except json.JSONDecodeError:
            pass

        if (i + 1) % 10 == 0:
            print(f"  [{i+1}/{len(examples)}] valid_json={valid_json}/{total}")

    elapsed = time.time() - t0
    separator = "=" * 60
    print()
    print(separator)
    print("Fine-tuned Qwen 3 8B - Positive Observations")
    print(f"Examples: {valid_json}/{total} valid JSON ({100*valid_json/total:.0f}%)")
    print(f"Inference time: {elapsed:.1f}s ({elapsed/total:.1f}s/example)")

    if type_total > 0:
        pred_total = sum(per_type_pred.values())
        type_prec = type_matches / pred_total if pred_total > 0 else 0
        type_rec = type_matches / type_total
        type_f1 = 2 * type_prec * type_rec / (type_prec + type_rec) if (type_prec + type_rec) > 0 else 0
        print()
        print("Type-level match:")
        print(f"  Precision: {type_prec:.3f}")
        print(f"  Recall:    {type_rec:.3f}")
        print(f"  F1:        {type_f1:.3f}")

    print()
    print("Per-type (pred / gold):")
    all_types = sorted(set(per_type_pred.keys()) | set(per_type_gold.keys()))
    for t in all_types:
        p = per_type_pred.get(t, 0)
        g = per_type_gold.get(t, 0)
        print(f"  {t:<28} pred={p:>4}  gold={g:>4}")
    print(separator)


if __name__ == "__main__":
    main()
