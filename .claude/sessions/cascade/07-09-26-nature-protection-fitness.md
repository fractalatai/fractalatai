---
session: Nature Protection Fitness
status: closed
opened: 2026-07-09
closed: 2026-07-09
outcome: success

summary: >
  Scoping session for fitness (applicability) extraction. Reviewed existing
  fitness module (OH&S-only regex dictionaries), identified the gap for nature
  protection laws (25/6,644 provisions tagged), and produced a Gemini-reviewed
  strategy (v0.2) with NER-first staircase, flexible entity types, and law-level
  LRT persistence pattern.

decisions:
  - what: Replace rigid 6-P ontology with flexible typed entities
    why: OH&S P-dimensions don't translate across domains — "Plant" means machinery in OH&S but literal plants in nature law
    result: Six entity types (ACTOR, ACTIVITY, GEOGRAPHY, LEGAL_DESIGNATION, SUBJECT_MATTER, CONDITION) replace fixed schema

  - what: NER-first staircase instead of jumping to generative SLM
    why: Gemini review — generative models are the most expensive and least controllable option. NER is cheaper to train, faster to run, more explainable
    result: Pipeline mirrors taxa cascade — NER → Relations → SLM for complex cases

  - what: Implicit applicability (subject-matter conditions) handled from day one
    why: Gemini review — criminal offence provisions are core compliance obligations, not edge cases
    result: Extraction targets both "this Part applies to..." and "any person who kills a wild bird..."

  - what: Fitness aggregates to law-level LRT, persists in DuckDB
    why: LAT is transient (only kept for customer laws), but fitness signal needed on all 19K laws for applicability filtering
    result: Mirrors taxa triage pattern — parse provisions, aggregate to law, publish to sertantai

lessons:
  - title: Fitness is not like actors — dictionaries don't scale
    detail: Actor extraction works because there's a finite set of duty-bearers. Fitness conditions are bespoke per law. "Marine conservation zone", "European protected species", "licensable marine activity" — each law defines its own applicability vocabulary.
    tag: architecture

  - title: Gemini Pro produces better architectural reviews than Flash
    detail: Used Pro with 16K thinking budget for the strategy review. The depth of critique (DAG structure, temporal dimension, training data contamination risk) justified the cost vs Flash. For code-level reviews Flash is fine.
    tag: tooling

  - title: Two review cycles catches more than one
    detail: v0.1 review identified fundamental flaws. v0.2 review confirmed fixes and surfaced new, more sophisticated risks (training data contamination, relation extraction design). The second review was cheaper but equally valuable.
    tag: methodology

  - title: Don't suspend sessions without being asked
    detail: Attempted to suspend the session prematurely. The user has their own rhythm — wait for the instruction.
    tag: methodology

artifacts:
  - .claude/plans/FITNESS-STRATEGY.md
  - data/code-review/gemini-fitness-strategy-review.md
  - data/code-review/gemini-fitness-strategy-v02-review.md

depends_on:
  - 07-09-26-nature-protection.md

enables:
  - Phase 1 implementation (regex applicability detection)
  - NER training data preparation from OH&S dictionary bootstrap
  - Golden benchmark labelling for nature protection laws
---

# Session: Nature Protection Fitness (CLOSED)

## Problem

QQ's concern: obligations buried inside administrative laws that apply to their operations but aren't obviously about quarrying. The fitness parse (POPIMAR: Person, Organisation, Process, Place, Plant, Property) tags provisions with applicability signals — "does this provision apply to *my* type of workplace/activity?" — but coverage on nature protection laws is near-zero.

Across the 4 benchmark laws from the Nature Protection session, only 25 of 6,644 provisions have any fitness tags. The fitness dictionary likely has no nature/wildlife/habitat terms, so the regex extraction finds nothing.

Benchmark laws:
- **UK_ukpga_1981_69** — Wildlife and Countryside Act 1981 (WILDLIFE & COUNTRYSIDE) — 16 provisions with fitness
- **UK_ukpga_2009_23** — Marine and Coastal Access Act 2009 (MARINE & RIVERINE) — 8 provisions
- **UK_uksi_2017_1012** — Conservation of Habitats and Species Regs 2017 (WILDLIFE & COUNTRYSIDE) — 1 provision
- **UK_ukpga_2006_16** — NERC Act 2006 (X: No Family) — 0 provisions

Target families: WILDLIFE & COUNTRYSIDE, MARINE & RIVERINE, ENVIRONMENTAL PROTECTION

## Applicability Pattern

Fitness (who/what does this law apply to) is mostly declared in **applicability sections** — early provisions that state the scope of the entire law. These are law-level, not provision-level: "This regulation applies to every employer who carries out work involving exposure to vibration" catches all subsequent duties. Individual provisions may narrow or extend the general applicability, but the baseline comes from these scope-setting sections.

This means fitness extraction shouldn't only work provision-by-provision — it needs to identify the law's applicability sections and propagate their fitness tags to all substantive provisions in the law. The DRY principle: the law states applicability once, the pipeline should too.

## Architecture Thinking

Fitness is not like actors. Actors have a finite dictionary — employer, person, authority — that recurs across all legislation. Fitness conditions are bespoke per law: "marine activities", "confined spaces", "extraction of minerals by dredging", "European protected species". A dictionary approach won't scale.

Three dimensions of fitness from applicability sections:
- **Activity** — what you're doing (marine dredging, construction, handling chemicals)
- **Geography** — where it applies (England, Welsh offshore region, Scottish zone, territorial sea)
- **Sector/subject** — what domain (wildlife, marine conservation, workplace safety)

Proposed approach:
1. **Regex to find applicability provisions** — "this Part applies to", "licensable activity", "nothing in this Part applies to" — these are structurally identifiable
2. **SLM to extract the fitness signal** — short text, structured question: "what activities/places/sectors does this law/Part apply to?" Not a dictionary lookup but a comprehension task
3. **Propagate law-level or Part-level** — applicability declared once, applies to all provisions in scope. Not per-provision extraction.

This is closer to triage (law-level classification from key provisions) than to actor parsing (per-provision regex).

## Work

1. ✅ Review existing fitness module — polarity detection (strong), P-dimension dictionaries (OH&S-only)
2. ✅ Examine applicability patterns in 4 benchmark laws
3. ✅ Write meta strategy plan (`.claude/plans/FITNESS-STRATEGY.md`)
4. ✅ Write FITNESS-STRATEGY.md v0.2 — reviewed by Gemini Pro, all major concerns addressed
5. ⏸️ Eyeball the 25 existing fitness-tagged provisions (deferred — new session)
6. ⏸️ Phase 1: Improve regex identification of applicability provisions (deferred — new session)
7. ⏸️ Phase 2: NER staircase for fitness extraction (deferred — new session)
8. ⏸️ Build golden benchmark labels for 4 nature protection laws (deferred — new session)
9. ⏸️ Training data from OH&S laws (deferred — new session)
10. ⏸️ Phase 3: Law-level propagation (deferred — new session)
11. ⏸️ Re-parse and measure improvement (deferred — new session)
12. ⏸️ Republish updated fitness data (deferred — new session)

## Gemini Review Feedback (2026-07-09)

v0.1 reviewed by Gemini Pro — harsh but productive. Five major concerns raised. v0.2 addressed all but temporal (already handled at law level via commencement/revocation data in DuckDB). Two new risks flagged:

1. **Training data contamination** — OH&S dictionary bootstrap may bake in legacy errors. Need manual review of labels.
2. **Relation extraction underspecified** — rule-based entity linking needs concrete design before implementation.

Full reviews: `data/code-review/gemini-fitness-strategy-review.md` (v0.1) and `gemini-fitness-strategy-v02-review.md` (v0.2).

## Dependencies

- ✅ Nature Protection session closed — 4 laws fully enriched with DRRP + significance
- ✅ Fitness module exists (`crates/fractalaw-core/src/taxa/fitness.rs`)
- ✅ POPIMAR columns in Postgres and publish payload
- ✅ FITNESS-STRATEGY.md v0.2 reviewed and approved by Gemini Pro
