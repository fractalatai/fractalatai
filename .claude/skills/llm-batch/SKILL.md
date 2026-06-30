---
description: Run Gemini LLM classification on pending_llm actors. Human-triggered tier for low-confidence SLM predictions.
---

# LLM Batch Classification

## When This Applies

After SLM classification and reconciliation. Actors with `extraction_method = 'pending_llm'` (SLM confidence < 0.9) need Gemini to resolve.

## Prerequisites

- `GEMINI_API_KEY` set in environment (source ~/.bashrc)
- Postgres running with provision_actors populated
- Reconciliation already run (pending_llm actors flagged)

## IMPORTANT: Exclude benchmark laws

**NEVER send benchmark laws to Gemini.** They already have gold LLM labels. The script must exclude them.

Before running, verify the pending_llm actors don't include benchmarks:

```bash
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
SELECT count(*) FROM provision_actors pa
JOIN legislation_text lt ON pa.section_id = lt.section_id
WHERE pa.extraction_method = 'pending_llm'
AND pa.llm_position IS NULL
AND lt.law_name IN (SELECT DISTINCT split_part(section_id, ':', 1) FROM gold_benchmarks);
"
```

If non-zero, the reconciliation ran on benchmarks — re-run reconcile on non-benchmark laws only.

## Usage

```bash
# Dry run — see what would be sent
source ~/.bashrc
/usr/bin/python3 scripts/gemini_llm_batch.py --dry-run

# Test with 10 actors
/usr/bin/python3 scripts/gemini_llm_batch.py --limit 10

# Full run
/usr/bin/python3 scripts/gemini_llm_batch.py
```

## What it does

1. Queries `pending_llm` actors with `llm_position IS NULL`
2. Sends each (provision text + actor label) to Gemini Flash
3. Gemini returns dual `{"drrp": "...", "position": "..."}`
4. Writes to `llm_drrp` and `llm_position` columns (does NOT overwrite SLM signal)
5. After completion, update extraction_method directly:

```sql
UPDATE provision_actors
SET extraction_method = 'llm',
    drrp = llm_drrp,
    position = llm_position,
    reconcile_confidence = 'HIGHEST'
WHERE llm_position IS NOT NULL
AND extraction_method = 'pending_llm';
```

No reconciliation step needed — LLM is the final tier, there's nothing to reconcile against.

## Cost and time

- Gemini Flash: ~$0.50 per 1,000 actors
- Rate: ~1-2 actors/s (API rate limited)
- Typical run: 1,000-2,000 actors, 10-30 min, <$1

## Post-LLM

After LLM completes:
1. Run the SQL update above to set extraction_method
2. Run `customer-stats` to verify pending_llm ≈ 0
3. Proceed to backfill + publish

## Training data feedback loop

LLM results are high-quality labels. After running:
- Export new training data including LLM-classified actors
- Retrain SLM on RunPod with expanded dataset
- Next iteration: fewer actors fall below 0.9 confidence

## Script

`scripts/gemini_llm_batch.py`
