---
session: "Meta-plan: improve the DRRP pipeline from the regex end"
status: closed
opened: 2026-10-06
closed: 2026-10-06
outcome: partial
issue: 72
related: [60, 65, 74, 75, 76, 78, "parsing/2026-10-01-training-labels-slm.md", "docs/architecture/DRRP-LABELLING-REVIEW-2026-10-06.md"]

summary: >
  Replaced building training data from the LLM end with improving each tier from the regex end, using
  LLM labels as evaluation. After a Gemini review it put a human gold set first (phase 0a) and a purpose review (0b).
  Phase 0a misfired: the plan never fixed the unit of analysis, so the gold set labelled row snippets and grew
  a case-law rulebook. Closed for a new meta-plan built on sentence units.

decisions:
  - what: Improve tiers from the regex end; LLM labels become the evaluation set, not training data
    why: The one-prompt LLM approach grew to ~45 rules and ~10K tokens; 32% of provisions have no duty word
    result: Principle carries into the new plan
  - what: Human gold set before scoring any tier (Gemini review)
    why: Scoring against LLM labels is circular
    result: Phase 0a built the machinery; the unit was wrong
  - what: Layered purpose with statutory-term classes (phase 0b)
    why: The 18-value scheme is an Airtable legacy, involved in 58% of model disputes
    result: 11 coarse classes; carries into the new plan

metrics:
  labelling_review: { substantive: 234000, no_duty_word: "32%", purpose_in_disputes: "58%", purpose_gate_skip: "28%", gate_relation_loss: "0.5%" }

lessons:
  - title: A pipeline plan must start from what is labelled, not from the tiers
    detail: Six phases were planned around tiers (regex, classifier, SLM, LLM) with stubs written up front. None defined the unit of meaning (the legal sentence vs the source row), and the first phase spent a day learning that. Define the unit, the label set and the yardstick before phasing the tiers.
    tag: methodology
  - title: Stubs written before the yardstick exists go stale
    detail: The phase 1–6 stubs assumed row-level labels and the old purpose scheme; all were deleted unopened. Write stubs for the next one or two phases only.
    tag: methodology

artifacts:
  - docs/architecture/DRRP-LABELLING-REVIEW-2026-10-06.md
  - data/code-review/drrp-pipeline-regex-end-meta-plan.md

depends_on:
  - 2026-10-01-training-labels-slm

enables:
  - new meta-plan (sentence units, rules vs precedents)
---

# Session: Meta-plan: improve the DRRP pipeline from the regex end (CLOSED)

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
- ✅ **Gemini critical review of this meta-plan** (Gemini 2.5 Pro, raw: `data/code-review/drrp-pipeline-regex-end-meta-plan.md`). Action summary below; ⏸️ Jason's rulings on it
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) **Phase 0a: A trustworthy yardstick first** (from the Gemini review). A human-verified gold set (stratified, rare classes over-sampled: Liberty, beneficiary, applying holders), used to measure the bias of the 7K LLM "silver" labels **before** any tier is scored against them. Merges with gold v2 / Jason's 300-label spot check. **(Jason)** size and his review time
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) **Phase 0b: Purpose suitability review** (Jason, 2026-10-06: "the Purpose classification is a legacy of my time working with Airtable"):
  - which consumers actually use purpose (the purpose profile for legal#172, the DRRP gates #76, compliance screening);
  - whether the 18 values are needed, or a coarse operative/machinery/procedure/sanctions split would do;
  - legal compatibility (the published field).
  - Decide **before** phases 2–3 build gates or a classifier on it
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) **Phase 1: Tier measurement** (`parsing/2026-10-06-phase1-tier-measurement.md`). Score each existing tier against the labels, regex first. Where each tier is right or wrong, and what consumers lose
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) **Phase 2: Regex end and guards** (`parsing/2026-10-06-phase2-regex-and-guards.md`):
  - regex purpose on the published vocabulary (#76) and updated purpose gates;
  - the duty-word prefilter;
  - regex `act` (#75);
  - patterns for the ~100 trigger-only dictionary labels (by frequency, with masks and tests)
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) **Phase 3: Middle tiers** (`parsing/2026-10-06-phase3-classifier-tier.md`):
  - retrain the cheap position classifier and add a cheap purpose classifier (embeddings) on the labels;
  - confidence thresholds decide what escalates
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) **Phase 4: SLM on the residual** (`parsing/2026-10-06-phase4-slm-residual.md`): retrain on what actually reaches the SLM after phases 2–3. Relation + actors (+ act?), not purpose. RunPod: Jason launches, cost approved first
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) **Phase 5: LLM tier on the low-confidence residual** (`parsing/2026-10-06-phase5-llm-short-prompt.md`): split the prompt into a short relation/actor prompt; purpose leaves the LLM. Rule governance: a rule enters only if it can change a verdict, holder or correlative
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) **Phase 6: Evaluation and QA** (`parsing/2026-10-06-phase6-evaluation-qa.md`; gold v2 is `benchmarks/2026-09-30-gold-v2.md`):
  - acceptance thresholds per tier and end to end, before the single run;
  - the referee is used only for gold and release QA
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) Then the single run (`parsing/09-29-26-reenrichment-backlog.md`) on the improved pipeline

## Legal-side refactoring (collected; for after this plan is worked through)

Jason (2026-10-06): legal will have refactoring to do once we've worked this through. Collected here:
- **Provision titles:** read P1group/Title in legal's LAT parser so section/regulation titles reach fractalaw. It changes `lat_hash` for ~all laws (one re-pull). The cheapest purpose signal (phase 0b).
- **Law-level purpose:** retire legal's own 15-value PurposeClassifier (from the law title) and derive law-level purpose from fractalaw's provision purposes (#172), so there's one vocabulary.
- The purpose vocabulary migration of the published `purposes` field (after phase 0b).
- **List items counted once** (Jason, 2026-10-06): an item that only completes its stem carries no DRRP of its own (`drrp_types = []`), but its actors and their positions are still sent. Legal's per-provision display shows the obligation on the stem. Worth telling legal with the other changes.
- Actor label renames and the class change are already queued for the single run (`mix actors.rename_labels`).
- ~~Actor entries keyed by (label, role)~~: not this iteration (Jason, 2026-10-06). One entry per label stays; the loss is tracked in #78.

## Gemini review feedback (2026-10-06)

Raw review: `data/code-review/drrp-pipeline-regex-end-meta-plan.md`. Claude's reading of it:

**Valid, and adopted into the plan (pending Jason):**
- **Circularity.** Scoring tiers against LLM labels caps quality at the LLM's level, and the gates could silently drop exactly the relations the LLM misses. The refereed batches are still LLM-derived, so they don't fix this. → Phase 0a: a human-verified gold set measures the silver labels' bias first.
- **Order.** The ruler comes first: gold and measurement before improving tiers. → Phase 0a before phase 1; phase 6's gold work moves forward.
- **Phase 3 may be redundant.** Measure the existing dependency-feature classifier against the SLM in phase 1 (a bake-off). If the SLM wins, cut phase 3 and go regex → SLM → LLM.
- **Rare classes.** Report metrics per class (Liberty, beneficiary, applying holders), not just overall accuracy. The sample over-weights duties.
- **Error analysis, not just scores.** Phase 1 records *why* each tier fails (syntax, missing keyword, semantics), to drive phase 2.
- **Acceptance metrics per phase**, to be set by Jason. Gemini proposes:
  - prefilter precision > 99.8%;
  - purpose-gate recall > 99.5% on gold relations;
  - SLM resolving ≥ 80% without escalation, with (holder, position) F1 > 0.9 and beneficiary F1 > 0.7;
  - end to end: Making > 99%, holder P/R > 98%, correlatives F1 > 0.95, purpose-profile JS divergence < 0.1.

**Over-reach, or adopt lightly:**
- A full cost/error optimisation model for escalation thresholds: a simple threshold sweep on gold will do.
- Feeding upstream confidence into the SLM as a feature: worth it only if phase 1 shows gate errors cascading.
- "0.5% loss is an illusion": overstated, but valid as a reason to re-measure it on gold.

**Plus Jason's point:** the purpose scheme itself is an Airtable legacy, so review its suitability (phase 0b) before gates or a classifier depend on it.

## Dependencies

- ✅ Labels for evaluation: Gemini v1.3 (6,959, 656 laws, test split by law), refereed batches 1–2, `holder60_cases`, the 50 hand-checked rows
- ✅ Spec and prompt v1.4 (rulings 1–15) committed. Under this plan, most purpose rulings move to the purpose tier's label definitions
- ✅ Actor dictionary reconciled with legal (~245 labels; many trigger-only, which is phase 2's input)
- ⏸️ (deferred — superseded by the sentence-unit meta-plan, 2026-10-06) Gemini review of this plan
- **Paid services: walk, don't run** (memory `feedback_paid_api_walk_dont_run`). Every paid run gets a total cost approved first; OpenAI credit ~$12.70
- Enables: the single run (backlog session), gold v2 (#74)

## Notes

- **Not discarded:**
  - the dictionary work, the rulings (they define correct answers for evaluation), the refereed labels, and the labeller tooling;
  - `--carry-from` and the stale check, which still serve any LLM-tier relabel.
- **Open question for phase 1:** how good is the regex tier today on relation and actors? That number decides how much each later phase has to carry.
