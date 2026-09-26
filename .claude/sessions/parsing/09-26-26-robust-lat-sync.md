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
- ⬜ Diff-apply in the store: delete rows legal no longer has; text changed → update + mark for re-parse; sort_key-only change → update in place, no re-parse, tier data kept; carry tier data across section_id renames on a unique exact normalised-text match; leave unchanged rows alone
- ⬜ `pull-lat --stale` (manifest compare → re-pull the drifted laws); `sync watch` runs the check at startup and on a schedule
- ⬜ Protect benchmarks: report drift, apply only with approval
- ⬜ Tests: Wester Ross reg.4(2) → Obligation; no duplicate normalised text within a law; tier data survives unchanged rows
- ⬜ Resume `09-26-26-stale-lat-repull.md`: pilot, roll-out, parse → reconcile → backfill, held-downgrade diff

## Dependencies

- ✅ Legal: LAT manifest queryable, stored lat_hash (triggers), hash on events after commit; cross-checked 980/980
- ✅ Legal: updated test vectors, cross-checked by an independent Python implementation (synthetic, empty, 3 live laws all match)
- ✅ Legal: classified the 76 hub-only laws (66 revoked, 5 regnal-year duplicates, 5 in force awaiting legal LAT)
- ✅ Measurement (`parsing/09-26-26-stale-lat-repull.md`, `data/qq-readiness/lat/lat_compare.csv`)

## Contract (agreed with legal 2026-09-26)

- **Manifest:** `fractalaw/@{tenant}/data/legislation/lat-manifest/{law_name}` and `/*`, returning `{law_name, row_count, lat_hash, updated_at}` (JSON; Arrow for `*`).
- **Hash rows:** the full row set the LAT queryable serves, including empty-text structural rows (NULL → ""), ordered by section_id in byte order.
  - Each row is `section_id \t sort_key \t normalise(text) \n`: sort_key as-is, NULL → "", position excluded. The hash is the lowercase hex SHA-256.
  - The sort_key change was proposed by both sides after legal's sort_key bug. Legal relayed Jason's approval; **to be confirmed by Jason in the fractalaw session.**
  - `normalise`: NFC → collapse runs of the explicit Unicode White_Space set to a space → trim. Zero-width characters are kept.
- **Legal:** computes the hash on demand (~127 ms for the corpus); events carry the hash and row_count; `lat_deleted` = (0, sha256("")); the manifest is the source of truth.
- **section_id isn't stable** (legal #120 rewrote ids in place without events). Diff-apply carries tier data on a unique exact normalised-text match.
- **Test vectors:**
  - **Pin:** synthetic `TEST:reg.1`, sort_key `00001~`, text `"  A person  must   not\tdeploy.\u200b "` → `79bc96eafd545bbab104423e759bb2787d4b8aae4c34a2545eb41249a67cf07b` (with sort_key);
  - **Pin:** empty law → sha256("") `e3b0c442…`;
  - legal's checked-in fixture law, to follow.
  - Live values with sort_key, not to pin: UK_ssi_2016_88 (28 rows, f75e2c32…), UK_ukpga_1974_37 (835, 979269b6…), UK_uksi_2015_1947 (1,323, be6964f8…).
  - All were verified 2026-09-26 by an independent Python implementation against `legal_articles`.

## 76 hub-only laws (legal's classification, 2026-09-26)

- **66 revoked:** legal holds no LAT, and DuckDB has `status = revoked` for 50 of them. Under the manifest (row_count 0), diff-apply would delete their hub rows. The first run will list them for Jason's approval rather than applying silently.
  - **9 of the 11 #58 false→true flips are revoked laws:** UK_nisr_2008_55, UK_uksi_2000_1973, UK_uksi_2000_3184, UK_uksi_2001_1091, UK_uksi_2004_107, UK_uksi_2004_3212, UK_uksi_2009_785, UK_uksi_2010_105, UK_uksi_2014_255.
  - #58's verdict comparison didn't filter on `status`. **Open for Jason:** re-send or leave those verdicts, and add a revoked guard to enrichment/publish.
- **5 regnal-year duplicates**, which legal holds under modern names (e.g. `UK_ukpga_1875_Vict/38-39/17` → `UK_ukpga_1875_17`). These are the 3 #57 publish skips. **Drop or rename for Jason's approval.**
- **5 in force with no legal LAT** (UK_ssi_2005_157, UK_uksi_1998_892, UK_uksi_2015_10, UK_wsi_2014_3303, UK_ukpga_1994_27): legal's gap, queued for LAT parse.

## Legal updates (2026-09-26, later)

- **The 5 in-force hub-only laws are now LAT-parsed in legal:**

  | Law | Rows |
  |---|---|
  | UK_ssi_2005_157 | 258 |
  | UK_uksi_2015_10 | 141 |
  | UK_wsi_2014_3303 | 133 |
  | UK_ukpga_1994_27 | 20 |
  | UK_uksi_1998_892 | 9 |

  - `lat` events were emitted.
  - The hub holds stale copies. A plain `pull-lat` would leave the old-generation rows next to the new ones, so these wait for diff-apply, or for a one-off clean re-pull if Jason approves.
  - The last 4 are marked enriched in legal from stale hub LAT and need re-enrichment on fresh LAT.
- **Legal bug: `sort_key` ordering** (legal fix session pending).
  - Lettered items (c) and (d) are encoded as Roman numerals, so they sort after (g). Also, every `signed` row has an all-zero sort_key and sorts first.
  - This doesn't affect `lat_hash`, which orders by section_id.
  - **Fractalaw's exposure:**
    - `fitness.rs:540` concatenates child text in sort_key order, so stem + child text can arrive scrambled;
    - `pg.rs:544-553` assigns each part by the preceding sort_key. The signed row sorts first, but paragraph misorder stays within a section, so this is probably harmless;
    - `pg.rs:61/68` loads provisions in sort_key order;
    - `scripts/compliance/generate_controls.py:131`.
  - Nothing to change until legal re-parses. Section ids and text are unaffected.

## Legal side live + cross-check (2026-09-26)

- **Queryables:** `fractalaw/@dev/data/legislation/lat-manifest/{law}` and `/*`. Arrow IPC by default, `?format=json` for JSON.
  - `*` lists only laws with LAT (980).
  - A single law with no LAT returns row_count 0 and sha256("").
- **Hash storage:** legal now stores `legal_register.lat_hash`, maintained by triggers on section_id, sort_key and text. Computing on demand turned out to take 8.4 s for the corpus, not 127 ms.
- **Events:** `lat` persist and `lat_deleted` events carry row_count + lat_hash and are sent after commit. `lat.fix_section_ids` now emits events too.
- **Fixture vectors:** `sertantai-legal/backend/test/fixtures/lat_hash/vectors.json` (synthetic, empty, fixture_law `UK_uksi_2099_1` with 9 rows, `aa3c16e4…`). Copy them into fractalaw-core tests when building.
- **Cross-check** (scratch Python client, read-only):
  - all 3 vectors match;
  - 980/980 laws: the hash recomputed from `lat/{law}` rows matches the manifest on hash and row_count.
- **Client note:** `lat/{law}` for a law with no LAT replies with a zero-length payload. Treat that as 0 rows.

## Legal sort_key fix (2026-09-26, later)

- **Fixed:** paragraph segments are letters-only, so (c) and (d) no longer sort as Roman numerals, and `signed` rows sort after the body. The stored rows were rewritten in place: 20,848 rows in **579 laws**, sort_key only. **These 579 have a new lat_hash**, so the manifest will show them as stale.
  - Diff-apply must treat a sort_key-only change as an in-place update: no re-parse, tier data kept.
- **Still open on legal's side, held until fractalaw's diff-apply exists:**
  - **386 laws** carry sort_keys from older parser generations and need a legal re-parse. That can shift section_ids, and 101 of them are enriched.
  - **Parent-drop bug:** after a nested sub-paragraph the parser can drop the parent paragraph, e.g. UK_wsi_2025_1321 `reg.39(e)`, which should be `reg.39(2)(e)`. The fix changes section_ids.
  - Both rely on the text-match carry-over to preserve tier data.
