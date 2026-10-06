---
session: "Phase 4: SLM on the residual"
status: pending
opened: 2026-10-06
closed:
outcome:
related: ["parsing/2026-10-06-pipeline-from-the-regex-end.md"]
---

# Session: Phase 4: SLM on the residual (PENDING)

## Problem

The SLM (`gemma3-position`) was trained on old, inconsistent labels. Retrain it on the cases that actually reach it after phases 2–3, for relation + actors only (not purpose).

## Todo

- ⬜ Build training data = labelled provisions that reach the SLM after the new gates (train split); class weighting for beneficiary
- ⬜ Targets: relation, holder(s), position, holds (+ act?: decide)
- ⬜ **(Jason)** RunPod fine-tune; total cost stated and approved first
- ⬜ Evaluate on the test laws, `holder60_cases` and the 50 hand-checked rows; bar: match Gemini on counterparty/beneficiary

## Dependencies

- Meta-plan `parsing/2026-10-06-pipeline-from-the-regex-end.md` (after its Gemini review)
