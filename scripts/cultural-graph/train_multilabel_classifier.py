#!/usr/bin/env python3
"""Fine-tune MiniLM for multi-label cultural edge type classification.

12 binary outputs: for each narrative, predict which cultural edge types are present.
Labels derived from production extraction results.

Usage:
    python3 -u train_multilabel_classifier.py
"""

import json
import glob
import random
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
from torch.utils.data import Dataset, DataLoader
from transformers import AutoTokenizer, AutoModel
from sklearn.metrics import f1_score, precision_score, recall_score

DATA_PATH = Path("/workspace/cultural-graph/classifier-voice-drift-care.parquet")
PRODUCTION_DIR = Path("/workspace/cultural-graph/production-output")
OUTPUT_DIR = Path("/workspace/cultural-graph/output/classifier-multilabel")
ENCODER_MODEL = "sentence-transformers/all-MiniLM-L6-v2"
EPOCHS = 10
BATCH_SIZE = 64
LR = 2e-5
TEST_FRACTION = 0.15
SEED = 42

EDGE_TYPES = [
    "shares-information-with", "speaks-up-to", "responds-to-failure-of",
    "directs", "cooperates-with", "protects", "monitors", "normalises",
    "adapts-to", "learns-from", "recognises", "cares-for",
]


def mean_pooling(model_output, attention_mask):
    token_embeddings = model_output[0]
    input_mask_expanded = attention_mask.unsqueeze(-1).expand(token_embeddings.size()).float()
    return torch.sum(token_embeddings * input_mask_expanded, 1) / torch.clamp(input_mask_expanded.sum(1), min=1e-9)


class CulturalClassifier(nn.Module):
    def __init__(self, encoder, tokenizer, n_labels=12):
        super().__init__()
        self.encoder = encoder
        self.tokenizer = tokenizer
        self.head = nn.Sequential(
            nn.Linear(384, 256),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(256, 128),
            nn.ReLU(),
            nn.Dropout(0.1),
            nn.Linear(128, n_labels),
        )

    def forward(self, texts):
        encoded = self.tokenizer(texts, padding=True, truncation=True, max_length=256, return_tensors="pt")
        encoded = {k: v.to(self.encoder.device) for k, v in encoded.items()}
        output = self.encoder(**encoded)
        embeddings = mean_pooling(output, encoded["attention_mask"])
        embeddings = torch.nn.functional.normalize(embeddings, p=2, dim=1)
        return self.head(embeddings)


def load_data():
    import pyarrow.parquet as pq

    t = pq.read_table(str(DATA_PATH))
    id_to_text = {}
    for i in range(t.num_rows):
        nid = t.column("id")[i].as_py()
        text = t.column("narrative_text")[i].as_py()
        id_to_text[nid] = text

    edge_labels = {}
    for path in sorted(glob.glob(str(PRODUCTION_DIR / "cultural-graph-*.jsonl"))):
        with open(path) as f:
            for line in f:
                r = json.loads(line)
                if not r["valid"]:
                    continue
                nid = r["id"]
                types_present = set()
                for rel in r["extraction"].get("relationships", []):
                    et = rel.get("edge_type", "")
                    if et in EDGE_TYPES:
                        types_present.add(et)
                label = [1.0 if et in types_present else 0.0 for et in EDGE_TYPES]
                edge_labels[nid] = label

    texts = []
    labels = []
    for nid, text in id_to_text.items():
        if nid in edge_labels:
            texts.append(text)
            labels.append(edge_labels[nid])

    labels = np.array(labels, dtype=np.float32)
    print(f"Matched records: {len(texts)}")
    print(f"Label distribution (% positive per type):")
    for i, et in enumerate(EDGE_TYPES):
        pct = 100 * labels[:, i].mean()
        print(f"  {et:<28} {pct:.1f}%")

    random.seed(SEED)
    indices = list(range(len(texts)))
    random.shuffle(indices)
    n_test = int(len(indices) * TEST_FRACTION)

    return (
        [texts[i] for i in indices[n_test:]], labels[indices[n_test:]],
        [texts[i] for i in indices[:n_test]], labels[indices[:n_test]],
    )


class NarrativeDataset(Dataset):
    def __init__(self, texts, labels):
        self.texts = texts
        self.labels = torch.tensor(labels, dtype=torch.float32)
    def __len__(self):
        return len(self.texts)
    def __getitem__(self, idx):
        return self.texts[idx], self.labels[idx]


def collate_fn(batch):
    texts, labels = zip(*batch)
    return list(texts), torch.stack(labels)


def evaluate(model, loader, device):
    model.eval()
    all_preds, all_labels = [], []
    with torch.no_grad():
        for texts, labels in loader:
            logits = model(texts)
            preds = (torch.sigmoid(logits) > 0.5).float()
            all_preds.append(preds.cpu().numpy())
            all_labels.append(labels.numpy())

    all_preds = np.concatenate(all_preds)
    all_labels = np.concatenate(all_labels)

    per_type = {}
    for i, et in enumerate(EDGE_TYPES):
        if all_labels[:, i].sum() > 0:
            per_type[et] = {
                "f1": f1_score(all_labels[:, i], all_preds[:, i], zero_division=0),
                "precision": precision_score(all_labels[:, i], all_preds[:, i], zero_division=0),
                "recall": recall_score(all_labels[:, i], all_preds[:, i], zero_division=0),
            }

    macro_f1 = np.mean([v["f1"] for v in per_type.values()])
    return macro_f1, per_type


def main():
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Device: {device}")

    train_texts, train_labels, test_texts, test_labels = load_data()
    print(f"\nTrain: {len(train_texts)}, Test: {len(test_texts)}")

    train_ds = NarrativeDataset(train_texts, train_labels)
    test_ds = NarrativeDataset(test_texts, test_labels)
    train_loader = DataLoader(train_ds, batch_size=BATCH_SIZE, shuffle=True, collate_fn=collate_fn)
    test_loader = DataLoader(test_ds, batch_size=BATCH_SIZE, shuffle=False, collate_fn=collate_fn)

    print(f"\nLoading {ENCODER_MODEL}...")
    tokenizer = AutoTokenizer.from_pretrained(ENCODER_MODEL)
    encoder = AutoModel.from_pretrained(ENCODER_MODEL).to(device)
    model = CulturalClassifier(encoder, tokenizer, n_labels=len(EDGE_TYPES)).to(device)

    optimizer = torch.optim.AdamW([
        {"params": model.encoder.parameters(), "lr": LR},
        {"params": model.head.parameters(), "lr": LR * 10},
    ])

    pos_counts = train_labels.sum(axis=0)
    neg_counts = len(train_labels) - pos_counts
    pos_weight = torch.tensor(neg_counts / np.maximum(pos_counts, 1), dtype=torch.float32).to(device)
    criterion = nn.BCEWithLogitsLoss(pos_weight=pos_weight)

    print(f"\nTraining {EPOCHS} epochs")
    t0 = time.time()
    best_f1 = 0
    best_per_type = {}

    for epoch in range(EPOCHS):
        model.train()
        total_loss = 0
        n_batches = 0
        for texts, labels in train_loader:
            labels = labels.to(device)
            logits = model(texts)
            loss = criterion(logits, labels)
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            total_loss += loss.item()
            n_batches += 1

        macro_f1, per_type = evaluate(model, test_loader, device)
        elapsed = time.time() - t0
        print(f"Epoch {epoch+1}/{EPOCHS} ({elapsed:.0f}s) — "
              f"loss={total_loss/n_batches:.4f} | macro F1={macro_f1:.3f}")

        if macro_f1 > best_f1:
            best_f1 = macro_f1
            best_per_type = per_type
            OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
            model.encoder.save_pretrained(OUTPUT_DIR / "encoder")
            tokenizer.save_pretrained(OUTPUT_DIR / "encoder")
            torch.save(model.head.state_dict(), OUTPUT_DIR / "head.pt")

    # Export ONNX
    dummy = torch.randn(1, 384).to(device)
    model.head.eval()
    torch.onnx.export(
        model.head, dummy,
        str(OUTPUT_DIR / "cultural-edge-classifier.onnx"),
        input_names=["embedding"],
        output_names=["logits"],
        dynamic_axes={"embedding": {0: "batch"}, "logits": {0: "batch"}},
    )

    sep = "=" * 60
    print(f"\n{sep}")
    print(f"BEST RESULTS (macro F1={best_f1:.3f})")
    print(f"{sep}")
    print(f"{'Edge type':<28} {'F1':>6} {'Prec':>6} {'Rec':>6}")
    print("-" * 48)
    for et in EDGE_TYPES:
        if et in best_per_type:
            m = best_per_type[et]
            print(f"{et:<28} {m['f1']:>6.3f} {m['precision']:>6.3f} {m['recall']:>6.3f}")
    print(f"{'MACRO AVERAGE':<28} {best_f1:>6.3f}")

    head_size = (OUTPUT_DIR / "cultural-edge-classifier.onnx").stat().st_size / 1024
    meta = {
        "encoder": ENCODER_MODEL,
        "encoder_finetuned": True,
        "task": "multi-label binary classification",
        "labels": EDGE_TYPES,
        "threshold": 0.5,
        "best_macro_f1": round(best_f1, 4),
        "per_type_f1": {et: round(best_per_type[et]["f1"], 4) for et in best_per_type},
        "onnx_head_kb": round(head_size, 1),
    }
    with open(OUTPUT_DIR / "classifier-meta.json", "w") as f:
        json.dump(meta, f, indent=2)

    print(f"\nONNX head: {head_size:.0f} KB")
    print(f"Saved to: {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
