---
session: OH&S Controls Pilot
status: closed
opened: 2026-07-11
closed: 2026-07-11
outcome: success

summary: >
  Ran the controls generation pipeline across 47 QQ OH&S laws. 1,033 provisions
  consolidated to 349 controls at 3.9:1 ratio with 1.1% structural flag rate and
  zero intra-law duplicates. HDBSCAN consolidation dropped — cross-law similarity
  is a Mode 2 presentation concern, not a generation pipeline step.

decisions:
  - what: HDBSCAN consolidation dropped from pipeline
    why: >
      Cross-law controls are law-specific, not duplicates. A COSHH risk assessment
      control and an MHSW risk assessment control are both legitimate — they serve
      different laws. Surfacing similarity is valuable for the customer (tube map
      interchanges) but belongs in Mode 2 (Reconciliation), not the generation pipeline.
    result: Phase 2b removed from build plan. Pipeline stays simple — generate per law, no post-hoc merging.

  - what: Added --qq scope flag and skip-if-exists to generate_controls.py
    why: >
      First batch run processed ALL OH&S laws in the corpus (~hundreds), not just QQ
      applicable laws (~54). Wasted API calls on EU Directives not in the customer
      register. Also needed to avoid re-processing laws already in the staging table.
    result: --qq filters to QQ applicable laws. Skip-if-exists checks staging table, --force overrides.

  - what: Controls are law-specific, not duplicates
    why: >
      User reframed the cross-law dedup question. Each control meets the needs of a
      specific law. Similar controls across laws are tube map interchanges — the customer
      maps their single procedure to multiple law-specific controls. This is reconciliation,
      not deduplication.
    result: Fundamental architecture clarification. Simplifies pipeline, moves complexity to Mode 2.

metrics:
  pilot_batch:
    laws_total: 47
    laws_skipped: 12
    laws_generated: 35
    laws_no_provisions: 7
    provisions_total: 1033
    controls_total: 263
    consolidation_ratio: "3.9:1"
    flag_rate_structural: "1.1%"
    flag_rate_soft: "4.2%"
    deontic_leaks: 1
    intra_law_duplicates: 0
    predicate_errors: 1
  full_staging_table:
    laws: 47
    controls: 349

lessons:
  - title: --family without --qq processes the entire corpus, not just the customer
    detail: >
      The first batch run on OH&S family hit hundreds of laws across the full 19K corpus.
      The QQ customer register has only 54 OH&S laws. Wasted Gemini API calls on EU
      Directives with no provisions. Always scope to the customer register with --qq.
    tag: tooling

  - title: Controls across laws are not duplicates — they are interchanges
    detail: >
      The design doc called cross-law similar controls "tube map interchanges" but the
      build plan still treated them as duplicates to merge via HDBSCAN. The user reframed:
      each control is law-specific and legitimate. Similarity is a presentation concern
      for the customer to map their procedures against, not a pipeline dedup step.
    tag: architecture

  - title: Zero intra-law duplicates at 0.6 title similarity threshold
    detail: >
      Across 349 controls, only 2 pairs had >0.6 title similarity — both legitimate
      (RIDDOR injuries vs dangerous occurrences, CDM pre-construction vs construction
      phase). The LLM's consolidation constraint in the prompt is working — no need
      for post-hoc dedup within a law either.
    tag: methodology

  - title: EU Directives produce valid controls despite being framework legislation
    detail: >
      EU Directives like 89/391/EEC (Framework Directive) produced 13 well-formed
      controls from 49 provisions. The prompt handles both UK SIs and EU Directives
      without modification. Citation style differs (art.Article 5(1) vs reg.3(1))
      but the controls quality is the same.
    tag: methodology

artifacts:
  - scripts/generate_controls.py
  - data/compliance-controls/generated/

depends_on:
  - 07-11-26-phase0-prompt-engineering.md
  - 07-11-26-phase1-pipeline.md

enables:
  - Phase 3 full corpus run
  - Mode 2 Reconciliation design (cross-law similarity surfacing)
---

# Session: OH&S Controls Pilot (CLOSED)

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
