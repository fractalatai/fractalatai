---
session: "Phase B: Principles and precedents"
status: pending
opened: 2026-10-06
related: ["parsing/2026-10-06-meta-plan-sentence-units.md", "docs/architecture/DRRP-RULE-CATALOGUE.md", "docs/architecture/DRRP-CLASSIFICATION.md"]
---

# Session: Phase B: Principles and precedents (PENDING)

## Problem

`docs/architecture/DRRP-RULE-CATALOGUE.md` holds 123 active rules (REL 41, HOLD 17, TYPE 7, INF 4, POS 19, ACT 12, LBL 15, DEF 8). 44 were added or amended on 2026-10-06, many from a single provision, and some oscillated. The tiers below the LLM can't carry that many, and a gold set built on them measures case law. The catalogue becomes about 30 to 40 general principles; the rest become precedents.

## Todo

- ⬜ Classify every rule: **principle** (general, recurs, a model can learn it), **precedent** (one provision or a narrow pattern), **retired by sentence units** (stems, items, continuation: REL-02, REL-13, REL-15, REL-16, REL-30, REL-45, HOLD-02, TYPE-06, POS-17, POS-18, INF-04's stem clause, the item half of REL-28), or **merge** (overlapping rules)
- ⬜ Recurrence check: for each candidate principle, count the provisions it decides in the 240 reviewed and the silver labels; under about 3 → precedent
- ⬜ Draft catalogue v2 (principles only, one line plus one example each), for Jason's review
- ⬜ Precedent store: precedents live as reviewed rows in `drrp_gold` (section_id, decision, comment), tagged with a short pattern name, and are shown to the justifier as examples, not cited as rules
- ⬜ Promotion rule written into the brief: a new pattern becomes a principle only after about 3 provisions; the justifier reports candidate patterns, it doesn't make rules
- ⬜ Spec (`DRRP-CLASSIFICATION.md`) trimmed to match: special-case rows that are precedents move to an appendix or the precedent store
- ⬜ The justifier brief rewritten for sentence units and principles (input to phase C)

## Dependencies

- ✅ Catalogue, spec and 240 reviewed provisions with Jason's comments
- ⬜ Phase A's unit definition (for the retired rules); the classification can start before it
