#!/usr/bin/env python3
"""Re-export GGUF from saved LoRA adapter on RunPod.

Run on RunPod after training is done:
    python3 /workspace/runpod_export_gguf.py
"""
import glob
import os
import subprocess

print("=== Loading adapter ===")
from unsloth import FastModel

model, tokenizer = FastModel.from_pretrained(
    "/workspace/output/gemma3-position-lora",
    max_seq_length=1024,
    load_in_4bit=True,
)

print("=== Exporting GGUF (Q4_K_M) ===")
model.save_pretrained_gguf(
    "/workspace/output/gguf",
    tokenizer,
    quantization_method="q4_k_m",
)

print("\n=== Verifying ===")
for f in sorted(glob.glob("/workspace/output/gguf/*.gguf")):
    size = os.path.getsize(f) / 1024 / 1024
    print(f"  {f} ({size:.0f} MB)")
    os.system(f"xxd {f} | head -3")

print("\n=== Ready to transfer ===")
q4_files = [f for f in glob.glob("/workspace/output/gguf/*.gguf") if "Q4_K_M" in f]
if q4_files:
    print(f"Run: runpodctl send {q4_files[0]}")
