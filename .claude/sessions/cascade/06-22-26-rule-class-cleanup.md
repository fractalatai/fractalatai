---
session: "Rule Class Cleanup"
status: closed
opened: 2026-06-22
closed: 2026-06-22
outcome: success

summary: >
  Remapped DutyFamily::Rule to Obligation in the regex tier. Rule had 0 gold support
  but 23 pipeline predictions (all false positives). Rule regex tier still runs for
  thing-subject detection but outputs Obligation. 138 orphaned provisions (DRRP but no
  actors) identified for follow-on work.

decisions:
  - what: "Remap Rule to Obligation instead of removing Rule detection"
    why: "Thing-subject detection is a useful signal (equipment shall be suitable). The DRRP type should be Obligation with an implied duty-holder."
    result: "Rule FPs eliminated. Liberty precision improved 66.7% to 81.8%. Liberty recall dropped to 64.1% (addressed in liberty-false-positives session)."

lessons:
  - title: "Legacy classes with 0 gold support are noise"
    detail: "Rule was producing only false positives. Remapping to a supported class eliminated an entire error category."
    tag: data-quality
  - title: "Orphaned provisions (DRRP but no actors) are a systemic gap"
    detail: "138 thing-subject provisions have Obligation but no duty-holder. Position classifier skips empty-actor provisions entirely."
    tag: pipeline
---

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
