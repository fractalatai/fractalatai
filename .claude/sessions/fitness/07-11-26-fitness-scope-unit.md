---
session: Fitness Scope Unit
status: closed
opened: 2026-07-11
closed: 2026-07-11
outcome: success

summary: >
  Phase 3a: populated scope_unit on all 14,258 fitness mentions. Regex extracts
  whether each mention scopes the whole law (6.8%), a Part (0.6%), Chapter (0.1%),
  Schedule (0.05%), or just the provision (92.5%). Validated against benchmark laws.
  Prerequisite for Phase 3b graph propagation.

decisions:
  - what: Scope unit determined by legislative unit as SUBJECT of applicability verb, not qualifiers
    why: "Subject to the provisions of this Part, any person who..." has "this Part" in a qualifier, not as the scope subject. Only "This Part applies to..." means Part-level scope. Prevents over-broadening.
    result: Conservative assignment — 92.5% provision-level. Only explicit "This Part/Act/Chapter applies" gets broader scope.

  - what: Provision-level as default scope for all unmatched patterns
    why: Most conservative — a provision's fitness applies only to itself unless explicitly declared otherwise. Over-propagating is worse than under-propagating for compliance.
    result: 13,189 provisions default to provision scope. Phase 3b propagation only acts on the 1,069 broader-scoped mentions.

metrics:
  scope_distribution:
    provision: { count: 13189, pct: "92.5%" }
    law: { count: 974, pct: "6.8%" }
    part: { count: 95, pct: "0.7%" }
    chapter: { count: 10, pct: "0.1%" }
    schedule: { count: 7, pct: "0.05%" }

lessons:
  - title: Legislative self-references serve dual roles — scope declaration vs qualifier
    detail: "This Part" in "This Part applies to employers" is a scope declaration (Part-level). "This Part" in "Subject to the provisions of this Part" is a qualifier (the provision is scoped to itself, with Part-wide exceptions). The regex must match the legislative unit as the SUBJECT of the applicability verb to distinguish these.
    tag: data

artifacts:
  - .claude/plans/fitness/FITNESS-STRATEGY.md

depends_on:
  - 07-11-26-fitness-slm-extraction.md

enables:
  - Phase 3b graph propagation (scope_unit tells the tree walk which level to propagate at)
---

# Session: Fitness Scope Unit (CLOSED)

## Problem

Phase 3a of FITNESS-STRATEGY.md: determine what structural unit each fitness mention scopes. The `scope_unit` field on fitness_mentions is empty for all 14,258 rows. Without it, the Phase 3b propagation algorithm can't distinguish "This Part applies to employers" (scopes all sections in the Part) from "Subsection (3) does not apply" (scopes only that provision).

The scope_unit comes from the legislative self-reference in the provision text — the same pattern used by the polarity detection's self-reference filter. "This Part" → Part, "These Regulations" → law, "Subsection (3)" → provision.

## Work

1. ✅ Analysed scope_unit patterns: 58.5% numbered refs, 32.7% law-level, 4.2% Part, 0.9% Chapter. Most provisions have multiple self-ref types.
2. ✅ Built regex extraction: LAW_SCOPE_RE, PART_SCOPE_RE, CHAPTER_SCOPE_RE, SCHEDULE_SCOPE_RE — matches legislative unit as SUBJECT of applicability verb, not qualifiers.
3. ✅ Populated scope_unit on all 14,258 mentions: 92.5% provision, 6.8% law, 0.6% Part (with Part number), 0.1% Chapter, 0.05% Schedule.
4. ✅ Validated: Wildlife Act s.27(5) → Part 1 (territorial extension), MCAA s.66(1) → Part 4 (licensable activities). Correct.

## Dependencies

- ✅ Phase 2e: 14,258 mentions with 97.7% entity coverage (07-11-26-fitness-slm-extraction)
- ✅ Legislative self-reference patterns already defined in fitness.rs (SELF_REF_DEMO_RE, SELF_REF_NUM_RE)
- ✅ Hierarchy data in Postgres (hierarchy_path, part, chapter)
