---
session: "EU Retained Law Support"
status: closed
opened: 2026-06-05
closed: 2026-06-05
outcome: success

summary: >
  Enabled fractalaw to correctly enrich EU retained laws (Regulations, Directives, Decisions) arriving via Zenoh. Expanded actor dictionaries with 11 EU-specific actors, fixed the APPLICATION_SCOPE classifier for EU structural references, and added EU-specific fitness terms. DRRP extraction rates for focused directives reached 34-49%, on par with UK domestic OH&S laws.

decisions:
  - what: "Add EU actors to core dictionaries, not family-gated"
    why: "Terms like Member State, ECHA, Downstream User are unambiguous -- no risk of false positives in UK domestic text"
    result: "11 new actors; ordering matters (more specific patterns before generic ones to avoid partial matches)"
  - what: "No changes needed for section_id prefix handling"
    why: "art. citations flow through the pipeline without problems -- initial concern about pattern-matching was unfounded"
    result: "EU structural hierarchy (EUTitle/EUChapter) handled without code changes"

lessons:
  - title: "Actor extraction is the gate for DRRP classification"
    detail: "Initial REACH test showed only 11.7% DRRP extraction. Root cause was missing EU actors in dictionaries -- no actor match means no DRRP classification. After adding actors, focused directives hit 34-49%."
    tag: pipeline
  - title: "Large regulations have inherently lower DRRP density"
    detail: "REACH at 13.4% DRRP is not a bug -- bulk procedural and definitional content legitimately lacks duty/right/responsibility/power language."
    tag: domain
  - title: "Scope classifier must handle diverse structural references"
    detail: "EU scope provisions use \"provisions\", \"Directive\", plural references (\"Articles 21, 22\"), \"Paragraphs 1 to 5\" -- UK-only regex missed these patterns."
    tag: pipeline

metrics:
  new_actors: 11
  eu_laws_tested: 5
  reach_drrp_pct: 13.4
  focused_directive_drrp_pct: "34-49"
  eu_laws_with_fitness: 27
  core_tests_passing: 424

artifacts:
  - crates/fractalaw-core/src/taxa/actors.rs
  - crates/fractalaw-core/src/taxa/purpose.rs
  - crates/fractalaw-core/src/taxa/fitness.rs
  - docs/ACTOR-DICTIONARY.md

depends_on:
  - 06-03-26-lat-taxa-fitness-columns.md

enables: []
---

# Session: 2026-06-05 — EU Retained Law Support (CLOSED)

## Context

**Issue**: None
**Briefing**: `data/EU-LAW-SUPPORT-BRIEFING.md`
**Objective**: Enable fractalaw to correctly enrich EU retained laws (Regulations, Directives, Decisions) arriving from sertantai via the existing Zenoh pipeline.

## Problem

Sertantai now parses and serves LAT for EU retained laws. The Arrow IPC wire format and zenoh key expressions are unchanged — the data just arrives. But EU laws have structural and linguistic differences that fractalaw's DRRP and Fitness extraction pipelines don't handle:

1. **`art.` citation prefix** — section_ids use `art.Article 1(1)(a)` instead of `reg.2(1)(a)` or `s.2(1)`
2. **"Article N" provision field** — provision contains `"Article 1"` not just `"1"`
3. **Different regulatory language** — "Member States shall ensure..." (Responsibility) vs "Every employer shall..." (Duty)
4. **Broader fitness scope** — "installations listed in Annex I" vs UK-style "every factory, mine, quarry"
5. **Deeper hierarchy** — EUTitle → EUChapter → Article → Para (mapped to part/chapter/article/sub_article)
6. **New type_codes** — `eur` (Regulation), `eudr` (Directive), `eudn` (Decision)

## Volume

705 EU laws, estimated 60K–125K LAT rows. For comparison: current UK domestic is ~184K rows from ~2,500 laws.

## Making Classification

Pre-classified by sertantai:
- `eur` → making (0.95) — Regulations create direct obligations
- `eudr` → not_making (0.9) — Directives bind Member States, not businesses
- `eudn` → not_making (0.5) — Decisions vary

Fractalaw may confirm or override during enrichment.

## Impact Analysis

### Areas to investigate

1. **section_id parsing** — any code that pattern-matches `reg.`, `s.` prefixes needs to also handle `art.`
2. **provision field extraction** — code that extracts bare provision numbers needs to handle "Article N" prefix
3. **hierarchy_path parsing** — splits on `/` that extract provision components
4. **DRRP actor extraction** — "Member States" as a government actor, EU bodies (Agency, Commission, Authority)
5. **Modal verb patterns** — "Member States shall ensure" = Responsibility, not Duty. Current tier classification may mis-tag these.
6. **Fitness dictionaries** — EU-specific place/sector terms ("installations listed in Annex I", REACH chemical categories)
7. **Purpose classifier** — may not have training data for EU regulatory language
8. **sort_key generation** — `art.Article 1` needs correct normalisation for document order

### Areas that should work unchanged

- Arrow IPC ingest (same schema, same zenoh topics)
- LanceDB upsert (same LAT columns)
- DuckDB LRT upsert (same legislation columns)
- Law-level aggregation (operates on parsed DRRP output, not raw text)
- Provision-level taxa publish (reads from LanceDB, format-agnostic)
- Change tracking (taxa_hash, published_hash, provisions_published_at)

## Test Laws

| Law | Description | Why useful |
|-----|-------------|-----------|
| UK_eur_2006_1907 | REACH | Large (173 articles), many duties on manufacturers/importers |
| UK_eudr_2010_75 | Industrial Emissions Directive | 84 articles, permits. Tests EUChapter structure |
| UK_eur_2008_1272 | CLP Regulation | Chemical classification. Direct duties on suppliers |
| UK_eudr_2012_18 | Seveso III | Major accident hazards. Complex articles + Annexes |

## Progress

### Initial REACH test (before changes)
- 1186 provisions in, 1090 enriched, pipeline completed without errors
- **11.7% DRRP extraction** — very low due to missing EU actors in dictionaries
- Root cause: actor extraction is a gate — no actor match = no DRRP classification
- Modal verb patterns work fine for EU text; the problem is upstream

### Actor dictionary expansion (actors.rs)
Added EU actors to core dictionaries (not family-gated — terms are unambiguous):

**Government actors:**
- `EU: Member State` — "Member States" / "Member State"
- `EU: Agency: ECHA` — "European Chemicals Agency" / "ECHA"
- `EU: Agency: EFSA` — "European Food Safety Authority" / "EFSA"
- `EU: Agency: EEA` — "European Environment Agency" / "EEA"
- (EU Commission was already present)

**Governed actors:**
- `SC: Downstream User` — placed before `Ind: User` to avoid partial match
- `SC: Distributor`
- `SC: Registrant`
- `SC: Applicant`
- `SC: Authorised Representative`
- `SC: Notified Body`

Ordering matters: more specific patterns before generic ones (ECHA before generic Agency, Downstream User before User).

All 424 core tests pass, 0 regressions.

### Enrichment results after actor expansion

| Law | Type | LAT | DRRP | DRRP % | Duties | Resps | Powers | Rights |
|-----|------|-----|------|--------|--------|-------|--------|--------|
| UK_eur_2006_1907 (REACH) | eur | 1,186 | 159 | 13.4% | 105 | 32 | 8 | 9 |
| UK_eur_2008_1272 (CLP) | eur | — | — | 18.8% | — | — | — | — |
| 89/391 Framework Directive | eudr | 122 | 47 | 38.5% | 36 | 8 | 1 | 2 |
| 98/24 Chemical Agents | eudr | 94 | 46 | 48.9% | 32 | 13 | 1 | — |
| 166/2006 E-PRTR | eur | 85 | 29 | 34.1% | 14 | 11 | 4 | — |

Key findings:
- Focused directives hit 34–49% DRRP, on par with UK domestic OH&S laws
- Large regulations (REACH) are lower (~13%) due to bulk procedural/definitional content
- New EU actors (SC: Downstream User, SC: Registrant, EU: Agency: ECHA) matching correctly
- No section_id prefix issues found — `art.` citations flow through without problems
- Pipeline handles EU structural hierarchy (EUTitle/EUChapter) without changes

### APPLICATION_SCOPE classifier fix (purpose.rs)

207 EU scope provisions were missed because the regex only matched UK structural references. Fixed:
- Added "provisions" and "Directive" to self-ref branch
- Handle plural provision references ("Paragraphs 1 to 5", "Articles 21, 22")
- ~4x the fitness-eligible pool (66 → ~273 provisions)

### Fitness dictionary expansion (fitness.rs)

Added EU-specific terms across all dimensions:
- **Plant**: substances, mixtures, polymers, biocidal products, waste
- **Process**: placing on the market, manufacture, work involving hazardous agents, classification and labelling, chemical registration
- **Place**: installation(s), undertaking(s)
- **Person**: registrant, downstream user, applicant, distributor, authorised representative
- **Sector**: cosmetics, food, pharmaceuticals, public/private sector

### Fitness results after changes

| Law | Person | Process | Place | Plant | Sector |
|-----|--------|---------|-------|-------|--------|
| REACH (eur/2006/1907) | 5 | 1 | 1 | 3 | 1 |
| CLP (eur/2008/1272) | 3 | — | 2 | 3 | — |
| Chemical Agents (eudr/1998/24) | 1 | 1 | 1 | 1 | — |
| Waste Dir (eudr/2008/98) | — | — | 2 | 1 | — |
| Seveso/Euratom (eudr/2013/59) | 1 | — | — | 1 | — |

27 of 61 EU laws now have fitness data. 34 remain without — mostly older framework directives that delegate detail to transposing SIs.

## Summary of all changes (3 commits)

1. `1a730c8` — EU actor dictionaries (11 new actors) + `docs/ACTOR-DICTIONARY.md`
2. `af610d4` — APPLICATION_SCOPE classifier fix + fitness dictionary expansion
3. `3808cc4` — Provision-level taxa publish via zenoh (pre-EU, but enables the new data flow)

## Remaining (low priority)

- 34 EU laws without fitness — likely genuinely fitness-sparse framework directives
- Object-centred prohibitions ("substances shall not be manufactured") lack a named actor — would need a new pattern tier
- Monitor ongoing EU law ingestion for new actor gaps
