---
session: Law-level Roll-up in Backfill
status: pending
opened: 2026-09-25
---

# Session: Law-level Roll-up in Backfill (PENDING)

## Problem

`taxa backfill` updates only provision-level data. Nothing rolls the reconciled provision data up to the DuckDB law-level summary that `sync publish` sends to sertantai-legal:
- **Law-level DRRP** comes only from the regex pass in `taxa parse` (`write_law_taxa`). It never uses the classifier/SLM/LLM tiers: 30% of actor rows were changed by those tiers, and 44 laws have reconciled duties but no duty in DuckDB.
- **Law-level significance** (`significance_rating` + K profile) has no pipeline writer. Only the one-off July run for 553 laws exists, so everything enriched since has NULL (63/65 T4 laws, 18/18 LAT-pilot laws).

Tracked in fractalatai #55. Pulled forward for compliance v0.1 (Jason, 2026-09-25).

## Todo

- ⬜ Recover frozen Approach L thresholds (HIGH top 20%, LOW bottom 33%) from the July `significance_score` values in DuckDB
- ⬜ Extend `taxa backfill`: significance roll-up (`avg_sig × log2(total+1)` + high/med/low/total counts) → DuckDB, with tests
- ⬜ Extend `taxa backfill`: DRRP roll-up from reconciled `provision_actors`
  - Obligation → duties, or responsibilities for `Gvt:`/`EU:`
  - Liberty → rights, or powers for `Gvt:`
  - exclude amendment-instruction text; empty lists where DRRP ran, NULL where it didn't; recompute `taxa_hash`
- ⬜ Dry run: significance distribution vs the July baseline; verdict diff vs legal `making_funnel` (`is_making`, `is_making_source`), fresh vs republished, true→false highlighted
- ⬜ Jason reviews the diff; legal snapshots before any DRRP publish
- ⬜ Publish: significance changes `--fitness-only`; DRRP changes as a full publish after review
- ⬜ Update the `customer-batch-parse` and `fitness-pipeline` skills: backfill now owns the law-level roll-up

## Dependencies

- ✅ QQ data readiness T4 + LAT pilot published (`09-25-26-qq-data-readiness.md`)
- ✅ Significance design and Approach L formula (`cascade/07-01-26-significance-publish.md`, `cascade/07-01-26-significance-aggregation.md`)
- ✅ Reconciliation engine / `taxa backfill` provision level (`archive/06-26-26-reconciliation.md`)
- ⬜ Legal snapshot + running TaxaSubscriber at publish time

## Notes

- **Existing pipelines first:** extend `taxa backfill`. No new parallel command (feedback memory `feedback-existing-pipelines-first`).
- **Known reconciled-tier false positives** to check in the verdict diff: `UK_ssi_2021_50`, `UK_wsi_2020_1489` (Covid PPE, "HSE has notified" → Obligation) and `UK_ssi_2005_63` (code of practice purpose clause).
- **Benchmark laws** (`is_benchmark`) carry gold `provision_actors`. Rolling them up is a read; never re-classify them.
