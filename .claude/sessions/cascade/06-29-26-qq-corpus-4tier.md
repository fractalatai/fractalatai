# Session: QQ Corpus 4-Tier Enrichment & Republish (ACTIVE)

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
| [Tier 0 — Base Case](06-29-26-tier0-base-case.md) | PENDING | Define in-scope provisions, clean out-of-scope actors |
| [Tier 1 — Regex & Embed](06-29-26-tier1-regex.md) | PENDING | Regex parse + embed on in-scope provisions only |
| [Tier 2 — Classifier](06-29-26-tier2-classifier.md) | PENDING | Dep features + position classifier (fix bugs) |
| [Tier 3 — SLM](06-29-26-tier3-slm.md) | PENDING | SLM on pending_slm, re-reconcile with per-class gating |
| [Tier 4 — Publish](06-29-26-tier4-publish.md) | PENDING | Backfill + publish to sertantai |

## Progress so far (before tier sessions created)

Steps already run on the QQ corpus (will be validated/re-run by tier sessions):

- ✅ Dep features computed (108,386 actors)
- ✅ Classify ran (48,416 actors got cls_position — 42%, gap identified)
- ✅ Infer ran (1,146 actors inferred)
- ✅ Reconcile ran (59,203 actors — 19,813 pending_slm)
- ⬜ SLM — not yet run (19,813 actors, ~18 hrs)
- ⬜ Backfill + publish

## Bugs identified

- **Base case undefined** — regex parses schedules/definitions creating dead-end actors (Tier 0)
- **Classify reads from legislation_text.actors not provision_actors** — 50,837 actors skipped (Tier 2)
- **Classify silently skips actors with missing dep features** — no QA gate (Tier 2)

## TODOs (beyond tier sessions)

- ⬜ Create customer onboarding skill — repeatable recipe wrapping Tier 0-4 for a customer's legal register

## Dependencies

- ✅ 4-tier cascade wired and tested on benchmarks (78.2% accuracy)
- ✅ gemma3-position loaded in Ollama
- ✅ All CLI commands built
