---
session: Denormalize Fitness/Scope onto LRT (#7)
status: closed
opened: 2026-03-05
closed: 2026-03-05
outcome: success
summary: 'Shipped fitness extraction data from in-memory-only to both LanceDB (7 per-provision columns) and DuckDB (6 tag
  columns + 1 FitnessEntry detail column). Wired fitness into compute_taxa_hash() and sync publish payload (12 to 19 columns).
  Schema grew from 91 to 98 LRT columns and 28 to 35 LAT columns.

  '
decisions:
- what: Tag columns union both polarities for filtering
  why: A law exempting self-employed workers is still relevant when filtering for self-employed laws
  result: 6 tag columns answer "is this concept mentioned?" while polarity context lives in the detail column
- what: FitnessEntry struct fields as scalar Utf8 with comma-separated multi-values
  why: Avoids nested List<List<Utf8>> which is painful in DuckDB and impossible in Airtable
  result: Matches DRRPEntry pattern, practical for rendering in sertantai UI
- what: Follow established DRRP two-tier pattern for schema
  why: Tag columns for filtering/grouping + detail column for UI rendering and hot-path matching
  result: Consistent with duty_holder/duties pattern, maps to multi-select facets in Baserow/Airtable
lessons:
- title: Computed data silently discarded when not persisted
  detail: fitness.rs was fully implemented and wired into parse_v2() but enrich_single_law() never read record.fitness_rules,
    silently dropping computed values
  tag: pipeline-wiring
metrics:
  lat_columns_before: 28
  lat_columns_after: 35
  lrt_columns_before: 91
  lrt_columns_after: 98
  publish_columns_before: 12
  publish_columns_after: 19
  tests_passing: 494
artifacts:
- crates/fractalaw-core/src/taxa/fitness.rs
- crates/fractalaw-core/src/taxa/mod.rs
- crates/fractalaw-core/src/schema.rs
- crates/fractalaw-store/src/duck.rs
- crates/fractalaw-cli/src/main.rs
- docs/SCHEMA.md
depends_on: []
enables: []
---


# Session: Denormalize Fitness/Scope onto LRT (#7) (CLOSED)

**Date**: 2026-03-05
**Issue**: [#7 — Denormalize fitness/scope and penalty provisions onto LRT hot path](https://github.com/fractalaw/fractalaw/issues/7)
**Objective**: Ship the fitness extraction data (currently computed in-memory and discarded) onto the DuckDB LRT table so it's available on the hot path and can be published to sertantai.
**Priority context**: See [priority-reviews.md](../../plans/priority-reviews.md) — rose to #1 after #15 (QA report) closed.

## Problem

The fitness extraction pipeline (`fractalaw-core/src/taxa/fitness.rs`) is fully implemented and wired into `parse_v2()`. It runs on Application+Scope provisions, producing structured `FitnessRule`s with polarity (AppliesTo/DisappliesTo/ExtendsTo) and p-dimension tags (Person, Process, Place, Plant, Property, Sector). But `enrich_single_law()` never reads `record.fitness_rules` — the computed values are silently discarded after each provision loop iteration.

### Current state

| Layer | Fitness data? | Notes |
|-------|--------------|-------|
| `fractalaw-core/src/taxa/fitness.rs` | Full implementation | `FitnessRule`, `RulePolarity`, `PDimension`, `extract()`, 23 tests |
| `TaxaRecord.fitness_rules` | Populated | Set during `parse_v2()` for APPLICATION_SCOPE provisions |
| LanceDB `legislation_text` | **Not stored** | Fitness rules dropped after parse — no per-provision master data |
| DuckDB `legislation` | **Not stored** | No fitness columns in schema |
| `sync publish` | **Not published** | Only 12 DRRP columns go to zenoh |

### What needs to happen

**LAT (LanceDB — per-provision):**
1. Add fitness columns to LAT schema for per-provision storage
2. Collect fitness rules in `enrich_single_law()` and write per-provision to LanceDB (alongside existing taxa columns like `purposes`, `drrp_types`, etc.)
3. This is the master data — needed for AI polishing, QA validation, and future provision-level matching

**LRT (DuckDB — per-law aggregated):**
4. Add fitness columns to LRT schema (`schema.rs`, `duck.rs`)
5. Aggregate per-law fitness from provision-level rules into law-level summary
6. Write aggregated fitness to DuckDB UPDATE in `enrich_single_law()`
7. Include fitness in `compute_taxa_hash()`

**Publish:**
8. Include fitness in `sync publish` payload
9. Coordinate schema extension with sertantai (Elixir/PostgreSQL side)

**Docs:**
10. Update `docs/SCHEMA.md` with fitness sections (LAT 3.x + LRT 1.x)

## Key Files

- `crates/fractalaw-core/src/taxa/fitness.rs` — `FitnessRule`, `RulePolarity`, `PDimension`, `extract()`
- `crates/fractalaw-core/src/taxa/mod.rs` — `TaxaRecord.fitness_rules`, `parse_v2()`
- `crates/fractalaw-core/src/schema.rs` — `legislation_schema()` (currently 91 columns)
- `crates/fractalaw-store/src/duck.rs` — `ensure_taxa_hash_columns()` pattern for idempotent column adds
- `crates/fractalaw-cli/src/main.rs` — `enrich_single_law()`, `compute_taxa_hash()`, `cmd_sync_publish()`
- `docs/SCHEMA.md` — Schema design document (v0.6, needs fitness section)

## Schema Recommendation

Follows the established DRRP two-tier pattern: **tag columns** (List\<Utf8\>) for filtering/grouping/Airtable sync + **detail column** (List\<Struct\>) for UI rendering and hot-path matching.

### Use cases driving the design

1. **Sertantai UI** — users browse laws, filter by "who does this apply to?", sort by sector, see at-a-glance applicability. Tag columns map to multi-select facets. Detail column renders as structured cards.
2. **No-code DB sync** (Baserow, Airtable) — tag columns map directly to multi-select fields. Detail column syncs as a JSON text field or linked records. Flat List\<Utf8\> is ideal.
3. **Hot-path micro-apps** — fitness matching evaluates user profile against tag columns (fast set intersection). Detail column provides full context for the match result.

### LAT columns (LanceDB — per-provision, 7 new columns)

Flat tags per provision, consistent with existing `purposes`, `drrp_types`, `governed_actors` pattern. Compound provisions (which produce 2 FitnessRules with opposite polarities) union their tags into the same lists — polarity carries both values.

| Column | Arrow Type | Description |
|--------|-----------|-------------|
| `fitness_polarity` | List\<Utf8\> | `["applies_to"]`, `["disapplies_to"]`, or `["applies_to", "disapplies_to"]` for compounds |
| `fitness_person` | List\<Utf8\> | Person terms: `["employer", "self-employed person"]` |
| `fitness_process` | List\<Utf8\> | Process terms: `["construction work"]` |
| `fitness_place` | List\<Utf8\> | Place terms: `["Great Britain", "offshore"]` |
| `fitness_plant` | List\<Utf8\> | Plant terms: `["asbestos"]` |
| `fitness_property` | List\<Utf8\> | Property terms: `["at work"]` |
| `fitness_sector` | List\<Utf8\> | Sector terms: `["construction"]` |

Empty lists for non-Application+Scope provisions (same pattern as `drrp_types` being empty for non-DRRP provisions).

### LRT columns (DuckDB — per-law aggregated, 7 new columns)

**6 tag columns** — union across all provisions, both polarities. Answer "does this law mention X at all?" for filtering. Exactly parallel to `duty_holder`, `rights_holder`, etc.

| Column | Arrow Type | Description | Airtable/Baserow |
|--------|-----------|-------------|-----------------|
| `fitness_person` | List\<Utf8\> | All person terms across all rules | Multi-select |
| `fitness_process` | List\<Utf8\> | All process terms | Multi-select |
| `fitness_place` | List\<Utf8\> | All place terms | Multi-select |
| `fitness_plant` | List\<Utf8\> | All plant terms | Multi-select |
| `fitness_property` | List\<Utf8\> | All property terms | Multi-select |
| `fitness_sector` | List\<Utf8\> | All sector terms | Multi-select |

**Why union both polarities in tag columns?** Same reasoning as `duty_holder` containing all holder types regardless of duty_type. Tags answer "is this concept mentioned?" — a law that exempts self-employed workers is still relevant when filtering for self-employed laws. Polarity context lives in the detail column.

**1 detail column** — full rules with polarity and article reference, for UI rendering and programmatic matching. Parallel to `duties`, `rights`, `responsibilities`, `powers` (List\<DRRPEntry\>).

| Column | Arrow Type | Description |
|--------|-----------|-------------|
| `fitness` | List\<FitnessEntry\> | All fitness rules for this law |

**FitnessEntry struct** — all fields scalar Utf8, matching DRRPEntry pattern. Multi-value p-dimensions are comma-separated within the string (practical for rendering; avoids nested List\<List\<Utf8\>\> which is painful in DuckDB and impossible in Airtable).

| Struct Field | Arrow Type | Description |
|-------------|-----------|-------------|
| `polarity` | Utf8 | `"applies_to"`, `"disapplies_to"`, `"extends_to"` |
| `person` | Utf8 | Comma-separated: `"employer, worker"` or null |
| `process` | Utf8 | `"construction work"` or null |
| `place` | Utf8 | `"Great Britain"` or null |
| `plant` | Utf8 | `"asbestos"` or null |
| `property` | Utf8 | `"at work"` or null |
| `sector` | Utf8 | `"construction"` or null |
| `article` | Utf8 | Source provision: `"regulation/2"` |

### Aggregation logic (in `enrich_single_law()`)

Mirrors DRRP pattern with `LawTaxa`:

```rust
// New fields on LawTaxa struct:
fitness_persons:    BTreeSet<String>,  // union of all person tags
fitness_processes:  BTreeSet<String>,
fitness_places:     BTreeSet<String>,
fitness_plants:     BTreeSet<String>,
fitness_properties: BTreeSet<String>,
fitness_sectors:    BTreeSet<String>,
fitness_entries:    Vec<(String, String, String, String, String, String, String, String)>,
//                      polarity, person, process, place, plant, property, sector, article
```

Per provision, when `record.fitness_rules` is non-empty:
- For each `FitnessRule`, insert all tags into the corresponding BTreeSet
- Push a tuple into `fitness_entries` with comma-joined tags per dimension

### Hash and publish scope

- `compute_taxa_hash()`: add the 6 fitness BTreeSets + sorted fitness_entries after the existing 11 DRRP fields
- `cmd_sync_publish()` SELECT: add 7 fitness columns to the 12 existing → 19 columns total

### Column count impact

- LAT: 28 → 35 columns (+7)
- LRT: 91 → 98 columns (+7)

### Example rendering (sertantai UI)

```
━━━ Fitness / Applicability ━━━
✓ Applies to: employer, self-employed person
  Process: construction work
  Place: Great Britain
  Sector: construction
  Source: regulation/2

✗ Does not apply to: master, crew
  Place: sea-going ship
  Source: regulation/3(2)
```

## Progress

### LAT (LanceDB — per-provision master data)
- [x] Design LAT fitness columns (per-provision shape) — 7 `List<Utf8>` columns in `legislation_text_schema()` §3.10
- [x] Add fitness columns to LAT taxa batch in `enrich_single_law()` — 7 list builders, per-provision flat tags
- [x] Write per-provision fitness to LanceDB `update_taxa()` — included in taxa_batch RecordBatch

### LRT (DuckDB — per-law aggregated)
- [x] Design LRT fitness columns (law-level aggregation shape) — 6 tag `List<Utf8>` + 1 `List<FitnessEntry>` detail
- [x] Add fitness columns to `schema.rs` — §1.10c, `fitness_entry_struct()` helper, 98 total columns
- [x] Add `ensure_fitness_columns()` to `duck.rs` — idempotent ALTER TABLE, 2 tests
- [x] Aggregate fitness rules per-law in `enrich_single_law()` — 6 BTreeSets + fitness_entries Vec
- [x] Include fitness in DuckDB UPDATE — 7 columns in UPDATE SET, `format_sql_fitness_entries()` helper
- [x] Include fitness in `compute_taxa_hash()` — 7 new params (6 BTreeSets + entries slice)

### Publish + Docs
- [x] Include fitness in `cmd_sync_publish()` SELECT + publish payload — 12→19 columns
- [x] Update `docs/SCHEMA.md` with fitness sections (LAT §3.9 + LRT §1.9a + FitnessEntry struct)
- [x] Tests — all 494 workspace tests pass, updated `taxa_hash_deterministic` + `taxa_hash_changes_on_different_input`
- [x] Coordinate sertantai schema extension — [sertantai-legal#39](https://github.com/shotleybuilder/sertantai-legal/issues/39) created with full schema detail

## Status: **Closed**
