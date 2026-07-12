---
description: Fine-tune a model on RunPod GPU and export GGUF for Ollama. Covers pod setup, SSH, training, GGUF export, transfer, and cleanup.
---

# RunPod Fine-Tune & GGUF Export

## When This Applies

When you need to fine-tune a model with LoRA and produce a GGUF for local Ollama inference. This machine has no GPU — RunPod provides temporary GPU access.

## Prerequisites

- RunPod account with credit ($2-5 per training run)
- Training data as JSONL in `data/` (HuggingFace chat format: messages/role/content)
- `runpodctl` installed locally: `curl -sL https://github.com/runpod/runpodctl/releases/latest/download/runpodctl-linux-amd64 -o /tmp/runpodctl && chmod +x /tmp/runpodctl`
- Ollama running locally

## Pod Setup

### GPU Selection

| Model size | Precision | Min VRAM | Recommended GPU | Cost |
|-----------|-----------|----------|----------------|------|
| 1-4B | 4-bit (QLoRA) | 16GB | RTX 4090 (24GB) | $0.69/hr |
| 1-4B | 16-bit (clean GGUF) | 32GB | RTX 5090 (32GB) | $0.99/hr |
| 7-13B | 4-bit | 24GB | RTX 4090 (24GB) | $0.69/hr |
| 7-13B | 16-bit | 48GB+ | L40S (48GB) | $0.99/hr |

**IMPORTANT**: Use 16-bit training (`load_in_4bit=False`) if you need a working GGUF. 4-bit training produces a model that works great on GPU but the LoRA weights get lost during GGUF quantisation due to rounding errors.

### Create Pod

1. RunPod → Pods → Deploy
2. Select GPU (RTX 5090 for 4B 16-bit)
3. Template: **PyTorch** (includes CUDA, Python, Jupyter)
4. Container disk: **20 GB**
5. Volume disk: **30+ GB** (persists across stop/start, NOT across terminate)
6. Deploy On-Demand

### SSH Setup

RunPod SSH keys added in Settings only apply to NEW pods. For existing pods, inject via Jupyter terminal or this command after pod is up:

```bash
# Get PORT and IP from pod Connect tab → "SSH over exposed TCP"
ssh -p <PORT> -i ~/.ssh/id_ed25519 -o StrictHostKeyChecking=no root@<IP> \
  "echo '$(cat ~/.ssh/id_ed25519.pub)' >> /root/.ssh/authorized_keys"
```

If SSH asks for a password, the key isn't registered. Inject via Jupyter terminal instead.

## Training Procedure

### 1. Upload files

```bash
scp -P <PORT> -i ~/.ssh/id_ed25519 -o StrictHostKeyChecking=no \
  scripts/finetune_runpod_16bit.py \
  data/ml/slm_train.jsonl \
  data/ml/slm_test.jsonl \
  root@<IP>:/workspace/
```

### 2. Start training

```bash
ssh -p <PORT> -i ~/.ssh/id_ed25519 root@<IP> \
  "nohup bash -c 'pip install unsloth -q --break-system-packages && python3 /workspace/finetune_runpod_16bit.py' > /workspace/training.log 2>&1 &"
```

### 3. Monitor progress

```bash
ssh -p <PORT> -i ~/.ssh/id_ed25519 root@<IP> "tail -20 /workspace/training.log"
```

Training script does everything: install → preflight → train → save adapter → eval → merge → GGUF export.

### 4. Check results

Look for these in the log:
- `Test accuracy: X/Y = Z%` — eval result
- `Q4_K_M GGUF: /path (XXXX MB)` — GGUF ready
- `Transfer: runpodctl send <path>` — ready to download

## GGUF Export Details

The training script handles this automatically, but if it fails (disk issues), here's the manual procedure:

### Disk management

- Container disk (`/`, overlay, 20-30GB) fills up fast with model downloads and caches
- Volume disk (`/workspace`, network mount) has plenty of space but llama-quantize fails writing to it (iostream error)
- **Write GGUF to `/tmp`** (container local disk), then transfer

### Clean up if disk is full

```bash
# On pod:
pip cache purge --break-system-packages
rm -rf /root/.cache/huggingface/hub/*/blobs
rm -rf /workspace/output/merged_model  # after GGUF is exported
```

### Manual GGUF conversion (if auto-export fails)

```bash
# 1. Find convert script and quantize binary
ls ~/.unsloth/llama.cpp/unsloth_convert_hf_to_gguf.py
ls ~/.unsloth/llama.cpp/llama-quantize

# 2. Convert merged HF model to F16 GGUF (write to /tmp)
python3 ~/.unsloth/llama.cpp/unsloth_convert_hf_to_gguf.py \
  /workspace/output/merged_model \
  --outfile /tmp/model-bf16.gguf \
  --outtype f16

# 3. Verify header (must show 47 47 55 46 = "GGUF")
od -A x -t x1z -N 16 /tmp/model-bf16.gguf

# 4. Quantise to Q4_K_M
~/.unsloth/llama.cpp/llama-quantize \
  /tmp/model-bf16.gguf \
  /tmp/model-q4.gguf \
  Q4_K_M

# 5. Verify Q4 header
od -A x -t x1z -N 16 /tmp/model-q4.gguf

# 6. Delete BF16 to free space
rm /tmp/model-bf16.gguf
```

## File Transfer

### DO NOT use Jupyter browser download for files >500MB — it corrupts them (all zeros).

### Use runpodctl (peer-to-peer, reliable):

```bash
# On pod:
runpodctl send /tmp/model-q4.gguf
# Shows a code like: 1234-word-word-word-5

# On local machine:
cd /var/home/jason/fractalaw/models
/tmp/runpodctl receive 1234-word-word-word-5
```

### Use SCP for small files (<500MB):

```bash
scp -P <PORT> -i ~/.ssh/id_ed25519 root@<IP>:/workspace/output/file ./
```

### Verify downloaded GGUF:

```bash
xxd models/model-q4.gguf | head -2
# First bytes must be: 4747 5546 (GGUF magic)
# If all zeros → corrupt download, retry with runpodctl
```

## Load into Ollama

```bash
cd /var/home/jason/fractalaw/models

# Create Modelfile (no system prompt — fine-tuned model has it baked in from training)
cat > Modelfile << 'EOF'
FROM ./gemma3-position-q4.gguf
PARAMETER temperature 0.0
PARAMETER num_predict 50
EOF

ollama create gemma3-position -f Modelfile
```

## Post-Training Cleanup

1. **Copy ALL outputs from `/tmp` to `/workspace`** — GGUF files written to `/tmp` (to avoid network mount IO errors) are LOST on pod stop. Copy immediately after quantisation:
   ```bash
   cp /tmp/gemma3-*-q4.gguf /workspace/
   ```
2. **Verify all artifacts are on `/workspace`** before stopping:
   ```bash
   ls -lh /workspace/*.gguf /workspace/output/
   ```
3. **Stop the pod** immediately after downloading outputs
4. **Terminate the pod** if you don't need to re-run (volume disk costs $0.13/day idle)
5. **Back up to NAS**: `cp models/gemma3-position-q4.gguf /mnt/nas/sertantai-data/data/fractalaw-backups/YYYYMMDD/`
6. **Back up adapter**: `cp -r data/slm-adapter/ /mnt/nas/sertantai-data/data/fractalaw-backups/YYYYMMDD/slm-adapter/`

## Critical Lessons

- **16-bit training for GGUF**: 4-bit LoRA works great on GPU but LoRA weights get rounded away during GGUF quantisation. Always use `load_in_4bit=False` if the goal is a GGUF file.
- **Unsloth's GGUF export can fail silently**: The file may have valid size but zero/wrong content. Always verify the GGUF header (`47 47 55 46`) before transferring.
- **Network mount breaks llama-quantize**: Write intermediate and output GGUF files to `/tmp` or `/workspace` (container-local), NOT network-mounted paths.
- **`--break-system-packages`**: Some RunPod templates use externally-managed Python. Add this flag to pip install commands.
- **Unsloth model copies are ungated**: Use `unsloth/gemma-3-4b-it` instead of `google/gemma-3-4b-it` to avoid HuggingFace gating requirements.
- **bf16 not fp16 for Gemma**: Gemma weights are bfloat16. Set `fp16=False, bf16=True` in SFTConfig.

## Scripts

- `scripts/finetune_runpod_16bit.py` — full pipeline: install, train, eval, merge, GGUF export
- `scripts/finetune_runpod.py` — 4-bit version (faster training, but GGUF loses LoRA)
- `scripts/export_slm_training_data.py` — export gold benchmarks to JSONL training data
- `scripts/eval_slm_position.py` — evaluate Ollama model against gold benchmarks

## Cost Reference

| Task | GPU | Time | Cost |
|------|-----|------|------|
| Train 4B 16-bit (3K examples, 3 epochs) | RTX 5090 | ~90 min | ~$1.50 |
| Train 4B 4-bit (3K examples, 3 epochs) | RTX 4090 | ~90 min | ~$1.00 |
| GGUF export only (from saved adapter) | Any GPU | ~15 min | ~$0.15 |
| Idle volume storage | — | per day | $0.13 |
