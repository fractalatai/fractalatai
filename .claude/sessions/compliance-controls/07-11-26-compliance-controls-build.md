---
session: Compliance Controls Build Plan
status: active
opened: 2026-07-11
---

# Session: Compliance Controls Build Plan (ACTIVE)

## Problem

The COMPLIANCE-CONTROLS.md v0.2 design is reviewed and approved. It needs to be broken into implementable sessions that can be executed independently, with clear prerequisites between them. This is a meta session — it defines the build phases and spawns implementation sessions as PENDING.

## External Prerequisites

Before any implementation session starts:

- ⬜ `explanatory_note` column populated in DuckDB LRT (sertantai scraping — Jason)
- ✅ Postgres provisions available with DRRP, actors, fitness, significance
- ✅ DuckDB LRT available with law metadata, family, description, fitness dimensions
- ✅ 384-dim embedding model in pipeline (all-MiniLM-L6-v2)
- ✅ Gemini batch API infrastructure (scripts/gemini_*.py)
- ✅ Design v0.2 reviewed by Gemini Pro (2x), all major concerns resolved

## Build Phases

### Phase 0: Prompt Engineering (no code)

Write and test the system prompt + few-shot examples against 5-10 laws manually. This is the foundation — if the prompt doesn't produce good controls, the pipeline is worthless.

1. ✅ Write system prompt encoding constraints 1-8
2. ✅ Write few-shot examples: good control, bad→good rewrite, consolidation
3. ✅ Write the policy predicate prompt (separate from specific controls)
4. ✅ Test manually against 5 laws (CS 1997, MHSW 1999, HSWA 1974, COSHH 2002, FSO 2005)
5. ✅ Evaluate outputs against quality signals — all pass
6. ✅ No iteration needed — prompt v1 passed all tests
7. ✅ Final prompts saved to `.claude/plans/compliance-controls/prompts/`

**Depends on**: nothing — can start now
**Enables**: Phase 1
**Estimated effort**: 1-2 sessions

### Phase 1: Pipeline Script — Generate + Validate

Build the Python script that assembles prompts from DuckDB/Postgres and calls Gemini. Includes Phase 2 validation (lint + self-critique).

1. ⬜ Prompt assembly: query DuckDB for law outline, query Postgres for governed provisions, format as the tested prompt template
2. ⬜ Chunking logic: detect large laws (>N governed provisions), split by structural Part, generate independently
3. ⬜ Gemini Pro call: batch-compatible, structured JSON output, error handling
4. ⬜ Phase 2 automated lint: deontic verb check, paperwork referent check, missing judgement flag, provision linkage validation, enum validation
5. ⬜ Phase 2 LLM self-critique: Gemini Flash call with checklist prompt
6. ⬜ Output storage: write to DuckDB `suggested_controls` staging table
7. ⬜ CLI integration: `fractalaw controls generate <law_name>` with `--dry-run`
8. ⬜ Test on the 5 laws from Phase 0

**Depends on**: Phase 0 (finalised prompt)
**Enables**: Phase 2
**Estimated effort**: 2-3 sessions

### Phase 2: Pipeline Script — Consolidate

Build the consolidation step: embed candidate controls, cluster, synthesise, generate policy predicates.

1. ⬜ Embed generated controls (title + description + what_it_checks) using existing embedding infrastructure
2. ⬜ HDBSCAN clustering with min_cluster_size=2, fallback to cosine threshold (>0.85) for small sets
3. ⬜ LLM synthesis prompt for clusters: merge into single control preserving all provision links
4. ⬜ Policy predicate generation: prompt from description + explanatory_note + consolidated controls
5. ⬜ Cross-law dedup: title hash + embedding similarity within family, propose merges
6. ⬜ Update staging table with consolidated controls + policy predicates
7. ⬜ CLI: `fractalaw controls generate --family <family>` runs per-law then consolidation
8. ⬜ Test on one complete family (e.g., OH&S)

**Depends on**: Phase 1 (generate + validate working)
**Enables**: Phase 3
**Estimated effort**: 1-2 sessions

### Phase 3: Full Corpus Run

Run the complete pipeline across ~2K laws. Assess results, tune parameters.

1. ⬜ Run Phase 1-2 across full corpus in batch mode
2. ⬜ Measure: total controls generated, controls per law distribution, constraint pass rates, cluster merge rates
3. ⬜ Spot-check 20-30 controls across families — quality review
4. ⬜ Identify failure modes: laws that produce bad controls, prompt weaknesses, chunking edge cases
5. ⬜ Iterate prompt and parameters based on findings
6. ⬜ Rerun on failed laws after iteration
7. ⬜ Final corpus stats and quality report

**Depends on**: Phase 2 (consolidation working), `explanatory_note` populated
**Enables**: Phase 4
**Estimated effort**: 1-2 sessions

### Phase 4: Publish + Delivery

Publish canonical controls to sertantai. Integrate with Baserow Controls template.

1. ⬜ Zenoh publish format for suggested controls (align with BASEROW-CONTROLS-DESIGN.md schema)
2. ⬜ CLI: `fractalaw controls publish --tenant dev`
3. ⬜ Sertantai sync: receive controls into Baserow Controls + Control Mappings tables
4. ⬜ Verify round-trip: generated → published → visible in Baserow with correct provision links
5. ⬜ Customer delivery: filter canonical set to customer's Legal Register, deliver as Planned controls

**Depends on**: Phase 3 (corpus run complete), sertantai Controls template built
**Enables**: Customer use of L3 Controls, future Mode 2 Reconciliation

### Future (not scoped)

- Mode 2: Reconciliation prompt design and pipeline
- Three-way merge on regeneration
- L4 last_touched update mechanism
- Feedback loop: customer edits → prompt improvement
- Multi-domain regulation detection

## Dependencies

- ✅ `07-10-26-compliance-controls` — design session, produced COMPLIANCE-CONTROLS.md v0.2
