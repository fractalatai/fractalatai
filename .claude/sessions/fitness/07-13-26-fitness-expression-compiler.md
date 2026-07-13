---
session: Fitness Expression Compiler
status: closed
opened: 2026-07-13
closed: 2026-07-13
outcome: success

summary: >
  Built the expression tree compiler in Rust: ApplicabilityNode enum with serde JSON,
  compiler applying 5 heuristics, fitness compile CLI command, 701 laws compiled.
  Added compiled_applicability to Zenoh publish payload. ZENOH-SPEC v2.3 and sertantai
  implementation guide updated. Publish signature now stable — confidence stays as
  fractalaw-internal QA signal, not published.

decisions:
  - what: ApplicabilityNode as tagged union with serde JSON
    why: Sertantai (Elixir) needs to decode and walk the tree. Tagged JSON ({"op": "Match", ...}) is natural for pattern matching in Elixir and self-describing for debugging.
    result: 6 node types (Match, And, Or, Not, Conditional, TimeWindow). Roundtrip JSON serialisation tested.

  - what: TimeWindow is a leaf gate, not a wrapper with inner node
    why: The compiler always places TimeWindow as a sibling in an And node, never wrapping another subtree. Simpler for both compiler and evaluator.
    result: Docs and evaluator code aligned. No inner field in published trees.

  - what: Confidence stays as fractalaw QA signal, not published in trees
    why: Confidence is an extraction quality metric internal to fractalaw. The expression tree represents the law's applicability, not how sure we are about the extraction. Sertantai evaluates the tree as-is.
    result: Publish signature is stable — no foreseeable schema additions.

  - what: Don't bulk publish until sertantai confirms it handles the new fields
    why: Every publish change is a major migration on sertantai. Publishing 700+ laws before sertantai's decoder is ready creates churn. Benchmark laws first, bulk after confirmation.
    result: 4 benchmark laws published for testing. 701 ready for bulk publish on confirmation.

metrics:
  compiler:
    laws_compiled: 701
    mentions_processed: 14258
    node_types_used: ["Match", "And", "Not", "TimeWindow"]
  publish_payload:
    new_fields: ["compiled_applicability"]
    spec_version: "2.3"
    benchmark_laws_published: 4

lessons:
  - title: Finish the build before publishing
    detail: Published benchmark laws before the compiler was built, then had to republish after adding compiled_applicability. Every publish change forces sertantai migration + 700-law republish. Should have completed the full payload design first.
    tag: methodology

  - title: Document the spec BEFORE publishing — not after
    detail: ZENOH-SPEC should be updated and agreed before the first publish with new fields. Sertantai's decoder needs the spec to build against. Publishing first creates unknown-column errors.
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/taxa/applicability.rs
  - crates/fractalaw-cli/src/commands/fitness.rs
  - crates/fractalaw-cli/src/main.rs
  - crates/fractalaw-sync-cli/src/sync.rs
  - /var/home/jason/Desktop/sertantai-legal/docs/zenoh/ZENOH-SPEC.md
  - /var/home/jason/Desktop/sertantai-legal/docs/controls/FITNESS-APPLICABILITY.md

depends_on:
  - 07-11-26-fitness-rule-compiler.md
  - 07-13-26-fitness-reconcile-publish.md

enables:
  - Phase 6 rules engine in sertantai (trees now published)
  - Bulk publish of 701 laws (after sertantai confirms)
  - Customer applicability matching ("does this law apply to me?")
---

# Session: Fitness Expression Compiler (CLOSED)

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
7. ⏸️ Publish all 701 laws (deferred — waiting for sertantai to confirm it handles new fields)
8. ✅ Updated ZENOH-SPEC.md v2.3 with compiled_applicability field + full node type spec
9. ✅ Updated FITNESS-APPLICABILITY.md — TimeWindow is a leaf gate (no inner), evaluator fixed
10. ✅ Provided prompt for sertantai Claude to build against the new field

## Dependencies

- ✅ Phase 4 design spike: 5 heuristics proven, prototype in Python (07-11-26-fitness-rule-compiler)
- ✅ fitness_mentions with reconciled entities + scope_unit + polarity
- ✅ FITNESS-RULES-ENGINE.md with ApplicabilityNode spec
- ✅ Zenoh publish payload already includes fitness_entities (just needs one more field)
- ✅ Sertantai has FITNESS-APPLICABILITY.md guide with Elixir evaluator code
