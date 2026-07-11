---
session: Fitness Entity Catalogue
status: closed
opened: 2026-07-11
closed: 2026-07-11
outcome: success

summary: >
  Phase 2b: built canonical entity catalogue (228 entities) from P-dimension dictionaries
  (164) and corpus-mined terms (64 across 10 domain families). 100% link rate on existing
  mention entities. Gap quantified: 9,849 polarity-only mentions (65%) need NER — worst
  in Transport/Planning/Building families (85-90%), best in OH&S (27-45%).

decisions:
  - what: Entity URI scheme is lowercase underscore-delimited from display name
    why: Simple, deterministic, human-readable. No external ontology dependency.
    result: to_uri("marine conservation zone") → "marine_conservation_zone"

  - what: Scope dimensions assigned at catalogue insertion, not at mention linking
    why: The scope dimension is a property of the entity (employer IS personal scope), not of the mention. Assigning at the entity level means all mentions that link to "employer" inherit personal scope automatically.
    result: 228 entities each have 1-2 scope dimensions. Legal designations (MCZ, SSSI) get both material and territorial.

  - what: Corpus-mined entities are family-gated like the specialist dictionaries
    why: Domain-specific terms should only match in their domain. "Listed building" is Historic Environment, not OH&S. Mirrors the actor dictionary pattern (OH&S/FIRE specialists).
    result: 64 corpus-mined entities across 10 families. 30 core (all families), 34 family-scoped.

metrics:
  catalogue:
    total_entities: 228
    from_dictionaries: 164
    from_corpus_mining: 64
    core_entities: 149
    family_scoped: 79
  linking:
    link_rate: "100%"
    entity_occurrences_linked: 8722
    unlinked_terms: 0
  gap:
    polarity_only_mentions: 9849
    gap_rate: "65%"
    worst_families: { transport_road_safety: "90%", town_country_planning: "87%", building_safety: "86%", historic_environment: "86%" }
    best_families: { hr_working_time: "27%", ohs_mines_quarries: "30%", fire: "31%", ohs_offshore: "35%" }

lessons:
  - title: The entity gap IS the NER work queue with family priorities
    detail: The gap measurement directly tells you where NER adds the most value. Transport/Planning/Building families have 85-90% gaps — these families have zero domain dictionaries. Adding even 20 entities per family would dramatically improve coverage. OH&S at 27-45% gap is already well-served by dictionaries.
    tag: data

  - title: Legal designations are inherently multi-dimensional
    detail: "Marine conservation zone" is both territorial (it's a place) and material (it's a legal concept that triggers specific duties). The catalogue supports this via scope_dimensions[] array — an entity can span multiple dimensions. This validates the v0.3 decision to separate extraction from classification.
    tag: architecture

artifacts:
  - .claude/sessions/cascade/07-11-26-fitness-entity-catalogue.md

depends_on:
  - 07-10-26-fitness-mention-storage.md
  - 07-10-26-fitness-applicability-regex.md

enables:
  - Phase 2c scope dimension classification (entities already have dimensions — this phase may be trivially complete)
  - Phase 2d NER training (gap map identifies priority families for labelling)
  - Entity feedback loop (new NER-discovered entities promote to catalogue)
---

# Session: Fitness Entity Catalogue (CLOSED)

## Problem

Phase 2b of FITNESS-STRATEGY.md: build the canonical entity catalogue and link existing mentions to it. Currently fitness_mentions stores entities as raw string arrays (`{"employer","construction work","England"}`) — there's no canonical identity, no deduplication, no way to know which entities are known vs unknown. 15,066 mentions exist, 5,217 with entity strings, 9,849 polarity-only (the NER gap).

The entity catalogue defines what the system "knows" — every known applicability entity with a canonical URI, scope dimension(s), and provenance. Linking mentions to catalogue entities separates known terms (dictionary-matched) from unknown terms (need NER/SLM). The gap measurement tells us the size of the Phase 2d NER problem.

## Work

1. ✅ Design `fitness_entities` table (uri PK, display_name, scope_dimensions[], source, family_scope)
2. ✅ Seed from P-dimension dictionaries: 164 entities (29 Person, 28 Process, 22 Place, 19 Plant, 8 Property, 13 Sector + OH&S/FIRE specialists)
3. ✅ Add corpus-mined domain entities: 64 new (local authority, national park, MCZ, listed building, wild bird, etc.) across 10 family-scoped domains
4. ✅ Scope dimensions assigned at insert: personal, material, territorial, conditional. Legal designations get multi-dimensional (material + territorial).
5. ✅ Link rate: 100% of existing entity strings map to catalogue entries (8,722 occurrences)
6. ✅ Gap measured: 9,849 polarity-only mentions (65%) need NER. Worst: Transport Road Safety 90%, Town & Country Planning 87%, Building Safety 86%

## Dependencies

- ✅ Phase 2a fitness_mentions table populated (07-10-26-fitness-mention-storage session)
- ✅ 15,066 mentions across 701 laws, 5,217 with entity strings
- ✅ Phase 1 corpus mining identified domain-specific terms (07-10-26-fitness-applicability-regex session)
- ✅ P-dimension dictionaries exist in fitness.rs (core + OH&S + FIRE specialists)
