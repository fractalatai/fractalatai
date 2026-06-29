# Session: Tier 1 — Regex Parse & Embed (ACTIVE)

## Problem

Regex parse and embedding are the foundation. Every downstream tier depends on them. Tier 0 base case filter is now wired in. Two gaps remain:

1. **28,551 in-scope provisions without embeddings** (19.7%) — entire laws that were never embedded (ingested after last embed run)
2. **Actors on in-scope provisions may be stale** — need to verify regex coverage is complete

## Current state (from corpus_stats.py)

| Metric | Value |
|--------|-------|
| In-scope provisions | 145,158 |
| Has embedding | 116,607 (80.3%) |
| Missing embedding | 28,551 (19.7%) |
| Laws with 100% missing | ~15 laws (never embedded) |
| Has extraction_method | 141,263 (97.3%) |

## Work

Implementation first, heavy processing last:

1. ✅ Base case filter wired into parse_provisions (done in Tier 0)
2. ✅ Tier 1 stats added to `corpus_stats.py` — embedding gap, laws with gaps, actor coverage, QA check
3. ✅ Identified 160 laws with embedding gaps (28,551 in-scope provisions)
4. ✅ Regex coverage: 113,653 of 114,799 actors have regex_position (99.0%)
5. ⬜ Run embedding on 106 QQ gap laws (~28K provisions, ~2-3 hours on CPU). 54 non-QQ gap laws are out of scope.
6. ⬜ Passing descriptive stats = session close signal
7. ✅ Updated corpus-stats skill with Tier 1 checks + --law-file for customer corpus

Note: regex parse and embed are independent — both operate on raw text. The 106 gap laws already have regex parse done (extraction_method set, actors in provision_actors). They just need embeddings so the classifier (Tier 2) can run.

## QA checks (close signal)

- count(in-scope provisions without embedding) = 0
- count(in-scope provisions with DRRP but no actors in provision_actors) = 0
- count(out-of-scope provisions WITH actors in provision_actors) = 0 (inherited from Tier 0)
- Every actor has regex_drrp and regex_position

## Depends on

- ✅ 06-29-26-tier0-base-case (CLOSED)
