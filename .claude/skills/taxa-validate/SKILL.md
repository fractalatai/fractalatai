---
description: LLM validation of DRRP classifications using Gemini — dry-run, execute, and review results
---

# Skill: Taxa Validate

## When This Applies

When the user wants to validate DRRP classifications using Gemini LLM review. This is the final automated step in the pipeline after parse → embed → classify.

## Prerequisites

- `GEMINI_API_KEY` must be set (lives in `~/.bashrc`, may need `source ~/.bashrc` in the session)
- Laws must have been through parse + embed + classify first
- Provision store must be accessible (LanceDB or `--pg` for Postgres)

## Usage

### 1. Dry-run: preview scope and token cost

```bash
fractalaw taxa validate --laws UK_uksi_2015_627,UK_uksi_2015_810 --dry-run
```

With Postgres:
```bash
fractalaw taxa validate --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw \
  --laws UK_uksi_2015_627,UK_uksi_2015_810 --dry-run
```

Dry-run output shows per-law:
- Total provisions and how many are **targets** (pending_llm, orphans, low-confidence)
- How many **sections** will be sent (provisions grouped by parent section for context)
- Approximate token count per section

Laws with **0 targets** are skipped — no API calls needed.

### 2. Execute: audit-only (no writes)

```bash
fractalaw taxa validate --pg postgres://... --laws UK_uksi_2015_627
```

Calls Gemini for each section batch. Writes audit log JSON to `data/audit/<law_name>.json` but does NOT write corrections back to the store.

### 3. Execute with apply: write corrections back

```bash
fractalaw taxa validate --pg postgres://... --laws UK_uksi_2015_627 --apply
```

Same as above, but corrections are written back to the provision store with `extraction_method = "agentic"`.

## How It Works

The validate command uses **targeted section-based** validation (schema_version 2):

1. **Identify targets**: provisions with `pending_llm`, orphans (DRRP but no actors), or low confidence (<0.3)
2. **Group by section**: targets are batched with their section siblings for context
3. **Send to Gemini 2.5 Flash**: each section batch gets a structured prompt asking for DRRP classification
4. **Compare**: LLM response compared against existing regex/classifier result
5. **Audit log**: all results (including no-ops) written to `data/audit/`
6. **Apply** (optional): corrections written to store as `extraction_method = "agentic"`

## Checking Results

### Audit log files

```bash
# List recent audit logs
ls -lt data/audit/ | head 10

# Check a specific law's results
cat data/audit/UK_uksi_2015_627.json | python3 -m json.tool
```

### Audit log schema (v2, section_targeted)

```json
{
  "law_name": "UK_uksi_2015_627",
  "strategy": "section_targeted",
  "schema_version": 2,
  "model": "gemini-2.5-flash",
  "provisions_count": 349,
  "targets_count": 59,
  "sections_sent": 5,
  "corrections_count": 0,
  "corrections": [],
  "pre_llm_summary": {
    "obligation": 56, "liberty": 3, "none": 290, "pending_llm": 0, "total": 349
  },
  "token_usage": { "input_estimate": 31909 },
  "latency_ms": 20922,
  "integrity_hash": "e4e36fa6d4d8d787"
}
```

### Quick stats across all audit logs

```bash
# Count corrections across all validated laws
for f in data/audit/*.json; do
  law=$(basename "$f" .json)
  corr=$(python3 -c "import json; print(json.load(open('$f'))['corrections_count'])")
  [ "$corr" -gt 0 ] && echo "$law: $corr corrections"
done

# Total corrections
python3 -c "
import json, glob
total = sum(json.load(open(f))['corrections_count'] for f in glob.glob('data/audit/*.json'))
laws = len(glob.glob('data/audit/*.json'))
print(f'{total} corrections across {laws} validated laws')
"
```

### Verify in Postgres

```bash
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
  SELECT extraction_method, count(*)
  FROM legislation_text
  WHERE law_name = 'UK_uksi_2015_627'
  AND extraction_method IS NOT NULL
  GROUP BY extraction_method
  ORDER BY count DESC;
"
```

## Token Cost Estimation

- Gemini 2.5 Flash: ~$0.15/1M input tokens, ~$0.60/1M output tokens
- A typical section batch: 1-5K input tokens, ~500 output tokens
- Dry-run reports `input_estimate` per section — sum for total cost
- Small laws (< 200 provisions): usually whole-law strategy, ~$0.001-0.005 per law
- Large laws: section-targeted, only uncertain provisions sent

## Notes

- **Never validate gold benchmark laws with --force parse first** — this overwrites agentic-tier with regex
- Audit logs accumulate — each run overwrites the log for that law
- The `--audit-dir` flag changes the output directory (default: `data/audit`)
- `integrity_hash` in the audit log can verify the result wasn't tampered with
- Corrections with `delta: "drrp_override"` mean the LLM disagreed with regex/classifier
