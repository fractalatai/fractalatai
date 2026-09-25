---
session: QQ Data Readiness
status: closed
opened: 2026-09-25
closed: 2026-09-25
outcome: success

summary: >
  Fractalaw side of sertantai-legal #161 (QQ go-live). Investigated the no-duty and Rights-only laws (T1/T2).
  Rebuilt the fitness tree compiler with law application as a new concept distinct from extent (T3; spec v2.4 agreed with legal).
  Ran T4 through existing pipelines: 628 laws published with 0 failures, no Making downgrades, and tree defects L1/L2/L3/L5 → 0 and L7 538 → 1.

decisions:
  - what: Extent and application are distinct; fractalaw owns and publishes application
    why: 'geo_extent mixed four legislation.gov.uk sources, and unrevised docs carry a placeholder E+W+S+N.I. An "(England)" SI extends E+W but applies in England, and the screener needs application.'
    result: 'sertantai-legal #162 (extent fixes) and #163 (ZENOH-SPEC v2.4 application_regions/source/evidence); root territorial gate in every tree'
  - what: Fix tree defects at compile time in fractalaw-core, not by re-extraction
    why: No RunPod needed; deterministic and testable; the defects came from ungrounded ft entities, procedural sentences and missing reconcile
    result: '58 tests; lint on 562 trees: L1 330→0, L2 52→0, L3 6→0, L4 256→10, L5 156→0, L7 538→1, whole-law Not 319→25'
  - what: Use existing pipelines only for T4; drop the new taxa aggregate command
    why: 'Jason: fractalaw does two things here (improve the fitness parser, parse missed laws) and the existing pipelines are proven. Legal''s asks are filtered through what fractalaw does.'
    result: New law-level DRRP code deleted uncommitted; the roll-up gap raised as fractalatai #55
  - what: Republish the 566 trees with --fitness-only (no DRRP columns)
    why: Law-level DuckDB DRRP is regex-only (#55), and legal's resolver treats any published DRRP as an authoritative enrichment verdict
    result: 'Legal-side: 628 received = 628 updated; only UK_ukpga_1947_48 flipped (f→t); no downgrades'
  - what: Hold 3 f→t laws back from DRRP publish (UK_ssi_2010_434, UK_eur_2008_307, UK_ssi_2011_226)
    why: Government-only duties, or duties inserted by an amending SI (Jason's policy that amending SIs are not Making)
    result: Published fitness-only; they keep legal's is_making=false
  - what: Generic codes (L6) are a fitness-parser bug, not org facts
    why: The codes come from procedural "applies" sentences (offences by bodies corporate, hearings); dropping or implying them just makes laws universal
    result: 'fractalatai #56: only 11% of 10,637 AppliesTo mentions have the law or a Part as subject'

metrics:
  t1_no_duty_laws: { total: 37, a_no_duties: 33, b: 0, c_not_backfilled: 2, d: 0, e_lat_gap: 2 }
  t2_rights_only: { wrong: 2, correct_label_wrong: 5, borderline: 1 }
  legal_lat_gap: { laws_with_schedules: "84/910", insert_without_text: 3483 }
  extent_mislabels: { devolved_making_tagged_uk: 425, sample_unrevised_placeholder: "20/24" }
  fitness_grounding: { construction_mentions: 1085, construction_grounded: "2-4%", other_entities_grounded: "~94%" }
  unreconciled_mentions_found: { mentions: 1714, laws: 136 }
  drrp_tier_divergence: { actor_rows: 158225, changed_by_later_tiers: 47716, regex_missed_obligations: 14722, laws_pg_duty_duck_none: 44, laws_duck_duty_pg_none: 179 }
  t4_publish: { drrp_laws: 62, fitness_only_laws: 566, provisions: 32638, provision_laws: "63/65", failures: 0, making_flips: 1 }
  runpod: { gpu: "RTX PRO 4000 Blackwell", ft_fitness: "1112/1114, 174s", position_slm: "10720 actors, 34 min, 0 errors" }
  qq_benchmark_t4: { overmatch: "-51", caveats: "32→4", left_both: 47 }

lessons:
  - title: '"Stale" DRRP meant fractalaw''s own law-level roll-up lagged its provision tiers'
    detail: 'write_law_taxa rolls up the regex pass only; the 2026-06-28 reconciliation work added provision-level backfill but never a law-level roll-up. So DuckDB (and legal) only ever received regex-quality DRRP: 30% of actor rows changed by later tiers. I called it "by design" before checking git history; it was an ordering gap (#55).'
    tag: architecture
  - title: Check existing workflows before writing new pipeline code
    detail: 'I built a taxa aggregate command to satisfy legal''s "DRRP from provision_actors" request. Jason stopped it: parse/classify/reconcile/backfill have been run many times and are the right tools. Filter a peer repo''s asks through what fractalaw actually does. The eventual change was a 20-line --fitness-only flag on existing publish.'
    tag: methodology
  - title: The fine-tuned fitness model hallucinates a default label
    detail: 'gemma3-fitness emits "construction"/"construction work" for unrelated provisions (2-4% lexically grounded). Legal assumed statutory "construction" sense. A cheap lexical grounding check (provision + sub-provisions + law title, citations stripped) removes most model noise.'
    tag: models
  - title: Always run reconcile after any fitness extraction batch
    detail: 'Batches on 07-14/07-29 wrote tier columns but entities stayed NULL, silently shrinking 136 laws'' trees. Added `fitness reconcile`. Reconcile only fills NULL rows, so after a later ft batch you must clear regex-reconciled entities first (560 rows here).'
    tag: data
  - title: legislation.gov.uk puts a placeholder extent on unrevised legislation
    detail: 'DocumentStatus=final docs have no law-level RestrictExtent and E+W+S+N.I. on every ContentsItem; legal''s fallback turned that into "UK" for ~425 devolved laws. Revised docs are reliable; LAT provision extent is empty (honest) when unrevised.'
    tag: data
  - title: A peer session's relayed approval is not the user's approval
    detail: Legal's session reported Jason approved T4. I held until Jason confirmed in this session before the outward publish; he then refined the plan (5-step with review gate).
    tag: methodology
  - title: taxa_hash excludes fitness/tree fields
    detail: 'publish --changed never republishes tree changes. UK_ukpga_1990_9 had a 12 KB tree in DuckDB that never reached legal (not a size limit). Use explicit --laws lists.'
    tag: tooling
  - title: pkill -f over ssh can kill the ssh session itself
    detail: 'pkill -f "ollama serve" matched the remote bash -c command line containing the same text. Use bracket patterns ([o]llama) and keep kill and start in separate ssh calls.'
    tag: infrastructure
  - title: LRT merge never refreshed extent_code
    detail: 'merge_legislation mapped title_en/family_ii only; sertantai sends geo_extent/geo_region, so DuckDB extent stayed at the original seed. Mapped geo_extent/geo_region/geo_extent_source → extent_code/extent_regions/extent_source.'
    tag: data

artifacts:
  - crates/fractalaw-core/src/taxa/applicability.rs
  - crates/fractalaw-core/src/taxa/applicability_compile.rs
  - crates/fractalaw-core/src/taxa/application.rs
  - crates/fractalaw-cli/src/commands/fitness.rs
  - crates/fractalaw-cli/src/main.rs
  - crates/fractalaw-sync-cli/src/sync.rs
  - crates/fractalaw-sync-cli/src/main.rs
  - crates/fractalaw-store/src/duck.rs
  - scripts/maintenance/lint_trees.py
  - .claude/skills/fitness-pipeline/SKILL.md
  - data/qq-readiness/t1-no-duty-findings.csv
  - data/qq-readiness/t2-rights-only-verdicts.csv
  - data/qq-readiness/t3-tree-lint-before-after.csv
  - data/qq-readiness/t4/T4-publish-log.md
  - /mnt/ssd/fractalaw-backups/fractalaw_pre_t4_20260925.duckdb
  - 'GitHub: sertantai-legal #162, #163; fractalatai #55, #56'

depends_on:
  - 07-13-26-fitness-expression-compiler.md
  - 07-13-26-fitness-reconcile-publish.md

enables:
  - 'fractalatai #56: fitness parser takes real application clauses only (L6 fix; validation set sertantai-legal worklists/07-generic-code-gates.csv)'
  - 'fractalatai #55: law-level DRRP roll-up from reconciled provision_actors in taxa backfill'
  - T4 step 4 for the 35 laws awaiting LAT from legal
  - Legal review of 90 territory-only (L8) trees
---

# Session: QQ Data Readiness (CLOSED)

## Problem

QQ loses its ENHESA register on 31 Oct 2026; sertantai-compliance v0.1 ships ~27 Oct. The applicability screener is now data-limited, and much of that data is fractalaw output (`duty_type` → `is_making`, `compiled_applicability` trees). Fractalaw fixes are due ~6 Oct, then one combined re-enrichment + publish. Brief: `~/Desktop/sertantai-legal/.claude/plans/qq-fractalaw-brief.md` (sertantai-legal #161).

## Todo

- ✅ T1 — Investigate 37 laws with a tree but no `duty_type` → CSV `law_name, finding (a|b|c|d), evidence, fix` + counts
- ✅ T2 — Spot-check 8 Rights/Powers-only laws → verdict per law, one-off vs pattern
- ✅ Report T1/T2 to Jason before any code change
- ✅ Untangle geo extent sources (prerequisite for L7)
- ✅ Raise legal extent findings + fixes → sertantai-legal #162 (cross-linked on #161)
- ✅ Design `application` field + ZENOH-SPEC v2.4 proposal → sertantai-legal #163 (awaiting legal columns/subscriber)
- ✅ Implement application derivation (`taxa/application.rs`, `fitness application`) + L7 root gate
- ✅ T3 — Tree compiler fixes L1–L5, L7 with tests + before/after counts (L6 on hold; L8 list for legal review)
- ✅ T3 — Whole-law `Not`: only law-level disapplications kept (317 → 25)
- ✅ T4.5 — `UK_ukpga_1990_9`: DuckDB has a 12 KB tree, never reached legal (not size; `taxa_hash` ignores trees); goes in T4
- ⏸️ DRRP for publish: aggregate from PG provision_actors (deferred: Jason chose existing pipelines for T4; roll-up gap raised as fractalatai #55, post go-live). Dry-run verdict diff done for the fresh 65
- ✅ Add `application_*` to publish payload once #163 is ready on legal (`43f782b`)
- ✅ T4 — Combined enrichment + publish: 62 DRRP + 566 fitness-only + provisions for 63/65, 0 failures (the 35 laws awaiting LAT are deferred)
- ✅ Hand back: T1 CSV, T2 verdicts, T3 fix list + counts, T4 publish log, contradictions with legal's funnel

## Dependencies

- ✅ Fitness expression compiler (`fitness/07-13-26-fitness-expression-compiler.md`)
- ✅ Fitness reconcile/publish (`fitness/07-13-26-fitness-reconcile-publish.md`)
- ✅ Legal `geo_extent` fix (#162, sertantai-legal `106962e`)
- ✅ Legal snapshot of `compiled_applicability` before T4 publish (t4_taxa_snapshot_20260925)
- ⏸️ Legal's final list for T4 step 4 (deferred: 35 laws still awaiting LAT parse on legal's side)

## T1 — 37 laws with tree but no duty_type

Output: `data/qq-readiness/t1-no-duty-findings.csv` (gitignored). Sources: DuckDB `legislation`, fractalaw PG hub (`provision_actors`), legal `legal_articles` (LAT, read-only).

**Counts: a = 33, b = 0, c = 2, d = 0 (1 partial, inside a c), e = 2** (e = a new category: duties exist but are in inserted text that is missing from LAT)

- **None of the 37 has any DRRP output in DuckDB.** Pipeline state: 25 triage `not_making` (gated before Pass 2), 2 `uncertain`, and 10 with no triage and `enrichment_pending=true` (queued, never completed).
- **Text review:** 33 are amending, commencement, exemption, definitional, fee or designation instruments with no duties on regulated persons in their own text. Empty `duty_type` is correct, and so is fractalaw's triage.
- **c:** `UK_ssi_2012_148` (Waste (Scotland) Regs 2012 inserts the EPA s34(2E)–(2L) occupier duties) and `UK_uksi_1998_3111` (Authorised Weight prohibition + offence). PG has Obligation rows; DuckDB was never backfilled. 1998_3111 also shows a DRRP gap: the agentless passive prohibition "no vehicle … shall be used on a road" has no actor, so it is missed.
- **e:** `UK_uksi_2008_198` (inserts TA 1968 ss97C–G) and `UK_uksi_2016_1245` (inserts CA 2006 s414CA–CB). The provisions read "insert—" with no body.
- **Contradicts brief:** `UK_uksi_2025_140` only disapplies/defers EPA s45A–45AZB conditions (micro-firms to 2027), so it has no duties. `UK_uksi_2018_24` only amends offence/enforcement machinery.

## T2 — 8 Rights/Powers-only laws

Output: `data/qq-readiness/t2-rights-only-verdicts.csv`.

- **Wrong (2):** `UK_ssi_2010_435` has a genuine duty (reg.5(1) "a person has a duty to provide information to SEPA"). `UK_ssi_2005_22` has an offence-backed registration duty in substituted text.
- **Correctly not Making (5), but the label is wrong:** `UK_uksi_2006_3368` (the signs duty is in HA 2006 s6 + SI 2007/923, not here; this SI defines "enclosed" and designates enforcers) and `UK_uksi_2006_2950` (enabling order, so Power). `2005_1904`, `2014_549` and `2024_666` are also label noise.
- **Borderline (1):** `UK_uksi_2007_765`. reg.11 (smoke-free vehicles with under-18s) triggers the Act's duties.
- **Pattern: not "must display" misses.** Rather:
  1. **Stale DuckDB aggregates.** These come from the pre-hub LanceDB regex pass (2026-07-11, `section/N` article refs) and were never refreshed from PG `provision_actors`. Corpus-wide, 24 laws have PG Obligation rows but no duty in DuckDB (16 empty, 8 Liberty-only).
  2. **Actor false positive:** "Commission Decision" and "Competition Commission" map to `EU: Commission`.

## Cross-repo findings (for legal)

- **LAT gap:**
  - Schedules are present for only 84 of 910 laws in `legal_articles`.
  - 3,483 provisions end "insert—/substitute—" with the inserted text missing.
  - DRRP cannot find duties that aren't in the text (T1 e, and likely many more).
- **Legal's Making funnel versus fractalaw triage:** most of the 37 are amending instruments whose duties belong to the principal Act/SI. Legal's own `making_classification` also says `not_making` for 18 of them (4 `uncertain`, 15 unclassified).

## Geo extent: untangling the sources (before L7)

Four sources are in play, and they get mixed:

| # | Source | Where it lands | Reliability |
|---|---|---|---|
| 1 | Law-level `Legislation/@RestrictExtent` (metadata XML) | legal `md_restrict_extent`; `geo_extent` via `metadata.ex` | Good for **revised** docs. Absent for unrevised (`DocumentStatus=final`) |
| 2 | `ContentsItem/@RestrictExtent` (contents XML) | legal `geo_extent`/`geo_region`/`geo_detail` via the `staged_parser` extent stage (the fallback "first ContentsItem" overwrites #1) | **Placeholder `E+W+S+N.I.` on every item of an unrevised doc** |
| 3 | Per-provision extent in body XML | legal `legal_articles.extent_code` | Correct when revised; **empty** when unrevised (no placeholder) |
| 4 | Prose: "These Regulations extend to…" / "apply in relation to England only" / title "(Wales)" | fractalaw fitness Place mentions → territorial `Match`/`Not` in trees | Only source of **application** as opposed to extent |

- **Fractalaw copies legal's values.** DuckDB `legislation.extent_code/extent_regions/extent_detail` are copied from legal's `geo_extent/geo_region/geo_detail` via LRT, so they carry the same errors. The tree compiler (`fitness.rs::compile_law`) reads **none** of them; its jurisdiction nodes come only from #4.
- **Sample of 24 in-force Making `ssi/wsi/nisr` tagged `UK`:**
  - 20 are unrevised: no law-level extent and a placeholder on every item. Legal's fallback turns this into `UK`.
  - 4 are revised with the correct law-level extent (`E+W`, `N.I.`), but legal still stores `UK`, so the value is stale. `UK_ssi_2013_286` (site: `S`) and `UK_uksi_2025_140` (site: `E+W`) are the same case.
- **Dev DB in-force Making `UK` mislabels:** nisr 157, ssi 175, wsi 92, anaw 1. Another 136 ssi/wsi have null extent.
- **LAT #3 vs `geo_extent`:** across 611 laws, 561 agree; about 50 disagree, mostly `geo_extent=UK` where #3 is narrower.
- **Text coverage:** all 910 LAT laws have at least one signal (605 have #3; 294 have an extent clause; 73 have an application clause).
- **Extent ≠ application.**
  - Extent is the legal system: Welsh and English SIs extend `E+W`.
  - Application is where the law operates: `UK_uksi_2006_3368` and `UK_uksi_2025_140` extend `E+W` but apply to England only; a WSI extends `E+W` but applies in Wales.
  - The screener needs application.

**Decisions (Jason, 2026-09-25):**
- Extent and application are distinct; this hadn't been recognised before.
- The screener needs **application**.
- **Fractalaw owns application and publishes it** as its own field, which needs a ZENOH-SPEC change.
- Legal owns extent. The findings and fixes were raised as sertantai-legal #162 and cross-linked on #161.

## T3 — Tree compiler fixes

The compiler moved into `fractalaw-core/src/taxa/applicability_compile.rs` (pure, 58 tests with `application.rs` and `normalize`). The CLI `fitness compile` gained `--out` (JSONL dry run). New commands: `fitness reconcile` and `fitness application`. Lint: `scripts/maintenance/lint_trees.py` (mirrors QQ-03; `--jsonl` overlay).

**Before/after** over 551 Making, not-revoked trees (`data/qq-readiness/t3-tree-lint-before-after.csv`). The after-state is a JSONL dry run; DuckDB and legal are untouched until T4.

| Defect | Before | After | Fix |
|---|---|---|---|
| L1 duplicate siblings / single-child | 326 | 0 | `ApplicabilityNode::normalize` |
| L2 past / inverted `to` | 51 | 0 | `from` = earliest law-level commencement; `to` only from a law-level sunset (none exist in the corpus) |
| L3 Not on own jurisdiction | 6 | 0 | jurisdiction codes lifted out of branches |
| L4 `construction` (in Not) | 253 (52) | 10 (0) | grounding: code must appear in provision + sub-provisions or the title, after stripping citations; interpretation and vehicle/product senses dropped |
| L5 gov actor gating (any use) | 155 (230) | 0 (0) | government actor codes dropped (Making laws only; a gov-only mention = lost regulated person) |
| L7 no jurisdiction gate | 528 | 9 | root `territorial` gate = application (9 laws have no title/extent/clause: `t3-l7-no-application.txt`) |
| L8 territory-only | 49 | 82 | **up**: gate-only trees where the rest was hallucinated/gov codes. List for legal review: `t3-l8-territory-only.txt` |
| Whole-law Not | 317 | 25 | only law-level disapplications |
| L6 generic codes | 228 | 225 | on hold (ownership) |

**Findings along the way:**
- **L4 root cause is not the interpretation sense.** The fine-tuned fitness model emits `construction` / `construction work` for unrelated provisions: 1,085 mentions, 2–4% lexically grounded, ~100% from the ft tier alone. Other entities are ~94% grounded.
- **Reconcile had never been re-run after the 07-14 / 07-29 extraction batches.** 1,714 mentions (136 laws) had tier output but empty `entities`, silently shrinking trees.
  - `fitness reconcile` added and run. Only NULL rows were filled; tier columns untouched.
  - Row ids in `data/qq-readiness/reconcile-20260925-null-entity-ids.txt` allow a revert.
- **15 in-scope laws have no mentions left.** They are repaired from the existing JSON (text-free fixes + gate).
- **Title fallback:** 91 in-scope laws have no DuckDB title, so the compiler uses the law's own citation provision.
- **CDM 2015:** its actual application provision (reg.3) was never a fitness mention. It keeps `construction` via title grounding; its application comes from the reg.3 text clause (GB).
- **`taxa_hash` excludes fitness/tree fields,** so `publish --changed` never publishes tree changes.
- **Pre-existing failing test:** `taxa::tests::mixed_content_provision_employer_duty_extracted` also fails on HEAD. It is unrelated to this work.

## Legal TaxaSubscriber contract (QQ-01a, received 2026-09-25)

Legal's resolver now decides `is_making`. Null DRRP columns carry no verdict; empty lists or an empty payload mean `no_obligations`. Enrichment outranks triage and legacy data. Policy: amending instruments that insert duties are **not** Making. Brief section: "Legal's TaxaSubscriber contract".

Gaps on the fractalaw side, to be fixed before T4:
- **`pipeline.rs::write_law_taxa` NULLs DRRP when nothing is found.** This must become empty lists, but only for laws where DRRP actually ran.
- **No code builds law-level DRRP from reconciled `provision_actors.drrp`.** DuckDB aggregates come from the enrich pipeline's regex pass. A new aggregation step is needed.
- **The aggregation must exclude amendment-instruction provisions** (insert/substitute), per the policy. This covers T1 "e" plus `UK_ssi_2012_148` and `UK_ssi_2005_22`.
- **PG Obligation false positives** (`UK_ssi_2005_63`, Covid PPE laws) would become Making verdicts. Plan: a dry-run verdict diff against legal's `is_making`, for review before publish.

Acknowledged to the legal session by cross-session message.
- **Legal confirmed (2026-09-25):**
  - Empty lists only where DRRP actually ran.
  - Amending-SI handling agreed. Recorded in legal QQ-01: `UK_ssi_2012_148` and `UK_ssi_2005_22` not Making; `UK_uksi_1998_3111` and `UK_ssi_2010_435` stay Making.
  - #163 is a QQ-04 blocker: legal adds the application columns before fractalaw publishes them.
- **The dry-run verdict diff must compare against `is_making_source` as well as `is_making`** (legal dev view `making_funnel`: source, reason and stage per law).
  - `review` (human) laws never change.
  - `legacy_drrp` laws flip only if fractalaw sends an enrichment verdict.

**2026-09-25:** T3 committed as `d9c62fe`. Waiting on legal's fixes:
- #162 extent fixes
- #163 application columns + TaxaSubscriber
- QQ-01a resolver deploy
- snapshot and Phoenix server running before T4

The DRRP aggregation work (next todo) is also on hold until Jason gives the go-ahead.
- **Legal #162 done (sertantai-legal `106962e`).** `geo_extent` is now resolved by source. The new `geo_extent_source` column takes `law_level | lat_provisions | contents_items | text_clause | type_code`; NULL means legacy/unverified, an upper bound only.
  - The LRT queryable doesn't send `geo_extent_source` yet. I've asked legal to add it (with #163).
  - Impact: 51 in-scope laws derive application from LRT extent. 40 of them are NULL-source `UK`, which gives a UK-wide gate as the upper bound.
  - After legal ships it: re-pull LRT for these laws and mark legacy fallback in `application_evidence`.
- **Legal #163 ready (sertantai-legal `8d96d27`, ZENOH-SPEC 3.1).** Fractalaw commit `43f782b`: publish sends `application_*`; the LRT merge now maps `geo_extent`, `geo_region` and `geo_extent_source` (it previously never refreshed `extent_code`); a verified LRT extent outranks LAT extents.
  - **T4 prep order:**
    1. Legal starts its server.
    2. `sync pull-lrt` for all laws in scope, to get the #162 extents and sources.
    3. `fitness application`.
    4. `fitness compile`.
    5. DRRP aggregation.
    6. Publish.

## T4 — decision: existing pipelines only (Jason, 2026-09-25)

- **What fractalaw is doing in T4:** (1) the improved fitness parser (T3), and (2) parsing the laws that were missed. The existing pipelines cover both. Legal's asks are filtered through what fractalaw does.
- **`taxa aggregate` dropped.** It was a new law-level DRRP built from `provision_actors`; it was never committed or run, and has been removed.
- **DRRP comes from the standard pipeline:** `taxa parse` → dep features → embed → classify → infer → reconcile. The law-level summary is `write_law_taxa`, as in every previous publish.
- **Amending-SI laws aren't in the T4 lists:** `UK_ssi_2012_148`, `UK_ssi_2005_22`, `UK_uksi_2008_198` and `UK_uksi_2016_1245`. So no conflict with legal's QQ-01 decisions.
- **Progress:**
  - Done: backups (Parquet + full DuckDB copy on the SSD); LRT pull 628/628; LAT pull 27 laws (19,635 provisions); parse 58/58; dep features 10,730 actors.
  - In progress: embed 17,317 provisions.

## T4 — published (2026-09-25)

**Pipeline** (all existing commands):
1. LRT pull 628 laws.
2. LAT pull 27 laws.
3. `taxa parse` 58, dep features, embed 17,317, classify, infer, reconcile, backfill (65 laws).
4. `fitness extract` for 36 laws.
5. RunPod (RTX PRO 4000 Blackwell): ft fitness 1,112/1,114 → re-reconcile 560 mentions (ids in `t4/rereconcile_ids.txt`).
6. `fitness application` → `fitness compile` into DuckDB.

**Lint on the 562-tree scope:** L1 330→0, L2 52→0, L3 6→0, L4 256→10, L5 156→0, L7 538→1, Not 319→25, L8 53→90.

**Publish:**

| Publish | Laws | Failures | Notes |
|---|---|---|---|
| DRRP | 62 | 0 | Only flip: `UK_ukpga_1947_48` f→t |
| `--fitness-only` (commit `7bd5938`) | 566 | 0 | Includes held-back `UK_ssi_2010_434`, `UK_eur_2008_307`, `UK_ssi_2011_226` |

Log: `data/qq-readiness/t4/T4-publish-log.md`; sent to legal.

**Raised:** fractalatai #55. Law-level DRRP is rolled up from the regex tier only (post go-live).

**Still open:**
- Position SLM on the pod (10,720 actors in 27 laws), then reconcile, backfill, and a provision-level publish decision.
- Stop the pod.
- The 35 laws awaiting LAT from legal.
- **Raised fractalatai #56: fitness parser AppliesTo comes mostly from procedural "applies" sentences.**
  - Only 11% of 10,637 AppliesTo mentions have the law or a Part as subject; 53% are provision-level ("paragraph (1) applies").
  - This is the root cause of L6 (generic codes). Jason: the focus is the parser, taking real application clauses only.
  - Legal's QQ benchmark after T4: over-match -51, caveats 32→4, 47 left "both" (generic_code_gate + material_condition_miss).
- **Position SLM done** (10,720 actors, 0 errors). Reconcile left `pending_slm` = 0, then backfill. **Provisions published:** 32,638 across 63/65 (2 have no actors). The pod can be stopped.
- **L6 decided (Jason, via legal):** generic codes are fitness-parser bugs in fractalaw. Tracked in #56.
  - Legal evidence: `sertantai-legal/.claude/sessions/qq-data-readiness/worklists/07-generic-code-gates.csv` (94 QQ laws; 67 with suspect codes).
