---
description: Load cultural graph inference results into DuckDB. Normalises edge types, appends to existing tables, checks for duplicates, recomputes site profiles. Step 3 of the monthly cultural graph workflow.
---

# Cultural Graph: Load Results

## When This Applies

After `/cultural-graph-runpod` has produced inference results. This is step 3 of the monthly workflow:

1. `/cultural-graph-ingest` → cleaned JSONL
2. `/cultural-graph-runpod` → extraction results JSONL
3. **Load results** (this skill) → DuckDB + site profiles

## Prerequisites

- Results JSONL from RunPod at `data/qq/cultural-graph/results/<name>-results.jsonl`
- DuckDB at `data/cultural-graph.duckdb` with existing tables
- Normalisation config at `scripts/cultural-graph/config/normalise.yaml`

## Usage

### Dry run (show what would be loaded, no writes)

```bash
/usr/bin/python3 scripts/cultural-graph/load_results.py \
  --input data/qq/cultural-graph/results/<name>-results.jsonl \
  --dry-run
```

### Load into DuckDB

```bash
/usr/bin/python3 scripts/cultural-graph/load_results.py \
  --input data/qq/cultural-graph/results/<name>-results.jsonl
```

## What the script does

1. **Loads normalisation config** from `scripts/cultural-graph/config/normalise.yaml`
2. **Reads results JSONL** from RunPod inference
3. **Normalises edge types** — maps model hallucinations to canonical types, drops unmappable variants
4. **Normalises entity types** — maps non-5P types to nearest 5P category
5. **Checks for duplicates** — skips records already in DuckDB
6. **Appends to DuckDB** — narratives, entities, edges tables (incremental, not recreate)
7. **Shows site profile** — Voice/Drift percentages for the new data's financial year

## Edge Type Normalisation

The normalisation config (`scripts/cultural-graph/config/normalise.yaml`) maps model hallucination patterns to canonical types:

```yaml
edge_normalise:
  "oper,ational": operational
  shares-information_with: shares-information-with
  speaks-up-up: speaks-up-to
  # ... etc
```

**When to update the config:** After each batch run, check the script's "Edge types dropped" count. If significant, inspect the raw results for new hallucination patterns and add mappings to the config.

## Post-Load Checklist

1. **Check insert counts** — narratives, entities, edges should all increase
2. **Check normalisation stats** — normalised count should be low (<2%), dropped should be near-zero
3. **Review site profile** — does the new data's Voice/Drift look reasonable?
4. **Back up DuckDB** to NAS: `cp data/cultural-graph.duckdb /mnt/nas/sertantai-data/data/fractalaw-backups/YYYYMMDD/cultural-graph/`
5. **Update site profiles brief** if significant changes

## File Locations

| File | Purpose |
|------|---------|
| `scripts/cultural-graph/load_results.py` | Load and normalise script |
| `scripts/cultural-graph/config/normalise.yaml` | Edge/entity type normalisation rules |
| `data/qq/cultural-graph/results/` | Input: inference results JSONL |
| `data/cultural-graph.duckdb` | Output: production graph database |

## DuckDB Schema

```sql
narratives (id, site, report_type, fy, sector, sub_sector, word_count,
            entity_count, cultural_edge_count, operational_edge_count, extracted_at)
entities   (narrative_id, entity_text, entity_type)
edges      (narrative_id, source_node, target_node, edge_type, detail, is_cultural)
```

## Notes

- **Append, not recreate.** The script adds to existing tables. Run it multiple times safely — duplicates are detected and skipped.
- **Back up before large loads.** If loading a full year or reprocessing historical data, back up DuckDB to NAS first.
- **The normalisation config is the single source of truth** for mapping model output to canonical types. Keep it updated.
