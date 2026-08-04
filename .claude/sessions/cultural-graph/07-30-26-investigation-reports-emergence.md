---
session: Investigation Reports Emergence Pass
status: pending
opened: 2026-07-30
---

# Session: Investigation Reports Emergence Pass (PENDING)

## Problem

The `blames` and `silences` edge types remain absent across all three reporter-authored source types (positive observations, hazards, near-misses). This is not a schema problem — it reflects the author perspective. Reporters describe their own experience and culturally avoid assigning blame. Investigation reports are written by investigators in third-person analytical language, tracing root causes and assigning contributing factors. This is the narrative source where blame attribution ("the supervisor failed to ensure"), silence dynamics ("the concern had been raised previously but not actioned"), and systemic failure analysis surface.

Investigation reports represent a fundamentally different author perspective — not just a different report type. The same extraction schema and scripts apply, but the training data would teach the model to recognise blame and silence from investigator language.

## Todo

- ⬜ Confirm investigation reports are available from QQ and in what format
- ⬜ Run emergence pass — check for new edge types from investigator perspective
- ⬜ Run constrained pass
- ⬜ Export training data — namespaced as investigation-reports-*
- ⬜ Assess whether blames and silences appear at meaningful frequencies
- ⬜ Produce executive brief

## Dependencies

- ✅ Extraction pipeline validated across three source types
- ✅ Schema validated — blames/silences confirmed as gaps requiring investigator-perspective data
- ⬜ Investigation reports data from QQ
