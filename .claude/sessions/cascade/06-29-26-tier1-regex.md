---
session: Tier 1 — Regex Parse & Embed
status: closed
opened: 2026-06-29
closed: 2026-06-29
outcome: success

summary: >
  Added scope column to legislation_text (out/substantive). Embedded 106 QQ gap
  laws (16,973 provisions in 21 min). Embedding coverage 81.3% → 99.9%.
  taxa embed now respects scope and uses --laws for targeted processing.

decisions:
  - what: Add scope column to legislation_text persisting the base case
    why: Every downstream step was re-evaluating section_type + text length. A column makes filtering a one-line WHERE clause.
    result: out (31,678) / substantive (156,655). STRUCTURAL deferred to parse-time Rust evaluation.

  - what: Use taxa embed --laws (not top-level embed) for gap filling
    why: Top-level embed reads stale Parquet. taxa embed reads Postgres, accepts --laws, skips existing embeddings.
    result: 16,973 embeddings in 21 min vs ~3 hours for full Parquet re-process

metrics:
  embedding_before: { coverage: 81.3%, count: 74500, in_scope: 91605 }
  embedding_after: { coverage: 100%, count: 91605, in_scope: 91605 }
  new_embeddings: 16973
  time: 21 minutes

lessons:
  - title: Boundary alignment matters — > vs >= on text length filter
    detail: Scope backfill used >= 20, embed code used > 20. 132 provisions fell through the 1-char gap. All exactly 20 chars. Trivial but illustrates why the filter should be defined once.
    tag: methodology

  - title: taxa embed --laws already existed under the taxa subcommand
    detail: The top-level embed command reads Parquet and has no --laws flag. taxa embed reads Postgres, accepts --laws, and skips existing embeddings. Use taxa embed for gap filling.
    tag: tooling

artifacts:
  - crates/fractalaw-cli/src/commands/taxa.rs
  - scripts/pg_schema.sql
  - scripts/corpus_stats.py
  - .claude/skills/corpus-stats/SKILL.md

depends_on:
  - 06-29-26-tier0-base-case

enables:
  - 06-29-26-tier2-classifier
---

# Session: Tier 1 — Regex Parse & Embed (CLOSED)

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
5. ✅ Embedded 106 QQ gap laws — 16,973 new embeddings in 21 min. Coverage: 81.3% → 99.9% (132 remaining, 0.1%)
6. ✅ Passing descriptive stats: Tier 0 PASS, Tier 1 99.9% embedding coverage
7. ✅ Updated corpus-stats skill with Tier 1 checks + --law-file for customer corpus

Note: regex parse and embed are independent — both operate on raw text. The 106 gap laws already have regex parse done (extraction_method set, actors in provision_actors). They just need embeddings so the classifier (Tier 2) can run.

## Implemented

- ✅ **`scope` column added to `legislation_text`** — currently two values: `out` (31,678) and `substantive` (156,655). Backfilled via SQL.
- ✅ **`taxa embed` now filters by scope** — only embeds `scope = 'substantive'` provisions. Reads from Postgres via `--laws`, not stale Parquet.
- ✅ **`taxa embed --laws` already existed** — under the `taxa` subcommand (not top-level `embed`). Accepts comma-separated law names.

## TODOs (beyond this session)

- ⬜ **Set STRUCTURAL scope from Rust during parse** — the SQL backfill can't reliably evaluate purpose + modal override. The `provision_scope()` function in `taxa/mod.rs` does this correctly (Pass 2 with purpose). Wire it into `parse_provisions` to set `scope` on `legislation_text` at parse time. Then backfill existing provisions by re-running parse.
- ⬜ **Wire scope into LAT sync ingest** — when sertantai sends new laws, set scope at ingest time so downstream steps know immediately what's in/out.
- ⬜ **Top-level `embed` command should use Postgres** — currently reads from stale Parquet. Low priority since `taxa embed --laws` works correctly.

## QA checks (close signal)

- count(in-scope provisions without embedding) = 0
- count(in-scope provisions with DRRP but no actors in provision_actors) = 0
- count(out-of-scope provisions WITH actors in provision_actors) = 0 (inherited from Tier 0)
- Every actor has regex_drrp and regex_position

## Depends on

- ✅ 06-29-26-tier0-base-case (CLOSED)
