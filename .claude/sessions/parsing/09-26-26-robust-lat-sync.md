---
session: Robust LAT Sync
status: pending
opened: 2026-09-26
---

# Session: Robust LAT Sync (PENDING)

## Problem

LAT sync from legal to the hub is one-way and lossy. `upsert_lat` never deletes superseded rows, and events carry no hash or version, so a missed event is never recovered. The only delete path (`lat_deleted`) cascades away tier data. As a result, 345/802 hub laws have drifted from legal, and 128 are missing duty paragraphs (1,545 paragraphs). fractalatai #62.

## Todo

- ✅ Agree the manifest contract with legal (see Contract; #62 comment)
- ⬜ Hash function in `fractalaw-core` per contract (explicit White_Space set, NULL→"", all served rows) + legal's test vectors
- ⬜ Store `lat_hash` per law (hub + DuckDB)
- ⬜ Diff-apply in the store: delete rows legal no longer has; update changed rows (mark for re-parse); carry tier data across section_id renames on a unique exact normalised-text match; leave unchanged rows alone
- ⬜ `pull-lat --stale` (manifest compare → re-pull the drifted laws); `sync watch` runs the check at startup and on a schedule
- ⬜ Protect benchmarks: report drift, apply only with approval
- ⬜ Tests: Wester Ross reg.4(2) → Obligation; no duplicate normalised text within a law; tier data survives unchanged rows
- ⬜ Resume `09-26-26-stale-lat-repull.md`: pilot, roll-out, parse → reconcile → backfill, held-downgrade diff

## Dependencies

- ⬜ Legal: LAT manifest queryable, LatHash module, hash on events (legal pending session)
- ⬜ Legal: updated test vectors (after the all-rows change)
- ⬜ Legal: classify the 76 hub-only laws (`data/qq-readiness/lat/hub_not_in_legal.txt`)
- ✅ Measurement (`parsing/09-26-26-stale-lat-repull.md`, `data/qq-readiness/lat/lat_compare.csv`)

## Contract (agreed with legal 2026-09-26)

- **Manifest:** `fractalaw/@{tenant}/data/legislation/lat-manifest/{law_name}` and `/*`, returning `{law_name, row_count, lat_hash, updated_at}` (JSON; Arrow for `*`).
- **Hash rows:** the full row set the LAT queryable serves, including empty-text structural rows (NULL → ""), ordered by section_id in byte order.
  - Each row is `section_id \t normalise(text) \n`; the hash is the lowercase hex SHA-256.
  - `normalise`: NFC → collapse runs of the explicit Unicode White_Space set to a space → trim. Zero-width characters are kept.
- **Legal:** computes the hash on demand (~127 ms for the corpus); events carry the hash and row_count; `lat_deleted` = (0, sha256("")); the manifest is the source of truth.
- **section_id isn't stable** (legal #120 rewrote ids in place without events). Diff-apply carries tier data on a unique exact normalised-text match.
- **Pre-change vectors** (with the length(text)>0 filter, now superseded):
  - UK_ssi_2016_88: 23 rows, af6c4fb9…;
  - UK_uksi_2015_1947: 1,230 rows, 2a01ce91…
  
  Wait for legal's updated vectors.
