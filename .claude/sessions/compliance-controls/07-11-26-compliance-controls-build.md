---
session: Compliance Controls Build Plan
status: suspended
opened: 2026-07-11
---

# Session: Compliance Controls Build Plan (SUSPENDED)

## Problem

The COMPLIANCE-CONTROLS.md v0.2 design is reviewed and approved. It needs to be broken into implementable sessions that can be executed independently, with clear prerequisites between them. This is a meta session — it defines the build phases and spawns implementation sessions as PENDING.

## External Prerequisites

- ✅ `explanatory_note` column populated in DuckDB LRT — 312/428 QQ laws (73%), backfilled via pull-lrt
- ✅ Postgres provisions available with DRRP, actors, fitness, significance
- ✅ DuckDB LRT available with law metadata, family, description, fitness dimensions
- ✅ 384-dim embedding model in pipeline (all-MiniLM-L6-v2)
- ✅ Gemini batch API infrastructure (scripts/gemini_*.py)
- ✅ Design v0.2 reviewed by Gemini Pro (2x), all major concerns resolved

## Build Phases

### Phase 0: Prompt Engineering (no code) — DONE

1. ✅ System prompt v1 with 8 constraints + 3 few-shot examples
2. ✅ Policy predicate prompt v1 with 2 examples
3. ✅ Tested on 5 laws — 100% constraint pass rate, ~4:1 consolidation ratio
4. ✅ Saved to `.claude/plans/compliance-controls/prompts/`

**Session**: `07-11-26-phase0-prompt-engineering.md` (closed)

### Phase 1: Pipeline Script — Generate + Validate — DONE

1. ✅ `scripts/generate_controls.py` — prompt assembly, Gemini call, lint validation, staging table, policy predicate
2. ✅ `scripts/test_generate_controls.py` — 30 tests passing
3. ✅ `pull-lrt` CLI command with defensive `merge_legislation` (whitelist + COALESCE)
4. ✅ Explanatory Note backfilled for QQ corpus
5. ⏸️ Chunking logic (deferred — most laws fit in single call, implement when needed at scale)
6. ⏸️ LLM self-critique via Flash (deferred — deterministic lint catches the main failures)

**Session**: `07-11-26-phase1-pipeline.md` (closed)

### Phase 2: Corpus Run — QQ Family Pilot — DONE

1. ✅ 47 OH&S laws → 349 controls at 3.9:1 ratio
2. ✅ Zero intra-law duplicates, HDBSCAN dropped
3. ✅ Cross-law controls reframed as law-specific interchanges (Mode 2)

**Session**: `07-11-26-phase2-ohs-pilot.md` (closed)

### Phase 2b: Consolidation — DROPPED

Pilot showed 3.9:1 consolidation from the LLM prompt alone. Cross-law "duplicates" are reframed: each control is law-specific and legitimate. Similar controls across laws are tube map interchanges — a presentation/reconciliation concern for Mode 2, not a generation pipeline step. HDBSCAN not needed.

### Phase 3: Full Corpus Run

Run across all ~2K laws in batch. Quality review and iterate.

1. ✅ QQ corpus complete: 220/428 laws → 1,341 controls + 222 predicates
2. ✅ 94.1% validated, 5.9% flagged (73 JUDGEMENT_MISSING, 36 INVALID_REF, 4 PAPERWORK, 2 DEONTIC)
3. ✅ Spot-checked 12 non-OH&S laws (86 controls) — zero deontic, zero paperwork
4. ✅ No prompt iteration needed
5. ✅ 208 QQ laws have no governed provisions — 100% of processable laws complete

**Session**: `07-11-26-phase3-qq-corpus.md` (closed)

### Phase 4: Publish + Delivery

Publish canonical controls to sertantai.

1. ⬜ Zenoh publish format for suggested controls
2. ⬜ CLI: `fractalaw controls publish --tenant dev`
3. ⬜ Sertantai sync into Baserow Controls + Control Mappings tables
4. ⬜ Verify round-trip
5. ⬜ Customer delivery: filter canonical set to customer's Legal Register

**Depends on**: Phase 3 (corpus run complete), sertantai Controls template built
**Enables**: Customer use of L3 Controls

### Future (not scoped)

- Mode 2: Reconciliation — match canonical controls to customer's existing control library, surface tube map interchanges
- Three-way merge on regeneration
- L4 last_touched update mechanism
- Feedback loop: customer edits → prompt improvement

## Dependencies

- ✅ `07-10-26-compliance-controls` — design session, produced COMPLIANCE-CONTROLS.md v0.2
- ✅ `07-11-26-phase0-prompt-engineering` — prompts validated
- ✅ `07-11-26-phase1-pipeline` — pipeline script, pull-lrt, defensive merge
