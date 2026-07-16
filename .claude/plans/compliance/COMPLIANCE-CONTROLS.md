# Compliance Controls: LLM-Assisted Control Generation from Legal Obligations

**Status**: Design v0.2
**Date**: 2026-07-10
**Scope**: Prompt architecture and pipeline for generating L3 Controls from L1 Obligations using an LLM
**Reviewed by**: Gemini 2.5 Pro (2026-07-10) — see `data/code-review/compliance-controls-design-review.md`

---

## The Problem

L3 Controls is the critical gap in the compliance architecture. Without it:

```
L1 Obligation → L4 Evidence (direct)
```

This skips the operational question: *how* is the obligation met? The control is the bridge — the operational mechanism that implements the obligation and that evidence can be gathered against.

Controls are customer-specific. SertantAI cannot pre-populate the control register from legal data alone. But we *can* generate a **canonical suggested control set** from the law's obligation structure, which the customer's compliance team then reviews, adapts, and owns.

The insight from the ought-is brief: a control written as a checkable indicative statement — "Isolation is verified before entry" rather than "Personnel must ensure isolation" — is simultaneously the standard *and* the test. The gap between the two readings (what is required vs what is true) is the compliance signal. This is the form the LLM should produce.

---

## Pipeline Overview

The pipeline has four phases, not one. v0.1 described a single generate-and-store step. v0.2 separates generation, validation, consolidation, and review.

```
Phase 1: GENERATE           Phase 2: VALIDATE          Phase 3: CONSOLIDATE       Phase 4: REVIEW
─────────────────           ───────────────────        ────────────────────        ───────────────
Per-law (or chunk)          Automated lint +           Embedding + clustering     Human-in-the-loop
  → candidate controls      LLM self-critique          + LLM synthesis            acceptance/edit
                              → deontic check            → dedup within law
                              → referent check            → dedup across family
                              → flag for rework           → policy predicate
```

---

## Phase 1: Generate Candidate Controls

### Input Specification

#### Law Outline (from DuckDB LRT)

| Field | Purpose | Example |
|-------|---------|---------|
| `title` | Full law title | Health and Safety at Work etc. Act 1974 |
| `family` | Domain classification | OH&S: Occupational / Personal Safety |
| `sub_family` | Sub-domain | General duties |
| `year` | Enactment year | 1974 |
| `jurisdiction` | Territory | UK |
| `extent_regions` | Where it applies | England, Wales, Scotland |
| `status` | In force / repealed | In force |
| `description` | Long title / purpose statement — the law's own summary of its intent. Primary input for the policy predicate. | "An Act to make further provision for securing the health, safety and welfare of persons at work..." |
| `explanatory_note` | Explanatory Note text (SIs) or introductory summary. Plain-language statement of the law's purpose, written by the drafting lawyers. Richer than the long title. Key input for the policy predicate. Will be available in DuckDB before implementation (scraped by sertantai). | "These Regulations impose requirements with respect to the carrying out of work in confined spaces..." |
| `body_paras` | Substantive provisions | 120 |
| `duty_holder` | Aggregated duty bearers | [Employer, Self-employed, Controller of premises] |
| `rights_holder` | Aggregated rights holders | [Employee, Safety representative] |
| `duty_type` | DRRP types present | [Obligation, Power, Responsibility] |

#### Fitness Dimensions (from LRT or aggregated from Postgres)

| Dimension | Example values | Purpose |
|-----------|---------------|---------|
| `fitness_person` | [Employer, Employee, Self-employed person] | Who bears duties |
| `fitness_process` | [Construction work, Work at height, Manual handling] | What activities are regulated |
| `fitness_place` | [Workplace, Construction site, Great Britain] | Where obligations apply |
| `fitness_plant` | [Work equipment, Personal protective equipment] | What equipment/substances |
| `fitness_sector` | [Construction, Manufacturing, Healthcare] | Which industries |

#### Obligation Set (from Postgres)

Governed provisions — filtered to DRRP = Obligation, HIGH + MEDIUM significance, governed actors only:

| Field | Purpose | Example |
|-------|---------|---------|
| `section_id` | Structural citation | UK_ukpga_1974_37:s.2(1) |
| `text` | Full provision text | "It shall be the duty of every employer..." |
| `drrp_types` | Classification | [Obligation] |
| `governed_actors` | Who bears this duty | [Employer] |
| `clause_refined` | "Who must do what" extract | "Employer must ensure health, safety and welfare of employees" |
| `purposes` | Provision purpose | [Substantive] |
| `popimar` | Management system classification | [Policy, Organisation, Planning] |
| `actors` | Full actor struct | [{label: "Employer", position: "active"}, ...] |
| `significance_overall` | How important | HIGH |

#### Structural Context

Provisions are presented in document order (by `sort_key`). Application/scope provisions (purposes = "Application+Scope") are included as context even if not Obligation-typed — they define who and what the duties apply to. Definitional provisions are included as context but not mapped to controls.

### Chunking Strategy for Large Laws

Laws that exceed the context window (~20K provision tokens) cannot be processed in a single call. v0.1 proposed sequential processing with carry-forward — this is broken because it prevents cross-part consolidation and produces an incoherent overarching control based only on the first chunk.

**Corrected approach:**

1. **Chunk by structural unit.** Split along the law's own structure (Part, Chapter, Schedule). Each chunk should be semantically coherent — provisions that belong together stay together.
2. **Handle oversized Parts.** If a single Part exceeds the token budget, split at the next structural level (heading group, regulation block). Never split mid-provision.
3. **Generate independently.** Each chunk produces *candidate* specific controls only. No overarching control at this stage — the overarching control requires seeing the full picture.
4. **Carry metadata, not controls.** Each chunk receives the full law outline and fitness dimensions (small, constant) but NOT the controls generated from other chunks.

This produces a flat list of candidate controls per law, which Phase 3 consolidates.

### Output: Candidate Control

Each candidate control from the LLM:

```json
{
  "title": "indicative statement — the standard and the test in one sentence",
  "description": "what reality this stands for — the referent, not the paperwork",
  "what_it_checks": "the discriminating test — what would look different if this control had failed",
  "control_type": "Preventive | Detective | Corrective | Directive",
  "nature": "Manual | Automated | IT-dependent manual",
  "domain": "Organisational | People | Physical | Technical",
  "frequency": "Continuous | Daily | Weekly | Monthly | Quarterly | Annual | Ad-hoc",
  "info_distance": "Direct | Adjacent | Mediated | Remote",
  "blast_radius": "Local | Area | Site | Enterprise",
  "expected_touch_frequency": "how often this control is exercised under normal demand — distinct from verification frequency",
  "linked_provisions": ["section_id", ...],
  "mapping_strength": "Primary | Supporting | Ancillary",
  "load_bearing_judgement": "the judgement term the obligation encodes (adequate, competent, proportionate, etc.) — null if fully reducible",
  "evidence_hint": {
    "type_a": "what activity evidence looks like (the legible proxy)",
    "type_b": "what outcome evidence looks like (the discriminating test)"
  },
  "honest_limit": "what part resists reduction to a checkable predicate"
}
```

### LLM-Estimated Fields vs Runtime Fields vs Customer Fields

Three categories, clearly separated:

#### LLM-estimated (defaults, customer overrides)

| Field | How the LLM estimates | Reliability | Customer overrides? |
|-------|----------------------|-------------|-------------------|
| `info_distance` | From obligation text and control nature. "The employer shall ensure" = the duty-bearer is organisationally distant from the point of work (Mediated/Remote). "Every person shall" = the duty-bearer *is* the person at the point of work (Direct). A manual operator procedure = Direct/Adjacent; a management review = Mediated; a corporate policy = Remote. | **Moderate** — the obligation text strongly implies the distance in most cases. The LLM is estimating the *typical* organisational pattern, not the customer's actual structure. | Yes — actual org structure may differ |
| `blast_radius` | From obligation scope and hazard profile. A provision about a single workstation = Local; site-wide system = Site; duty on employer for "all employees" = Enterprise. Fitness dimensions (Process, Place) are strong signals. | **Low-moderate** — depends on customer's operational scale. A "Site" control for a single-site company = Enterprise. | Yes — actual operational scale may differ |
| `expected_touch_frequency` | From the control's nature and demand regime. A machine guard is touched every time the machine runs (Continuous). A confined space permit is touched per entry (Ad-hoc but frequent). An annual fire risk assessment is touched once per year. | **Moderate-high** — the demand regime is largely inherent to the control type, not the customer's specific operations. | Yes — actual demand may differ |

#### Runtime (tracked by the system, not estimated)

| Field | What it is | How it's tracked |
|-------|-----------|-----------------|
| `last_touched` | The last time anyone *exercised* this control — not a formal verification, but the control being used in the normal course of work. The machine operator whose guard is working is touching that control. The person who enters a confined space through a functioning permit system is touching that control. | Updated from L4 evidence flow. Any artefact or judgement linked to the control updates `last_touched`. |
| `staleness` | The gap between `expected_touch_frequency` and `last_touched`. A control expected to be exercised daily that hasn't been touched in 30 days is stale. A control expected to be exercised annually that was touched 6 months ago is fresh. | Computed: `now() - last_touched` vs `expected_touch_frequency`. |
| `coverage_status` | No Artefact / Artefact Only / Judgement Current / Judgement Stale | Computed from L4 artefact and judgement records |

**Why this matters for Expected Loss:**

```
Expected Loss = Uncertainty × Consequence

Where:
  Uncertainty = f(Info_Distance, Staleness)
  Consequence = Blast_Radius
```

- `Info_Distance` tells you how much signal degrades between the control and the person who needs assurance
- `Staleness` (derived from `last_touched` vs `expected_touch_frequency`) tells you how confident you are that the control still works
- `Blast_Radius` tells you how bad it is if you're wrong

A Remote, stale control with Enterprise blast radius has the highest expected loss. A Direct, fresh control with Local blast radius has the lowest. The LLM provides the starting estimates for `info_distance`, `blast_radius`, and `expected_touch_frequency`. The system tracks `last_touched`. The customer overrides the estimates where their context differs. The VoI calculation drives where evidence effort goes.

#### Customer-set (no LLM estimation)

| Field | Why | Who sets it |
|-------|-----|-------------|
| `Owner` | Requires knowledge of customer's org structure | Customer compliance team |
| `Org_Unit` | Requires Hierarchy table | Customer |
| `Location` | Requires Hierarchy table | Customer |
| `External_Ref` | Pointer to existing procedure/policy | Customer |
| `Demand_Mode` | Current operating mode (Normal/Abnormal/Emergency) | Customer |
| `Design_Effectiveness` | Requires assessment of control adequacy | L2 Assessment |
| `Operating_Effectiveness` | Requires evidence the control operates | L4/L5 |

---

## Phase 2: Validate

v0.1 relied on the system prompt alone to enforce the design constraints. This is insufficient — LLMs drift back to deontic language and paperwork referents. Phase 2 catches this.

### Step 1: Automated Lint

A deterministic script (no LLM) checks structural requirements:

| Check | Rule | Action on failure |
|-------|------|------------------|
| Valid JSON | Parseable, required fields present | Reject — regenerate |
| Deontic verbs in title | Title contains "must", "shall", "should", "will ensure", "needs to" | Flag for rework |
| Paperwork referent | Description contains "document exists", "record is maintained", "form is completed" without a follow-up reality check | Flag for rework |
| Empty load_bearing_judgement | Obligation text contains "adequate", "competent", "proportionate", "sufficient", "suitable", "effective", "independent" but `load_bearing_judgement` is null | Flag — likely missed |
| Provision linkage | Every `linked_provisions` section_id exists in the input obligation set | Reject invalid links |
| Enum values | All enum fields contain valid values | Reject — regenerate |

### Step 2: LLM Self-Critique

A second, cheaper LLM call (Gemini Flash) reviews each control against the design constraints:

```
Review this generated control against the following checklist:
1. Is the title a statement of a verifiable condition, or a vague goal? ("Safety is maintained" = FAIL. "All access points are fitted with a functioning interlock guard" = PASS)
2. Does the description refer to the state of the work, or the state of a file?
3. Does what_it_checks describe what would look DIFFERENT if the control failed (Type-B), or just whether an activity was performed (Type-A)?
4. If there is a judgement term in the obligation, is it flagged in load_bearing_judgement?

For each check, answer PASS or FAIL with a one-sentence reason.
```

Controls that fail any check are flagged for human review with the critique attached.

---

## Phase 3: Consolidate

v0.1 said "consolidate where the operational mechanism is the same" but gave no algorithm. This section specifies one.

### Step 1: Intra-Law Consolidation

After Phase 2, each law has a flat list of validated candidate controls. Many will overlap — e.g., three provisions about risk assessment producing three nearly-identical controls.

**Algorithm:**

1. **Embed.** Generate a vector embedding for each candidate control from `title + description`. Use the same sentence-transformer model already in the pipeline (all-MiniLM-L6-v2, 384-dim).
2. **Cluster.** Apply HDBSCAN (density-based, no pre-specified k) on the embeddings. Controls within a cluster are semantically similar enough to merge.
3. **Synthesise.** For each cluster of 2+ controls, feed the full text of all clustered controls to the LLM: "These N controls address related obligations. Synthesise them into a single control that captures the shared operational mechanism. Preserve links to ALL original provisions. Follow the same output schema."
4. **Pass through singletons.** Controls that didn't cluster are kept as-is.

### Step 2: The Policy Predicate (Replacing "Overarching Control per Law")

v0.1 proposed one "overarching control" per law. Gemini's review argued this was too coarse for framework acts and proposed multiple theme controls per structural Part. On reflection, this over-engineers the problem and misunderstands the purpose.

The overarching control is not trying to be a comprehensive operational control. It is a **policy predicate** — the law's "big idea" stated as a checkable proposition. It answers: *what is the shift the state wants to achieve?* or *if you do one thing, what is it?*

UK legislation already gives us this. Every Act carries a long title ("An Act to make further provision for securing the health, safety and welfare of persons at work..."). Every SI carries an Explanatory Note summarising the instrument's purpose. The lawyers have already done the summarisation work — the LLM's job is to restate it as a checkable indicative predicate.

**Examples:**

| Law | Long title / purpose | Policy predicate |
|-----|---------------------|-----------------|
| HSWA 1974 | "...securing the health, safety and welfare of persons at work, for protecting others against risks..." | "Work does not harm the people who do it or the people affected by it" |
| Confined Spaces Regs 1997 | "...safe working in confined spaces" | "People do not enter confined spaces unless entry is unavoidable, and when they do, the specific risks are assessed and emergency rescue is ready" |
| MHSW Regs 1999 | "...implementing Council Directive 89/391/EEC on the introduction of measures to encourage improvements in the safety and health of workers at work" | "Every significant workplace risk is assessed, and the assessment drives the controls that are actually in place" |
| COSHH Regs 2002 | "...control of substances hazardous to health" | "No one is exposed to a hazardous substance at work without the exposure being assessed and controlled to a level that does not damage their health" |

Note what these are NOT:
- Not comprehensive (they don't cover every provision)
- Not operational (they don't specify *how*)
- Not always fully checkable (they may contain irreducible judgement — and that's honest)

They ARE:
- The law's intent, in indicative mood
- A policy position the organisation either meets or doesn't
- A starting point for the customer to adopt and own as their policy statement for that domain

**For goal-setting legislation** (like HSWA), the policy predicate is almost the entire law. The specific controls derived from ss.2-8 flesh it out operationally, but the predicate itself may be as broad as "the organisation is safe" — and the `honest_limit` should say so. That's not a failure of the method; it's the method telling you the truth about what HSWA is: a goal-setting framework that deliberately encodes judgement everywhere.

**For prescriptive legislation** (like CDM 2015, LOLER 1998), the policy predicate is more specific and the specific controls are more concrete.

**Most laws need one policy predicate, not multiple themes.** HSWA is occupational safety — one domain. The governed sections (ss.2-8) all serve the same goal. The administrative machinery (inspectors, enforcement, HSE powers) is excluded by the governed-only filter. Multi-domain regulations like Environmental Permitting may genuinely need 2-3 predicates (waste, water, emissions), but this is the exception.

**Generation:**

After intra-law consolidation, the LLM receives:
- The law's `description` (long title / purpose statement)
- The law's `explanatory_note` (Explanatory Note — plain-language summary by the drafting lawyers, available in DuckDB)
- The consolidated set of specific controls
- Prompt: "What is the single shift this law is trying to achieve? State it as a checkable proposition in indicative mood. This is a policy predicate, not an operational control."

The Explanatory Note is the richest input for policy predicate generation. It is written by the law's own drafters to explain the instrument's purpose in accessible language. For SIs, it typically opens with "These Regulations impose requirements with respect to..." or "These Regulations make provision for..." — a direct statement of legislative intent that the LLM restates as a checkable proposition.

**Mapping:**

```
Policy predicate → maps at law level, Primary strength
  └── Specific controls → map at provision level, Primary/Supporting/Ancillary
```

### Step 3: Cross-Law Deduplication (Family Consolidation)

Two laws in the same family may produce nearly-identical controls. E.g., a "risk assessment" control from HSWA and a "risk assessment" control from MHSW Regulations.

**Algorithm:**

1. **Title hash.** Normalise titles (lowercase, strip punctuation, stem words), hash. Exact matches are trivial duplicates — merge, combine provision links.
2. **Embedding similarity.** For non-identical titles, compute cosine similarity on embeddings. Pairs above 0.9 threshold are near-duplicates.
3. **Propose merge.** Near-duplicates are flagged for human confirmation in Phase 4. The system proposes: "These 3 controls from 3 different laws appear to describe the same operational mechanism. Merge?" The merge combines provision links from all source controls.
4. **Cross-law theme controls.** After family-level dedup, if a single control now maps to provisions across multiple laws, it is a **tube map interchange** — a high-value control. Flag it as such.

---

## Phase 4: Human Review

The pipeline produces *suggested* controls, not L3 records. A human must accept, edit, or reject each one.

### Status Transitions

```
Generated → Validated → Consolidated → Suggested → Accepted / Rejected / Edited
                                           │
                                           ├── Accepted → Active (in L3 Controls table)
                                           ├── Rejected → Discarded (with reason)
                                           └── Edited → the customer's version becomes the control
```

### What the Review UI Shows

For each suggested control:
- The control itself (title, description, what_it_checks, evidence hints)
- The linked provisions with their obligation text (so the reviewer can judge coverage)
- The LLM-estimated fields (info_distance, blast_radius, expected_touch_frequency) clearly marked as "AI-suggested defaults — override with your operational context"
- Any Phase 2 validation flags
- Any Phase 3 merge proposals

### Feedback Capture

Customer edits are a training signal. When a customer rewrites a control title, changes a control type, or adjusts an evidence hint, the before/after is captured:

| Field | Generated value | Customer value |
|-------|----------------|---------------|
| `title` | "Every person entering a confined space holds current competence" | "Confined space entrants hold current CS01 certification and have completed the site-specific induction" |

This `(generated, customer_edited)` pair is stored alongside the control. Over time, these pairs become few-shot examples for prompt improvement and potential fine-tuning data.

---

## Design Constraints: The System Prompt

### Constraint 1: Indicative Mood — The Ought-Is Shift

> Write every control as a statement that is observably true or false right now. Not "must ensure" but "is ensured." Not "shall provide" but "is provided." The obligation lives in ownership — a named person has declared this is required. The grammar is indicative; the authority comes from the role.
>
> **Test**: can you walk onto a site and check whether this statement is true? If not, rewrite it until you can.

### Constraint 2: Referent, Not Paperwork

> State what reality the control stands for, not what document proves it. "The risk assessment is adequate to the actual hazard" — not "a risk assessment document exists." The referent is the state of the work, not the state of the file.
>
> **Test**: would this control still be true if the paperwork were perfect but the workplace were dangerous? If yes, the control is checking paperwork, not reality.

### Constraint 3: The Discriminating Test

> For each control, state what would look different if the control had failed. This is the Type-B evidence — the outcome test that discriminates between "the control works" and "the control doesn't work."
>
> A filed form that would exist whether or not the control works is Type-A (activity evidence). A test result that would look different depending on whether the control works is Type-B (outcome evidence). Always specify both, but prioritise Type-B.

### Constraint 4: Honest Limits — Flag the Judgement

> Some obligations encode judgement terms that resist full reduction to a checkable predicate: *adequate, competent, proportionate, sufficient, independent, effective*. These are the load-bearing obligations.
>
> Do not pretend these reduce cleanly. Flag the judgement term explicitly. State what can be checked (the partial predicate) and what needs a calibrated person's assessment.

### Constraint 5: Consolidation — The Tube Map

> One control can serve multiple provisions. Where several provisions create related duties, produce a single control that covers them all. Do not create one control per provision. Flag where consolidation would lose specificity.

### Constraint 6: Proportionality

> Not every provision needs a dedicated control. Definitional, procedural, and application-scoping provisions qualify other provisions — they do not create standalone duties. Low-significance obligations may be covered by theme controls alone.

### Constraint 7: Control Type Accuracy

> Classify the control by what it *does*:
> - **Preventive**: stops harm before it occurs (risk assessment, permit, guard, training)
> - **Detective**: identifies harm/non-compliance after it occurs (inspection, monitoring, reporting)
> - **Corrective**: restores safe state after detection (emergency response, investigation)
> - **Directive**: guides behaviour toward compliance (policy, procedure, signage)

### Constraint 8: Estimate Operational Properties

> For each control, estimate three operational properties from the obligation text and control nature:
>
> - **Info Distance**: how far the controller is from the controlled. "Every person shall" = Direct. "The employer shall ensure" = Mediated. A corporate policy = Remote.
> - **Blast Radius**: scope of consequence if the control fails. Single workstation = Local. Site-wide system = Site. All employees = Enterprise.
> - **Expected Touch Frequency**: how often this control is exercised under normal demand. A machine guard = every time the machine runs (Continuous). A confined space permit = per entry (Ad-hoc/frequent). An annual fire risk assessment = Annual.
>
> These are defaults. The customer overrides them with their operational context.

### Few-Shot Examples

The system prompt includes 2-3 worked examples showing:
- A **good** control (indicative, checkable, discriminating test, load-bearing flagged)
- A **bad** control (deontic verbs, paperwork referent, no discriminating test) rewritten into a good one
- A **consolidation** example (3 provisions → 1 control with all links preserved)

This few-shot anchoring is critical for preventing deontic drift. Stating the principle alone is insufficient — the model needs to see the pattern concretely.

---

## Prompt Architecture

### Structure

```
┌─────────────────────────────────────────┐
│  SYSTEM PROMPT                          │
│  - Role: compliance controls architect  │
│  - Design constraints 1-8              │
│  - Output JSON schema                   │
│  - Few-shot examples (good, bad, merge) │
│  - ~2,500 tokens                        │
└─────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│  USER PROMPT (per law or chunk)         │
│  - Law outline (metadata + fitness)     │
│  - Obligation set (provisions in order) │
│  - "Generate candidate controls"        │
│  - ~500 + 2,000–15,000 tokens           │
└─────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│  ASSISTANT RESPONSE                     │
│  - Candidate controls (N)               │
│  - ~2,000–5,000 tokens                  │
└─────────────────────────────────────────┘
```

Note: no policy predicate at this stage. That is generated in Phase 3 after consolidation, from the law's long title/purpose and the consolidated control set.

### Token Budget

| Component | Estimated tokens | Notes |
|-----------|-----------------|-------|
| System prompt | ~2,500 | Constraints + schema + few-shot examples |
| Law outline + fitness | ~500 | Constant per law |
| Obligation set (typical law, ≤30 provisions) | 2,000–6,000 | Fits in one call |
| Obligation set (medium law, 30–80 provisions) | 6,000–15,000 | Fits in one call |
| Obligation set (large law, 80+ provisions) | 15,000+ | Chunk into structural units |
| Output per call | 2,000–5,000 | Structured JSON |
| **Total per call** | **~5,000–23,000** | Within Gemini Pro context |

### Model Choice

- **Phase 1 (Generate)**: Gemini 2.5 Pro — complex generation task, needs reasoning
- **Phase 2 (Validate)**: Gemini 2.5 Flash — simple checklist evaluation, cost-sensitive
- **Phase 3 (Consolidate/Synthesise)**: Gemini 2.5 Pro — synthesis requires understanding
- **Phase 3 (Theme controls)**: Gemini 2.5 Pro — summarisation from consolidated set

### Two Modes

**Mode 1: Generation (canonical)**

Generate controls from the law alone. Produces a clean, reusable baseline that can be offered to any customer. This is the primary mode described in this document.

**Mode 2: Reconciliation (customer-specific)**

A separate prompt and pipeline. Input: the canonical controls from Mode 1 + the customer's existing control library. Task: for each canonical control, find the best match in the customer's library. Output: proposed mappings + identified gaps.

These modes are deliberately separated. Mixing them in one prompt contaminates the canonical generation — the LLM borrows the customer's terminology, making controls non-standard and harder to compare across customers.

---

## Post-Processing: LLM Output → L3 Schema

### Control Record

| LLM field | → Control field | Notes |
|-----------|----------------|-------|
| `title` | `Title` | Indicative statement |
| `description` | `Description` | What reality it stands for |
| `control_type` | `Control_Type` | Preventive / Detective / Corrective / Directive |
| `nature` | `Nature` | Manual / Automated / IT-dependent manual |
| `domain` | `Domain` | Organisational / People / Physical / Technical |
| `frequency` | `Frequency` | Normal demand rate |
| `blast_radius` | `Blast_Radius` | LLM-estimated default |
| `info_distance` | `Info_Distance` | LLM-estimated default |
| `expected_touch_frequency` | `Expected_Touch_Frequency` | LLM-estimated — drives staleness calculation |
| — | `Owner` | Customer assigns |
| — | `Status` | Default: Planned |
| — | `Tier` | Default: Jurisdiction |
| — | `Design_Effectiveness` | Default: Not Tested |
| — | `Operating_Effectiveness` | Default: Not Tested |
| — | `Last_Touched` | Runtime — updated from L4 evidence flow |
| `what_it_checks` | `Notes` | Appended with honest_limit |
| `evidence_hint` | `Notes` | Appended: Type-A / Type-B evidence guidance |

### Control Mapping Record

| LLM field | → Mapping field | Notes |
|-----------|----------------|-------|
| — | `Law` | Always populated from the law being processed |
| `linked_provisions` | `Obligation` | section_id → Duties row lookup |
| — | `Control` | Link to the generated Control |
| `mapping_strength` | `Strength` | Primary / Supporting / Ancillary |

### Policy Predicate Mapping

The policy predicate maps at **law level** with **Primary** strength. It is a directive — a policy position in indicative mood. Specific controls map at provision level beneath it.

---

## Storage

### Staging Table (DuckDB)

Suggested controls live in a `suggested_controls` staging table until human review. This table preserves the LLM's full output including evidence hints, honest limits, and validation flags that may not survive the Baserow schema.

| Column | Type | Purpose |
|--------|------|---------|
| `id` | UUID | Primary key |
| `law_name` | TEXT | Source law |
| `control_json` | JSON | Full LLM output |
| `status` | TEXT | generated / validated / consolidated / suggested / accepted / rejected / edited |
| `validation_flags` | JSON | Phase 2 lint + critique results |
| `merge_proposal` | JSON | Phase 3 dedup candidates, if any |
| `customer_edits` | JSON | Before/after pairs from Phase 4 |
| `generation_model` | TEXT | Which model + prompt version |
| `generated_at` | TIMESTAMP | When |
| `base_hash` | TEXT | Hash of the generated control — for three-way merge on regeneration |

### Versioning: Three-Way Merge on Regeneration

When controls are regenerated (law amended, model improved), the system must not overwrite customer edits. This is a three-way merge:

1. **Identify three versions:**
   - `base`: the control as originally generated (stored as `base_hash` + full JSON)
   - `theirs`: the customer's current version (may have been edited in Phase 4)
   - `ours`: the newly generated version

2. **Compute diffs:**
   - `diff(base, theirs)` = customer changes
   - `diff(base, ours)` = system changes

3. **Merge logic:**
   - Only `theirs` changed → keep `theirs` (customer made it their own)
   - Only `ours` changed → apply to `theirs` automatically if non-conflicting, flag for review if the change touches the same field
   - Both changed the same field → **merge conflict** — present both versions to the customer in the review UI

---

## Batch Strategy

### Canonical Generation (one-time, reusable)

The canonical control set is generated **once per law in the corpus**, not per customer. It is a reusable asset — the starting point any customer builds from.

| Segment | Estimated laws | Calls per law | Total Phase 1 calls |
|---------|---------------|---------------|---------------------|
| Small SIs (≤20 governed provisions) | ~1,600 (80%) | 1 | ~1,600 |
| Medium laws (20–80 provisions) | ~300 (15%) | 1 | ~300 |
| Large framework Acts (80+ provisions) | ~100 (5%) | 2–3 (chunked) | ~250 |
| **Total Phase 1** | **~2,000** | | **~2,150** |

Add Phase 2 validation (Flash, cheap): ~2,150 calls. Phase 3 consolidation/synthesis (Pro): ~500–1,000 calls (only laws with clusters or cross-law dedup candidates). Policy predicate generation: ~2,000 calls (one per law, cheap — short prompt).

**Total: ~6,000–7,000 LLM calls for the full corpus.** Well within Gemini batch API capacity. Run once, store canonical controls, update incrementally.

### Customer Delivery

Each customer receives the canonical controls for the laws in their Legal Register (~200–300 laws). No per-customer generation needed — the canonical set is filtered to their register and delivered as suggested controls with Status = Planned.

### Mode 2: Reconciliation (on request)

If a customer has an existing control library, Mode 2 matches canonical controls to their existing mechanisms and identifies gaps. This is a separate, per-customer operation — smaller, cheaper, and only run when requested.

### Incremental Generation

When a new law enters the corpus (via L6 change detection), generate canonical controls for that law only. When a law is amended, regenerate and apply the three-way merge against any customer-edited versions.

### CLI Commands

```bash
# Phase 1-3: Generate, validate, consolidate for a single law
fractalaw controls generate UK_ukpga_1974_37

# Phase 1-3 for a family
fractalaw controls generate --family "OH&S: Occupational / Personal Safety"

# Phase 1-3 for a customer's full register
fractalaw controls generate --all --significance HIGH,MEDIUM

# Dry run — show Phase 1 prompt without calling LLM
fractalaw controls generate UK_ukpga_1974_37 --dry-run

# List suggested controls (Phase 4 input)
fractalaw controls list UK_ukpga_1974_37

# Publish accepted controls to sertantai
fractalaw controls publish --tenant dev
```

---

## Worked Example: Confined Spaces Regulations 1997

### Phase 1 Input (abbreviated)

```
## Law Outline

Title: The Confined Spaces Regulations 1997
Family: OH&S: Occupational / Personal Safety
Year: 1997, Jurisdiction: UK, Status: In force
Description: "Regulations for safe working in confined spaces"
Explanatory Note: "These Regulations impose requirements with respect to
the carrying out of work in confined spaces. They require the avoidance
of entry to confined spaces where this is not reasonably practicable,
and where entry is unavoidable, the preparation of a suitable and
sufficient assessment of the risks, the taking of adequate safety
precautions and the provision of emergency arrangements."
Duty holders: [Employer, Self-employed person]
Rights holders: [Employee]
Fitness — Person: [Employer, Self-employed person, Employee]
Fitness — Process: [Work in confined space, Entry into confined space]
Fitness — Place: [Confined space]

## Obligations

### reg.1(2) — Application+Scope
"These Regulations shall apply to and in relation to work in a confined space."

### reg.3(a) — Obligation
"No person at work shall enter a confined space to carry out work for any
purpose unless it is not reasonably practicable to achieve that purpose
without such entry."
Actors: [{label: "Employer", position: "active"}, {label: "Employee", position: "counterparty"}]

### reg.3(b) — Obligation
"...and so far as is reasonably practicable, without entering the confined
space, there is established a safe system of work."
Actors: [{label: "Employer", position: "active"}]

### reg.4(1) — Obligation
"No person at work shall enter or carry out work in a confined space unless
there has been prepared... a suitable and sufficient assessment of the risks..."
Actors: [{label: "Employer", position: "active"}]

### reg.4(2) — Obligation
"The assessment referred to in paragraph (1) shall include consideration of—
(a) the nature of the confined space; (b) the risk of fire or explosion;
(c) the risk of loss of consciousness or asphyxiation..."

### reg.5 — Obligation
"Where the assessment identifies a risk of serious injury... emergency
arrangements shall be in place before any person enters or carries out work..."
Actors: [{label: "Employer", position: "active"}]
```

### Phase 1 Output: Candidate Controls

```json
[
  {
    "title": "Before any confined space entry, a documented consideration of alternatives demonstrates that entry cannot reasonably be avoided",
    "description": "The reality: someone has genuinely thought about whether the work can be done from outside. Not a tick-box 'alternatives considered: N/A' but a recorded reasoning that names what alternatives were evaluated and why they were ruled out.",
    "what_it_checks": "The alternatives log names specific methods considered (CCTV, long-reach tools, remote sampling) and gives reasons for rejection. A blank or formulaic entry is a signal, not a pass.",
    "control_type": "Preventive",
    "nature": "Manual",
    "domain": "Organisational",
    "frequency": "Ad-hoc",
    "info_distance": "Adjacent",
    "blast_radius": "Local",
    "expected_touch_frequency": "Per confined space entry — exercised every time a permit is raised",
    "linked_provisions": ["UK_uksi_1997_1713:reg.3(a)", "UK_uksi_1997_1713:reg.3(b)"],
    "mapping_strength": "Primary",
    "load_bearing_judgement": "reasonably practicable — whether an alternative is practicable requires weighing cost, effort, and time against the risk of entry",
    "evidence_hint": {
      "type_a": "Alternatives section completed on permit form",
      "type_b": "Alternatives log names specific methods with substantive reasons for rejection, reviewed by an independent person"
    },
    "honest_limit": "'Reasonably practicable' is the central judgement. No checklist captures whether the organisation has genuinely explored alternatives or just ticked the box."
  },
  {
    "title": "A risk assessment specific to the confined space, its current conditions, and the planned work is completed and available at the point of entry",
    "description": "The reality: the assessment addresses THIS space, THIS day's conditions, and THIS task. 'Suitable and sufficient' means it matches the actual hazard, not that a generic template was filled in.",
    "what_it_checks": "The assessment names the specific space, dates, atmospheric conditions expected, adjacent processes, and the particular task. A generic assessment reused across spaces is a signal.",
    "control_type": "Preventive",
    "nature": "Manual",
    "domain": "Organisational",
    "frequency": "Ad-hoc",
    "info_distance": "Adjacent",
    "blast_radius": "Local",
    "expected_touch_frequency": "Per confined space entry — exercised every time an entry is authorised",
    "linked_provisions": ["UK_uksi_1997_1713:reg.4(1)", "UK_uksi_1997_1713:reg.4(2)"],
    "mapping_strength": "Primary",
    "load_bearing_judgement": "suitable and sufficient — whether the assessment is adequate to the actual hazard requires competence in confined space hazards",
    "evidence_hint": {
      "type_a": "Risk assessment form completed and signed",
      "type_b": "Assessment references specific atmospheric monitoring results, names the space and date, identifies hazards unique to this entry"
    },
    "honest_limit": "'Suitable and sufficient' is tested in enforcement. The assessment's adequacy is a judgement — it requires a person who understands the specific hazards of the space."
  },
  {
    "title": "Emergency rescue arrangements for the confined space have been tested with the designated rescue team before the entry begins",
    "description": "The reality: not 'an emergency plan exists on paper' but 'the people who would perform the rescue have practised it for this type of space and the equipment is present and functional'.",
    "what_it_checks": "The rescue team is identified by name, has practised rescue from this space type within the last 12 months, the rescue equipment is present and tested, and communication is confirmed.",
    "control_type": "Corrective",
    "nature": "Manual",
    "domain": "People",
    "frequency": "Ad-hoc",
    "info_distance": "Direct",
    "blast_radius": "Local",
    "expected_touch_frequency": "Per confined space entry (standby activation) + quarterly drill for each space type",
    "linked_provisions": ["UK_uksi_1997_1713:reg.5"],
    "mapping_strength": "Primary",
    "load_bearing_judgement": null,
    "evidence_hint": {
      "type_a": "Emergency arrangements section completed on permit",
      "type_b": "Rescue drill record for this space type within last 12 months, rescue equipment functional test, named rescue team members with current competence"
    },
    "honest_limit": null
  }
]
```

### Phase 3 Output: Policy Predicate

The Explanatory Note states: "These Regulations impose requirements with respect to the carrying out of work in confined spaces. They require the avoidance of entry to confined spaces where this is not reasonably practicable, and where entry is unavoidable, the preparation of a suitable and sufficient assessment of the risks, the taking of adequate safety precautions and the provision of emergency arrangements." The LLM restates this as a checkable policy predicate:

```json
{
  "title": "People do not enter confined spaces unless entry is unavoidable, and when they do, the specific risks are assessed and emergency rescue is ready",
  "description": "This is the shift the Confined Spaces Regulations are trying to achieve. The hierarchy is: avoid entry entirely (reg.3), and if entry is unavoidable, assess the specific risks (reg.4) and have tested emergency arrangements (reg.5). The policy position is that confined space entry is a last resort, not a routine.",
  "what_it_checks": "Are entries being avoided where alternatives exist? When entries happen, are they preceded by space-specific assessment and supported by tested rescue arrangements?",
  "honest_limit": "'Reasonably practicable' (reg.3) and 'suitable and sufficient' (reg.4) are irreducible judgement terms. This policy predicate encodes the law's goal-setting intent — the specific controls below operationalise it.",
  "maps_to": "law level, Primary"
}
```

---

## Quality Signals

### Red Flags (control needs rework)

- **Deontic verbs**: title contains "must", "shall", "should", "will ensure" — failed Constraint 1
- **Paperwork referent**: description says "a document exists" or "a record is maintained" without naming the reality behind the paper — failed Constraint 2
- **No discriminating test**: `what_it_checks` describes activity (was it done?) not outcome (did it work?) — failed Constraint 3
- **Hidden judgement**: obligation text contains a judgement term but `load_bearing_judgement` is null — failed Constraint 4
- **One control per provision**: no consolidation, overlapping duties get separate controls — failed Constraint 5
- **Controls for definitions**: a control mapped to a definitional or scope provision — failed Constraint 6
- **Vague indicative**: title is indicative but uncheckable ("Safety is maintained", "Risks are managed") — passes Constraint 1 technically but fails its spirit

### Green Flags (control is well-formed)

- Title reads as something you can walk onto a site and verify
- Description distinguishes the reality from the proxy
- Evidence hints include a Type-B test that discriminates
- Load-bearing judgement terms are surfaced, not hidden
- Multiple provisions consolidated where the operational mechanism is the same
- `expected_touch_frequency` is specific to the control's demand regime, not generic

---

## Open Questions (Remaining)

Questions 1-6 from v0.1 are addressed in the design above. Remaining open questions:

1. ~~**Explanatory Note scraping.**~~ **Resolved.** Explanatory Notes will be scraped by sertantai and available as `explanatory_note` in DuckDB LRT before this pipeline is implemented. Now a first-class input to the policy predicate prompt.

2. **Feedback loop to prompt improvement.** Phase 4 captures customer edits as (generated, edited) pairs. How and when do these feed back into the system prompt's few-shot examples? Manual curation? Automated selection of highest-signal edits? Fine-tuning dataset?

3. **Control hierarchy.** The pipeline produces a flat list + a policy predicate. Real-world control frameworks are sometimes hierarchical (e.g., 5 specific controls grouped under a "Change Management Process" parent). Should the system suggest hierarchical groupings, or leave this entirely to the customer?

4. **QA approach.** A traditional golden dataset (human-written reference controls) is problematic: human compliance officers write in imperatives, which is exactly the form this system is designed to improve on. You cannot benchmark a new form against examples written in the old form. QA will be empirical: (a) Phase 2 automated constraint-check pass rates (deontic lint, referent check, judgement flag); (b) customer edit rates from Phase 4 feedback capture — high edit rates on a field signal the prompt needs work; (c) spot-check review of generated controls for a representative sample of laws across families. The test of the pudding will be the eating.

5. **Mode 2 (Reconciliation) design.** The separation between Generation and Reconciliation is clear. The Reconciliation prompt — matching canonical controls to a customer's existing library — is a distinct design task. Spec needed.

6. **Multi-domain regulations.** Most laws need one policy predicate. But genuinely multi-domain regulations (Environmental Permitting covers waste, water, emissions, radioactive substances) may need 2-3 predicates. Should this be detected automatically (e.g., from sub_family or POPIMAR diversity in the provision set) or flagged for human decision?

---

## Related Documents

- [`BASEROW-CONTROLS-DESIGN.md`](../../Desktop/sertantai-legal/docs/compliance/l3-controls/BASEROW-CONTROLS-DESIGN.md) — L3 schema and ontology
- [`COMPLIANCE-7-LAYERS.md`](../../Desktop/sertantai-legal/docs/compliance/COMPLIANCE-7-LAYERS.md) — architecture
- [`BASEROW-7-LAYERS.md`](../../Desktop/sertantai-legal/docs/compliance/BASEROW-7-LAYERS.md) — layer status and gap analysis
- [`brief_ought-is_for_leaders.md`](../../Desktop/dialectics/dialectics/output/sms-form-dialectic/brief_ought-is_for_leaders.md) — indicative mood design philosophy
- [`L3-CONTROLS.md`](../../Desktop/sertantai-legal/docs/qq/L3-CONTROLS.md) — QQ brief for controls
- [`LEGIBLE-vs-LOAD-BEARING.md`](../../Desktop/sertantai-legal/docs/compliance/l4-evidence/LEGIBLE-vs-LOAD-BEARING.md) — the operationalisation paradox
- [`DEFINITION-OF-EVIDENCE.md`](../../Desktop/sertantai-legal/docs/compliance/l4-evidence/DEFINITION-OF-EVIDENCE.md) — evidence as information that changes credence
- [`compliance-controls-design-review.md`](../../data/code-review/compliance-controls-design-review.md) — Gemini 2.5 Pro review of v0.1
