---
session: Controls Pipeline Script
status: active
opened: 2026-07-11
---

# Session: Controls Pipeline Script (ACTIVE)

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
