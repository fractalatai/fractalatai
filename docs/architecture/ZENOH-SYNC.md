# Zenoh Sync — DRRP Taxa Enrichment Micro-Service

## Purpose

This is the **bi-directional bridge** between sertantai and fractalaw, implementing the Elixir-to-Fractalaw and Fractalaw-to-Elixir bridge patterns from the [micro-apps architecture](../.claude/plans/micro-apps.md) (§5.13, §5.14).

Sertantai scrapes, parses, and stores legislation but has no DRRP capability. Fractalaw owns the DRRP parser — the regex-based engine that extracts duties, rights, responsibilities, and powers from legislation text at the provision level. This micro-service connects the two: sertantai provides the raw text (LAT), fractalaw enriches it, and publishes the law-level taxa (LRT) back.

### What it produces

For every provision in a law, the parser extracts:

- **drrp_types** — Duty, Right, Responsibility, Power classifications
- **governed_actors** / **government_actors** — who holds the obligation
- **duty_family** / **duty_sub_type** — classification hierarchy
- **popimar** — People, Organisation, Plant, Information, Materials, Assets, Records
- **purposes** — what the provision is for (interpretation, enforcement, etc.)
- **clause_refined** — the core obligation text, stripped of qualifiers
- **taxa_confidence** — parser confidence score

### How it works

The service is event-driven. `fractalaw sync watch` subscribes to sertantai's change notifications and runs a pipeline for each law:

1. **Ensure LRT** — if the law's metadata doesn't exist locally, pull it from sertantai into DuckDB
2. **Pull LAT** — pull the full legislation text into LanceDB
3. **Enrich** — run the DRRP parser on every provision, write per-provision taxa to LanceDB and law-level aggregates to DuckDB
4. **Publish** — send the law-level taxa from DuckDB back to sertantai over zenoh

No manual steps, no polling. Sertantai persists a law, fractalaw enriches it, sertantai receives the results.

## Data Flow

```
sertantai                          fractalaw
    |                                  |
    |-- events/sync (JSON) ----------->|  notification: "lat persisted for UK_ukpga_1974_37"
    |                                  |
    |<-- query LRT (Arrow IPC) --------|  ensure law metadata in DuckDB
    |-- reply LRT -------------------->|
    |                                  |
    |<-- query LAT (Arrow IPC) --------|  pull legislation text
    |-- reply LAT -------------------->|  upsert into LanceDB
    |                                  |
    |                                  |  run DRRP parser on each provision
    |                                  |  write taxa to LanceDB + DuckDB
    |                                  |
    |<-- taxa/enrichment (Arrow IPC) --|  publish law-level taxa (from DuckDB)
    |                                  |
```

**Important**: LanceDB is inbound-only. It receives LAT and stores per-provision enrichment locally. Only DuckDB (LRT) data is published back to sertantai.

## Published Schema

The `taxa/enrichment/{law_name}` payload is a single Arrow IPC RecordBatch with one row (one law). Columns match §1.10 of the [legislation schema](../crates/fractalaw-core/src/schema.rs).

### Flat columns (`List<Utf8>`)

| Column | Description |
|--------|-------------|
| `name` | Law identifier (e.g. `UK_ukpga_1974_37`) |
| `duty_holder` | Actors with duties (aggregated across all provisions) |
| `rights_holder` | Actors with rights |
| `responsibility_holder` | Actors with responsibilities |
| `power_holder` | Actors with powers |
| `duty_type` | DRRP type classifiers (`Duty`, `Right`, `Responsibility`, `Power`) |
| `role` | All governed actors (semantic roles) |
| `role_gvt` | Government/authority actors |

### Struct columns (`List<Struct>`)

| Column | Description |
|--------|-------------|
| `duties` | Detailed duty entries |
| `rights` | Detailed right entries |
| `responsibilities` | Detailed responsibility entries |
| `powers` | Detailed power entries |

Each struct has four fields:

```
STRUCT(
  holder     VARCHAR,  -- e.g. "employer", "Secretary of State"
  duty_type  VARCHAR,  -- e.g. "DUTY", "RIGHT", "RESPONSIBILITY", "POWER"
  clause     VARCHAR,  -- extracted obligation text (truncated to ~200 chars)
  article    VARCHAR   -- citation reference, e.g. "section/2"
)
```

### Fitness columns (`List<Utf8>` + `List<Struct>`)

| Column | Description |
|--------|-------------|
| `fitness_person` | Person terms: `["employer", "self-employed person"]` |
| `fitness_process` | Process terms: `["construction work"]` |
| `fitness_place` | Place terms: `["Great Britain", "offshore"]` |
| `fitness_plant` | Plant terms: `["asbestos"]` |
| `fitness_property` | Property terms: `["at work"]` |
| `fitness_sector` | Sector terms: `["construction"]` |
| `fitness` | `List<FitnessEntry>` — full rules with polarity and article reference |

Canonical definition: [`drrp_entry_struct()`](../crates/fractalaw-core/src/schema.rs) (line 18).

## Enrichment States

The DRRP parser produces one of three outcomes per law. These determine what gets written to DuckDB, what gets published, and whether LAT (per-provision text) is pruned from LanceDB.

| State | DuckDB taxa written? | Published to sertantai? | LAT pruned? | Meaning |
|-------|---------------------|------------------------|-------------|---------|
| **Making** (`ok`) | Yes — all 19 taxa columns + `taxa_hash` | Yes — full 19-column Arrow IPC batch | No | Law creates at least one duty or responsibility. Full DRRP signal. |
| **NonMaking** (`non-making`) | Yes — all 19 taxa columns + `taxa_hash` | Yes — full 19-column Arrow IPC batch | Yes | Law has taxa metadata (rights, powers, fitness) but no duties or responsibilities. Taxa are published but per-provision text is pruned from LanceDB since it's not needed for duty tracking. |
| **NoTaxa** (`no taxa signal`) | No — nothing written to DuckDB | Yes — but all taxa columns are NULL/empty lists | Yes | Parser found nothing — no duties, rights, powers, fitness, or roles. The publish sends the row from DuckDB as-is (taxa columns remain NULL from initial LRT insert). LAT is pruned. Typical for procedural/administrative instruments with no substantive ESH content. |

### What sertantai receives per state

The published Arrow IPC payload always has the same 19-column schema (see Published Schema above). The difference is in the column values:

**Making / NonMaking**: Columns contain populated `List<Utf8>` and `List<Struct>` arrays with the extracted taxa. NonMaking laws will have empty `duties` and `responsibilities` lists but may have populated `rights`, `powers`, `fitness_*`, `role`, `role_gvt`.

**NoTaxa**: All list columns are NULL (not empty lists). Sertantai receives a valid Arrow batch with the law's `name` and 18 NULL columns. This distinguishes "we analysed this law and found nothing" from "we haven't analysed this law yet" (no publish at all).

## Re-enrichment Behaviour

When a previously enriched law is sent through the pipeline again (e.g. because sertantai re-persisted updated text), the enrichment uses content hashing to avoid redundant writes:

1. The parser runs on all provisions and produces a new taxa result.
2. A `taxa_hash` is computed from the 18 taxa column values (DRRP + fitness).
3. The new hash is compared against the existing `taxa_hash` in DuckDB.

| Scenario | DuckDB UPDATE? | Published? | `published_hash` updated? |
|----------|---------------|-----------|--------------------------|
| Hash unchanged (same taxa) | No — skipped | Yes — publishes existing DuckDB row | Yes |
| Hash changed (taxa differ) | Yes — all 19 columns overwritten | Yes — publishes updated row | Yes |
| Previously Making, now NoTaxa | Yes — clears all 18 taxa columns + `taxa_hash` to NULL | Yes — publishes NULLs (signals "analysed, found nothing") | Yes |

**Important**: the publish step always runs regardless of hash match. This means re-enrichment is idempotent from sertantai's perspective — it receives the same data shape whether the taxa changed or not. The `published_hash` is set to `taxa_hash` after each successful publish, so `--changed` filtering works correctly.

### Change tracking columns

| Column | Purpose |
|--------|---------|
| `taxa_hash` | SHA-256 of all 18 taxa column values. Set on enrichment. NULL if never enriched. |
| `published_hash` | Copied from `taxa_hash` after successful publish. NULL if never published. |

`sync publish --changed` selects laws where `taxa_hash IS NOT NULL AND (published_hash IS NULL OR taxa_hash != published_hash)`.

## Key Expressions

All keys are namespaced by tenant (default: `local`, set via `--tenant` or `FRACTALAW_TENANT`).

| Key | Direction | Format | Purpose |
|-----|-----------|--------|---------|
| `fractalaw/@{tenant}/events/sync` | sertantai → fractalaw | JSON | Change notification |
| `fractalaw/@{tenant}/data/legislation/lrt/{law_name}` | sertantai → fractalaw | Arrow IPC stream | Law metadata record (request/reply) |
| `fractalaw/@{tenant}/data/legislation/lat/{law_name}` | sertantai → fractalaw | Arrow IPC stream | Legislation text (request/reply) |
| `fractalaw/@{tenant}/taxa/enrichment/{law_name}` | fractalaw → sertantai | Arrow IPC stream | DRRP taxa results (pub/sub) |

## Event Payload

Sertantai publishes a JSON notification on `events/sync` whenever data changes:

```json
{
  "table": "lat",
  "action": "persist",
  "metadata": { "law_name": "UK_ukpga_1974_37", "count": 350 },
  "timestamp": "2026-02-27T12:00:00Z"
}
```

`table` is `lat`, `lrt`, `amendments`, etc. Fractalaw acts on `lat` and `lrt` events.

## CLI Commands

### `sync watch` — reactive pipeline (long-running)

Subscribes to `events/sync`. On any `lat` or `lrt` event, runs the full round-trip:

1. Ensure LRT exists in DuckDB — if missing, pull from sertantai and upsert
2. Pull LAT from sertantai (Arrow IPC query/reply) and upsert into LanceDB
3. Run DRRP taxa enrichment
4. Update DuckDB with law-level taxa
5. Publish taxa back to sertantai

```
fractalaw sync watch --tenant dev
```

Output:

```
Watching for sync events (tenant: dev, timeout: 30s per pull)...
Pipeline: ensure LRT → pull LAT → enrich → publish taxa
Press Ctrl+C to stop.

  UK_ukpga_1974_37: pull LRT → 1 row(s) → pull LAT → 350 provisions → enrich → ok → publish → done
  UK_uksi_2020_1647: → pull LAT → 214 provisions → enrich → ok → publish → done
^C
Done. 2 events, 1 LRT pulls, 2 LAT pulls (564 provisions), 2 enriched, 2 published.
```

Options:

- `--tenant <name>` — tenant namespace (default: `local`, env: `FRACTALAW_TENANT`)
- `--timeout <secs>` — per-query timeout (default: 30)

### `sync pull-lat` — one-shot LAT pull

Pull specific laws without waiting for events.

```
fractalaw sync pull-lat --laws UK_ukpga_1974_37,UK_uksi_2004_1309 --tenant dev
```

### `sync publish` — push taxa to sertantai

Publish DRRP taxa enrichment for laws that already have data in DuckDB.

```
# Single law
fractalaw sync publish --laws UK_ukpga_1974_37 --tenant dev

# All laws in a family
fractalaw sync publish --family "OH&S: Occupational / Personal Safety" --tenant dev

# Everything
fractalaw sync publish --all --tenant dev
```

### `sync crdt` — CRDT document management

```
fractalaw sync crdt status --tenant dev
fractalaw sync crdt create <doc_id> --tenant dev
fractalaw sync crdt inspect <doc_id> --tenant dev
fractalaw sync crdt save --tenant dev
```

## Typical Workflow

**First time** — pull existing laws manually:

```
fractalaw sync pull-lat --laws UK_ukpga_1974_37 --tenant dev
fractalaw taxa enrich --laws UK_ukpga_1974_37
fractalaw sync publish --laws UK_ukpga_1974_37 --tenant dev
```

**Ongoing** — leave the watcher running:

```
fractalaw sync watch --tenant dev
```

Sertantai persists a law → fires event → fractalaw pulls, enriches, publishes back. No manual steps.
