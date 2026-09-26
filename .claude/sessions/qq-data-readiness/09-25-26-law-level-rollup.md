---
session: Law-level Roll-up in Backfill
status: closed
opened: 2026-09-25
closed: 2026-09-26
outcome: success

summary: >
  Built the missing law-level roll-ups in taxa backfill (fractalatai #55): DRRP from reconciled provision_actors
  (excluding amendment-insertion text) and Approach L significance with thresholds frozen from July.
  Fixed data gaps on the way (unreconciled actors, missing hierarchy, 6,011 unrated provisions), then after Jason's
  review published 704/704 laws to sertantai-legal with 0 failures and only reviewed Making flips.

decisions:
  - what: Extend taxa backfill rather than add a new command
    why: 'The July significance design already put the law-level roll-up in backfill ("reconcile → backfill → publish"); existing-pipelines-first'
    result: 'query_significance_profile (July, unused) now called; law_significance + law_drrp pure fns in fractalaw-core; 18 tests'
  - what: Freeze significance thresholds from the July scores (LOW ≤ 6.14, HIGH ≥ 11.06)
    why: Percentiles recomputed each run would shift existing ratings as laws are added
    result: July values reproduced exactly (HSWA 12.189, CDM 16.909, MHSW 13.573, PPP 10.690)
  - what: Amendment-insertion text is a pipeline bug, not legal policy
    why: 'Jason: inserted text belongs to the amended instrument, and legal''s handoff filter is not foolproof'
    result: 'fractalatai #57 raised; #55 roll-up excludes it as an interim guard; 16 amendment-only laws → not Making'
  - what: Guard the DRRP roll-up (unreconciled or zero actor rows → leave law-level DRRP unchanged)
    why: The first dry runs showed false "no obligations" from never-reconciled actors (186 true→false) and actor-model gaps (75 zero-actor laws)
    result: true→false fell 186 → 59 → 41 → 23 published, all reviewed
  - what: 'Hold 18 sparse-actor laws, fill significance first, publish once, actor drift to a later session'
    why: 'Jason: don''t downgrade on incomplete actor evidence; one publish is cleaner than two'
    result: '704 laws published; 93 laws tracked in fractalatai #58'

metrics:
  publish: { laws: 704, failures: 0, making: 649, no_obligations: 47, empowering: 8, duration: "1m50s" }
  verdict_effect: { same: 628, true_to_false: 23, false_to_true: 15, review_wins: 38 }
  excluded: { zero_actor_laws: 75, held_sparse_actor_laws: 18 }
  reconcile_fix: { laws: 174, actor_rows: 33255 }
  significance: { hierarchy_derived: 10591, slm_rated: 6011, slm_minutes: 18.5, law_ratings: 705, high: 127, medium: 347, low: 231 }
  amendment_text: { provisions: 14012, laws: 419, obligation_signals: 642, fitness_mentions: 684 }
  actor_gap: { laws: 93, actorless_obligation_provisions: 556 }

lessons:
  - title: Dry-run the verdict diff before any DRRP publish. It found three data gaps.
    detail: 'The first diff showed 186 true→false. Each round exposed a different cause: never-reconciled provision_actors (168 laws), zero-actor laws (actor-model gap), then genuine amendment-only laws. Publishing the first roll-up would have downgraded ~160 Making laws on bad evidence.'
    tag: methodology
  - title: Pipeline steps silently skipped per law leave holes that aggregates hide
    detail: 'Reconcile never ran on 33K actor rows; hierarchy was never derived for 4,580 rated provisions; LAT re-imports under new section_ids dropped July significance. Law-level numbers looked plausible until checked against provision coverage. Check coverage (reconciled, rated, hierarchy) before trusting a roll-up.'
    tag: data
  - title: Parse can find duty text without naming a duty-bearer
    detail: '"The well operator must…" gets drrp_types = Obligation but no provision_actors row when the actor isn''t in the dictionary. Known actors (Secretary of State) were also missed. An actor-based roll-up then sees nothing, so treat zero actors as "no evidence", not "no duties".'
    tag: data
  - title: The significance SLM selects on significance_overall IS NULL
    detail: Provisions already rated but missing hierarchy look unrated and get re-rated. Derive hierarchy and backfill first; that cut the pod workload from 9,481 to 6,011.
    tag: tooling
  - title: Code built in July was never wired in
    detail: 'PgStore::query_significance_profile existed from the July significance session but backfill never called it. Before building, grep for the aggregate you need, not just the column name.'
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/taxa/law_significance.rs
  - crates/fractalaw-core/src/taxa/law_drrp.rs
  - crates/fractalaw-store/src/provision_store.rs
  - crates/fractalaw-store/src/pg.rs
  - crates/fractalaw-cli/src/main.rs
  - crates/fractalaw-cli/src/commands/pipeline.rs
  - .claude/skills/customer-batch-parse/SKILL.md
  - .claude/skills/fitness-pipeline/SKILL.md
  - data/qq-readiness/rollup/P55-publish-log.md
  - data/qq-readiness/rollup/P55-publish-verdicts.csv
  - /mnt/ssd/fractalaw-backups/fractalaw_pre_55_20260926.duckdb
  - /mnt/ssd/fractalaw-backups/fractalaw_pre_55drrp_20260926.duckdb
  - 'GitHub: fractalatai #55 (done), #57, #58 raised'

depends_on:
  - 09-25-26-qq-data-readiness.md
  - 07-01-26-significance-publish.md
  - 06-26-26-reconciliation.md

enables:
  - 'fractalatai #58: actor-model session (93 held/not-sent laws)'
  - 'fractalatai #57: skip amendment-insertion text at parse time'
  - QQ-01 62-law handoff (after #57 and #58)
---

# Session: Law-level Roll-up in Backfill (CLOSED)

## Problem

`taxa backfill` updates only provision-level data. Nothing rolls the reconciled provision data up to the DuckDB law-level summary that `sync publish` sends to sertantai-legal:
- **Law-level DRRP** comes only from the regex pass in `taxa parse` (`write_law_taxa`). It never uses the classifier/SLM/LLM tiers: 30% of actor rows were changed by those tiers, and 44 laws have reconciled duties but no duty in DuckDB.
- **Law-level significance** (`significance_rating` + K profile) has no pipeline writer. Only the one-off July run for 553 laws exists, so everything enriched since has NULL (63/65 T4 laws, 18/18 LAT-pilot laws).

Tracked in fractalatai #55. Pulled forward for compliance v0.1 (Jason, 2026-09-25).

## Todo

- ✅ Recover frozen Approach L thresholds from the July `significance_score` values in DuckDB: LOW ≤ 6.14, HIGH ≥ 11.06
- ✅ Extend `taxa backfill`: significance roll-up (`avg_sig × log2(total+1)` + high/med/low/total counts) → DuckDB, with tests (589 laws run)
- ✅ Extend `taxa backfill`: DRRP roll-up from reconciled `provision_actors`, with guards (unreconciled or zero actors → unchanged). Run on 802 laws
  - Obligation → duties, or responsibilities for `Gvt:`/`EU:`
  - Liberty → rights, or powers for `Gvt:`
  - exclude amendment-instruction text; empty lists where DRRP ran, NULL where it didn't; recompute `taxa_hash`
- ✅ Dry run: verdict diff vs legal `making_funnel`, fresh vs republished (`rollup/verdict_diff.csv`); significance vs July
- ✅ Jason reviewed the diff: hold the 18 sparse-actor laws; fill significance first, then publish once; actor drift → #58 (later session)
- ✅ Legal snapshot (`p55_taxa_snapshot_20260926`) + TaxaSubscriber running
- ✅ Published once: 704/704 laws, 0 failures (`rollup/P55-publish-log.md`, `P55-publish-verdicts.csv`)
- ✅ Significance fill: hierarchy for 4,580 already-rated provisions (67 laws), SLM for 6,011 unrated (18.5 min, 0 errors), hierarchy again, then backfill
- ✅ Update the `customer-batch-parse` and `fitness-pipeline` skills: backfill now owns the law-level roll-up (`f54d81e`)

## Dependencies

- ✅ QQ data readiness T4 + LAT pilot published (`09-25-26-qq-data-readiness.md`)
- ✅ Significance design and Approach L formula (`cascade/07-01-26-significance-publish.md`, `cascade/07-01-26-significance-aggregation.md`)
- ✅ Reconciliation engine / `taxa backfill` provision level (`archive/06-26-26-reconciliation.md`)
- ✅ Legal snapshot + running TaxaSubscriber at publish time

## Notes

- **Existing pipelines first:** extend `taxa backfill`. No new parallel command (feedback memory `feedback-existing-pipelines-first`).
- **Known reconciled-tier false positives** to check in the verdict diff: `UK_ssi_2021_50`, `UK_wsi_2020_1489` (Covid PPE, "HSE has notified" → Obligation) and `UK_ssi_2005_63` (code of practice purpose clause).
- **Benchmark laws** (`is_benchmark`) carry gold `provision_actors`. Rolling them up is a read; never re-classify them.

## Significance roll-up

- **Thresholds** (frozen from the July scores in DuckDB, 520 laws): LOW ≤ 6.126 < 6.14 < 6.158 ≤ MEDIUM ≤ 11.017 < 11.06 < 11.103 ≤ HIGH.
- **Formula and inputs verified** against the stored July values: HSWA 12.189 (31/48/93), CDM 16.909, MHSW 13.573, PPP 10.690 all reproduced exactly from current PG `significance_overall`.
- **Mostly wiring.** The July `PgStore::query_significance_profile` already existed but the backfill never called it. New pure fn: `fractalaw-core/src/taxa/law_significance.rs` (4 tests). `taxa backfill` now writes the 6 law-level columns, or NULL when no provision is rated.
- **DuckDB snapshot before the run:** `/mnt/ssd/fractalaw-backups/fractalaw_pre_55_20260926.duckdb`.

## DRRP roll-up

- **Pure logic:** `fractalaw-core/src/taxa/law_drrp.rs` (5 tests): the mapping, amendment-text exclusion and verdict.
- **Store:** `ProvisionStore::query_law_drrp_inputs` (PgStore impl).
- **Writer:** `pipeline::write_law_drrp` (typed empty lists, `taxa_hash`).
- **Where it runs:** wired into `taxa backfill`, only for laws where DRRP ran (parsed provisions).

## Findings (2026-09-26)

- **Significance run** (589 laws, vs the pre-run snapshot): 75 newly rated, 0 lost, 31 rating changes, 205 score-only changes.
  - The changes come from **provision data changing since July**, not the formula.
  - Rises: more provisions rated since July.
  - Drops: July-rated provisions no longer exist. LAT was re-imported with new section_ids (e.g. `UK_uksi_2019_196`, 106 → 7; `UK_uksi_2013_971`, 85 → 7).
- **Coverage gap:** 12,098 Obligation provisions are unrated.
  - Of 703 laws with obligations: 225 fully rated, 281 ≥80%, 128 partly, 69 none.
  - Law-level ratings for partly-rated laws are provisional until the significance SLM fill runs.
- **Amendment insertions (Jason): a pipeline bug, not legal policy.** The pipeline must not process text an amending law inserts into another instrument. Raised as **fractalatai #57**.
  - Scale: 14,012 provisions in 419 laws; 642 Obligation signals; 684 fitness mentions.
  - The #55 roll-up keeps the same detection as an interim guard.
- **QQ-01 handoff** (62 laws, sertantai-legal `11-qq01-fractalaw-handoff.txt`): parked by Jason until #55 and #57 are in. Legal was told.
  - Its new policy: fee regulations count as Making (DRRP decides).
- **DuckDB snapshots:**
  - `fractalaw_pre_55_20260926.duckdb`: before the significance run;
  - `fractalaw_pre_55drrp_20260926.duckdb`: before the DRRP run.

## Dry-run results and review (2026-09-26)

- **First run:** 186 true→false. Cause: 168 laws' `provision_actors` had never been reconciled (28K rows with NULL `extraction_method`).
  - Ran `taxa reconcile` on 174 laws (33,255 rows; no benchmarks).
  - Added a guard: unreconciled → unchanged.
- **Second run:** 59 true→false.
  - 75 of the 802 laws have **zero actor rows**, even though parse found duty text: an actor-model gap, e.g. "the well operator must".
  - Guard added: zero actors → law-level DRRP unchanged (restored from snapshot).
- **Final diff:**

  | Group | Same | True→false | False→true | Review wins | Not sent (no actors) |
  |---|---|---|---|---|---|
  | Fresh | 70 | 7 | 3 | – | 3 |
  | Republished | 558 | 34 | 12 | 38 | 72 |

- **The 41 true→false:**
  - 16 amendment-only: correct per #57;
  - 7 amending/definitional with 10+ actors: spot-checked, plausible;
  - 18 sparse-actor: **held** (Jason) and restored from snapshot (`rollup/held_sparse_actors.txt`).
- **Actor model:** 93 laws (75 zero + 18 sparse) with 556 actorless Obligation provisions → **fractalatai #58**. Covers both dictionary gaps (duty holder, well operator, …) and extraction gaps (known actors like Secretary of State with no rows).
- **Amendment insertions → fractalatai #57** (Jason: a pipeline bug).

## Publish (2026-09-26)

- **Code:** `7894eaa` (backfill roll-ups + guards); `f54d81e` (skills).
- **Published** to @dev: 704/704 laws, 0 failures, 1m50s.
  - Verdicts: making 649, no_obligations 47, empowering 8.
  - Expected vs legal: same 628, true→false 23 (reviewed), false→true 15, review wins 38.
- **Significance:** law-level now on 705 laws: HIGH 127, MEDIUM 347, LOW 231 (18/49/33%).
  - Not all remaining unrated Obligation provisions could be selected: the SLM script only picks provisions that have actor rows (#58).
- **Held / not sent:** 75 zero-actor + 18 sparse-actor laws (#58). The QQ-01 handoff stays parked (#57, #58).
