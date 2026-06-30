---
description: Run the full compliance enrichment pipeline for a customer's legal register. Step-by-step batch parse from dep features through to publish.
---

# Customer Batch Parse

## When This Applies

When onboarding a customer's legal register for compliance enrichment — the full 4-tier cascade. The customer's applicable laws are provided as a CSV file (comma-separated law names, one line, no header).

## Prerequisites

- Customer law file exists (e.g. `data/qq-applicable-laws.csv`)
- Discovery enrichment already done (`taxa enrich` — regex parse + DuckDB)
- Postgres running (`systemctl --user start fractalaw-pg.service`)
- Ollama running with `gemma3-position` model loaded
- Sertantai running for publish (if publishing)

## Pipeline Steps

Run in order. Each step depends on the previous. Use `customer-stats` skill between steps to verify coverage.

### Step 0: Baseline stats

```bash
/usr/bin/python3 scripts/corpus_stats.py --law-file data/<customer>-applicable-laws.csv
```

Check Tier 0 PASS and note the starting state.

### Step 1: Dep features (spaCy batch)

Compute dependency parsing features for all actors. Required by the v3 position classifier.

```bash
# Get laws needing dep features
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -t -A -c "
SELECT string_agg(DISTINCT lt.law_name, ',')
FROM provision_actors pa
JOIN legislation_text lt ON pa.section_id = lt.section_id
WHERE pa.dep_is_subject IS NULL AND pa.regex_position IS NOT NULL
AND lt.law_name IN (SELECT unnest(string_to_array('$(cat data/<customer>-applicable-laws.csv)', ',')));
"

# Run (if any laws returned)
/usr/bin/python3 scripts/compute_dep_features.py --laws <output>
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
AND lt.law_name IN (SELECT unnest(string_to_array('$(cat data/<customer>-applicable-laws.csv)', ',')));
"

# Run (if any laws returned)
cargo run -p fractalaw-cli -- taxa embed --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws <output>
```

~20 min for 100 laws on CPU.

### Step 3: Classify

Position classifier on all actors with embeddings + dep features.

```bash
# Get laws needing classification
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -t -A -c "
SELECT string_agg(DISTINCT lt.law_name, ',')
FROM provision_actors pa
JOIN legislation_text lt ON pa.section_id = lt.section_id
WHERE pa.cls_position IS NULL AND lt.embedding IS NOT NULL AND pa.dep_is_subject IS NOT NULL
AND lt.scope = 'substantive'
AND lt.law_name IN (SELECT unnest(string_to_array('$(cat data/<customer>-applicable-laws.csv)', ',')));
"

# Run
cargo run -p fractalaw-cli -- taxa classify --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws <output>
```

~30 min for 250 laws.

### Step 4: Infer

Correlative actor inference (Employee active → Employer counterparty).

```bash
LAWS=$(cat data/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa infer --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

Instant — rule-based, no ML.

### Step 5: Reconcile

4-tier reconciliation. Flags `pending_slm` for SLM and `pending_llm` for human-triggered LLM.

```bash
LAWS=$(cat data/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa reconcile --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

Instant.

### Step 6: Check stats before SLM

```bash
/usr/bin/python3 scripts/corpus_stats.py --law-file data/<customer>-applicable-laws.csv
```

Note the `pending_slm` count — this is the SLM workload. At ~0.3 actors/s, estimate time.

### Step 7: SLM

Classify pending_slm actors via local Ollama gemma3-position. **This is the bottleneck.**

```bash
LAWS=$(cat data/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa slm --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

~0.3 actors/s. 25K actors ≈ 24 hours. Run overnight.

### Step 8: Re-reconcile

With SLM tier populated. Per-class gating: active/counterparty accepted, beneficiary/mentioned → pending_llm.

```bash
LAWS=$(cat data/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa reconcile --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

### Step 9: Backfill

Aggregate provision_actors → legislation_text for sertantai publish.

```bash
LAWS=$(cat data/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa backfill --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

### Step 10: Final stats

```bash
/usr/bin/python3 scripts/corpus_stats.py --law-file data/<customer>-applicable-laws.csv
```

All QA checks should PASS. `pending_slm` should be zero (or near-zero from parse errors). `pending_llm` is expected — human-triggered.

### Step 11: Publish

```bash
LAWS=$(cat data/<customer>-applicable-laws.csv)

# Enrichment (law-level LRT)
cargo run -p fractalaw-cli -- sync publish --tenant dev --connect tcp/localhost:7447 --laws "$LAWS"

# Provisions (per-provision LAT taxa)
cargo run -p fractalaw-cli -- sync publish --tenant dev --connect tcp/localhost:7447 --laws "$LAWS" --provisions --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw
```

## Targeting gaps only

For re-runs, don't re-process the whole corpus. Query for laws with gaps at each step (shown in the SQL above) and pass only those to `--laws`. This avoids re-processing tens of thousands of already-classified actors.

## Time estimates

| Step | Typical time (250 laws) |
|------|------------------------|
| Dep features | 5-10 min |
| Embed | 20 min |
| Classify | 30 min |
| Infer | 1 min |
| Reconcile | 30s |
| SLM | 10-24 hours (bottleneck) |
| Re-reconcile | 30s |
| Backfill | 1 min |
| Publish | 2 min |

## Notes

- **Benchmark laws** (is_benchmark = true in DuckDB) should not be re-processed. Exclude from batch operations.
- **SLM is the bottleneck** — run overnight. Everything else completes in under an hour.
- **Run customer-stats between steps** to catch issues early rather than after hours of processing.
- **NAS quick backup** before and after: `/nas-backup` skill, quick mode.
