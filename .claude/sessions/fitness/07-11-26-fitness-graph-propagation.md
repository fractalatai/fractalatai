---
session: Fitness Graph Propagation
status: closed
opened: 2026-07-11
closed: 2026-07-11
outcome: success

summary: >
  Phase 3b: propagated broader-scope fitness mentions down to child provisions.
  Law-level (334 laws) and Part-level (62 mentions) propagation created 408,478
  inherited mentions. Corpus fitness coverage jumped from 9.4% to 62.7%
  (14,204 → 94,240 provisions).

decisions:
  - what: Propagated mentions get discounted confidence (80% of source)
    why: Inherited applicability is less certain than direct — the source mention declares scope for the Part/law, but individual provisions may have unstated overrides.
    result: Propagated confidence = source confidence * 0.8

  - what: Source mention ID stored in source_detail for traceability
    why: Each propagated mention links back to the exact provision that declared the broader scope. Enables audit trail ("why does this provision have fitness? Because s.3(1) declared law-level scope").
    result: source_detail = 'law_scope:UK_uksi_...:s.3(1)' or 'part_scope:...'

  - what: Laws without broader-scope declarations get no propagation
    why: Conservative — if a law doesn't say "these Regulations apply to...", we don't assume applicability. NERC Act and Habitats Regs have no law-level scope declaration, so only provision-level mentions apply.
    result: 37.3% of provisions remain without fitness (56,153) — laws that never declare broad applicability

metrics:
  propagation:
    law_level_mentions: 974
    law_level_laws: 334
    law_level_propagated: 399662
    part_level_mentions: 95
    part_level_propagated: 8816
    total_propagated: 408478
  coverage:
    before: { provisions: 14204, pct: "9.4%" }
    after: { provisions: 94240, pct: "62.7%" }
  benchmark:
    wildlife_act: { before: 207, after: 1441, pct: "69%" }
    mcaa: { before: 240, after: 630, pct: "28%" }
    nerc: { before: 39, after: 39, pct: "5%", note: "no broader-scope declarations" }
    habitats_regs: { before: 92, after: 92, pct: "12%", note: "no broader-scope declarations" }

lessons:
  - title: Not all laws declare broad applicability
    detail: NERC Act and Habitats Regs have no "these Regulations apply to..." provision. Their fitness is entirely provision-level. Propagation only helps laws that explicitly scope at Part or law level. 37.3% of provisions remain without fitness — this is correct, not a gap.
    tag: data

  - title: Law-level propagation creates high row counts
    detail: 974 law-level mentions × ~222 avg provisions per law = 399K propagated rows. Each source mention creates one propagated mention per child provision. A law with 5 scope declarations creates 5 inherited mentions per provision. This is correct but creates a large table.
    tag: architecture

artifacts:
  - .claude/sessions/fitness/07-11-26-fitness-graph-propagation.md

depends_on:
  - 07-11-26-fitness-scope-unit.md

enables:
  - Phase 4 rule compiler (propagated mentions + scope_unit available for expression tree compilation)
  - Law-level fitness aggregation to DuckDB LRT for publish
---

# Session: Fitness Graph Propagation (CLOSED)

## Problem

Phase 3b of FITNESS-STRATEGY.md: propagate broader-scope fitness mentions down to child provisions. 136,189 substantive provisions have no fitness mention. 1,069 mentions have scope broader than their own provision (974 law-level, 95 Part/Chapter/Schedule). "This Part applies to any public authority" should propagate to every section in that Part.

The propagation is a top-down tree walk using existing hierarchy data (hierarchy_path, part, chapter). No new graph database — SQL-driven creation of inherited mentions with extraction_method='propagated'.

## Work

1. ✅ Identified: 334 laws with law-level scope (74K child provisions), 62 Part-scoped mentions (8.8K children)
2. ✅ Law-level propagation: 399,662 mentions created (both regex + slm entities carried)
3. ✅ Part-level propagation: 8,816 mentions created
4. ✅ All propagated mentions have extraction_method='propagated', source_detail links to source mention, confidence discounted to 80%
5. ✅ Corpus coverage: 9.4% → 62.7% (14,204 → 94,240 provisions with fitness)
6. ✅ Benchmark: Wildlife Act 207→1,441 (69%), MCAA 240→630 (28%). NERC/Habitats no propagation (no broader-scope declarations)

## Dependencies

- ✅ Phase 3a: scope_unit populated on all 14,258 mentions (07-11-26-fitness-scope-unit)
- ✅ Hierarchy data in Postgres (hierarchy_path, part, chapter)
- ✅ Per-tier columns prevent overwrite (regex_entities, slm_entities independent)
