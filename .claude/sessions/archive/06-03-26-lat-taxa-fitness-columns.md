---
session: "Taxa & Fitness on LAT Records"
status: closed
opened: 2026-06-03
closed: 2026-06-03
outcome: success

summary: >
  Designed and implemented provision-level taxa/fitness publishing from LanceDB to sertantai via Zenoh. Provision-level DRRP data already existed in LanceDB but was never published -- only law-level aggregates went to sertantai. Added publish_provision_taxa() to zenoh_sync, LanceDB query projection for 18 taxa columns, and a --provisions flag on sync publish.

decisions:
  - what: "Publish provision-level taxa directly from LanceDB, not mirrored through DuckDB"
    why: "Data already exists in LanceDB with the right columns; mirroring 97K rows into DuckDB just to publish adds complexity for no analytical benefit"
    result: "Controlled exception to the \"LanceDB never publishes\" rule -- publishing classification metadata only, not raw text or embeddings"
  - what: "Batch by law for zenoh messages"
    why: "All provisions for one law as a unit is cleanest for the subscriber; matches existing law-level pattern"
    result: "One Arrow IPC message per law on topic fractalaw/@{tenant}/taxa/provisions/{law_name}"
  - what: "Change tracking via taxa_classified_at vs provisions_published_at"
    why: "Need incremental delta publishing for 97K rows; same pattern as law-level taxa_hash/published_hash"
    result: "provisions_published_at column added to DuckDB legislation table (field count 99 to 100)"

lessons:
  - title: "Provision-level data unlocks filtering that law-level aggregates cannot"
    detail: "Sertantai cannot filter by actor or DRRP type at the provision level with law-level data only. Each provision carrying its own taxa enables queries like \"show duties on employers in this law\"."
    tag: architecture

artifacts:
  - crates/fractalaw-sync/src/zenoh_sync.rs
  - crates/fractalaw-store/src/lance.rs
  - crates/fractalaw-store/src/duck.rs
  - crates/fractalaw-core/src/schema.rs
  - crates/fractalaw-cli/src/main.rs

depends_on:
  - 02-20-26-phase2-lancedb-embeddings.md

enables:
  - 06-05-26-eu-retained-law-support.md
---

# Session: 2026-06-03 — Taxa & Fitness on LAT Records (CLOSED)

## Context

**Issue**: None (research session)
**Objective**: Design how to capture Taxa (DRRP) and Fitness classification data on the Legal Article Table (LAT) provision-level records, so that sertantai can filter and serve the provisions that contain the key parts of a law — duties, powers, rights, responsibilities, and applicability scope.

## Problem

Currently the taxa pipeline aggregates provision-level DRRP extractions **up** to the law level in DuckDB, then publishes law-level records to sertantai via zenoh. Sertantai receives a single record per law with aggregated `duty_holder`, `rights_holder`, `duties: List<Struct>`, `fitness: List<Struct>`, etc.

This means sertantai **cannot filter at the provision level**. If a user wants "show me the provisions that impose duties on employers", sertantai has to either:
1. Return the entire law and let the client pick through it, or
2. Unpack the `duties` struct list and match the `article` field back to provisions — a fragile, indirect lookup.

What we actually need is for **each LAT row** (provision) to carry its own taxa and fitness classification directly, so sertantai can query and serve individual provisions by DRRP type, actor, fitness dimension, etc.

## Current State

### What exists per provision (LanceDB `legislation_text`)

Already populated by the enrich pipeline (`parse_v2()` writes back to LanceDB):

| Column | Type | Status |
|--------|------|--------|
| `drrp_types` | List<Utf8> | Populated during enrich |
| `governed_actors` | List<Utf8> | Populated during enrich |
| `government_actors` | List<Utf8> | Populated during enrich |
| `duty_family` | Utf8 | Populated during enrich |
| `duty_sub_type` | Utf8 | Populated during enrich |
| `popimar` | List<Utf8> | Populated during enrich |
| `purposes` | List<Utf8> | Populated during enrich |
| `clause_refined` | Utf8 | Populated during enrich |
| `taxa_confidence` | Float32 | Populated during enrich |
| `fitness_polarity` | List<Utf8> | Populated during enrich |
| `fitness_person` | List<Utf8> | Populated during enrich |
| `fitness_process` | List<Utf8> | Populated during enrich |
| `fitness_place` | List<Utf8> | Populated during enrich |
| `fitness_plant` | List<Utf8> | Populated during enrich |
| `fitness_property` | List<Utf8> | Populated during enrich |
| `fitness_sector` | List<Utf8> | Populated during enrich |

### What gets published to sertantai

Only **law-level** aggregates from DuckDB's `legislation` table (18 columns):
- `name`, `duty_holder`, `rights_holder`, `responsibility_holder`, `power_holder`
- `duty_type`, `role`, `role_gvt`
- `duties`, `rights`, `responsibilities`, `powers` (List<Struct> with article back-refs)
- `fitness_person/process/place/plant/property/sector`, `fitness`

Published via zenoh key: `fractalaw/@{tenant}/taxa/enrichment/{law_name}`

### The gap

The provision-level taxa data **already exists in LanceDB** but is never published to sertantai. Sertantai only gets the law-level roll-up.

## Research Answers (from sertantai-legal Claude + Jason)

### 1. Sertantai schema — zero taxa columns on LAT

The `legal_articles` / `lat` view currently has: `section_id`, `law_name`, `law_id`, `sort_key`, `position`, `section_type`, `hierarchy_path`, `depth`, `part`, `chapter`, `heading_group`, `provision`, `paragraph`, `sub_paragraph`, `schedule`, `text`, `language`, `extent_code`, annotation counts (amendment/modification/commencement/extent/editorial), `embedding` + model, `token_ids` + model, `legacy_id`, timestamps. **No DRRP or fitness columns at all.** Taxa currently lives only on `uk_lrt` (law level).

### 2. Filtering requirements — provision-level by actor, DRRP type, fitness

Customer use case: *"show me the sections of this law that create duties for my organisation type."* Baserow sync needs to show relevant provisions, not the whole law.

Required filters:
- **By actor**: "provisions involving Org: Employer"
- **By DRRP type**: "provisions that are Duty vs Power vs Right"
- **By fitness dimensions**: sector, place, plant, process
- **Group by**: provision → duties within it

### 3. Ingest — Zenoh, law-level only today

Sertantai-legal publishes LRT/LAT/AmendmentAnnotation queryables; fractalaw subscribes and returns taxa enrichment at law level. **No provision-level ingest path yet.** The pattern would be the same (zenoh queryable/subscriber) but with a new topic for provision-level DRRP.

### 4. Volume — 97K is fine, batch per law

Full snapshots for initial load, deltas for updates — same pattern as law-level taxa. **Batched by law** (all provisions for one law as a unit) is cleanest for the subscriber.

### 5. Embeddings — metadata only

Classification metadata only. Sertantai already has its own 384-dim embedding column on LAT. The value from fractalaw is the structured DRRP classification, not the vectors.

---

## Design

### Publish payload per provision

New zenoh topic: `fractalaw/@{tenant}/taxa/provisions/{law_name}`

Columns to publish (keyed by `section_id`):

| Column | Type | Purpose |
|--------|------|---------|
| `section_id` | Utf8 | FK to sertantai LAT row |
| `drrp_types` | List<Utf8> | Duty/Right/Responsibility/Power |
| `governed_actors` | List<Utf8> | Who the provision regulates |
| `government_actors` | List<Utf8> | Government actors responsible |
| `duty_family` | Utf8 | Duty family classification |
| `duty_sub_type` | Utf8 | Duty sub-type |
| `clause_refined` | Utf8 | "who must do what" extraction |
| `purposes` | List<Utf8> | Purpose/function classifications |
| `popimar` | List<Utf8> | Management system categories |
| `taxa_confidence` | Float32 | Classification confidence (0-1) |
| `fitness_polarity` | List<Utf8> | AppliesTo/DisappliesTo/ExtendsTo |
| `fitness_person` | List<Utf8> | Person dimension |
| `fitness_process` | List<Utf8> | Process dimension |
| `fitness_place` | List<Utf8> | Place dimension |
| `fitness_plant` | List<Utf8> | Plant/equipment dimension |
| `fitness_property` | List<Utf8> | Property condition |
| `fitness_sector` | List<Utf8> | Sector/industry |

### Architecture decision: publish from LanceDB directly

The existing pattern is LanceDB → DuckDB → zenoh. But for provision-level taxa:
- The data already exists in LanceDB with the right columns
- Mirroring 97K rows into DuckDB just to publish them adds complexity and storage for no analytical benefit
- DuckDB is for law-level metadata (LRT); LanceDB is for provision-level data (LAT)

**Decision: read from LanceDB, serialize to Arrow IPC, publish via zenoh.** This is a controlled exception to the "LanceDB never publishes" rule — we're publishing classification metadata, not raw LAT text or embeddings.

### Incremental change tracking

Use `taxa_classified_at` timestamp on LanceDB provisions:
- Track last publish time per law (in DuckDB, e.g. `provisions_published_at` column on `legislation`)
- On publish, select provisions where `taxa_classified_at > provisions_published_at`
- After successful publish, update `provisions_published_at`

### Batching

- One zenoh message per law containing all enriched provisions for that law
- Same Arrow IPC format as law-level publish
- `sync publish-provisions` command (or `--provisions` flag on existing `sync publish`)

## Implementation Plan

### Fractalaw side (this repo)

1. **Add `provisions_published_at` column** to DuckDB `legislation` table schema
2. **Add LanceDB query function** — read provision-level taxa columns for a given law, filtered by `taxa_classified_at`
3. **Add zenoh publish function** — `publish_provision_taxa()` serializes and sends per-law batches
4. **Add CLI command** — `sync publish --provisions` or `sync publish-provisions`
5. **Wire up change tracking** — compare `taxa_classified_at` vs `provisions_published_at`

### Sertantai side (their repo)

1. Add taxa/fitness columns to `legal_articles` table (ALTER TABLE)
2. New zenoh subscriber for `taxa/provisions/{law_name}` topic
3. Upsert by `section_id`
4. Expose taxa columns in the `lat` view and Baserow sync

## Progress

### Fractalaw implementation — DONE

All fractalaw-side changes implemented and tested:

- [x] `zenoh_sync.rs` — `keys::taxa_provisions()`, `keys::taxa_provisions_wildcard()`, `ZenohSync::publish_provision_taxa()` + 3 new tests (all passing)
- [x] `lance.rs` — `LanceStore::query_provision_taxa()` with `Select::Columns` projection (18 taxa/fitness columns, skips text/embeddings)
- [x] `duck.rs` — `DuckStore::ensure_provisions_published_column()` adds `provisions_published_at TIMESTAMP`
- [x] `schema.rs` — `provisions_published_at` added to `legislation_schema()` (field count 99→100)
- [x] `main.rs` — `--provisions` flag on `SyncAction::Publish`, `cmd_sync_publish_provisions()` function

Usage: `fractalaw sync publish --provisions --laws UK_ukpga_1974_37 --tenant dev`

### Remaining — sertantai side

- [ ] Add taxa/fitness columns to `legal_articles` table (ALTER TABLE)
- [ ] New zenoh subscriber for `taxa/provisions/{law_name}` topic
- [ ] Upsert by `section_id`
- [ ] Expose taxa columns in the `lat` view and Baserow sync
- [ ] End-to-end test: enrich → publish provisions → query in sertantai
