# Session: Rule Class Cleanup (CLOSED)

## Problem

The benchmark showed `Rule` has **0 gold support** but the pipeline produced **23 `Rule` predictions** — all false positives.

## Investigation

Rule was a legacy 3rd DRRP class for "thing-subject" provisions — obligations attaching to things (equipment, routes, scaffolding) rather than person-actors. Produced by `duty_patterns_rule.rs` (Tier 5 regex). The gold standard decision on 2026-06-17 was to treat these as Obligation with implied duty-holders.

### Findings

- **Source**: only the regex Rule tier (`duty_patterns_rule.rs`) produced Rule. The trained classifier already used the 3-class model (Obligation/Liberty/none).
- **176 Rule provisions** in benchmark laws: 38 had actors from regex, **138 orphaned** (no actor at all)
- Position classifier skips empty-actor provisions (`main.rs:5271`) — orphans never get actors assigned
- Orphan resolution deferred to pending session `06-22-26-actor-position-coverage.md`

## Fix applied

Remapped `DutyFamily::Rule → vec![DutyType::Obligation]` in `duty_type.rs:137`. The Rule regex tier still runs (useful thing-subject detection signal) but outputs Obligation.

### Benchmark results after fix

**DRRP accuracy: 84.0%** (1,891/2,250)

| Class | Precision | Recall | F1 | Support |
|-------|-----------|--------|-----|---------|
| Liberty | 81.8% | 64.1% | 71.9% | 357 |
| Obligation | 86.1% | 80.5% | 83.2% | 791 |
| none | 83.4% | 93.0% | 87.9% | 1102 |

Rule false positives eliminated. Liberty precision improved (66.7% → 81.8%), but recall dropped (81.8% → 64.1%) — the re-parse redistributed some classifications. Liberty recall is now the dominant issue → feeds into `06-22-26-liberty-false-positives.md`.

## Files changed

- `crates/fractalaw-core/src/taxa/duty_type.rs:137` — Rule→Obligation mapping
- `crates/fractalaw-core/src/taxa/duty_type.rs:329` — test updated
- `scripts/benchmark_report.py:23` — BENCHMARK_DIR pointed to local `data/benchmarks/`
