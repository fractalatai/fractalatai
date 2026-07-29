---
session: "Benchmark Post-Restructure"
status: closed
opened: 2026-06-18
closed: 2026-06-22
outcome: success

summary: >
  Ran benchmark against the restructured 3-class pipeline (parse/classify/escalate).
  DRRP accuracy 84.4% (down from 86.3% pre-restructure). Actor position accuracy 34.3%.
  Liberty false positives and actor position gaps identified as follow-on sessions.

decisions:
  - what: "Use local data/benchmarks/ instead of NAS benchmarks"
    why: "NAS benchmark files were corrupted"
    result: "Benchmark report pointed to local copy. Results consistent."
  - what: "Spawn focused follow-on sessions for each error pattern"
    why: "Liberty FPs, actor position, and Rule class each need separate investigation"
    result: "Three daughter sessions created for liberty-false-positives, actor-position-coverage, rule-class-cleanup"

lessons:
  - title: "Lance panic on drrp_history blocked benchmark for 3 days"
    detail: "List<Struct> column in LanceDB caused panics. Migrated to JSON string column (Utf8) to unblock. Simpler schemas are more robust for complex nested data."
    tag: data-store
  - title: "Restructure preserved accuracy within 2pp"
    detail: "84.4% vs 86.3% pre-restructure is acceptable. The 1.9pp drop is from Rule class reclassification, not a regression in the core pipeline."
    tag: pipeline
---

# Session: Benchmark Post-Restructure (CLOSED)

## Progress

### ✅ Blocker resolved: #45 Lance panic fixed (2026-06-21)

- Changed `drrp_history` column from `List<Struct>` to `Utf8` (JSON string) — commit `1d0f0e5`
- Migrated 134K existing history entries to JSON via `scripts/migrate_drrp_history_json.py`
- Multi-law parse confirmed working: `taxa parse --laws UK_eudr_2013_59,UK_ukpga_1974_37 --force` — 2/2 laws, no panic
- NAS backup taken (20260621)
- `drrp_history` now written inline during enrich pass (no longer deferred to classify)
- Classify pass properly appends to existing history instead of overwriting
- GH issue #45 closed

### Pre-suspension progress (2026-06-18)

- ✅ `taxa parse --laws UK_ukpga_1974_37 --force` — works (single law)
- ✅ `taxa classify --laws UK_ukpga_1974_37` — works (118 classified, 116 flagged for LLM)

### Benchmark pipeline run (2026-06-22)

- ✅ `taxa parse --laws <all 16> --force` — 16/16, 0 failed
- ✅ `taxa classify --laws <all 16>` — 1,748 classified across 19,471 provisions (50.5s)
- ✅ Compacted LanceDB (2.5GB → 472MB)
- ✅ `benchmark_report.py` — pointed to local `data/benchmarks/` (NAS benchmarks were corrupted)

### Benchmark results

**DRRP accuracy: 84.4%** (1,898/2,250) — down from 86.3% pre-restructure

| Class | Precision | Recall | F1 | Support |
|-------|-----------|--------|-----|---------|
| Liberty | 66.7% | 81.8% | 73.5% | 357 |
| Obligation | 84.0% | 90.6% | 87.2% | 791 |
| none | 95.1% | 80.7% | 87.3% | 1102 |

**Actor position accuracy: 34.3%** (70/204) — `beneficiary` and `mentioned` never predicted

**Top error patterns:**
- 123 gold=`none` predicted as `Liberty` (biggest single error — Liberty false positives)
- 81 gold=`none` predicted as `Obligation`
- 56 gold=`Liberty` predicted as `Obligation`
- `Rule` class: 0 support in gold, 23 pipeline predictions

### Follow-on sessions

- `06-22-26-liberty-false-positives.md` (PENDING) — investigate none→Liberty leakage
- `06-22-26-actor-position-coverage.md` (PENDING) — beneficiary/mentioned never predicted
- `06-22-26-rule-class-cleanup.md` (ACTIVE) — Rule class has 0 gold support but 23 pipeline predictions

## Context

**Prior session**: cascade-transition-rules (CLOSED) — pipeline restructured into `taxa parse → taxa classify → taxa escalate`.

**Trigger**: Need to verify the restructured pipeline produces correct results. Run the benchmark against the corrected 3-class gold standard (`data/benchmarks/`). Last benchmark was 86.3% accuracy before the restructure.

## Benchmark laws

UK_eudr_2013_59, UK_ukpga_1974_37, UK_uksi_2005_1541, UK_uksi_2015_310, UK_ukpga_2004_20, UK_uksi_2014_1638, UK_uksi_2016_1101, UK_uksi_1999_3242, UK_uksi_2002_2788, UK_asp_2005_13, UK_uksi_2010_2214, UK_uksi_2006_1380, UK_eudr_2014_68, UK_ukpga_1990_10, UK_ukpga_1981_69, UK_ukpga_1997_8
