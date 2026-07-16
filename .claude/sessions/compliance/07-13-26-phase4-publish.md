---
session: Controls Publish
status: closed
opened: 2026-07-13
closed: 2026-07-14
outcome: success

summary: >
  Published 1,341 controls + 218 predicates to sertantai. Built publish-controls CLI,
  zenoh key expressions, indexed provision approach (100% resolution). Full section_id
  migration: art→reg prefix for 126 UK SI laws, doubled number fix for 389 laws (6,307 rows).
  172-law reparse batch enriched end-to-end with SLM on RunPod. 33 non-making laws cleaned.

decisions:
  - what: Indexed provision approach for controls
    why: LLM hallucinated section_ids when asked to return string refs. Numbering provisions in the prompt and returning integer indices eliminates hallucination entirely.
    result: 100% resolution on all regenerated laws. Zero corrupted refs.

  - what: CASCADE FKs on provision_actors and fitness_mentions
    why: Deleting LAT for non-making laws required manual 3-table cascade. Adding ON DELETE CASCADE makes it a single DELETE.
    result: Single DELETE FROM legislation_text cascades to all child tables.

  - what: Section_id migration (art→reg + doubled numbers)
    why: February 2026 sertantai parser bug created doubled section_ids (reg.14(14)(2)). art→reg prefix mismatch from pre-fix parser. Both prevented controls from resolving on sertantai.
    result: Zero art. on UK type codes, zero doubled section_ids across entire corpus.

  - what: Significance sequencing fix
    why: Ran significance before derive_hierarchy → backfill set significance_overall = NULL despite 4/5 dimensions present. derive_hierarchy provides the 5th dimension.
    result: Documented in skill: backfill → significance → derive_hierarchy → backfill.

metrics:
  controls_published: { controls: 1341, predicates: 218, laws: 219 }
  section_id_migration: { art_to_reg_renamed: 12000, doubled_deleted: 6307, orphan_actors: 3811, orphan_fitness: 13324 }
  reparse_batch: { laws: 172, position_slm: 13744, significance_slm: 13439, fitness_slm: 1146 }
  non_making_cleanup: { deleted: 33, kept_false_neg: 5 }

lessons:
  - title: Sertantai parser bug caused doubled section_ids, not fractalaw ingestion
    detail: >
      The doubled reg.14(14)(2) section_ids came from sertantai's LAT parser in February 2026,
      not from our pull-lat ingestion. When sertantai fixed and reparsed, correct reg.14(2)
      rows arrived alongside stale doubled ones. Our upsert_lat stored both because different
      section_ids = no conflict. Fix: delete stale, keep correct.
    tag: data

  - title: Two-pass section_id rename needed for parent/child relationships
    detail: >
      reg.N(N) parents must rename before reg.N(N)(sub) children — otherwise children clash
      with the not-yet-renamed parent. But bare reg.N rows may already exist, blocking
      parent rename. Solution: delete doubled rows that clash, then rename parents, then children.
    tag: data

  - title: Triage false negatives self-correct via taxa publish
    detail: >
      Small laws with few obligations get triaged as not_making but enrichment finds obligations.
      Sertantai resolves this: taxa publish sends obligation data → sertantai sets is_making=true
      regardless of triage signal. No DuckDB override needed.
    tag: architecture

  - title: Fitness extract must run before fitness SLM
    detail: >
      Fitness SLM queries fitness_mentions table. After art→reg migration, the old art.
      fitness_mentions were deleted. New reg. fitness_mentions need fitness extract first
      (regex polarity + dictionary extraction) before SLM can process them.
    tag: methodology

  - title: Non-making law identification by title keywords
    detail: >
      Amendment, Commencement, Revocation, and Extension in the title reliably identify
      non-making laws. Combined with DuckDB is_amending/is_commencing flags, these are
      safe to delete without provision-level review.
    tag: methodology

artifacts:
  - crates/fractalaw-sync/src/zenoh_sync.rs
  - crates/fractalaw-sync-cli/src/sync.rs
  - crates/fractalaw-sync-cli/src/main.rs
  - crates/fractalaw-store/src/duck.rs
  - scripts/generate_controls.py
  - scripts/ml/runpod_slm_batch.py
  - scripts/ml/runpod_significance_batch.py
  - scripts/ml/runpod_fitness_batch.py
  - .claude/skills/customer-batch-parse/SKILL.md
  - .claude/skills/runpod-batch-inference/SKILL.md
  - .claude/skills/control-creation/SKILL.md
  - data/qa-results/not-making-reparse-2026-07-14.csv

depends_on:
  - 07-11-26-phase3-qq-corpus.md
  - 07-11-26-phase1-pipeline.md

enables:
  - Clean controls delivery to customers (100% section_id resolution)
  - Future LAT repulls won't create duplicates (doubled IDs eliminated at source)
  - Skill refactor (fractalatai/fractalatai#49)
---

# Session: Controls Publish (CLOSED)

## Problem

1,341 controls + 222 predicates are in the DuckDB staging table. The end-to-end flow requires work on both sides:

- **Sertantai** needs: a trigger mechanism to request control generation, a Postgres table to store controls, and a subscriber to receive published controls
- **Fractalaw** needs: an Arrow schema for the controls payload, a zenoh publish path, and a sync watch handler to respond to control generation requests

The Baserow Controls template already exists in sertantai — that's the UI layer. The missing piece is the data pipeline between fractalaw (generates controls) and sertantai (stores and serves them).

## Work

### Fractalaw side
1. ✅ Define Arrow schema for controls + predicate publish payloads — `CONTROLS-PUBLISH-SCHEMA.md`
2. ✅ Define trigger schema — sertantai requests control generation via `events/controls`
3. ✅ Zenoh key expressions added to fractalaw-sync (`keys::controls`, `keys::controls_predicate`, `keys::events_controls`)
4. ✅ Publish code — `publish-controls` CLI command reads DuckDB staging, extracts JSON fields to Arrow columns, publishes via zenoh. Tested: 3 controls + 1 predicate for CS 1997.
5. ⏸️ Extend sync watch to handle control generation requests (deferred — DRRP and Controls are async)
   ✅ Created `/control-creation` skill capturing the full generation workflow

### Sertantai side (external)
6. ⬜ Postgres controls table to store published controls
7. ⬜ Zenoh subscriber to receive controls from fractalaw
8. ⬜ Trigger mechanism — request control generation for a law or customer register
9. ⬜ Baserow sync — populate Controls + Control Mappings templates from Postgres

### Blockers
- 244 SI laws have `art.` section_ids in our Postgres — sertantai has `reg.`. Old LAT from Feb 2026 before sertantai parser fix. Enrichment is on the `art.` rows. Need section_id rename migration (separate session).
- 172 laws reparsed (2026-07-13) — 95 now have mixed `art.`/`reg.` rows. `reg.` rows are bare (no enrichment). Need full pipeline on `reg.` rows then drop `art.` rows.
- Indexed provision approach (prompt v2) eliminates LLM hallucination but needs corpus-wide regeneration to replace old string-ref controls.

### Integration
10. ⬜ Test round-trip: sertantai requests → fractalaw generates → publishes → sertantai stores → visible in Baserow

## Dependencies

- ✅ Phase 3 QQ corpus complete (1,341 controls + 222 predicates in DuckDB staging)
- ✅ Baserow Controls template exists in sertantai
- ⬜ Sertantai Postgres controls table (items 6-7)
- ⬜ Sertantai trigger mechanism (item 8)
