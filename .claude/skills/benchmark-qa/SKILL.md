---
description: Run benchmark regression tests — compare pipeline DRRP and actor positions against Gemini golden benchmarks on NAS.
---

# Skill: Benchmark QA — Golden Benchmark Regression Testing

## When This Applies

After code changes to the DRRP pipeline (regex patterns, purpose gates, classifiers) to verify accuracy hasn't regressed. Compares current pipeline output against Gemini-verified golden benchmarks.

**Trigger**: User asks to run benchmarks, check regression, compare against gold standard, or test pipeline accuracy.

## What It Does

1. Loads golden benchmark Parquet files from NAS (2,250+ Gemini-verified provisions across 16 families)
2. Queries LanceDB for the same provisions (current pipeline output)
3. Reports DRRP type accuracy: confusion matrix, per-class precision/recall/F1
4. Reports actor position accuracy: regex vs gold standard
5. Optionally runs the position classifier to compare regex vs classifier vs gold (three-way)

## Usage

```bash
# Full benchmark report — DRRP types + actor positions vs gold
/usr/bin/python3 scripts/benchmark_report.py

# Filter to one family
/usr/bin/python3 scripts/benchmark_report.py --family "OH&S"

# Show more mismatches
/usr/bin/python3 scripts/benchmark_report.py --mismatches 30

# Position classifier disagreement analysis — regex vs classifier vs gold
/usr/bin/python3 scripts/benchmark_classifier_disagreements.py

# Filter classifier analysis to one family
/usr/bin/python3 scripts/benchmark_classifier_disagreements.py --family "OH&S"
```

## Benchmark Data

- **Location**: `/mnt/nas/sertantai-data/data/fractalaw-benchmarks/tier2-*.parquet`
- **Size**: 2,250 provisions across 16 families (20 Parquet files)
- **Gold source**: Gemini 2.5 Flash structured classification
- **Schema**: `section_id, law_name, family, text, gold_drrp_types, gold_actors (JSON), gold_reasoning, gold_source, created_at`

## Key Metrics (Baseline — 2026-06-11)

| Metric | Value | Target |
|--------|-------|--------|
| DRRP accuracy | 67.1% | 80%+ |
| Position accuracy (regex) | 37.1% → 43.9%* | 60%+ |
| Position accuracy (classifier) | 57.1%* | 60%+ |
| Duty recall | 47% | 70%+ |
| Right recall | 37% | 60%+ |

*Measured on 1,040 actor-position pairs with embeddings (2026-06-17)

## Classifier Disagreement Analysis

The three-way analysis (`benchmark_classifier_disagreements.py`) compares:
- **Regex position**: what the pipeline currently ships
- **Classifier prediction**: logistic regression on embeddings (413-dim features)
- **Gemini gold**: ground truth

Key finding: classifier beats regex overall (57.1% vs 43.9%) but the advantage is entirely from non-DRRP provisions. For actual Duty/Right/Responsibility/Power provisions, regex is better. See session doc for full breakdown.

## Environment

- Requires NAS mounted at `/mnt/nas/sertantai-data/`
- Uses `/usr/bin/python3` (system Python)
- Dependencies: `lancedb`, `pyarrow`, `numpy`
- LanceDB at `data/lancedb`
- Position classifier weights at `docs/position_classifier_v1.json`

## Scripts

- `scripts/benchmark_report.py` — DRRP + position accuracy vs gold
- `scripts/benchmark_classifier_disagreements.py` — three-way regex vs classifier vs gold
- `scripts/generate_benchmarks.py` — generate new benchmarks (requires `GEMINI_API_KEY`)
- `scripts/generate_benchmarks_batch.py` — batch benchmark generation

## Limitations

- Gemini is the gold standard — LLM-checking-LLM, not human-verified
- 39% of benchmark provisions lack embeddings (invisible to classifier analysis)
- Actor label matching between gold and pipeline is fuzzy — some pairs missed
- 6 benchmark laws failed due to Gemini rate limits (Environmental Protection x2, HR Employment, Nuclear, Planning, Pollution)
