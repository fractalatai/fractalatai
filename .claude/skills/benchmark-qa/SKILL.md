---
description: Run benchmark regression tests — compare pipeline DRRP and actor positions against golden benchmarks per tier.
---

# Skill: Benchmark QA

## When This Applies

After code changes to the DRRP pipeline (regex patterns, classifier, reconciliation) to verify accuracy hasn't regressed. Compares each tier's output independently against gold standard.

**Trigger**: User asks to run benchmarks, check regression, compare against gold, or test pipeline accuracy.

## Architecture

Benchmarks use two Postgres tables:
- `provision_actors` — per-actor signals from each tier (regex_drrp, regex_position, cls_drrp, cls_position, etc.)
- `gold_benchmarks` — gold standard (section_id, actor_label, gold_drrp, gold_position)

Comparison is a simple SQL JOIN — no JSONB parsing, no Python extraction.

## Usage

```bash
# Full benchmark report — per-tier accuracy + disagreement analysis
/usr/bin/python3 scripts/benchmarks/benchmark_report.py

# Filter by actor category
/usr/bin/python3 scripts/benchmarks/benchmark_report.py --category Gvt
```

## What It Reports

1. **Per-tier accuracy**: regex vs classifier DRRP and position accuracy
2. **Disagreement analysis**: when tiers disagree, who's right?
3. **Per-category breakdown**: accuracy by actor type (Org, Ind, Gvt, Spc, etc.)
4. **Confusion matrices**: per-tier position confusion matrix

## Current Baseline (2026-06-26)

| Metric | Regex | Classifier |
|--------|-------|-----------|
| DRRP | 84.6% | 84.6% |
| Position | 51.2% | 57.7% |
| Matched actors | 986/4,062 | Same |

### Disagreement analysis
- Agree + correct: 37.9%
- Agree + wrong: 18.5%
- Disagree, regex right: 13.3%
- Disagree, classifier right: 19.8%
- Both wrong: 10.5%

### Per-category highlights
- Classifier better on: Ind (52.8% vs 42.2%), Spc (72.3% vs 44.7%)
- Regex better on: Gvt (59.7% vs 55.1%), Org (72.5% vs 70.6%)

## Benchmark Data

- **Gold table**: `gold_benchmarks` in Postgres (4,062 rows)
- **Source**: NAS parquets at `/mnt/nas/sertantai-data/data/fractalaw-benchmarks/tier2-*.parquet`
- **Gold source**: Gemini 2.5 Flash structured classification
- **Scope**: 20 benchmark laws across 16 families
- **IMPORTANT**: Never re-process benchmark laws (is_benchmark = true in DuckDB)

## Prerequisites

- Postgres running on port 5433
- `provision_actors` table populated (run `taxa parse --pg --force` + `taxa classify --pg` on benchmark laws)
- `gold_benchmarks` table populated (run the load script in benchmark-qa session)

## Populating for new benchmarks

```bash
# Re-parse + re-classify benchmark laws to populate provision_actors
BENCH="UK_asp_2005_13,UK_eudr_2013_59,..."
fractalaw taxa parse --pg postgres://...  --laws "$BENCH" --force
fractalaw taxa classify --pg postgres://... --laws "$BENCH"
```

## Notes

- Gold standard is Gemini-generated, not human-verified
- Actor label matching uses canonical labels (ALIASES dict in benchmark_report.py)
- 35% actor recall — regex only finds ~1/3 of gold actors. The rest are gap-fill candidates.
- Position classifier v2 weights: `docs/position_classifier_v2.json` (4-class, 411 features)
