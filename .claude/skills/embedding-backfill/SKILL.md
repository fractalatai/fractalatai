# Skill: Embedding Backfill

## When This Applies

When provisions in LanceDB have been enriched (DRRP, actors, positions) but are missing embedding vectors. This happens when:
- Gemini Tier 2 or Tier 3 enriches provisions via write-back (updates actors/DRRP but doesn't compute embeddings)
- QA correction write-back stamps provisions as `agentic` without recomputing embeddings
- New provisions are added to LanceDB without running the full `fractalaw embed` pipeline

The embedding gap prevents these provisions from being used as training data for the Tier 2 classifier.

## Prerequisites

- ONNX model at `models/all-MiniLM-L6-v2/` (model.onnx + tokenizer.json)
- Python packages: `onnxruntime`, `tokenizers`, `lancedb`, `pyarrow`, `numpy`

## Usage

```bash
# Backfill all agentic provisions missing embeddings
/usr/bin/python3 .claude/skills/embedding-backfill/backfill.py

# Backfill a specific extraction method
/usr/bin/python3 .claude/skills/embedding-backfill/backfill.py --method agentic

# Backfill specific laws
/usr/bin/python3 .claude/skills/embedding-backfill/backfill.py --laws UK_uksi_1992_2793,UK_uksi_2005_735

# Dry run — count how many need backfill without computing
/usr/bin/python3 .claude/skills/embedding-backfill/backfill.py --dry-run
```

## How It Works

1. Queries LanceDB for provisions matching the filter with `embedding IS NULL`
2. Loads the ONNX embedding model (`all-MiniLM-L6-v2`, 384-dim)
3. Tokenizes provision text (truncated to 128 tokens)
4. Runs ONNX inference → mean pooling → L2 normalisation
5. Writes embeddings back via `merge_insert` on `section_id`

## Performance

- ~43 embeddings/second on CPU (Intel CoffeeLake)
- 1,539 provisions in 35.6 seconds
- Batch size 50 for merge_insert efficiency

## Notes

- The `fractalaw embed` CLI command rebuilds the ENTIRE table from Parquet — do not use for backfill
- This skill only updates the `embedding` column on existing provisions
- Embeddings are 384-dim float32, L2-normalised (same as the existing embeddings)
- Text is truncated to 128 tokens to match the original embedding pipeline
