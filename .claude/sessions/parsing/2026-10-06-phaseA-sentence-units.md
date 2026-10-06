---
session: "Phase A: Sentence units"
status: pending
opened: 2026-10-06
related: ["parsing/2026-10-06-meta-plan-sentence-units.md", "parsing/2026-10-06-phase0a-gold-set-scaffolding.md"]
---

# Session: Phase A: Sentence units (PENDING)

## Problem

Labels attached to the source's rows, and a sentence split into a stem and items needed rules for every piece. The legal sentence becomes the unit: a stem ("X shall—", "The notice must specify—") joined with its items and closing words, or a row with no stem above it. First measure (scratchpad `units.py`, a stem is an ancestor whose text ends in a dash or colon): the 61 test laws' 17,837 rows make 10,462 units (2,142 multi-row); the gold selection's 1,000 provisions make 883 units; unit text has a median of 265 characters, p90 718, max 11,880, max 37 rows.

## Todo

- ⬜ Unit definition, written down: what counts as a stem (dash, colon, "the following", "as follows"); nested stems ((2)— (a)— (i)); closing words that the source puts on the stem row; a stem whose items are in a Schedule
- ⬜ Edge cases, with counts and examples: long lists (over 4,000 characters), "and"/"or" joins, items that are themselves full sentences with their own modal ("(b) the Secretary of State shall…"), provisos ("Provided that…") on separate rows
- ⬜ **Decision (Jason):** a whole sentence that only details a duty or power in another sentence ("A notice of appeal shall be accompanied by…"): keep relation `continues` for it, or `no` with a pointer
- ⬜ Decision (Jason): purpose for a unit: its own class, or `inherit` from the section when it has no purpose of its own
- ⬜ The assembler: `scripts/benchmarks/gold_v3/units.py` (unit id = root row id; ordered member rows; assembled text), tested on the known cases (PUWER reg.11(2), Flood Risk s.43(5), EAW reg.16, the 77 units whose items were reviewed)
- ⬜ Counts on the 61 test laws and the selection, written below; the selection re-keyed to units for phase C
- ⬜ Note for phase E: the same assembler in `fractalaw-core` so the pipeline labels sentences

## Dependencies

- ✅ `legislation_text` rows with section ids (stem chain via `drrp_prompt.ancestors`)
- ✅ Phase 0a selection and reviewed rows
