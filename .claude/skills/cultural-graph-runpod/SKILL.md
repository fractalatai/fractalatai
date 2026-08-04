---
description: Run cultural graph extraction on RunPod GPU. Uploads cleaned JSONL, runs inference via Ollama, downloads results. Step 2 of the monthly cultural graph workflow.
---

# Cultural Graph: RunPod Inference

## When This Applies

After `/cultural-graph-ingest` has produced cleaned JSONL. This is step 2 of the monthly workflow:

1. `/cultural-graph-ingest` → cleaned JSONL
2. **RunPod inference** (this skill) → extraction results JSONL
3. `/cultural-graph-load` → DuckDB + site profiles

## Prerequisites

- Cleaned JSONL from ingest skill at `data/qq/cultural-graph/ingest/<name>-clean.jsonl`
- RunPod pod running with GPU (RTX 4090/5090)
- GGUF model on RunPod network volume: `/workspace/cultural-graph/output/qwen3-8b-cultural-graph-v2-q4.gguf`

## Pod Setup

### 1. Get SSH details from RunPod Connect tab

```
SSH → <IP>:<PORT>
```

### 2. Install Ollama and create model

```bash
ssh -o StrictHostKeyChecking=no root@<IP> -p <PORT> -i ~/.ssh/id_ed25519 '
curl -fsSL https://ollama.com/install.sh | sh
OLLAMA_HOST=0.0.0.0 OLLAMA_NUM_PARALLEL=4 nohup ollama serve &>/tmp/ollama.log &
sleep 3
sed "s|FROM .*gguf|FROM /workspace/cultural-graph/output/qwen3-8b-cultural-graph-v2-q4.gguf|" \
  /workspace/cultural-graph/Modelfile > /tmp/Modelfile
ollama create cultural-graph -f /tmp/Modelfile
ollama list
'
```

### 3. Upload cleaned JSONL and inference script

```bash
scp -P <PORT> -i ~/.ssh/id_ed25519 \
  data/qq/cultural-graph/ingest/<name>-clean.jsonl \
  scripts/cultural-graph/runpod_inference.py \
  root@<IP>:/workspace/cultural-graph/
```

## Running Inference

```bash
ssh root@<IP> -p <PORT> -i ~/.ssh/id_ed25519 \
  'cd /workspace/cultural-graph && \
   pip install requests -q --break-system-packages && \
   nohup python3 -u runpod_inference.py \
     --input /workspace/cultural-graph/<name>-clean.jsonl \
     --workers 4 \
     > /workspace/cultural-graph/inference.log 2>&1 &'
```

### Monitor progress

```bash
ssh root@<IP> -p <PORT> -i ~/.ssh/id_ed25519 \
  'tail -5 /workspace/cultural-graph/inference.log'
```

### Resume after failure

```bash
ssh root@<IP> -p <PORT> -i ~/.ssh/id_ed25519 \
  'cd /workspace/cultural-graph && \
   python3 -u runpod_inference.py \
     --input /workspace/cultural-graph/<name>-clean.jsonl \
     --workers 4 --resume'
```

## Download Results

```bash
mkdir -p data/qq/cultural-graph/results
scp -P <PORT> -i ~/.ssh/id_ed25519 \
  root@<IP>:/workspace/cultural-graph/results/<name>-clean-results.jsonl \
  data/qq/cultural-graph/results/
```

### Verify

```bash
wc -l data/qq/cultural-graph/results/<name>-clean-results.jsonl
```

Row count should match the input JSONL.

## Speed Reference

| GPU | Workers | Speed | 848 records (FY2027) |
|-----|---------|-------|---------------------|
| RTX 4090 | 4 | ~2-3 records/s | ~5 min |
| RTX 5090 | 4 | ~3-4 records/s | ~4 min |

## Post-Inference Checklist

1. **Verify row count** matches input
2. **Check error rate** — should be <5%. Grep for `"valid": false`
3. **Download results** to local machine
4. **Back up to NAS** (optional but recommended for large batches)
5. **Stop the pod** — GPU charges by the minute
6. Proceed to `/cultural-graph-load`

## File Locations

| File | Purpose |
|------|---------|
| `scripts/cultural-graph/runpod_inference.py` | Inference script (runs on pod) |
| `data/qq/cultural-graph/ingest/` | Input: cleaned JSONL from ingest |
| `data/qq/cultural-graph/results/` | Output: inference results JSONL |
| `/workspace/cultural-graph/` | RunPod network volume (persists across pod stop/start) |

## Notes

- Results persist per-record — if the pod dies, restart with `--resume`
- The model and Modelfile are on the network volume — no need to re-upload between runs
- `OLLAMA_NUM_PARALLEL` must be set BEFORE `ollama serve` starts
- Always verify results are on `/workspace` (network volume) before stopping the pod
