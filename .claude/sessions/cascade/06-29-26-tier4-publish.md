---
session: Tier 4 — LLM + Backfill & Publish
status: closed
opened: 2026-06-29
closed: 2026-06-30
outcome: success

summary: >
  Ran Gemini Flash on 985 low-confidence SLM actors ($0.50, 32 min). Backfilled
  37,101 provisions, published 82,237 provisions across 229 laws to sertantai.
  99.8% of actors resolved. Pipeline complete for QQ customer corpus.

decisions:
  - what: LLM results set extraction_method directly, no reconciliation needed
    why: LLM is the final tier — there's nothing to reconcile against. Running reconcile on 59K actors to flip 985 is wasteful.
    result: Direct SQL update, instant

  - what: Gemini Flash with thinkingBudget=0 for classification
    why: Thinking mode returns multi-part responses that break JSON parsing. Budget=0 gives direct answers.
    result: Zero parse errors after fix (5 on full run were edge cases)

metrics:
  gemini_classified: 2531
  gemini_errors: 5
  gemini_time: 32.3min
  benchmark_waste: 1552 actors (61% of calls — script now excludes benchmarks)
  provisions_backfilled: 37101
  provisions_published: 82237
  laws_published: 229
  pending_llm_remaining: 2
  resolution_rate: 99.8%

lessons:
  - title: LLM is the final tier — no reconciliation needed
    detail: Reconciliation picks between competing signals. LLM has no competition. Write extraction_method='llm' directly when LLM classifies. Don't re-run reconcile on 59K actors just to flip 985.
    tag: architecture

  - title: Always exclude benchmark laws from LLM batch
    detail: SLM ran on all actors including benchmarks. Low-confidence benchmark actors became pending_llm and got sent to Gemini — 61% of calls wasted. Script now has benchmark exclusion in the SQL query.
    tag: methodology

  - title: Low-confidence SLM predictions are genuinely wrong
    detail: On the <0.9 confidence actors, SLM and LLM agreed only 24% on position. SLM said active/counterparty, LLM said mentioned (59% of cases). The confidence signal correctly identifies uncertain predictions.
    tag: models

artifacts:
  - scripts/gemini_llm_batch.py
  - .claude/skills/llm-batch/SKILL.md

depends_on:
  - 06-30-26-slm-all-actors

enables:
  - QQ customer corpus live in sertantai
  - SLM retraining with LLM results as additional training data
---

# Session: Tier 4 — LLM + Backfill & Publish (CLOSED)

## Problem

1,640 actors across 316 laws have SLM confidence < 0.9 — flagged as `pending_llm`. These are spread thinly (top law has 135, most have single digits). After LLM resolution, backfill and publish to sertantai.

## Low-confidence distribution

Not clustered — 316 laws, average 5 actors per law. Running Gemini per-actor (not per-law) is the practical approach.

Top laws by low-confidence count:
| Law | Actors |
|-----|--------|
| UK_ukpga_1996_18 | 135 |
| UK_ukpga_1988_52 | 56 |
| UK_ukpga_1990_43 | 39 |
| UK_ukpga_1984_55 | 34 |

Estimated LLM cost: ~$0.50-1.00 (Gemini Flash), ~10-20 min.

## Work

1. ✅ Re-reconcile with confidence-based rules — 985 pending_llm (QQ corpus)
2. ✅ Run Gemini Flash on pending_llm — 2,531 classified (1,552 were benchmark actors, wasted calls — script now excludes benchmarks)
3. ✅ Set extraction_method = 'llm' directly (no reconcile needed — LLM is the final tier)
4. ✅ Customer-stats — all tiers PASS (122 embedding gap = 0.2%, negligible)
5. ✅ Backfill — 37,101 provisions across 259 laws
6. ✅ Publish — 82,237 provisions across 229 laws (30 had no enriched provisions)
7. ✅ HSWA test publish verified by sertantai
8. ✅ Stats passing — 99.8% actors resolved

## Current state

| Method | Count | % |
|--------|-------|---|
| slm | 54,610 | 86.8% |
| reconciled_agree | 3,122 | 5.0% |
| llm | 1,805 | 2.9% |
| reconciled_classifier | 1,762 | 2.8% |
| inferred | 1,520 | 2.4% |
| pending_slm | 101 | 0.2% |
| pending_llm | 2 | 0.0% |

99.8% resolved. 2 pending_llm (Gemini parse errors).

## Mistake: benchmark laws sent to Gemini

SLM ran on all 113K actors including benchmarks. Benchmark actors with confidence < 0.9 became pending_llm and were sent to Gemini — 1,552 of 2,531 calls (61%) were wasted. Gold benchmarks table untouched. Script now excludes benchmark laws from query.

## QA checks (close signal)

- pending_llm ≈ 0 (2 remaining — parse errors)
- count(provisions with reconciled actors BUT no actors in legislation_text) = 0
- All published laws have provisions_published_at set
- customer-stats all tiers PASS

## Depends on

- ✅ 06-30-26-slm-all-actors (CLOSED — confidence-based gating implemented)
- ✅ 06-29-26-tier3-slm (CLOSED)
