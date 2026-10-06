---
session: "Phase 5: LLM tier on the low-confidence residual"
status: pending
opened: 2026-10-06
closed:
outcome:
related: ["parsing/2026-10-06-pipeline-from-the-regex-end.md"]
---

# Session: Phase 5: LLM tier on the low-confidence residual (PENDING)

## Problem

The definitive prompt grew to ~10K tokens and ~45 rules, mostly purpose and machinery edge cases. With purpose decided earlier, the LLM only needs relation + actors on the residual the SLM is unsure of.

## Todo

- ⬜ Split `drrp_prompt.py`: a short relation/actor prompt (Hohfeld positions, the act test, holder resolution: stem, referenced, applying); purpose rules move to the purpose tier's definitions
- ⬜ Rule governance: a rule enters the relation prompt only if it can change a verdict, holder or correlative; otherwise it's a default
- ⬜ LLM tier (`gemini_llm_batch.py`) on the SLM's low-confidence residual; total cost approved first
- ⬜ Measure vs the refereed labels

## Dependencies

- Meta-plan `parsing/2026-10-06-pipeline-from-the-regex-end.md` (after its Gemini review)
