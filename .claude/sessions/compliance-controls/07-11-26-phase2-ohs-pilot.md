---
session: OH&S Controls Pilot
status: active
opened: 2026-07-11
---

# Session: OH&S Controls Pilot (ACTIVE)

## Problem

The pipeline script and prompts are validated on 5 hand-picked laws. Before scaling to ~2K laws, run the full pipeline on the 54 QQ OH&S laws as a family pilot. This tests batch throughput, measures duplication, and reveals failure modes on laws we haven't hand-tuned for.

## Work

1. ✅ Generate controls for QQ OH&S laws (batch mode)
2. ✅ Measure: total controls, controls per law distribution, lint pass rates
3. ✅ Spot-checked 13 laws (3 small, 5 medium, 5 large) — all indicative, one deontic leak (Workplace Regs "must"), load-bearing judgement well-flagged throughout
4. ✅ Intra-law dedup: 2 pairs with >0.6 title similarity across 349 controls — both legitimate (RIDDOR injuries vs dangerous occurrences, CDM pre-construction vs construction phase). Zero actual duplicates.
5. ❌ Cross-law dedup — reframed: controls are law-specific, not duplicates. Similar controls across laws are tube map interchanges for the customer to map, not pipeline dedup. This is Mode 2 (Reconciliation) territory.
6. ✅ Generate policy predicates for the pilot set
7. ✅ Decision: HDBSCAN not needed in generation pipeline. Cross-law similarity is a presentation concern for Mode 2.

## Results

### Batch run: 47 OH&S laws (QQ-scoped)

- 12 skipped (already in staging from Phase 0 + earlier partial run)
- 35 generated, 7 skipped (no governed provisions)
- **1,033 provisions → 263 controls (3.9:1 ratio)**
- 14 validation flags across 12/263 controls (4.6% flag rate)
- 1 predicate JSON parse error (UK_ukpga_1920_65 — tiny 1920 Act)

### Validation flags

| Type | Count | Severity |
|------|-------|----------|
| JUDGEMENT_MISSING | 11 | Low — term in text but not in field |
| DEONTIC | 1 | Medium — "must" in title (Workplace Regs) |
| INVALID_REF | 1 | Low — excluded provision referenced |
| PAPERWORK | 1 | Medium — CDM "document exists" in description |

3/263 structural issues (1.1%). 11/263 soft misses on load_bearing_judgement (4.2%).

## Dependencies

- ✅ Phase 0 prompts validated (system-prompt-v1.md, policy-predicate-prompt-v1.md)
- ✅ Phase 1 pipeline script (scripts/generate_controls.py) with 30 tests
- ✅ Explanatory Note backfilled (312/428 QQ laws)
- ✅ Gemini API key available
