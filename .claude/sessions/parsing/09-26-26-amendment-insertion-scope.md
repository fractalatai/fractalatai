---
session: Amendment Insertion Scope
status: closed
opened: 2026-09-26
closed: 2026-09-26
outcome: success

summary: >
  Fixed fractalatai #57: text an amending law inserts into another instrument now gets its own provision scope
  ('amendment') that no tier processes. 16,268 provisions in 448 laws were re-scoped; 1,280 Obligation signals and
  716 fitness mentions are excluded. After Jason's review, 410/413 laws were published with 0 errors; only 2 Making
  verdicts changed (both correct), since #55's interim guard had already done most of it.

decisions:
  - what: A distinct ProvisionScope::Amendment rather than folding into Structural
    why: 'Consumers should be able to tell amendment text apart, and Structural''s modal override was the leak'
    result: Scope assigned in taxa parse; every consumer filters on it; tier data is kept
  - what: Amendment text is a pipeline bug, not legal policy
    why: 'Jason: inserted text applies to the amended instrument; legal''s handoff filter is not foolproof'
    result: Laws that only insert text come back no_obligations regardless of the handoff
  - what: Exclude gold benchmark laws from the re-parse and review them manually
    why: Re-scoping would alter gold provisions
    result: 'Gold already distinguishes amendment text (58/64 labels none). One gold error (s.29) was corrected with Jason''s go-ahead, and one detection false positive (mixed operative provisions) was fixed'
  - what: Re-scope existing data with the existing pipeline (parse → reconcile → backfill)
    why: Existing pipelines first; parse always writes tier-0 scope rows, even on protected provisions
    result: 459 laws re-parsed; held/zero-actor laws restored from snapshot afterwards

metrics:
  rescope: { provisions: 16268, laws: 448, obligation_signals_excluded: 1280, fitness_mentions_excluded: 716 }
  reconcile: { new_rows: 3732, laws: 114, pending_slm_non_amendment: 275 }
  publish: { laws: 410, candidates: 413, errors: 0, true_to_false: 2, review_wins: 22, same: 386 }
  gold: { benchmark_laws_with_amendment_text: 17, amendment_provisions: 839, gold_labels: 64, gold_none: 58, corrected: 2 }

lessons:
  - title: The gold standard checked the detection, not the other way round
    detail: 'The manual gold review found that the "…; and accordingly … amended as follows" detection would scope out real duties (UK_asp_2005_13 s.12(1)). Gold was right. It also found one genuine gold error (s.29). Review benchmarks before trusting a new rule.'
    tag: methodology
  - title: Locator stems carry misleading duty words
    detail: '"In section 78E (Duty of enforcing authority…)–" produced an Obligation from the quoted section title. Locator stems ("In section X (…)—") are amendment text even without an instruction verb.'
    tag: data
  - title: taxa parse has law-level side effects
    detail: Re-parsing rewrites the regex law-level DRRP in DuckDB, which the backfill guard then keeps for zero-actor/held laws. Restore excluded laws from a snapshot after a bulk re-parse.
    tag: tooling
  - title: Stop and restart a long run when a rule changes mid-flight
    detail: The first re-parse ran with the old binary after the mixed-provision fix. Parse is idempotent, so stopping and restarting was cheaper than patching 38 laws afterwards.
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/taxa/amendment.rs
  - crates/fractalaw-core/src/taxa/mod.rs
  - crates/fractalaw-cli/src/commands/pipeline.rs
  - crates/fractalaw-cli/src/commands/fitness.rs
  - crates/fractalaw-store/src/pg.rs
  - scripts/ml/runpod_fitness_batch.py
  - data/qq-readiness/amendment/P57-publish-log.md
  - data/qq-readiness/amendment/verdict_diff.csv
  - data/qq-readiness/amendment/gold_s29_before_20260926.csv
  - /mnt/ssd/fractalaw-backups/fractalaw_pre_57_20260926.duckdb

depends_on:
  - 09-25-26-law-level-rollup.md

enables:
  - 'QQ-01 62-law handoff (still needs #58 actor model)'
  - 'fractalatai #58 actor-model session'
---

# Session: Amendment Insertion Scope (CLOSED)

## Problem

Text an amending law inserts into another instrument ("after paragraph (aa) insert— …must ensure…") belongs to the amended instrument, but the pipeline processes it as the amending law's own. Amendment provisions are `Structural`, but two things leak:
- `has_drrp_modal` promotes them to `Substantive` whenever the inserted text has "shall/must";
- sub-provisions under an "insert—" stem aren't recognised as amendment text at all.

Scale: 14,012 provisions in 419 laws, carrying 642 Obligation signals and 684 fitness mentions. fractalatai #57 (Jason: a pipeline bug, not legal policy).

## Todo

- ✅ Move amendment detection into a shared `taxa/amendment.rs` (+ `is_amendment_text`, locator-stem rule)
- ✅ New `ProvisionScope::Amendment` (no modal override); `taxa parse` assigns it to instructions and their sub-provisions, and extracts no DRRP/actors from them
- ✅ Consumers skip amendment scope: backfill_from_actors, significance profile/parts, DRRP roll-up, fitness extract/compile/application clauses, `runpod_fitness_batch.py` (the SLM and significance scripts already select `substantive`)
- ✅ Tests on real examples + controls (CDM, Plant Protection Products not over-scoped)
- ✅ Existing data: re-parsed 459 laws (17 benchmarks excluded, manually reviewed), reconciled 3,732 new rows (114 laws), backfilled; held/zero-actor laws restored from snapshot
- ✅ Before/after: 16,268 provisions / 448 laws now amendment scope; 1,280 Obligation signals + 716 fitness mentions excluded; application + trees recompiled (412 + 16 repaired)
- ✅ Verdict diff vs legal (after #55): 413 candidates → 386 same, 22 review wins, 2 true→false (both locator-stem-only obligations, correct)
- ✅ Published 410/413 (0 errors; 3 regnal-year names not in DuckDB), with legal snapshot `p57_taxa_snapshot_20260926` (`amendment/P57-publish-log.md`)

## Dependencies

- ✅ #55 law-level roll-up in `taxa backfill` (`qq-data-readiness/09-25-26-law-level-rollup.md`), which already carries the interim amendment guard
- ✅ Scope system (`ProvisionScope` Out/Structural/Substantive in `fractalaw-core/src/taxa/mod.rs`)
- ✅ Legal snapshot + TaxaSubscriber running at publish time

## Implementation (2026-09-26)

- **Detection:** `fractalaw-core/src/taxa/amendment.rs` (moved from `law_drrp.rs`). A provision is amendment text if it is an instruction (insert/substitute/omit/"is amended"/"there is substituted"/"has effect as if … substituted", or a locator stem "In section 34 (…)—") or sits under one (ancestor section_ids). 14 tests.
- **Scope:** `ProvisionScope::Amendment` ("amendment"). `pipeline::parse_provisions` builds a section_id→text map per law and records amendment provisions like `Out` (no DRRP/actors); parse always writes them (tier 0, not protected). Existing tier data on those provisions is kept; consumers filter on scope.
- **Examples after parse + reconcile + backfill:**

  | Law | Amendment / total | Verdict |
  |---|---|---|
  | UK_ssi_2012_148 | 24/32 | no_obligations |
  | UK_ssi_2011_226 | 57/81 | no_obligations |
  | UK_uksi_2021_511 | 170/263 | no_obligations |
  | UK_ssi_2005_22 | 57/85 | empowering (own reg.8 exemptions) |
  | UK_uksi_2008_198 | 5/10 | no actors, unchanged |
  | **Control:** CDM UK_uksi_2015_51 | 0/474 | making (253 duties) |
  | **Control:** PPP UK_uksi_2012_1657 | 1/278 (reg.32(1) "are amended") | making |

- **Order matters:** parse writes new regex actor rows (NULL method), so reconcile must run before backfill. The unreconciled guard leaves the law unchanged otherwise.
- **Benchmarks (17 with amendment text)** are excluded from the re-parse: gold provisions, a separate decision.
- **DuckDB snapshot:** `/mnt/ssd/fractalaw-backups/fractalaw_pre_57_20260926.duckdb`.

## Gold benchmark check (manual review requested by Jason)

17 benchmark laws contain amendment text and are **excluded from the re-parse** (gold provisions). Of 839 benchmark provisions detected as amendment text, gold has 64 actor labels: 58 are none (57 "mentioned"), 4 Obligation, 2 Liberty. **The gold standard does distinguish amendment text.** All 6 non-none labels are in `UK_asp_2005_13`:
- **s.12(1), s.13(1):** Obligation (4 labels). These are **mixed operative provisions** ("…are to be free of charge; and accordingly those Acts are amended as follows"). Gold is right; the *detection* was wrong.
  - Fixed: `MIXED_OPERATIVE_RE` means an operative clause followed by "; and accordingly … amended" is not amendment text.
  - The first re-parse was stopped (38 laws done with the old binary) and restarted with the fix.
- **s.29:** Liberty (2 labels). Inserted power ("after paragraph (d) insert '…may be excepted … by regulations'"), which belongs to the 2001 Act. **For Jason's manual check.**
- **Jason's review of s.29:** an error in the gold standard (genuinely inserted text). **Corrected 2026-09-26** with his go-ahead: both rows (Scottish Ministers (implied), services) Liberty → none/mentioned. Before values in `data/qq-readiness/amendment/gold_s29_before_20260926.csv`.

## Results (2026-09-26)

- **Code:** `87fa819`.
- **Re-scope:** 16,268 provisions in 448 laws, more than the 14,012 estimate thanks to the locator-stem rule.
- **Reconcile:** 3,732 new regex rows (114 laws). 275 non-amendment actors are `pending_slm`; no pod is needed now (the roll-up ignores them).
- **Excluded from all tiers' consumption:** 1,280 Obligation signals (164 laws) and 716 fitness mentions.
- **Verdict diff:** only 2 changes, because #55's interim guard had already excluded most amendment text:
  - `UK_ssi_2005_658` (Contaminated Land (Scotland) Regs 2005);
  - `UK_uksi_1998_1856` (Private Water Supplies (Scotland) Amendment Regs).
  
  In both, the only Obligation was on a locator stem, e.g. "In section 78E (Duty of enforcing authority…)–". Both are amending instruments, so no_obligations is correct.
- **Publish candidates:** 413 laws (the re-parsed laws minus held/zero-actor), in `data/qq-readiness/amendment/publish_candidates.txt`.
