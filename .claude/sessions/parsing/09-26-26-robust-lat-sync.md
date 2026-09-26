---
session: Robust LAT Sync
status: pending
opened: 2026-09-26
---

# Session: Robust LAT Sync (PENDING)

## Problem

LAT sync from legal to the hub is one-way and lossy. `upsert_lat` never deletes superseded rows, and events carry no hash or version, so a missed event is never recovered. The only delete path (`lat_deleted`) cascades away tier data. As a result, 345/802 hub laws have drifted from legal, and 128 are missing duty paragraphs (1,545 paragraphs). fractalatai #62.

## Todo

- ⬜ Agree the manifest contract with legal: per-law `row_count`, `lat_hash` (over ordered section_id + normalised text), `updated_at`; Zenoh queryable, hash also on `lat` events
- ⬜ Hash function in `fractalaw-core`, identical to legal's (shared test vectors)
- ⬜ Store `lat_hash` per law (hub + DuckDB)
- ⬜ Diff-apply in the store: delete rows legal no longer has, update changed rows (mark for re-parse), leave unchanged rows and their tier data alone
- ⬜ `pull-lat --stale` (manifest compare → re-pull the drifted laws); `sync watch` runs the check at startup and on a schedule
- ⬜ Protect benchmarks: report drift, apply only with approval
- ⬜ Tests: Wester Ross reg.4(2) → Obligation; no duplicate normalised text within a law; tier data survives unchanged rows
- ⬜ Resume `09-26-26-stale-lat-repull.md`: pilot, roll-out, parse → reconcile → backfill, held-downgrade diff

## Dependencies

- ⬜ Legal: LAT manifest queryable (+ hash on events)
- ✅ Measurement (`parsing/09-26-26-stale-lat-repull.md`, `data/qq-readiness/lat/lat_compare.csv`)
