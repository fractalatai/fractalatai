#!/usr/bin/env python3
"""Fine-tune Qwen 3 0.6B for SIF mechanism classification via LoRA.

20-class single-label text classification from incident narratives.
Runs on RunPod GPU. Outputs LoRA adapter + ONNX export.

Usage (on RunPod):
    python3 /workspace/sif/scripts/finetune_mechanism.py
    python3 /workspace/sif/scripts/finetune_mechanism.py --epochs 5 --lr 2e-4
"""

import argparse
import json
import os
import sys

import numpy as np
import pyarrow.parquet as pq
import torch
from datasets import Dataset
from peft import LoraConfig, TaskType, get_peft_model
from sklearn.metrics import classification_report, f1_score
from transformers import (
    AutoModelForSequenceClassification,
    AutoTokenizer,
    Trainer,
    TrainingArguments,
)

DATA_DIR = "/workspace/sif/data"
OUTPUT_BASE = "/workspace/sif/models"
LABELS_FILE = f"{DATA_DIR}/classifier-labels.json"

# Model
MODEL_NAME = "Qwen/Qwen3-1.7B"
MAX_LENGTH = 512  # Most narratives are <300 chars; 512 tokens is generous


def load_data():
    """Load train/val Parquet files and build label mappings."""
    train_table = pq.read_table(f"{DATA_DIR}/train.parquet")
    val_table = pq.read_table(f"{DATA_DIR}/val.parquet")

    train_df = train_table.to_pandas()
    val_df = val_table.to_pandas()

    # Build label → id mapping from classifier-labels.json
    with open(LABELS_FILE) as f:
        label_data = json.load(f)

    label_names = sorted([l["id"] for l in label_data["labels"]])
    label2id = {name: i for i, name in enumerate(label_names)}
    id2label = {i: name for name, i in label2id.items()}

    print(f"Labels: {len(label_names)}")
    print(f"Train: {len(train_df):,}  Val: {len(val_df):,}")

    # Convert to HuggingFace datasets
    train_ds = Dataset.from_pandas(train_df[["text", "label"]].rename(columns={"text": "text", "label": "label_name"}))
    val_ds = Dataset.from_pandas(val_df[["text", "label"]].rename(columns={"text": "text", "label": "label_name"}))

    # Map string labels to integer ids
    train_ds = train_ds.map(lambda x: {"label": label2id[x["label_name"]]})
    val_ds = val_ds.map(lambda x: {"label": label2id[x["label_name"]]})

    return train_ds, val_ds, label2id, id2label, label_names


def compute_class_weights(train_ds, num_labels):
    """Compute inverse-frequency weights for weighted loss."""
    labels = train_ds["label"]
    counts = np.bincount(labels, minlength=num_labels)
    # Inverse frequency, normalised so mean weight = 1
    weights = 1.0 / (counts + 1)  # +1 to avoid div by zero
    weights = weights / weights.mean()
    return torch.tensor(weights, dtype=torch.float32)


class WeightedTrainer(Trainer):
    """Trainer with class-weighted cross-entropy loss."""

    def __init__(self, class_weights=None, **kwargs):
        super().__init__(**kwargs)
        self.class_weights = class_weights

    def compute_loss(self, model, inputs, return_outputs=False, **kwargs):
        labels = inputs.pop("labels")
        outputs = model(**inputs)
        logits = outputs.logits

        if self.class_weights is not None:
            weight = self.class_weights.to(logits.device)
            loss_fn = torch.nn.CrossEntropyLoss(weight=weight)
        else:
            loss_fn = torch.nn.CrossEntropyLoss()

        loss = loss_fn(logits, labels)
        return (loss, outputs) if return_outputs else loss


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=str, required=True, help="Run name for output namespace (e.g. run5)")
    parser.add_argument("--epochs", type=int, default=3)
    parser.add_argument("--lr", type=float, default=2e-4)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--lora-r", type=int, default=16)
    parser.add_argument("--lora-alpha", type=int, default=32)
    parser.add_argument("--no-weighted-loss", action="store_true")
    args = parser.parse_args()

    OUTPUT_DIR = f"{OUTPUT_BASE}/mechanism-{args.run}"
    print(f"Run: {args.run} → {OUTPUT_DIR}")
    print(f"GPU: {torch.cuda.get_device_name(0)}")
    print(f"VRAM: {torch.cuda.get_device_properties(0).total_memory / 1024**3:.1f} GB")
    print()

    # Load data
    train_ds, val_ds, label2id, id2label, label_names = load_data()
    num_labels = len(label_names)

    # Load tokenizer and model
    print(f"\nLoading {MODEL_NAME}...")
    tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME, trust_remote_code=True)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    model = AutoModelForSequenceClassification.from_pretrained(
        MODEL_NAME,
        num_labels=num_labels,
        id2label=id2label,
        label2id=label2id,
        trust_remote_code=True,
    )
    model.config.pad_token_id = tokenizer.pad_token_id

    # Apply LoRA
    lora_config = LoraConfig(
        task_type=TaskType.SEQ_CLS,
        r=args.lora_r,
        lora_alpha=args.lora_alpha,
        lora_dropout=0.1,
        target_modules=["q_proj", "v_proj", "k_proj", "o_proj"],
    )
    model = get_peft_model(model, lora_config)
    model.print_trainable_parameters()

    # Tokenize
    def tokenize(batch):
        return tokenizer(
            batch["text"],
            truncation=True,
            max_length=MAX_LENGTH,
            padding="max_length",
        )

    train_ds = train_ds.map(tokenize, batched=True, remove_columns=["text", "label_name"])
    val_ds = val_ds.map(tokenize, batched=True, remove_columns=["text", "label_name"])

    train_ds.set_format("torch")
    val_ds.set_format("torch")

    # Class weights
    class_weights = None
    if not args.no_weighted_loss:
        class_weights = compute_class_weights(train_ds, num_labels)
        print(f"\nClass weights (top 5 heaviest):")
        sorted_weights = sorted(
            [(id2label[i], w.item()) for i, w in enumerate(class_weights)],
            key=lambda x: x[1],
            reverse=True,
        )
        for name, w in sorted_weights[:5]:
            print(f"  {name:25s} {w:.2f}")

    # Metrics
    def compute_metrics(eval_pred):
        logits, labels = eval_pred
        preds = np.argmax(logits, axis=-1)
        macro_f1 = f1_score(labels, preds, average="macro")
        micro_f1 = f1_score(labels, preds, average="micro")
        return {"macro_f1": macro_f1, "micro_f1": micro_f1}

    # Training arguments
    # Warmup: 10% of total steps
    steps_per_epoch = len(train_ds) // args.batch_size
    total_steps = steps_per_epoch * args.epochs
    warmup_steps = int(total_steps * 0.1)

    training_args = TrainingArguments(
        output_dir=OUTPUT_DIR,
        num_train_epochs=args.epochs,
        per_device_train_batch_size=args.batch_size,
        gradient_accumulation_steps=1,  # batch_size=16 fits in 32GB VRAM (RTX 5090)
        per_device_eval_batch_size=args.batch_size,
        learning_rate=args.lr,
        weight_decay=0.01,
        eval_strategy="epoch",
        save_strategy="epoch",
        load_best_model_at_end=True,
        metric_for_best_model="macro_f1",
        greater_is_better=True,
        logging_steps=50,
        warmup_steps=warmup_steps,
        bf16=True,
        report_to="none",
        save_total_limit=2,
    )

    # Train
    trainer = WeightedTrainer(
        class_weights=class_weights,
        model=model,
        args=training_args,
        train_dataset=train_ds,
        eval_dataset=val_ds,
        compute_metrics=compute_metrics,
    )

    print(f"\nTraining: {args.epochs} epochs, lr={args.lr}, batch={args.batch_size}")
    print(f"LoRA: r={args.lora_r}, alpha={args.lora_alpha}")
    print()

    trainer.train()

    # Final evaluation
    print("\n=== Final Validation Results ===")
    results = trainer.evaluate()
    print(f"Macro F1: {results['eval_macro_f1']:.4f}")
    print(f"Micro F1: {results['eval_micro_f1']:.4f}")

    # Per-class report
    val_preds = trainer.predict(val_ds)
    preds = np.argmax(val_preds.predictions, axis=-1)
    labels = val_preds.label_ids
    report = classification_report(
        labels, preds, target_names=label_names, digits=3, zero_division=0
    )
    print(f"\n{report}")

    # Save
    print(f"\nSaving to {OUTPUT_DIR}...")
    trainer.save_model(OUTPUT_DIR)
    tokenizer.save_pretrained(OUTPUT_DIR)

    # Save label mapping and results
    meta = {
        "model_name": MODEL_NAME,
        "num_labels": num_labels,
        "label2id": label2id,
        "id2label": id2label,
        "macro_f1": results["eval_macro_f1"],
        "micro_f1": results["eval_micro_f1"],
        "epochs": args.epochs,
        "lr": args.lr,
        "lora_r": args.lora_r,
        "lora_alpha": args.lora_alpha,
        "train_size": len(train_ds),
        "val_size": len(val_ds),
        "weighted_loss": not args.no_weighted_loss,
    }
    with open(f"{OUTPUT_DIR}/training_meta.json", "w") as f:
        json.dump(meta, f, indent=2)

    # Save classification report
    with open(f"{OUTPUT_DIR}/classification_report.txt", "w") as f:
        f.write(report)

    print(f"\nDone. Macro F1 = {results['eval_macro_f1']:.4f}")


if __name__ == "__main__":
    main()
