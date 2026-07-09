# Fitness Extraction Strategy (v0.2)

*v0.1 reviewed by Gemini Pro 2026-07-09. Key feedback incorporated: merge implicit applicability into Phase 2, replace rigid 6-P ontology with flexible entity extraction, adopt NER-first staircase, add negative test cases. Full review at `data/code-review/gemini-fitness-strategy-review.md`.*

## The Problem

Fitness answers: "does this law apply to me?" It's the bridge between a law's obligations and a specific customer's operations. Without fitness, every obligation in the corpus looks equally relevant — a quarry operator sees obligations about ships, nuclear installations, and food safety alongside their actual duties.

The current fitness module (`crates/fractalaw-core/src/taxa/fitness.rs`) uses the same pattern as actor extraction: regex dictionaries for 6 P-dimensions (Person, Process, Place, Plant, Property, Sector). This works well for OH&S laws where applicability language is standardised ("every employer", "construction work", "asbestos"), but fails for nature protection and other domains where applicability is expressed in bespoke terms specific to each law.

### Evidence of Failure

Across 4 nature protection benchmark laws (6,644 provisions), only 25 have any fitness tags. The regex dictionaries have no terms for: marine conservation zones, European protected species, SSSIs, wild birds, confined dredging operations, biodiversity duty.

## Applicability Pattern in Legislation

Fitness is mostly declared in **applicability sections** — early provisions that set the scope of the entire law or Part. These are law-level, not provision-level: "This regulation applies to every employer who carries out work involving exposure to vibration" catches all subsequent duties. Individual provisions may narrow or extend the general applicability, but the baseline comes from these scope-setting sections.

**Two types of fitness signal** (Gemini review — must handle both from day one):

1. **Scope Declarations**: "This Part applies to any public authority having..." — explicit scoping language
2. **Subject-Matter Conditions**: "any person who deliberately captures a European protected species" — the applicability is implicit in the duty itself. These are not edge cases — criminal offences and regulatory duties ARE the compliance obligations.

Both types need extraction. They are not separate phases.

## What's There Today

### Polarity Detection (strong, keep)

Regex patterns detect three polarities reliably:
- **AppliesTo**: "shall apply to", "applies in relation to", "applies where"
- **DisappliesTo**: "shall not apply", "does not extend to", "ceases to have effect"
- **ExtendsTo**: "extends to", "shall extend to" (geographic scope)

These are structurally consistent across all legislation. No change needed.

### P-Dimension Dictionaries (OH&S-centric, being replaced)

Six rigid dictionaries (Person, Process, Place, Plant, Property, Sector) with OH&S and FIRE specialist extensions. The ontology doesn't translate across domains — "Plant" means machinery in OH&S but literal plants in nature law. The fixed schema forces bespoke legal concepts into wrong buckets.

**Decision**: Keep the dictionaries as a baseline signal for OH&S/FIRE (where they work), but don't extend them to new domains. New domains use the NER approach (see Phase 2).

### Purpose Detection (key signal)

The purpose classifier identifies `APPLICATION_SCOPE` provisions — the polarity regex only runs on these. This gate is correct but may miss some applicability patterns in nature protection laws. Phase 1 expands this.

## Proposed Approach

### Phase 1: Better Applicability Provision Detection

**Goal**: Find more applicability provisions reliably via regex.

The polarity regex is good but the purpose classifier may not tag all applicability patterns. Add regex patterns for:
- "For the purposes of this Part/Act, ..." (definition of scope)
- "This section/regulation applies to any..." (explicit scope)
- "The following are licensable/prohibited activities..." (activity lists)
- "A person who [verb] ... is guilty of an offence" (implicit subject-matter — criminal provisions)
- Schedule references that define protected species/sites

This expands which provisions get tagged as applicability. Cheap, safe, pure regex.

### Phase 2: Staircase Extraction (NER → Relations → SLM)

**Goal**: Extract what the applicability means, using progressively expensive tools.

Gemini review recommended a "staircase of complexity" rather than jumping straight to generative SLM. This mirrors the taxa pipeline (regex → classifier → SLM → LLM).

#### Step 1: Named Entity Recognition (NER)

Train a lightweight NER model to find and classify applicability entities in provision text:
- `[Public Authority]ACTOR` — who it applies to
- `[Marine Conservation Zone]LEGAL_DESIGNATION` — what legal concept triggers it
- `[UK marine licensing area]GEOGRAPHY` — where it applies
- `[deliberately captures a European protected species]ACTIVITY` — what activity triggers it
- `[England and Wales]TERRITORIAL_EXTENT` — territorial scope

This is cheaper to train than generative models, faster to run, and produces structured entities. Training data from:
- OH&S laws where dictionary results become ground truth labels
- Manual labelling of ~200+ provisions across nature protection benchmark laws (Gemini flagged 50-100 as insufficient)

#### Step 2: Relation Extraction

Once entities are identified, classify their role in the applicability:
- `(Public Authority) -[subjectTo]-> (Biodiversity Duty)` — who bears the duty
- `(Marine Conservation Zone) -[triggerFor]-> (Assessment Obligation)` — what triggers the duty
- `(UK marine licensing area) -[scopedTo]-> (Licensing Regime)` — geographic scope

This can be rule-based initially (if ACTOR + ACTIVITY in same provision with AppliesTo polarity → subjectTo relation) and upgraded to ML later.

#### Step 3: SLM for Complex Cases

Generative SLM only for provisions where NER + relations fail:
- Multi-clause applicability with exceptions
- Cross-referential scope ("a section 28G authority")
- Compound conditions ("applies to employers who employ 5 or more employees in premises where...")

SLM prompt extracts structured entities and relations, not fixed P-dimension slots.

**Entity types replace the rigid 6-P ontology**:
- ACTOR (who) — replaces Person dimension, reuses existing actor dictionary
- ACTIVITY (what you do) — replaces Process
- GEOGRAPHY (where) — replaces Place, with granularity for devolved/marine zones
- LEGAL_DESIGNATION (what legal concept) — new, covers MCZs, SSSIs, EPS, Schedules
- SUBJECT_MATTER (what domain) — replaces Sector, but extracted not dictionary-matched
- CONDITION (qualifying criteria) — replaces Property, covers "5 or more employees" etc.

The OH&S dictionaries map cleanly onto these entity types. Existing fitness data migrates without loss.

### Phase 3: Law-Level Propagation

**Goal**: Applicability declared once, applied to all provisions in scope.

After extracting fitness from applicability provisions:
1. Identify which Part/Chapter/law the applicability scopes
2. Apply the fitness entities to all substantive provisions in that scope
3. Where a provision has its own applicability modifier, merge (narrow or extend)

**Graph structure**: Legislation is not a neat tree — provisions reference schedules, other Acts, and definitions elsewhere (a directed acyclic graph). We already have `law_edges` in DuckDB with amendment/enactment/rescission edges, and `hierarchy_path` + `sort_key` for structural position. Cross-reference detection already exists in `fitness.rs`. The propagation needs to follow both structural hierarchy (Part → sections) and explicit cross-references.

For v1, structural hierarchy propagation handles the 80% case. Graph traversal for cross-references is a later enhancement.

### Negative Fitness (DisappliesTo)

Negative cases are as important as positive. The fitness module already detects DisappliesTo polarity. The extraction pipeline must produce both:
- "This law applies to employers doing construction work" → fitness tags
- "This law does NOT apply to domestic premises" → negative fitness tags
- "Nothing in this Part applies to extraction of minerals by dredging in the Scottish zone" → negative, scoped to Part + geography

Negative fitness feeds directly into customer-level applicability: "does this law apply to MY operations?"

## Training & Benchmark Data

### Golden Benchmarks

Use the 4 nature protection laws as initial benchmarks, plus extend to OH&S laws where existing dictionary results provide ground truth:

**Positive cases** (law applies):
- Wildlife Act s.1: ACTOR=any person, ACTIVITY=killing/disturbing wild birds, GEOGRAPHY=England+Wales
- Habitats Regs reg.43: ACTOR=any person, ACTIVITY=capturing/killing European protected species
- MCAA s.66: ACTIVITY=licensable marine activities (deposit, dredge, construct), GEOGRAPHY=UK marine licensing area
- MCAA s.125: ACTOR=public authority, LEGAL_DESIGNATION=Marine Conservation Zone
- NERC s.40: ACTOR=public authority/statutory undertaker, ACTIVITY=all functions (biodiversity duty)

**Negative cases** (law does NOT apply):
- MCAA s.76: ACTIVITY=extraction of minerals by dredging, GEOGRAPHY=Scottish zone → DisappliesTo
- MCAA s.77: ACTIVITY=petroleum operations → DisappliesTo
- For a software company: Wildlife Act s.1 does not apply (no interaction with wild birds)
- For an offshore operator: onshore-only regulations do not apply

### Training Data Sources

1. **OH&S dictionary results** — existing fitness tags become NER training labels (~5,000 tagged provisions)
2. **Manual labelling** — 200+ provisions across nature protection + environmental protection laws
3. **Negative examples** — DisappliesTo provisions, plus provisions where fitness clearly doesn't match a customer profile

## Success Criteria

### Component-level
- NER model identifies applicability entities with >85% F1 on held-out test set
- Polarity detection maintains current accuracy on new domains
- Propagation correctly scopes law-level applicability to Part boundaries

### System-level (end-to-end)
- For QQ (quarry operator in England): system correctly identifies applicable nature protection obligations AND correctly excludes non-applicable ones
- Precision >80%: provisions tagged as applicable are genuinely applicable
- Recall >70%: genuinely applicable provisions are not missed
- Negative test: for a London software company, nature protection criminal offences are correctly excluded

## Risks

- **NER training data scarcity** — mitigated by bootstrapping from OH&S dictionary results
- **Propagation scope errors** — mitigated by structural hierarchy for v1, graph traversal later
- **SLM hallucination** — mitigated by NER-first staircase (SLM only for complex cases)
- **Entity type evolution** — the 6 entity types may need expansion as new domains are added. Flexible schema (list of typed entities per provision) avoids rigid column proliferation

## Law-Level Fitness (LRT) vs Provision-Level (LAT)

Critical architectural distinction: fitness is a **law-level signal** that lives in DuckDB (LRT), not just a provision-level annotation.

The data flow mirrors taxa triage:
- **LAT** (provision text) is transient — only persisted for laws with obligations that a customer has in their legal register
- **LRT** (law-level metadata) is permanent — every law in the corpus has an LRT record in DuckDB, published to sertantai
- Fitness must be **aggregated from provisions up to law level** and persisted in DuckDB, even when the underlying LAT isn't kept

This means fitness extraction is a two-stage process:
1. **Parse provisions** (when LAT is available) — extract applicability entities from individual provisions
2. **Aggregate to law level** (always) — roll up provision-level fitness into law-level tags in DuckDB

The law-level fitness in LRT is what customers use to decide "does this law apply to me?" — before they ever look at individual provisions. It's the filter that narrows 19K laws to the ~400 in a customer's register. Sertantai needs this signal on every law, not just the ones with persisted LAT.

This is exactly how taxa triage works today: sync-watch ingests LAT, runs triage (regex scan of provisions), writes the result to DuckDB (law-level), and publishes back to sertantai. The LAT may later be cleaned, but the triage decision persists. Fitness follows the same pattern.

## Pipeline Integration

```
LAT arrives (sync-watch or batch)
  → Purpose classifier: is this an applicability provision? (regex gate)
  → If explicit scope declaration:
      → NER extracts typed entities (ACTOR, ACTIVITY, GEOGRAPHY, etc.)
      → Relation extraction links entities to applicability
  → If subject-matter/offence provision:
      → NER extracts ACTIVITY + ACTOR from the duty text itself
  → Propagate to all provisions in structural scope (Part/law)
  → DisappliesTo provisions narrow the propagated scope
  → Aggregate provision fitness → law-level fitness (DuckDB LRT)
  → Publish law-level fitness to sertantai (persists even if LAT is later cleaned)
  → If customer law: also publish provision-level fitness alongside DRRP taxa
```
