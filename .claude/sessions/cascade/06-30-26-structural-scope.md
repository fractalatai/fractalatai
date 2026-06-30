---
session: STRUCTURAL Scope
status: closed
opened: 2026-06-30
closed: 2026-06-30
outcome: success

summary: >
  Wired STRUCTURAL scope into pipeline — provisions with structural purposes
  (definitions, amendments, repeals) and no DRRP modals get actors defaulted
  to "mentioned". Reduced pending_llm by 19% (11,615 → 9,368).

decisions:
  - what: Three-way scope column (out/structural/substantive)
    why: Binary out/substantive left definitional provisions creating false "active" actors that disagreed with SLM, inflating pending_llm
    result: 23,823 provisions reclassified as structural, 2,204 actors defaulted to mentioned

  - what: SQL backfill for existing data, Rust for new parses
    why: One-time fix — pipeline.rs now sets scope at parse time using provision_scope() Pass 2
    result: Consistent going forward, backfill not a pipeline step

metrics:
  structural_provisions: 23823
  actors_defaulted_mentioned: 2204
  pending_llm_before: 11615
  pending_llm_after: 9368
  pending_llm_reduction: 19%
  scope_distribution: { out: 14.6%, structural: 14.6%, substantive: 70.8% }

lessons:
  - title: Fixing the base case reduces downstream workload without re-running expensive tiers
    detail: No re-embedding, no re-classifying, no re-SLM. Just SQL update on scope + regex_position, then re-reconcile. 19% pending_llm reduction at zero compute cost.
    tag: methodology

artifacts:
  - crates/fractalaw-cli/src/commands/pipeline.rs
  - scripts/corpus_stats.py

depends_on:
  - 06-29-26-tier0-base-case

enables:
  - Smaller pending_llm backlog for human-triggered LLM
  - More accurate pipeline coverage stats
---

# Session: STRUCTURAL Scope — Fix Purpose-Based Provision Gating (CLOSED)

## Problem

The `scope` column on `legislation_text` has two values: `out` and `substantive`. The third category — `structural` — was deferred because it requires purpose classification (Rust, not SQL). Without it:

- Regex over-classifies actors on definitional/amendment provisions as "active"
- These create disagreements with SLM (which correctly says "mentioned")
- Reconciliation flags them as `pending_llm` — 3,763 actors (32% of pending_llm backlog)
- These aren't genuinely hard cases — they're provisions that shouldn't have active actors

## Impact

| Metric | Current | After fix |
|--------|---------|-----------|
| pending_llm | 11,615 | ~7,852 (-32%) |
| Actors needing LLM | 16.6% of corpus | ~11.2% |
| LLM cost saved | — | ~32% fewer Gemini calls |

## What STRUCTURAL means

A provision is STRUCTURAL when:
- Its purpose is structural (Interpretation, Amendment, Repeal, Extent, Transitional, Enactment, Unclassified)
- AND it has no DRRP modals (shall, must, ensure, entitled to, may, etc.)
- The `should_default_to_mentioned()` function in `taxa/mod.rs` already implements this check
- The `provision_scope()` function in `taxa/mod.rs` already handles Pass 2 (purpose-based)

## What needs to happen

1. ✅ Wire `provision_scope()` Pass 2 into `parse_provisions` — scope re-evaluated after purpose classification
2. ✅ Backfill: 23,823 provisions set to `structural` (SQL, one-time)
3. ✅ 2,204 actors on STRUCTURAL provisions defaulted to `mentioned`
4. ✅ Re-reconciled — pending_llm dropped from 11,615 to 9,368 (-19%)
5. ✅ pending_llm reduced (9,368 vs predicted ~7,852 — difference is `may` modal correctly keeping some as substantive)
6. ✅ Updated `corpus_stats.py` — Tier 0 now reports OUT / STRUCTURAL / SUBSTANTIVE from scope column
7. ✅ QA: Tier 0 PASS, Tier 2 PASS. pending_llm at 9,368 (14.9%) — remaining are substantive provisions

## Existing code

- `provision_scope(section_type, text, purposes)` — already handles STRUCTURAL in Pass 2, just never called with actual purposes
- `should_default_to_mentioned(purposes, text)` — returns true for structural purposes without DRRP modals
- `STRUCTURAL_PURPOSES` constant in `purpose.rs` — the list of structural purpose labels
- `has_drrp_modal(text)` — checks for obligation + rights + powers modals

## Depends on

- ✅ Tier 0 base case (OUT/SUBSTANTIVE implemented)
- ✅ Purpose classification already populated for most provisions
