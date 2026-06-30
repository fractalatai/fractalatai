---
session: QQ Corpus 4-Tier Enrichment & Republish
status: closed
opened: 2026-06-29
closed: 2026-06-30
outcome: success

summary: >
  Full compliance enrichment of 274 QQ customer laws — from base case definition
  through SLM classification to sertantai publish. 82,237 provisions published,
  99.8% actor resolution. Pipeline evolved from 4-tier to confidence-based SLM+LLM.

decisions:
  - what: Two-tier LAT enrichment model (discovery vs compliance)
    why: Not all laws need full classification — only customer-applicable laws get the cascade
    result: Discovery is seconds per law (regex only). Compliance is minutes (SLM bottleneck).

  - what: Per-tier sessions with QA close signals
    why: Pipeline was running end-to-end without validation. Breaking into tiers with QA checks catches problems before expensive downstream processing.
    result: Found and fixed — base case gap, classifier skip bug, agentic skip, embedding gaps, STRUCTURAL scope

  - what: SLM replaces classifier as primary position+DRRP classification
    why: SLM 79.7% position / 92.5% DRRP vs classifier 59.9% / regex 87.7%. Confidence signal enables LLM elevation at <0.9.
    result: 86.8% of actors resolved by SLM, 1.4% flagged for LLM

metrics:
  corpus: { laws: 274, provisions: 107247, substantive: 75965, actors: 62922 }
  scope: { out: 14.6%, structural: 14.6%, substantive: 70.8% }
  resolution: { slm: 86.8%, agree: 5.0%, llm: 2.9%, classifier: 2.8%, inferred: 2.4%, pending: 0.2% }
  published: { provisions: 82237, laws: 229 }
  cost: { slm_runpod: ~$3, llm_gemini: ~$0.50, total: ~$3.50 }

lessons:
  - title: Break the pipeline into tiers with QA gates
    detail: Running end-to-end hides problems. Per-tier stats (customer-stats) catch gaps at each stage before committing hours of compute. The base case definition alone revealed 16.9% of provisions should never enter the pipeline.
    tag: methodology

  - title: The pipeline simplified itself through measurement
    detail: Started with regex → classifier → reconcile → SLM → reconcile. Ended with regex → SLM (confidence-based) → LLM. Measurement showed the classifier added no value over SLM. Each iteration removed complexity.
    tag: architecture

  - title: Confidence-based elevation is better than per-class gating
    detail: Per-class gating (don't trust beneficiary/mentioned) flagged 16.6% for LLM. Confidence threshold at 0.9 flags 1.4%. The model's own certainty is a more precise signal than hardcoded position lists.
    tag: methodology

artifacts:
  - scripts/corpus_stats.py
  - scripts/gemini_llm_batch.py
  - scripts/runpod_slm_batch.py
  - .claude/skills/customer-batch-parse/SKILL.md
  - .claude/skills/customer-stats/SKILL.md
  - .claude/skills/llm-batch/SKILL.md

depends_on:
  - 06-29-26-slm-pipeline-wiring
  - 06-27-26-local-llm-tier

enables:
  - QQ customer data live in sertantai
  - Repeatable customer onboarding via skills
  - SLM retraining with LLM feedback data
---

# Session: QQ Corpus 4-Tier Enrichment & Republish (CLOSED)

## Problem

274 QQ applicable laws need compliance-level enrichment (full 4-tier cascade) and republish to sertantai.

## Two-tier LAT enrichment model

| | Discovery (non-customer) | Compliance (customer) |
|--|-------------------------|----------------------|
| **Purpose** | "Is this a making law?" | "Who owes what to whom?" |
| **Pipeline** | Regex parse only | Regex → Classifier → SLM → LLM |
| **Time per law** | Seconds | Minutes (SLM bottleneck) |

Discovery enrichment already done for the QQ corpus. This session runs compliance enrichment.

## Pipeline — per-tier sessions

Each tier has its own PENDING session with descriptive stats, QA checks, and a close signal:

| Session | Status | Scope |
|---------|--------|-------|
| [Tier 0 — Base Case](06-29-26-tier0-base-case.md) | CLOSED | OUT/STRUCTURAL/SUBSTANTIVE scope |
| [Tier 1 — Regex & Embed](06-29-26-tier1-regex.md) | CLOSED | 99.8% embedding coverage |
| [Tier 2 — Classifier](06-29-26-tier2-classifier.md) | CLOSED | 98.3% classifier coverage, gap=0 |
| [Tier 3 — SLM](06-29-26-tier3-slm.md) | CLOSED | 25K actors on RunPod, 18.8/s |
| [Tier 4 — LLM + Publish](06-29-26-tier4-publish.md) | CLOSED | 985 LLM, 82K provisions published |
| [STRUCTURAL Scope](06-30-26-structural-scope.md) | CLOSED | pending_llm -19% |
| [SLM All Actors](06-30-26-slm-all-actors.md) | CLOSED | Dual DRRP+position, confidence-based gating |

## Progress so far (before tier sessions created)

Steps already run on the QQ corpus (will be validated/re-run by tier sessions):

- ✅ Dep features computed (108,386 actors)
- ✅ Classify ran (48,416 actors got cls_position — 42%, gap identified)
- ✅ Infer ran (1,146 actors inferred)
- ✅ Reconcile ran (59,203 actors — 19,813 pending_slm)
- ✅ SLM — 113,833 actors on RunPod (3hrs, $3)
- ✅ LLM — 985 pending_llm via Gemini ($0.50)
- ✅ Backfill + publish — 82,237 provisions to sertantai

## Bugs identified

- **Base case undefined** — regex parses schedules/definitions creating dead-end actors (Tier 0)
- **Classify reads from legislation_text.actors not provision_actors** — 50,837 actors skipped (Tier 2)
- **Classify silently skips actors with missing dep features** — no QA gate (Tier 2)

## TODOs (beyond tier sessions)

- ✅ Created customer-batch-parse skill, customer-stats skill, llm-batch skill

## Dependencies

- ✅ 4-tier cascade wired and tested on benchmarks (78.2% accuracy)
- ✅ gemma3-position loaded in Ollama
- ✅ All CLI commands built
