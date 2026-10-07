---
session: "Phase D: Tier measurement"
status: pending
opened: 2026-10-07
related: ["parsing/2026-10-06-meta-plan-sentence-units.md", "parsing/2026-10-07-phaseC-gold-sentence-level.md"]
---

# Session: Phase D: Tier measurement (PENDING)

## Problem

Once the sentence gold is frozen, measure each tier against it: regex first, then the dependency-feature classifier against the SLM, with error analysis per class (Liberty, beneficiary, applying holders, `both`, second entries). The baseline is an LLM alone: the existing Gemini v1.3 labels (6,959 provisions) scored against gold at no new cost. That sets the ceiling the cheap tiers are judged against.

## Todo

- ⬜ Apply phase C's row-to-sentence scoring spec: combine row-level tier outputs into sentence predictions
- ⬜ LLM-only baseline: Gemini v1.3 labels scored against gold (no new cost)
- ⬜ Regex tier scored per field and class; error analysis
- ⬜ Classifier vs SLM bake-off; error analysis per class
- ⬜ Context principles (HOLD-05, INF-03) scored apart, via the holder-linking step (#60, #77)
- ⬜ Findings feed phase E (tier improvement), including the assembler in `fractalaw-core`

## Dependencies

- ⬜ Phase C: frozen gold (`gold-v4.0`) and the scoring spec
- ⬜ Legal's parse repair pulled, and the 55 test laws re-parsed (regex), so tier outputs match the repaired text
