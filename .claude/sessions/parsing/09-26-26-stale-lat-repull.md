---
session: Stale LAT Re-pull
status: suspended
opened: 2026-09-26
---

# Session: Stale LAT Re-pull (SUSPENDED)

## Suspended (2026-09-26)

Waiting on #62 (robust LAT sync, `09-26-26-robust-lat-sync.md`). Re-pulling with today's `upsert_lat` would leave superseded and duplicate rows in place, and clearing them cascades tier data. Resume once diff-apply and the hash exist.

## Problem

The Postgres hub holds out-of-date LAT for some laws, so their duties never reach parse. Example: Wester Ross MCO (UK_ssi_2016_88). The hub has a single reg.4 row, `reg.4(4)`, holding only "4.—(1) …". Legal's current LAT has all 9 rows, including `reg.4(2)` "A person must not deploy…". fractalatai #61 called this "unsplit" articles, but the text was never there to split. It comes from an older generation of legal's LAT parser. Nothing re-syncs LAT once the hub has it (one-way sync).

## Todo

- ✅ Measure: hub vs legal LAT per law (`data/qq-readiness/lat/lat_compare.csv`)
- ⬜ Confirm legal's current LAT is complete for the stale laws (spot checks)
- ✅ Understand the current LAT sync path: one-way, no delete, no hash (see findings; fix raised as #62)
- ⬜ Pilot re-pull + re-parse + reconcile (Wester Ross as regression test)
- ⬜ Roll out to the stale laws (exclude benchmarks; snapshot first)
- ⬜ Re-run the verdict diff for the 13 held #58 downgrades
- ✅ Correct #61's description (comment → #62)

## Dependencies

- ✅ #58 actor fixes (`parsing/09-26-26-actor-model-gaps.md`)
- ✅ #57 amendment scope
- ⬜ Legal's LAT current for the affected laws

## Measurement (2026-09-26)

Method: for each of the 802 hub laws, check whether each of legal's `legal_articles` rows (non-empty) appears in the hub's text for that law. Text is normalised to lowercase alphanumerics, so id and formatting differences don't count.

| Class | Laws |
|---|---|
| Hub has all of legal's text | 352 |
| Hub has everything, plus extra superseded text | 29 |
| **Legal has text the hub lacks** | **345** |
| Law not in legal's LAT | 76 |

Of the 345 laws with missing text:
- **Only non-duty text missing:** 217 laws.
- **Paragraphs with must/shall missing:**
  - 1–4 paragraphs: 90 laws (122 paragraphs);
  - 5–19: 18 laws (161);
  - **20+: 20 laws (1,262)**, e.g. Water Supply (Water Quality) 2016, 583 of 632 paragraphs missing; UK_uksi_2013_971, 513/546; Water Resources (Agricultural Pollution) (Wales) 2021, 278/314.
- **Also in the hub:**
  - 14.7K hub rows whose text isn't in legal (superseded wording or old-generation rows);
  - ~1.2K duplicate rows (the same text under an old and a new section_id).

**Held #58 downgrades:** UK_uksi_2017_1044, UK_ssi_2016_88 and UK_ssi_2016_90 are missing 3–4 duty paragraphs each. The other 10 aren't materially stale, so their downgrades are more likely genuine or #60 cases.

**Benchmarks among the ≥5 group:** UK_ukpga_1981_69 and UK_uksi_2014_1643 are excluded. There are 36 non-benchmark laws with 5+ missing duty paragraphs (`lat/stale5.txt`).

## Why the hub goes stale: sync path findings

- LAT enters the hub only via `pull-lat` or `sync watch` (on a legal `lat` event), then `upsert_lat`.
- **The upsert is keyed on section_id and never deletes rows.** When legal re-parses a law and its ids change, the old rows stay: superseded text, and the same text under two ids.
- **Events carry no content hash or version.** An event missed while `watch` isn't running is never replayed, and nothing detects drift afterwards.
- `lat_deleted` events trigger `delete_law_lat`. That cascades `provision_actors` (#58 cause A), which is the only path that clears old rows.
