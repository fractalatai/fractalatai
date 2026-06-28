#!/usr/bin/env python3
"""Merge LoRA adapter into base model and export to GGUF.

Uses Unsloth's save_pretrained_merged (handles 4-bit dequant + LoRA merge),
then llama.cpp for GGUF conversion and quantisation.

Run on RunPod:
    pip install unsloth -q
    python3 /workspace/runpod_merge_and_export.py
"""
import os
import subprocess
import sys

ADAPTER_DIR = "/workspace/output/gemma3-position-lora"
MERGED_DIR = "/workspace/merged_model"
GGUF_BF16 = "/tmp/gemma3-position-bf16.gguf"
GGUF_Q4 = "/tmp/gemma3-position-q4.gguf"

print("=== Step 1: Load base model + LoRA adapter ===")
from unsloth import FastModel

model, tokenizer = FastModel.from_pretrained(
    ADAPTER_DIR,
    max_seq_length=1024,
    load_in_4bit=True,
)
print(f"Model loaded with adapter from {ADAPTER_DIR}")

print("\n=== Step 2: Save merged model (dequant 4-bit + apply LoRA) ===")
os.makedirs(MERGED_DIR, exist_ok=True)
model.save_pretrained_merged(
    MERGED_DIR,
    tokenizer,
    save_method="merged_16bit",
)
print(f"Merged 16-bit model saved to {MERGED_DIR}")

for f in os.listdir(MERGED_DIR):
    path = os.path.join(MERGED_DIR, f)
    if os.path.isfile(path):
        size = os.path.getsize(path) / 1024 / 1024
        print(f"  {f} ({size:.1f} MB)")

print("\n=== Step 3: Convert to F16 GGUF ===")
convert_script = os.path.expanduser("~/.unsloth/llama.cpp/unsloth_convert_hf_to_gguf.py")
if not os.path.exists(convert_script):
    convert_script = os.path.expanduser("~/.unsloth/llama.cpp/convert_hf_to_gguf.py")

subprocess.check_call([
    sys.executable, convert_script,
    MERGED_DIR,
    "--outfile", GGUF_BF16,
    "--outtype", "f16",
])

os.system(f"od -A x -t x1z -N 16 {GGUF_BF16}")
size = os.path.getsize(GGUF_BF16) / 1024 / 1024
print(f"F16 GGUF: {GGUF_BF16} ({size:.0f} MB)")

print("\n=== Step 4: Quantise to Q4_K_M ===")
quantize_bin = os.path.expanduser("~/.unsloth/llama.cpp/llama-quantize")
subprocess.check_call([quantize_bin, GGUF_BF16, GGUF_Q4, "Q4_K_M"])

os.system(f"od -A x -t x1z -N 16 {GGUF_Q4}")
size = os.path.getsize(GGUF_Q4) / 1024 / 1024
print(f"Q4_K_M GGUF: {GGUF_Q4} ({size:.0f} MB)")

print("\n=== Step 5: Cleanup ===")
os.remove(GGUF_BF16)
print(f"Deleted {GGUF_BF16}")

print("\n=== Done! ===")
print(f"Transfer: runpodctl send {GGUF_Q4}")
