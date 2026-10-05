---
session: Re-enrichment Backlog
status: suspended
opened: 2026-09-29
---

# Session: Re-enrichment Backlog (SUSPENDED)

**Blocked (Jason, 2026-10-01): waits until the data models are finished** (#73 verdict split + as made, #72 correlatives), so the re-run populates the final model in one pass.

## Problem

The first #62 hub sync (2026-09-27) flagged 422 laws `reparse_needed`: their text changed, and 15,296 changed-text snapshots plus 38,161 superseded rows were archived with their tier data. Only QQ Tier 0 (122) and the live-fix 8 have been re-enriched since. Further laws have joined from legal: its 101-law re-enrichment list after the LAT re-parse, and 4 from the 42-parent-Act extent re-parse. Until re-enriched, these laws' law-level results in DuckDB (and in legal) reflect old actors, and they carry no provenance.

## Todo

- ⬜ Build the backlog list: `lat_sync_state.reparse_needed` ∪ legal's `reenrichment-list.csv` (101) ∪ extent re-parse sync 6 (UK_ukpga_1990_16, UK_ukpga_1988_52, UK_ukpga_1993_11, UK_anaw_2017_2, UK_ukpga_1989_14, UK_uksi_2000_3184 — the last is served by legal again after being archived as revoked), minus laws legal discarded as not Making (lean LAT) and the benchmark laws (#65)
- ⬜ From legal's Tier 1 writes (2026-09-29):
  - **Re-enrich** after the Tier 1 sync (applied 2026-09-29, 26 hub laws; 136 renames from legal's log, 13,763 actors carried): UK_ukpga_2023_55, UK_ukpga_1996_18, UK_ukpga_1991_22, UK_ukpga_2008_29 (reparse_needed).
  - **24 Making Acts newly parsed with full LAT** (31,264 rows), candidates for full enrichment when Jason schedules it: UK_anaw_2017_3, UK_asc_2023_2, UK_asp_2002_3, UK_asp_2003_3, UK_asp_2005_15, UK_asp_2005_3, UK_asp_2009_6, UK_asp_2011_9, UK_asp_2024_13, UK_ukpga_1967_8, UK_ukpga_1973_26, UK_ukpga_1987_53, UK_ukpga_1991_46, UK_ukpga_1995_21, UK_ukpga_1995_23, UK_ukpga_1996_8, UK_ukpga_1997_28, UK_ukpga_1999_29, UK_ukpga_2002_40, UK_ukpga_2004_18, UK_ukpga_2021_26, UK_ukpga_2023_52, UK_ukpga_2023_6, UK_ukpga_2025_5.
  - **89 non-Making Acts with scoped `enabling_extent` LAT** (7,556 rows): never triage, enrich or derive Making from them (needs the scoped-LAT handling first).
- ⬜ Prioritise with legal: Making laws in customer registers first; QQ tiers beyond Tier 0
- ⬜ **Rename recovery first** (cheaper than recomputing SLM work). A bug meant the first #62 sync (2026-09-27) consumed none of legal's rename log: the naive `created_at` failed RFC 3339 parsing and was silently skipped (fixed 2026-09-29). Renamed rows were archived + re-inserted, so their actor rows sit in `lat_archive` (reason `removed`).
  - For laws not re-enriched since (not Tier 0 or the live-fix 8), map archived old ids → new ids via legal's `lat-renames` (status `renamed`).
  - Where the new row's text still matches (`match_key` equal), restore the archived actor/fitness rows onto the new id.
  - Mark the law reparse_needed and record provenance.
- ⬜ Run the pipeline in batches (customer-batch-parse): parse → dep → embed → classify → infer → reconcile → pod SLM (position/significance/fitness) → LLM tier (Gemini, fixed prompt, test batch of 10 then 50s) → hierarchy → backfill → fitness → verdict diff → Jason review → publish with provenance
- ✅ #67 implied rights: done in its own session `parsing/2026-09-29-issue-67.md` (closed)
- ⬜ **pending_slm added 2026-09-30:** 3,785 in the 20 benchmark laws after their LAT sync (893 + 2,892) and 310 in 5 re-parsed held laws (UK_uksi_1999_1676, 2003_751, 2005_1726, 2010_768, 2013_1119). Add them to the next pod batch, re-backfill and republish; legal then re-checks the holder-unknown share
- ⬜ **(Jason)** Decide whether revoked laws are excluded from pod/LLM runs (64 DuckDB-revoked laws, ~87K substantive provisions; verify statuses first). See memory `feedback_revoked_laws`
- ✅ 6 Tier 0 pending_slm actors: resolved (0 pending as of 2026-09-29)
- ⬜ **Batch 1** (EPA 1990 + 24 new Making Acts): local steps done 2026-09-29 (17,137 actors reconciled, 7,845 pending_slm; 1,447 fitness mentions). Pod workload: position ~16,342 actors, significance ~2,351+ obligation provisions (after re-reconcile), fitness 1,447 mentions. DuckDB snapshot `fractalaw_pre_batch1_20260929.duckdb`
- ⬜ Audit the earlier corpus LLM labels (~2,449 `llm` actors) for the penalty-clause error fixed in `e7bc2ab`: find Obligation labels on offence/penalty text, clear, re-run with the fixed prompt
- ⬜ After legal #166 scopes large Acts: re-enrich the 6 Tier 0 big Acts on scoped LAT (Companies Act 2006, PH(S)A 2008, IPA 2016, PCA 2017, CTBSA 2019, EU(W)A 2018)
- ⬜ As laws are re-parsed: check the first `provision_versions` rows (legislative history, #73 L9) against legal's `lat-changes` log (moved from #73)
- ⬜ Definitions knock-on rule (#73 L4): when a definition changes, re-parse the same law's provisions that use the term, if legal serves definition links (moved from #73)
- ⬜ Stale law-level verdicts on ~40 zero-actor laws (old regex roll-ups; e.g. UK_uksi_2000_3184, UK_uksi_2012_3018): parse them, or give them a holder-unknown verdict; dry run + Jason review first (from #73, 2026-10-01)
- ⬜ Keep DuckDB snapshots before each batch's parse (parse rewrites law-level DRRP); restore held/zero-actor laws afterwards

## Single-run checklist (from `parsing/2026-09-30-issue-72.md`, 2026-10-01)

This session runs the **single run**: one enrichment pass and **one** publish, after the data model is finished (Jason: build first, run once). Before publishing, the final dry run must show:
- ⬜ **0 `partially_parsed` laws.** That's the check that the backlog parse covered every live row (397 laws had unparsed live rows; all are `reparse_needed`)
- ⬜ The **21 stored negative verdicts** set before a LAT sync (no_obligations/empowering on now-partial parses) are re-decided
- ⬜ **Smell list** (live, amended, not making) sent to legal for Jason's per-law made-text approval (preview: 4 laws)
- ⬜ **Purpose vocabulary:** count of laws still carrying old purpose labels (not republished). Legal's store mixes the two vocabularies until they're re-enriched
- ⬜ Correlatives (claim/liability/protected holders), current_* and as-made verdicts reviewed together (Jason)
- ⬜ **Purpose change list for compliance (legal#172):** per law, old `purpose` values → new (from `purpose_profile`, share ≥ 0.05), sent to legal after the dry run. Process+Rule has no 1:1 mapping
- ⬜ **(Jason)** Compliance screening: `current_verdict` or `is_making`. Decided before the publish
- ⬜ Company officer repair (2026-10-05, `0e7846d`):
  - Gvt: Officer was matching "director … or other similar officer of the body corporate": 401 hub provisions in 144 laws, 166 active rows (false government Responsibility holders).
  - The extractor is fixed. The 90 backlog laws are re-parsed in the run anyway.
  - **54 laws outside the backlog added to the run's re-parse** (Jason, 2026-10-05; regex/LLM rows, not just backfill): UK_asc_2026_5, UK_asp_2003_8, UK_asp_2005_13, UK_ssi_2009_266, UK_ssi_2014_161, UK_ssi_2014_258, UK_ukpga_1947_39, UK_ukpga_1961_34, UK_ukpga_1968_72, UK_ukpga_1971_60, UK_ukpga_1971_61, UK_ukpga_1974_37, UK_ukpga_1976_72, UK_ukpga_1982_16, UK_ukpga_1982_45, UK_ukpga_1984_26, UK_ukpga_1987_12, UK_ukpga_1989_15, UK_ukpga_1989_40, UK_ukpga_1990_8, UK_ukpga_1994_21, UK_ukpga_1996_6, UK_ukpga_1997_8, UK_ukpga_2000_23, UK_ukpga_2000_5, UK_ukpga_2003_21, UK_ukpga_2004_20, UK_ukpga_2005_16, UK_ukpga_2006_28, UK_ukpga_2006_36, UK_ukpga_2008_32, UK_ukpga_2010_15, UK_ukpga_2018_12, UK_ukpga_2022_30, UK_uksi_1997_2962, UK_uksi_2001_1701, UK_uksi_2006_2183, UK_uksi_2006_2184, UK_uksi_2007_3075, UK_uksi_2007_3077, UK_uksi_2007_3106, UK_uksi_2008_2164, UK_uksi_2010_330, UK_uksi_2010_332, UK_uksi_2011_1483, UK_uksi_2012_3032, UK_uksi_2015_1640, UK_uksi_2015_668, UK_uksi_2016_1026, UK_uksi_2016_1092, UK_uksi_2016_1154, UK_uksi_2017_1013, UK_uksi_2020_206, UK_wsi_2005_1806.
- ⬜ `Ind: Public` (Jason, 2026-10-05; hub ✅):
  - The correlative rule is **retired** (not required).
  - **`Ind: Public` is the canonical label for the public**: "innocent" third parties with no defined role or organisation. The dictionary's `Public` was renamed via `renamed_from`.
  - Hub: `scripts/migrations/retire_public_correlative_20261005.py` deletes the rule's 1,148 rows (99 laws), then `rename_actor_labels_20261005.py` renames 4,972 `Public` rows. Both applied 2026-10-05 (backup `data/backups/pre_ind_public_20261005.dump`): 1,148 deleted, 4,972 + 9 gold_v2 renamed, 0 collisions.
  - Legal, at pre-publish: delete the inferred `Ind: Public` beneficiary entries (`reason: inferred, position: beneficiary, drrp: none`; 1,137 rows, 97 laws), then rename `Public` → `Ind: Public` across stored rows.
- ⬜ 23 legal laws with bare "Public" as a holder in responsibilities/powers entries (legal_register):
  - All 23 have no LAT in the hub (not_held), so the single run can't re-derive them. Our DuckDB has no "Public" on the government side for them; legal's entries are stale from an older publish.
  - **Jason: yes.** Legal removes "Public" from those government-side entries in its pre-publish migration. Layer 4 "never cross-assign": a governed actor never holds a Responsibility or Power.
  - Laws: UK_asc_2025_4, UK_asp_2012_8, UK_asp_2016_2, UK_nia_2011_9, UK_nisr_1999_90, UK_nisr_2008_315, UK_nisr_2009_238, UK_ssi_2025_417, UK_ukpga_1937_67, UK_ukpga_1962_58, UK_ukpga_1968_27, UK_ukpga_1984_22, UK_ukpga_1991_65, UK_ukpga_2025_10, UK_uksi_1975_335, UK_uksi_2003_835, UK_uksi_2004_756, UK_uksi_2005_1992, UK_uksi_2005_676, UK_uksi_2006_336, UK_uksi_2008_1660, UK_uksi_2010_404, UK_uksi_2013_602
- ⬜ **Actor label renames and the Authorised Person class change (2026-10-05):**
  - ✅ hub rows migrated 2026-10-05 (`scripts/migrations/rename_actor_labels_20261005.py`);
  - ✅ legal's `government_label?/1` no longer lists Spc: Authorised Person: live 2026-10-05 (legal bb4bb624); legal's rename task is `mix actors.rename_labels` (6f9600e4), run at our pre-publish message;
  - ⬜ **30 laws carrying Spc: Authorised Person outside the backlog → added to the single run's backfill + publish** (Jason, 2026-10-05; option c). No re-parse; `taxa backfill` re-derives law-level holders under the governed class. Laws: UK_asp_2009_12, UK_asp_2026_6, UK_nisr_2011_240, UK_nisr_2026_14, UK_ssi_2009_266, UK_ukpga_1968_73, UK_ukpga_1982_16, UK_ukpga_1982_45, UK_ukpga_1989_40, UK_ukpga_1994_21, UK_ukpga_1996_6, UK_ukpga_2003_21, UK_ukpga_2004_20, UK_ukpga_2006_28, UK_ukpga_2006_49, UK_ukpga_2010_15, UK_ukpga_2020_7, UK_ukpga_2022_30, UK_uksi_1996_1527, UK_uksi_1997_2962, UK_uksi_2002_2786, UK_uksi_2007_3106, UK_uksi_2012_3032, UK_uksi_2016_765, UK_uksi_2019_156, UK_uksi_2020_206, UK_uksi_2021_1309, UK_uksi_2021_534, UK_wsi_2005_1806, UK_wsi_2009_995;
  - legal runs the same rename map on its stored legal_articles rows at the publish. 88 of the 235 affected laws are outside the backlog, so our payloads alone won't rename them.

## Dependencies

- ⬜ SLM retrained on drrp-v1.0 labels (`parsing/2026-10-01-training-labels-slm.md`): the run uses it

- ✅ #62 hub sync + diff-apply; #63 provenance; refined zero-actor guard (`e7bc2ab`)
- ⬜ RunPod per batch (Jason launches when the local steps are done)
- ⬜ Legal's snapshots per publish; sertantai-legal #166 for the big Acts
