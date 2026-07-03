---
session: Law Status Tracker
status: closed
opened: 2026-06-24
closed: 2026-06-25
outcome: success

summary: >
  Designed and implemented per-law pipeline status tracking. 6 timestamp columns
  on DuckDB legislation table derive pipeline stage. CLI command (taxa status) with
  --laws, --law-file, --summary, --stage filters. Zenoh queryable for sertantai
  to query status on demand.

decisions:
  - what: Timestamps on existing legislation table, not a separate status table
    why: Status is per-law metadata — adding columns is simpler than a new table with joins
    result: 6 timestamp columns, derived stage from NULL/non-NULL progression
  - what: "Fractalaw = engine, sertantai = customer record of which laws apply"
    why: Customer-law membership lives in sertantai. Fractalaw processes whatever arrives and exposes status.
    result: Separation of concerns confirmed by Gemini review
  - what: Both Zenoh queryable + proactive events
    why: Events for real-time dashboard reactivity, queryable for reliable current state
    result: Queryable on @tenant/fractalaw/status, events post-stage

metrics:
  timestamp_columns: 6
  pipeline_stages: 7

lessons:
  - title: Inferring status from published taxa is insufficient
    detail: "Published taxa only shows the endpoint. Laws stuck in early stages (needs_embed, needs_parse) are invisible without per-stage timestamps."
    tag: architecture

artifacts:
  - crates/fractalaw-cli/src/commands/taxa.rs
  - crates/fractalaw-cli/src/commands/sync.rs
  - crates/fractalaw-store/src/duck.rs

depends_on:
  - 06-24-26-pgstore-implementation.md
---

# Session: Law Status Tracker (CLOSED)

## Problem

We repeatedly run ad-hoc Python queries to determine where each law is in the pipeline. With 274 QQ laws across multiple stages (LAT pull → embed → parse → classify → validate → adjudicate → publish), it's hard to know at a glance what's done, what's pending, and what's blocked.

Every moving part — sync watch, manual parse, classify, validate, embed — updates different columns in LanceDB/DuckDB but there's no unified view of per-law pipeline status.

## Architecture

Fractalaw is a processing engine, not the customer system of record. Customer-law membership lives in sertantai. The two services compose at query time:

- **Sertantai**: "customer X has these 71 laws" (customer API)
- **Fractalaw**: "here's the pipeline status for these laws" (status API/command)

Fractalaw doesn't store customer associations. It processes whatever arrives via sync watch (monthly batch or real-time) and exposes per-law pipeline status on demand.

## Schema

Pipeline status columns on the existing `legislation` table:

```sql
ALTER TABLE legislation ADD COLUMN IF NOT EXISTS lat_pulled_at TIMESTAMPTZ;
ALTER TABLE legislation ADD COLUMN IF NOT EXISTS embedded_at TIMESTAMPTZ;
ALTER TABLE legislation ADD COLUMN IF NOT EXISTS parsed_at TIMESTAMPTZ;
ALTER TABLE legislation ADD COLUMN IF NOT EXISTS classified_at TIMESTAMPTZ;
ALTER TABLE legislation ADD COLUMN IF NOT EXISTS validated_at TIMESTAMPTZ;
ALTER TABLE legislation ADD COLUMN IF NOT EXISTS adjudicated_at TIMESTAMPTZ;
-- provisions_published_at already exists
```

Derived stage from timestamps:
```
published        → taxa_hash = published_hash
ready_to_publish → validated_at IS NOT NULL
needs_validate   → classified_at IS NOT NULL
needs_classify   → parsed_at IS NOT NULL  
needs_parse      → embedded_at IS NOT NULL
needs_embed      → lat_pulled_at IS NOT NULL
needs_lat        → otherwise
```

## CLI command

```bash
# Status for specific laws
fractalaw taxa status --laws UK_ukpga_1974_37,UK_uksi_1999_3242

# Status for a CSV law list (e.g. from sertantai export)
fractalaw taxa status --law-file data/qq-applicable-laws.csv

# Summary only
fractalaw taxa status --law-file data/qq-applicable-laws.csv --summary

# Filter by stage
fractalaw taxa status --laws <csv> --stage needs_validate
```

Output:
```
Pipeline Status (71 laws)
  published:          42
  ready_to_publish:    7
  needs_validate:      0
  needs_classify:      0
  needs_parse:         2
  needs_embed:         0
  needs_lat:          20

Laws needing LAT:
  UK_ukpga_1933_13
  UK_ukpga_1947_41
  ...
```

## Zenoh service exposure

### Queryable: status on demand
Expose `@{tenant}/fractalaw/status` as a Zenoh queryable. Sertantai sends a list of law names, fractalaw returns pipeline stage + timestamps for each. No CLI/SSH needed.

Request: JSON array of law names
Response: JSON array of `{law_name, stage, lat_pulled_at, embedded_at, parsed_at, classified_at, validated_at, adjudicated_at, published_at, provision_count}`

`provision_count` derived from Postgres at request time (not stored in DuckDB).

### Events: proactive status updates
After each pipeline stage completes, publish a lightweight event to `@{tenant}/fractalaw/status/{law_name}`:
```json
{"law_name": "UK_ukpga_1974_37", "stage": "classified", "timestamp": "2026-06-25T10:00:00Z"}
```
Sertantai can subscribe for real-time dashboard updates. Events complement the queryable — events for reactivity, queryable for reliability.

## Gemini review feedback (2026-06-25)

1. **Separation of concerns**: Confirmed correct. Fractalaw = engine, sertantai = customer record.
2. **Zenoh queryable**: Yes, implement it. Inferring status from published taxa is insufficient — doesn't show laws stuck in early stages.
3. **Proactive events + queryable**: Do both. Events for real-time, queryable for current state.
4. **Schema enrichment**: Gemini suggested per-stage error counts, status enums, durations. **Deferred** — `enrichment_retry_count` already exists, provision counts derived at query time. Premature to add per-stage error tracking now.
5. **Risks flagged**: Zenoh topic complexity (mitigate with docs), DuckDB query perf for large batches (not a concern at current scale), law_name consistency (already universal identifier).

## Implementation plan

1. Add status columns to DuckDB `legislation` table (idempotent ALTER TABLE)
2. Backfill timestamps from existing data (Postgres provision store has the ground truth)
3. Build `cmd_taxa_status` CLI command (local diagnostic)
4. Wire status timestamp updates into pipeline stages
5. Build Zenoh queryable handler for `@{tenant}/fractalaw/status`
6. Build Zenoh status event publisher (post-stage notifications)

## Key files

- `crates/fractalaw-cli/src/commands/taxa.rs` — new status command + pipeline stage updates
- `crates/fractalaw-cli/src/commands/sync.rs` — Zenoh queryable handler + event publisher
- `crates/fractalaw-store/src/duck.rs` — DuckDB schema migrations

## Integration points

Each pipeline stage updates its timestamp + publishes event:
- `sync watch` / `sync pull-lat` → sets `lat_pulled_at = now`, publishes status event
- `taxa embed` → sets `embedded_at = now`, publishes status event
- `taxa parse` → sets `parsed_at = now`, publishes status event
- `taxa classify` → sets `classified_at = now`, publishes status event
- `taxa validate` → sets `validated_at = now`, publishes status event
- `/human-review` apply → sets `adjudicated_at = now`, publishes status event
- `sync publish` → sets `provisions_published_at = now`, publishes status event
