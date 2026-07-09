---
description: Run the full compliance enrichment pipeline for a customer's legal register. Step-by-step batch parse from dep features through to publish.
---

# Customer Batch Parse

## When This Applies

When onboarding a customer's legal register for compliance enrichment. The customer's applicable laws are provided as a CSV file (comma-separated law names, one line, no header).

## Prerequisites

- Customer law file exists (e.g. `data/sertantai/qq-applicable-laws.csv`)
- LAT provisions pulled into Postgres (via sync-watch or `pull-lat`)
- Postgres running (`systemctl --user start fractalaw-pg.service`)
- SLM available (RunPod for batch, or local Ollama with `gemma3-position`)
- Sertantai running for publish (if publishing)

## Pipeline Steps

Run in order. Each step depends on the previous. Use `customer-stats` skill between steps to verify coverage.

### Step 0: Baseline stats

```bash
/usr/bin/python3 scripts/maintenance/corpus_stats.py --law-file data/sertantai/<customer>-applicable-laws.csv
```

Check Tier 0 PASS and note the starting state.

### Step 0b: Parse (regex DRRP extraction)

Run on laws that have LAT in Postgres but haven't been parsed. Sets scope (out/structural/substantive), extracts actors, writes `provision_actors` rows. **Must run before embed** — embed requires `scope = 'substantive'`.

```bash
# Get laws needing parse (have provisions but no extraction_method)
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -t -A -c "
SELECT string_agg(DISTINCT law_name, ',')
FROM legislation_text
WHERE extraction_method IS NULL AND scope IS NULL
AND law_name IN (SELECT unnest(string_to_array('$(cat data/sertantai/<customer>-applicable-laws.csv)', ',')));
"

# Run
cargo run -p fractalaw-cli -- taxa parse --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws <output>
```

### Step 1: Dep features (spaCy batch)

Compute dependency parsing features for all actors. Required by the v3 position classifier.

```bash
# Get laws needing dep features
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -t -A -c "
SELECT string_agg(DISTINCT lt.law_name, ',')
FROM provision_actors pa
JOIN legislation_text lt ON pa.section_id = lt.section_id
WHERE pa.dep_is_subject IS NULL AND pa.regex_position IS NOT NULL
AND lt.law_name IN (SELECT unnest(string_to_array('$(cat data/sertantai/<customer>-applicable-laws.csv)', ',')));
"

# Run (if any laws returned)
/usr/bin/python3 scripts/ml/compute_dep_features.py --laws <output>
```

~450 provisions/s. Typically 5-10 min for 250 laws.

### Step 2: Embed

Compute 384-dim MiniLM embeddings for substantive provisions missing them.

```bash
# Get laws needing embeddings
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -t -A -c "
SELECT string_agg(DISTINCT lt.law_name, ',')
FROM legislation_text lt
WHERE lt.embedding IS NULL AND lt.scope = 'substantive'
AND lt.law_name IN (SELECT unnest(string_to_array('$(cat data/sertantai/<customer>-applicable-laws.csv)', ',')));
"

# Run (if any laws returned)
cargo run -p fractalaw-cli -- taxa embed --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws <output>
```

~20 min for 100 laws on CPU.

### Step 3: Classify

Position classifier on all actors with embeddings + dep features. SLM outperforms the classifier on accuracy (79.7% vs 59.9%), but classifier must run first — it provides a second signal for reconciliation, which then flags only unresolved actors as `pending_slm` for the SLM. Without classifier, all actors go to SLM unnecessarily.

```bash
# Get laws needing classification
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -t -A -c "
SELECT string_agg(DISTINCT lt.law_name, ',')
FROM provision_actors pa
JOIN legislation_text lt ON pa.section_id = lt.section_id
WHERE pa.cls_position IS NULL AND lt.embedding IS NOT NULL AND pa.dep_is_subject IS NOT NULL
AND lt.scope = 'substantive'
AND lt.law_name IN (SELECT unnest(string_to_array('$(cat data/sertantai/<customer>-applicable-laws.csv)', ',')));
"

# Run
cargo run -p fractalaw-cli -- taxa classify --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws <output>
```

~30 min for 250 laws.

### Step 4: Infer

Correlative actor inference (Employee active → Employer counterparty).

```bash
LAWS=$(cat data/sertantai/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa infer --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

Instant — rule-based, no ML.

### Step 5: Reconcile

4-tier reconciliation. Flags `pending_slm` for SLM and `pending_llm` for human-triggered LLM.

```bash
LAWS=$(cat data/sertantai/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa reconcile --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

Instant.

### Step 6: Check stats before SLM

```bash
/usr/bin/python3 scripts/maintenance/corpus_stats.py --law-file data/sertantai/<customer>-applicable-laws.csv
```

Note the `pending_slm` count — this is the SLM workload. At ~0.3 actors/s, estimate time.

### Step 7: SLM (RunPod)

Classify pending_slm actors via RunPod GPU. Both position and significance run on the same pod.

#### RunPod setup

1. Spin up a pod (RTX 4090 or 5090) with network volume attached
2. SSH in and install deps:
   ```bash
   apt-get update -qq && apt-get install -y -qq zstd postgresql-client > /dev/null
   curl -fsSL https://ollama.com/install.sh | sh
   pip install psycopg2-binary requests
   ```
3. Start Ollama with parallel slots enabled, then load the position model:
   ```bash
   OLLAMA_NUM_PARALLEL=8 ollama serve &
   ollama create gemma3-position -f /workspace/Modelfile
   ```
   **Without `OLLAMA_NUM_PARALLEL`**, Ollama defaults to 1 concurrent request. Multiple workers will hang waiting for the single slot.
4. Open reverse SSH tunnel from LOCAL machine (keeps running in foreground):
   ```bash
   ssh -T -R 5433:localhost:5433 root@<IP> -p <PORT> -i ~/.ssh/id_ed25519 -N
   ```
5. Models and batch scripts are on the network volume at `/workspace/`:
   - `gemma3-position-q4.gguf` + `Modelfile` — DRRP + position
   - `gemma3-significance-q4.gguf` + `Modelfile.significance` — obligation significance
   - `runpod_slm_batch.py` — position batch script
   - `runpod_significance_batch.py` — significance batch script

#### Run position SLM

```bash
# On pod (via SSH):
python3 /workspace/runpod_slm_batch.py --dry-run      # check pending count
python3 /workspace/runpod_slm_batch.py --workers 8     # full run
```

~10 actors/s on RTX 4090. 10K actors ≈ 17 min.

#### Run significance SLM

```bash
# On pod — load significance model first:
ollama create gemma3-significance -f /workspace/Modelfile.significance

python3 /workspace/runpod_significance_batch.py --dry-run
python3 /workspace/runpod_significance_batch.py --workers 8
```

~10 provisions/s. The query must filter `significance_overall IS NULL` — without this, incremental runs reload the entire corpus (43K+) instead of just pending provisions. The script on the network volume was fixed 2026-07-09 to include this filter. If the dry-run count matches the full Obligation count rather than the pending count, the filter is missing.

### Step 8: Re-reconcile

With SLM tier populated.

```bash
LAWS=$(cat data/sertantai/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa reconcile --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

### Step 8b: Derive hierarchy significance

After significance SLM completes, derive hierarchy from metadata locally:

```bash
/usr/bin/python3 .claude/skills/customer-batch-parse/scripts/derive_hierarchy.py --law-file data/sertantai/<customer>-applicable-laws.csv
```

### Step 9: Backfill

Aggregate provision_actors → legislation_text for sertantai publish.

```bash
LAWS=$(cat data/sertantai/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa backfill --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

### Step 10: Final stats

```bash
/usr/bin/python3 scripts/maintenance/corpus_stats.py --law-file data/sertantai/<customer>-applicable-laws.csv
```

All QA checks should PASS. `pending_slm` should be zero (or near-zero from parse errors). `pending_llm` is expected — human-triggered.

### Step 11: Publish

```bash
LAWS=$(cat data/sertantai/<customer>-applicable-laws.csv)

# Enrichment (law-level LRT)
cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/localhost:7447 --laws "$LAWS"

# Provisions (per-provision LAT taxa)
cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/localhost:7447 --laws "$LAWS" --provisions --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw
```

## Targeting gaps only

For re-runs, don't re-process the whole corpus. Query for laws with gaps at each step (shown in the SQL above) and pass only those to `--laws`. This avoids re-processing tens of thousands of already-classified actors.

## Time estimates

| Step | Typical time (250 laws) |
|------|------------------------|
| Parse | 2-5 min |
| Dep features | 5-10 min |
| Embed | 20 min (CPU) |
| Classify | 30 min |
| Infer | 1 min |
| Reconcile | 30s |
| SLM position (RunPod) | 15-20 min (10K actors) |
| SLM significance (RunPod) | 60-70 min (40K provisions) |
| Re-reconcile | 30s |
| Backfill | 1 min |
| Publish | 2 min |

## Notes

- **Benchmark laws** (is_benchmark = true in DuckDB) should not be re-processed. Exclude from batch operations.
- **SLM is the bottleneck** — run overnight. Everything else completes in under an hour.
- **Run customer-stats between steps** to catch issues early rather than after hours of processing.
- **NAS quick backup** before and after: `/nas-backup` skill, quick mode.
