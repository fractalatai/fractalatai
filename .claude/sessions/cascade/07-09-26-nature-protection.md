---
session: Nature Protection
status: closed
opened: 2026-07-09
closed: 2026-07-09
outcome: success

summary: >
  Full QQ corpus enrichment from 274 to 428 laws. Pulled latest register via new
  customer-laws CLI, processed ~100 new laws through the complete pipeline (parse,
  embed, classify, SLM, significance, LLM, backfill, publish). Fixed 7 bugs found
  during the process. Published 428 laws / 119,835 provisions to sertantai.

decisions:
  - what: Classifier runs before SLM, not dropped
    why: Classifier provides second signal for reconciliation — without it all actors go to SLM unnecessarily
    result: 1,615 actors classified, reconciliation uses both signals to flag only unresolved actors for SLM

  - what: Significance must run after backfill
    why: Backfill sets drrp_types on legislation_text — significance script queries this field. Running before backfill misses new Obligation provisions
    result: Required two significance runs to catch provisions created by backfill

  - what: Disable logprobs in significance batch for concurrent workers
    why: Ollama deadlocks under concurrent logprobs requests (Gemini review identified)
    result: Multi-worker went from hung to 6.4/s on RTX 5090

  - what: LLM validation of LOW-confidence HIGH-significance actors
    why: Targeted QA of most important provisions where SLM was uncertain
    result: 27/29 downgraded to mentioned/none — confirms SLM confidence threshold is a genuine uncertainty signal

metrics:
  corpus: { laws: 428, provisions: 136185, substantive: 99255, published: 119835 }
  pipeline: { embed: 18632, classify: 1615, slm: 10413, llm: 316, inferred: 1720 }
  reconciled: { total: 86356, slm: 75075, regex: 6668, llm: 2536, inferred: 1720, pending_llm: 357 }
  significance: { rated: 27249, part_breakdowns: 110 }
  embedding_coverage: 100%
  bugs_fixed: 7
  revoked_cleaned: { laws: 28, provisions: 4519 }
  runpod_cost: { position_4090: ~0.40, significance_5090: ~0.30, total: ~0.70 }

lessons:
  - title: taxa parse --pg must write scope to Postgres
    detail: Parse computed scope in memory but never persisted it. Embed filters on scope=substantive so new laws got 0 embeddings. Wasted a full embed run before discovering the bug. Always verify the first batch output before running at scale.
    tag: data

  - title: Ollama logprobs deadlocks under concurrent inference
    detail: logprobs=True in the request body causes Ollama to hang with multiple workers. Single-worker works fine. Position script never had this because it doesn't use logprobs. Gemini review identified it in seconds.
    tag: infrastructure

  - title: Significance must run after backfill, not before
    detail: Backfill writes drrp_types to legislation_text. Significance queries this field. Running significance before backfill misses all new Obligation provisions. Pipeline ordering is critical and wasn't documented.
    tag: methodology

  - title: sync-watch must set lat_pulled_at in DuckDB
    detail: Without this, taxa status reports stale needs_lat counts. The DuckDB pipeline timestamps are the source of truth for taxa status but sync-watch wasn't updating them.
    tag: architecture

  - title: Use nohup for RunPod batch scripts
    detail: SSH sessions drop after ~10 min of inactivity. Scripts launched via SSH die when the connection drops. nohup + log file prevents lost work.
    tag: infrastructure

  - title: OLLAMA_NUM_PARALLEL must be set before ollama serve starts
    detail: Setting the env var after Ollama is running has no effect. Must kill and restart with the var set.
    tag: infrastructure

  - title: Gemini 2.5 Flash splits JSON across multiple response parts
    detail: The model returns thinking tokens and response tokens in separate parts. Concatenate all parts before JSON parsing, and use regex fallback for extraction.
    tag: tooling

  - title: Customer-batch-parse skill had stale paths throughout
    detail: Script paths (corpus_stats.py, compute_dep_features.py), data paths (qq-applicable-laws.csv), and publish commands (fractalaw sync → fractalaw-sync-cli) were all wrong after the CLI split and directory reorg. Skills rot when the codebase moves underneath them.
    tag: tooling

artifacts:
  - crates/fractalaw-sync-cli/src/main.rs
  - crates/fractalaw-sync-cli/src/sync.rs
  - crates/fractalaw-sync/src/zenoh_sync.rs
  - crates/fractalaw-cli/src/commands/pipeline.rs
  - crates/fractalaw-core/data/actor-dictionary.yaml
  - scripts/gemini_llm_batch.py
  - scripts/ml/runpod_significance_batch.py
  - .claude/skills/customer-batch-parse/SKILL.md
  - .claude/skills/publish/SKILL.md
  - .claude/skills/llm-batch/SKILL.md
  - .claude/skills/db-changes/SKILL.md
  - data/code-review/significance-logprobs-deadlock.md

depends_on:
  - 06-29-26-qq-corpus-4tier.md
  - 07-04-26-qq-corpus-completion.md
  - 06-30-26-slm-all-actors.md

enables:
  - QQ nature protection provisions live in sertantai with significance ratings
  - 428-law register fully enriched and published
  - customer-batch-parse skill validated and corrected for future customer onboarding
  - Triage round-trip operational (sync-watch → triage → publish back)
---

# Session: Nature Protection (CLOSED)

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
4. ✅ Pull LAT via sync-watch (~106 laws landed), manually pulled NERC 2006 + Factories Act
5. ✅ Triage new laws — sync-watch triaged on ingestion, published results back to sertantai
6. ✅ Parse new laws (93 batch + 4 nature protection laws) — regex DRRP extraction
7. ✅ Embed 18,632 provisions (23 min CPU)
8. ✅ Dep features for 10,413 actors
9. ✅ Classifier for 117 laws (1,615 actors)
10. ✅ SLM position batch on RunPod RTX 4090 — 10,413 actors, 0 errors
11. ✅ Infer correlative actors (1,720 inferred)
12. ✅ Reconcile 86,356 actors across 428 laws
13. ✅ Significance SLM on RunPod RTX 5090 — 3,434 provisions (after backfill sequencing fix)
14. ✅ Backfill — 54,409 provisions, 27,249 significance, 110 Part breakdowns
15. ✅ LLM batch (Gemini) — 316 pending_llm actors classified, 0 errors
16. ✅ LLM validation — 29 HIGH sig + low conf actors: 27/29 downgraded to mentioned/none (confirms SLM confidence threshold)
17. ✅ Re-parse Habitats Regs + Wildlife Act with new actor dictionary triggers
18. ✅ Publish — 428/428 enrichment + 119,835 provisions across 364 laws
19. ✅ Spot-check nature protection provisions — all gaps closed

## Bugs Found and Fixed

- **scope not persisted** — `taxa parse --pg` computed scope in memory but never wrote it to Postgres. `taxa embed` requires `scope = 'substantive'`, so new laws got 0 embeddings. Fixed: scope now written for all provisions (out, structural, substantive).
- **lat_pulled_at stale** — sync-watch pulled LAT but never set `lat_pulled_at` in DuckDB, so `taxa status` showed 130 needs_lat when only ~37 genuinely missing. Fixed: sync-watch now sets timestamp after LAT pull.
- **triage not published** — sync-watch triaged but didn't push the result back to sertantai. Fixed: triage result now published to `triage/{law_name}` after every ingestion.
- **orphaned sync.rs** — 929 lines of dead code in fractalaw-cli from the CLI split. Removed.
- **significance IS NULL filter** — RunPod significance script loaded all 43K Obligation provisions instead of just pending ones. Fixed: added `significance_overall IS NULL` to query.
- **significance logprobs deadlock** — `logprobs: True` in Ollama request body causes deadlock under concurrent workers. Gemini review identified. Fixed: disabled logprobs.
- **significance sequencing** — significance must run after backfill (backfill sets `drrp_types`). Documented in skill.

## Revoked Laws Cleaned

28 revoked/repealed laws removed from Postgres LAT (4,519 provisions). 5 laws incorrectly marked revoked reinstated (live status bug on sertantai, now fixed).

## Remaining

- **~37 laws still need LAT** from sertantai (not yet parsed there)
- **Comms Act 2003** (UK_ukpga_2003_21) — 10K provisions timing out during sertantai parse, needs fix
- **~7K Obligation provisions without significance** — provisions with no actors in provision_actors (obligation detected from text but no actor extracted)

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
- ✅ Sertantai LAT availability — ~106 laws pulled via sync-watch
- ✅ RunPod — RTX 4090 (position SLM) + RTX 5090 (significance SLM)
