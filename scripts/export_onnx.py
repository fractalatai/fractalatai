#!/usr/bin/env python3
"""Export trained DRRP model to ONNX with optional INT8 quantisation.

Reads a PyTorch checkpoint from `train_drrp_model.py` and exports to ONNX.
Optionally quantises to INT8 for edge deployment.

Usage:
  python scripts/export_onnx.py \
    --model models/deberta-v3-drrp \
    --output models/deberta-v3-drrp/model.onnx \
    --quantize

  # Validate the exported model:
  python scripts/export_onnx.py \
    --model models/deberta-v3-drrp \
    --output models/deberta-v3-drrp/model.onnx \
    --validate
"""

import argparse
import json

# Import the model class from the training script.
import sys
from pathlib import Path

import numpy as np
import onnx
import torch
from transformers import AutoTokenizer

sys.path.insert(0, str(Path(__file__).parent))
from train_drrp_model import DrrpExtractorModel


def export_to_onnx(model, tokenizer, output_path: Path, max_length: int = 512):
    """Export PyTorch model to ONNX format."""
    model.eval()
    device = next(model.parameters()).device

    # Create dummy inputs.
    dummy_text = "DUTY : Org: Employer"
    dummy_source = "The employer shall ensure the safety of employees."
    encoding = tokenizer(
        dummy_text,
        dummy_source,
        max_length=max_length,
        truncation="only_second",
        padding="max_length",
        return_tensors="pt",
    )

    input_ids = encoding["input_ids"].to(device)
    attention_mask = encoding["attention_mask"].to(device)

    # Export using legacy exporter for broad ONNX Runtime compatibility.
    torch.onnx.export(
        model,
        (input_ids, attention_mask),
        str(output_path),
        input_names=["input_ids", "attention_mask"],
        output_names=[
            "clause_start_logits",
            "clause_end_logits",
            "qualifier_start_logits",
            "qualifier_end_logits",
            "has_qualifier_logits",
            "holder_logits",
        ],
        dynamic_axes={
            "input_ids": {0: "batch_size"},
            "attention_mask": {0: "batch_size"},
            "clause_start_logits": {0: "batch_size"},
            "clause_end_logits": {0: "batch_size"},
            "qualifier_start_logits": {0: "batch_size"},
            "qualifier_end_logits": {0: "batch_size"},
            "has_qualifier_logits": {0: "batch_size"},
            "holder_logits": {0: "batch_size"},
        },
        opset_version=17,
        do_constant_folding=True,
        dynamo=False,
    )
    print(f"Exported ONNX model to {output_path}")
    print(f"  Size: {output_path.stat().st_size / 1024 / 1024:.1f} MB")


def quantize_int8(input_path: Path, output_path: Path):
    """Quantise ONNX model to INT8."""
    from onnxruntime.quantization import QuantType, quantize_dynamic

    quantize_dynamic(
        str(input_path),
        str(output_path),
        weight_type=QuantType.QInt8,
    )
    print(f"Quantised to INT8: {output_path}")
    print(f"  Size: {output_path.stat().st_size / 1024 / 1024:.1f} MB")


def validate_onnx(
    onnx_path: Path, tokenizer, holder_labels: list, max_length: int = 512
):
    """Run a smoke test on the exported ONNX model."""
    import onnxruntime as ort

    # Check model structure.
    model = onnx.load(str(onnx_path))
    onnx.checker.check_model(model)
    print(f"ONNX model valid: {onnx_path}")

    # Run inference.
    session = ort.InferenceSession(str(onnx_path))

    query = "DUTY : Org: Employer"
    source = (
        "It shall be the duty of every employer to ensure, "
        "so far as is reasonably practicable, the health, "
        "safety and welfare at work of all his employees."
    )

    encoding = tokenizer(
        query,
        source,
        max_length=max_length,
        truncation="only_second",
        padding="max_length",
        return_tensors="np",
    )

    outputs = session.run(
        None,
        {
            "input_ids": encoding["input_ids"],
            "attention_mask": encoding["attention_mask"],
        },
    )

    clause_start_logits, clause_end_logits = outputs[0], outputs[1]
    qual_start_logits, qual_end_logits = outputs[2], outputs[3]
    has_qual_logits = outputs[4]
    holder_logits = outputs[5]

    # Decode results.
    clause_start = int(np.argmax(clause_start_logits[0]))
    clause_end = int(np.argmax(clause_end_logits[0]))
    has_qualifier = bool(np.argmax(has_qual_logits[0]) == 1)
    holder_idx = int(np.argmax(holder_logits[0]))
    holder = holder_labels[holder_idx] if holder_idx < len(holder_labels) else "Unknown"

    # Convert token positions back to text.
    tokens = tokenizer.convert_ids_to_tokens(encoding["input_ids"][0])
    clause_tokens = tokens[clause_start : clause_end + 1]
    clause_text = tokenizer.convert_tokens_to_string(clause_tokens)

    print(f"\nSmoke test:")
    print(f"  Input:     {query}")
    print(f"  Source:    {source[:80]}...")
    print(f"  Clause:    [{clause_start}-{clause_end}] {clause_text[:80]}")
    print(f"  Holder:    {holder} (idx={holder_idx})")
    print(f"  Qualifier: {'yes' if has_qualifier else 'no'}")

    latency_ms = benchmark_latency(session, encoding, n=100)
    print(f"  Latency:   {latency_ms:.1f} ms/inference (100 runs)")


def benchmark_latency(session, encoding, n=100):
    """Measure average inference latency."""
    import time

    inputs = {
        "input_ids": encoding["input_ids"],
        "attention_mask": encoding["attention_mask"],
    }
    # Warmup.
    for _ in range(10):
        session.run(None, inputs)
    # Benchmark.
    start = time.perf_counter()
    for _ in range(n):
        session.run(None, inputs)
    elapsed = time.perf_counter() - start
    return (elapsed / n) * 1000


def main():
    parser = argparse.ArgumentParser(description="Export DRRP model to ONNX")
    parser.add_argument(
        "--model", required=True, help="Model directory (from train_drrp_model.py)"
    )
    parser.add_argument("--output", required=True, help="Output ONNX file path")
    parser.add_argument(
        "--checkpoint",
        default="best_model.pt",
        help="Checkpoint file name (default: best_model.pt)",
    )
    parser.add_argument("--quantize", action="store_true", help="Quantise to INT8")
    parser.add_argument("--validate", action="store_true", help="Validate after export")
    args = parser.parse_args()

    model_dir = Path(args.model)
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Load metadata.
    with open(model_dir / "metadata.json") as f:
        metadata = json.load(f)

    with open(model_dir / "holder_labels.json") as f:
        holder_labels = json.load(f)

    base_model = metadata["base_model"]
    num_holder_classes = metadata["num_holder_classes"]
    max_length = metadata["max_length"]

    # Load tokenizer and save locally for Rust inference.
    print(f"Loading tokenizer: {base_model}")
    tokenizer = AutoTokenizer.from_pretrained(base_model)
    tokenizer.save_pretrained(str(model_dir))
    print(f"Saved tokenizer to {model_dir}")

    # Load model.
    checkpoint_path = model_dir / args.checkpoint
    if not checkpoint_path.exists():
        checkpoint_path = model_dir / "final_model.pt"
    print(f"Loading checkpoint: {checkpoint_path}")

    model = DrrpExtractorModel(base_model, num_holder_classes)
    model.load_state_dict(
        torch.load(checkpoint_path, map_location="cpu", weights_only=True)
    )
    model.eval()

    # Export to ONNX.
    export_to_onnx(model, tokenizer, output_path, max_length)

    # Optionally quantise.
    if args.quantize:
        quant_path = output_path.with_suffix(".int8.onnx")
        quantize_int8(output_path, quant_path)

    # Optionally validate.
    if args.validate:
        target = output_path.with_suffix(".int8.onnx") if args.quantize else output_path
        validate_onnx(target, tokenizer, holder_labels, max_length)


if __name__ == "__main__":
    main()
