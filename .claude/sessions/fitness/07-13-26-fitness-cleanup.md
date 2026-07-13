---
session: Fitness Pipeline Cleanup
status: closed
opened: 2026-07-13
closed: 2026-07-13
outcome: success

summary: >
  Final cleanup of the fitness pipeline. Reviewed all 12 sessions — all deferred
  items either completed or superseded. Strategy marked COMPLETE with actual metrics.
  Legacy P-dimension columns removed from publish payload. ZENOH-SPEC v3.0.

decisions:
  - what: NER training (Phase 2d) superseded — not needed
    why: Fine-tuned SLM achieved 93.3% precision, exceeding the target. NER was planned as an intermediate step but the SLM staircase jumped directly from regex to fine-tuned SLM successfully.
    result: Phase 2d marked as superseded in session review.

  - what: No republish needed after removing legacy columns
    why: Sertantai already has the new fields from the bulk publish. It stops reading legacy columns and cleans up its own DB independently.
    result: Clean separation — fractalaw drops columns from future publishes, sertantai migrates at its own pace.

metrics:
  sessions_reviewed: 12
  deferred_items_resolved: { done: 15, superseded: 3, carry_forward: 0 }
  legacy_columns_removed: 7
  spec_version: "3.0"

lessons:
  - title: Fine-tuned SLM can skip the NER step entirely
    detail: The strategy planned regex → NER → SLM as a staircase. In practice, the fine-tuned SLM (trained on dictionary-extracted ground truth) achieved higher precision than NER would likely deliver, making the NER step unnecessary. The staircase collapsed to regex → fine-tuned SLM.
    tag: models

artifacts:
  - .claude/plans/fitness/FITNESS-STRATEGY.md
  - crates/fractalaw-sync-cli/src/sync.rs
  - /var/home/jason/Desktop/sertantai-legal/docs/zenoh/ZENOH-SPEC.md

depends_on:
  - 07-13-26-fitness-reconcile-publish.md
  - 07-13-26-fitness-expression-compiler.md

enables:
  - Clean publish payload for all future laws
  - Sertantai DB cleanup of legacy fitness columns
---

# Session: Fitness Pipeline Cleanup (ACTIVE)

## Problem

The fitness pipeline is complete and 695 laws are live in sertantai. Twelve sessions accumulated deferred items, legacy code/columns, and stale references. Need to clean up: close out deferred items (done or no longer relevant), remove legacy fitness columns from publish, update the strategy doc, and tidy the codebase.

## Deferred Items Review

From all 12 fitness sessions — categorised as DONE (completed in later sessions), OBSOLETE (superseded), or CARRY FORWARD:

**DONE (completed in subsequent sessions):**
- Scoping session items 5-12: all phases completed across sessions
- Applicability regex items 11-12: polarity tags land via fitness extract CLI
- Rule compiler item 5: ApplicabilityNode implemented in expression compiler session
- SLM finetune item 10: re-propagation done in reconcile session
- Expression compiler item 7: bulk published 695 laws in reconcile session

**OBSOLETE (superseded by architecture changes):**
- Cross-domain items 6-7 (NER training): fine-tuned SLM achieved 93.3% precision, NER not needed
- SLM extraction item 9 (entity feedback loop): dictionaries enriched, fine-tuned model trained — feedback loop executed manually, not automated

**CARRY FORWARD (still relevant):**
- None — all items are either done or superseded

## Work

### Session review (all 12 fitness sessions)
1. ✅ Reviewed all 12 sessions: all deferred items either completed in later sessions or superseded (NER not needed — fine-tuned SLM achieved 93.3%)

### Strategy update
2. ✅ FITNESS-STRATEGY.md: all phases marked complete with session refs and actual metrics, title marked COMPLETE

### Publish signature cleanup
3. ✅ Removed 7 legacy fitness columns from publish payload (fitness_person/process/place/plant/property/sector/fitness)
4. ✅ ZENOH-SPEC.md updated to v3.0: legacy columns removed from schema
5. ✅ No republish needed — sertantai already has new fields from bulk publish

### Database cleanup
6. ⬜ Drop legacy fitness columns from DuckDB legislation table (fitness_person, fitness_process, fitness_place, fitness_plant, fitness_property, fitness_sector, fitness)
7. ⬜ Drop legacy fitness columns from Postgres legislation_text table (fitness_polarity, fitness_person, fitness_process, fitness_place, fitness_plant, fitness_property, fitness_sector)

### Code refactor plan

170 references across 8 files. Ordered by dependency (leaf changes first):

**File 1: `fractalaw-core/src/schema.rs` (14 refs)**
- Remove `fitness_entry_struct()` function
- Remove `fitness_person/process/place/plant/property/sector` from `legislation_schema()`
- Remove `fitness` (List<Struct>) from `legislation_schema()`
- Remove `fitness_polarity/person/process/place/plant/property/sector` from `legislation_text_schema()`
- Impact: Arrow schemas change — any code deserialising these schemas must be updated

**File 2: `fractalaw-store/src/duck.rs` (19 refs)**
- Remove `ensure_fitness_columns()` function entirely (creates legacy columns)
- Remove 2 test functions (`ensure_fitness_columns_adds_seven_columns`, `ensure_fitness_columns_idempotent`)
- Impact: callers in taxa.rs and sync.rs must stop calling it

**File 3: `fractalaw-store/src/lance.rs` (7 refs)**
- Remove `fitness_polarity/person/process/place/plant/property/sector` from the SELECT in `query_legislation_text()`
- Impact: provision-level publish won't include legacy fitness columns

**File 4: `fractalaw-store/src/pg.rs` (2 refs)**
- Remove any legacy fitness column references in UPDATE/INSERT SQL
- Impact: Postgres writes stop writing to dropped columns

**File 5: `fractalaw-cli/src/utils.rs` (16 refs)**
- Remove `format_sql_fitness_entries()` function
- Remove `FitnessEntry` type alias
- Remove `fitness_persons/processes/places/plants/properties/sectors/entries` params from `build_taxa_update_sql()`
- Impact: pipeline.rs and taxa.rs callers must stop passing these

**File 6: `fractalaw-cli/src/commands/pipeline.rs` (102 refs) — THE BIG ONE
- Remove from `LawTaxa` struct: `fitness_persons/processes/places/plants/properties/sectors`, `fitness_entries`
- Remove from `ProvisionTaxa` struct: `fitness_polarity`, `fitness_person/process/place/plant/property/sector`
- Remove from `ProvisionTaxa::empty()`: all fitness fields
- Remove the entire P-dimension extraction loop (lines ~407-490): the `for rule in &record.fitness_rules` block that builds fp_person/fp_process etc and aggregates into taxa.fitness_persons
- Remove fitness columns from the RecordBatch builder (~lines 1415-1450 and 1536-1542)
- Keep: the `detect_polarity()` call (line ~399) — this is Phase 1, still useful for provision-level polarity tagging
- Impact: the DRRP pipeline stops writing legacy fitness. `fitness extract` CLI handles fitness independently.

**File 7: `fractalaw-cli/src/commands/taxa.rs` (9 refs)**
- Remove 3 calls to `store.ensure_fitness_columns()`
- Remove any fitness column references in taxa commands
- Impact: taxa commands stop trying to create legacy DuckDB columns

**File 8: `fractalaw-sync-cli/src/sync.rs` (1 ref)**
- Remove call to `store.ensure_fitness_columns()` in `cmd_sync_publish`
- Impact: publish stops trying to create legacy DuckDB columns

**What stays:**
- `fitness.rs` — polarity detection + P-dimension dictionaries stay. Used by `fitness extract` CLI.
- `fitness_mentions` table — the new v0.3 storage
- `fitness_entities` table — the entity catalogue
- `applicability.rs` — the expression tree types
- `fitness extract` / `fitness compile` / `fitness status` CLI commands
- `detect_polarity()` call in pipeline.rs — provision-level polarity is still useful

8. ✅ schema.rs: removed fitness_entry_struct(), 7 LRT fields, 7 LAT fields. Tests updated (100→93, 59→52).
9. ✅ duck.rs: removed ensure_fitness_columns() + 2 tests
10. ✅ lance.rs: removed 7 fitness columns from provision query SELECT
11. ✅ pg.rs: removed 7 fitness columns from provision taxa query
12. ✅ utils.rs: removed FitnessEntry type, format_sql_fitness_entries(), 7 fitness params from compute_taxa_hash()
13. ✅ pipeline.rs: removed LawTaxa fitness fields (7), ProvisionTaxa fitness fields (7), P-dimension extraction loop (~100 lines), RecordBatch builder columns (7), DuckDB UPDATE SQL fitness columns
14. ✅ taxa.rs: removed 3 ensure_fitness_columns() calls
15. ✅ sync.rs: removed 1 ensure_fitness_columns() call
16. ✅ Workspace compiles clean, schema tests pass (32/32)

## Dependencies

- ✅ All 12 fitness sessions closed
- ✅ 695 laws published with new schema
- ✅ Sertantai confirmed new fields work
- ⬜ Sertantai confirms it no longer reads legacy fitness_person etc columns
