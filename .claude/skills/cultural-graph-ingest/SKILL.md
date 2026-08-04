---
description: Ingest and QA raw safety narrative CSVs for cultural graph extraction. Validates encoding, profiles data, checks for duplicates, outputs cleaned JSONL for RunPod inference.
---

# Cultural Graph: Data Ingestion & QA

## When This Applies

When new safety narrative data arrives from QQ for cultural graph processing. This is step 1 of the monthly workflow:

1. **Ingest & QA** (this skill) → cleaned JSONL
2. `/cultural-graph-runpod` → production inference results
3. `/cultural-graph-load` → DuckDB + site profiles

## Prerequisites

- Raw CSV from QQ in `data/qq/cultural-graph/qq-data/`
- DuckDB at `data/cultural-graph.duckdb` (for duplicate checking)

## Usage

### Profile only (no output)

```bash
/usr/bin/python3 scripts/cultural-graph/ingest_qa.py \
  --input data/qq/cultural-graph/qq-data/Redactor_2027.csv \
  --profile-only
```

### Full ingest with duplicate check

```bash
/usr/bin/python3 scripts/cultural-graph/ingest_qa.py \
  --input data/qq/cultural-graph/qq-data/Redactor_2027.csv \
  --check-dupes
```

Output: `data/qq/cultural-graph/ingest/<filename>-clean.jsonl`

### Custom output path

```bash
/usr/bin/python3 scripts/cultural-graph/ingest_qa.py \
  --input data/qq/cultural-graph/qq-data/Redactor_2027.csv \
  --check-dupes \
  --output data/qq/cultural-graph/ingest/fy2027-batch1.jsonl
```

## What the script does

1. **Detects format** — headerless Redactor CSVs (cp1252) or column-header CSVs (utf-8-sig)
2. **Loads and normalises** — maps columns to standard schema (Id, Site, What, Type, Action, FY, Sector, SubSector)
3. **Combines narrative fields** — `What` + `Action` → single narrative text
4. **Profiles** — record count, report types, sites, word counts, short narratives, encoding issues
5. **Duplicate check** (with `--check-dupes`) — compares IDs against existing DuckDB, excludes duplicates from output
6. **Writes cleaned JSONL** — one record per line, ready for RunPod upload

## QA checks to review

After running, review the profile output for:

- **Very short narratives** (<10 words) — these will produce zero/minimal cultural edges. Expected behaviour.
- **Encoding issues** — records with replacement characters. May need manual inspection.
- **Duplicate IDs** — records already in DuckDB. Automatically excluded from output.
- **New report types** — types not in the training data (e.g., Environmental Incident, Property Damage). The model handles these but extraction quality is unvalidated for these types.
- **New sites** — sites not in historical data. Worth noting for site profile initialisation.

## Supported CSV formats

| Format | Detection | Columns |
|--------|-----------|---------|
| Redactor (headerless) | First field is numeric | Id, Site, What, Type, AtWork, Action, FY, AP, Sector, SubSector |
| Original samples (headers) | First field is `R_SiteCode` | R_SiteCode, RE_What, RE_ReportType, IsAtWork, RE_ActionImmediate |

## File locations

| File | Purpose |
|------|---------|
| `scripts/cultural-graph/ingest_qa.py` | Ingestion and QA script |
| `data/qq/cultural-graph/qq-data/` | Raw CSV input directory |
| `data/qq/cultural-graph/ingest/` | Cleaned JSONL output directory |
| `data/cultural-graph.duckdb` | Existing data (for duplicate check) |
