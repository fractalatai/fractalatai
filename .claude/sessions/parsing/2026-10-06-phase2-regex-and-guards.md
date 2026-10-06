---
session: "Phase 2: Regex end and guards"
status: pending
opened: 2026-10-06
closed:
outcome:
related: ["parsing/2026-10-06-pipeline-from-the-regex-end.md"]
---

# Session: Phase 2: Regex end and guards (PENDING)

## Problem

The regex tier and its guards predate the current data model: the purpose vocabulary is old (#76), SKIP_PURPOSES covers only 4 purposes, and ~100 dictionary labels are trigger-only (invisible to regex). Fix the cheap end first, measured against phase 1.

## Todo

- ⬜ Regex purpose on the published 18 purposes (#76), measured against the labels
- ⬜ Purpose gates: skip machinery + Procedure+Detail, except duty-word "may"-powers (commence, exempt, time-limited transitional); measure relations lost
- ⬜ Duty-word prefilter: no duty word in text or stem → relation no
- ⬜ Regex `act` from the clause verb (#75), measured against labelled acts
- ⬜ Patterns for trigger-only labels, by corpus frequency, each with masks/excludes and tests (regex clashes found so far: company officer, economic operator, verifier, gas transporter, temporary work agency)
- ⬜ Re-measure phase 1 numbers after each change

## Dependencies

- Meta-plan `parsing/2026-10-06-pipeline-from-the-regex-end.md` (after its Gemini review)
