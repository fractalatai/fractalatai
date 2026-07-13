---
session: Controls Publish
status: active
opened: 2026-07-13
---

# Session: Controls Publish (ACTIVE)

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

### Integration
10. ⬜ Test round-trip: sertantai requests → fractalaw generates → publishes → sertantai stores → visible in Baserow

## Dependencies

- ✅ Phase 3 QQ corpus complete (1,341 controls + 222 predicates in DuckDB staging)
- ✅ Baserow Controls template exists in sertantai
- ⬜ Sertantai Postgres controls table (items 6-7)
- ⬜ Sertantai trigger mechanism (item 8)
