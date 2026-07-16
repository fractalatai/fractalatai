---
session: Controls Pipeline Script
status: closed
opened: 2026-07-11
closed: 2026-07-11
outcome: success

summary: >
  Built the controls generation pipeline script (prompt assembly, Gemini call, lint
  validation, DuckDB staging table) with 30 tests. Added pull-lrt CLI command with
  defensive merge_legislation that whitelists sertantai-owned columns and skips NULLs.
  Backfilled explanatory_note for 312/428 QQ laws.

decisions:
  - what: Defensive merge_legislation with whitelist + COALESCE
    why: >
      The original upsert_legislation does DELETE+INSERT, wiping all enriched columns
      (taxa, fitness, significance). A naive UPDATE also overwrites non-NULL values with
      NULLs from sertantai. The fix: whitelist only sertantai-owned columns, use
      COALESCE(src.col, legislation.col) to preserve existing data when sertantai sends NULL.
    result: Zero data loss on 428-law QQ backfill. Enriched columns fully preserved.

  - what: Always include Explanatory Note in controls prompt (no body_paras threshold)
    why: >
      Initial threshold of body_paras > 30 excluded COSHH (22 body_paras, 51 governed
      provisions). The note is only ~350 tokens at 500 chars — cheap to include. Performance
      was good without it, so it's additive context.
    result: All laws with notes get them in the prompt. Truncated to 500 chars (controls) / 2000 chars (predicate).

  - what: Column name mapping in merge (title_en → title, family_ii → sub_family)
    why: Sertantai uses different field names than DuckDB for some columns
    result: Mapping handled in merge_legislation, transparent to callers

metrics:
  pipeline_tests: { total: 30, passing: 30 }
  explanatory_note: { qq_total: 428, with_note: 312, coverage_pct: 73, avg_chars: 4645, max_chars: 10069 }
  pull_lrt: { laws_pulled: 428, skipped: 0 }
  data_integrity: { qq_with_title: 427, qq_with_duty_holder: 294, qq_in_duckdb: 428 }

lessons:
  - title: upsert_legislation is destructive — DELETE+INSERT wipes enriched columns
    detail: >
      The existing upsert_legislation deletes the row then inserts with only the columns
      in the Arrow batch. Sertantai's LRT response doesn't include taxa, fitness, or
      significance columns — so they get wiped. Discovered when a test pull-lrt on
      Confined Spaces nulled the title, body_paras, and duty_holder. Fixed with
      merge_legislation (UPDATE with whitelist + COALESCE).
    tag: data

  - title: Sertantai LRT Arrow response didn't include explanatory_note initially
    detail: >
      The ZENOH-SPEC lists explanatory_note in the JSON schema but the Arrow IPC
      encoder on sertantai's side wasn't including it. Required a fix on sertantai
      before the backfill could work. Always verify what the Arrow batch actually
      contains, not what the spec says.
    tag: infrastructure

  - title: customer-laws --output writes JSON not CSV
    detail: >
      The CLI customer-laws command with --output wrote the raw JSON response instead
      of comma-separated law names. The downstream scripts expect CSV format. Fixed
      manually with a Python conversion. The CLI output format should match the expected
      input format.
    tag: tooling

  - title: NAS backup is the safe restore source, not seed parquet
    detail: >
      Seed parquet may be stale. NAS backups are dated and recent. Use ATTACH to
      restore specific rows from a backup DuckDB without replacing the whole file.
      Schema mismatches between backup and live need column intersection handling.
    tag: data

artifacts:
  - scripts/generate_controls.py
  - scripts/test_generate_controls.py
  - scripts/backfill_explanatory_note.py
  - crates/fractalaw-store/src/duck.rs
  - crates/fractalaw-sync-cli/src/main.rs
  - crates/fractalaw-sync-cli/src/sync.rs

depends_on:
  - 07-11-26-phase0-prompt-engineering.md
  - 07-10-26-compliance-controls.md

enables:
  - Phase 2 consolidation (embedding + HDBSCAN + synthesis)
  - Phase 3 full corpus run
  - Safe LRT refresh from sertantai (pull-lrt with defensive merge)
---

# Session: Controls Pipeline Script (CLOSED)

## Problem

Phase 0 validated the prompts. Now we need a Python script that assembles prompts from DuckDB (law metadata) and Postgres (provisions), calls Gemini Pro, validates the output, and stores results. This is the core pipeline — `scripts/generate_controls.py`.

## Work

1. ✅ Script skeleton: argparse, DB connections (DuckDB + Postgres), Gemini API setup
2. ✅ Prompt assembly: query law outline from DuckDB, governed provisions from Postgres, format as tested template
3. ✅ Gemini Pro call: structured JSON output, thinkingBudget, error handling, rate limiting
4. ✅ Phase 2 automated lint: deontic verb check, paperwork referent check, missing judgement flag, provision linkage, enum validation
5. ✅ Output storage: write to DuckDB `suggested_controls` staging table
6. ✅ CLI flags: `--law`, `--family`, `--all`, `--dry-run`, `--limit`, `--skip-predicate`
7. ✅ Test suite: 28 tests covering DB access, prompt assembly, lint validation, staging table
8. ✅ Policy predicate generation (separate call after controls)
9. ✅ Explanatory Note: column added to DuckDB, prompt logic integrated (500 chars for controls, 2000 chars for predicates)
   - ✅ `pull-lrt` CLI command added to fractalaw-sync (with `merge_legislation` to preserve enriched columns)
   - ✅ Backfilled 428 QQ laws — 312 (73%) have explanatory_note, avg 4,645 chars

## Dependencies

- ✅ Phase 0 prompts validated (system-prompt-v1.md, policy-predicate-prompt-v1.md)
- ✅ Existing Gemini batch pattern (scripts/gemini_llm_batch.py — psycopg2 + requests)
- ✅ DuckDB legislation table accessible
- ✅ Postgres provisions accessible (fractalaw:fractalaw@localhost:5433)
