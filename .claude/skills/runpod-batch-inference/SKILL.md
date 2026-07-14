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
```

### Post-SLM QA

**Use these exact queries** — not ad-hoc queries on `slm_position IS NULL` which conflates "reconcile decided SLM isn't needed" with "SLM hasn't run yet".

#### Position SLM

```bash
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
-- The ONLY correct test: pending_slm count should be zero
SELECT count(*) AS pending_slm
FROM provision_actors WHERE extraction_method = 'pending_slm';

-- Context: breakdown by extraction_method (informational, not a pass/fail)
SELECT extraction_method, count(*) 
FROM provision_actors 
WHERE extraction_method IS NOT NULL
GROUP BY 1 ORDER BY 2 DESC;
"
```

**PASS**: `pending_slm = 0`
**FAIL**: `pending_slm > 0` — SLM batch didn't process all queued actors

> **WARNING**: `slm_position IS NULL` is NOT a valid test. Actors with `extraction_method = 'regex'` and `reconcile_confidence = 'MEDIUM'` intentionally have no `slm_position` — reconcile accepted the regex result without SLM. This is correct behaviour, not a gap.

#### Significance SLM

```bash
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
-- Obligation provisions that should have significance but don't
SELECT count(*) AS missing_significance
FROM legislation_text 
WHERE 'Obligation' = ANY(drrp_types)
AND significance_overall IS NULL
AND scope = 'substantive';
"
```

**PASS**: `missing_significance = 0` (or near-zero — some edge cases are expected)

#### Fitness SLM

```bash
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
-- Provisions with regex fitness mentions but no SLM extraction
SELECT count(*) AS missing_slm_fitness
FROM fitness_mentions 
WHERE slm_entities IS NULL
AND regex_entities IS NOT NULL;
"
```

**PASS**: `missing_slm_fitness = 0`

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

### System-level ollama serve blocks custom startup

RunPod PyTorch templates may auto-start `ollama serve` at boot (PID in the low hundreds). Your `nohup ollama serve` silently fails because the port is already bound. The system serve runs with default `OLLAMA_NUM_PARALLEL=1`, causing ~4x slower throughput.

**Always check and kill before starting:**

```bash
# Check for existing ollama
ps aux | grep "ollama serve" | grep -v grep

# Kill system serve AND its llama-server child
pkill -9 -f "llama-server"  # kill child FIRST
sleep 1
pkill -9 -f "ollama serve"  # then parent
sleep 2

# Verify clean
ps aux | grep -E "ollama|llama" | grep -v grep || echo "Clean"

# Start with correct parallel count
OLLAMA_NUM_PARALLEL=8 nohup ollama serve &>/tmp/ollama.log &
```

### Verify -np matches workers BEFORE starting batch

After starting ollama, warm up a model then check the llama-server args:

```bash
ollama run gemma3-position "test" 2>/dev/null | head -1
ps aux | grep llama-server | grep -v grep | grep -o "\-np [0-9]*"
```

If `-np` doesn't match your `--workers` count, the batch will be bottlenecked. Kill and restart ollama.

### Logs may appear empty despite progress

The batch script log file may show only the preflight checks and nothing else, even with `-u` (unbuffered). **Check the database directly** to confirm progress:

```bash
# Position: count remaining pending
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -t -A -c \
  "SELECT count(*) FROM provision_actors WHERE slm_position IS NULL AND regex_position IS NOT NULL"

# Significance: count remaining unrated
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -t -A -c \
  "SELECT count(*) FROM legislation_text WHERE significance_overall IS NULL AND 'Obligation' = ANY(drrp_types)"
```

### Scripts and models persist on /workspace

All artefacts (scripts, Modelfiles, GGUF models) are on the RunPod network volume at `/workspace/scripts/`, `/workspace/models/`. They survive pod stop/start. **No upload needed** for repeat runs — just create the ollama models from the existing Modelfiles:

```bash
ollama create gemma3-position -f /workspace/models/drrp/Modelfile
ollama create gemma3-significance -f /workspace/models/significance/Modelfile
ollama create gemma3-fitness -f /workspace/models/fitness/Modelfile
```

## Speed Reference

| Script | Model | GPU | Workers | Speed | Batch size example |
|--------|-------|-----|---------|-------|--------------------|
| Position SLM | gemma3-position (Q4) | RTX 4090 | 8 | ~10/s | 10K actors → 17 min |
| Significance | gemma3-significance (Q4) | RTX 5090 | 4 | ~6/s | 40K provisions → 110 min |
| Fitness | gemma3:4b (base) | RTX 5090 | 4 | ~5.5/s | 6.7K provisions → 20 min |

## Post-Batch

1. **Verify results in DB** before stopping the pod
2. **Copy ALL outputs to `/workspace`** before stopping — `/tmp` and container-local storage are LOST on stop. This includes GGUF files, logs, eval results. If it's not on `/workspace`, it doesn't survive.
3. **Stop the pod** immediately — GPU charges by the minute
4. **Don't terminate** if the network volume has models/scripts you'll reuse
5. Idle volume storage costs $0.13/day

### CRITICAL: Each extraction tier gets its own column — NEVER overwrite another tier's data

Every extraction method writes to its OWN column. No method may clear, update, or overwrite another tier's column. The columns are:

- `regex_entities` — dictionary extraction only
- `slm_entities` — base SLM (prompted, no fine-tune)
- `ft_entities` — fine-tuned SLM
- `llm_entities` — LLM (Gemini etc.)

`--force` on ANY command must ONLY clear its own tier's column. NEVER `DELETE FROM` the whole table or clear all columns. If you need to re-run regex extraction, clear `regex_entities` only. If you need to re-run SLM, clear `slm_entities` only.

**The whole point of per-tier columns is that data from different tiers can be compared.** Destroying one tier's data to re-run another defeats the purpose of the architecture.

```python
# BAD — destroys all tiers
DELETE FROM fitness_mentions
DELETE FROM fitness_mentions WHERE extraction_method = 'regex'  # deletes ROWS that have slm data too

# GOOD — clears only this tier's columns
UPDATE fitness_mentions SET regex_entities = NULL, regex_scope_dimensions = NULL
UPDATE fitness_mentions SET ft_entities = NULL WHERE ...
```

### CRITICAL: Save-as-you-go, never batch writes at the end

Batch scripts MUST write each result to the database immediately after extraction — not collect results in memory and write at the end. If the process dies, the tunnel drops, or the pod is stopped, all in-memory results are lost.

```python
# BAD — loses everything if process dies
results = []
for row in rows:
    results.append(extract(row))
write_results(conn, results)  # dies here = all work lost

# GOOD — each result persisted immediately
write_conn = psycopg2.connect(PG_DSN)
write_conn.autocommit = True
write_cur = write_conn.cursor()
for row in rows:
    result = extract(row)
    write_one(write_cur, result)  # committed immediately
```

Use `autocommit = True` on the write connection so each UPDATE is committed independently. Use a separate connection from the read connection. Use a `threading.Lock` for multi-worker writes.

### CRITICAL: `/tmp` vs `/workspace`

- **`/workspace`** = network volume. Survives stop/start. Persists until pod is terminated.
- **`/tmp`**, **`/root`**, **`/`** = container-local. LOST on stop. Gone forever.

**NEVER leave artifacts on `/tmp`**. If a script writes to `/tmp` (e.g. GGUF quantisation to avoid network mount IO errors), copy to `/workspace` immediately after:

```bash
cp /tmp/gemma3-fitness-q4.gguf /workspace/
```

Before reporting a pod is ready to stop, verify ALL outputs are on `/workspace`:

```bash
ls -lh /workspace/*.gguf /workspace/output/
```

## Scripts

| Script | Table | Method | Purpose |
|--------|-------|--------|---------|
| `runpod_slm_batch.py` | `provision_actors` | `slm` | DRRP position classification |
| `runpod_significance_batch.py` | `legislation_text` | significance columns | Obligation significance rating |
| `runpod_fitness_batch.py` | `fitness_mentions` | `slm` | Fitness entity extraction |
