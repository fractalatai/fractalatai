---
description: Run Gemini LLM classification on pending_llm actors or low-confidence SLM predictions on high-significance provisions.
---

# LLM Batch Classification

## When This Applies

After SLM classification and reconciliation. Two modes:

1. **Default**: Actors with `extraction_method = 'pending_llm'` — reconciliation flagged these as unresolved.
2. **Significance-gated**: Low-confidence SLM actors on HIGH/MEDIUM significance provisions — targeted QA of the most important obligations.

## Prerequisites

- `GEMINI_API_KEY` set in environment (source ~/.bashrc)
- Postgres running with provision_actors populated
- Reconciliation already run (pending_llm actors flagged)
- For significance mode: significance SLM must have run first

## IMPORTANT: Exclude benchmark laws

**NEVER send benchmark laws to Gemini.** They already have gold LLM labels. The script excludes them automatically via the gold_benchmarks table.

## Usage

```bash
source ~/.bashrc

# Default mode: pending_llm actors
/usr/bin/python3 scripts/gemini_llm_batch.py --dry-run
/usr/bin/python3 scripts/gemini_llm_batch.py --limit 10
/usr/bin/python3 scripts/gemini_llm_batch.py

# Significance-gated mode: LOW-confidence actors on HIGH provisions
/usr/bin/python3 scripts/gemini_llm_batch.py --significance HIGH --max-confidence 0.9 \
  --law-file data/sertantai/qq-applicable-laws.csv --dry-run

# HIGH + MEDIUM provisions
/usr/bin/python3 scripts/gemini_llm_batch.py --significance HIGH --max-confidence 0.9 \
  --law-file data/sertantai/qq-applicable-laws.csv
/usr/bin/python3 scripts/gemini_llm_batch.py --significance MEDIUM --max-confidence 0.9 \
  --law-file data/sertantai/qq-applicable-laws.csv
```

## What it does

1. Queries actors (pending_llm or significance-filtered) with `llm_position IS NULL`
2. Sends each (provision text + actor label) to Gemini 2.5 Flash (thinking disabled)
3. Gemini returns `{"drrp": "...", "position": "..."}`
4. Writes to `llm_drrp` and `llm_position` columns (does NOT overwrite SLM signal)
5. For pending_llm actors, update extraction_method after completion:

```sql
UPDATE provision_actors
SET extraction_method = 'llm',
    drrp = llm_drrp,
    position = llm_position,
    reconcile_confidence = 'HIGHEST'
WHERE llm_position IS NOT NULL
AND extraction_method = 'pending_llm';
```

## Typical workloads

| Mode | Actors | Cost | Time |
|------|--------|------|------|
| pending_llm (full corpus) | 300-2,000 | <$1 | 5-30 min |
| HIGH significance, <0.9 conf | ~30 | <$0.01 | 1 min |
| MEDIUM significance, <0.9 conf | ~70 | <$0.05 | 2 min |

## Results pattern

LOW-confidence SLM actors on definition/interpretation provisions (reg.2, s.27, etc.) are almost always `mentioned/none` — the LLM confirms these actors are referenced but not governed. The SLM confidence threshold is a genuine uncertainty signal.

## Script

`scripts/gemini_llm_batch.py`
