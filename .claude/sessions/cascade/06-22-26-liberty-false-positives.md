# Session: Liberty False Positives (PENDING)

## Problem

Benchmark shows 123 gold=`none` provisions classified as `Liberty` — the single largest error source, accounting for ~35% of all DRRP mismatches. Liberty precision is only 66.7% (81.8% recall).

An additional 56 gold=`Liberty` provisions are misclassified as `Obligation`, suggesting the Liberty/Obligation boundary is also fuzzy.

## Benchmark reference

After Rule→Obligation remap (06-22-26-rule-class-cleanup.md, CLOSED):

| Class | Precision | Recall | F1 | Support |
|-------|-----------|--------|-----|---------|
| Liberty | 81.8% | 64.1% | 71.9% | 357 |
| Obligation | 86.1% | 80.5% | 83.2% | 791 |
| none | 83.4% | 93.0% | 87.9% | 1102 |

Overall: 84.0% (1,891/2,250)

**Primary issue shifted**: Liberty precision is now strong (81.8%) but **recall dropped to 64.1%** — 128 gold=Liberty provisions are being missed (68 classified as none, 60 as Obligation). The false-positive problem (none→Liberty) shrank from 123 to 33 after the re-parse.

## Investigation plan

1. Sample the 68 Liberty→none misses and 60 Liberty→Obligation misses — what patterns are being missed?
2. Check regex enabling-modal patterns ("may", "entitled", "power to") — are they too narrow?
3. Check classifier behaviour on Liberty provisions — is the 0.9 disagreement threshold too high?
4. Compare against pre-restructure breakdown to identify regression source

## Key files

- `fractalaw-core/src/taxa/` — regex DRRP extraction
- `fractalaw-ai/src/drrp_classifier/` — trained classifier
- `scripts/benchmark_report.py` — can filter by family for focused analysis
- `data/benchmarks/` — gold standard Parquet files
