---
session: Actor Model Gaps
status: closed
opened: 2026-09-26
closed: 2026-09-26
outcome: success

summary: >
  Fixed fractalatai #58: 93 laws had duty text but no or few provision_actors. Several causes stacked up: actor rows were
  lost after LAT re-pulls; upsert errors were swallowed; family gating never matched because families carry an emoji prefix;
  11 dictionary entries had no regex patterns; and there were no Mayor/CCA entries. Actorless Obligation provisions went
  556 → 109. After Jason's review, 31 laws were published (11 false→true, 20 same, 0 errors); the 13 downgrades were held.

decisions:
  - what: Treat actorless provisions as an extraction problem first, a dictionary problem second
    why: 'UK_uksi_2015_398 matched actors on legislation_text but had zero provision_actors rows; a re-parse restored 441'
    result: Four code/data causes fixed in 48c89b9; only Mayor and CCA were genuinely missing
  - what: Hold all 13 true→false downgrades
    why: 'Jason: evidence incomplete. E.g. the Wester Ross MCO prohibition sits in an unsplit article row (#61)'
    result: Restored from the pre_58 snapshot and not sent; 31 published
  - what: Raise impersonal/passive duties, unsplit articles and the law-level fitness roll-up as separate issues
    why: 'Jason: "we''re fixing the actor issue"; they need tackling, but not in this session'
    result: '#59, #60, #61'
  - what: Leave Administrator and Receiver trigger-only
    why: Too generic as regex (administrator of a scheme, receiver of a notice)
    result: LLM tier still matches them

metrics:
  laws: { total: 93, rich: 20, sparse: 24, zero: 49 }
  actorless_obligation_provisions: { before: 556, after: 109, laws_after: 17 }
  reconcile_new_rows: 3125
  verdict_diff: { with_actors: 44, same: 20, false_to_true: 11, true_to_false_held: 13 }
  publish: { laws: 31, errors: 0, legal_is_making_before: 3587, legal_is_making_after: 3598 }
  dictionary: { entries: 125, patterns_added: 11, new_entries: 2 }

lessons:
  - title: Emoji-prefixed families silently disabled family gating corpus-wide
    detail: 'Every DuckDB family starts with an emoji, so `families:` gating in the actor dictionary and fitness specialist dictionaries never matched. Nothing errored; the specialist entries just never fired. normalize_family now strips the prefix at both call sites.'
    tag: data
  - title: Actorless is not the same as missing from the dictionary
    detail: Most "missing" actors were in the dictionary. Their rows were lost after LAT re-pulls (ON DELETE CASCADE), or the entry had triggers but no regex. Check re-parse recency and regex coverage before adding entries (now in the actor-drift skill).
    tag: methodology
  - title: .ok() on a batch upsert hides partial loss
    detail: 'upsert_provision_actors aborts on the first bad row; .await.ok() dropped every remaining actor for the law without a trace. Failures are now printed.'
    tag: tooling
  - title: Dictionary order is semantic
    detail: 'First match wins: Young Person must precede Person, and Mayor/CCA must precede Local Authority. Reordering YAML by script dropped a label once, so verify label counts after bulk edits.'
    tag: data
  - title: Downgrades from a partial model are not evidence
    detail: When coverage improves unevenly, a law can lose its only regex-era duty while its real duties still sit in unsplit or impersonal provisions. Hold downgrades until the gap issues (#60, #61) land.
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/taxa/mod.rs
  - crates/fractalaw-core/src/taxa/actors.rs
  - crates/fractalaw-core/src/taxa/fitness.rs
  - crates/fractalaw-core/data/actor-dictionary.yaml
  - crates/fractalaw-cli/src/commands/pipeline.rs
  - .claude/skills/actor-drift/SKILL.md
  - .claude/skills/actor-drift/scripts/surface_missing_actors.py
  - data/qq-readiness/actors/P58-publish-log.md
  - data/qq-readiness/actors/verdict_diff.csv
  - data/qq-readiness/actors/held_downgrades.txt
  - /mnt/ssd/fractalaw-backups/fractalaw_pre_58_20260926.duckdb
  - /mnt/ssd/fractalaw-backups/fractalaw_pre_58trees_20260926.duckdb

depends_on:
  - 09-25-26-law-level-rollup.md
  - 09-26-26-amendment-insertion-scope.md

enables:
  - 'QQ-01 62-law handoff'
  - 'fractalatai #60 impersonal duties, #61 unsplit articles'
---

# Session: Actor Model Gaps (CLOSED)

## Problem

93 laws have duty text but zero or very few `provision_actors` rows (75 zero, 18 sparse), leaving 556 actorless Obligation provisions. Their duties are invisible to the law-level DRRP roll-up, so they were held or not sent in #55 and #57. Example: "The well operator must ensure…" (UK_uksi_2015_398) gets `drrp_types = Obligation` but no actor. Some gaps are dictionary gaps (duty holder, well operator); some affect actors already in the dictionary (Secretary of State, local authority), so it's an extraction gap too. fractalatai #58.

## Todo

- ✅ Root-cause the extraction gap: lost rows after LAT re-pulls, swallowed upsert errors, trigger-only entries, emoji-prefixed family gating
- ✅ Surface missing duty-bearers across the 93 laws (subject extraction against the PG hub)
- ✅ Classify and add to `actor-dictionary.yaml`, with tests (Mayor, CCA; 11 entries given patterns)
- ✅ Fix the extraction gap (`48c89b9`)
- ✅ Re-run the existing pipeline on the 93: parse, reconcile, backfill (SLM for new actors deferred to the next pod session)
- ✅ Verdict diff + Jason review: publish 31 (11 gains + 20 same), hold 13 downgrades
- ✅ Published 31/31 (0 errors), legal snapshot `p58_taxa_snapshot_20260926` (`actors/P58-publish-log.md`)
- ✅ actor-drift skill: `--source pg` (actorless substantive provisions, `--laws/--law-file/--family`, emoji-safe family filter)

## Dependencies

- ✅ #55 law-level roll-up with guards (`qq-data-readiness/09-25-26-law-level-rollup.md`)
- ✅ #57 amendment scope (`parsing/09-26-26-amendment-insertion-scope.md`)
- ✅ Actor dictionary + matcher (`archive/06-09-26-actor-labels.md`)
- ⏸️ GPU pod for SLM on the new actors (next pod session, with the ~275 pending_slm)
- ✅ Legal snapshot + TaxaSubscriber at publish time

## Root cause findings (2026-09-26)

- **Cause A: actor rows lost after parse, not a dictionary gap.**
  - `UK_uksi_2015_398` had `governed/government_actors` on `legislation_text` (actors matched) but zero `provision_actors` rows.
  - Re-parsing restored 441 rows. The likely cause: `provision_actors` is `ON DELETE CASCADE` from `legislation_text`, so a LAT re-pull after parse wipes actors, and nothing re-parses.
- **Cause B (latent bug): parse swallowed upsert errors.**
  - `upsert_provision_actors(...).await.ok()` in `pipeline.rs` hid failures, and the upsert aborts on the first failing row, which would drop every remaining actor for the law.
  - The error is now printed (uncommitted). No failure seen on re-parse.
- **Coverage after the #57 re-parse (46 of the 93 were included):** 11 now have 10+ actors (fixed), 12 sparse, 23 zero. Not re-parsed: 39 zero, 8 sparse.
  - Actorless Obligation provisions: 556 → 230.
- **Next:** re-parse the other 47, reconcile, re-measure. What remains is the genuine dictionary/extraction gap.
- **Side find (legal's question on #57): law-level fitness fields are stale.**
  - The fields: `fitness_mention_count`, `fitness_applies/disapplies_count`, `fitness_entities`, `fitness_scope_dimensions`.
  - No pipeline command writes them; they're July one-off values resent every publish. Same shape as the #55 gaps.
- **Note:** the `UK_uksi_2015_398` re-parse rewrote its law-level DuckDB DRRP (regex). Restore excluded laws from the snapshot before any publish.
- **Downstream impact of the stale fitness fields (from legal):**
  - **Legal:** `has_fitness` is generated from `fitness_entities IS NOT NULL`, so 119 laws have a current tree but `has_fitness = false`. That skews making_funnel's `enrich` next_action (318 laws) and `check_enrichment_output`. Legal will make `has_fitness` derive from `compiled_applicability`, in its own session.
  - **Compliance:** the QQ benchmark screener is unaffected (it evaluates `compiled_applicability`), but the screening UI entity picklist (`EntityIndex.list_entities`) is built from the July `fitness_entities`. That matters for QQ-05 vocabulary.
  - A fractalaw law-level fitness roll-up fixes both.

## Fixes and results (2026-09-26)

- **Code:** `48c89b9`.
  - `normalize_family` fixes family gating: every family is emoji-prefixed, so specialist actors and fitness dictionaries never applied.
  - Regex patterns added to 11 trigger-only dictionary entries (Water Undertaker, Licence Holder, Young Person, …).
  - New Gvt: Mayor and Gvt: Authority: Combined County; Judiciary matches "Court".
  - Parse surfaces upsert errors.
- **Re-parse:** the 47 not re-parsed for #57, then all 93 with the fixes; reconciled 409 + 2,716 new rows.
  - Coverage: 20 laws 10+ actors, 24 sparse, 49 zero.
  - **Actorless Obligation provisions: 556 → 109 (17 laws).** The rest are impersonal/passive (#60) or in unsplit articles (#61).
- **Backfill + diff (44 laws with actors):** 20 same, **11 false→true** (PPC 2000, Water Supply (Water Quality) 2000, Renewables Obligation Order, …), 13 true→false.
  - **Jason: hold all 13 downgrades.** Evidence is incomplete: the Wester Ross MCO prohibition sits in an unsplit reg.4 row with empty `drrp_types`.
  - All 13 restored from the `fractalaw_pre_58_20260926.duckdb` snapshot. Zero-actor laws (49) restored too.
- **New issues:**
  - #59: law-level fitness roll-up;
  - #60: duties separated from holders; #16's Rule has regressed (0 provisions);
  - #61: unsplit multi-paragraph provisions (3,856 rows, 237 with modal but no DRRP).
- **Publish:** 31/31 laws, 0 errors, tenant dev. Legal snapshot `p58_taxa_snapshot_20260926` (baseline 3,587 is_making; expected +11).

## Follow-ups (after publish)

- **Legal verification:** 31/31 received; is_making 3,587 → 3,598; all 11 flips are source=enrichment; 0 true→false. Corrected the `held` column in `verdict_diff.csv` (5 rows had meant "previously held").
- **Trees for 2 new Making laws** (UK_ukpga_1933_13, UK_ukpga_1947_41) had never had fitness extracted. Ran extract → reconcile → application → compile. Both are territorial-only trees (extent fallback; parser limit, #56).
  - ⏸️ **Not published**, per Jason: deferred to next time (`--fitness-only`, legal snapshot first).
- **Legal's broader enrichment pass idea:** only 43 not-Making laws in legal have LAT and no verdict. 16 are in the hub: all small, none with actors or Obligation, and 5 with modals (UK_uksi_1971_162, UK_uksi_1969_1263, UK_ssi_2004_112, UK_ssi_2005_52, UK_ssi_2006_181). 27 were never handed off.
  - ⏸️ Re-run of the 5 deferred.
  - The ~15.6K legacy laws have no LAT; that's legal's handoff scope.
