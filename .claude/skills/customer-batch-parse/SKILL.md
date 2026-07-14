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

### Step 0: Identify the batch from DuckDB queue

Laws triaged as `making` or `uncertain` by sync watch have `enrichment_pending = true` in DuckDB. Use this to identify the batch rather than manually querying Postgres for gaps:

```bash
/usr/bin/python3 -c "
import duckdb
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
pending = conn.execute(\"SELECT count(*) FROM legislation WHERE enrichment_pending = true\").fetchone()[0]
not_making = conn.execute(\"SELECT count(*) FROM legislation WHERE triage_classification = 'not_making'\").fetchone()[0]
print(f'Queued for enrichment: {pending}')
print(f'Triaged not_making: {not_making}')
conn.close()
"
```

### Step 0a: Remove LAT for not_making laws

Laws triaged as `not_making` should have their LAT provisions removed from Postgres to prevent wasted enrichment. This was previously automatic but is now a manual authorised step.

**Review the list before deleting** — confirm these are genuinely not-making (amending-only, commencement-only, etc.):

```bash
# List not_making laws
/usr/bin/python3 -c "
import duckdb
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
rows = conn.execute(\"\"\"
    SELECT name, title, triage_confidence 
    FROM legislation 
    WHERE triage_classification = 'not_making'
    ORDER BY triage_confidence
\"\"\").fetchall()
for r in rows:
    print(f'  {r[2]:.0%} {r[0]:30s} {(r[1] or \"(no title)\")[:50]}')
print(f'\\nTotal: {len(rows)}')
conn.close()
"
```

After review, delete their provisions from Postgres:

```bash
# DELETE LAT for not_making laws — DESTRUCTIVE, review list first
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
DELETE FROM legislation_text
WHERE law_name IN (
    SELECT name FROM duckdb_scan('data/fractalaw.duckdb', 'legislation')
    WHERE triage_classification = 'not_making'
);
"
```

> **Note**: This requires the DuckDB foreign data wrapper or a two-step approach — export law names from DuckDB then pass to Postgres. If duckdb_scan isn't available, use:

```bash
# Two-step: export names then delete
NOT_MAKING=$(/usr/bin/python3 -c "
import duckdb
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
rows = conn.execute(\"SELECT name FROM legislation WHERE triage_classification = 'not_making'\").fetchall()
print(','.join(f\"'{r[0]}'\" for r in rows))
conn.close()
")

PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
DELETE FROM provision_actors WHERE section_id IN (SELECT section_id FROM legislation_text WHERE law_name IN ($NOT_MAKING));
DELETE FROM fitness_mentions WHERE section_id IN (SELECT section_id FROM legislation_text WHERE law_name IN ($NOT_MAKING));
DELETE FROM legislation_text WHERE law_name IN ($NOT_MAKING);
"
```

### Step 0b: Baseline stats

```bash
/usr/bin/python3 scripts/maintenance/corpus_stats.py --law-file data/sertantai/<customer>-applicable-laws.csv
```

Check Tier 0 PASS and note the starting state.

### Step 0c: Parse (regex DRRP extraction)

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

**See `/runpod-batch-inference` skill for pod setup, SSH tunnel, Ollama configuration, and known issues.**

#### Run position SLM

```bash
# Upload script + verify writes with --limit 10 first (see runpod-batch-inference skill)
python3 -u /workspace/runpod_slm_batch.py --workers 8
```

~10 actors/s on RTX 4090. 10K actors ≈ 17 min.

#### Run significance SLM

```bash
# Load significance model:
ollama create gemma3-significance -f /workspace/Modelfile.significance

# Verify writes with --limit 10 first
python3 -u /workspace/runpod_significance_batch.py --workers 4
```

~6 provisions/s on RTX 5090.

#### Run fitness SLM

Fitness extraction is an independent pipeline. See `/fitness-pipeline` skill for full details including pod setup, model loading, and verification.

```bash
# On RunPod — load fitness model and run batch
ollama create gemma3-fitness -f /workspace/Modelfile.fitness
python3 -u /workspace/scripts/runpod_fitness_batch.py --workers 4
```

~6 provisions/s on RTX 5090. Runs on all provisions (not gated by DRRP type). After batch completes, compile expression trees locally:

```bash
cargo run -p fractalaw-cli -- fitness compile --laws "$(cat data/sertantai/<customer>-applicable-laws.csv)"
```

### Step 8: Re-reconcile

With SLM tier populated.

```bash
LAWS=$(cat data/sertantai/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa reconcile --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

**Sequencing matters**: Significance must run AFTER backfill, not before. Backfill writes `drrp_types = {Obligation}` to `legislation_text` — the significance script queries provisions with this flag. If significance runs before backfill, new provisions won't have `drrp_types` set yet and will be skipped. Run significance → backfill → significance again if needed, or simply run significance after the final backfill.

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

### Step 12: Clear enrichment queue

After publish is confirmed, clear the `enrichment_pending` flag for the processed laws:

```bash
/usr/bin/python3 -c "
import duckdb
conn = duckdb.connect('data/fractalaw.duckdb')
updated = conn.execute(\"\"\"
    UPDATE legislation SET enrichment_pending = false
    WHERE enrichment_pending = true
    AND name IN (SELECT unnest(string_to_array('$(cat data/sertantai/<customer>-applicable-laws.csv)', ',')))
\"\"\").fetchone()
remaining = conn.execute('SELECT count(*) FROM legislation WHERE enrichment_pending = true').fetchone()[0]
print(f'Cleared enrichment_pending. Remaining in queue: {remaining}')
conn.close()
"
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
