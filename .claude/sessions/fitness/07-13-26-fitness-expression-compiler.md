---
session: Fitness Expression Compiler
status: active
opened: 2026-07-13
---

# Session: Fitness Expression Compiler (ACTIVE)

## Problem

Sertantai is blocked on Phase 6 (rules engine evaluator) because fractalaw doesn't publish compiled expression trees. The Phase 4 design spike proved the compiler works (~100 lines Python, 5 heuristics), but it was never implemented in Rust or wired into the publish payload.

Currently fractalaw publishes raw `fitness_entities` lists. Sertantai needs `ApplicabilityNode` JSON trees to evaluate "does this law apply to me?" The compiler transforms per-law fitness mentions into boolean expression trees.

## Work

1. ✅ `ApplicabilityNode` enum in `fractalaw-core/src/taxa/applicability.rs` — Match, And, Or, Not, Conditional, TimeWindow + serde JSON + helper methods. 4 tests pass.
2. ✅ Compiler in `fitness.rs`: groups mentions by law, entities by scope dimension (same-dim=OR, cross-dim=AND), DisappliesTo=NOT, temporal=TimeWindow.
3. ✅ `fractalaw fitness compile` CLI command — reads fitness_mentions from Postgres, writes compiled JSON to DuckDB.
4. ✅ `compiled_applicability` VARCHAR column added to DuckDB legislation table. 701 laws compiled.
5. ✅ Added `compiled_applicability` to Zenoh publish SELECT query.
6. ✅ Published 4 benchmark laws with compiled trees — sertantai received valid JSON.
7. ⬜ Publish all 701 laws with compiled trees
8. ✅ Updated ZENOH-SPEC.md v2.3 with compiled_applicability field + full node type spec
9. ✅ Updated FITNESS-APPLICABILITY.md — TimeWindow is a leaf gate (no inner), evaluator fixed
10. ✅ Provided prompt for sertantai Claude to build against the new field

## Dependencies

- ✅ Phase 4 design spike: 5 heuristics proven, prototype in Python (07-11-26-fitness-rule-compiler)
- ✅ fitness_mentions with reconciled entities + scope_unit + polarity
- ✅ FITNESS-RULES-ENGINE.md with ApplicabilityNode spec
- ✅ Zenoh publish payload already includes fitness_entities (just needs one more field)
- ✅ Sertantai has FITNESS-APPLICABILITY.md guide with Elixir evaluator code
