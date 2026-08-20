#!/usr/bin/env python3
"""Export LoRA mechanism classifier to ONNX and run QQ bulk inference.

Usage (on RunPod):
    python3 /workspace/sif/scripts/export_and_infer.py
"""

import json
import time
from pathlib import Path

import numpy as np
import torch
from peft import PeftModel
from transformers import AutoModelForSequenceClassification, AutoTokenizer

BASE_MODEL = "Qwen/Qwen3-1.7B"
QQ_DATA = "/workspace/sif/data/qq_benchmark.json"
MAX_LENGTH = 512
INSTRUCTION = "Classify the following workplace incident narrative. The mechanism of injury is:"


def load_model(model_dir):
    """Load LoRA model merged with base for export."""
    print("Loading tokenizer...")
    tokenizer = AutoTokenizer.from_pretrained(model_dir, trust_remote_code=True)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    print(f"Loading base model {BASE_MODEL}...")
    with open(f"{model_dir}/training_meta.json") as f:
        meta = json.load(f)

    base = AutoModelForSequenceClassification.from_pretrained(
        BASE_MODEL,
        num_labels=meta["num_labels"],
        id2label=meta["id2label"],
        label2id=meta["label2id"],
        trust_remote_code=True,
    )
    base.config.pad_token_id = tokenizer.pad_token_id

    print("Loading LoRA adapter...")
    model = PeftModel.from_pretrained(base, model_dir)

    print("Merging LoRA into base model...")
    model = model.merge_and_unload()
    model.eval()

    return model, tokenizer, meta


def export_onnx(model, tokenizer):
    """Export merged model to ONNX."""
    print(f"\nExporting to ONNX: {ONNX_PATH}")

    dummy = tokenizer(
        "test narrative",
        truncation=True,
        max_length=MAX_LENGTH,
        padding="max_length",
        return_tensors="pt",
    )

    torch.onnx.export(
        model,
        (dummy["input_ids"], dummy["attention_mask"]),
        ONNX_PATH,
        input_names=["input_ids", "attention_mask"],
        output_names=["logits"],
        dynamic_axes={
            "input_ids": {0: "batch", 1: "seq"},
            "attention_mask": {0: "batch", 1: "seq"},
            "logits": {0: "batch"},
        },
        opset_version=17,
    )

    size_mb = Path(ONNX_PATH).stat().st_size / 1024 / 1024
    print(f"  ONNX exported: {size_mb:.1f} MB")
    return size_mb


def run_qq_inference(model, tokenizer, meta, run_name):
    """Run bulk inference on QQ correlation test data."""
    print(f"\nLoading QQ data from {QQ_DATA}...")
    with open(QQ_DATA) as f:
        qq_events = json.load(f)
    print(f"  {len(qq_events)} events")

    id2label = meta["id2label"]
    label2id = meta["label2id"]

    # Load gate mapping
    with open("/workspace/sif/data/classifier-labels.json") as f:
        label_data = json.load(f)
    gate_map = {l["id"]: l["gate"] for l in label_data["labels"]}

    model.cuda()
    results = []
    batch_size = 32
    t0 = time.time()

    for i in range(0, len(qq_events), batch_size):
        batch = qq_events[i : i + batch_size]

        # Narrative only — no structured context (runs 3+4 showed it doesn't generalise)
        texts = [f"{INSTRUCTION}\n{evt['narrative'].lower().strip()}" for evt in batch]

        inputs = tokenizer(
            texts,
            truncation=True,
            max_length=MAX_LENGTH,
            padding=True,
            return_tensors="pt",
        ).to("cuda")

        with torch.no_grad():
            logits = model(**inputs).logits

        probs = torch.softmax(logits, dim=-1)
        pred_ids = logits.argmax(dim=-1).cpu().tolist()
        confidences = probs.max(dim=-1).values.cpu().tolist()

        for j, evt in enumerate(batch):
            pred_label = id2label[str(pred_ids[j])]
            pred_gate = gate_map.get(pred_label, "UNKNOWN")
            results.append(
                {
                    "event_id": evt.get("event_id", f"qq_{i+j}"),
                    "qq_sifp": evt.get("qq_sifp", ""),
                    "qq_report_type": evt.get("report_type", ""),
                    "qq_hazard": evt.get("hazard_category", ""),
                    "predicted_mechanism": pred_label,
                    "predicted_gate": pred_gate,
                    "confidence": round(confidences[j], 4),
                    "narrative_preview": evt["narrative"][:100],
                }
            )

    elapsed = time.time() - t0
    print(f"  Inference: {elapsed:.1f}s ({len(qq_events)/elapsed:.0f} events/s)")

    # Save results
    output_dir = Path(f"/workspace/sif/output/{run_name}")
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "qq_mechanism_predictions.json"
    with open(output_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"  → {output_path}")

    # Summary
    from collections import Counter

    pred_gates = Counter(r["predicted_gate"] for r in results)
    pred_mechs = Counter(r["predicted_mechanism"] for r in results)

    print(f"\n=== QQ Prediction Summary ===")
    print(f"Gate distribution:")
    for gate, n in pred_gates.most_common():
        print(f"  {gate:20s} {n:>5} ({100*n/len(results):.1f}%)")

    print(f"\nTop 10 predicted mechanisms:")
    for mech, n in pred_mechs.most_common(10):
        print(f"  {mech:25s} {n:>5} ({100*n/len(results):.1f}%)")

    # Cross-tab: QQ SIFp vs predicted gate
    print(f"\n=== QQ SIFp vs Predicted Gate ===")
    sifp_labels = sorted(set(r["qq_sifp"] for r in results))
    print(f"{'QQ SIFp':>20s}  {'NEEDS_ASSESSMENT':>17s}  {'AUTO_NON_SIF':>13s}  {'Total':>6s}")
    print("-" * 65)
    for sifp in sifp_labels:
        subset = [r for r in results if r["qq_sifp"] == sifp]
        na = sum(1 for r in subset if r["predicted_gate"] == "NEEDS_ASSESSMENT")
        ns = sum(1 for r in subset if r["predicted_gate"] == "AUTO_NON_SIF")
        print(f"{sifp:>20s}  {na:>17}  {ns:>13}  {len(subset):>6}")

    return results


def save_merged(model, tokenizer, meta, run_name):
    """Save LoRA-merged model as safetensors for later ONNX conversion."""
    merged_dir = f"/workspace/sif/models/mechanism-{run_name}-merged"
    print(f"\nSaving merged model to {merged_dir}...")
    model.save_pretrained(merged_dir)
    tokenizer.save_pretrained(merged_dir)
    with open(f"{merged_dir}/training_meta.json", "w") as f:
        json.dump(meta, f, indent=2)
    size_mb = sum(f.stat().st_size for f in Path(merged_dir).rglob("*")) / 1024 / 1024
    print(f"  Merged model: {size_mb:.0f} MB")
    return size_mb


def main():
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=str, required=True, help="Run name (e.g. run5)")
    args = parser.parse_args()

    model_dir = f"/workspace/sif/models/mechanism-{args.run}"
    print(f"Run: {args.run} → {model_dir}")

    model, tokenizer, meta = load_model(model_dir)

    # Save merged model (ONNX deferred — DynamicCache incompatible with legacy tracer)
    save_merged(model, tokenizer, meta, args.run)

    # QQ bulk inference
    model.cuda()
    results = run_qq_inference(model, tokenizer, meta, args.run)

    print(f"\nDone. QQ predictions: {len(results)}")


if __name__ == "__main__":
    main()
