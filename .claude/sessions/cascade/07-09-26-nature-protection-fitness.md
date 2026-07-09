---
session: Nature Protection Fitness
status: active
opened: 2026-07-09
---

# Session: Nature Protection Fitness (ACTIVE)

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
5. ⬜ Eyeball the 25 existing fitness-tagged provisions — are they correct?
5. ⬜ Phase 1: Improve regex identification of applicability provisions
6. ⬜ Phase 2: SLM prompt design for fitness extraction from applicability provisions
7. ⬜ Build golden benchmark labels for 4 nature protection laws
8. ⬜ Training data from OH&S laws (dictionary results as ground truth)
9. ⬜ Phase 3: Law-level propagation (structural hierarchy)
10. ⬜ Re-parse and measure improvement
11. ⬜ Republish updated fitness data to sertantai

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
