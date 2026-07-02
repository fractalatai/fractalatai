#!/usr/bin/env python3
"""Fine-tune DeBERTa-v3-base for DRRP structured extraction.

Three task heads:
  1. Clause span extraction (start/end token positions in source_text)
  2. Qualifier span extraction (start/end token positions, or no-qualifier)
  3. Holder classification (87+ taxonomy categories)

Reads training data from Parquet files produced by `fractalaw export-training-data`.
Outputs a PyTorch model checkpoint for ONNX export via `export_onnx.py`.

Usage:
  python scripts/train_drrp_model.py \
    --train data/drrp-training/train.parquet \
    --val data/drrp-training/val.parquet \
    --output models/deberta-v3-drrp \
    --epochs 10 --batch-size 16 --lr 2e-5
"""

import argparse
import json
from pathlib import Path

import numpy as np
import pyarrow.parquet as pq
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, Dataset
from transformers import AutoConfig, AutoModel, AutoTokenizer

# ── Model ─────────────────────────────────────────────────────────────


class DrrpExtractorModel(nn.Module):
    """DeBERTa encoder with three task heads."""

    def __init__(self, model_name: str, num_holder_classes: int):
        super().__init__()
        self.config = AutoConfig.from_pretrained(model_name)
        self.encoder = AutoModel.from_pretrained(model_name, torch_dtype=torch.float32)
        hidden = self.config.hidden_size

        # Clause span head: predicts start/end token positions in the sequence.
        self.clause_start = nn.Linear(hidden, 1)
        self.clause_end = nn.Linear(hidden, 1)

        # Qualifier span head: predicts start/end + a no-qualifier flag.
        self.qualifier_start = nn.Linear(hidden, 1)
        self.qualifier_end = nn.Linear(hidden, 1)
        self.has_qualifier = nn.Linear(hidden, 2)  # binary: [no, yes]

        # Holder classification head: over fixed taxonomy.
        self.holder_cls = nn.Linear(hidden, num_holder_classes)

    def forward(self, input_ids, attention_mask, token_type_ids=None):
        outputs = self.encoder(
            input_ids=input_ids,
            attention_mask=attention_mask,
            token_type_ids=token_type_ids,
        )
        seq_out = outputs.last_hidden_state  # (B, T, H)
        cls_out = seq_out[:, 0, :]  # (B, H) — CLS token

        clause_start_logits = self.clause_start(seq_out).squeeze(-1)  # (B, T)
        clause_end_logits = self.clause_end(seq_out).squeeze(-1)  # (B, T)

        qual_start_logits = self.qualifier_start(seq_out).squeeze(-1)  # (B, T)
        qual_end_logits = self.qualifier_end(seq_out).squeeze(-1)  # (B, T)
        has_qual_logits = self.has_qualifier(cls_out)  # (B, 2)

        holder_logits = self.holder_cls(cls_out)  # (B, C)

        return {
            "clause_start_logits": clause_start_logits,
            "clause_end_logits": clause_end_logits,
            "qualifier_start_logits": qual_start_logits,
            "qualifier_end_logits": qual_end_logits,
            "has_qualifier_logits": has_qual_logits,
            "holder_logits": holder_logits,
        }


# ── Dataset ───────────────────────────────────────────────────────────


class DrrpDataset(Dataset):
    """Loads training examples from Parquet and tokenizes for DeBERTa."""

    def __init__(
        self,
        parquet_path: str,
        tokenizer,
        holder_to_idx: dict,
        max_length: int = 512,
        min_match_ratio: float = 0.5,
    ):
        table = pq.read_table(parquet_path)
        self.tokenizer = tokenizer
        self.holder_to_idx = holder_to_idx
        self.max_length = max_length

        # Filter to usable examples (matched to LAT with decent quality).
        self.examples = []
        for i in range(table.num_rows):
            ratio = table.column("match_ratio")[i].as_py()
            clause_start = table.column("clause_start")[i].as_py()
            if clause_start < 0:
                continue  # no match
            if ratio < min_match_ratio:
                continue

            self.examples.append(
                {
                    "drrp_type": table.column("drrp_type")[i].as_py(),
                    "regex_holder": table.column("regex_holder")[i].as_py(),
                    "source_text": table.column("source_text")[i].as_py(),
                    "clause_start": clause_start,
                    "clause_end": table.column("clause_end")[i].as_py(),
                    "holder_label": table.column("holder_label")[i].as_py(),
                    "qualifier_start": table.column("qualifier_start")[i].as_py(),
                    "qualifier_end": table.column("qualifier_end")[i].as_py(),
                }
            )

        print(
            f"  Loaded {len(self.examples)} examples from {parquet_path} "
            f"(filtered from {table.num_rows}, min_ratio={min_match_ratio})"
        )

    def __len__(self):
        return len(self.examples)

    def __getitem__(self, idx):
        ex = self.examples[idx]

        # Build input: [CLS] {drrp_type} : {holder} [SEP] {source_text} [SEP]
        query = f"{ex['drrp_type']} : {ex['regex_holder']}"
        source = ex["source_text"]

        try:
            encoding = self.tokenizer(
                query,
                source,
                max_length=self.max_length,
                truncation="only_second",
                padding="max_length",
                return_offsets_mapping=True,
                return_tensors="pt",
            )
        except Exception:
            # Fallback: truncate source if it's too short for "only_second".
            encoding = self.tokenizer(
                query,
                source,
                max_length=self.max_length,
                truncation=True,
                padding="max_length",
                return_offsets_mapping=True,
                return_tensors="pt",
            )

        input_ids = encoding["input_ids"].squeeze(0)
        attention_mask = encoding["attention_mask"].squeeze(0)
        token_type_ids = encoding.get("token_type_ids")
        if token_type_ids is not None:
            token_type_ids = token_type_ids.squeeze(0)
        offset_mapping = encoding["offset_mapping"].squeeze(0)  # (T, 2)

        # Map char offsets to token positions.
        # offset_mapping[i] = (char_start, char_end) for token i.
        # We need to find which tokens fall within the clause/qualifier spans.
        # The source text starts after the [SEP] token — we need to adjust
        # char offsets to be relative to the source_text portion.

        # Find the start of the second segment (source_text) in the token sequence.
        # token_type_ids == 1 marks the second segment for some models.
        # For DeBERTa-v3, we don't get token_type_ids, so find the second [SEP].
        sep_token_id = self.tokenizer.sep_token_id
        sep_positions = (input_ids == sep_token_id).nonzero(as_tuple=True)[0]
        if len(sep_positions) >= 1:
            # Source text tokens start after the first [SEP].
            source_start_tok = sep_positions[0].item() + 1
        else:
            source_start_tok = 0

        # Convert char offsets to token indices within the source text.
        clause_start_tok = self._char_to_token(
            offset_mapping, ex["clause_start"], source_start_tok, find_start=True
        )
        clause_end_tok = self._char_to_token(
            offset_mapping, ex["clause_end"], source_start_tok, find_start=False
        )

        has_qualifier = ex["qualifier_start"] >= 0
        if has_qualifier:
            qual_start_tok = self._char_to_token(
                offset_mapping, ex["qualifier_start"], source_start_tok, find_start=True
            )
            qual_end_tok = self._char_to_token(
                offset_mapping, ex["qualifier_end"], source_start_tok, find_start=False
            )
        else:
            qual_start_tok = 0
            qual_end_tok = 0

        holder_idx = self.holder_to_idx.get(ex["holder_label"], 0)

        result = {
            "input_ids": input_ids,
            "attention_mask": attention_mask,
            "clause_start": torch.tensor(clause_start_tok, dtype=torch.long),
            "clause_end": torch.tensor(clause_end_tok, dtype=torch.long),
            "qualifier_start": torch.tensor(qual_start_tok, dtype=torch.long),
            "qualifier_end": torch.tensor(qual_end_tok, dtype=torch.long),
            "has_qualifier": torch.tensor(1 if has_qualifier else 0, dtype=torch.long),
            "holder_label": torch.tensor(holder_idx, dtype=torch.long),
        }
        if token_type_ids is not None:
            result["token_type_ids"] = token_type_ids
        return result

    def _char_to_token(self, offset_mapping, char_pos, source_start_tok, find_start):
        """Map a char-level offset in source_text to a token index."""
        for tok_idx in range(source_start_tok, len(offset_mapping)):
            start, end = offset_mapping[tok_idx].tolist()
            if start == 0 and end == 0:
                continue  # padding or special token
            if find_start and start <= char_pos < end:
                return tok_idx
            if not find_start and start < char_pos <= end:
                return tok_idx
        # Fallback: return last non-padding token.
        for tok_idx in range(len(offset_mapping) - 1, source_start_tok - 1, -1):
            start, end = offset_mapping[tok_idx].tolist()
            if start != 0 or end != 0:
                return tok_idx
        return source_start_tok


# ── Training ──────────────────────────────────────────────────────────


def train_epoch(model, dataloader, optimizer, device):
    model.train()
    total_loss = 0
    ce_span = nn.CrossEntropyLoss(ignore_index=-1)
    ce_cls = nn.CrossEntropyLoss()
    n_batches = 0

    for batch in dataloader:
        input_ids = batch["input_ids"].to(device)
        attention_mask = batch["attention_mask"].to(device)
        token_type_ids = batch.get("token_type_ids")
        if token_type_ids is not None:
            token_type_ids = token_type_ids.to(device)

        outputs = model(input_ids, attention_mask, token_type_ids)

        # Clause span loss.
        clause_start_loss = ce_span(
            outputs["clause_start_logits"], batch["clause_start"].to(device)
        )
        clause_end_loss = ce_span(
            outputs["clause_end_logits"], batch["clause_end"].to(device)
        )

        # Qualifier span loss (only for examples that have qualifiers).
        has_qual = batch["has_qualifier"].to(device)
        qual_mask = has_qual == 1
        if qual_mask.any():
            qual_start_loss = ce_span(
                outputs["qualifier_start_logits"][qual_mask],
                batch["qualifier_start"].to(device)[qual_mask],
            )
            qual_end_loss = ce_span(
                outputs["qualifier_end_logits"][qual_mask],
                batch["qualifier_end"].to(device)[qual_mask],
            )
        else:
            qual_start_loss = torch.tensor(0.0, device=device)
            qual_end_loss = torch.tensor(0.0, device=device)

        has_qual_loss = ce_cls(outputs["has_qualifier_logits"], has_qual)

        # Holder classification loss.
        holder_loss = ce_cls(outputs["holder_logits"], batch["holder_label"].to(device))

        # Combined loss with weighting.
        loss = (
            clause_start_loss
            + clause_end_loss
            + 0.5 * (qual_start_loss + qual_end_loss + has_qual_loss)
            + 0.3 * holder_loss
        )

        optimizer.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=1.0)
        optimizer.step()

        total_loss += loss.item()
        n_batches += 1

    return total_loss / max(n_batches, 1)


def evaluate(model, dataloader, device):
    model.eval()
    clause_correct = 0
    holder_correct = 0
    total = 0

    with torch.no_grad():
        for batch in dataloader:
            input_ids = batch["input_ids"].to(device)
            attention_mask = batch["attention_mask"].to(device)
            token_type_ids = batch.get("token_type_ids")
            if token_type_ids is not None:
                token_type_ids = token_type_ids.to(device)

            outputs = model(input_ids, attention_mask, token_type_ids)

            # Clause span accuracy (both start and end must match).
            pred_start = outputs["clause_start_logits"].argmax(dim=-1)
            pred_end = outputs["clause_end_logits"].argmax(dim=-1)
            true_start = batch["clause_start"].to(device)
            true_end = batch["clause_end"].to(device)
            clause_correct += (
                ((pred_start == true_start) & (pred_end == true_end)).sum().item()
            )

            # Holder accuracy.
            pred_holder = outputs["holder_logits"].argmax(dim=-1)
            true_holder = batch["holder_label"].to(device)
            holder_correct += (pred_holder == true_holder).sum().item()

            total += input_ids.size(0)

    clause_acc = clause_correct / max(total, 1)
    holder_acc = holder_correct / max(total, 1)
    return clause_acc, holder_acc


# ── Main ──────────────────────────────────────────────────────────────


def main():
    parser = argparse.ArgumentParser(
        description="Fine-tune DeBERTa for DRRP extraction"
    )
    parser.add_argument("--train", required=True, help="Path to train.parquet")
    parser.add_argument("--val", default=None, help="Path to val.parquet (optional)")
    parser.add_argument(
        "--output", required=True, help="Output directory for model checkpoint"
    )
    parser.add_argument(
        "--base-model",
        default="microsoft/deberta-v3-base",
        help="HuggingFace model name",
    )
    parser.add_argument("--epochs", type=int, default=10)
    parser.add_argument("--batch-size", type=int, default=16)
    parser.add_argument("--lr", type=float, default=2e-5)
    parser.add_argument("--max-length", type=int, default=512)
    parser.add_argument(
        "--min-match-ratio",
        type=float,
        default=0.5,
        help="Minimum clause match ratio for training examples",
    )
    args = parser.parse_args()

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Device: {device}")

    # Load tokenizer.
    print(f"Loading tokenizer: {args.base_model}")
    tokenizer = AutoTokenizer.from_pretrained(args.base_model)

    # Build holder label vocabulary from training data.
    print("Building holder vocabulary...")
    train_table = pq.read_table(args.train)
    holders = set()
    for i in range(train_table.num_rows):
        holders.add(train_table.column("holder_label")[i].as_py())
    holder_labels = sorted(holders)
    holder_to_idx = {h: i for i, h in enumerate(holder_labels)}
    num_holder_classes = len(holder_labels)
    print(f"  {num_holder_classes} holder categories")

    # Create datasets.
    print("Loading training data...")
    train_dataset = DrrpDataset(
        args.train,
        tokenizer,
        holder_to_idx,
        max_length=args.max_length,
        min_match_ratio=args.min_match_ratio,
    )
    train_loader = DataLoader(
        train_dataset,
        batch_size=args.batch_size,
        shuffle=True,
        num_workers=0,
    )

    val_loader = None
    if args.val and Path(args.val).exists():
        print("Loading validation data...")
        val_dataset = DrrpDataset(
            args.val,
            tokenizer,
            holder_to_idx,
            max_length=args.max_length,
            min_match_ratio=0.0,  # include all val examples
        )
        val_loader = DataLoader(
            val_dataset,
            batch_size=args.batch_size,
            shuffle=False,
            num_workers=0,
        )

    # Create model.
    print(f"Loading model: {args.base_model}")
    model = DrrpExtractorModel(args.base_model, num_holder_classes)
    model.to(device)

    # Optimizer with differential learning rate:
    # encoder gets lower LR, task heads get higher LR.
    encoder_params = list(model.encoder.parameters())
    head_params = (
        list(model.clause_start.parameters())
        + list(model.clause_end.parameters())
        + list(model.qualifier_start.parameters())
        + list(model.qualifier_end.parameters())
        + list(model.has_qualifier.parameters())
        + list(model.holder_cls.parameters())
    )
    optimizer = torch.optim.AdamW(
        [
            {"params": encoder_params, "lr": args.lr},
            {"params": head_params, "lr": args.lr * 10},
        ],
        weight_decay=0.01,
    )

    # Training loop.
    print(f"\n{'=' * 60}")
    print(
        f"Training: {len(train_dataset)} examples, {args.epochs} epochs, "
        f"batch_size={args.batch_size}, lr={args.lr}"
    )
    print(f"{'=' * 60}\n")

    best_clause_acc = 0.0
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    for epoch in range(1, args.epochs + 1):
        avg_loss = train_epoch(model, train_loader, optimizer, device)

        msg = f"Epoch {epoch:>2}/{args.epochs}  loss={avg_loss:.4f}"

        if val_loader:
            clause_acc, holder_acc = evaluate(model, val_loader, device)
            msg += f"  clause_acc={clause_acc:.3f}  holder_acc={holder_acc:.3f}"

            if clause_acc > best_clause_acc:
                best_clause_acc = clause_acc
                torch.save(model.state_dict(), output_dir / "best_model.pt")
                msg += "  *best*"
        else:
            # No validation set — evaluate on training data periodically.
            if epoch % 5 == 0 or epoch == args.epochs:
                clause_acc, holder_acc = evaluate(model, train_loader, device)
                msg += f"  train_clause_acc={clause_acc:.3f}  train_holder_acc={holder_acc:.3f}"

        print(msg)

    # Save final model.
    torch.save(model.state_dict(), output_dir / "final_model.pt")

    # Save metadata.
    metadata = {
        "base_model": args.base_model,
        "num_holder_classes": num_holder_classes,
        "holder_labels": holder_labels,
        "max_length": args.max_length,
        "training_examples": len(train_dataset),
        "epochs": args.epochs,
        "best_clause_acc": best_clause_acc,
    }
    with open(output_dir / "metadata.json", "w") as f:
        json.dump(metadata, f, indent=2)

    # Save holder label mapping.
    with open(output_dir / "holder_labels.json", "w") as f:
        json.dump(holder_labels, f, indent=2)

    print(f"\nSaved to {output_dir}/")
    print(f"  final_model.pt, metadata.json, holder_labels.json")
    if best_clause_acc > 0:
        print(f"  best_model.pt (clause_acc={best_clause_acc:.3f})")


if __name__ == "__main__":
    main()
