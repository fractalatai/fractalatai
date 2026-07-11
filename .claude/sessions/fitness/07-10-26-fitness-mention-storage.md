---
session: Fitness Mention Storage
status: closed
opened: 2026-07-10
closed: 2026-07-10
outcome: success

summary: >
  Phase 2a: created fitness_mentions table and independent `fractalaw fitness extract`
  CLI command, fully decoupled from DRRP taxa pipeline. Populated 15,066 mentions across
  701 laws (14,389 provisions). Entity extraction covers 35% — OH&S/FIRE families 60-73%,
  non-safety families 10-22%. The gap quantifies the work for Phase 2d (NER).

decisions:
  - what: Separate fitness CLI command independent of DRRP taxa pipeline
    why: taxa parse tier protection blocks all column writes (including fitness) when a provision has SLM/LLM classification. Fitness and DRRP are different workstreams that should not share write paths.
    result: "`fractalaw fitness extract` and `fractalaw fitness status` — own table, own write path, no tier protection coupling. Gemini Flash confirmed Option A unanimously."

  - what: fitness_mentions as one-to-many table (not JSON blob on LAT row)
    why: A single provision can contain multiple applicability mentions (AppliesTo + DisappliesTo + ExtendsTo). Same pattern as provision_actors.
    result: Separate table with section_id FK, GIN indexes on entities[] and scope_dimensions[]

  - what: Store polarity-only mentions (no entities) alongside entity-bearing mentions
    why: Polarity detection fires on 14,389 provisions but dictionaries only match 5,217. The 9,849 polarity-only mentions mark the gap for NER/SLM — they're the work queue for Phase 2d.
    result: 65% of mentions are polarity-only — the NER training signal

metrics:
  corpus:
    total_mentions: 15066
    provisions_with_mentions: 14389
    laws_with_mentions: 701
    with_entities: 5217
    polarity_only: 9849
    entity_rate: "35%"
  family_entity_rates:
    ohs_personal_safety: "61%"
    fire_dangerous_substances: "60%"
    ohs_mines_quarries: "70%"
    wildlife_countryside: "31%"
    marine_riverine: "21%"
    transport_road_safety: "10%"
    town_country_planning: "13%"
  benchmark_laws:
    wildlife_act: { mentions: 207, with_entities: 66 }
    nerc_act: { mentions: 39, with_entities: 12 }
    mcaa: { mentions: 240, with_entities: 59 }
    habitats_regs: { mentions: 92, with_entities: 30 }

lessons:
  - title: DRRP tier protection silently blocks fitness writes
    detail: The pipeline's source_tier protection skips entire provision updates (including fitness columns) when SLM/LLM data exists. This meant re-parsing laws through taxa parse wrote zero new fitness data. The coupling was invisible until we measured — the pipeline reported success but polarity counts didn't change.
    tag: architecture

  - title: Polarity-only mentions are the NER work queue
    detail: 9,849 mentions have polarity but no dictionary entities. These are the provisions where the law uses applicability language but the OH&S dictionaries find nothing. This is the exact input for NER training — provisions with known applicability signal but unknown content.
    tag: data

  - title: PgStore pool accessor enables direct SQL without adding dependencies
    detail: Added pool() accessor to PgStore and re-exported PgPool from fractalaw-store. The fitness module uses sqlx directly via the pool rather than going through the ProvisionStore trait. Cleaner than adding tokio-postgres as a separate dependency.
    tag: architecture

artifacts:
  - crates/fractalaw-cli/src/commands/fitness.rs
  - crates/fractalaw-cli/src/main.rs
  - crates/fractalaw-store/src/pg.rs
  - crates/fractalaw-store/src/lib.rs
  - crates/fractalaw-cli/Cargo.toml

depends_on:
  - 07-10-26-fitness-applicability-regex.md

enables:
  - Phase 2b entity catalogue and linking (what are the canonical entities?)
  - Phase 2d NER training (9,849 polarity-only mentions = training work queue)
  - Full corpus re-extraction when dictionaries are expanded
  - Fitness coverage reports per customer register
---

# Session: Fitness Mention Storage (CLOSED)

## Problem

Phase 2a of FITNESS-STRATEGY.md: create the mention storage infrastructure and populate it with regex extraction from existing dictionaries. The three-layer model needs a `fitness_mentions` table (one-to-many per provision) before any extraction work can land. Currently fitness data lives in flat arrays on the `legislation_text` row (fitness_polarity, fitness_person, etc.) — this can't represent multiple distinct mentions per provision.

The existing P-dimension dictionaries already extract applicability entities for OH&S/FIRE laws. Running them against the 12,280 provisions with polarity (Phase 1 output) and storing as mentions gives us the baseline for the staircase.

## Work

1. ✅ Design `fitness_mentions` table schema in Postgres (section_id FK, span, polarity, scope_unit, entities[], scope_dimensions[], extraction_method, confidence)
2. ✅ Create the table + indexes (GIN on entities and scope_dimensions, law-level index via split_part)
3. ✅ Migrate existing P-dimension fitness data into mentions (5,946 rows)
4. ✅ Run polarity detection + core dictionaries on all 144K remaining provisions — 8,039 new mentions (13,985 total)
5. ✅ Fix architectural coupling: created `fractalaw fitness extract` CLI command — independent of DRRP taxa pipeline, no tier protection, writes directly to fitness_mentions. Also `fractalaw fitness status` for coverage reports.
6. ✅ Verified on benchmark laws: Wildlife Act 71→207, NERC 8→39, MCAA 89→240, Habitats Regs 26→92. 578 mentions total, 167 with entities.
7. ✅ Full corpus extraction: 15,066 mentions across 701 laws. OH&S/FIRE 60-73% entity rate, non-safety 10-22%.
8. ✅ Benchmark validation: polarity counts tripled vs old gated pipeline. Entity gap quantified as NER work queue (9,849 polarity-only mentions).

## Dependencies

- ✅ Phase 1 polarity detection implemented (07-10-26-fitness-applicability-regex session)
- ✅ fitness.rs has P-dimension dictionaries (OH&S + FIRE specialist)
- ✅ 12,280 provisions with polarity tags across the corpus
- ✅ provision_actors table exists as reference pattern for one-to-many storage
