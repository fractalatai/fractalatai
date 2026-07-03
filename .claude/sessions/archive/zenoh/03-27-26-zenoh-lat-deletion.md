---
session: Zenoh LAT Deletion Signal
status: closed
opened: 2026-03-27
closed: 2026-03-27
outcome: success

summary: >
  Implemented handling for lat_deleted change notifications from sertantai. When
  admin deletes LAT data for a law, fractalaw receives the event and deletes local
  LanceDB rows. Taxa/fitness data in DuckDB preserved per spec.

decisions:
  - what: No DuckDB changes on LAT deletion — taxa/fitness data preserved
    why: Taxa data is independent of LAT and must survive LAT purges (revoked/repealed laws still have historical enrichment)
    result: Only LanceDB legislation_text and amendment_annotations rows deleted
  - what: Count-before-delete pattern for accurate logging
    why: LanceDB delete returns no count. Count matching rows first, then delete, report the count.
    result: Accurate deletion counts in sync watch logs
  - what: No startup reconciliation (deferred)
    why: Would need to diff local LanceDB law_names against LRT lat_count on every reconnect. Complexity not justified yet.
    result: Noted as future work — catch missed signals while offline

metrics:
  methods_added: 3
  tests_added: 2

lessons:
  - title: LanceDB delete is idempotent but returns no count
    detail: table.delete(predicate) silently succeeds even if no rows match. Must count before delete if you need to report numbers.
    tag: infrastructure

artifacts:
  - crates/fractalaw-sync/src/zenoh_sync.rs
  - crates/fractalaw-store/src/lance.rs
  - crates/fractalaw-cli/src/main.rs

depends_on:
  - 02-27-26-zenoh-sync.md

enables:
  - Clean law lifecycle — sertantai can revoke laws and fractalaw purges local copies
---

# Session: Zenoh LAT Deletion Signal (CLOSED)

**Date:** 2026-03-27
**Signal spec:** `data/ZENOH-LAT-DELETION-SIGNAL.md`

## Summary

Implemented handling for the `lat_deleted` change notification from sertantai-legal. When an admin deletes LAT data for a law (revoked/repealed laws, incorrect `is_making` flags), sertantai publishes a zenoh event on `events/sync` with `action: "lat_deleted"`. Fractalaw now receives this and deletes local LanceDB rows.

## Changes Made

### fractalaw-sync (`zenoh_sync.rs`)
- Added `lat_deleted: Option<u64>` and `annotations_deleted: Option<u64>` to `SyncEventMetadata`
- Both `#[serde(default)]` — backward compatible with existing event payloads

### fractalaw-store (`lance.rs`)
- Added `delete_law_lat(law_name)` — deletes all `legislation_text` rows for a law
- Added `delete_law_annotations(law_name)` — deletes all `amendment_annotations` rows for a law
- Both use a shared `delete_by_law()` helper: checks table exists → counts rows → `table.delete(predicate)` → returns count
- Idempotent: returns 0 if table missing or no matching rows
- Added tests: `delete_law_lat_removes_only_target_law`, `delete_law_lat_no_table_returns_zero`

### fractalaw-cli (`main.rs`)
- Added `lat_deleted` branch in `cmd_sync_watch` event loop — fires before the upsert/enrich/publish pipeline
- Calls `delete_law_lat` + `delete_law_annotations`, logs counts, continues
- Added `total_deletions` counter to summary output

## Key Decisions

1. **No DuckDB changes** — taxa/fitness data is independent of LAT and must be preserved per spec
2. **No re-query** — after deletion, sertantai returns `[]` for that law's LAT; no point querying
3. **No startup reconciliation** — spec mentions checking `lat_count: 0` on reconnect as future work
4. **Count-before-delete** — we count matching rows before deleting so we can report accurate numbers in logs

## Follow-up / Future Work

- **Startup reconciliation**: On reconnect, diff local LanceDB law_names against LRT `lat_count` to catch signals missed while offline
- **Metrics**: Emit tracing spans / metrics for deletion events (e.g. for Prometheus)
