#!/usr/bin/env python3
"""Fine-tune gemma-3-4b-it for fitness entity extraction, then export GGUF.

16-bit base means LoRA merges cleanly into full-precision weights.
Requires RTX 5090 (32GB) or A100 (40GB).

Usage on RunPod:
    pip install unsloth -q --break-system-packages
    python3 /workspace/finetune_fitness_16bit.py
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
TRAIN_FILE = "/workspace/fitness_train.jsonl"
TEST_FILE = "/workspace/fitness_test.jsonl"
OUTPUT_DIR = "/workspace/output/gemma3-fitness-lora"
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
    print("PREFLIGHT CHECKS — Fitness Entity Extraction Fine-Tune")
    print("=" * 60)
    ok = True

    import torch
    print(f"[1/5] GPU: ", end="")
    if torch.cuda.is_available():
        name = torch.cuda.get_device_name(0)
        vram = torch.cuda.get_device_properties(0).total_memory / 1024**3
        print(f"{name} ({vram:.0f} GB)", end="")
        if vram < 30:
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
        assert isinstance(gold, list), "Expected JSON array"
        assert len(gold) > 0, "Expected non-empty array"
        assert "name" in gold[0] and "scope" in gold[0], "Expected {name, scope}"
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

    print("\n=== Saving LoRA adapter ===")
    model.save_pretrained(OUTPUT_DIR)
    tokenizer.save_pretrained(OUTPUT_DIR)
    for name in ["adapter_model.safetensors", "adapter_config.json"]:
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
    entity_matches = 0
    entity_total_gold = 0
    entity_total_pred = 0
    scope_correct = 0
    scope_total = 0
    parse_errors = 0
    empty_correct = 0  # correctly returned [] when gold is []
    t0 = time.time()

    for i, example in enumerate(test_dataset):
        messages = example["messages"]
        eval_messages = [m for m in messages if m["role"] != "assistant"]
        gold = json.loads(messages[-1]["content"])
        gold_names = {e["name"].lower() for e in gold}

        input_text = tokenizer.apply_chat_template(
            eval_messages, tokenize=False, add_generation_prompt=True
        )
        inputs = tokenizer(input_text, return_tensors="pt").to(model.device)

        with torch.no_grad(), warnings.catch_warnings():
            warnings.simplefilter("ignore")
            outputs = model.generate(
                **inputs, max_new_tokens=256, temperature=0.0, do_sample=False
            )

        response = tokenizer.decode(
            outputs[0][inputs["input_ids"].shape[1]:], skip_special_tokens=True
        ).strip()

        try:
            if "```json" in response:
                response = response.split("```json")[1].split("```")[0].strip()
            elif "```" in response:
                response = response.split("```")[1].split("```")[0].strip()

            start = response.find("[")
            end = response.rfind("]")
            if start >= 0 and end > start:
                response = response[start:end + 1]

            parsed = json.loads(response)
            if not isinstance(parsed, list):
                parse_errors += 1
                continue

            pred_names = set()
            for item in parsed:
                if isinstance(item, dict) and "name" in item:
                    pred_names.add(item["name"].lower())
                    # Check scope accuracy
                    if item["name"].lower() in gold_names:
                        scope_total += 1
                        gold_scope = next((e["scope"] for e in gold if e["name"].lower() == item["name"].lower()), None)
                        if gold_scope and item.get("scope") == gold_scope:
                            scope_correct += 1

            # Entity-level precision and recall
            matches = gold_names & pred_names
            entity_matches += len(matches)
            entity_total_gold += len(gold_names)
            entity_total_pred += len(pred_names)
            total += 1

        except (json.JSONDecodeError, KeyError):
            parse_errors += 1

        if (i + 1) % 100 == 0:
            p = entity_matches / entity_total_pred if entity_total_pred > 0 else 0
            r = entity_matches / entity_total_gold if entity_total_gold > 0 else 0
            print(f"  [{i+1}/{len(test_dataset)}] P={p:.1%} R={r:.1%}")

    elapsed = time.time() - t0
    precision = entity_matches / entity_total_pred if entity_total_pred > 0 else 0
    recall = entity_matches / entity_total_gold if entity_total_gold > 0 else 0
    f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0
    scope_acc = scope_correct / scope_total if scope_total > 0 else 0

    print(f"\n{'=' * 60}")
    print(f"RESULTS: Fine-tuned gemma-3-4b-it (fitness entity extraction)")
    print(f"{'=' * 60}")
    print(f"Entity precision: {precision:.1%} ({entity_matches}/{entity_total_pred})")
    print(f"Entity recall:    {recall:.1%} ({entity_matches}/{entity_total_gold})")
    print(f"Entity F1:        {f1:.1%}")
    print(f"Scope accuracy:   {scope_acc:.1%} ({scope_correct}/{scope_total})")
    print(f"Parse errors:     {parse_errors}")
    print(f"Time: {elapsed:.0f}s")

    results_path = "/workspace/output/eval_results.json"
    with open(results_path, "w") as f:
        json.dump({
            "precision": precision, "recall": recall, "f1": f1,
            "scope_accuracy": scope_acc,
            "entity_matches": entity_matches,
            "entity_total_gold": entity_total_gold,
            "entity_total_pred": entity_total_pred,
            "parse_errors": parse_errors,
        }, f, indent=2)
    print(f"\nResults saved to {results_path}")

# ── Step 4: Merge and export GGUF ──────────────────────────────────────

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

    bf16_gguf = "/tmp/gemma3-fitness-bf16.gguf"
    q4_gguf = "/tmp/gemma3-fitness-q4.gguf"

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
