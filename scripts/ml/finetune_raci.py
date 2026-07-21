#!/usr/bin/env python3
"""Fine-tune gemma-3-4b-it for JSP RACI classification on RunPod.

16-bit base, LoRA R=16. Classifies organisational role assignments
(R/A/C/I) from JSP obligation text.

IMPORTANT: This script is namespaced to /workspace/raci/ on RunPod.
Do NOT reuse paths from other fine-tuning scripts (position, fitness).

Setup on RunPod (A100 40GB+):
    pip install unsloth -q
    # Upload training data:
    mkdir -p /workspace/raci
    scp data/jsp-raci-training/train.jsonl pod:/workspace/raci/train.jsonl
    scp data/jsp-raci-training/val.jsonl pod:/workspace/raci/val.jsonl
    scp scripts/ml/finetune_raci.py pod:/workspace/raci/finetune_raci.py
    # Run:
    python3 /workspace/raci/finetune_raci.py

Output:
    /workspace/raci/output/gemma3-raci-lora/    — LoRA adapter
    /workspace/raci/output/merged_model/         — merged 16-bit model
    /tmp/gemma3-raci-q4.gguf                     — quantised GGUF for Ollama
"""

import json
import os
import subprocess
import sys
import time
import warnings
from collections import Counter

# ── Config ──────────────────────────────────────────────────────────────
# ALL paths namespaced to /workspace/raci/ — no collision with other models

MODEL_NAME = "unsloth/gemma-3-4b-it"
TRAIN_FILE = "/workspace/raci/train.jsonl"
TEST_FILE = "/workspace/raci/val.jsonl"
OUTPUT_DIR = "/workspace/raci/output/gemma3-raci-lora"
MERGED_DIR = "/workspace/raci/output/merged_model"
GGUF_NAME = "gemma3-raci"  # produces gemma3-raci-q4.gguf
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
    print("PREFLIGHT CHECKS — RACI classifier")
    print("=" * 60)
    ok = True

    import torch
    print(f"[1/5] GPU: ", end="")
    if torch.cuda.is_available():
        name = torch.cuda.get_device_name(0)
        vram = torch.cuda.get_device_properties(0).total_memory / 1024**3
        print(f"{name} ({vram:.0f} GB)", end="")
        if vram < 35:
            print(" — WARNING: 16-bit training needs ~30GB VRAM")
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

    print(f"[3/5] Val data: ", end="")
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
        assert msgs[0]["role"] == "system", "first message must be system"
        assert msgs[1]["role"] == "user", "second message must be user"
        assert msgs[2]["role"] == "assistant", "third message must be assistant"
        gold = json.loads(msgs[2]["content"])
        assert isinstance(gold, list), "assistant response must be a JSON array"
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

    print("\n=== Loading model (16-bit) ===")
    model, tokenizer = FastModel.from_pretrained(
        MODEL_NAME,
        max_seq_length=MAX_SEQ_LENGTH,
        load_in_4bit=False,
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
    val_dataset = load_dataset("json", data_files=TEST_FILE, split="train")
    print(f"Train: {len(train_dataset)}, Val: {len(val_dataset)}")

    def apply_chat_template(examples):
        texts = tokenizer.apply_chat_template(examples["messages"], tokenize=False)
        return {"text": texts}

    train_dataset = train_dataset.map(apply_chat_template, batched=True)
    val_dataset_mapped = val_dataset.map(apply_chat_template, batched=True)

    total_steps = len(train_dataset) * EPOCHS // (BATCH_SIZE * GRAD_ACCUM)
    print(f"\n=== Training ({EPOCHS} epochs, ~{total_steps} steps) ===")
    t0 = time.time()

    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=train_dataset,
        eval_dataset=val_dataset_mapped,
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
    print("\n=== Merging LoRA into base model (16-bit) ===")
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

    bf16_gguf = f"/tmp/{GGUF_NAME}-bf16.gguf"
    q4_gguf = f"/tmp/{GGUF_NAME}-q4.gguf"

    subprocess.check_call([
        sys.executable, convert_script,
        MERGED_DIR,
        "--outfile", bf16_gguf,
        "--outtype", "f16",
    ])
    size = os.path.getsize(bf16_gguf) / 1024 / 1024
    print(f"F16 GGUF: {bf16_gguf} ({size:.0f} MB)")

    print("\n=== Quantising to Q4_K_M ===")
    quantize_bin = os.path.expanduser("~/.unsloth/llama.cpp/llama-quantize")
    subprocess.check_call([quantize_bin, bf16_gguf, q4_gguf, "Q4_K_M"])
    size = os.path.getsize(q4_gguf) / 1024 / 1024
    print(f"Q4_K_M GGUF: {q4_gguf} ({size:.0f} MB)")

    os.remove(bf16_gguf)
    print(f"Deleted {bf16_gguf}")

    # Copy to workspace for persistence
    final_path = f"/workspace/raci/output/{GGUF_NAME}-q4.gguf"
    os.makedirs(os.path.dirname(final_path), exist_ok=True)
    subprocess.check_call(["cp", q4_gguf, final_path])
    print(f"Copied to {final_path}")

    return q4_gguf

# ── Step 4: Evaluate ───────────────────────────────────────────────────

def evaluate(model, tokenizer):
    import torch
    from datasets import load_dataset

    print("\n=== Evaluating on validation set ===")
    val_dataset = load_dataset("json", data_files=TEST_FILE, split="train")

    correct = 0
    total = 0
    errors = 0
    t0 = time.time()

    for i, example in enumerate(val_dataset):
        messages = example["messages"]
        eval_messages = [m for m in messages if m["role"] != "assistant"]
        gold_text = messages[-1]["content"]

        try:
            gold = json.loads(gold_text)
        except json.JSONDecodeError:
            errors += 1
            continue

        input_text = tokenizer.apply_chat_template(
            eval_messages, tokenize=False, add_generation_prompt=True
        )
        inputs = tokenizer(input_text, return_tensors="pt").to(model.device)

        with torch.no_grad(), warnings.catch_warnings():
            warnings.simplefilter("ignore")
            outputs = model.generate(
                **inputs,
                max_new_tokens=200,
                temperature=0.0,
                do_sample=False,
            )

        response = tokenizer.decode(outputs[0][inputs["input_ids"].shape[1]:], skip_special_tokens=True).strip()

        try:
            pred = json.loads(response)
        except json.JSONDecodeError:
            errors += 1
            continue

        # Compare: same set of (role, type) assignments
        gold_set = set((a["role"], a["type"]) for a in gold) if gold else set()
        pred_set = set((a.get("role", ""), a.get("type", "")) for a in pred) if pred else set()

        if gold_set == pred_set:
            correct += 1
        total += 1

        if (i + 1) % 50 == 0:
            elapsed = time.time() - t0
            print(f"  {i+1}/{len(val_dataset)} ({correct}/{total} correct, {errors} parse errors, {elapsed:.0f}s)")

    elapsed = time.time() - t0
    accuracy = correct / total if total > 0 else 0
    print(f"\nResults: {correct}/{total} exact match ({accuracy:.1%})")
    print(f"Parse errors: {errors}")
    print(f"Time: {elapsed:.0f}s ({elapsed/len(val_dataset):.1f}s/example)")

# ── Main ───────────────────────────────────────────────────────────────

if __name__ == "__main__":
    install()
    if not preflight():
        sys.exit(1)

    model, tokenizer = train()
    evaluate(model, tokenizer)
    merge_and_export(model, tokenizer)

    print("\n" + "=" * 60)
    print("DONE — RACI fine-tuning complete")
    print(f"GGUF: /workspace/raci/output/{GGUF_NAME}-q4.gguf")
    print(f"To load in Ollama:")
    print(f"  ollama create {GGUF_NAME} -f /workspace/raci/Modelfile")
    print("=" * 60)
