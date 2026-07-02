#!/usr/bin/env python3
"""Fine-tune gemma-3-1b-it locally on CPU for Hohfeldian position classification.

Designed to be watertight:
- All preflight checks run FIRST (disk, RAM, data, model download, tokenizer test)
- Training only starts after everything is verified
- Adapter saved immediately after training
- Eval and GGUF export happen after save — failures there don't lose the model

Usage:
    python3 scripts/finetune_local.py
    python3 scripts/finetune_local.py --preflight-only   # just check everything
    python3 scripts/finetune_local.py --eval-only         # skip training, eval saved adapter
"""

import argparse
import json
import os
import shutil
import sys
import time
import warnings
from collections import Counter

# ── Config ──────────────────────────────────────────────────────────────

MODEL_NAME = "google/gemma-3-1b-it"
TRAIN_FILE = "data/slm_train.jsonl"
TEST_FILE = "data/slm_test.jsonl"
OUTPUT_DIR = "data/slm-adapter"
GGUF_DIR = "data/slm-gguf"
MAX_SEQ_LENGTH = 768  # shorter = less RAM, faster training
EPOCHS = 3
BATCH_SIZE = 1
GRAD_ACCUM = 8  # effective batch = 8
LEARNING_RATE = 2e-4
LORA_R = 16
LORA_ALPHA = 16

# ── Preflight checks ───────────────────────────────────────────────────

def preflight():
    """Run all checks before committing to training. Fail fast."""
    print("=" * 60)
    print("PREFLIGHT CHECKS")
    print("=" * 60)
    ok = True

    # 1. Disk space
    stat = shutil.disk_usage("/var/home")
    free_gb = stat.free / (1024 ** 3)
    print(f"[1/8] Disk space: {free_gb:.1f} GB free", end="")
    if free_gb < 10:
        print(" — FAIL (need 10 GB)")
        ok = False
    else:
        print(" — OK")

    # 2. RAM
    import psutil
    ram = psutil.virtual_memory()
    avail_gb = ram.available / (1024 ** 3)
    print(f"[2/8] Available RAM: {avail_gb:.1f} GB", end="")
    if avail_gb < 8:
        print(f" — WARNING (only {avail_gb:.1f} GB, may be tight)")
    else:
        print(" — OK")

    # 3. Training data exists
    print(f"[3/8] Training data: {TRAIN_FILE}", end="")
    if not os.path.exists(TRAIN_FILE):
        print(" — FAIL (file not found)")
        ok = False
    else:
        with open(TRAIN_FILE) as f:
            n_train = sum(1 for _ in f)
        print(f" — OK ({n_train} examples)")

    # 4. Test data exists
    print(f"[4/8] Test data: {TEST_FILE}", end="")
    if not os.path.exists(TEST_FILE):
        print(" — FAIL (file not found)")
        ok = False
    else:
        with open(TEST_FILE) as f:
            n_test = sum(1 for _ in f)
        print(f" — OK ({n_test} examples)")

    # 5. Training data format
    print(f"[5/8] Data format check", end="")
    try:
        with open(TRAIN_FILE) as f:
            row = json.loads(f.readline())
        msgs = row["messages"]
        assert len(msgs) == 3, f"expected 3 messages, got {len(msgs)}"
        assert msgs[0]["role"] == "system"
        assert msgs[1]["role"] == "user"
        assert msgs[2]["role"] == "assistant"
        gold = json.loads(msgs[2]["content"])
        assert "position" in gold
        assert gold["position"] in ("active", "counterparty", "beneficiary", "mentioned")
        print(" — OK (system/user/assistant, valid position)")
    except Exception as e:
        print(f" — FAIL ({e})")
        ok = False

    # 6. PyTorch
    print(f"[6/8] PyTorch", end="")
    try:
        import torch
        print(f" — OK ({torch.__version__}, device=cpu)")
    except ImportError:
        print(" — FAIL (not installed)")
        ok = False

    # 7. Required packages
    print(f"[7/8] Required packages", end="")
    missing = []
    for pkg in ["transformers", "peft", "trl", "accelerate", "datasets", "sentencepiece"]:
        try:
            __import__(pkg)
        except ImportError:
            missing.append(pkg)
    if missing:
        print(f" — FAIL (missing: {', '.join(missing)})")
        ok = False
    else:
        print(" — OK")

    # 8. Model download + tokenizer test
    print(f"[8/8] Model download + tokenizer test", end="")
    sys.stdout.flush()
    try:
        from transformers import AutoTokenizer
        tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
        test_msgs = [
            {"role": "user", "content": "test"},
            {"role": "assistant", "content": "ok"},
        ]
        out = tokenizer.apply_chat_template(test_msgs, tokenize=False)
        assert len(out) > 0
        print(f" — OK (tokenizer works, model cached)")
    except Exception as e:
        print(f" — FAIL ({e})")
        ok = False

    print("=" * 60)
    if ok:
        print("ALL CHECKS PASSED — ready to train")
    else:
        print("PREFLIGHT FAILED — fix issues above before training")
    print("=" * 60)
    return ok


# ── Training ────────────────────────────────────────────────────────────

def train():
    """Load model, configure LoRA, train, save adapter."""
    import torch
    from datasets import load_dataset
    from transformers import AutoModelForCausalLM, AutoTokenizer
    from peft import LoraConfig, get_peft_model, TaskType
    from trl import SFTTrainer, SFTConfig

    print("\n=== Loading tokenizer ===")
    tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    print("=== Loading model (this may take a few minutes on CPU) ===")
    t0 = time.time()
    model = AutoModelForCausalLM.from_pretrained(
        MODEL_NAME,
        torch_dtype=torch.float32,
        device_map="cpu",
    )
    print(f"Model loaded in {time.time() - t0:.0f}s — {model.num_parameters():,} params")

    print("=== Configuring LoRA ===")
    lora_config = LoraConfig(
        r=LORA_R,
        lora_alpha=LORA_ALPHA,
        lora_dropout=0.0,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj",
                         "gate_proj", "up_proj", "down_proj"],
        task_type=TaskType.CAUSAL_LM,
    )
    model = get_peft_model(model, lora_config)
    model.print_trainable_parameters()

    print("=== Loading data ===")
    train_dataset = load_dataset("json", data_files=TRAIN_FILE, split="train")
    test_dataset = load_dataset("json", data_files=TEST_FILE, split="train")
    print(f"Train: {len(train_dataset)}, Test: {len(test_dataset)}")

    def apply_chat_template(examples):
        texts = tokenizer.apply_chat_template(examples["messages"], tokenize=False)
        return {"text": texts}

    train_dataset = train_dataset.map(apply_chat_template, batched=True)
    test_dataset_mapped = test_dataset.map(apply_chat_template, batched=True)

    # Verify one example tokenizes within max_seq_length
    sample = tokenizer(train_dataset[0]["text"], return_tensors="pt")
    seq_len = sample["input_ids"].shape[1]
    print(f"Sample sequence length: {seq_len} tokens (max: {MAX_SEQ_LENGTH})")
    if seq_len > MAX_SEQ_LENGTH:
        print(f"WARNING: some examples will be truncated")

    print(f"\n=== Training ({EPOCHS} epochs, ~6-8 hours on CPU) ===")
    print(f"Effective batch size: {BATCH_SIZE * GRAD_ACCUM}")
    print(f"Total steps: ~{len(train_dataset) * EPOCHS // (BATCH_SIZE * GRAD_ACCUM)}")
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
            logging_steps=50,
            eval_strategy="steps",
            eval_steps=500,
            save_strategy="steps",
            save_steps=500,
            save_total_limit=2,
            fp16=False,  # CPU doesn't support fp16
            bf16=False,
            optim="adamw_torch",
            seed=42,
            report_to="none",
            dataset_text_field="text",
            max_seq_length=MAX_SEQ_LENGTH,
            dataloader_num_workers=2,
        ),
    )
    trainer.train()
    elapsed = time.time() - t0
    print(f"\nTraining complete in {elapsed/3600:.1f} hours")

    # === SAVE IMMEDIATELY ===
    print("\n=== Saving adapter (this is the critical step) ===")
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    model.save_pretrained(OUTPUT_DIR)
    tokenizer.save_pretrained(OUTPUT_DIR)

    # Verify save
    adapter_path = os.path.join(OUTPUT_DIR, "adapter_model.safetensors")
    if os.path.exists(adapter_path):
        size_mb = os.path.getsize(adapter_path) / 1024 / 1024
        print(f"SAVED: {adapter_path} ({size_mb:.0f} MB)")
    else:
        # Try .bin format
        adapter_path = os.path.join(OUTPUT_DIR, "adapter_model.bin")
        if os.path.exists(adapter_path):
            size_mb = os.path.getsize(adapter_path) / 1024 / 1024
            print(f"SAVED: {adapter_path} ({size_mb:.0f} MB)")
        else:
            print("ERROR: adapter not saved! Check output directory:")
            for f in os.listdir(OUTPUT_DIR):
                print(f"  {f}")

    return model, tokenizer


# ── Evaluation ──────────────────────────────────────────────────────────

def evaluate(model=None, tokenizer=None):
    """Evaluate the fine-tuned model on the test set."""
    import torch
    from datasets import load_dataset

    if model is None or tokenizer is None:
        print("\n=== Loading saved adapter for evaluation ===")
        from transformers import AutoModelForCausalLM, AutoTokenizer
        from peft import PeftModel

        tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
        if tokenizer.pad_token is None:
            tokenizer.pad_token = tokenizer.eos_token

        base_model = AutoModelForCausalLM.from_pretrained(
            MODEL_NAME, torch_dtype=torch.float32, device_map="cpu"
        )
        model = PeftModel.from_pretrained(base_model, OUTPUT_DIR)
        print("Adapter loaded")

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
            elapsed = time.time() - t0
            rate = (i + 1) / elapsed
            eta = (len(test_dataset) - i - 1) / rate if rate > 0 else 0
            print(f"  [{i+1}/{len(test_dataset)}] {correct}/{total} = {acc:.1f}% "
                  f"({rate:.1f}/s, ETA {eta:.0f}s)")

    elapsed = time.time() - t0
    acc = 100 * correct / total if total > 0 else 0

    print(f"\n{'=' * 60}")
    print(f"RESULTS: Fine-tuned gemma-3-1b-it")
    print(f"{'=' * 60}")
    print(f"Test accuracy: {correct}/{total} = {acc:.1f}%")
    print(f"Parse errors:  {errors}")
    print(f"Baseline (prompt-only gemma3:4b): 47.5%")
    print(f"Kaggle result (same model): 77.4%")
    print(f"Time: {elapsed:.0f}s ({total/elapsed:.2f} actors/s)")

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

    # Save results
    results_path = "data/slm_finetune_eval.json"
    with open(results_path, "w") as f:
        json.dump({
            "accuracy": acc,
            "correct": correct,
            "total": total,
            "errors": errors,
            "confusion": {f"{p}->{g}": c for (p, g), c in confusion.items()},
        }, f, indent=2)
    print(f"\nResults saved to {results_path}")


# ── Main ────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Fine-tune gemma-3-1b-it locally")
    parser.add_argument("--preflight-only", action="store_true",
                        help="Run preflight checks only, don't train")
    parser.add_argument("--eval-only", action="store_true",
                        help="Skip training, evaluate saved adapter")
    args = parser.parse_args()

    os.chdir("/var/home/jason/fractalaw")

    if args.eval_only:
        if not os.path.exists(os.path.join(OUTPUT_DIR, "adapter_config.json")):
            print(f"ERROR: no adapter found at {OUTPUT_DIR}")
            sys.exit(1)
        evaluate()
        return

    if not preflight():
        sys.exit(1)

    if args.preflight_only:
        return

    print("\nStarting in 5 seconds... (Ctrl+C to abort)")
    time.sleep(5)

    model, tokenizer = train()
    evaluate(model, tokenizer)


if __name__ == "__main__":
    main()
