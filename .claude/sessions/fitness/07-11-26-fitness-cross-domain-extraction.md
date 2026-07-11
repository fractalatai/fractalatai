---
session: Fitness Cross-Domain Extraction
status: closed
opened: 2026-07-11
closed: 2026-07-11
outcome: success

summary: >
  Expanded fitness dictionaries from OH&S/FIRE-only to 12 family-scoped domains.
  Added 17 cross-domain actors to core Person dict, 6 geographic terms to Place dict,
  and 10 new specialist dictionaries. Entity extraction rate jumped from 35% to 53%
  (gap 9,849 → 6,671) with zero ML. Updated fitness extract CLI to use law family
  from DuckDB for specialist dictionary selection.

decisions:
  - what: Add cross-domain actors (Secretary of State, local authority, etc.) to core Person dict rather than family-scoping them
    why: These actors appear across all legislation families — they're not domain-specific. Secretary of State appears in 747 gap provisions spanning every family.
    result: 17 new core Person entries, biggest single contributor to gap closure

  - what: Family-scope specialist dictionaries for domain-specific terms
    why: "Marine conservation zone" is only relevant in MARINE family, "listed building" in HISTORIC/PLANNING. Matches the OH&S/FIRE specialist pattern already proven.
    result: 10 new specialist dictionaries covering Wildlife, Marine, Environmental Protection, Planning, Water, Energy, Climate, Nuclear

  - what: Fitness extract loads law→family mapping from DuckDB
    why: specialist_dicts_for() needs the family to select the right dictionaries. The DRRP pipeline gets family from DuckDB — fitness should too.
    result: 13,476 law→family mappings loaded at extract start

metrics:
  entity_rate: { before: "35%", after: "53%", delta: "+18pp" }
  gap: { before: 9849, after: 6671, reduction: "32%" }
  mentions: { total: 14258, with_entities: 7587, polarity_only: 6671 }
  dictionaries_added: 10
  core_person_terms_added: 17
  core_place_terms_added: 6
  biggest_improvements:
    planning_infrastructure: { before: "22%", after: "61%", delta: "+39pp" }
    historic_environment: { before: "14%", after: "50%", delta: "+36pp" }
    town_country_planning: { before: "13%", after: "44%", delta: "+31pp" }
    marine_riverine: { before: "21%", after: "50%", delta: "+29pp" }
    wildlife_countryside: { before: "31%", after: "58%", delta: "+27pp" }

lessons:
  - title: Cross-domain actor terms deliver more value than domain-specific terms
    detail: "Secretary of State" alone hit 747 gap provisions. The 17 core Person entries (government actors, institutional authorities) closed more of the gap than all 10 specialist dictionaries combined, because they appear across every family.
    tag: data

  - title: Dictionary expansion before NER closes 32% of the gap at zero cost
    detail: The entity feedback loop (strategy section) works as designed — corpus mining surfaced terms, terms got added to dictionaries, dictionaries closed the gap. No model training, no GPU time, no labelling. NER should only target the remaining 47% that dictionaries genuinely can't reach.
    tag: methodology

  - title: fitness extract needs DuckDB for family lookup — adds a dependency
    detail: The fitness CLI was designed to be independent of DRRP, but it now needs DuckDB to look up law families for specialist dictionary selection. This is the right trade-off — the family data lives in DuckDB, not Postgres. The dependency is read-only and lightweight.
    tag: architecture

artifacts:
  - crates/fractalaw-core/src/taxa/fitness.rs
  - crates/fractalaw-cli/src/commands/fitness.rs
  - crates/fractalaw-cli/src/main.rs

depends_on:
  - 07-11-26-fitness-entity-catalogue.md
  - 07-10-26-fitness-mention-storage.md

enables:
  - Phase 2d-NER training (remaining 6,671 gap provisions as work queue)
  - Further dictionary expansion via entity feedback loop (add more families)
  - Transport: Road Safety specialist dictionary (worst remaining gap at 81%)
---

# Session: Fitness Cross-Domain Extraction (CLOSED)

## Problem

Phase 2d of FITNESS-STRATEGY.md: close the entity extraction gap outside OH&S/FIRE families. 9,849 polarity-only mentions (65% of all mentions) have applicability language detected but no entity extraction. The gap is worst in Transport (90%), Planning (87%), Building Safety (86%), Historic Environment (86%).

Before NER training, there's low-hanging fruit: 64 corpus-mined entities are in the entity catalogue but NOT in the regex extraction dictionaries. Adding them would close 1,967/9,849 (20%) of the gap with zero ML. The feedback loop from the strategy: entities surface in analysis, get promoted to regex dictionaries.

## Work

1. ✅ Expanded core Person dict: +17 cross-domain actors (Secretary of State, local authority, national park authority, Welsh/Scottish Ministers, etc.)
2. ✅ Expanded core Place dict: +6 terms (national park, conservation area, inland waters, AONB, EEZ, coastal waters)
3. ✅ Added 10 family-scoped specialist dictionaries: Wildlife (3 dicts), Marine (2), Environmental Protection (2), Planning/Historic (2), Water (1), Energy (1), Climate (1), Nuclear (1)
4. ✅ Updated `fitness extract` to look up law family from DuckDB and pass to `fitness::extract()`
5. ✅ Re-ran full corpus: entity extraction 35% → 53%. Gap 9,849 → 6,671 (32% reduction, zero ML).
6. ⏸️ NER training for remaining 6,671 gap (deferred — separate session, Phase 2d-NER)
7. ⏸️ Build training data from dictionary-extracted mentions as ground truth labels (deferred — Phase 2d-NER)

## Dependencies

- ✅ Phase 2b entity catalogue: 228 entities, 64 corpus-mined (07-11-26-fitness-entity-catalogue)
- ✅ Phase 2a fitness_mentions: 15,066 mentions, 9,849 polarity-only gap (07-10-26-fitness-mention-storage)
- ✅ Independent `fitness extract` CLI command (no DRRP coupling)
- ✅ Gap measurement: top 30 corpus-mined entities identified with hit counts
