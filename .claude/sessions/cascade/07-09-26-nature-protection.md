---
session: Nature Protection
status: active
opened: 2026-07-09
---

# Session: Nature Protection (ACTIVE)

## Problem

QQ customer is asking questions about nature protection provisions. QA eyeballing reveals laws at inconsistent pipeline stages — some missing LAT entirely (NERC Act 2006), others with SLM results that were never reconciled (Marine and Coastal Access Act 2009 s.126). The corpus needs a systematic sweep to identify and fix gaps across all tiers.

Examples found during ad-hoc QA (2026-07-08):
- **UK_ukpga_2006_16** (NERC Act 2006) — zero provisions in Postgres, no LAT pulled, family = "X: No Family"
- **UK_ukpga_2009_23** (MCAA 2009) — SLM ran correctly (Ind: Person as active/Obligation on s.126(6)-(7)) but reconcile never ran, so final position/drrp columns are empty

## Corpus Status (428 laws)

| Stage | Count | Notes |
|-------|-------|-------|
| published | 239 | Done — some need reconcile fixes (e.g. MCAA 2009) |
| ready_to_publish | 10 | Quick win — just need `sync publish` |
| needs_validate | 41 | Need LLM validation |
| needs_classify | 8 | Skipping classifier — send straight to SLM |
| needs_lat | 130 | No provision text (79 Scottish, 13 Welsh, 23 UK SI, 4 UK Act, misc) |

Register grew from 274 to 428 laws. The 130 needing LAT are mostly new Scottish/Welsh SIs.

## Pipeline (simplified)

Previous sessions proved SLM outperforms the regex -> classifier -> SLM chain (SLM 79.7% position / 92.5% DRRP vs classifier 59.9%). Classifier is being dropped. The pipeline for this session:

1. **Pull LAT** from sertantai for 130 missing laws
2. **Triage** (regex) — identify making laws with obligations
3. **Embed** — compute embeddings for new provisions
4. **SLM** (RunPod) — classify actors for laws with obligations (130 new + 8 needs_classify)
5. **Reconcile + backfill** — land SLM results into final columns (includes fixing published laws like MCAA 2009)
6. **Publish** — all unpublished laws to sertantai (10 ready + newly processed)

## Work

1. ✅ Wire `customer-laws` CLI command into `fractalaw-sync-cli` (two-step: discover customers, fetch laws)
2. ✅ Pull latest QQ register from sertantai: **428 laws** (was 274). UUID: `c075d56b-8420-4408-b695-ccfbc1ba15ec`
3. ✅ Run `taxa status` across full 428-law corpus
4. ⬜ Publish 10 ready_to_publish laws (quick win)
5. ✅ Pull LAT via sync-watch (~106 laws landed), manually pulled NERC 2006 + Factories Act
6. ✅ Triage new laws — sync-watch triaged on ingestion, published results back to sertantai
7. ✅ Parse new laws (93 batch + 4 nature protection laws) — regex DRRP extraction
8. ⬜ Embed new provisions (93 laws ready, scope bug fixed)
9. ⬜ SLM batch on RunPod — new laws + 8 needs_classify
10. ⬜ Reconcile + backfill all laws with unreconciled SLM results (including published laws like MCAA 2009)
11. ⬜ Publish all updated laws to sertantai
12. ✅ Spot-check nature protection provisions (s.40 NERC, s.125-126 MCAA, Habitats Regs, Wildlife Act)

## Bugs Found and Fixed

- **scope not persisted** — `taxa parse --pg` computed scope in memory but never wrote it to Postgres. `taxa embed` requires `scope = 'substantive'`, so new laws got 0 embeddings. Fixed: scope now written for all provisions (out, structural, substantive).
- **lat_pulled_at stale** — sync-watch pulled LAT but never set `lat_pulled_at` in DuckDB, so `taxa status` showed 130 needs_lat when only ~37 genuinely missing. Fixed: sync-watch now sets timestamp after LAT pull.
- **triage not published** — sync-watch triaged but didn't push the result back to sertantai. Fixed: triage result now published to `triage/{law_name}` after every ingestion.
- **orphaned sync.rs** — 929 lines of dead code in fractalaw-cli from the CLI split. Removed.

## Revoked Laws Cleaned

28 revoked/repealed laws removed from Postgres LAT (4,519 provisions). 5 laws incorrectly marked revoked reinstated (live status bug on sertantai, now fixed).

## Remaining

- **~37 laws still need LAT** from sertantai (not yet parsed there)
- **Comms Act 2003** (UK_ukpga_2003_21) — 10K provisions timing out during sertantai parse, needs fix
- **101 pending_slm** actors from existing corpus need SLM batch

## Nature Protection QA — Eyeball Analysis

Spot-checked 4 nature protection laws at QQ's request. Mixed results — highlights gaps the rework will fix.

### UK_ukpga_2006_16 — NERC Act 2006 (s.40 biodiversity duty)

**Status**: No LAT. Zero provisions in Postgres. Family = "X: No Family". Triage = uncertain (48%).

The s.40 biodiversity duty ("every public authority must have regard to conserving biodiversity") hasn't been processed at all. LAT pulled during this session (871 provisions) but not yet enriched.

### UK_ukpga_2009_23 — Marine and Coastal Access Act 2009

**Status**: 3,115 provisions, 2,845 enriched. SLM ran but reconcile never completed — final position/drrp columns empty.

- **s.66(1)** — Lists licensable marine activities (dredging, construction, deposits). No actors extracted — obligation is implicit ("it is a licensable marine activity to do..."). Pipeline missed it.
- **s.125(2)-(9)** — Obligations on public authorities re MCZs. Government-facing. Actors found but all classified as `mentioned` by classifier; SLM correctly reclassified as `active` but results never reconciled.
- **s.126(6)-(7)** — "the person seeking the authorisation" must satisfy the authority. SLM correctly identified `Ind: Person` as `active/Obligation` and `Gvt: Authority` as `counterparty`. Key provision for anyone applying for marine licences (e.g. quarry operator doing marine dredging). **Not reconciled.**

### UK_uksi_2017_1012 — Conservation of Habitats and Species Regs 2017

**Status**: 1,026 provisions, enriched. SLM ran and reconciled. Best quality of the four.

- **reg.43(1)** — Criminal offence: any person who deliberately captures/injures/kills European protected species. `Ind: Person` correctly `active/Obligation`. Applies to employers on site.
- **reg.55(1)** — Licensing body may grant a licence. Liberty, **but no actor extracted** — "relevant licensing body" not in dictionary triggers (only "licensing authority" matched). **Fixed**: added "licensing body" trigger.
- **reg.55(9)** — Licensing body must not grant unless satisfied (three tests for EPS derogation). Obligation detected, **but no actor** — same dictionary gap. Fixed by same trigger addition.
- **reg.63(1),(3),(5)** — Habitats Regulations Assessment duties on competent authorities. `Gvt: Authority` correctly identified.

### UK_ukpga_1981_69 — Wildlife and Countryside Act 1981

**Status**: Enriched, SLM ran and reconciled.

- **s.1(5)** — Criminal offence: disturbing Schedule 1 birds. `Ind: Person` correctly `active/Obligation`. Directly relevant to quarry operators near nesting sites.
- **s.9(4)** — Criminal offence: damaging Schedule 5 animal shelters. `Ind: Person` correctly `active/Obligation`.
- **s.28G(2)** — SSSI conservation duty on public authorities. `Gvt: Authority` as `active`. Government-facing.
- **s.28H(1)** — "a section 28G authority shall give notice..." — Obligation detected but **no actor extracted**. Cross-reference format "section 28G authority" doesn't match regex. Downstream subsections (4)-(6) do match because they use "the authority".
- **s.28I(1)-(6)** — Authorising operations near SSSIs. Government-facing duties, correctly classified.

### Summary

| Law | Criminal offences (governed) | Government duties | Gaps |
|-----|------------------------------|-------------------|------|
| NERC 2006 | Unknown (no LAT processed) | s.40 biodiversity | No LAT |
| MCAA 2009 | s.66(1) implicit | s.125, s.126 | SLM not reconciled, implicit actors missed |
| Habitats Regs 2017 | reg.43(1) Person | reg.63 authority | reg.55 "licensing body" trigger missing (now fixed) |
| Wildlife Act 1981 | s.1(5), s.9(4) Person | s.28G-I authority | s.28H(1) cross-ref format missed by regex |

The criminal offence provisions (applying to "any person") are the most relevant to QQ as a governed employer. These are generally well-classified. Government-facing duties are correctly identified but less relevant to the customer. The main gaps are: missing LAT (NERC), unreconciled SLM results (MCAA), and two actor dictionary gaps (now one fixed).

## Dependencies

- ✅ PgStore hub operational
- ✅ SSD installed — disk pressure eliminated
- ✅ `fractalaw-sync-cli` publish working (verified 2026-07-08)
- ✅ SLM classifications exist for many laws (just not reconciled)
- ✅ Sertantai Zenoh queryable for customer laws (verified)
- ⬜ Sertantai LAT availability for 130 missing laws
- ⬜ RunPod for SLM batch
