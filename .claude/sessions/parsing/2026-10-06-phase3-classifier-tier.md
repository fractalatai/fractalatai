---
session: "Phase 3: Middle tiers (classifiers)"
status: pending
opened: 2026-10-06
closed:
outcome:
related: ["parsing/2026-10-06-pipeline-from-the-regex-end.md"]
---

# Session: Phase 3: Middle tiers (classifiers) (PENDING)

## Problem

Between regex and the SLM sit cheap learned tiers. They should decide what regex can't, with confidence, and escalate the rest. The labels are now plentiful enough to train them.

## Todo

- ⬜ Purpose classifier (embeddings + linear / kNN) on the ~7K purpose labels; measure vs referee purpose decisions; feeds gates and the purpose profile
- ⬜ Position classifier (dep features) retrained on labelled positions (counterparty/beneficiary act test)
- ⬜ Confidence thresholds: decide vs escalate; measure the residual sent to the SLM
- ⬜ Reconcile precedence reviewed for the new tiers

## Dependencies

- Meta-plan `parsing/2026-10-06-pipeline-from-the-regex-end.md` (after its Gemini review)
