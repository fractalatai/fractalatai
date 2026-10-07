---
session: "Phase C: Gold set at the sentence level"
status: active
opened: 2026-10-07
related: ["parsing/2026-10-06-meta-plan-sentence-units.md", "parsing/2026-10-06-phaseB-principles-precedents.md", "docs/architecture/DRRP-RULE-CATALOGUE.md", "scripts/benchmarks/gold_v3/JUSTIFY_V2.md"]
---

# Session: Phase C: Gold set at the sentence level (ACTIVE)

## Problem

The gold set has 220 reviewed sentences. The rest of the selection (848 sentences, less 23 excluded) still needs labels: **605 sentences**, labelled on catalogue v2 by brief v2, with the 5 nearest precedents in every evidence pack, and reviewed by Jason on the sentence page. Phase C also tops up the scarce classes, checks Jason against himself, writes the scoring spec phase D needs, and freezes the set.

## Todo

- ⬜ **Batch 1** (about 100 sentences): evidence packs (`unit_evidence.py --selection`, the not-yet-gold units), two Opus justifier agents on `JUSTIFY_V2.md`, load (`load_units.py`), export and sync to the sentence page; Jason reviews
- ⬜ Convergence after each batch: approve-unchanged rate, new candidate patterns and dictionary candidates (notes), new principles promoted (target: none without Jason, ~3 provisions from 2+ laws)
- ⬜ Batches 2 to 6 until the 605 are reviewed; stop early if the approve rate holds and no new patterns appear (meta-plan stop signals)
- ⬜ Amending text and out-of-domain sentences that justifiers propose for exclusion → `excluded_units.csv` once Jason agrees
- ⬜ **Top-up** of scarce classes from other held-out laws (not the 60 test laws): REL-27 enforcing-authority designations, `implied-access-right`, `functions-list`, applying-provision holders (HOLD-05 forms), TYPE-03 cases; target ~3 provisions from 2+ laws each so rare precedents can be judged for promotion
- ⬜ **Intra-rater check:** about 50 reviewed sentences re-reviewed blind after a gap; agreement per field
- ⬜ **Row-to-sentence scoring spec** for phase D: how row-level tier outputs (regex, classifier, SLM) combine into one sentence prediction, and how (label, position) pairs and list `act` are scored; context principles (HOLD-05, INF-03) scored apart
- ⬜ Dictionary candidates from notes → the dictionary list (2+ Families rule); `Gvt: Authorised Person` with Family gating (pending task)
- ⬜ **Freeze:** gold-v4-sentence → a frozen version (`gold-v4.0`), with counts per field and class

## Dependencies

- ✅ Phase A: sentence units, the assembler and its QA, the #78 schema
- ✅ Phase B: catalogue v2, the precedent store, `JUSTIFY_V2.md`, the trimmed spec, gold made consistent with v2
- ✅ The 60 test laws repaired by legal (list-text fix pulled 2026-10-07)
- ⬜ Legal's remaining parse repair (batches 4–11, #174 definitions/BlockText). It doesn't touch the 60 test laws' text except for the #174 section-row fix; pull once after it lands (NAS backup first). Top-up laws may change, so take top-up sentences after the pull, or re-check their text
