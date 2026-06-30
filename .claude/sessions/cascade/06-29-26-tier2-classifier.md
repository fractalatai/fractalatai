---
session: Tier 2 — Position Classifier + Dep Features
status: closed
opened: 2026-06-29
closed: 2026-06-30
outcome: success

summary: >
  Classifier gap closed from 50K to zero. Root cause was sequencing (classify ran
  before embedding gap fill) and agentic skip (legacy guard, removed). QA gate added
  to log skipped actors. 68,634 eligible actors classified, QA PASS.

decisions:
  - what: Remove agentic skip from classifier
    why: Each tier writes to its own columns (cls_position). No overwrite risk. Reconciliation picks the winner. Skipping agentic provisions left 1,857 actors unclassified.
    result: Classifier gap zero after re-run

  - what: Target gap laws only when re-running pipeline steps
    why: Re-classifying 68K actors to pick up 1,857 wastes 30 min CPU. Query the gap laws and classify only those.
    result: Lesson learned — not implemented this time but noted for future

metrics:
  eligible_actors: 68634
  classified: 68634
  not_eligible_no_embedding: 1173
  classifier_gap: 0
  qa_warnings: 13

lessons:
  - title: The 50K classifier gap was sequencing, not a code bug
    detail: Classify ran before Tier 1 filled embedding gaps. Re-running classify after embedding fixed most of it. Always run pipeline steps in order — embed before classify.
    tag: methodology

  - title: Each tier writes to its own columns — no skip guards needed
    detail: The agentic skip was from a pre-provision_actors design where classifier wrote directly to extraction_method. With per-tier columns, every tier should write its signal unconditionally. Reconciliation decides the winner.
    tag: architecture

  - title: Target gap laws, don't re-process the whole corpus
    detail: 1,857 gap actors were across ~50 laws. Re-running classify on all 259 laws processed 68K actors unnecessarily. Query for laws with gaps and pass just those to --laws.
    tag: methodology

artifacts:
  - crates/fractalaw-cli/src/commands/taxa.rs
  - scripts/corpus_stats.py

depends_on:
  - 06-29-26-tier1-regex

enables:
  - 06-29-26-tier3-slm
---

# Session: Tier 2 — Position Classifier + Dep Features (CLOSED)

## Problem

The v3 position classifier requires dep features (spaCy batch job) + embeddings. The 50K actor gap identified earlier was NOT a code bug — classify reads actors from `legislation_text.actors` which is populated. The gap was caused by:

1. **Sequencing**: classify was run before Tier 1 filled embedding gaps (16,973 new embeddings)
2. **Silent skip**: when dep features were missing (now fixed — all actors have dep features)

Fix: re-run classify on laws with eligible actors that lack cls_position. Add QA gate to log skipped actors.

## Work

1. ⬜ ~~Fix classify to read actors from provision_actors~~ — NOT a bug, actors are in both tables
2. ✅ Add QA gate: log warning + count when actors skipped for feature mismatch
3. ✅ Dep features computed for all in-scope actors (done in Tier 1 prep)
4. ✅ Descriptive stats updated in corpus_stats.py — uses `scope` column, reports eligible/classified/gap + confidence coverage
5. ✅ Removed agentic skip — classifier now writes cls_position for all eligible actors regardless of extraction_method
6. ✅ Re-run classify — 68,634 eligible actors all classified. QA: PASS (gap = 0)
7. ✅ Updated corpus-stats.py to use `scope` column for all tiers

## Remaining gap: 1,857 actors (2.7%)

These are actors in `provision_actors` that exist on provisions with embeddings and dep features, but the classifier didn't process them. Likely actors added by `taxa infer` (correlative rules) after the last classify run — they're in `provision_actors` but not in `legislation_text.actors` JSONB which the classifier iterates over. A backfill → re-classify cycle would close this, but 2.7% is acceptable for now.

## QA checks (close signal)

- count(actors with embedding AND dep features BUT no cls_position) = 0
- count(dep_is_subject IS NULL WHERE regex_position IS NOT NULL AND in-scope) = 0
- Classifier coverage = 100% of eligible actors (not 42%)

## Depends on

- 06-29-26-tier1-regex
