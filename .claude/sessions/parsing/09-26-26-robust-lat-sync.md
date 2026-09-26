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
- ✅ Legal: updated test vectors, cross-checked by an independent Python implementation (synthetic, empty, 3 live laws all match)
- ✅ Legal: classified the 76 hub-only laws (66 revoked, 5 regnal-year duplicates, 5 in force awaiting legal LAT)
- ✅ Measurement (`parsing/09-26-26-stale-lat-repull.md`, `data/qq-readiness/lat/lat_compare.csv`)

## Contract (agreed with legal 2026-09-26)

- **Manifest:** `fractalaw/@{tenant}/data/legislation/lat-manifest/{law_name}` and `/*`, returning `{law_name, row_count, lat_hash, updated_at}` (JSON; Arrow for `*`).
- **Hash rows:** the full row set the LAT queryable serves, including empty-text structural rows (NULL → ""), ordered by section_id in byte order.
  - Each row is `section_id \t normalise(text) \n`; the hash is the lowercase hex SHA-256.
  - `normalise`: NFC → collapse runs of the explicit Unicode White_Space set to a space → trim. Zero-width characters are kept.
- **Legal:** computes the hash on demand (~127 ms for the corpus); events carry the hash and row_count; `lat_deleted` = (0, sha256("")); the manifest is the source of truth.
- **section_id isn't stable** (legal #120 rewrote ids in place without events). Diff-apply carries tier data on a unique exact normalised-text match.
- **Test vectors:**
  - **Pin:** synthetic `TEST:reg.1` / `"  A person  must   not\tdeploy.\u200b "` → `f6ae5038725aeb87b6b37e9684c92dedb5ca34422ea0db5a8e350eafc918f8a5`;
  - **Pin:** empty law → sha256("") `e3b0c442…`;
  - legal's checked-in fixture law, to follow.
  - Live values, not to pin (they change when legal re-parses): UK_ssi_2016_88 (28 rows, 347ca311…), UK_ukpga_1974_37 (835, 8263fdac…), UK_uksi_2015_1947 (1,323, e901ecda…).
  - All were verified 2026-09-26 by an independent Python implementation against `legal_articles`.

## 76 hub-only laws (legal's classification, 2026-09-26)

- **66 revoked:** legal holds no LAT, and DuckDB has `status = revoked` for 50 of them. Under the manifest (row_count 0), diff-apply would delete their hub rows. The first run will list them for Jason's approval rather than applying silently.
  - **9 of the 11 #58 false→true flips are revoked laws:** UK_nisr_2008_55, UK_uksi_2000_1973, UK_uksi_2000_3184, UK_uksi_2001_1091, UK_uksi_2004_107, UK_uksi_2004_3212, UK_uksi_2009_785, UK_uksi_2010_105, UK_uksi_2014_255.
  - #58's verdict comparison didn't filter on `status`. **Open for Jason:** re-send or leave those verdicts, and add a revoked guard to enrichment/publish.
- **5 regnal-year duplicates**, which legal holds under modern names (e.g. `UK_ukpga_1875_Vict/38-39/17` → `UK_ukpga_1875_17`). These are the 3 #57 publish skips. **Drop or rename for Jason's approval.**
- **5 in force with no legal LAT** (UK_ssi_2005_157, UK_uksi_1998_892, UK_uksi_2015_10, UK_wsi_2014_3303, UK_ukpga_1994_27): legal's gap, queued for LAT parse.
