# Session: Gold Standard Correction (PENDING)

## Context

**Prior session**: `.claude/sessions/cascade/06-11-26-drrp-qa-plan.md`
**Trigger**: Benchmark analysis revealed ~95 provisions where gold=Duty but pipeline correctly says Responsibility. The LLM was prompted with old actor classifications when generating benchmarks.

## Problem

The golden benchmark Parquet files on NAS (`/mnt/nas/sertantai-data/data/fractalaw-benchmarks/tier2-*.parquet`) contain stale DRRP labels:

1. **Category A: Government actors labelled as Duty (~95 provisions)** — NDA, HSE Executive, Secretary of State, Member States bear statutory obligations. Our Hohfeldian model correctly maps these to Responsibility (government + obligation). The gold says Duty because the LLM wasn't told about the governed/government distinction when generating benchmarks.

2. **Offence provisions labelled as Duty (~38 provisions)** — "A person who contravenes is guilty of an offence", "is liable on summary conviction to a fine". These have no modal verb, no DRRP obligation. They are penalty/offence sections that should be gold=none or a separate Offence category. See `.claude/sessions/cascade/06-17-26-offence-provision-gating.md`.

3. **Rule provisions labelled as Duty (~28 provisions)** — "A notice must be given", "The procedures must require". Thing-subject provisions where the duty-bearer is implied from context (prior sections). These are correctly classified as Rule by the pipeline. The gold should either be Rule or flagged as LLM-only (context-dependent).

## Scope

- Fix ~95 Category A provisions: Duty → Responsibility in gold
- Fix ~38 offence provisions: Duty → none in gold
- Review ~28 Rule provisions: Duty → Rule or flag as LLM-territory
- Regenerate affected benchmark Parquet files on NAS
- Re-run `benchmark_report.py` to establish corrected baseline

## Approach

1. Write a script to identify all affected provisions by comparing pipeline output with gold
2. For Category A: automated fix — any gold=Duty provision where the pipeline found a government actor → change gold to Responsibility
3. For offence provisions: automated fix — any gold=Duty provision with no modal verb and offence language → change gold to none
4. For Rule provisions: manual review of a sample, then automated fix for clear cases
5. Write corrected Parquet files to NAS
6. Re-run benchmarks to establish the true baseline

## Expected outcome

After correction, the benchmark should show:
- DRRP accuracy: ~80%+ (up from 72.4% due to removing false "misclassifications")
- Duty detection: ~87%+ (up from 82.7% due to removing offence/rule false expectations)
- Cleaner signal for identifying real pipeline improvements vs gold standard noise
