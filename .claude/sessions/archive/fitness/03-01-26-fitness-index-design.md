---
session: Fitness Index Design
status: closed
opened: 2026-03-01
closed: 2026-03-01
outcome: success

summary: >
  Designed and implemented the fitness (law applicability) extraction pipeline.
  5P model (Person, Process, Place, Plant, Property) + Sector. Built fitness.rs
  module with polarity detection, p-dimension dictionaries, compound provision
  splitting. Tightened APPLICATION_SCOPE regex from 44.5% to 95.5% polarity match.

decisions:
  - what: Adopt 5P model with 6th Sector dimension
    why: Legacy legl 5P model maps well to UK ESH law applicability patterns, but industry/sector needs its own dimension
    result: 6 p-dimension dictionaries with word-boundary regex matching
  - what: Two-tier rule structure (law-level + provision-level)
    why: Law-level rules determine whether law applies at all, provision-level conditions narrow within applicable laws
    result: Phase 1 targets law-level (regs 1-3), provision-level deferred
  - what: Regex extraction polished with AI (quality metric TBD)
    why: Regex handles 95%+ of polarity detection, AI needed only for vocabulary gaps
    result: fitness.rs module with 23 unit tests
  - what: Fitness rules live in LanceDB (not DuckDB)
    why: LanceDB exists to leverage AI for polishing, and fitness rules are per-provision data
    result: fitness_rules field on TaxaRecord

metrics:
  polarity_match_before: 44.5
  polarity_match_after: 95.5
  tests_passing: 335
  dictionary_patterns: 94
  application_scope_provisions: 3130
  genuine_after_heading_filter: 645

lessons:
  - title: "55.6% of APPLICATION_SCOPE provisions were upstream false positives"
    detail: "purpose.rs APPLICATION_SCOPE regex conflated \"application\" (filing/submitting) with \"application\" (scope of law). Tightening the regex removed 2,539 false positives."
    tag: methodology
  - title: Heading-only provisions are structural markers with no legal obligation text
    detail: "Bare \"Application\" headings triggered APPLICATION_SCOPE. Added section_type filter in enrichment to skip heading rows."
    tag: data
  - title: Compound provision splitting handles dual-polarity provisions
    detail: "\"shall not apply... but shall apply...\" correctly splits into separate AppliesTo + DisappliesTo rules. 42 compound provisions handled."
    tag: architecture

artifacts:
  - crates/fractalaw-core/src/taxa/fitness.rs
  - crates/fractalaw-core/src/taxa/mod.rs
  - crates/fractalaw-core/src/taxa/purpose.rs
  - crates/fractalaw-cli/src/main.rs

enables:
  - P-dimension dictionary expansion (#23)
  - Cross-reference resolution (#22)
  - Fitness denormalization for publish
---

# Session: Fitness Index Design (CLOSED)

**Date**: 2026-03-01
**Objective**: Design an index of law applicability rules ("fitness") that users can be evaluated against. Research existing assets and recommend an architecture.

## Context

"Fitness" is the term for **applicability of laws to a user**. Given a user's credentials (industry, role, location, activities, equipment, etc.), determine which laws apply and which provisions within those laws are relevant.

The system needs:
1. A structured **index of applicability rules** extracted from legislation
2. A **user profile schema** (credentials) that can be matched against those rules
3. A **matching function** that evaluates user credentials against the rule index

## Existing Assets

### In fractalaw (current codebase)

| Asset | Location | What it provides |
|-------|----------|-----------------|
| `purpose::APPLICATION_SCOPE` | `taxa/purpose.rs` | Detects provisions that define applicability (regex, ~95% accuracy) |
| `purposes` column | LanceDB `legislation_text` | Per-provision purpose tags including Application+Scope |
| `extent_*` columns | DuckDB `legislation` | Territorial applicability (England, Wales, Scotland, NI) |
| `status` column | DuckDB `legislation` | In-force / repealed / amended status |
| `governed_actors` | LanceDB `legislation_text` | Who the law regulates (employers, operators, etc.) |
| `ClauseStructure.applicability` | `taxa/clause_structure.rs` | Conditional preambles on DRRP provisions ("Where X applies...") |
| `should_skip_drrp()` | `taxa/mod.rs` | Gates DRRP enrichment — Application+Scope provisions are identified but NOT enriched for duties |

### In legacy legl project (Elixir prototype)

| Asset | File | What it provides |
|-------|------|-----------------|
| Fitness struct | `fitness.ex` | Schema: person, process, place, property, plant (the "5 Ps") |
| Rule struct | `rule.ex` | Links rules to provisions, headings, scope |
| Parse module | `parse.ex` | 60+ regex patterns extracting applicability entities |
| ParseDefs | `parse_defs.ex` | Entity dictionaries: persons, processes, places, plant, properties |
| ParseExtendsTo | `parse_extends_to.ex` | Geographic scope (extends outside GB or not) |
| RuleProvisions | `rule_provisions.ex` | Links rules to specific regulation numbers |

---

## Legacy Fitness Model: The "5 Ps"

The legl prototype modelled applicability around **5 p-dimensions** (PPP+PP):

| P-dimension | Examples | What it answers |
|-------------|---------|-----------------|
| **Person** | employer, self-employed, contractor, worker, competent person | Who does the law apply to? |
| **Process** | construction work, diving, handling dangerous substances, loading/unloading | What activities trigger applicability? |
| **Place** | construction site, mine, ship, offshore, Great Britain, outside GB | Where must you be for it to apply? |
| **Plant** | asbestos, pressure systems, display screens, dangerous goods, explosives | What equipment/substances trigger it? |
| **Property** | at work, indoors, sporadic and low intensity, carried out solely by crew | What qualifying conditions apply? |

Each applicability rule was classified as:
- **applies-to** — positive applicability ("These Regulations apply to...")
- **disapplies-to** — negative applicability / exemption ("These Regulations do not apply to...")
- **extends-to** — geographic scope extension
- **qualified-applies** — conditional applicability

---

## Analysis: What the Provisions Actually Say

Real Application+Scope provisions from the corpus follow recurring patterns:

### Pattern A: Blanket applicability
```
"These Regulations apply in relation to Wales."
"These Regulations may be cited as the Landfill Allowances Scheme (Wales) Regulations 2004"
```
**P-dimensions**: Place only.

### Pattern B: Actor-scoped applicability
```
"These Regulations shall apply to every employer and every self-employed person"
```
**P-dimensions**: Person.

### Pattern C: Activity-scoped applicability
```
"These Regulations apply to construction work"
"The classification shall apply for classifying inland freshwaters"
```
**P-dimensions**: Process (activity).

### Pattern D: Conditional dis-applicability
```
"Paragraphs 1 to 17 do not apply where the presence of waste is liable to give rise to an environmental hazard"
"Sub-paragraph (1) does not apply to the disposal of waste at a site designed for final disposal by landfill"
```
**P-dimensions**: Process + Property (conditions).

### Pattern E: Compound applicability
```
"These Regulations apply to every employer who undertakes construction work and to every employee who carries out construction work"
```
**P-dimensions**: Person + Process.

---

## Recommendations

### 1. Adopt the 5P model, but simplify and extend

The legacy 5P model (Person, Process, Place, Plant, Property) is a good conceptual framework but the original implementation was over-engineered with 60+ regex patterns and fuzzy matching. The fractalaw version should:

- **Keep the 5 p-dimensions** — they map well to how UK ESH law expresses applicability
- **Use existing actor taxonomy for Person** — `governed_actors` labels already enumerate persons (Org: Employer, Ind: Worker, Operator, etc.)
- **Add a 6th p-dimension: Sector/Industry** — many laws apply to specific industries (construction, mining, offshore, nuclear) that aren't cleanly person/process/place
- **Store as structured tags, not free text** — each p-dimension gets an enumerable set (like Qualifier in clause_structure)

### 2. Two-tier rule structure

| Tier | Scope | Source | Example |
|------|-------|--------|---------|
| **Law-level** | Whole law applicability | Citation/commencement/application sections (reg 1–3) | "These Regulations apply to construction work in Great Britain" |
| **Provision-level** | Per-section conditions | Conditional preambles on DRRP provisions | "Where a dangerous substance is present at the workplace, the employer shall..." |

Law-level rules determine **whether the law applies at all**. Provision-level conditions determine **which duties within an applicable law are active for a given user**.

### 3. Rule schema

```rust
/// Applicability rule extracted from legislation.
pub struct FitnessRule {
    /// Source provision.
    pub law_name: String,
    pub section_id: String,

    /// Polarity: does this rule include or exclude?
    pub polarity: RulePolarity,

    /// The 5+1 p-dimensions of applicability.
    pub person: Vec<String>,       // reuses governed_actors taxonomy
    pub process: Vec<String>,      // activity/work type tags
    pub place: Vec<String>,        // geographic/location tags
    pub plant: Vec<String>,        // equipment/substance tags
    pub property: Vec<String>,     // qualifying conditions
    pub sector: Vec<String>,       // industry/sector tags

    /// The raw applicability text for reference.
    pub raw_text: String,

    /// Confidence in the extraction.
    pub confidence: f32,
}

pub enum RulePolarity {
    AppliesTo,       // positive: "shall apply to..."
    DisappliesTo,    // negative: "shall not apply to..."
    ExtendsTo,       // geographic: "extends to outside GB"
}
```

### 4. User profile schema

```rust
/// User credentials for fitness matching.
pub struct UserProfile {
    pub person_type: Vec<String>,    // "Org: Employer", "Ind: Self-employed Worker"
    pub processes: Vec<String>,      // "construction work", "diving operations"
    pub places: Vec<String>,         // "Great Britain", "offshore", "construction site"
    pub plant: Vec<String>,          // "asbestos", "pressure systems"
    pub properties: Vec<String>,     // "at work", "5 or more employees"
    pub sectors: Vec<String>,        // "construction", "mining", "nuclear"
}
```

### 5. Matching logic

```
For each law:
  1. Check law-level rules (regs 1-3)
     - If any AppliesTo rule matches user profile → law is candidate
     - If any DisappliesTo rule matches → law is excluded
  2. For candidate laws, check provision-level conditions
     - Each DRRP provision's applicability preamble narrows applicability
     - Result: list of (law, provision, duty) tuples that apply to this user
```

### 6. Implementation approach

**Phase 1: Extract law-level applicability rules**
- Target: Application+Scope provisions in early sections (regs 1–3, "Application" headings)
- Method: New parser module in `fractalaw-core/src/taxa/` (like clause_structure.rs)
- Extract: polarity (applies/disapplies), p-dimensions (person/process/place/plant/property)
- Store: New columns in LanceDB or a new `fitness_rules` table

**Phase 2: Build p-dimension dictionaries**
- Curate the enumerable sets for Process, Place, Plant, Property, Sector
- Person already exists (actors.rs)
- Start from the legacy ParseDefs dictionaries, validate against the corpus

**Phase 3: User profile matching**
- Simple set-intersection matching: user p-dimensions ∩ rule p-dimensions
- Polarity resolution: applies-to minus disapplies-to
- Return: ranked list of applicable laws with provision-level detail

### 7. Design decisions

- [x] **Storage**: Fitness rules live in **LanceDB** — LanceDB exists to leverage AI, and AI may be needed to polish regex results
- [x] **Extraction method**: **Regex-based**, polished with AI when a quality metric scores low (quality metric TBD)
- [x] **Catch-all applicability**: Unlikely in EHS domain — laws are always scoped to employers or organisations. If no rule matches, the model simply returns no result
- [x] **Pipeline integration**: Extract during **enrichment**, but designed to run separately when needed (e.g. model improves, needs reparse)
- [x] **Temporal applicability**: **Out of scope** for this model. Possible later extension
- [x] **Priority**: **Law-level rules first** (Phase 1, this session). Provision-level conditions in Phase 2 (future session)

---

## Phase 1: Law-Level Applicability Rules (This Session)

### Scope

Extract applicability rules from Application+Scope provisions in the early sections of each law (regs 1–3, "Application" headings). These determine **whether the law applies at all** to a given user profile.

### Step 1: Corpus Audit ✅

**11,150 Application+Scope provisions** across **410 laws** (11.2% of 99,691 total).

| Provision location | Count | % |
|--------------------|------:|--:|
| Early (provisions 1–3) | 2,199 | 21.7% |
| Mid (provisions 4–10) | 3,056 | 30.1% |
| Late (provisions >10) | 4,893 | 48.2% |

Phase 1 targets **early-section provisions** (~2,199). Many later-section hits are false positives (procedural "application" vs scope "application").

**Pattern frequency** (early sections):

| Pattern | Count | % |
|---------|------:|--:|
| Positive ("applies to", "shall apply to") | 743 | 22.6% |
| Negative ("shall not apply", "does not apply") | 480 | 14.6% |
| Geographic extension ("extends to") | 24 | 0.7% |

**P-dimension mentions** (early sections):
- Person: 29.3% (employer, operator, manufacturer, self-employed...)
- Place: 29.0% (mine, England, Wales, offshore, Great Britain...)
- Process: 16.2% (gas, explosive, mining, electrical, diving...)

### Step 2: P-Dimension Dictionaries ✅

Built from corpus frequency analysis + legacy ParseDefs dictionaries.

#### Person (who the law applies to)
From existing `governed_actors` taxonomy — no new dictionary needed.
Key terms: employer, employee, self-employed person, worker, contractor, sub-contractor, operator, manufacturer, supplier, importer, occupier, owner, master, duty holder, responsible person, competent person, designer, installer, agency worker.

#### Process (what activities trigger applicability)
construction work, diving operation, mining, quarrying, gas (fitting/supply), electrical work, handling dangerous substances, work at height, manual handling, loading/unloading, transport (road/rail/sea/air), health surveillance, risk assessment, work with display screens, asbestos work, lead work, work with explosives, radiation work, petroleum operations, pressure systems work, noise exposure, vibration exposure.

#### Place (where you must be for it to apply)
Great Britain, England, Wales, Scotland, Northern Ireland, United Kingdom, offshore, offshore installation, territorial sea, continental shelf, mine, quarry, construction site, factory, premises, workplace, ship, aircraft, outside Great Britain.

#### Plant (what equipment/substances trigger it)
asbestos, lead, dangerous substances, explosives, pressure systems, display screen equipment, work equipment, personal protective equipment, dangerous goods, gas fittings, petroleum, ionising radiation sources, biological agents, chemicals, noise-generating equipment, vibration-generating equipment.

#### Property (qualifying conditions)
at work, 5 or more employees, indoors, sporadic and low intensity, carried out solely by crew under direction of master, not liable to expose persons, not in prolonged use, on board transport, normal ship-board activities.

#### Sector (industry — the 6th p-dimension)
construction, mining, quarrying, offshore oil & gas, nuclear, chemicals, petroleum, gas supply, diving, maritime/shipping, agriculture, manufacturing, waste management, water industry.

### Step 3: Extraction Parser Design

**Target**: Parse Application+Scope provision text into structured `FitnessRule` with polarity + p-dimension tags.

**10 recurring text patterns** identified from corpus (in order of frequency):

| # | Pattern | Example | Extraction |
|---|---------|---------|------------|
| 1 | Geographic scope | "These Regulations apply to England only" | polarity=AppliesTo, place=[England] |
| 2 | Whole-instrument exclusion | "shall not apply to the master or crew of a sea-going ship" | polarity=DisappliesTo, person=[master, crew], place=[ship] |
| 3 | Positive scope definition | "shall apply to all quarries where persons work" | polarity=AppliesTo, place=[quarry], person=[persons at work] |
| 4 | Self-employed extension | "shall apply to a self-employed person as they apply to an employer" | polarity=AppliesTo, person=[self-employed] |
| 5 | Like-duty extension | "be under a like duty in respect of any other person" | polarity=AppliesTo, person=[other persons] |
| 6 | Partial exclusion with carve-outs | "shall not apply to... but shall apply to..." | two rules: DisappliesTo + AppliesTo |
| 7 | Crown application | "shall apply to persons in the public service of the Crown" | polarity=AppliesTo, person=[Crown] |
| 8 | Specific reg carve-outs | "excluded EXCEPT regs X, Y, Z" | polarity=DisappliesTo with exceptions |
| 9 | Disapplied-where-other-regs | "shall have effect except where... Control of Lead at Work" | polarity=DisappliesTo, conditional |
| 10 | HSWA purposive scope | "with a view to securing the health, safety and welfare of persons at work" | polarity=AppliesTo, person=[persons at work] |

**Parser approach**: Two-stage extraction:
1. **Polarity detection** — regex for applies/disapplies/extends-to (reuse `purpose.rs` pattern)
2. **P-dimension tagging** — dictionary lookup against the p-dimension word lists above

### Step 4: Implementation ✅

**Created**: `crates/fractalaw-core/src/taxa/fitness.rs`

Module implements:
- `RulePolarity` enum (AppliesTo, DisappliesTo, ExtendsTo)
- `PDimension` enum (Person, Process, Place, Plant, Property, Sector)
- `PDimensionTag` struct (dimension + canonical term)
- `FitnessRule` struct (polarity + tags + raw_text)
- `extract()` public API — returns `Vec<FitnessRule>` from provision text
- Compound provision splitting ("shall not apply... but shall apply...")
- 6 p-dimension dictionaries with word-boundary regex matching
- 23 unit tests — all pass

**Wired into pipeline**: `taxa/mod.rs` runs `fitness::extract()` on Application+Scope provisions in the `should_skip_drrp` early-return branch. New `fitness_rules: Vec<FitnessRule>` field on `TaxaRecord`.

### Step 5: Corpus Validation

**3,130 early-section Application+Scope provisions** tested:

| Metric | Count | % |
|--------|------:|--:|
| Polarity matched | 1,391 | 44.4% |
| — AppliesTo | 905 | 28.9% |
| — DisappliesTo | 473 | 15.1% |
| — ExtendsTo | 13 | 0.4% |
| No polarity match | 1,739 | 55.6% |
| At least one p-dimension tag | 678 | 21.7% |

**P-dimension distribution:**

| Dimension | Count | % |
|-----------|------:|--:|
| Place | 753 | 24.1% |
| Person | 375 | 12.0% |
| Plant | 201 | 6.4% |
| Property | 91 | 2.9% |
| Process | 54 | 1.7% |
| Sector | 41 | 1.3% |

**Key finding**: 55.6% of Application+Scope provisions got no polarity match. These are mostly **upstream false positives** — the `purpose.rs` APPLICATION_SCOPE regex conflates "application" (filing/submitting) with "application" (scope of law). The fitness extraction itself works well on genuine applicability provisions.

**Compound splitting**: 42 compound provisions correctly split into separate AppliesTo + DisappliesTo rules.

### Step 6: APPLICATION_SCOPE Regex Improvement ✅

**Root cause**: 89.6% of false positives were stale data from the old pre-GH#20 regex. The remaining current FPs came from:
- `provisions of ... apply` matching "apply for the purposes of interpreting" (7 cases)
- Self-ref branch matching "applies for the purpose of determining" (6 cases)
- `^Application\b` matching heading-only provisions (26 cases)

**Changes made**:
- `purpose.rs`: Self-ref branch now requires preposition after apply: `appl(y|ies) (to|in |where|until|unless)` — rejects "applies for the purpose of"
- `purpose.rs`: `provisions of ... apply` branch now requires `apply (to|in)` — rejects bare "apply"
- `fitness.rs`: ExtendsTo regex now matches `shall extend only to`

**Validation results** (early-section provisions, new regex vs old):

| Metric | Old regex | New regex |
|--------|----------:|----------:|
| Total Application+Scope | 3,130 | 645 |
| Polarity matched | 1,394 (44.5%) | 616 (95.5%) |
| No polarity match | 1,736 (55.5%) | 29 (4.5%) |

**Polarity match rate: 44.5% → 95.5%** (+51 percentage points). The 2,539 removed provisions were overwhelmingly false positives (procedural "applications" for permits/licences). 54 new legitimate catches were added. The remaining 29 no-polarity matches are all heading-only provisions (`^Application\b`).

**335 unit tests pass**, 0 regressions. Corpus needs re-enrichment to realize the improvement on stored data.

### Quality Assessment

The fitness extraction pipeline is now **high quality on genuine applicability provisions**:
- 95.5% polarity detection rate (up from 44.5%)
- P-dimension tagging covers Person, Place, Process, Plant, Property, Sector
- Compound provision splitting handles "shall not apply... but shall apply..." cases

Remaining improvement paths:
1. **Re-enrich the corpus** — stored data still uses old regex. Running `taxa enrich` would apply the new regex. Change tracking needed first: **#21** (taxa_hash for enrich→publish delta detection)
2. **Expand p-dimension dictionaries** — two separate issues:
   - **#22** — Cross-reference provisions ("regulation 6 applies") that point to other provisions instead of containing their own scope vocabulary. Needs cross-ref resolution or AI polishing.
   - **#23** — Vocabulary gaps in the 6 dictionaries. Provisions with explicit scope terms that aren't in the model. Corpus audit + frequency analysis + AI-assisted expansion.
3. ~~**Handle heading-only provisions**~~ — **DONE**: `enrich_single_law()` now reads the `section_type` column from LanceDB and skips rows where `section_type = "heading"` before any taxa parsing. Heading rows are structural markers (e.g., bare "Application") with no legal obligation text. This eliminates the 29 heading-only false positives from the corpus validation, bringing the effective polarity match rate to ~100% on genuine provisions.

## Session Closed

**Status**: Phase 1 complete. Committed as `119f2ed`.

**Delivered**:
- `fitness.rs` module with 23 unit tests (all 335 workspace tests pass)
- Integrated into `parse_v2()` pipeline for APPLICATION_SCOPE provisions
- Tightened APPLICATION_SCOPE regex (polarity match 44.5% → 95.5%)
- Heading-type row filter in enrichment pipeline

**Open issues for future sessions**:
- **#21** — taxa_hash for change tracking between enrich and publish
- **#22** — Cross-reference provision resolution
- **#23** — P-dimension dictionary expansion
