---
description: Run the Tier 2 DRRP classifier on LanceDB provisions. Replaces LLM calls with microsecond inference.
---

# Skill: Classifier Enrich

## When This Applies

After regex enrichment, to classify DRRP type (Obligation/Liberty/none) on provisions that regex can't reliably classify — multi-actor provisions and DRRP=none with actors.

This is the **production** Tier 2 — no API calls, no Ollama, microsecond inference from a trained logistic regression on embeddings + modal features.

## Usage

```bash
# Classify all eligible provisions in specific laws
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/classify.py --laws UK_uksi_1992_2793,UK_uksi_2005_735

# Classify all QQ applicable laws
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/classify.py --laws $(cat data/qq-applicable-laws.csv)

# Dry run — show what would be classified without writing
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/classify.py --laws UK_uksi_1992_2793 --dry-run
```

## How It Works

1. Loads classifier from `crates/fractalaw-cli/config/drrp_classifier_v8.json`
2. Queries LanceDB for regulation-level provisions needing classification
3. Skips provisions with existing confidence ≥ 0.85 (protects agentic gold data at 0.90)
4. Computes 384-dim embedding + 13 modal features per provision
5. Predicts Obligation/Liberty/none
6. Decomposes to DRRP sub-type using actor label prefix
7. Writes back with `extraction_method = "classifier"`, confidence from model probability
8. Compacts LanceDB after batch write

## Protection gate

The classifier overwrites everything **except agentic** (development gold data):

| Existing method | Action |
|---|---|
| `agentic` / `agentic_unvalidated` | **Skip** — development gold, verified by Gemini |
| `classifier` | **Overwrite** — re-classify with latest model |
| `local` | **Overwrite** |
| `regex` | **Overwrite** |
| `inherited` | **Overwrite** |
| `none` | **Overwrite** |

Structural provision types (title, heading, schedule, etc.) are always skipped — no DRRP to classify.

## Notes

- Requires `scikit-learn`, `onnxruntime`, `tokenizers`, `lancedb`, `pyarrow`
- Model file: `crates/fractalaw-cli/config/drrp_classifier_v8.json` (old .pkl versions deleted)
- ONNX model: `models/all-MiniLM-L6-v2/`
- Always compact after classification to prevent fragment bloat
