---
session: "Phase 1: Tier measurement"
status: pending
opened: 2026-10-06
closed:
outcome:
related: ["parsing/2026-10-06-pipeline-from-the-regex-end.md"]
---

# Session: Phase 1: Tier measurement (PENDING)

## Problem

We don't know how good each existing tier is against the labels we now have (Gemini v1.3 on 6,959 provisions; refereed batches 1–2). Without that, we can't say what each later phase must carry. Measure the regex end first.

## Todo

- ⬜ Evaluation set: Gemini v1.3 labels plus referee/consensus for batches 1–2 (higher-quality subset); test split by law
- ⬜ Regex purpose vs the labelled purpose (map the old vocabulary to the published 18): confusion by purpose
- ⬜ Purpose gate (`should_skip_drrp`, SKIP_PURPOSES): what it skips, relations lost, relations it should have skipped
- ⬜ Duty-word prefilter (proposed): precision/recall on relation (expected ~98% / 2% loss)
- ⬜ Regex DRRP relation + actors + positions (`taxa parse`): accuracy per field
- ⬜ Classifier tier (cls position) and current SLM (`gemma3-position`): accuracy on what reaches them
- ⬜ Reconcile output vs labels; the cost of each error to consumers (Making verdict, holders, correlatives)
- ⬜ Report: per-tier decide/escalate rates and the residual at each step

## Dependencies

- Meta-plan `parsing/2026-10-06-pipeline-from-the-regex-end.md` (after its Gemini review)
