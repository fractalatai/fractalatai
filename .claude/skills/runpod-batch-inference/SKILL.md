---
description: Run batch SLM inference on RunPod GPU. Covers pod setup, SSH tunnel, Ollama, script execution, and verification. Used by DRRP position, significance, and fitness extraction batches.
---

# RunPod Batch Inference

## When This Applies

When running batch SLM inference via Ollama on a RunPod GPU. This machine has no GPU — RunPod provides temporary GPU access for batch classification/extraction tasks.

Current batch scripts:
- `scripts/ml/runpod_slm_batch.py` — DRRP position classification
- `scripts/ml/runpod_significance_batch.py` — obligation significance rating
- `scripts/ml/runpod_fitness_batch.py` — fitness entity extraction

For fine-tuning (training LoRA adapters, GGUF export), see `/runpod-finetune` skill instead.

## Pod Setup

### 1. Create pod

RunPod → Pods → Deploy:
- GPU: RTX 4090 (24GB, $0.69/hr) or RTX 5090 (32GB, $0.99/hr)
- Template: **PyTorch** (includes CUDA, Python)
- Container disk: **20 GB**
- Volume disk: **30+ GB** (persists across stop/start)

### 2. Install Ollama + deps

```bash
# Get PORT and IP from pod Connect tab → "SSH over exposed TCP"
ssh -o StrictHostKeyChecking=no -p <PORT> -i ~/.ssh/id_ed25519 root@<IP> '
curl -fsSL https://ollama.com/install.sh | sh
pip install --break-system-packages psycopg2-binary requests
'
```

### 3. Start Ollama with parallel workers

```bash
ssh -p <PORT> -i ~/.ssh/id_ed25519 root@<IP> '
OLLAMA_NUM_PARALLEL=4 nohup ollama serve &>/tmp/ollama.log &
sleep 3
ollama pull gemma3:4b
'
```

**`OLLAMA_NUM_PARALLEL` must be set BEFORE `ollama serve` starts.** Setting it after has no effect. Must kill and restart if changing.

### 4. Upload batch script

```bash
scp -o StrictHostKeyChecking=no -P <PORT> -i ~/.ssh/id_ed25519 \
  scripts/ml/runpod_fitness_batch.py \
  root@<IP>:/workspace/
```

### 5. Open reverse SSH tunnel (in a separate terminal)

```bash
ssh -p <PORT> -i ~/.ssh/id_ed25519 -R 5433:localhost:5433 root@<IP> -N
```

This forwards the pod's localhost:5433 to your local Postgres. Keep this terminal open for the duration of the batch.

## Running a Batch

### CRITICAL: Verify writes before full batch

**NEVER run the full batch without first verifying that results land in the database.** The `--test` flag is a dry run that skips DB writes — it does NOT test the write path.

```bash
# Step 1: Run a small batch WITH writes (not --test)
ssh -p <PORT> -i ~/.ssh/id_ed25519 root@<IP> \
  'python3 -u /workspace/<script>.py --limit 10 --workers 1'

# Step 2: Verify data landed in Postgres (from LOCAL machine)
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
SELECT extraction_method, count(*)
FROM <target_table>
WHERE extraction_method = 'slm'
GROUP BY 1;
"

# Step 3: Only if Step 2 shows data → run full batch
ssh -p <PORT> -i ~/.ssh/id_ed25519 root@<IP> \
  'nohup python3 -u /workspace/<script>.py --workers 4 > /workspace/batch.log 2>&1 &'
```

If you skip Step 2 and the write path is broken, you burn GPU time for nothing.

### Monitor progress

```bash
ssh -p <PORT> -i ~/.ssh/id_ed25519 root@<IP> 'tail -3 /workspace/batch.log'
```

### Verify completion

```bash
# Check final stats
ssh -p <PORT> -i ~/.ssh/id_ed25519 root@<IP> 'tail -10 /workspace/batch.log'

# Verify in DB
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
SELECT extraction_method, count(*) FROM <target_table> GROUP BY 1;
"
```

## Known Issues

### Python output buffering with nohup

`nohup python3 script.py > log 2>&1` buffers stdout — the log file appears empty or frozen. Use `python3 -u` (unbuffered) to see progress in real-time:

```bash
nohup python3 -u /workspace/script.py --workers 4 > /workspace/batch.log 2>&1 &
```

### torch import hangs on some pods

The `import torch` GPU check can hang indefinitely on certain RunPod templates. Batch scripts should use `nvidia-smi` subprocess instead:

```python
# BAD — can hang
import torch
torch.cuda.is_available()

# GOOD — always returns
import subprocess
result = subprocess.run(["nvidia-smi", "--query-gpu=name", "--format=csv,noheader"],
                       capture_output=True, text=True, timeout=5)
```

### logprobs deadlocks under concurrent workers

`logprobs: True` in the Ollama request body causes deadlock with multiple workers. Single-worker works fine. **Always disable logprobs for multi-worker batches.** The significance script was fixed for this 2026-07-09.

### SSH tunnel drops

Long-running batches can outlive the SSH tunnel. If the tunnel drops:
1. The batch script will start getting DB connection errors
2. Re-establish the tunnel in another terminal
3. The script may recover automatically (depends on psycopg2 reconnection logic) or need restarting
4. Scripts are idempotent — they skip provisions that already have results

### OLLAMA_NUM_PARALLEL must be set before serve

Setting `OLLAMA_NUM_PARALLEL=4` AFTER `ollama serve` is running has no effect. Must kill and restart:

```bash
pkill -f "ollama serve"; sleep 2
OLLAMA_NUM_PARALLEL=4 nohup ollama serve &>/tmp/ollama.log &
```

## Speed Reference

| Script | Model | GPU | Workers | Speed | Batch size example |
|--------|-------|-----|---------|-------|--------------------|
| Position SLM | gemma3-position (Q4) | RTX 4090 | 8 | ~10/s | 10K actors → 17 min |
| Significance | gemma3-significance (Q4) | RTX 5090 | 4 | ~6/s | 40K provisions → 110 min |
| Fitness | gemma3:4b (base) | RTX 5090 | 4 | ~5.5/s | 6.7K provisions → 20 min |

## Post-Batch

1. **Verify results in DB** before stopping the pod
2. **Stop the pod** immediately — GPU charges by the minute
3. **Don't terminate** if the network volume has models/scripts you'll reuse
4. Idle volume storage costs $0.13/day

## Scripts

| Script | Table | Method | Purpose |
|--------|-------|--------|---------|
| `runpod_slm_batch.py` | `provision_actors` | `slm` | DRRP position classification |
| `runpod_significance_batch.py` | `legislation_text` | significance columns | Obligation significance rating |
| `runpod_fitness_batch.py` | `fitness_mentions` | `slm` | Fitness entity extraction |
