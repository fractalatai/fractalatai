#!/usr/bin/env python3
"""Fine-tune gemma-3-4b-it for JSP control title generation on RunPod.

16-bit base, LoRA R=16. Generates indicative-mood control titles from
obligation text + artefact type. Trained on 1,000 Gemini-generated
legislation controls (gold standard) + 1,064 JSP controls.

IMPORTANT: Namespaced to /workspace/control-titles/ on RunPod.

Setup on RunPod (A100 40GB+):
    pip install unsloth -q
    mkdir -p /workspace/control-titles
    # Upload training data + this script
    python3 /workspace/control-titles/finetune_control_titles.py

Output:
    /workspace/control-titles/output/gemma3-control-titles-lora/
    /workspace/control-titles/output/merged_model/
    /workspace/control-titles/output/gemma3-control-titles-q4.gguf
"""

import json
import os
import subprocess
import sys
import time
import warnings
from collections import Counter

MODEL_NAME = "unsloth/gemma-3-4b-it"
TRAIN_FILE = "/workspace/control-titles/train.jsonl"
TEST_FILE = "/workspace/control-titles/val.jsonl"
OUTPUT_DIR = "/workspace/control-titles/output/gemma3-control-titles-lora"
MERGED_DIR = "/workspace/control-titles/output/merged_model"
GGUF_NAME = "gemma3-control-titles"
MAX_SEQ_LENGTH = 1024
EPOCHS = 3
BATCH_SIZE = 2
GRAD_ACCUM = 4
LEARNING_RATE = 2e-4
LORA_R = 16
LORA_ALPHA = 16


def install():
    subprocess.check_call([sys.executable, "-m", "pip", "install", "-q", "--break-system-packages", "unsloth"])


def preflight():
    print("=" * 60)
    print("PREFLIGHT — Control title generator")
    print("=" * 60)
    ok = True
    import torch
    if not torch.cuda.is_available():
        print("NO GPU"); ok = False
    else:
        print(f"GPU: {torch.cuda.get_device_name(0)}")
    for path, label in [(TRAIN_FILE, "Train"), (TEST_FILE, "Val")]:
        if os.path.exists(path):
            with open(path) as f: n = sum(1 for _ in f)
            print(f"{label}: {n} examples")
        else:
            print(f"{label}: MISSING"); ok = False
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    return ok


def train():
    from unsloth import FastModel
    from trl import SFTTrainer, SFTConfig
    from datasets import load_dataset

    model, tokenizer = FastModel.from_pretrained(MODEL_NAME, max_seq_length=MAX_SEQ_LENGTH, load_in_4bit=False)
    model = FastModel.get_peft_model(model, r=LORA_R, lora_alpha=LORA_ALPHA, lora_dropout=0,
        target_modules=["q_proj","k_proj","v_proj","o_proj","gate_proj","up_proj","down_proj"],
        use_gradient_checkpointing="unsloth")

    train_ds = load_dataset("json", data_files=TRAIN_FILE, split="train")
    val_ds = load_dataset("json", data_files=TEST_FILE, split="train")

    def apply_template(ex):
        return {"text": tokenizer.apply_chat_template(ex["messages"], tokenize=False)}
    train_ds = train_ds.map(apply_template, batched=True)
    val_ds = val_ds.map(apply_template, batched=True)

    t0 = time.time()
    SFTTrainer(model=model, tokenizer=tokenizer, train_dataset=train_ds, eval_dataset=val_ds,
        args=SFTConfig(output_dir=OUTPUT_DIR, num_train_epochs=EPOCHS, per_device_train_batch_size=BATCH_SIZE,
            gradient_accumulation_steps=GRAD_ACCUM, learning_rate=LEARNING_RATE, lr_scheduler_type="cosine",
            warmup_ratio=0.1, logging_steps=25, eval_strategy="steps", eval_steps=200,
            save_strategy="steps", save_steps=500, save_total_limit=1,
            fp16=False, bf16=True, optim="adamw_8bit", seed=42, report_to="none",
            dataset_text_field="text", max_seq_length=MAX_SEQ_LENGTH)).train()
    print(f"Training: {(time.time()-t0)/60:.1f} min")
    model.save_pretrained(OUTPUT_DIR); tokenizer.save_pretrained(OUTPUT_DIR)
    return model, tokenizer


def merge_and_export(model, tokenizer):
    os.makedirs(MERGED_DIR, exist_ok=True)
    model.save_pretrained_merged(MERGED_DIR, tokenizer, save_method="merged_16bit")
    convert = os.path.expanduser("~/.unsloth/llama.cpp/unsloth_convert_hf_to_gguf.py")
    if not os.path.exists(convert):
        convert = os.path.expanduser("~/.unsloth/llama.cpp/convert_hf_to_gguf.py")
    bf16 = f"/tmp/{GGUF_NAME}-bf16.gguf"; q4 = f"/tmp/{GGUF_NAME}-q4.gguf"
    subprocess.check_call([sys.executable, convert, MERGED_DIR, "--outfile", bf16, "--outtype", "f16"])
    subprocess.check_call([os.path.expanduser("~/.unsloth/llama.cpp/llama-quantize"), bf16, q4, "Q4_K_M"])
    os.remove(bf16)
    final = f"/workspace/control-titles/output/{GGUF_NAME}-q4.gguf"
    subprocess.check_call(["cp", q4, final])
    print(f"GGUF: {final} ({os.path.getsize(final)/1024/1024:.0f} MB)")


if __name__ == "__main__":
    install()
    if not preflight(): sys.exit(1)
    model, tokenizer = train()
    merge_and_export(model, tokenizer)
    print(f"\nDone. Load: ollama create {GGUF_NAME} -f /workspace/control-titles/Modelfile")
