---
session: "Meta-plan: improve the DRRP pipeline from the regex end"
status: active
opened: 2026-10-06
closed:
outcome:
issue: 72
related: [60, 65, 74, 75, 76, "parsing/2026-10-01-training-labels-slm.md", "docs/architecture/DRRP-LABELLING-REVIEW-2026-10-06.md"]
---

# Session: Meta-plan: improve the DRRP pipeline from the regex end (ACTIVE)

## Problem

We already have the pipeline:
- regex parse (with a regex purpose and a purpose gate);
- dependency features → classifier → infer → reconcile;
- SLM → re-reconcile → backfill;
- the LLM tier on what's left.

The training-labels workstream (suspended, `parsing/2026-10-01-training-labels-slm.md`) built data **from the LLM end of the pipe**. One definitive prompt answered every question for every provision. It grew to ~45 rules and ~10K tokens in two days, with a second model and a referee on top, costly and complex.

**Jason (2026-10-06): the mistake was building training data from the LLM end, not the regex end.**

The review (`docs/architecture/DRRP-LABELLING-REVIEW-2026-10-06.md`) measured:
- **32%** of 234K substantive provisions have no duty word, and only 2% of those are relations;
- machinery and Procedure+Detail purposes are relations **1.5% / 0%** of the time;
- a purpose gate would skip **28%** of duty-word provisions and lose **0.5%** of relations;
- **58%** of model disputes involve purpose.

Most of the cost and rules sit where the cheap tiers could decide.

**Principle:** measure and improve each tier **from the regex end upward**. Each tier decides what it can decide reliably and passes only the residual up. The LLM-end labels we have (Gemini v1.3 on 6,959; refereed batches 1–2) become the **evaluation set** for every tier, not training data for one model.

## Todo

- ✅ Suspend the training-labels workstream properly (status, state, pointers, commits)
- ⬜ **Gemini critical review of this meta-plan** (gemini-review skill; one call, ~cents). Fold in its findings and Jason's rulings
- ⬜ **Phase 1: Tier measurement** (`parsing/2026-10-06-phase1-tier-measurement.md`). Score each existing tier against the labels, regex first. Where each tier is right or wrong, and what consumers lose
- ⬜ **Phase 2: Regex end and guards** (`parsing/2026-10-06-phase2-regex-and-guards.md`):
  - regex purpose on the published vocabulary (#76) and updated purpose gates;
  - the duty-word prefilter;
  - regex `act` (#75);
  - patterns for the ~100 trigger-only dictionary labels (by frequency, with masks and tests)
- ⬜ **Phase 3: Middle tiers** (`parsing/2026-10-06-phase3-classifier-tier.md`):
  - retrain the cheap position classifier and add a cheap purpose classifier (embeddings) on the labels;
  - confidence thresholds decide what escalates
- ⬜ **Phase 4: SLM on the residual** (`parsing/2026-10-06-phase4-slm-residual.md`): retrain on what actually reaches the SLM after phases 2–3. Relation + actors (+ act?), not purpose. RunPod: Jason launches, cost approved first
- ⬜ **Phase 5: LLM tier on the low-confidence residual** (`parsing/2026-10-06-phase5-llm-short-prompt.md`): split the prompt into a short relation/actor prompt; purpose leaves the LLM. Rule governance: a rule enters only if it can change a verdict, holder or correlative
- ⬜ **Phase 6: Evaluation and QA** (`parsing/2026-10-06-phase6-evaluation-qa.md`; gold v2 is `benchmarks/2026-09-30-gold-v2.md`):
  - acceptance thresholds per tier and end to end, before the single run;
  - the referee is used only for gold and release QA
- ⬜ Then the single run (`parsing/09-29-26-reenrichment-backlog.md`) on the improved pipeline

## Dependencies

- ✅ Labels for evaluation: Gemini v1.3 (6,959, 656 laws, test split by law), refereed batches 1–2, `holder60_cases`, the 50 hand-checked rows
- ✅ Spec and prompt v1.4 (rulings 1–15) committed. Under this plan, most purpose rulings move to the purpose tier's label definitions
- ✅ Actor dictionary reconciled with legal (~245 labels; many trigger-only, which is phase 2's input)
- ⬜ Gemini review of this plan
- **Paid services: walk, don't run** (memory `feedback_paid_api_walk_dont_run`). Every paid run gets a total cost approved first; OpenAI credit ~$12.70
- Enables: the single run (backlog session), gold v2 (#74)

## Notes

- **Not discarded:**
  - the dictionary work, the rulings (they define correct answers for evaluation), the refereed labels, and the labeller tooling;
  - `--carry-from` and the stale check, which still serve any LLM-tier relabel.
- **Open question for phase 1:** how good is the regex tier today on relation and actors? That number decides how much each later phase has to carry.
