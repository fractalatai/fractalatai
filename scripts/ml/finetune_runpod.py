#!/usr/bin/env python3
"""Fine-tune gemma-3-4b-it on RunPod GPU with LoRA.

Self-contained script: installs deps, trains, saves, evals, exports GGUF.
Designed for RTX 4090 (24GB) or A100. ~30-45 min total.

Usage on RunPod:
    # 1. Upload this script + slm_train.jsonl + slm_test.jsonl to /workspace/
    # 2. Run:
    python3 /workspace/finetune_runpod.py

Output saved to /workspace/output/ (persists across pod restarts)
"""

import json
import os
import subprocess
import sys
import time
import warnings
from collections import Counter

# ── Config ──────────────────────────────────────────────────────────────

MODEL_NAME = "unsloth/gemma-3-4b-it"  # Unsloth's optimised version (no HF gating)
TRAIN_FILE = "/workspace/slm_train.jsonl"
TEST_FILE = "/workspace/slm_test.jsonl"
OUTPUT_DIR = "/workspace/output/gemma3-position-lora"
GGUF_DIR = "/workspace/output/gemma3-position-gguf"
MAX_SEQ_LENGTH = 1024
EPOCHS = 3
BATCH_SIZE = 4
GRAD_ACCUM = 2  # effective batch = 8
LEARNING_RATE = 2e-4
LORA_R = 16
LORA_ALPHA = 16

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

    # GPU
    import torch
    print(f"[1/5] GPU: ", end="")
    if torch.cuda.is_available():
        name = torch.cuda.get_device_name(0)
        vram = torch.cuda.get_device_properties(0).total_memory / 1024**3
        print(f"{name} ({vram:.0f} GB) — OK")
    else:
        print("NONE — FAIL (need GPU)")
        ok = False

    # Training data
    print(f"[2/5] Train data: ", end="")
    if os.path.exists(TRAIN_FILE):
        with open(TRAIN_FILE) as f:
            n = sum(1 for _ in f)
        print(f"{n} examples — OK")
    else:
        print(f"FAIL ({TRAIN_FILE} not found)")
        ok = False

    # Test data
    print(f"[3/5] Test data: ", end="")
    if os.path.exists(TEST_FILE):
        with open(TEST_FILE) as f:
            n = sum(1 for _ in f)
        print(f"{n} examples — OK")
    else:
        print(f"FAIL ({TEST_FILE} not found)")
        ok = False

    # Data format
    print(f"[4/5] Data format: ", end="")
    try:
        with open(TRAIN_FILE) as f:
            row = json.loads(f.readline())
        msgs = row["messages"]
        assert msgs[0]["role"] == "system"
        assert msgs[1]["role"] == "user"
        assert msgs[2]["role"] == "assistant"
        gold = json.loads(msgs[2]["content"])
        assert gold["position"] in ("active", "counterparty", "beneficiary", "mentioned")
        print("OK")
    except Exception as e:
        print(f"FAIL ({e})")
        ok = False

    # Output dir
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
            logging_steps=25,
            eval_strategy="steps",
            eval_steps=200,
            save_strategy="steps",
            save_steps=200,
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

    # === SAVE IMMEDIATELY ===
    print("\n=== Saving adapter ===")
    model.save_pretrained(OUTPUT_DIR)
    tokenizer.save_pretrained(OUTPUT_DIR)

    # Verify
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

    correct = 0
    total = 0
    errors = 0
    confusion = Counter()
    t0 = time.time()

    for i, example in enumerate(test_dataset):
        messages = example["messages"]
        eval_messages = [m for m in messages if m["role"] != "assistant"]
        gold = json.loads(messages[-1]["content"])["position"]

        input_text = tokenizer.apply_chat_template(
            eval_messages, tokenize=False, add_generation_prompt=True
        )
        inputs = tokenizer(input_text, return_tensors="pt").to(model.device)

        with torch.no_grad(), warnings.catch_warnings():
            warnings.simplefilter("ignore")
            outputs = model.generate(
                **inputs, max_new_tokens=50, temperature=0.0, do_sample=False
            )

        response = tokenizer.decode(
            outputs[0][inputs["input_ids"].shape[1]:], skip_special_tokens=True
        ).strip()

        try:
            if "```json" in response:
                response = response.split("```json")[1].split("```")[0].strip()
            elif "```" in response:
                response = response.split("```")[1].split("```")[0].strip()
            predicted = json.loads(response)["position"]
            if predicted in ("active", "counterparty", "beneficiary", "mentioned"):
                total += 1
                if predicted == gold:
                    correct += 1
                confusion[(predicted, gold)] += 1
            else:
                errors += 1
        except (json.JSONDecodeError, KeyError):
            errors += 1

        if (i + 1) % 50 == 0:
            acc = 100 * correct / total if total > 0 else 0
            print(f"  [{i+1}/{len(test_dataset)}] {correct}/{total} = {acc:.1f}%")

    elapsed = time.time() - t0
    acc = 100 * correct / total if total > 0 else 0

    print(f"\n{'=' * 60}")
    print(f"RESULTS: Fine-tuned gemma-3-4b-it")
    print(f"{'=' * 60}")
    print(f"Test accuracy: {correct}/{total} = {acc:.1f}%")
    print(f"Parse errors:  {errors}")
    print(f"Baseline (prompt-only gemma3:4b): 47.5%")
    print(f"Kaggle (gemma-3-1b-it): 77.4%")
    print(f"Time: {elapsed:.0f}s")

    positions = ["active", "counterparty", "beneficiary", "mentioned"]
    print(f"\nPer-position accuracy:")
    for pos in positions:
        golds = sum(confusion.get((p, pos), 0) for p in positions)
        right = confusion.get((pos, pos), 0)
        if golds > 0:
            print(f"  {pos:15s}: {right}/{golds} = {100*right/golds:.1f}%")

    print(f"\nConfusion (predicted -> gold):")
    print(f"  {'':15s} " + " ".join(f"{p:>12s}" for p in positions))
    for pred in positions:
        counts = [confusion.get((pred, gold), 0) for gold in positions]
        print(f"  {pred:15s} " + " ".join(f"{c:12d}" for c in counts))

    results_path = "/workspace/output/eval_results.json"
    with open(results_path, "w") as f:
        json.dump({
            "accuracy": acc, "correct": correct, "total": total, "errors": errors,
            "confusion": {f"{p}->{g}": c for (p, g), c in confusion.items()},
        }, f, indent=2)
    print(f"\nResults saved to {results_path}")

# ── Step 4: Export GGUF ────────────────────────────────────────────────

def export_gguf(model, tokenizer):
    import glob

    print("\n=== Exporting GGUF for Ollama ===")
    try:
        model.save_pretrained_gguf(GGUF_DIR, tokenizer, quantization_method="q4_k_m")
        gguf_files = glob.glob(f"{GGUF_DIR}/*.gguf")
        for f in gguf_files:
            size = os.path.getsize(f) / 1024 / 1024
            print(f"  GGUF: {f} ({size:.0f} MB)")
    except Exception as e:
        print(f"  GGUF export failed: {e}")
        print("  (adapter is saved — you can convert to GGUF later)")

# ── Main ───────────────────────────────────────────────────────────────

def main():
    os.makedirs("/workspace/output", exist_ok=True)

    install()

    if not preflight():
        sys.exit(1)

    print("\nStarting in 3 seconds...")
    time.sleep(3)

    model, tokenizer = train()
    evaluate(model, tokenizer)
    export_gguf(model, tokenizer)

    print("\n" + "=" * 60)
    print("ALL DONE")
    print("=" * 60)
    print("Files in /workspace/output/:")
    for root, dirs, files in os.walk("/workspace/output"):
        for f in files:
            path = os.path.join(root, f)
            size = os.path.getsize(path) / 1024 / 1024
            print(f"  {path} ({size:.1f} MB)")
    print("\nDownload /workspace/output/ before stopping the pod!")

if __name__ == "__main__":
    main()
