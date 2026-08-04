#!/usr/bin/env python3
"""Train ONNX cultural signal classifier (Voice/Drift/Care).

Input: narrative text → 3 regression outputs (voice, drift, care).
Uses a sentence-transformer encoder + regression head.
Exports to ONNX for millisecond inference on CPU/NPU.

Usage:
    /usr/bin/python3 scripts/cultural-graph/train_classifier.py
    /usr/bin/python3 scripts/cultural-graph/train_classifier.py --test-fraction 0.15
"""

import argparse
import json
import random
import warnings
from pathlib import Path

import numpy as np
import pyarrow.parquet as pq

warnings.filterwarnings("ignore")

DATA_PATH = Path("data/qq/cultural-graph/outputs/training/classifier-voice-drift-care.parquet")
OUTPUT_DIR = Path("data/cultural-graph-models/classifier")
ENCODER_MODEL = "all-MiniLM-L6-v2"  # 22MB, 384-dim


def load_data(test_fraction=0.15, seed=42):
    """Load and split data."""
    t = pq.read_table(DATA_PATH)
    texts = t.column("narrative_text").to_pylist()
    voice = np.array(t.column("voice").to_pylist(), dtype=np.float32)
    drift = np.array(t.column("drift").to_pylist(), dtype=np.float32)
    care = np.array(t.column("care").to_pylist(), dtype=np.float32)
    labels = np.stack([voice, drift, care], axis=1)

    # Stratified split by cultural_edge_count bucket
    edge_counts = t.column("cultural_edge_count").to_pylist()
    indices = list(range(len(texts)))
    random.seed(seed)
    random.shuffle(indices)

    n_test = int(len(indices) * test_fraction)
    test_idx = indices[:n_test]
    train_idx = indices[n_test:]

    return (
        [texts[i] for i in train_idx], labels[train_idx],
        [texts[i] for i in test_idx], labels[test_idx],
    )


def train(train_texts, train_labels, test_texts, test_labels):
    """Train encoder + regression head."""
    from sentence_transformers import SentenceTransformer
    from sklearn.linear_model import Ridge
    from sklearn.metrics import mean_squared_error, r2_score

    print(f"=== Loading encoder: {ENCODER_MODEL} ===")
    encoder = SentenceTransformer(ENCODER_MODEL)

    print(f"=== Encoding {len(train_texts)} train texts ===")
    train_emb = encoder.encode(train_texts, show_progress_bar=True, batch_size=64)
    print(f"=== Encoding {len(test_texts)} test texts ===")
    test_emb = encoder.encode(test_texts, show_progress_bar=True, batch_size=64)

    print(f"\n=== Training regression head ===")
    # One Ridge regressor for all 3 outputs (multi-output)
    model = Ridge(alpha=1.0)
    model.fit(train_emb, train_labels)

    # Evaluate
    train_pred = model.predict(train_emb)
    test_pred = model.predict(test_emb)

    print(f"\n{'Metric':<20} {'Voice':>8} {'Drift':>8} {'Care':>8}")
    print("-" * 46)
    for name, true, pred in [("Train", train_labels, train_pred), ("Test", test_labels, test_pred)]:
        for i, col in enumerate(["voice", "drift", "care"]):
            mse = mean_squared_error(true[:, i], pred[:, i])
            r2 = r2_score(true[:, i], pred[:, i])
            if col == "voice":
                print(f"{name + ' MSE':<20} {mse:>8.4f}", end="")
            else:
                print(f" {mse:>8.4f}", end="")
        print()
        for i, col in enumerate(["voice", "drift", "care"]):
            r2 = r2_score(true[:, i], pred[:, i])
            if col == "voice":
                print(f"{name + ' R²':<20} {r2:>8.4f}", end="")
            else:
                print(f" {r2:>8.4f}", end="")
        print()

    return encoder, model, test_emb, test_labels, test_pred


def export_onnx(encoder, model):
    """Export the regression head to ONNX. The encoder stays as sentence-transformers."""
    import onnx
    from skl2onnx import convert_sklearn
    from skl2onnx.common.data_types import FloatTensorType

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    # Export regression head
    initial_type = [("embedding", FloatTensorType([None, 384]))]
    onnx_model = convert_sklearn(model, initial_types=initial_type)
    onnx_path = OUTPUT_DIR / "voice-drift-care-head.onnx"
    onnx.save(onnx_model, str(onnx_path))
    print(f"\nONNX head: {onnx_path} ({onnx_path.stat().st_size / 1024:.0f} KB)")

    # Save encoder info
    meta = {
        "encoder": ENCODER_MODEL,
        "embedding_dim": 384,
        "outputs": ["voice", "drift", "care"],
        "onnx_head": str(onnx_path.name),
    }
    meta_path = OUTPUT_DIR / "classifier-meta.json"
    with open(meta_path, "w") as f:
        json.dump(meta, f, indent=2)
    print(f"Metadata: {meta_path}")

    # Save the encoder locally for edge deployment
    encoder.save(str(OUTPUT_DIR / "encoder"))
    print(f"Encoder saved: {OUTPUT_DIR / 'encoder'}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--test-fraction", type=float, default=0.15)
    args = parser.parse_args()

    print(f"Loading data from {DATA_PATH}")
    train_texts, train_labels, test_texts, test_labels = load_data(args.test_fraction)
    print(f"Train: {len(train_texts)}, Test: {len(test_texts)}")
    print(f"Labels shape: {train_labels.shape}")

    encoder, model, test_emb, test_labels, test_pred = train(
        train_texts, train_labels, test_texts, test_labels
    )
    export_onnx(encoder, model)
    print("\nDone.")


if __name__ == "__main__":
    main()
