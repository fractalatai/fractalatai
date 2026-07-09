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
5. ⬜ Pull LAT for 130 missing laws from sertantai
6. ⬜ Triage new laws (regex) — identify which have obligations
7. ⬜ Embed new provisions
8. ⬜ SLM batch on RunPod — new laws + 8 needs_classify
9. ⬜ Reconcile + backfill all laws with unreconciled SLM results (including published laws like MCAA 2009)
10. ⬜ Publish all updated laws to sertantai
11. ⬜ Spot-check nature protection provisions (s.40 NERC, s.125-126 MCAA, etc.)

## Dependencies

- ✅ PgStore hub operational
- ✅ SSD installed — disk pressure eliminated
- ✅ `fractalaw-sync-cli` publish working (verified 2026-07-08)
- ✅ SLM classifications exist for many laws (just not reconciled)
- ✅ Sertantai Zenoh queryable for customer laws (verified)
- ⬜ Sertantai LAT availability for 130 missing laws
- ⬜ RunPod for SLM batch
