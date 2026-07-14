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

**Review the list before deleting.** Use DuckDB `is_amending` and `is_commencing` flags plus the title to identify obvious non-making candidates:

```bash
/usr/bin/python3 -c "
import duckdb
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
rows = conn.execute(\"\"\"
    SELECT name, title, triage_confidence,
           is_amending, is_commencing, is_making
    FROM legislation 
    WHERE triage_classification = 'not_making'
    ORDER BY is_amending DESC, is_commencing DESC, name
\"\"\").fetchall()

print('=== OBVIOUS NON-MAKING (Amendment/Commencement) ===')
obvious = []
review = []
for r in rows:
    title = (r[1] or '(no title)')
    is_amendment = r[3] or 'Amendment' in title
    is_commencement = r[4] or 'Commencement' in title
    is_revocation = 'Revocation' in title or 'Revoked' in title
    is_extension = 'Extension' in title and ('Byelaws' in title or 'Order' in title)
    if is_amendment or is_commencement or is_revocation or is_extension:
        tag = 'AMD' if is_amendment else ('COM' if is_commencement else ('REV' if is_revocation else 'EXT'))
        print(f'  [{tag}] {r[0]:35s} {title[:55]}')
        obvious.append(r[0])
    else:
        review.append(r)

print(f'\n=== NEED REVIEW ({len(review)}) ===')
for r in review:
    print(f'  {r[2]:.0%} {r[0]:35s} {(r[1] or \"(no title)\")[:55]}')

print(f'\nObvious non-making: {len(obvious)} (safe to delete)')
print(f'Need review: {len(review)} (check before deleting)')
conn.close()
"
```

**Amendment, Commencement, Revocation, and Extension laws** are always safe to delete — they modify, commence, revoke, or extend the application of other laws but contain no standalone obligations. Delete the obvious ones first, then review the remainder.

After review, delete from Postgres. **CASCADE FKs** on `legislation_text` automatically clean `provision_actors` and `fitness_mentions`:

```bash
# Export law names from DuckDB, delete from Postgres (CASCADE handles child tables)
NOT_MAKING=$(/usr/bin/python3 -c "
import duckdb
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
rows = conn.execute(\"SELECT name FROM legislation WHERE triage_classification = 'not_making'\").fetchall()
print(','.join(f\"'{r[0]}'\" for r in rows))
conn.close()
")

PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
DELETE FROM legislation_text WHERE law_name IN ($NOT_MAKING);
"
```

> **CASCADE**: `provision_actors` and `fitness_mentions` both have `ON DELETE CASCADE` foreign keys to `legislation_text.section_id`. A single `DELETE FROM legislation_text` cleans all three tables.

### Step 0a-QA: Triage vs Enrichment disagreements

After enrichment completes (Steps 1-9), check for triage false negatives — laws that triage classified as `not_making` or `uncertain` but enrichment found actual obligations. These are laws where the enrichment data is the authority, not the triage:

```bash
/usr/bin/python3 -c "
import duckdb, psycopg2

pg = psycopg2.connect('host=localhost port=5433 dbname=fractalaw user=fractalaw password=fractalaw')
cur = pg.cursor()
duck = duckdb.connect('data/fractalaw.duckdb', read_only=True)

rows = duck.execute(\"\"\"
    SELECT name, title, triage_classification, triage_confidence
    FROM legislation WHERE triage_classification IN ('not_making', 'uncertain')
    ORDER BY triage_classification, name
\"\"\").fetchall()

false_neg = []
true_neg = []
for r in rows:
    cur.execute('''
        SELECT count(DISTINCT section_id)
        FROM legislation_text
        WHERE law_name = %s AND 'Obligation' = ANY(drrp_types)
    ''', (r[0],))
    obligs = cur.fetchone()[0]
    if obligs > 0:
        false_neg.append((r[0], r[1], r[2], obligs))
    else:
        true_neg.append((r[0], r[1], r[2]))

print(f'=== FALSE NEGATIVES (triage wrong, enrichment found obligations) === ({len(false_neg)})')
for name, title, triage, obligs in false_neg:
    print(f'  [{triage:11s}] {name:35s} obligs={obligs:3d}  {(title or \"(no title)\")[:45]}')

print(f'\n=== TRUE NEGATIVES (triage correct, no obligations) === ({len(true_neg)})')
for name, title, triage in true_neg:
    print(f'  [{triage:11s}] {name:35s} {(title or \"(no title)\")[:50]}')

pg.close(); duck.close()
"
```

**False negatives**: enrichment found obligations → sertantai will set `is_making=true` from taxa publish regardless of triage signal. These laws should keep their LAT. No action needed — taxa takes priority over triage in sertantai.

**True negatives**: no obligations found by enrichment → confirm they are genuinely non-making (check for dot-filled/repealed text, amendment-only, commencement-only, forms/designations). Delete LAT for confirmed non-making laws.

> **Note**: Run this QA AFTER enrichment completes — it compares triage (regex-only, fast) against enrichment (full pipeline including SLM). The disagreements reveal where triage's confidence threshold is too aggressive for small laws with few provisions.

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

**Sequencing is critical for significance.** Three things must happen in order:

1. **Backfill first** — writes `drrp_types = {Obligation}` to `legislation_text`. The significance script queries `provision_actors.regex_drrp = 'Obligation' OR slm_drrp = 'Obligation'` joined to `legislation_text.significance_overall IS NULL`. Without backfill, new provisions don't have `drrp_types` set and may be skipped.

2. **Significance SLM** — writes 4 dimension columns (`significance_gravity`, `significance_scope_duty_bearer`, `significance_scope_protected_class`, `significance_strength`). Does NOT write `significance_overall`.

3. **Derive hierarchy** — writes `significance_hierarchy` from metadata (law type + depth). This is the 5th dimension.

4. **Backfill again** — computes `significance_overall` from all 5 dimensions. **Will set `significance_overall = NULL` if `significance_hierarchy` is missing.** This is why derive_hierarchy MUST run before the final backfill.

The full cycle is: **backfill → significance → derive_hierarchy → backfill**. If significance was run out of order, run: significance → derive_hierarchy → backfill to catch up.

### Step 8b: Derive hierarchy significance

After significance SLM completes, derive hierarchy from metadata locally. **Must run before final backfill** — backfill needs all 5 dimensions to compute `significance_overall`.

```bash
LAWS=$(cat data/sertantai/<customer>-applicable-laws.csv)
/usr/bin/python3 .claude/skills/customer-batch-parse/scripts/derive_hierarchy.py --laws "$LAWS"
```

### Step 9: Backfill (final)

Aggregate provision_actors → legislation_text AND compute `significance_overall` from all 5 dimensions.

```bash
LAWS=$(cat data/sertantai/<customer>-applicable-laws.csv)
cargo run -p fractalaw-cli -- taxa backfill --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw --laws "$LAWS"
```

**Verify significance_overall is populated** — if any Obligation provisions still have `significance_overall IS NULL`, check that `significance_hierarchy` is set. If not, re-run derive_hierarchy then backfill.

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
