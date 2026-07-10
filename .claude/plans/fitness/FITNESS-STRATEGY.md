# Fitness Extraction Strategy (v0.3)

*v0.1 reviewed by Gemini Pro 2026-07-09. v0.2 incorporated review feedback: NER-first staircase, flexible entity types, implicit applicability from day one. v0.3 replaces the typed-entity model with a three-layer architecture that separates extraction from classification, informed by Phase 1 implementation and legal informatics research.*

*Reviews: `data/code-review/gemini-fitness-strategy-review.md` (v0.1), `data/code-review/gemini-fitness-strategy-v02-review.md` (v0.2).*

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

### Phase 1: Polarity Detection (implemented)

**Goal**: Reliably find provisions that carry applicability language across the full corpus.

Polarity detection is separated from entity extraction — it's a cheap regex pass that flags provisions for downstream processing. Runs on ALL provisions, not gated by the purpose classifier.

**Implemented** (fitness-applicability-regex session, 2026-07-10):

1. `detect_polarity()` made public, decoupled from `extract()` in `fitness.rs`
2. Legislative self-reference filter: requires the law to be the subject of "apply" — "this Part shall apply" fires, "risks which apply" does not. Dictionary of legislative nouns: Act, Part, section, regulation, paragraph, article, Schedule, Order, Rules, Directive, provisions, Chapter.
3. Expanded patterns beyond "shall apply" / "does not apply":
   - `SUBJECT_TO_RE`: "Subject to the provisions of this Part" — scoping preamble for criminal offences
   - `ACTIVITY_SCOPE_RE`: "it is a licensable marine activity" — activity scope definitions
4. Pipeline calls `detect_polarity()` on every provision independently of DRRP parse

**Result**: 5,890 → 12,280 provisions with polarity (+108%). Key benchmark provisions: 1/8 → 6/8 detected. Two remaining (reg.43, s.40) have no applicability language — they are implicit subject-matter conditions, handled by Layer 1 extraction in Phase 2.

**Not in scope for Phase 1**: criminal offence provisions ("a person who ... is guilty of an offence") contain implicit applicability but no polarity language. These are extracted as mentions in Phase 2, not flagged by polarity detection.

### Phase 2: Three-Layer Extraction

**Goal**: Extract what the applicability means. Separate extraction from classification.

#### Why v0.2's typed entities don't work

v0.2 proposed 6 entity types (ACTOR, ACTIVITY, GEOGRAPHY, LEGAL_DESIGNATION, SUBJECT_MATTER, CONDITION) to replace the 6 P-dimensions. But both are classification schemas applied at extraction time. The problem:

- "public authority having any function capable of affecting the protected features of an MCZ" — is this ACTOR, ACTIVITY, or LEGAL_DESIGNATION? It's all three simultaneously.
- "every employer who carries out construction work" — is the entity "employer" (ACTOR) or "construction work" (ACTIVITY) or the whole phrase?
- Forcing an entity into one type loses information. The entity IS the applicability subject — it cuts across types.

Both v0.1 and v0.2 conflate extraction with classification. The three-layer model separates them.

#### Layer 1: Mention (extraction)

Extract the **verbatim text span** from the provision. This is what the law actually says — the applicability subject as written.

```
mention:
  provision:  UK_uksi_1999_3242:reg.3(1)
  span:       "every employer who carries out construction work"
  polarity:   AppliesTo
  scope_unit: "these Regulations"    -- what unit of law this scopes (Part, Act, reg)
```

The span is the ground truth. It doesn't change when classification schemes evolve. Extraction can be done by NER (staircase approach: regex → NER → SLM for complex cases).

**Storage**: a provision can contain multiple mentions (AppliesTo + DisappliesTo + ExtendsTo in one text). This is a one-to-many relationship — mentions belong in a dedicated table with a foreign key to provisions, following the same pattern as `provision_actors`.

#### Layer 2: Entity (linking)

Resolve each mention to one or more **canonical entities**. An entity is a normalised identity that recurs across laws — "employer" appears in hundreds of laws, always meaning the same thing.

```
link:
  mention:    "every employer who carries out construction work"
  entities:
    - uri: employer                  confidence: 0.95  source: dictionary
    - uri: construction_work         confidence: 0.90  source: dictionary
```

Linking separates what the law says (mention) from what it means (entity). A single mention can resolve to multiple entities. The link has a confidence score and provenance (dictionary, NER, SLM, manual).

Existing OH&S dictionaries already do this for their domain — the Person dict maps "employer(?:s)?" → "employer". The difference is that linking is now a separate step, not fused with extraction.

#### Layer 3: Classification

Assign labels to each entity from one or more classification schemes. An entity can carry multiple labels.

The **grounding ontology** is the four scope dimensions from legal scholarship — these are the foundational, domain-agnostic classification that applies to all legislation:

- **Personal** (ratione personae) — who: "every employer", "any person", "public authority"
- **Material** (ratione materiae) — what: "construction work", "kills a wild bird", "licensable marine activity"
- **Territorial** (ratione loci) — where: "England only", "UK marine licensing area", "MCZ"
- **Temporal** (ratione temporis) — when: commencement dates, sunset clauses

These are not entity types — they are dimensions of the applicability rule. A single mention can span multiple dimensions. Every entity gets at least one scope dimension label.

```
entity "employer":
  scope_dimension: personal
  actor_type: governed

entity "construction_work":
  scope_dimension: material
  hse_activity: construction

entity "marine_conservation_zone":
  scope_dimension: [territorial, material]   -- a place AND a legal concept
  defra_designation: mcz
```

**Domain-specific extensions** layer on top of the grounding ontology as needed: SIC codes, HSE activity classifications, DEFRA habitat designations, devolved jurisdiction codes. These are added as the system encounters new domains — no schema migrations, just new labels on existing entities.

**References**: The four scope dimensions are standard in EU legislative drafting methodology and legal informatics (Akoma Ntoso `<scope>` elements, LKIF Core Ontology `applies-to` relations, LegalRuleML `<lrml:appliesTo>` context bindings). They provide a universal structure that has been validated across multiple legal traditions.

#### How customer matching works

Customer matching — resolving "does this law apply to me?" from extracted fitness data against a customer profile — is a separate design problem requiring a rules engine that can handle boolean logic (OR conditions), hierarchical matching (SIC 08.11 vs SIC section B), negation (DisappliesTo set subtraction), and conditional applicability ("applies to employers *if* they handle asbestos").

Simple facet intersection is insufficient. See `FITNESS-RULES-ENGINE.md` for the matching design (separate plan).

#### Staircase extraction

The NER-first staircase applies across all three layers — progressively expensive tools:

1. **Regex** — existing polarity detection + dictionaries. Cheap, runs on every provision.
2. **NER model** — trained to extract applicability spans. Medium cost, runs on provisions with polarity.
3. **SLM** — generative extraction for complex multi-clause applicability. Expensive, only for NER failures.

Layer 2 linking and Layer 3 classification can also use the staircase: dictionary linking first, SLM linking for unknown entities.

#### Migration from P-dimensions

The existing OH&S data maps cleanly:

| P-dimension | → Entity URI | → Facets |
|---|---|---|
| Person: "employer" | `employer` | scope=personal, actor_type=governed |
| Process: "construction work" | `construction_work` | scope=material, hse_activity=construction |
| Place: "England" | `england` | scope=territorial, jurisdiction=england |
| Plant: "asbestos" | `asbestos` | scope=material, hazard_type=substance |
| Property: "at work" | `at_work` | scope=conditional |
| Sector: "construction" | `construction` | sic_section=F |

No data is lost. The rigid columns become flexible facets.

#### Sub-phases

Phase 2 is too large for a single session. Each sub-phase produces usable output and can be validated independently.

**Phase 2a: Mention Storage + Regex Extraction**
- Create `fitness_mentions` table in Postgres (section_id FK, span, polarity, scope_unit, extraction_method, confidence)
- Run existing P-dimension dictionaries against all provisions with polarity (the 12,280 from Phase 1)
- Store results as mentions — the existing dictionaries become the regex tier of the staircase
- Migrate existing fitness_* column data into the new table
- Measure: how many provisions get mentions, what's the coverage per family
- This gives us the baseline for Layers 1+2 using existing tools

**Phase 2b: Entity Catalogue + Linking**
- Define the canonical entity catalogue — start from existing dictionary terms + terms discovered in Phase 1 corpus mining
- Create `fitness_entities` and `fitness_entity_links` tables
- Link mentions to entities (dictionary lookup for known terms)
- Identify the gap: mentions that don't link to any known entity (these are the unknown terms that need NER/SLM)
- Measure: link rate, gap size per family

**Phase 2c: Scope Dimension Classification**
- Assign the four scope dimensions to each entity in the catalogue
- For existing dictionary entities this is a manual/rule-based mapping (Person→personal, Process→material, Place→territorial, etc.)
- For new entities discovered in 2b, use heuristics or SLM
- Validate against benchmark laws: do the scope dimension labels make sense?

**Phase 2d: NER Training + Cross-Domain Extraction**
- Build training data from Phase 2a mention output (OH&S dictionaries as ground truth labels)
- Manual labelling of 200+ provisions across nature protection + environmental protection benchmark laws
- Train NER model to extract applicability spans cross-domain
- Run NER on provisions where regex found polarity but no dictionary matches
- Measure: NER F1 on held-out test, new mentions discovered vs regex-only

**Phase 2e: SLM for Complex Cases**
- Identify provisions where NER fails (complex multi-clause, cross-referential scope)
- Design SLM prompt for structured mention extraction
- Run SLM on NER failures
- Measure: coverage gain, cost per provision

### Negative Fitness (DisappliesTo)

Negative cases are as important as positive. The fitness module already detects DisappliesTo polarity. The extraction pipeline must produce both:
- "This law applies to employers doing construction work" → fitness tags
- "This law does NOT apply to domestic premises" → negative fitness tags
- "Nothing in this Part applies to extraction of minerals by dredging in the Scottish zone" → negative, scoped to Part + geography

Negative fitness feeds directly into customer-level applicability: "does this law apply to MY operations?"

### Phase 3: Applicability Graph Propagation

**Goal**: Applicability declared once, applied to all provisions in scope.

Full design in `FITNESS-GRAPH.md`. Summary:

- **Nodes**: scope units — law, Part, Chapter, section, Schedule
- **Edges**: structural inheritance (Part → sections within), cross-reference overrides (DisappliesTo narrowing), commencement propagation from commencement orders
- **Algorithm**: top-down tree walk using existing `hierarchy_path`, `part`, `chapter` data. Law-level scope → Part-level merges → section-level overrides. No new graph database needed.
- **v1 scope**: intra-law hierarchy + commencement. Inter-law amendment scope inheritance deferred.

### Phase 4: Rule Compiler (design spike required)

**Goal**: Compile extracted mentions into boolean expression trees per law.

Full design in `FITNESS-RULES-ENGINE.md`. This phase bridges extraction (Phase 2) and evaluation (Phase 5). The compiler turns a set of mentions from provisions into a structured expression tree.

**Critical challenge** (Gemini v0.3 system review): inferring logical connectives between co-occurring mentions. "Any employer or self-employed person, except domestic premises" → three mentions, but the compiler must know employer/self-employed are OR, and domestic premises is NOT. This requires understanding sentence structure, not just span extraction.

**Design spike** before implementation:
1. Take 20-30 complex applicability clauses from the corpus
2. Manually write the target expression trees
3. Determine whether heuristics suffice or NLP tooling (dependency parsing, semantic role labelling) is needed
4. Assess expected accuracy

**Special cases to handle**:
- "Any person" = wildcard on personal scope dimension (matches all customers unless negated)
- Numeric thresholds ("5 or more employees") — v1: extract as CONDITION mention, v2: add numeric comparison to `Match` node
- "Crown application" provisions — government-facing, never matches private employers

### Phase 5: Rules Engine (sertantai, query-time)

**Goal**: Evaluate compiled expression trees against customer profiles to answer "does this law apply to me?"

Full design in `FITNESS-RULES-ENGINE.md`. Summary:

- **Stage 1**: coarse filter via inverted entity index + hierarchy expansion (19K → ~800 laws)
- **Stage 2**: expression tree evaluation per candidate law — recursive walk of `ApplicabilityNode` tree
- **Compile/evaluate split**: fractalaw compiles the tree at enrichment time, publishes as JSON in LRT payload via Zenoh. Sertantai evaluates at query time in Elixir.
- **Confidence**: extraction confidence propagated per tree node. `max()` for OR nodes, `min()` for AND nodes. High (>0.9) = included, medium (0.5-0.9) = flagged for review, low (<0.5) = excluded.
- **Hierarchy expansion**: SIC tree, jurisdiction tree, HSE activity codes — reference data managed in sertantai

## Training & Benchmark Data

### Golden Benchmarks

Use the 4 nature protection laws as initial benchmarks, plus extend to OH&S laws where existing dictionary results provide ground truth.

**Positive cases** (law applies) — shown in three-layer format:

Wildlife Act s.1(1):
- Mention: "any person who intentionally kills, injures or takes any wild bird"
- Entities: `any_person`, `wild_bird`, `killing_injuring_taking`
- Facets: personal=any_person, material=wild_bird+killing, territorial=England+Wales

Habitats Regs reg.43(1):
- Mention: "a person who deliberately captures, injures or kills any wild animal of a European protected species"
- Entities: `any_person`, `european_protected_species`, `capturing_killing`
- Facets: personal=any_person, material=european_protected_species+capturing

MCAA s.66(1):
- Mention: "it is a licensable marine activity to do any of the following — deposit, dredge, construct..."
- Entities: `licensable_marine_activity`, `deposit`, `dredge`, `construct`
- Facets: material=licensable_marine_activity, territorial=uk_marine_licensing_area

MCAA s.125(1):
- Mention: "any public authority having any function capable of affecting the protected features of an MCZ"
- Entities: `public_authority`, `marine_conservation_zone`, `protected_features`
- Facets: personal=public_authority, material=protected_features, territorial=MCZ

NERC s.40(1):
- Mention: "a public authority which has any functions exercisable in relation to England"
- Entities: `public_authority`, `biodiversity`
- Facets: personal=public_authority, material=biodiversity, territorial=england

**Negative cases** (DisappliesTo):
- MCAA s.76: mention="extraction of minerals by dredging in the Scottish zone", polarity=DisappliesTo
- MCAA s.77: mention="petroleum operations", polarity=DisappliesTo
- System-level: for a software company, Wildlife Act s.1 should score zero overlap on material facets

### Training Data Sources

1. **OH&S dictionary results** — existing fitness tags become NER training labels (~5,000 tagged provisions)
2. **Manual labelling** — 200+ provisions across nature protection + environmental protection laws
3. **Negative examples** — DisappliesTo provisions, plus provisions where fitness clearly doesn't match a customer profile

## Success Criteria

### Phase 1 (polarity detection)
- Polarity detection covers >90% of provisions with applicability language across all families (not just OH&S)
- False positive rate <10% (legislative self-reference filter)
- Temporal applicability (commencement/sunset) detected

### Phase 2 (three-layer extraction)
- NER model identifies applicability spans with >85% F1 on held-out test set
- Entity linking resolves >90% of mentions to canonical entities
- Scope dimension assignment covers all four dimensions

### Phase 3 (graph propagation)
- Propagation correctly scopes law-level applicability to Part boundaries
- DisappliesTo correctly narrows inherited scope
- Commencement dates propagated from commencement orders

### Phase 4 (rule compiler)
- Design spike: 20-30 complex applicability clauses compiled into correct expression trees
- Logical connective inference accuracy assessed and documented
- Numeric thresholds extracted as CONDITION mentions

### Phase 5 (rules engine — sertantai)
- For QQ (quarry operator in England): system correctly identifies applicable nature protection obligations AND correctly excludes non-applicable ones
- Precision >80%: laws tagged as applicable are genuinely applicable
- Recall >70%: genuinely applicable laws are not missed
- Negative test: for a London software company, nature protection criminal offences are correctly excluded
- "Any person" provisions match all governed customers unless negated

## Risks

- **NER training data scarcity** — mitigated by bootstrapping from OH&S dictionary results as Layer 1 mentions + Layer 2 links
- **Propagation scope errors** — mitigated by structural hierarchy for v1, graph traversal later
- **SLM hallucination** — mitigated by NER-first staircase (SLM only for complex cases)
- **Facet scheme proliferation** — mitigated by starting with scope dimensions only, adding domain facets (SIC, DEFRA, HSE) incrementally as customer matching demands them
- **Training data contamination** (Gemini v0.2 review) — OH&S dictionary bootstrap may bake in legacy errors. Layer separation helps: mention extraction can be validated independently of facet assignment
- **Rule compiler feasibility** (Gemini v0.3 review) — inferring logical connectives (OR vs AND) between co-occurring mentions requires sentence structure analysis, not just span extraction. Design spike required before Phase 4 implementation
- **Numeric thresholds** — "5 or more employees", "greater than 1 tonne per year" — the `Match` node handles categorical codes but not numeric comparisons. v1 extracts these as CONDITION mentions; v2 extends the node type
- **Temporal logic complexity** — `TimeWindow` covers commencement/sunset but not conditional temporal ("applies if the building was constructed before 1999"). v2+ concern

## Entity Feedback Loop (cross-cutting)

Entities discovered by higher tiers (NER, SLM, manual) feed back into the regex tier as family-gated dictionary entries. This is the staircase running in reverse — expensive tiers discover, cheap tiers absorb. Over time, the regex tier gets better and the expensive tiers have less work.

### Pattern: family-gated fitness dictionaries

Mirrors the existing actor dictionaries, which have core terms (employer, worker) that run everywhere plus family-scoped specialists (lifting operations → OH&S, fireworks → FIRE).

For fitness entities:

| Dictionary | Scope | Example entities |
|---|---|---|
| **Core** | all families | employer, England, construction work, premises, at work |
| **WILDLIFE & COUNTRYSIDE** | family-gated | wild bird, European protected species, SSSI, wild animal, Schedule 1/5 species |
| **MARINE & RIVERINE** | family-gated | marine conservation zone, licensable marine activity, UK marine licensing area, MCZ |
| **ENVIRONMENTAL PROTECTION** | family-gated | contaminated land, environmental permit, waste operation, discharge consent |
| **PLANNING & INFRASTRUCTURE** | family-gated | planning permission, development consent, listed building, conservation area |
| **WATER & WASTEWATER** | family-gated | water undertaker, sewerage undertaker, water abstraction, flood risk |
| **ENERGY** | family-gated | generating station, energy performance, renewable energy |

### Promotion criteria

An entity discovered by NER/SLM is promoted to a dictionary when:

1. It appears in **3+ laws** within the same family (not a one-off bespoke term)
2. Extraction confidence is **>0.8** across those appearances
3. It resolves to a **canonical entity** in the entity catalogue (Phase 2b)

Promotion can be automated (batch job after NER/SLM runs) or manual (analyst review of candidate terms). The dictionary is a YAML file compiled into the binary, same as the actor dictionary — changes require a rebuild but are version-controlled and auditable.

### Flow

```
Phase 2d/2e: NER or SLM discovers "marine conservation zone" in MCAA
  → appears in 5 MARINE & RIVERINE laws with confidence >0.9
  → promoted to MARINE & RIVERINE specialist dictionary
  → next `fitness extract` run catches it via regex (Phase 2a)
  → NER/SLM no longer needed for that term in that family
```

This closes the feedback loop concern from the Gemini v0.3 review. It's not model retraining — it's dictionary growth from observed entities, gated by frequency and confidence.

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
Fractalaw (enrichment-time, Rust):

  LAT arrives (sync-watch or batch)
    → Phase 1: Polarity detection on ALL provisions (no purpose gate)
        → flags provisions with applicability language
        → records polarity (AppliesTo / DisappliesTo / ExtendsTo)
        → includes temporal: commencement, sunset (ceases to have effect)
    → Phase 2 Layer 1: Extract mentions from flagged provisions
        → regex dictionaries (OH&S/FIRE — existing)
        → NER model (cross-domain — new)
        → SLM for complex cases
        → store in mentions table (one-to-many per provision)
    → Phase 2 Layer 2: Link mentions to canonical entities
        → dictionary lookup (existing terms)
        → SLM for unknown entities
    → Phase 2 Layer 3: Assign scope dimensions to entities
        → grounding ontology: personal / material / territorial / temporal
        → domain-specific facets added incrementally
    → Phase 3: Propagate scope via hierarchy tree walk
        → law-level → Part-level → section-level
        → DisappliesTo narrows, ExtendsTo widens
    → Phase 4: Compile expression tree per law from propagated mentions
        → infer logical connectives (OR/AND/NOT) between mentions
        → produce ApplicabilityNode JSON tree
    → Aggregate + publish:
        → law-level fitness → DuckDB LRT
        → compiled expression tree + entity index → sertantai via Zenoh
        → provision-level mentions → sertantai (customer laws only)

Sertantai (query-time, Elixir):

  Customer asks "what applies to me?"
    → Phase 5: Evaluate
        → Stage 1: coarse filter via inverted entity index + hierarchy expansion
        → Stage 2: expression tree evaluation per candidate law
        → confidence scoring: high = included, medium = flagged, low = excluded
        → present results with review flags
```
