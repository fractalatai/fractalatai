# JSP Service: Policy Knowledge Graph for MoD Joint Service Publications

**Status**: Design v0.3
**Date**: 2026-07-18
**Reviewed by**: Gemini 2.5 Flash (2026-07-18) — see `data/code-review/jsp-service-review.md`
**Scope**: Architecture and pipeline for classifying and cross-linking MoD JSPs within the fractalaw/sertantai ecosystem

---

## The Problem

Fractalaw processes legislation — Acts and SIs published by Parliament with stable citations, clear structural conventions (Part/Chapter/Section/Regulation), and a well-defined duty/right/responsibility/power (DRRP) taxonomy. The pipeline works: 183K+ provisions parsed, classified, cross-referenced, and published.

MoD Joint Service Publications (JSPs) are a different kind of document. They sit *below* legislation in the normative hierarchy — they are how the Ministry of Defence implements its statutory obligations in practice. JSPs are:

- **Policy, not law.** They interpret and operationalise legislation but have no direct legal force outside MoD. Their `legal_weight` is `contractual` — binding via contract, not statute.
- **Internally authored.** Written by MoD subject matter authorities, not Parliamentary Counsel. Terminology varies between JSPs.
- **Cross-referencing.** JSPs reference legislation (HSWA 1974, CDM 2015) *and* each other (JSP 375 references JSP 815, JSP 392 references JSP 375). The cross-reference graph is dense.
- **Responsibility-structured, not DRRP.** JSPs assign organisational roles — SRO, Operating Duty Holder, Commanding Officer, Accountable Person — within the existing legal framework. This is responsibility assignment, not Hohfeldian duty/right classification.
- **Version-controlled.** JSPs are periodically reissued. New versions supersede old ones, sometimes with significant structural changes.
- **Hierarchically deep.** A JSP may contain volumes, parts, chapters, sections, annexes, and appendices — deeper than most UK legislation.

The JSP corpus is already parsed and stored in sertantai-legal's Postgres:

| | Count |
|---|---|
| JSP families | 10 (375, 376, 392, 403, 418, 425, 520, 815, 816, 975) |
| Chapter/element sources | 158 |
| Parsed provisions | 13,854 |
| PDFs processed | 167 |

The enrichment columns on `secondary_source_provisions` (`drrp_types`, `actors`, `governed_actors`, `popimar`, `purposes`, `significance_overall`, `taxa_enriched_at`) are null — waiting for fractalaw to populate them.

What remains is to turn this structural data into an interconnected, queryable policy knowledge graph that connects JSP obligations to the legislation they implement, the roles they assign, and the definitions they use.

---

## Architectural Decision: Where Does JSP Data Live?

### The Existing Split

For legislation, fractalaw and sertantai have a clear division of labour:

```
sertantai-legal (Elixir/Phoenix)          fractalaw (Rust)
─────────────────────────────             ──────────────────
Owns: LAT text, LRT metadata,            Owns: DRRP parser, actor dictionary,
  scraper, amendment tracking               multi-tier classification, embeddings
Stores: legal_registers, legal_articles   Stores: DuckDB (staging, analytics),
  (production PG, synced to prod)           Postgres (pgvector), LanceDB (vectors)
Exposes: Zenoh queryables                 Consumes: Zenoh queryables
Receives: enrichment via Zenoh            Publishes: enrichment via Zenoh
```

The same split already exists for secondary sources. Phase 3 of the second-tier-duties work added Zenoh queryables:

```
fractalaw/@{tenant}/data/secondary/sources           → all sources (JSON)
fractalaw/@{tenant}/data/secondary/sources/{id}      → single source (JSON)
fractalaw/@{tenant}/data/secondary/provisions/{id}   → provisions (Arrow IPC)
```

Fractalaw can already pull JSP provisions on demand for enrichment. The question is: where do the *new* entities (obligations, RACI assignments, terms, cross-references) live?

### Three Options

**Option A: All new tables in fractalaw's Postgres**

v0.1 proposed six new tables (`jsp_document`, `jsp_section`, `jsp_obligation`, `jsp_raci`, `jsp_term`, `jsp_reference`) in fractalaw's local Postgres. This duplicates `secondary_sources` and `secondary_source_provisions` — data that already exists in sertantai. It creates a sync problem: two sources of truth for JSP provisions, two schemas to maintain, two places where data can go stale.

**Option B: All new tables in sertantai's Postgres**

Sertantai already owns the JSP source data. Adding obligation, RACI, term, and reference tables there would keep everything in one place. But sertantai's DB syncs to production — fractalaw's experimental enrichment pipeline (multi-tier classification, LLM staging, audit logs) shouldn't write directly to a production-synced database. And Ash resource migrations are sertantai's domain, not fractalaw's.

**Option C: Follow the existing pattern — sertantai owns structure, fractalaw enriches**

This is how legislation already works:

| | sertantai owns | fractalaw produces | Travels via |
|---|---|---|---|
| **Legislation** | `legal_registers` (LRT), `legal_articles` (LAT) | DRRP enrichment, fitness, significance, L3 controls, L4 evidence | Zenoh pub/sub |
| **JSPs** | `secondary_sources`, `secondary_source_provisions`, `source_links` | DRRP enrichment, RACI extraction, term extraction, cross-reference resolution | Zenoh pub/sub |

Fractalaw does the analytical work locally (DuckDB staging tables, local Postgres for pgvector), then publishes results back to sertantai via Zenoh. Sertantai stores the enriched data in columns that already exist on `secondary_source_provisions` — or in new Ash resources that sertantai creates when the enrichment types are defined.

### Decision: Option C

Option C is the right answer because:

1. **It's the established pattern.** Legislation already works this way. Deviating from it for JSPs adds complexity without benefit.
2. **No data duplication.** Fractalaw doesn't copy `secondary_sources` or `secondary_source_provisions` — it pulls on demand via Zenoh, enriches, and publishes back.
3. **Production safety.** Fractalaw writes to its own DuckDB staging tables during enrichment. Only validated results are published to sertantai via Zenoh.
4. **Clean domain boundaries.** Sertantai owns the schema (Ash resources, Ecto migrations). Fractalaw owns the intelligence (parsers, classifiers, LLM pipelines).
5. **Incremental delivery.** The DRRP enrichment columns on `secondary_source_provisions` already exist. Fractalaw can start enriching JSP provisions *today* without any schema changes in sertantai.

### What This Means Concretely

**Fractalaw keeps locally (DuckDB staging + local PG):**

- JSP actor dictionary (`jsp-actor-dictionary.yaml`, compile-time embedded)
- Obligation extraction staging (`jsp_obligations` DuckDB table — intermediate results before publish)
- RACI extraction staging (`jsp_raci` DuckDB table)
- Term extraction staging (`jsp_terms` DuckDB table)
- Cross-reference resolution staging (`jsp_references` DuckDB table)
- Embeddings in local Postgres (pgvector — same as legislation)

**Sertantai receives via Zenoh (new Ash resources when ready):**

- DRRP enrichment → existing columns on `secondary_source_provisions` (immediate)
- RACI assignments → new `secondary_raci` Ash resource (when sertantai schema is ready)
- Term definitions → new `secondary_terms` Ash resource (when sertantai schema is ready)
- Cross-references → enriched `source_links` (existing table, new `reference_type` values)

**Nothing needs to change in sertantai for Phase 1.** The enrichment columns already exist. Fractalaw starts by populating `drrp_types`, `actors`, `governed_actors`, `popimar`, `purposes`, and `significance_overall` on JSP provisions — exactly what it does for legislation, using the same Zenoh publish pattern.

---

## What the JSP Service Produces

Three interconnected layers built on top of the parsed hierarchy:

```
Layer 1: STRUCTURE (sertantai)   Layer 2: SEMANTICS (fractalaw→sertantai)   Layer 3: OPERATIONS (fractalaw→sertantai)
──────────────────────────       ─────────────────────────────────────       ─────────────────────────────────────────
secondary_sources                Term extraction, conflict detection         Obligation extraction, RACI assignment
secondary_source_provisions      Cross-reference resolution                  Responsibility-assignment classification
source_links                     Source traceability mapping                 Traceability gap analysis
```

### Layer 1: Structure (already in sertantai)

The parsed JSP hierarchy is in sertantai's Postgres. Source metadata in `secondary_sources`, provisions in `secondary_source_provisions`, law links in `source_links`. No fractalaw work needed here.

**Existing sertantai schema:**

```
secondary_sources (208 total, 158 JSP chapters)
├── source_id: "JSP-375-CH23"
├── source_type: "jsp"
├── title: "Health and Safety Handbook — Chapter 23"
├── issuer: "MoD"
├── legal_weight: "contractual"
├── parent_source_id: → JSP-375 (parent grouping)
├── supersedes_id: → previous edition
└── edition: "V1.3, November 2024"

secondary_source_provisions (13,854 JSP provisions)
├── section_id: "JSP_mod_2026_JSP375CH23:part-1-directive/policy-statements.para.23"
├── source_id: "JSP-375-CH23"
├── section_type: paragraph | part | chapter | section | heading | annex
├── hierarchy_path: "/part-1-directive/policy-statements.para.23"
├── text: "As part of the risk assessment the commander, manager or accountable person must..."
├── text_source: full_text | summary | heading_only
├── drrp_types: NULL      ← fractalaw enriches
├── actors: NULL           ← fractalaw enriches
├── governed_actors: NULL  ← fractalaw enriches
├── popimar: NULL          ← fractalaw enriches
├── purposes: NULL         ← fractalaw enriches
├── significance_overall: NULL  ← fractalaw enriches
└── taxa_enriched_at: NULL      ← fractalaw sets

source_links (JSP → legislation)
├── secondary_source_id: → JSP-375-CH23
├── law_name: "UK_ukpga_1974_37"
├── section_id: "UK_ukpga_1974_37:s.2" (nullable, provision-level)
├── link_type: implements | references | supplements | approved_under
└── secondary_section_id: (nullable, provision-to-provision)
```

### Layer 2: Semantics (fractalaw produces, publishes to sertantai)

The meaning layer — definitions, acronyms, and source traceability:

- **Term extraction.** JSPs define terms in glossaries, annexes, and inline. These definitions may conflict across JSPs (e.g., "risk" defined differently in safety vs. security JSPs). The service surfaces these conflicts.
- **Cross-reference resolution.** JSP text contains references to legislation, other JSPs, and standards embedded in prose. These need to be extracted and resolved against the fractalaw corpus and sertantai's source registry.
- **Source traceability.** Where a JSP paragraph implements a specific legislative provision. This enriches the existing `source_links` table with provision-level granularity.

### Layer 3: Operations (fractalaw produces, publishes to sertantai)

The action layer — who does what, and with what authority:

- **Responsibility-assignment classification.** JSPs assign organisational roles (SRO, Operating Duty Holder, Commanding Officer, Accountable Person) to tasks. This is NOT Hohfeldian DRRP — it's responsibility assignment within an existing legal framework.
- **Obligation strength classification.** JSPs use "must" (directive), "should" (guidance), and "may" (permissive) with different semantics from legislation — notably, "will" and "is to" are mandatory in JSP context.
- **RACI extraction.** Where JSPs contain explicit responsibility matrices, extract structured R/A/C/I assignments per obligation per role.
- **Traceability matrix.** For any legislative obligation, which JSP paragraph(s) implement it? For any JSP obligation, which legislation drives it?

### Layer 4: Mandated Artefacts and Operational Properties

JSPs don't just assign duties — they mandate the creation and maintenance of specific things: safety cases, risk assessments, permits to work, hazard logs, emergency plans, training records, inspection reports, audit programmes. Each of these has operational properties that legislation typically leaves unspecified.

A safety case is one instance of this pattern. So is a risk assessment, a permit to work, a COSHH assessment, a method statement, an emergency plan. The abstraction is: **a JSP obligation may mandate a thing with properties**.

#### Mandated Artefact (generic abstraction)

When a JSP provision says "The Commanding Officer shall maintain a Safety Case that demonstrates ALARP for each hazardous activity, reviewed annually by the Operating Duty Holder," it encodes:

| Property | Value from example | Abstraction |
|----------|--------------------|-------------|
| What is mandated | Safety Case | `artefact_type` |
| Who creates/maintains it | Commanding Officer | `owner_role` |
| What it must demonstrate | ALARP | `required_content` / `acceptance_criterion` |
| Scope | Each hazardous activity | `scope` |
| Review frequency | Annually | `review_frequency` |
| Reviewer/approver | Operating Duty Holder | `reviewer_role` / `approver_role` |

This is the same pattern for every mandated artefact:

| Artefact type | Owner role | Must demonstrate | Review frequency | Approver |
|---------------|-----------|------------------|-----------------|----------|
| Safety Case | CO | ALARP | Annual | ODH |
| Risk Assessment | Commander/Manager | Suitable & sufficient | Before activity + periodic | Accountable Person |
| Permit to Work | Authorised Person | Safe system in place | Per entry/activity | Supervisor |
| Hazard Log | Safety Case Author | All hazards identified | Continuous + formal review | ISA |
| Emergency Plan | CO | Adequate arrangements | Annual + after incidents | DSA |
| Training Record | Line Manager | Competence demonstrated | Per training event | Unit Safety Officer |
| Audit Programme | DSA | Compliance verified | Per audit cycle | Senior Duty Holder |
| COSHH Assessment | Manager | Exposure controlled | Annual + on change | Competent Person |

#### Mandated Artefact Schema (DuckDB staging)

| Column | Type | Purpose |
|--------|------|---------|
| `artefact_id` | TEXT PK | `{obligation_id}:art.{seq}` |
| `obligation_id` | TEXT FK | The obligation that mandates this artefact |
| `section_id` | TEXT | Source provision |
| `source_id` | TEXT | JSP chapter |
| `artefact_type` | TEXT | Safety Case / Risk Assessment / Permit / Hazard Log / Emergency Plan / Training Record / Audit Report / Inspection Report / Method Statement / Other |
| `owner_role` | TEXT | Who creates/maintains (from JSP actor dictionary) |
| `approver_role` | TEXT | Who approves (nullable) |
| `reviewer_role` | TEXT | Who reviews (nullable) |
| `review_frequency` | TEXT | Annual / Quarterly / Monthly / Per-activity / On-change / Continuous |
| `validity_period` | TEXT | How long it remains current (nullable) |
| `required_content` | TEXT[] | What it must contain/demonstrate (e.g., ["ALARP demonstration", "hazard identification"]) |
| `acceptance_criterion` | TEXT | The test for adequacy (nullable — may be a load-bearing judgement) |
| `scope` | TEXT | What it covers (e.g., "each hazardous activity", "per confined space") |
| `triggers` | TEXT[] | What triggers creation/update (e.g., ["new activity", "change of use", "incident"]) |
| `extraction_method` | TEXT | regex / slm / llm |
| `confidence` | REAL | |
| `created_at` | TIMESTAMP | |

This connects directly to the L3 Controls and L4 Evidence pipelines:

- A mandated artefact IS a control specification (L3) — the JSP has done the work of specifying what the control looks like.
- The artefact's properties map directly to evidence patterns (L4) — `review_frequency` → `recommended_interval`, `approver_role` → `judge identity`, `acceptance_criterion` → `discriminating_question`.

#### Additional Operational Properties on Obligations

Beyond mandated artefacts, JSP obligations encode operational properties that the model should capture as structured attributes on `jsp_obligations`:

| Property | Examples | Field |
|----------|----------|-------|
| **Competence requirements** | "competent person", "trained in confined space rescue", "CS01 certified" | `competence_requirements` (TEXT[]) |
| **Delegation conditions** | "may delegate to a competent subordinate", "delegation must be in writing" | `delegation_conditions` (TEXT) |
| **Escalation triggers** | "if risk cannot be reduced to ALARP", "in the event of a near miss" | `escalation_trigger` (TEXT) |
| **Escalation path** | "escalate to Senior Duty Holder", "report to DSA within 24 hours" | `escalation_path` (TEXT) |
| **Reporting line** | "report findings to the Safety Committee" | `reporting_role` (TEXT) |

These are extracted as structured attributes on obligations, not separate entity tables. They become separate tables only when query patterns demand it — for now, JSONB attributes on the staging table are sufficient.

#### Graph Structure

```
jsp_obligation ──(1:N)──► jsp_mandated_artefact
     │                         │
     │ (1:N)                   ├── owner_role ──► jsp_actor_dictionary
     ▼                         ├── approver_role ──► jsp_actor_dictionary
  jsp_raci                     ├── reviewer_role ──► jsp_actor_dictionary
     │                         └── required_content[]
     ├── role ──► jsp_actor_dictionary
     └── assignment_type (R/A/C/I)

jsp_mandated_artefact ──maps to──► L3 Control (control specification)
                      ──maps to──► L4 Evidence (artefact pattern + judgement guidance)
```

---

## JSP → Controls Pipeline Integration

### The Insight

Legislation is goal-setting: "the employer shall ensure the health, safety and welfare of employees." The L3 Controls pipeline generates indicative-mood controls from these obligations, but the LLM has to *infer* what the operational control looks like.

JSPs are prescriptive: "the Commanding Officer shall maintain a Safety Case that demonstrates ALARP for each hazardous activity, reviewed annually by the Operating Duty Holder." The operational control is already specified — the JSP has done the inferential work.

JSP-derived controls will be more concrete, more actionable, and more directly verifiable than legislation-derived controls. The `what_it_checks`, `evidence_hint`, and `expected_touch_frequency` fields that the LLM estimates for legislative controls can be read directly from JSP text.

### Mode 1b: JSP-Enriched Control Generation

A new generation mode in the controls pipeline, sitting between Mode 1 (canonical legislative) and Mode 2 (customer reconciliation):

```
Mode 1:  Legislative obligations → Canonical controls (goal-setting)
Mode 1b: JSP obligations + mandated artefacts → Operational controls (prescriptive)
Mode 2:  Customer existing controls → Reconciliation
```

**Input:** JSP obligations and mandated artefacts for a given JSP, plus the legislative controls they relate to (via `source_links` traceability).

**Output:** JSP-enriched controls that:
1. Link to both the JSP obligation (`obligation_id`) and the legislative provision(s) it implements
2. Carry a `source_normativity` field: `Legislative` (from law) or `Policy` (from JSP)
3. Have richer `what_it_checks`, `evidence_hint`, and operational properties because the JSP specifies them

### Enrichment, Not Replacement

JSP-derived controls enrich legislative controls — they don't replace them. The relationship is hierarchical:

```
Legislative control (canonical, goal-setting)
  └── IMPLEMENTED_BY → JSP control (operational, prescriptive)
```

During Phase 3 consolidation (HDBSCAN clustering), if a JSP-derived control is semantically very similar to a legislative control:
- The JSP control **refines** the legislative control, inheriting its `linked_provisions` and adding JSP-specific operational detail
- The legislative control is preserved but linked: `refined_by_jsp_control_id`
- Both remain in the control register — the legislative control for legal traceability, the JSP control for operational use

If a JSP-derived control has no legislative equivalent (JSP-only requirement):
- It stands alone as a Policy-tier control
- `source_normativity = "Policy"`, no legislative `linked_provisions`

### Mandated Artefacts as Control + Evidence Specifications

The mandated artefact abstraction maps directly to the existing L3/L4 schemas:

| Mandated artefact property | → L3 Control field | → L4 Evidence field |
|----------------------------|-------------------|-------------------|
| `artefact_type` | `domain` (Organisational/Technical) | `artefact_type` |
| `owner_role` | Informs `info_distance` | Informs `judge identity` |
| `approver_role` | — | `recommended_method` (the approval is a judgement act) |
| `reviewer_role` | — | `recommended_method` |
| `review_frequency` | `expected_touch_frequency` | `recommended_interval` |
| `required_content` | `what_it_checks` | `basis_guidance` |
| `acceptance_criterion` | `load_bearing_judgement` | `discriminating_question` |
| `scope` | `blast_radius` | `sample_size_guidance` |
| `triggers` | `frequency` (Ad-hoc when trigger-based) | `staleness_tolerance` |

A single mandated artefact from a JSP can generate both an L3 Control and its corresponding L4 Evidence pattern — the JSP has specified both the "what" and the "how to verify."

---

## Version Management

### The Problem

When a JSP is reissued, provisions can be added, removed, or modified. The current schema tracks document-level supersession (`supersedes_id` on `secondary_sources`) but not provision-level change.

All enrichment derived from a changed provision becomes stale: DRRP classification, RACI assignments, obligations, mandated artefacts, cross-references, and any L3 Controls or L4 Evidence generated from JSP obligations.

### Strategy

**Phase 1 (immediate):** Treat JSP reissue as a full re-enrichment event. When a JSP is re-parsed in sertantai:
1. Sertantai creates new `secondary_source_provisions` rows (new `section_id`s if structure changed, or updates existing rows)
2. Sertantai nulls the enrichment columns on affected provisions
3. Fractalaw detects unenriched provisions on next pull and re-enriches

This is coarse but functional. It mirrors how legislative amendments work today — the scraper updates LAT, fractalaw re-enriches.

**Phase 2+ (when needed):** Provision-level versioning in sertantai:
- `effective_from_edition` and `effective_to_edition` columns on `secondary_source_provisions`
- New provisions inserted on reissue; old ones marked with `effective_to_edition`
- Enrichment carries an `enrichment_edition` linking it to the source version
- Three-way merge for any JSP-derived controls (same pattern as legislative controls in `COMPLIANCE-CONTROLS.md`)

### Enrichment Invalidation

When fractalaw detects a provision has been re-parsed (different `updated_at` from sertantai vs. `taxa_enriched_at`):
1. Clear the DuckDB staging rows for that provision
2. Re-run extraction pipeline (DRRP, obligations, RACI, references)
3. Publish updated enrichment to sertantai

For JSP-derived L3 Controls: apply the three-way merge. `base_hash` identifies the original generation. Customer edits are preserved. Only system-changed fields are updated.

---

## Enrichment Column Extension

The existing enrichment columns on `secondary_source_provisions` (`drrp_types`, `actors`, `governed_actors`, `popimar`, `purposes`, `significance_overall`, `taxa_enriched_at`) are sufficient for Phase 1 DRRP enrichment.

Phase 2+ requires additional columns. These are sertantai migrations, planned when the enrichment types are validated in fractalaw's DuckDB staging:

| Column | Type | Phase | Purpose |
|--------|------|-------|---------|
| `obligation_strength` | TEXT | 2 | Mandatory / Recommended / Permissive |
| `modal_verb` | TEXT | 2 | shall / must / will / should / may / is to |
| `raci_summary` | JSONB | 3 | Denormalised RACI: `[{role: "CO", type: "R"}, ...]` |
| `mandated_artefacts` | JSONB | 3 | Artefact specs: `[{type: "Safety Case", owner: "CO", review: "Annual"}]` |
| `competence_requirements` | TEXT[] | 3 | Required qualifications/training |
| `review_frequency` | TEXT | 3 | If the provision mandates a recurring review |
| `cross_references` | JSONB | 2 | Resolved references: `[{target_id: "...", type: "implements"}]` |

The DuckDB staging schemas should be designed with these eventual sertantai columns in mind — same field names, same types, same semantics. This prevents transformation complexity in the Zenoh publish step.

---

## JSP Actor Vocabulary

The Phase 2b session established that JSPs use a responsibility assignment model, not Hohfeldian DRRP. The actor vocabulary is organisational, not legal:

| Actor | Frequency | Role |
|-------|-----------|------|
| SRO (Senior Responsible Owner) | 142 | Project/programme accountability |
| User | 57 | Military user/operator of equipment |
| Defence organisation | 50 | Any MoD unit/command |
| Accountable person | 19 | H&S accountability role |
| Commander, manager | 17 | Line management chain |
| Contractor | 15 | External supply chain |
| Project Technical Authority | 7 | Technical assurance role |
| Operator | 6 | Equipment operator |
| Infrastructure provider | 5 | Estate management |
| Prime contractor | 4 | Main contract holder |

A separate `jsp-actor-dictionary.yaml` is needed in `fractalaw-core/data/`:

```yaml
# Accountability chain
- label: "MoD: Senior Responsible Owner"
  triggers: [senior responsible owner, sro]
  category: MoD-Accountability

- label: "MoD: Accountable Person"
  triggers: [accountable person, ap]
  category: MoD-Accountability

- label: "MoD: Duty Holder"
  triggers: [duty holder, dutyholder, dh]
  category: MoD-Safety

- label: "MoD: Operating Duty Holder"
  triggers: [operating duty holder, odh]
  category: MoD-Safety

- label: "MoD: Senior Duty Holder"
  triggers: [senior duty holder, sdh]
  category: MoD-Safety

# Operational chain
- label: "MoD: Commanding Officer"
  triggers: [commanding officer, co, unit commander]
  category: MoD-Operational

- label: "MoD: Commander/Manager"
  triggers: [commander, manager, line manager]
  category: MoD-Operational

- label: "MoD: Head of Establishment"
  triggers: [head of establishment, hoe]
  category: MoD-Operational

# Technical
- label: "MoD: Project Technical Authority"
  triggers: [project technical authority, pta]
  category: MoD-Technical

- label: "MoD: User"
  triggers: [user, military user]
  category: MoD-Operational

- label: "MoD: Operator"
  triggers: [operator, equipment operator]
  category: MoD-Operational

# Safety governance
- label: "MoD: Defence Safety Authority"
  triggers: [defence safety authority, dsa]
  category: MoD-Safety

- label: "MoD: Unit Safety Officer"
  triggers: [unit safety officer, uso, safety officer]
  category: MoD-Safety

- label: "MoD: Safety Case Author"
  triggers: [safety case author]
  category: MoD-Safety

- label: "MoD: Independent Safety Adviser"
  triggers: [independent safety adviser, isa]
  category: MoD-Safety

# Environment
- label: "MoD: Environmental Protection Adviser"
  triggers: [environmental protection adviser, epa, environmental adviser]
  category: MoD-Environment

# Supply chain
- label: "MoD: Contractor"
  triggers: [contractor, defence contractor]
  category: MoD-Supply

- label: "MoD: Prime Contractor"
  triggers: [prime contractor, prime]
  category: MoD-Supply

- label: "MoD: Infrastructure Provider"
  triggers: [infrastructure provider]
  category: MoD-Supply

# Organisational
- label: "MoD: Defence Organisation"
  triggers: [defence organisation, mod organisation]
  category: MoD-Org

- label: "MoD: Top Level Budget Holder"
  triggers: [top level budget holder, tlb holder, tlb]
  category: MoD-Finance
```

**Integration with existing dictionary:** Loaded separately from the legislative actor dictionary. Different trigger conventions (JSPs use acronyms heavily), different categories (MoD organisational roles, not legal persons), different maintenance lifecycle. The two dictionaries connect through `source_links` — "for this legislative actor (Employer), which MoD roles implement that responsibility?"

---

## DuckDB Staging Tables

Fractalaw stages enrichment results locally before publishing to sertantai. These follow the same pattern as `suggested_controls` and `suggested_evidence`.

### `jsp_obligations` (DuckDB)

| Column | Type | Purpose |
|--------|------|---------|
| `obligation_id` | TEXT PK | `{section_id}:ob.{seq}` |
| `section_id` | TEXT | Source provision in sertantai |
| `source_id` | TEXT | JSP chapter identifier (e.g., "JSP-375-CH23") |
| `text` | TEXT | The obligation sentence |
| `modal_verb` | TEXT | shall / must / will / should / may / is to |
| `strength` | TEXT | Mandatory / Recommended / Permissive |
| `clause_refined` | TEXT | "Who must do what" extract |
| `competence_requirements` | TEXT[] | Required qualifications/training (e.g., ["CS01 certified", "confined space rescue trained"]) |
| `delegation_conditions` | TEXT | Conditions under which this obligation can be delegated (nullable) |
| `escalation_trigger` | TEXT | What condition triggers escalation (nullable) |
| `escalation_path` | TEXT | To whom and how (nullable) |
| `reporting_role` | TEXT | Who receives reports/findings (nullable) |
| `extraction_method` | TEXT | regex / slm / llm / raci_table |
| `confidence` | REAL | Extraction confidence |
| `status` | TEXT | extracted / validated / published |
| `created_at` | TIMESTAMP | |

### `jsp_raci` (DuckDB)

| Column | Type | Purpose |
|--------|------|---------|
| `raci_id` | TEXT PK | `{obligation_id}:{role_normalised}` |
| `obligation_id` | TEXT FK | |
| `section_id` | TEXT | Where the assignment is stated |
| `role_label` | TEXT | The role as written |
| `role_normalised` | TEXT | Canonical label from JSP actor dictionary |
| `assignment_type` | TEXT | R / A / C / I |
| `assignment_source` | TEXT | raci_table / narrative / inferred |
| `confidence` | REAL | |
| `created_at` | TIMESTAMP | |

### `jsp_terms` (DuckDB)

| Column | Type | Purpose |
|--------|------|---------|
| `term_id` | TEXT PK | `{source_id}:{normalised_term}` |
| `source_id` | TEXT | Defining JSP chapter |
| `defined_in` | TEXT | `section_id` where the definition appears |
| `term` | TEXT | The term as written |
| `normalised` | TEXT | Lowercased, stripped form for dedup |
| `definition` | TEXT | The definition text |
| `acronym` | TEXT | Acronym if applicable |
| `expansion` | TEXT | Acronym expansion |
| `created_at` | TIMESTAMP | |

**Conflict detection query (DuckDB):**

```sql
SELECT t1.term, t1.source_id, t1.definition,
       t2.source_id AS conflicting_source, t2.definition AS conflicting_definition
FROM jsp_terms t1
JOIN jsp_terms t2 ON t1.normalised = t2.normalised
  AND t1.source_id < t2.source_id
  AND t1.definition != t2.definition;
```

### `jsp_references` (DuckDB)

| Column | Type | Purpose |
|--------|------|---------|
| `reference_id` | TEXT PK | `{source_section_id}:ref.{seq}` |
| `source_section_id` | TEXT | The JSP section containing the reference |
| `target_type` | TEXT | legislation / jsp / standard |
| `target_id` | TEXT | Resolved identifier (law_name, section_id, or source_id) |
| `target_display` | TEXT | Human-readable citation |
| `reference_type` | TEXT | implements / references / supersedes / delegates / see_also |
| `extracted_by` | TEXT | regex / llm |
| `resolved` | BOOLEAN | Whether target_id was resolved against the corpus |
| `created_at` | TIMESTAMP | |

### `jsp_mandated_artefacts` (DuckDB)

| Column | Type | Purpose |
|--------|------|---------|
| `artefact_id` | TEXT PK | `{obligation_id}:art.{seq}` |
| `obligation_id` | TEXT FK | The obligation that mandates this artefact |
| `section_id` | TEXT | Source provision |
| `source_id` | TEXT | JSP chapter |
| `artefact_type` | TEXT | Safety Case / Risk Assessment / Permit / Hazard Log / Emergency Plan / Training Record / Audit Report / Inspection Report / Method Statement / Other |
| `owner_role` | TEXT | Who creates/maintains (from JSP actor dictionary) |
| `approver_role` | TEXT | Who approves (nullable) |
| `reviewer_role` | TEXT | Who reviews (nullable) |
| `review_frequency` | TEXT | Annual / Quarterly / Monthly / Per-activity / On-change / Continuous |
| `validity_period` | TEXT | How long it remains current (nullable) |
| `required_content` | TEXT[] | What it must contain/demonstrate |
| `acceptance_criterion` | TEXT | The test for adequacy (nullable — may be load-bearing judgement) |
| `scope` | TEXT | What it covers (e.g., "each hazardous activity") |
| `triggers` | TEXT[] | What triggers creation/update (e.g., ["new activity", "incident"]) |
| `extraction_method` | TEXT | regex / slm / llm |
| `confidence` | REAL | |
| `status` | TEXT | extracted / validated / published |
| `created_at` | TIMESTAMP | |

This table is the bridge between the JSP service and the compliance pipeline. Each row is simultaneously:
- A structured extraction from JSP text (what the JSP mandates)
- An L3 Control specification (what the operational control looks like)
- An L4 Evidence pattern seed (what evidence to collect and how to verify)

---

## Extraction Pipeline

Three phases, each building on the last. Phase 1 can start immediately — it only needs the Zenoh queryables that already exist.

### Phase 1: DRRP Enrichment (immediate — no schema changes needed)

The simplest and most valuable first step: run fractalaw's existing DRRP parser on JSP provisions, adapted for JSP modal conventions.

**What changes in fractalaw:**

1. **Pull JSP provisions via Zenoh.** Use the existing queryable: `fractalaw/@dev/data/secondary/provisions/{source_id}`.
2. **Adapt DRRP parser for JSP modal conventions.** Add a `policy` context mode to `parse_v2()` that treats "will" and "is to" as mandatory modals.
3. **Use JSP actor dictionary.** Load `jsp-actor-dictionary.yaml` separately, use it when `source_type = "jsp"`.
4. **Publish enrichment back via Zenoh.** Populate the existing enrichment columns on `secondary_source_provisions`.

**JSP-specific modal conventions:**

| Modal | Legislative meaning | JSP meaning |
|-------|-------------------|-------------|
| shall | Obligation (strong) | Obligation (strong) |
| must | Obligation (strong) | Obligation (strong) |
| will | Future tense (descriptive) | **Obligation (strong)** — JSPs use "will" as mandatory |
| should | Not typically used | Recommendation (medium) |
| may | Permission | Permission |
| is to | Not typically used | **Obligation (strong)** — common in JSP directives |

**What sertantai already handles:** The `TaxaSubscriber` on sertantai-legal already listens for enrichment data on `fractalaw/@{tenant}/taxa/enrichment/*`. A new subscriber (or extension of the existing one) for `fractalaw/@{tenant}/taxa/secondary/{source_id}` receives the enriched data and updates `secondary_source_provisions`.

**Output:** 13,854 JSP provisions classified with DRRP types, actor labels, significance. Queryable in sertantai alongside legislation.

### Phase 2: Reference Extraction & Resolution

Extract cross-references from JSP text and resolve them against the fractalaw corpus and sertantai's source registry.

**Regex patterns for legislation references:**

```
# Act references
(?:the\s+)?(?:Health and Safety at Work (?:etc\.? )?Act\s+1974|HSWA\s+1974)
(?:section|s\.)\s*(\d+[A-Z]?)(?:\((\d+)\))?

# SI references
(?:the\s+)?(.+?)\s+Regulations?\s+(\d{4})
(?:regulation|reg\.)\s*(\d+)(?:\((\d+)\))?

# JSP cross-references
JSP\s+(\d{3})\s*(?:Vol(?:ume)?\s*(\d+))?\s*(?:(?:Ch(?:apter)?\s*)?(\d+))?\s*(?:(?:para(?:graph)?\s*)?(\d+(?:\.\d+)*))?

# Standard references
(?:BS\s+)?(?:EN\s+)?ISO\s+(\d+)(?::(\d{4}))?
```

**Resolution:** Regex-extracted citations are resolved against:
- fractalaw's legislation corpus (`legislation_text.section_id`) for law references
- sertantai's `secondary_sources` for JSP cross-references
- Unresolvable references flagged for manual review

**Staging:** Results go to `jsp_references` DuckDB table. Once validated, they enrich sertantai's existing `source_links` table (adding provision-level `secondary_section_id` values) or create new rows.

**Output:** Cross-reference graph connecting JSPs to legislation and to each other. Enriches `source_links` with provision-level granularity.

### Phase 3: Obligation & RACI Extraction

Extract obligations from JSP text and assign RACI roles.

**Two extraction modes:**

#### Mode A: Structured RACI Tables

Many JSPs contain explicit responsibility matrices. If the PDF parser captured these as structured rows (section_type = `table`), the extraction pipeline reads them directly:

```
For each RACI table cell:
  1. Create jsp_obligation from the row header (the task/requirement)
  2. Create jsp_raci from each marked cell (role × assignment_type)
  3. Link to the section_id containing the table
```

Confidence: 0.95 (explicit assignment, minimal interpretation).

#### Mode B: Narrative Text

JSP body text contains narrative obligations: "The Commanding Officer shall ensure that all personnel receive safety briefings before deployment." These are extracted using the actor-anchored modal-verb pattern from Phase 1, adapted to also extract the RACI assignment:

- Actor named + mandatory modal ("The CO shall ensure...") → R (Responsible)
- Actor named + "is accountable for..." → A (Accountable)
- "in consultation with {actor}" → C (Consulted)
- Actor named + "shall be informed" / "shall be notified" → I (Informed)
- Passive voice ("Safety briefings shall be conducted...") → actor ambiguous, flag for LLM

#### Mode C: LLM Enrichment

Batch LLM calls for:

1. **Passive-voice obligations.** "Safety briefings shall be conducted" → who is responsible?
2. **RACI disambiguation.** When a paragraph names multiple roles, which is R and which is A?
3. **Source traceability.** For each JSP obligation, which legislative provision(s) does it implement?

**Prompt structure (source traceability):**

```
You are mapping MoD policy obligations to the UK legislation they implement.

JSP Obligation:
  "{obligation text}"
  From: {source_id}, {section heading}
  Roles: {assigned roles}

Candidate Legislative Provisions (from the same regulatory domain):
  1. {section_id}: "{provision text}" [DRRP: {type}]
  2. ...

For each JSP obligation, identify which legislative provision(s) it implements.
Return: [{obligation_id, target_section_id, reference_type, confidence}]
```

The candidate set is filtered by domain using the `source_links` join — a safety JSP's obligations are matched against legislation that `source_links` already connects it to.

**Output:** Structured obligations with RACI assignments, staged in DuckDB, published to sertantai when validated.

### Phase 4: Mandated Artefact Extraction

Extract mandated artefacts from JSP obligations — the things JSPs require to exist. This is the bridge to the compliance pipeline.

**Regex patterns for artefact detection:**

```
# Artefact creation/maintenance
(?:shall|must|will|is to)\s+(?:maintain|produce|prepare|create|develop|establish)\s+
  (?:a|an|the)\s+(safety case|risk assessment|hazard log|permit to work|
  emergency plan|method statement|training record|audit programme|
  inspection report|COSHH assessment)

# Review/approval
(?:reviewed|approved|endorsed|signed off)\s+(?:by|annually|quarterly|monthly|
  at least every|before each)

# Required content
(?:shall|must)\s+(?:demonstrate|show|include|contain|address|cover)\s+
  (ALARP|suitable and sufficient|all hazards|competence|compliance)
```

**LLM extraction:** For each obligation identified as mandating an artefact, an LLM call extracts the structured properties (owner, approver, reviewer, frequency, required content, acceptance criterion, scope, triggers). The LLM receives the obligation text plus surrounding context (the JSP chapter's roles and responsibilities section, if present).

**Output:** `jsp_mandated_artefacts` DuckDB staging table populated. Each row is a structured control+evidence specification ready for the compliance pipeline.

### Phase 5: Semantic Enrichment & Controls Integration

1. **Term extraction** from glossary sections and inline definitions. Stage in `jsp_terms` DuckDB table. Conflict detection across JSPs.
2. **Embedding** JSP provisions using the same sentence-transformer model (all-MiniLM-L6-v2, 384-dim). Store in fractalaw's local Postgres (pgvector) for semantic search.
3. **Traceability gap analysis** — find legislative obligations (from laws linked via `source_links`) that no JSP provision implements. This is the high-value output.
4. **Mode 1b control generation** — feed mandated artefacts into the L3 Controls pipeline as JSP-enriched controls. Consolidate with legislative controls via HDBSCAN clustering.

---

## Why Not a Graph Database?

Gemini's proposal suggested Neo4j or FalkorDB. For this use case, Postgres is the better choice:

1. **Shared infrastructure.** Both sertantai and fractalaw already run Postgres. Adding a third database type triples operational complexity for marginal query benefit.

2. **The graph is shallow.** The JSP knowledge graph has 4-5 hops at most: Document → Section → Obligation → RACI → Role. Postgres handles this with simple JOINs. Graph databases shine at 6+ hop traversals or variable-depth pathfinding — neither is needed here.

3. **The primary queries are aggregations, not traversals.** "All obligations for the Commanding Officer" is a filtered JOIN. "Term conflicts across JSPs" is a self-JOIN with GROUP BY. These are SQL's sweet spot.

4. **pgvector for semantic search.** The embedding-based similarity queries ("find JSP paragraphs semantically similar to this legislative provision") are already supported by pgvector in both databases.

5. **Arrow interchange.** fractalaw's data exchange format is Arrow RecordBatch. Postgres (via sqlx) produces Arrow-compatible output directly. Graph databases would need a serialisation layer.

**If traversal depth grows:** Postgres `ltree` extension handles hierarchical queries. For true multi-hop graph traversals in future (e.g., transitive delegation chains), recursive CTEs work up to ~10 hops. Apache AGE adds Cypher to Postgres without a separate database.

---

## Cross-Domain Queries

The power of keeping JSP enrichment in the same sertantai Postgres as legislation:

```sql
-- Legislation → JSP: Which JSP provisions reference HSWA?
SELECT sl.link_type, ssp.section_id, ssp.text, ssp.governed_actors
FROM source_links sl
JOIN secondary_source_provisions ssp
  ON ssp.secondary_source_id = sl.secondary_source_id
WHERE sl.law_name = 'UK_ukpga_1974_37'
  AND sl.link_type = 'implements';

-- RACI consolidation: All obligations for 'Commanding Officer' across JSPs
-- (requires RACI data published to sertantai as new resource)
SELECT o.text, o.strength, r.assignment_type, ss.title AS jsp_title
FROM secondary_raci r
JOIN secondary_obligations o ON r.obligation_id = o.obligation_id
JOIN secondary_sources ss ON o.source_id = ss.source_id
WHERE r.role_normalised = 'MoD: Commanding Officer'
ORDER BY ss.source_id, o.section_id;

-- Traceability gap: Laws linked to JSPs with unimplemented obligations
SELECT la.section_id, la.text, la.drrp_types
FROM legal_articles la
JOIN source_links sl ON sl.law_name = la.law_name
  AND sl.link_type = 'implements'
LEFT JOIN source_links sl2 ON sl2.law_name = la.law_name
  AND sl2.section_id = la.section_id  -- provision-level link
WHERE sl2.id IS NULL
  AND la.drrp_types @> ARRAY['Obligation']
  AND la.significance_overall = 'HIGH';

-- Contractor-applicable provisions across all JSPs
SELECT ssp.section_id, ssp.text, ssp.governed_actors, ss.title
FROM secondary_source_provisions ssp
JOIN secondary_sources ss ON ssp.source_id = ss.source_id
WHERE ss.source_type = 'jsp'
  AND ssp.governed_actors @> ARRAY['MoD: Contractor'];
```

The last query — contractor-applicable provisions — is immediately useful for QQ customers operating under MoD contracts. It works as soon as Phase 1 enrichment populates `governed_actors`.

---

## CLI Commands

New `fractalaw jsp` subcommand group:

```bash
# Pull and inspect
fractalaw jsp list                            # List JSP sources from sertantai (via Zenoh)
fractalaw jsp show JSP-375-CH23               # Show source metadata + provision count
fractalaw jsp text JSP-375-CH23 --limit 20    # Display provision text in order

# Phase 1: DRRP enrichment
fractalaw jsp enrich JSP-375-CH23             # Enrich one JSP chapter
fractalaw jsp enrich JSP-375 --all-chapters   # Enrich all chapters of a JSP
fractalaw jsp enrich --all                    # Enrich all JSP provisions
fractalaw jsp enrich JSP-375-CH23 --dry-run   # Show what would be enriched

# Phase 2: Reference extraction
fractalaw jsp extract-refs JSP-375-CH23       # Extract and resolve cross-references
fractalaw jsp extract-refs --all              # All JSPs

# Phase 3: Obligation & RACI
fractalaw jsp extract-obligations JSP-375-CH23  # Extract obligations + RACI
fractalaw jsp raci "Commanding Officer"         # Query RACI across JSPs (from DuckDB staging)

# Phase 4: Mandated artefacts
fractalaw jsp extract-artefacts JSP-375-CH23    # Extract mandated artefacts from obligations
fractalaw jsp artefacts JSP-375                 # List mandated artefacts for a JSP
fractalaw jsp artefacts --type "Safety Case"    # All safety case requirements across JSPs

# Phase 5: Semantics & controls integration
fractalaw jsp terms --conflicts               # Term conflicts across JSPs
fractalaw jsp trace UK_ukpga_1974_37:s.2(1)   # Which JSP provisions implement this?
fractalaw jsp gaps JSP-375                    # Legislative obligations with no JSP implementation
fractalaw jsp controls JSP-375-CH23           # Generate L3 controls from JSP mandated artefacts

# Publish enrichment to sertantai
fractalaw jsp publish --tenant dev            # Publish DRRP enrichment via Zenoh
fractalaw jsp publish --tenant dev --raci     # Publish RACI assignments (when sertantai resource exists)
fractalaw jsp publish --tenant dev --artefacts  # Publish mandated artefacts

# Stats
fractalaw jsp stats                           # Corpus statistics
```

---

## Zenoh Key Expressions

### Pull (fractalaw queries sertantai — already exist):

```
fractalaw/@{tenant}/data/secondary/sources             → all sources (JSON)
fractalaw/@{tenant}/data/secondary/sources/{source_id}  → single source (JSON)
fractalaw/@{tenant}/data/secondary/provisions/{source_id} → provisions (Arrow IPC)
```

### Publish (fractalaw → sertantai — new):

```
fractalaw/@{tenant}/taxa/secondary/{source_id}     → DRRP enrichment (Arrow IPC)
fractalaw/@{tenant}/jsp/obligations/{source_id}    → Obligations + RACI (Arrow IPC)
fractalaw/@{tenant}/jsp/artefacts/{source_id}      → Mandated artefacts (Arrow IPC)
fractalaw/@{tenant}/jsp/references/{source_id}     → Cross-reference edges (Arrow IPC)
fractalaw/@{tenant}/jsp/terms/{source_id}          → Term definitions (Arrow IPC)
```

The enrichment key (`taxa/secondary/{source_id}`) follows the existing `taxa/enrichment/{law_name}` pattern. Sertantai needs a new subscriber (or extension of `TaxaSubscriber`) to handle secondary source enrichment.

---

## Implementation Phases

### Phase 1: DRRP Enrichment (immediate — no sertantai changes needed)

1. Add JSP actor dictionary to `fractalaw-core/data/jsp-actor-dictionary.yaml`
2. Add `policy` context mode to `parse_v2()` — "will"/"is to" as mandatory modals
3. Add `fractalaw jsp list/show/text` CLI commands (pull from Zenoh queryables)
4. Add `fractalaw jsp enrich` CLI command (pull provisions, parse, publish enrichment)
5. Extend sertantai's `TaxaSubscriber` (or add `SecondaryTaxaSubscriber`) to receive enrichment for `secondary_source_provisions`

**Output:** 13,854 JSP provisions enriched with DRRP types, actors, significance. Contractor-applicable provisions immediately queryable. No new sertantai tables needed.

### Phase 2: Reference Extraction (minimal sertantai changes)

1. Implement regex patterns for legislation, JSP, and standard references
2. Resolve against fractalaw corpus and sertantai source registry
3. Stage in `jsp_references` DuckDB table
4. Publish resolved references to sertantai — enriching existing `source_links` with provision-level `secondary_section_id` values
5. Add `fractalaw jsp extract-refs` and `fractalaw jsp trace` CLI commands

**Output:** Cross-reference graph connecting JSP provisions to specific legislative provisions. Enriches existing `source_links`.

### Phase 3: Obligation & RACI Extraction (new sertantai resources needed)

1. Implement RACI table extraction (Mode A) and narrative extraction (Mode B)
2. Extract operational properties: competence requirements, delegation conditions, escalation paths
3. Stage in DuckDB (`jsp_obligations`, `jsp_raci`)
4. LLM enrichment for passive voice, RACI disambiguation, source traceability
5. Define Zenoh publish schema for obligations and RACI
6. Work with sertantai to create `secondary_obligations` and `secondary_raci` Ash resources
7. Add CLI commands

**Output:** Structured obligations with RACI assignments and operational properties. Unified RACI matrix queryable across all JSPs.

### Phase 4: Mandated Artefact Extraction (compliance pipeline integration)

1. Extract mandated artefacts from obligations (regex + LLM)
2. Stage in DuckDB (`jsp_mandated_artefacts`)
3. For each artefact, extract structured properties: owner, approver, reviewer, frequency, required content, acceptance criterion, scope, triggers
4. Validate artefact types against the generic taxonomy (Safety Case, Risk Assessment, Permit, etc.)
5. Publish to sertantai via Zenoh
6. Add CLI commands (`extract-artefacts`, `artefacts`)

**Output:** Structured mandated artefact specifications — the bridge to L3 Controls and L4 Evidence generation from JSPs.

### Phase 5: Semantic Enrichment & Controls Integration

1. Term extraction from glossary sections and inline definitions
2. Conflict detection across JSPs
3. Embed JSP provisions (pgvector in fractalaw's local Postgres)
4. Traceability gap analysis
5. Mode 1b control generation: feed mandated artefacts into L3 Controls pipeline
6. Consolidate JSP-derived controls with legislative controls (HDBSCAN, `source_normativity` field)
7. Generate L4 Evidence patterns from JSP-derived controls (mandated artefact properties provide richer evidence seeds)
8. Publish terms, controls, and evidence to sertantai

**Output:** Full knowledge graph with term definitions, traceability links, gap analysis, and JSP-enriched L3 Controls + L4 Evidence integrated with the legislative compliance pipeline.

---

## What This Enables

### For MoD Compliance (QQ customer use case)

1. **Contractor-applicable provisions.** Filter JSP provisions by `governed_actors` containing "MoD: Contractor" — immediately answers "what does the MoD policy framework require of us?" for defence contractors.

2. **Unified RACI.** Query all obligations for a role across all JSPs. Replaces manual spreadsheet maintenance.

3. **Traceability.** For any legislative change (e.g., new regulations under HSWA), immediately identify which JSPs need updating and which roles are affected.

4. **Gap analysis.** Find legislative obligations that no JSP addresses — compliance gaps in the policy framework.

5. **Term consistency.** Surface definitional conflicts across JSPs before they cause operational confusion.

### For the Compliance Controls Pipeline

JSP obligations feed into the L3 Controls pipeline as a richer source than legislation alone:

1. **Mandated artefacts ARE control specifications.** A JSP that says "maintain a Safety Case demonstrating ALARP, reviewed annually by the ODH" has already specified the control type (Preventive/Directive), the owner role (CO), the review frequency (Annual), the acceptance criterion (ALARP), and the reviewer (ODH). The LLM doesn't need to infer these — it reads them.

2. **JSP-derived controls enrich legislative controls.** A legislative control ("Safe systems of work are in place") is refined by the JSP control ("Safe systems of work are documented in the Unit Safety Case, reviewed annually by the Operating Duty Holder"). The `IMPLEMENTED_BY` relationship preserves legal traceability while adding operational specificity.

3. **L4 Evidence patterns are richer.** The mandated artefact's `required_content` maps to `basis_guidance`, its `acceptance_criterion` maps to `discriminating_question`, its `review_frequency` maps to `recommended_interval`. Evidence patterns generated from JSP-enriched controls will have less LLM estimation and more direct specification.

4. **Competence requirements inform evidence.** When a JSP obligation requires a "competent person" and specifies what competence means (e.g., "CS01 certified"), the L4 Evidence pattern can name the specific artefact type (Certificate) and the discriminating test (current CS01 certification) without LLM inference.

---

## Open Questions

1. **Enrichment subscriber pattern.** Should sertantai extend `TaxaSubscriber` to also handle secondary source enrichment (checking `secondary_source_provisions` when `legal_registers` lookup fails), or create a dedicated `SecondaryTaxaSubscriber` with its own key expression? The dedicated subscriber is cleaner but adds another GenServer.

2. **RACI table recognition.** The PDF parser captures JSP content by structural type (`section_type` enum: paragraph, heading, table, etc.). How well does it capture RACI matrices? If tables were flattened to prose, Mode A extraction degrades to Mode B. Need to inspect actual parsed data for JSPs known to contain RACI tables (JSP 375, JSP 815).

3. **Classification reuse.** Can fractalaw's existing DRRP classifier weights (`drrp_classifier_v8.json`, `position_classifier_v3.json`) be applied to JSP text? The modal verb distribution differs ("will"/"is to" as mandatory). Regex-tier with the `policy` context mode may be sufficient for Phase 1. SLM fine-tuning on JSP training data is Phase 3 territory.

4. **Applicability screening.** Sertantai already has `org_secondary_applicabilities` for per-org JSP applicability decisions. Should the JSP enrichment feed into the applicability screener (matching org profiles to JSP-applicable sectors/roles), or is JSP applicability always manual? For MoD contractors, it's likely "all safety JSPs apply" — but for non-defence customers, JSPs are irrelevant.

5. **JSP families.** Should JSPs be classified into families (OH&S, Environmental, Security) like legislation? The issuing authority and JSP title provide signal. Alternatively, use the `source_links` join to inherit the family classification from linked legislation — a JSP linked to HSWA inherits the OH&S family.

6. **Scope boundary.** 10 JSP families are parsed (13,854 provisions). The full JSP corpus is 800+ publications. Is the current scope sufficient for the pilot, or are there critical safety JSPs missing?

7. **Artefact type taxonomy.** The mandated artefact types (Safety Case, Risk Assessment, Permit, etc.) need a stable taxonomy. Should this be a closed enum (maintained like the actor dictionary) or open-ended with normalisation? The legislative compliance pipeline uses a fixed set of `artefact_type` values in L4 Evidence — the JSP taxonomy should align.

8. **Intra-JSP cross-references.** `source_links` is designed for secondary→primary links. JSP-to-JSP cross-references (JSP 375 referencing JSP 815) are a different relationship. Should these go in `source_links` with a `link_scope` discriminator, or in a separate `secondary_cross_references` table? For now, the `jsp_references` DuckDB staging table handles both — the sertantai schema decision can wait until Phase 2 data validates the volume and query patterns.

9. **Delegation chain depth.** Gemini flagged delegation chains as a missing graph structure. JSPs delegate authority with conditions and scope limitations. How deep do these chains go in practice? If delegation is typically one hop (SDH → CO), structured attributes on obligations suffice. If multi-hop (Secretary of State → TLB → SDH → ODH → CO), a recursive relationship table may be needed. Inspect JSP 375 and JSP 815 to determine actual delegation depth.

---

## Related Documents

- [`jsp-service-review.md`](../../data/code-review/jsp-service-review.md) — Gemini 2.5 Flash review of v0.2
- [`COMPLIANCE-CONTROLS.md`](../compliance/COMPLIANCE-CONTROLS.md) — L3 Controls pipeline (consumer of JSP mandated artefacts via Mode 1b)
- [`COMPLIANCE-EVIDENCE.md`](../compliance/COMPLIANCE-EVIDENCE.md) — L4 Evidence pipeline
- [`FITNESS-STRATEGY.md`](../fitness/FITNESS-STRATEGY.md) — Fitness applicability (may need JSP-specific dimensions)
- `crates/fractalaw-core/data/actor-dictionary.yaml` — existing legislative actor dictionary
- Second-tier-duties sessions (sertantai-legal):
  - `phase-1-data-model.md` — `secondary_sources`, `source_links`, `org_secondary_applicabilities` schema
  - `phase-2b-hsg-and-jsp-corpus.md` — JSP parsing, actor vocabulary analysis
  - `phase-3-zenoh-queryables.md` — Zenoh queryable spec for secondary sources
  - `ZENOH-SECONDARY-SOURCES.md` — published Zenoh spec
