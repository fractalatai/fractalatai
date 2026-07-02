#!/usr/bin/env python3
"""Fine-tune gemma-3-4b-it in 16-bit on RunPod A100, then export clean GGUF.

16-bit base means LoRA merges cleanly into full-precision weights,
producing a GGUF that preserves the fine-tuning.

Requires A100 (40GB+) — 4B model in 16-bit + LoRA + optimizer needs ~30GB.

Usage on RunPod:
    pip install unsloth -q
    python3 /workspace/finetune_runpod_16bit.py

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

MODEL_NAME = "unsloth/gemma-3-4b-it"
TRAIN_FILE = "/workspace/slm_train.jsonl"
TEST_FILE = "/workspace/slm_test.jsonl"
OUTPUT_DIR = "/workspace/output/gemma3-position-lora"
MERGED_DIR = "/workspace/output/merged_model"
MAX_SEQ_LENGTH = 1024
EPOCHS = 3
BATCH_SIZE = 2
GRAD_ACCUM = 4  # effective batch = 8
LEARNING_RATE = 2e-4
LORA_R = 16
LORA_ALPHA = 16

# ── Step 0: Install ────────────────────────────────────────────────────

def install():
    print("=== Installing dependencies ===")
    subprocess.check_call([
        sys.executable, "-m", "pip", "install", "-q", "--break-system-packages", "unsloth"
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
        print(f"{name} ({vram:.0f} GB)", end="")
        if vram < 35:
            print(" — WARNING: 16-bit training needs ~30GB VRAM, may OOM")
        else:
            print(" — OK")
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
        assert gold["position"] in ("active", "counterparty", "beneficiary", "mentioned")
        print("OK")
    except Exception as e:
        print(f"FAIL ({e})")
        ok = False

    print(f"[5/5] Output dir: ", end="")
    os.makedirs(OUTPUT_DIR, exist_ok=True)
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

    print("\n=== Loading model (16-bit, NO 4-bit quantisation) ===")
    model, tokenizer = FastModel.from_pretrained(
        MODEL_NAME,
        max_seq_length=MAX_SEQ_LENGTH,
        load_in_4bit=False,
    )
    print(f"Model loaded: {model.num_parameters():,} params (16-bit)")

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
            save_steps=500,
            save_total_limit=1,
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

    # === SAVE ADAPTER ===
    print("\n=== Saving LoRA adapter ===")
    model.save_pretrained(OUTPUT_DIR)
    tokenizer.save_pretrained(OUTPUT_DIR)
    for name in ["adapter_model.safetensors", "adapter_config.json"]:
        path = os.path.join(OUTPUT_DIR, name)
        if os.path.exists(path):
            size = os.path.getsize(path) / 1024 / 1024
            print(f"  SAVED: {path} ({size:.0f} MB)")

    return model, tokenizer

# ── Step 3: Merge and export GGUF ──────────────────────────────────────

def merge_and_export(model, tokenizer):
    print("\n=== Merging LoRA into base model (16-bit, clean merge) ===")
    os.makedirs(MERGED_DIR, exist_ok=True)
    model.save_pretrained_merged(
        MERGED_DIR,
        tokenizer,
        save_method="merged_16bit",
    )
    print(f"Merged model saved to {MERGED_DIR}")

    print("\n=== Converting to F16 GGUF ===")
    convert_script = os.path.expanduser("~/.unsloth/llama.cpp/unsloth_convert_hf_to_gguf.py")
    if not os.path.exists(convert_script):
        convert_script = os.path.expanduser("~/.unsloth/llama.cpp/convert_hf_to_gguf.py")

    bf16_gguf = "/tmp/gemma3-position-bf16.gguf"
    q4_gguf = "/tmp/gemma3-position-q4.gguf"

    subprocess.check_call([
        sys.executable, convert_script,
        MERGED_DIR,
        "--outfile", bf16_gguf,
        "--outtype", "f16",
    ])
    os.system(f"od -A x -t x1z -N 16 {bf16_gguf}")
    size = os.path.getsize(bf16_gguf) / 1024 / 1024
    print(f"F16 GGUF: {bf16_gguf} ({size:.0f} MB)")

    print("\n=== Quantising to Q4_K_M ===")
    quantize_bin = os.path.expanduser("~/.unsloth/llama.cpp/llama-quantize")
    subprocess.check_call([quantize_bin, bf16_gguf, q4_gguf, "Q4_K_M"])
    os.system(f"od -A x -t x1z -N 16 {q4_gguf}")
    size = os.path.getsize(q4_gguf) / 1024 / 1024
    print(f"Q4_K_M GGUF: {q4_gguf} ({size:.0f} MB)")

    os.remove(bf16_gguf)
    print(f"Deleted {bf16_gguf}")

    return q4_gguf

# ── Step 4: Evaluate ───────────────────────────────────────────────────

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

    drrp_confusion = Counter()
    drrp_correct = 0
    drrp_total = 0

    for i, example in enumerate(test_dataset):
        messages = example["messages"]
        eval_messages = [m for m in messages if m["role"] != "assistant"]
        gold_resp = json.loads(messages[-1]["content"])
        gold_pos = gold_resp["position"] if "position" in gold_resp else gold_resp.get("position", "")
        gold_drrp = gold_resp.get("drrp", None)

        input_text = tokenizer.apply_chat_template(
            eval_messages, tokenize=False, add_generation_prompt=True
        )
        inputs = tokenizer(input_text, return_tensors="pt").to(model.device)

        with torch.no_grad(), warnings.catch_warnings():
            warnings.simplefilter("ignore")
            outputs = model.generate(
                **inputs, max_new_tokens=80, temperature=0.0, do_sample=False
            )

        response = tokenizer.decode(
            outputs[0][inputs["input_ids"].shape[1]:], skip_special_tokens=True
        ).strip()

        try:
            if "```json" in response:
                response = response.split("```json")[1].split("```")[0].strip()
            elif "```" in response:
                response = response.split("```")[1].split("```")[0].strip()
            parsed = json.loads(response)
            predicted_pos = parsed.get("position", "")
            predicted_drrp = parsed.get("drrp", None)

            if predicted_pos in ("active", "counterparty", "beneficiary", "mentioned"):
                total += 1
                if predicted_pos == gold_pos:
                    correct += 1
                confusion[(predicted_pos, gold_pos)] += 1
            else:
                errors += 1

            if predicted_drrp and gold_drrp:
                drrp_total += 1
                if predicted_drrp == gold_drrp:
                    drrp_correct += 1
                drrp_confusion[(predicted_drrp, gold_drrp)] += 1
        except (json.JSONDecodeError, KeyError):
            errors += 1

        if (i + 1) % 50 == 0:
            acc = 100 * correct / total if total > 0 else 0
            print(f"  [{i+1}/{len(test_dataset)}] {correct}/{total} = {acc:.1f}%")

    elapsed = time.time() - t0
    acc = 100 * correct / total if total > 0 else 0

    print(f"\n{'=' * 60}")
    print(f"RESULTS: Fine-tuned gemma-3-4b-it (16-bit, dual DRRP+position)")
    print(f"{'=' * 60}")
    print(f"Position accuracy: {correct}/{total} = {acc:.1f}%")
    if drrp_total > 0:
        drrp_acc = 100 * drrp_correct / drrp_total
        print(f"DRRP accuracy:     {drrp_correct}/{drrp_total} = {drrp_acc:.1f}%")
    print(f"Parse errors:      {errors}")
    print(f"Time: {elapsed:.0f}s")

    positions = ["active", "counterparty", "beneficiary", "mentioned"]
    print(f"\nPer-position accuracy:")
    for pos in positions:
        golds = sum(confusion.get((p, pos), 0) for p in positions)
        right = confusion.get((pos, pos), 0)
        if golds > 0:
            print(f"  {pos:15s}: {right}/{golds} = {100*right/golds:.1f}%")

    print(f"\nPosition confusion (predicted -> gold):")
    print(f"  {'':15s} " + " ".join(f"{p:>12s}" for p in positions))
    for pred in positions:
        counts = [confusion.get((pred, gold), 0) for gold in positions]
        print(f"  {pred:15s} " + " ".join(f"{c:12d}" for c in counts))

    if drrp_total > 0:
        drrp_types = ["Obligation", "Liberty", "none"]
        print(f"\nPer-DRRP accuracy:")
        for d in drrp_types:
            golds = sum(drrp_confusion.get((p, d), 0) for p in drrp_types)
            right = drrp_confusion.get((d, d), 0)
            if golds > 0:
                print(f"  {d:15s}: {right}/{golds} = {100*right/golds:.1f}%")

        print(f"\nDRRP confusion (predicted -> gold):")
        print(f"  {'':15s} " + " ".join(f"{d:>12s}" for d in drrp_types))
        for pred in drrp_types:
            counts = [drrp_confusion.get((pred, gold), 0) for gold in drrp_types]
            print(f"  {pred:15s} " + " ".join(f"{c:12d}" for c in counts))

    results_path = "/workspace/output/eval_results.json"
    with open(results_path, "w") as f:
        json.dump({
            "accuracy": acc, "correct": correct, "total": total, "errors": errors,
            "confusion": {f"{p}->{g}": c for (p, g), c in confusion.items()},
        }, f, indent=2)
    print(f"\nResults saved to {results_path}")

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
    q4_gguf = merge_and_export(model, tokenizer)

    print("\n" + "=" * 60)
    print("ALL DONE")
    print("=" * 60)
    print(f"\nTransfer: runpodctl send {q4_gguf}")
    print("On local: /tmp/runpodctl receive <CODE>")
    print("\nThen stop the pod!")

if __name__ == "__main__":
    main()
