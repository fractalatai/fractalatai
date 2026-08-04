#!/usr/bin/env python3
"""Fine-tune Qwen 3 8B on cultural graph extraction with LoRA.

Self-contained script: installs deps, trains, saves, exports GGUF.
Designed for RTX 4090 (24GB) or A100 on RunPod.

Usage on RunPod:
    # 1. Upload this script + slm-train.jsonl + slm-test.jsonl to /workspace/
    # 2. Run:
    python3 /workspace/finetune_runpod.py

Output saved to /workspace/output/ (persists across pod restarts).
"""

import json
import os
import subprocess
import sys
import time
from collections import Counter

# ── Config ──────────────────────────────────────────────────────────────

MODEL_NAME = "unsloth/Qwen3-8B"
TRAIN_FILE = "/workspace/positive-observations-slm-train.jsonl"
TEST_FILE = "/workspace/positive-observations-slm-test.jsonl"
OUTPUT_DIR = "/workspace/output/cultural-graph-po-lora"
GGUF_DIR = "/workspace/output/cultural-graph-po-gguf"
MAX_SEQ_LENGTH = 2048  # narratives + JSON output can be long
EPOCHS = 3
BATCH_SIZE = 2         # 8B model needs smaller batches
GRAD_ACCUM = 4         # effective batch = 8
LEARNING_RATE = 1e-4   # slightly lower for 8B
LORA_R = 16
LORA_ALPHA = 32        # alpha = 2*r for 8B

CULTURAL_TYPES = {
    "shares-information-with", "monitors", "learns-from", "cooperates-with",
    "speaks-up-to", "recognises", "adapts-to", "responds-to-failure-of",
    "normalises", "directs", "cares-for", "protects",
}

# ── Step 0: Install ────────────────────────────────────────────────────

def install():
    print("=== Installing dependencies ===")
    subprocess.check_call([
        sys.executable, "-m", "pip", "install", "-q", "unsloth"
    ])
    print("Install complete\n")

# ── Step 1: Preflight ──────────────────────────────────────────────────

def preflight():
    print("=" * 60)
    print("PREFLIGHT CHECKS")
    print("=" * 60)
    ok = True

    import torch
    print(f"[1/5] GPU: ", end="")
    if torch.cuda.is_available():
        name = torch.cuda.get_device_name(0)
        vram = torch.cuda.get_device_properties(0).total_memory / 1024**3
        print(f"{name} ({vram:.0f} GB) — OK")
    else:
        print("NONE — FAIL (need GPU)")
        ok = False

    print(f"[2/5] Train data: ", end="")
    if os.path.exists(TRAIN_FILE):
        with open(TRAIN_FILE) as f:
            n = sum(1 for _ in f)
        print(f"{n} examples — OK")
    else:
        print(f"FAIL ({TRAIN_FILE} not found)")
        ok = False

    print(f"[3/5] Test data: ", end="")
    if os.path.exists(TEST_FILE):
        with open(TEST_FILE) as f:
            n = sum(1 for _ in f)
        print(f"{n} examples — OK")
    else:
        print(f"FAIL ({TEST_FILE} not found)")
        ok = False

    print(f"[4/5] Data format: ", end="")
    try:
        with open(TRAIN_FILE) as f:
            row = json.loads(f.readline())
        msgs = row["messages"]
        assert msgs[0]["role"] == "system"
        assert msgs[1]["role"] == "user"
        assert msgs[2]["role"] == "assistant"
        gold = json.loads(msgs[2]["content"])
        assert "entities" in gold
        assert "relationships" in gold
        print("OK")
    except Exception as e:
        print(f"FAIL ({e})")
        ok = False

    print(f"[5/5] Output dir: ", end="")
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    os.makedirs(GGUF_DIR, exist_ok=True)
    print(f"{OUTPUT_DIR} — OK")

    print("=" * 60)
    if ok:
        print("ALL CHECKS PASSED")
    else:
        print("PREFLIGHT FAILED")
    print("=" * 60)
    return ok

# ── Step 2: Train ──────────────────────────────────────────────────────

def train():
    from unsloth import FastModel
    from trl import SFTTrainer, SFTConfig
    from datasets import load_dataset

    print("\n=== Loading model ===")
    model, tokenizer = FastModel.from_pretrained(
        MODEL_NAME,
        max_seq_length=MAX_SEQ_LENGTH,
        load_in_4bit=True,
    )
    print(f"Model loaded: {model.num_parameters():,} params")

    print("=== Configuring LoRA ===")
    model = FastModel.get_peft_model(
        model,
        r=LORA_R,
        lora_alpha=LORA_ALPHA,
        lora_dropout=0,
        target_modules=[
            "q_proj", "k_proj", "v_proj", "o_proj",
            "gate_proj", "up_proj", "down_proj",
        ],
        use_gradient_checkpointing="unsloth",
    )
    trainable = sum(p.numel() for p in model.parameters() if p.requires_grad)
    total_params = sum(p.numel() for p in model.parameters())
    print(f"Trainable: {trainable:,} / {total_params:,} ({100*trainable/total_params:.1f}%)")

    print("=== Loading data ===")
    train_dataset = load_dataset("json", data_files=TRAIN_FILE, split="train")
    test_dataset = load_dataset("json", data_files=TEST_FILE, split="train")
    print(f"Train: {len(train_dataset)}, Test: {len(test_dataset)}")

    def apply_chat_template(examples):
        texts = tokenizer.apply_chat_template(examples["messages"], tokenize=False)
        return {"text": texts}

    train_dataset = train_dataset.map(apply_chat_template, batched=True)
    test_dataset_mapped = test_dataset.map(apply_chat_template, batched=True)

    total_steps = len(train_dataset) * EPOCHS // (BATCH_SIZE * GRAD_ACCUM)
    print(f"\n=== Training ({EPOCHS} epochs, ~{total_steps} steps) ===")
    t0 = time.time()

    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=train_dataset,
        eval_dataset=test_dataset_mapped,
        args=SFTConfig(
            output_dir=OUTPUT_DIR,
            num_train_epochs=EPOCHS,
            per_device_train_batch_size=BATCH_SIZE,
            gradient_accumulation_steps=GRAD_ACCUM,
            learning_rate=LEARNING_RATE,
            lr_scheduler_type="cosine",
            warmup_ratio=0.1,
            logging_steps=10,
            eval_strategy="steps",
            eval_steps=50,
            save_strategy="steps",
            save_steps=50,
            save_total_limit=2,
            fp16=False,
            bf16=True,
            optim="adamw_8bit",
            seed=42,
            report_to="none",
            dataset_text_field="text",
            max_seq_length=MAX_SEQ_LENGTH,
        ),
    )
    trainer.train()
    elapsed = time.time() - t0
    print(f"\nTraining complete in {elapsed/60:.1f} minutes")

    print("\n=== Saving adapter ===")
    model.save_pretrained(OUTPUT_DIR)
    tokenizer.save_pretrained(OUTPUT_DIR)

    for name in ["adapter_model.safetensors", "adapter_model.bin", "adapter_config.json"]:
        path = os.path.join(OUTPUT_DIR, name)
        if os.path.exists(path):
            size = os.path.getsize(path) / 1024 / 1024
            print(f"  SAVED: {path} ({size:.0f} MB)")

    return model, tokenizer

# ── Step 3: Evaluate ───────────────────────────────────────────────────

def evaluate(model, tokenizer):
    import torch
    from datasets import load_dataset

    print("\n=== Evaluating on test set ===")
    test_dataset = load_dataset("json", data_files=TEST_FILE, split="train")

    total = 0
    valid_json = 0
    type_matches = 0
    type_total = 0
    per_type_pred = Counter()
    per_type_gold = Counter()

    for i, example in enumerate(test_dataset):
        msgs = example["messages"]
        gold_output = json.loads(msgs[2]["content"])
        gold_rels = [r for r in gold_output.get("relationships", [])
                     if r.get("edge_type") in CULTURAL_TYPES]
        gold_types = Counter(r["edge_type"] for r in gold_rels)

        # Build input (system + user only)
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
            print(f"  [{i+1}/{len(test_dataset)}] valid_json={valid_json}/{total}")

    print(f"\nResults:")
    print(f"  Valid JSON: {valid_json}/{total} ({100*valid_json/total:.0f}%)")
    if type_total > 0:
        print(f"  Type-level recall: {type_matches}/{type_total} ({100*type_matches/type_total:.0f}%)")
    print(f"\n  Per-type (pred / gold):")
    all_types = sorted(set(per_type_pred.keys()) | set(per_type_gold.keys()))
    for t in all_types:
        print(f"    {t:<28} pred={per_type_pred.get(t,0):>4}  gold={per_type_gold.get(t,0):>4}")

# ── Step 4: Export GGUF ───────────────────────────────────────────────

def export_gguf(model, tokenizer):
    from unsloth import FastModel

    print("\n=== Exporting GGUF ===")
    FastModel.save_pretrained_gguf(
        model, tokenizer,
        GGUF_DIR,
        quantization_method="q4_k_m",
    )

    for f in os.listdir(GGUF_DIR):
        path = os.path.join(GGUF_DIR, f)
        if os.path.isfile(path):
            size = os.path.getsize(path) / 1024 / 1024
            print(f"  {f}: {size:.0f} MB")

# ── Main ──────────────────────────────────────────────────────────────

def main():
    install()
    if not preflight():
        sys.exit(1)

    model, tokenizer = train()
    evaluate(model, tokenizer)
    export_gguf(model, tokenizer)

    print("\n" + "=" * 60)
    print("DONE")
    print(f"  Adapter: {OUTPUT_DIR}")
    print(f"  GGUF:    {GGUF_DIR}")
    print("=" * 60)


if __name__ == "__main__":
    main()
