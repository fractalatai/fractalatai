# Compliance Evidence: LLM-Assisted Evidence Pattern Generation from Controls

**Status**: Design v0.2
**Date**: 2026-07-15
**Scope**: Prompt architecture and pipeline for generating L4 Evidence patterns from L3 Controls using an LLM
**Depends on**: COMPLIANCE-CONTROLS.md (pipeline patterns, staging table, prompts)
**Reviewed by**: Gemini 2.5 Pro (2026-07-15) — see `data/code-review/compliance-evidence-design-review.md`

---

## The Problem

The Controls pipeline generates L3 Controls from L1 Obligations. Each control already carries free-text `evidence_hint.type_a` and `evidence_hint.type_b` — but these are embedded in the control JSON, not structured L4 Evidence records a customer can operationalise.

The L4 Evidence tier has three entities: Artefacts (things registered), Judgements (acts performed), and Gaps (decisions taken). The customer creates these operationally — registering documents, exercising judgement, recording findings. But the *pattern* for what evidence to collect and how to assess it can be generated from the controls, just as controls are generated from obligations.

Without evidence patterns:

```
L3 Control → ??? → L4 Evidence (the customer has to figure it out)
```

With evidence patterns:

```
L3 Control → Suggested Evidence Patterns → Customer adapts → L4 Operational Evidence
```

The evidence pattern is not the evidence itself. It is the template: *for this control, here is what artefacts to register, what judgement method to use, what the basis should contain, how often to reassess, and where this sits on the VoI 2x2.*

---

## What the Pipeline Produces

For each control, an **evidence pattern** containing three sections:

### 1. Artefact Patterns

What to register as evidence that the control exists and operates.

| Field | Type | Description |
|-------|------|-------------|
| `artefact_type` | enum | What kind of thing (from L4 schema: Policy, Procedure, Certificate, Training Record, Inspection Report, Risk Assessment, Permit, Licence, Test Result, Sensor Reading, Other) |
| `artefact_class` | enum | Activity (Type-A) or Outcome (Type-B) |
| `title` | text | What this artefact is — e.g., "Atmospheric gas test reading at point of entry" |
| `what_it_proves` | text | What belief this artefact changes — the discriminating test for Type-B, the activity record for Type-A |
| `source` | enum | Where it comes from (Upload, System Generated, Sensor, External, Linked System) |
| `likelihood_ratio` | enum | Low / Medium / High — how much this artefact discriminates between "control works" and "control doesn't work" |
| `recommended_frequency` | text | How often a new instance should be registered (derived from control frequency + blast_radius) |
| `evidence_by_design` | boolean | True if this artefact is a natural byproduct of control execution, false if it requires separate collection |

Each control gets 1-3 artefact patterns: at least one Type-A (activity) and ideally one Type-B (outcome). The Type-B artefact is the one that matters — it discriminates. The Type-A artefact is table stakes.

### 2. Judgement Guidance

Whether this control needs judgement evidence (a calibrated person's assessment), and if so, what that judgement should look like.

| Field | Type | Description |
|-------|------|-------------|
| `needs_judgement` | boolean | Whether artefacts alone are insufficient — derived from control properties |
| `judgement_rationale` | text | **Always populated.** Why judgement is or isn't needed — when false, explains why artefacts alone are sufficient |
| `recommended_method` | enum | How to assess (Visual Inspection, Functional Test, Simulation, Interview, Observation, Exercise, Document Review). Null when needs_judgement=false. |
| `basis_guidance` | text | What a good basis should contain — what to look at, what to note, what to look for that's missing |
| `discriminating_question` | text | The specific question the judge should answer — restated from the control's `what_it_checks` as a question |
| `drift_signal` | text | What would indicate the measurement method has decoupled from reality (concept #3 from EVIDENCE-CALIBRATION.md) |
| `drift_conditions` | text | What would indicate the control itself has drifted — the specific conditions under which a judge should find 'Drifted' rather than 'Still True'. Bridges to the L4 Gaps entity. |

### 3. Evidence Strategy

The operational properties that determine evidence effort — largely derivable from the control's own properties, but the LLM adds domain context.

| Field | Type | Description |
|-------|------|-------------|
| `voi_quadrant` | enum | Table Stakes / No-Brainer / Judgement / Waste |
| `voi_rationale` | text | Why this control lands in this quadrant — references info_distance, blast_radius, nature |
| `evidence_standard` | enum | Basic / Focused / Comprehensive — from blast_radius (NIST depth mapping) |
| `recommended_interval` | text | How often evidence should be refreshed (not how often the control operates) |
| `sample_size_guidance` | text | For manual controls: how many instances to evidence per period (from PCAOB tables) |
| `staleness_tolerance` | enum | Low / Medium / High — how quickly stale evidence becomes a red flag |
| `nature_strategy` | text | Evidence strategy derived from the control's Nature (Automated → benchmark + ITGC; Manual → sample-based testing; IT-dependent manual → hybrid) |

---

## Pipeline Overview

The pipeline has three phases, mirroring the Controls pipeline but simpler — no consolidation needed since evidence patterns are per-control.

```
Phase 1: GENERATE             Phase 2: VALIDATE           Phase 3: REVIEW
──────────────────            ──────────────────          ─────────────────
Per-law (all controls)        Automated lint              Human-in-the-loop
  → evidence patterns           → schema check            acceptance/edit
  per control                   → enum validation
                                → needs_judgement logic
                                → VoI consistency
```

No consolidation phase: evidence patterns are per-control, so there's no semantic deduplication needed. A control maps to its evidence pattern 1:1.

---

## Phase 1: Generate Evidence Patterns

### Input Specification

The LLM receives all controls for a single law, plus the law's metadata and the underlying provision texts. This gives it the domain context to generate appropriate evidence patterns.

#### From DuckDB `suggested_controls` table

All controls for the law (control_type = 'specific'), with their full JSON:

| Field | Purpose |
|-------|---------|
| `title` | The control's indicative statement |
| `description` | What reality it stands for |
| `what_it_checks` | The discriminating test (Type-B) |
| `control_type` | Preventive / Detective / Corrective / Directive |
| `nature` | Manual / Automated / IT-dependent manual |
| `domain` | Organisational / People / Physical / Technical |
| `frequency` | How often the control operates |
| `info_distance` | Direct / Adjacent / Mediated / Remote |
| `blast_radius` | Local / Area / Site / Enterprise |
| `expected_touch_frequency` | How often the control is exercised |
| `evidence_hint` | The free-text Type-A / Type-B hints already generated |
| `load_bearing_judgement` | The judgement term, if any |
| `honest_limit` | What resists checkability |

#### From DuckDB `legislation` table

| Field | Purpose |
|-------|---------|
| `family` | Domain classification — drives artefact type expectations |
| `title` | Law title — context for the domain |

The policy predicate is not included — it's a law-level statement, not an operational control. Evidence patterns attach to specific controls.

### Why Batch Per Law

Controls within a law share regulatory context. A law about confined spaces has a domain-specific vocabulary of evidence (gas readings, rescue drill records, permit sign-offs). Processing all controls for a law in one call lets the LLM:

1. Avoid repeating domain context per control
2. Notice cross-control evidence reuse (one penetration test may serve multiple controls)
3. Calibrate VoI across controls relative to each other

### Chunking

Most laws have 3-12 controls. Even large framework Acts rarely exceed 15 controls. At ~200 tokens per control input + ~300 tokens per evidence pattern output, a 15-control law fits comfortably in a single call (~7,500 tokens input, ~4,500 tokens output, plus system prompt).

No chunking needed. If a law has >20 controls (unlikely given consolidation), split into two calls by control order.

### Output Schema

The LLM returns a JSON array, one element per control, keyed by the control's index in the input:

```json
[
  {
    "control_index": 1,
    "artefacts": [
      {
        "title": "Atmospheric gas test reading at point of entry",
        "artefact_type": "Test Result",
        "artefact_class": "Outcome",
        "what_it_proves": "The atmosphere in the confined space was tested immediately before entry and found safe for the planned work. A test showing dangerous levels would prevent entry — the evidence looks different when the control works vs when it doesn't.",
        "source": "Sensor",
        "likelihood_ratio": "High",
        "recommended_frequency": "Per confined space entry",
        "evidence_by_design": true
      },
      {
        "title": "Completed risk assessment form signed by the assessor",
        "artefact_type": "Risk Assessment",
        "artefact_class": "Activity",
        "what_it_proves": "A risk assessment was conducted for this space and this entry. This proves the activity happened — it does not prove the assessment was adequate.",
        "source": "Upload",
        "likelihood_ratio": "Low",
        "recommended_frequency": "Per confined space entry",
        "evidence_by_design": false
      }
    ],
    "judgement": {
      "needs_judgement": true,
      "judgement_rationale": "Control contains 'suitable and sufficient' — a load-bearing judgement term. The risk assessment's adequacy cannot be determined from the artefact alone. A person with confined space competence must assess whether the assessment actually matches the hazard.",
      "recommended_method": "Document Review",
      "basis_guidance": "Review the risk assessment against the actual space: does it name THIS space? Does it address the current atmospheric hazards, adjacent processes, and planned work? Has anything changed since the assessment was written? Compare the assessment's hazard list to what you can observe.",
      "discriminating_question": "Is this risk assessment actually adequate to the hazards of this specific space and this specific entry?",
      "drift_signal": "'Risk assessment completed' was once anchored to 'the assessor visited the space and identified the real hazards.' It may have drifted to mean 'the generic template was copied and the date updated.' Check whether the assessment references space-specific conditions."
    },
    "strategy": {
      "voi_quadrant": "Judgement",
      "voi_rationale": "High expected loss: the risk assessment is the gateway control — if it's inadequate, every subsequent control is calibrated to the wrong hazard. Blast_radius=Local but consequence of failure is severe (death/serious injury in confined spaces). Measurement cost is high — requires a competent person to assess adequacy. VoI justifies the cost.",
      "evidence_standard": "Focused",
      "recommended_interval": "Per entry (artefacts) + quarterly sample review (judgement)",
      "sample_size_guidance": "For judgement: review a sample of risk assessments per quarter — 2-5 depending on entry frequency. For artefacts: every entry produces a gas test reading (100% coverage, automated).",
      "staleness_tolerance": "Low",
      "nature_strategy": "Manual control: sample-based testing required. The assessor's judgement is the active ingredient — artefacts alone cannot assure adequacy. Review a sample of assessments against the actual spaces."
    }
  }
]
```

### What the LLM Estimates vs What's Deterministic

Three categories, mirroring the Controls design:

#### LLM-estimated (domain knowledge required)

| Field | Why the LLM estimates | Reliability |
|-------|----------------------|-------------|
| `artefact_type` | Requires domain knowledge — a confined space control needs gas readings, a training control needs completion records | High — the mapping from control domain to artefact type is well-established |
| `artefact_class` | Requires understanding of what discriminates — which artefacts look different when the control works vs doesn't | High — the Type-A/Type-B distinction is well-grounded |
| `what_it_proves` | Requires reasoning about the likelihood ratio — what belief the artefact changes | Moderate — this is the hardest field, requires genuine understanding of the control |
| `basis_guidance` | Requires domain expertise — what a competent assessor would look for | Moderate-High — the LLM has the obligation text and control description as anchors |
| `drift_signal` | Requires understanding of the operationalisation paradox — how measurement methods decouple from reality | Moderate — this is the most philosophically grounded field |
| `voi_rationale` | Requires reasoning about expected loss vs measurement cost | Moderate — control properties provide strong signals |

#### Deterministic (derivable from control properties)

| Field | Derivation | Rule |
|-------|-----------|------|
| `needs_judgement` | From control properties | True if: (nature=Manual AND info_distance in {Mediated, Remote}) OR blast_radius=Enterprise OR load_bearing_judgement is not null OR control_type=Directive |
| `evidence_standard` | From blast_radius | Enterprise→Comprehensive, Site→Focused, Area→Focused, Local→Basic |
| `staleness_tolerance` | From info_distance × blast_radius | Remote+Enterprise→Low, Direct+Local→High, else Medium |
| `evidence_by_design` | From control nature | Automated+Sensor→true, Manual→false (LLM can override with domain knowledge) |

The LLM receives these derivations in the system prompt as *defaults it should use unless domain knowledge says otherwise*. The lint checks verify consistency — if the LLM overrides a deterministic rule, it must provide a rationale.

#### Customer-set (no LLM estimation)

| Field | Why | Who sets it |
|-------|-----|-------------|
| Actual artefact instances | The customer registers real documents, not patterns | Customer |
| Judge identity | Requires knowledge of personnel | Customer |
| Actual assessment dates | Operational scheduling | Customer |
| calibration_mode | Maturity decision | Customer |

---

## Phase 2: Validate

### Step 1: Automated Lint

Deterministic checks — no LLM call needed.

| Check | Rule | Action on failure |
|-------|------|-------------------|
| Valid JSON | Parseable, required fields present | Reject — regenerate |
| Schema compliance | All fields present per output schema | Flag missing fields |
| Enum validation | artefact_type, artefact_class, source, likelihood_ratio, recommended_method, voi_quadrant, evidence_standard, staleness_tolerance all valid | Reject invalid enums |
| Type-B required | At least one artefact with artefact_class=Outcome per control | Flag — should have at least one discriminating artefact |
| Needs-judgement consistency | If control has load_bearing_judgement, needs_judgement should be true | Flag if inconsistent |
| VoI consistency | If blast_radius=Enterprise and nature=Manual, voi_quadrant should not be "Table Stakes" | Flag if inconsistent |
| Evidence-by-design consistency | If nature=Automated and source=Sensor, evidence_by_design should be true | Flag if inconsistent |
| Likelihood ratio consistency | Type-A artefacts should have Low or Medium likelihood_ratio, not High | Flag if inconsistent |
| Drift signal present | If needs_judgement=true, drift_signal should not be empty | Flag if missing |
| Drift conditions present | If needs_judgement=true, drift_conditions should not be empty | Flag if missing |
| Judgement rationale mandatory | judgement_rationale should always be populated (even when needs_judgement=false) | Flag if missing |

### Step 2: Deterministic Override Check

For fields with deterministic derivations, compare the LLM's output to the rule. If they differ and the LLM provides no rationale, flag for review.

| Field | Check |
|-------|-------|
| `needs_judgement` | LLM says false but load_bearing_judgement is non-null → flag |
| `evidence_standard` | LLM says Basic but blast_radius=Enterprise → flag |
| `staleness_tolerance` | LLM says High but info_distance=Remote and blast_radius=Site+ → flag |

---

## Phase 3: Human Review

The pipeline produces *suggested evidence patterns*, not L4 records. A customer's compliance team reviews and adapts:

### What the Review UI Shows

For each control and its evidence pattern:
- The control (title, description, what_it_checks)
- Suggested artefacts (Type-A and Type-B, with what_it_proves)
- Judgement guidance (whether needed, what to look for, the discriminating question)
- VoI classification (where this sits on the 2x2, and why)
- Any validation flags

### Customer Adaptation

Customers will:
- Add artefact types specific to their operations (their DMS, their sensor systems)
- Adjust recommended frequencies to their assessment calendar
- Override VoI classification based on their risk profile
- Decide whether to invest in judgement evidence or accept artefact-only coverage

---

## Design Constraints: The System Prompt

### Constraint 1: Evidence as Credence Change

> For each artefact, state what belief it changes. Not "a document is kept" but "a rational person would update their confidence that the control works based on seeing this artefact." An artefact that would look the same whether or not the obligation is met has zero evidential value.
>
> **Test**: would this artefact look the same if the control had failed? If yes, it has a likelihood ratio near 1 and low evidential value.

### Constraint 2: Type-B Priority

> Every control must have at least one Type-B (outcome) artefact — evidence that discriminates. Type-A (activity) artefacts are table stakes, not evidence. A filing cabinet full of Type-A artefacts and no Type-B is evidence theatre.
>
> **Test**: does this artefact prove the activity happened (Type-A) or that the control works (Type-B)?

### Constraint 3: Judgement Where Judgement Is Needed

> Controls with load-bearing judgement terms (adequate, competent, proportionate, sufficient, suitable) need judgement evidence — a named person's assessment. Artefacts alone cannot hold the load-bearing reality. The `drift_signal` must name how the measurement method can decouple from reality. The `drift_conditions` must describe when the control itself has drifted — the specific conditions under which a judge should find 'Drifted' rather than 'Still True'.
>
> Always explain *why* judgement is or isn't needed, even when artefacts alone are sufficient. A control where `needs_judgement=false` should have a `judgement_rationale` explaining why the outcome evidence discriminates without human assessment.
>
> **Test**: can a system check this control automatically? If not, a person needs to judge it. State what that person should look for.

### Constraint 4: Evidence-by-Design

> Prefer evidence that is a natural byproduct of control execution over evidence that requires a separate collection act. A backup log is evidence-by-design. A screenshot of the backup console is not.
>
> **Test**: does executing the control automatically produce this evidence record? If yes, mark evidence_by_design=true.

### Constraint 5: VoI Drives Effort

> Place each control on the VoI 2x2 (Expected Loss vs Measurement Cost). This determines where the customer should invest evidence effort. Do not treat all controls equally — a green dashboard of Table Stakes evidence is not assurance.
>
> The four quadrants:
> - **Table Stakes** (low loss, cheap): automate, don't count as work
> - **No-Brainer** (high loss, cheap): just collect it — discriminating and affordable
> - **Judgement** (high loss, expensive): fund calibrated people
> - **Waste** (low loss, expensive): stop

### Constraint 6: Domain-Specific Artefacts

> Name the specific artefact type for the domain, not a generic placeholder. A confined space control needs "gas test reading" not "test result." A training control needs "LMS completion record with assessment score" not "training record." The domain specificity is what makes the pattern actionable.

### Constraint 7: Basis Guidance Is Operational

> The basis_guidance field tells a real person what to look at when they exercise judgement. It must be specific enough to be useful, honest about what cannot be checked, and grounded in the control's actual domain.
>
> WRONG: "Review the documentation and assess compliance"
> RIGHT: "Review the risk assessment against the actual space: does it name THIS space? Does it address current atmospheric hazards and adjacent processes? Compare the hazard list to what you can observe on the ground."

### Constraint 8: Proportionality

> The evidence effort must be proportional to the control's risk profile. Do not suggest a comprehensive, multi-artefact evidence suite for a control with Local blast radius and no load-bearing judgement. A simple control with Direct info_distance and Local blast_radius may need only one Type-B artefact and no judgement. An Enterprise-blast-radius manual control with a load-bearing judgement term may need multiple artefacts, formal judgement, and a comprehensive evidence standard.
>
> **Test**: would a compliance officer look at this evidence pattern and say "this is disproportionate effort for this control"? If yes, simplify.

---

## Prompt Architecture

### Structure

```
+-----------------------------------------+
|  SYSTEM PROMPT                          |
|  - Role: evidence strategy architect    |
|  - Design constraints 1-8              |
|  - Output JSON schema                   |
|  - VoI 2x2 rules                       |
|  - Deterministic defaults table         |
|  - Few-shot examples                    |
|  - ~3,000 tokens                        |
+-----------------------------------------+
         |
         v
+-----------------------------------------+
|  USER PROMPT (per law)                  |
|  - Law outline (family, title)          |
|  - Controls array (numbered [1], [2])   |
|    with full JSON per control           |
|  - "Generate evidence patterns"         |
|  - ~1,500 + 200 per control tokens      |
+-----------------------------------------+
         |
         v
+-----------------------------------------+
|  ASSISTANT RESPONSE                     |
|  - Evidence patterns array (N)          |
|  - ~300 per control tokens              |
+-----------------------------------------+
```

### Token Budget

| Component | Estimated tokens | Notes |
|-----------|-----------------|-------|
| System prompt | ~3,000 | Constraints + schema + VoI rules + few-shot |
| Law outline | ~200 | Family, title — minimal |
| Controls (typical law, 3-12 controls) | 1,500-6,000 | ~500 tokens per control JSON |
| Output per call | 2,000-5,000 | ~300 tokens per evidence pattern |
| **Total per call** | **~7,000-14,000** | Fits comfortably in one call |

### Model Choice

- **Phase 1 (Generate)**: Gemini 2.5 Pro — needs domain reasoning for artefact types, basis guidance, drift signals
- **Phase 2 (Validate)**: Deterministic only — no LLM call needed

The evidence generation is simpler than controls generation (no consolidation, no structural analysis of provisions) so Flash is a candidate for cost reduction after the pilot validates Pro quality. Start with Pro, benchmark, then evaluate Flash.

---

## Post-Processing: LLM Output -> L4 Schema

### Artefact Template Record

| LLM field | -> Artefact field | Notes |
|-----------|------------------|-------|
| `title` | `title` | Artefact description |
| `artefact_type` | `artefact_type` | Enum value |
| `artefact_class` | `artefact_class` | Activity or Outcome |
| `source` | `source` | Where it comes from |
| -- | `control_id` | Linked to the source control |
| -- | `status` | Default: Current |
| -- | `uploaded_by_id` | Customer assigns |
| -- | `expiry_date` | Customer sets based on recommended_frequency |

### Judgement Template Record

| LLM field | -> Judgement field | Notes |
|-----------|-------------------|-------|
| `recommended_method` | `judgement_method` | Enum value |
| `basis_guidance` | Used as template for `basis` | The judge adapts this guidance |
| `discriminating_question` | Informs the `finding` decision | The question the judge answers |
| -- | `control_id` | Linked to the source control |
| -- | `judge_id` | Customer assigns |
| -- | `next_due` | Set from recommended_interval |

---

## Storage

### Staging Table (DuckDB)

Suggested evidence patterns live in a `suggested_evidence` staging table, mirroring `suggested_controls`.

| Column | Type | Purpose |
|--------|------|---------|
| `id` | VARCHAR | Primary key (UUID) |
| `law_name` | VARCHAR | Source law |
| `control_id` | VARCHAR | FK to suggested_controls.id |
| `control_title` | VARCHAR | Denormalised for readability |
| `evidence_json` | JSON | Full LLM output for this control's evidence pattern (artefacts + judgement + strategy) |
| `status` | VARCHAR | generated / validated / flagged / accepted / rejected / edited |
| `validation_flags` | JSON | Phase 2 lint results |
| `generation_model` | VARCHAR | Which model + prompt version |
| `generated_at` | TIMESTAMP | When |
| `base_hash` | VARCHAR | Hash of the generated evidence pattern — for three-way merge on regeneration |
| `customer_edits` | JSON | Before/after pairs from customer review — feedback signal for prompt improvement |

One row per control. The `evidence_json` contains the full artefacts + judgement + strategy structure. Strategy metadata (`voi_quadrant`, `evidence_standard`, etc.) lives here on the template — it is not pushed to operational L4 records.

### Relationship to Controls Staging

```
suggested_controls           suggested_evidence
------------------          ------------------
id (PK)          <------    control_id (FK)
law_name                    law_name
control_json                evidence_json
```

Evidence patterns are generated from controls. If a control is regenerated, its evidence pattern should be regenerated too — but customer edits must be preserved via three-way merge, identical to the Controls pipeline:

1. **Identify three versions:** `base` (original generation, stored as `base_hash` + full JSON), `theirs` (customer's current version, may have been edited), `ours` (newly generated version)
2. **Merge logic:** Only `theirs` changed → keep `theirs`. Only `ours` changed → apply if non-conflicting, flag if same field. Both changed same field → merge conflict, present both to customer.

The `control_id` link identifies which evidence patterns to regenerate. The `base_hash` enables the merge. The `customer_edits` column captures before/after pairs as a feedback signal for prompt improvement.

---

## Batch Strategy

### Canonical Generation

Evidence patterns are generated once per law, alongside or after controls. They are a reusable asset — the evidence template any customer starts from.

| Segment | Estimated laws | Controls per law | Calls | Notes |
|---------|---------------|-----------------|-------|-------|
| QQ applicable | ~220 | 3-12 | ~220 | One call per law |
| Full corpus | ~2,000 | 3-12 | ~2,000 | Phase 2 after QQ pilot |

**Total: ~220 calls for QQ pilot, ~2,000 for full corpus.** Each call is smaller than a controls generation call (controls are already generated, just being read). Cost is dominated by output tokens (~300 per evidence pattern).

### CLI Commands

```bash
# Generate evidence patterns for a single law
/usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713

# Dry run — show prompt without calling Gemini
/usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713 --dry-run

# Generate for all QQ applicable laws
/usr/bin/python3 scripts/compliance/generate_evidence.py --qq

# Generate for a family
/usr/bin/python3 scripts/compliance/generate_evidence.py --family "OH&S: Occupational / Personal Safety"

# Force regenerate
/usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713 --force
```

---

## Worked Example: Confined Spaces Regulations 1997

### Input: 3 Controls

```
Law: The Confined Spaces Regulations 1997
Family: OH&S: Occupational / Personal Safety

[1] "Before any confined space entry, a documented consideration of alternatives
    demonstrates that entry cannot reasonably be avoided"
    control_type: Preventive, nature: Manual, domain: Organisational
    info_distance: Adjacent, blast_radius: Local
    load_bearing_judgement: "reasonably practicable"

[2] "A risk assessment specific to the confined space, its current conditions,
    and the planned work is completed and available at the point of entry"
    control_type: Preventive, nature: Manual, domain: Organisational
    info_distance: Adjacent, blast_radius: Local
    load_bearing_judgement: "suitable and sufficient"

[3] "Emergency rescue arrangements for the confined space have been tested with
    the designated rescue team before the entry begins"
    control_type: Corrective, nature: Manual, domain: People
    info_distance: Direct, blast_radius: Local
    load_bearing_judgement: null
```

### Output: Evidence Patterns

```json
[
  {
    "control_index": 1,
    "artefacts": [
      {
        "title": "Alternatives consideration log naming specific methods evaluated and reasons for rejection",
        "artefact_type": "Risk Assessment",
        "artefact_class": "Activity",
        "what_it_proves": "Someone considered alternatives to entry. This is activity evidence — the log exists whether or not the alternatives were genuinely evaluated. A formulaic 'alternatives considered: N/A' entry has the same form as a substantive entry.",
        "source": "Upload",
        "likelihood_ratio": "Low",
        "recommended_frequency": "Per confined space entry",
        "evidence_by_design": false
      },
      {
        "title": "Record of work completed from outside the confined space (where alternatives were used)",
        "artefact_type": "Other",
        "artefact_class": "Outcome",
        "what_it_proves": "Entry was actually avoided where alternatives existed. This is outcome evidence — it looks different when the control works (some jobs done without entry) vs when it doesn't (every job requires entry regardless of alternatives).",
        "source": "System Generated",
        "likelihood_ratio": "High",
        "recommended_frequency": "Quarterly review of entry vs non-entry ratio",
        "evidence_by_design": true
      }
    ],
    "judgement": {
      "needs_judgement": true,
      "judgement_rationale": "Control encodes 'reasonably practicable' — a load-bearing judgement term. Whether alternatives were genuinely considered or the log was a formality cannot be determined from the artefact.",
      "recommended_method": "Document Review",
      "basis_guidance": "Review a sample of alternatives logs. For each: did the log name specific alternatives (CCTV, long-reach tools, remote sampling)? Were rejections substantiated with reasons, or formulaic? Compare the entry rate across similar spaces — a space that always requires entry despite available alternatives is a signal.",
      "discriminating_question": "Were alternatives to entry genuinely explored, or is the log a formality that rubber-stamps entry every time?",
      "drift_signal": "'Alternatives considered' was once anchored to 'an engineer evaluated remote methods.' It may have drifted to mean 'a box was ticked on the permit.' Check whether the log names specific methods with substantive rejection reasons.",
      "drift_conditions": "The control has drifted when: (a) the entry rate is not declining despite available alternatives becoming more practical (technology improvements, process redesign); (b) the alternatives log is formulaic across entries — identical wording, no space-specific reasoning; (c) entries that could demonstrably have been avoided are proceeding without challenge."
    },
    "strategy": {
      "voi_quadrant": "Judgement",
      "voi_rationale": "The alternatives consideration is the first gate — if it's a formality, people enter confined spaces unnecessarily, exposing them to the most lethal workplace hazard category. Blast_radius is Local but consequence is death. Measurement cost is moderate (document review by a competent person). VoI is high — the cost of an undetected rubber-stamp is catastrophic.",
      "evidence_standard": "Focused",
      "recommended_interval": "Per entry (artefacts) + quarterly sample review (judgement on 2-5 logs)",
      "sample_size_guidance": "Review 2-5 alternatives logs per quarter. Higher sample if entry rate is high relative to comparable operations.",
      "staleness_tolerance": "Low",
      "nature_strategy": "Manual control: sample-based testing. The quality of alternatives evaluation is a human judgement — no system can check whether the engineer genuinely considered remote methods."
    }
  },
  {
    "control_index": 2,
    "artefacts": [
      {
        "title": "Completed risk assessment form signed by the assessor, naming this space and this entry",
        "artefact_type": "Risk Assessment",
        "artefact_class": "Activity",
        "what_it_proves": "An assessment was performed. The form exists whether or not the assessment was adequate — a generic template reused across spaces produces the same artefact as a thorough space-specific assessment.",
        "source": "Upload",
        "likelihood_ratio": "Low",
        "recommended_frequency": "Per confined space entry",
        "evidence_by_design": false
      },
      {
        "title": "Atmospheric gas test reading taken at the point of entry immediately before the entry begins",
        "artefact_type": "Test Result",
        "artefact_class": "Outcome",
        "what_it_proves": "The atmosphere was tested and found safe (or unsafe — either way, the reading discriminates). A dangerous reading would prevent entry. The evidence looks different when the atmosphere is safe vs when it is not.",
        "source": "Sensor",
        "likelihood_ratio": "High",
        "recommended_frequency": "Per confined space entry — continuous monitoring during entry where practicable",
        "evidence_by_design": true
      }
    ],
    "judgement": {
      "needs_judgement": true,
      "judgement_rationale": "Control encodes 'suitable and sufficient' — the central enforcement term in confined space regulation. The risk assessment's adequacy to the actual hazard is a judgement that requires competence in confined space risks. The gas reading is strong outcome evidence but covers only one dimension of the assessment.",
      "recommended_method": "Document Review",
      "basis_guidance": "Review the risk assessment against the actual space: does it name THIS space by identifier? Does it reference current conditions (atmospheric hazards, adjacent processes, weather if relevant)? Does it identify hazards specific to this entry task? Compare the assessment's control measures to what is actually in place at the point of entry. A generic assessment reused verbatim across spaces is a signal.",
      "discriminating_question": "Is this risk assessment actually adequate to the hazards of this specific confined space and this specific entry?",
      "drift_signal": "'Risk assessment completed' was once anchored to 'the assessor visited the space and identified the actual hazards.' It may have drifted to mean 'the previous assessment was copied and the date was updated.' Check whether the assessment references space-specific conditions that change between entries.",
      "drift_conditions": "The control has drifted when: (a) assessments are being reused verbatim across different spaces or across entries where conditions have changed; (b) the hazard list does not reflect current adjacent processes or recent incidents; (c) atmospheric monitoring results are not referenced in the assessment despite being available; (d) the assessor has not visited the space for the current entry."
    },
    "strategy": {
      "voi_quadrant": "Judgement",
      "voi_rationale": "The risk assessment is the regulatory gateway — reg.4 prohibits entry without a suitable and sufficient assessment. If the assessment is inadequate, all downstream controls are calibrated to the wrong hazard. Consequence of failure is severe (confined space fatalities). Measurement cost is high (requires a person competent in confined space hazards). VoI justifies the cost — this is where calibrated people should spend their time.",
      "evidence_standard": "Focused",
      "recommended_interval": "Per entry (gas readings) + quarterly sample review (judgement on 2-5 assessments)",
      "sample_size_guidance": "Gas readings: 100% coverage (evidence-by-design from the sensor). Risk assessment judgement: 2-5 per quarter, biased toward assessments for high-hazard or unfamiliar spaces.",
      "staleness_tolerance": "Low",
      "nature_strategy": "Manual control: dual evidence strategy. The gas reading is automated outcome evidence (No-Brainer quadrant on its own). The assessment adequacy is pure judgement. Both needed — the gas reading checks one dimension, the judgement checks the whole assessment."
    }
  },
  {
    "control_index": 3,
    "artefacts": [
      {
        "title": "Rescue drill record for this space type within the last 12 months, showing drill date, participants, and performance",
        "artefact_type": "Training Record",
        "artefact_class": "Outcome",
        "what_it_proves": "The rescue team practised a rescue from this type of space and the drill was completed. The record shows performance metrics — a drill that failed or took too long looks different from a successful one. This is outcome evidence.",
        "source": "Upload",
        "likelihood_ratio": "High",
        "recommended_frequency": "Quarterly per space type + before first entry at unfamiliar spaces",
        "evidence_by_design": false
      },
      {
        "title": "Equipment functional test record for rescue equipment present at the point of entry",
        "artefact_type": "Inspection Report",
        "artefact_class": "Outcome",
        "what_it_proves": "The rescue equipment (breathing apparatus, winch, tripod, communications) was tested and found functional before the entry. Equipment that fails the test looks different from equipment that passes.",
        "source": "Upload",
        "likelihood_ratio": "High",
        "recommended_frequency": "Per confined space entry",
        "evidence_by_design": false
      },
      {
        "title": "Named rescue team members on the permit with current competence certification",
        "artefact_type": "Certificate",
        "artefact_class": "Activity",
        "what_it_proves": "Specific people were designated as the rescue team and their competence certificates are current. This is activity evidence — the names on the permit exist whether or not the people can actually perform a rescue.",
        "source": "Upload",
        "likelihood_ratio": "Medium",
        "recommended_frequency": "Per confined space entry (names) + annual (competence certification)",
                "evidence_by_design": false
      }
    ],
    "judgement": {
      "needs_judgement": false,
      "judgement_rationale": "No load-bearing judgement term in this control. Artefacts alone are sufficient because the outcome evidence is strongly discriminating: a rescue drill either succeeds within the target time or it doesn't, equipment either passes the functional test or it doesn't. These are binary, observable outcomes — no human assessment of 'adequacy' is needed. The drill record and equipment test speak for themselves.",
      "recommended_method": null,
      "basis_guidance": null,
      "discriminating_question": null,
      "drift_signal": null,
      "drift_conditions": "The control has drifted when: (a) the rescue team has not drilled for this space type within the required interval; (b) drill performance is declining (longer rescue times, failed extractions); (c) rescue equipment fails functional tests or is not present at the point of entry; (d) named rescue team members are no longer available or their competence has lapsed."
    },
    "strategy": {
      "voi_quadrant": "No-Brainer",
      "voi_rationale": "High expected loss (failure of rescue arrangements during a confined space emergency is fatal). Measurement cost is moderate — a full rescue drill requires personnel time and operational disruption, but this cost is non-discretionary (regulatory requirement) and the evidence is highly discriminating. The drill either worked or it didn't. The equipment either passed or it didn't. VoI is high because the evidence strongly discriminates at a cost the organisation must bear regardless.",
      "evidence_standard": "Focused",
      "recommended_interval": "Per entry (equipment test, named team) + quarterly (drills per space type)",
      "sample_size_guidance": "Equipment tests: 100% coverage (every entry). Drill records: all drills, not sampled. Competence certificates: verify currency at each entry.",
      "staleness_tolerance": "Medium",
      "nature_strategy": "Manual control but with strong outcome evidence. The drill record and equipment test are the discriminating artefacts. Unlike the risk assessment (Control 2), this control does not require a separate judgement about adequacy — the outcome evidence speaks for itself."
    }
  }
]
```

### What This Shows

1. **Control 1** (alternatives): Type-A artefact (the log) + Type-B artefact (entry vs non-entry ratio) + Judgement (was it genuine?). VoI = Judgement.
2. **Control 2** (risk assessment): Type-A artefact (the form) + Type-B artefact (gas reading) + Judgement (is the assessment adequate?). VoI = Judgement. The gas reading alone sits in No-Brainer but the assessment adequacy requires Judgement.
3. **Control 3** (rescue): Type-B artefacts dominate (drill record, equipment test). No judgement needed — the outcome evidence discriminates. VoI = No-Brainer. The `judgement_rationale` explicitly explains why artefacts suffice (binary outcomes, not judgement-laden).

All three controls have `drift_conditions` — bridging to the L4 Gaps entity by telling the assessor when to find 'Drifted' rather than 'Still True'.

The pattern illustrates the core insight: **controls 1 and 2 have load-bearing judgement terms and need human assessment. Control 3 has verifiable outcomes and artefacts suffice.** The VoI classification tells the customer where to invest calibrated people.

---

## Quality Signals

### Red Flags (evidence pattern needs rework)

- **No Type-B artefact**: every control should have at least one discriminating artefact
- **Generic artefact types**: "document" or "record" instead of the specific type for the domain
- **Judgement needed but not flagged**: control has load_bearing_judgement but needs_judgement=false
- **VoI mismatch**: Enterprise blast_radius manual control classified as Table Stakes
- **Empty basis_guidance**: judgement needed but no guidance on what to look for
- **Missing drift_signal**: judgement needed but no description of how the measurement method could decouple

### Green Flags (evidence pattern is well-formed)

- Type-B artefact names a specific, domain-relevant discriminating test
- Judgement guidance is operational — tells the person what to look at, not "review for compliance"
- VoI classification is consistent with control properties
- Drift signal names a concrete mechanism of proxy-to-referent decoupling
- Evidence-by-design artefacts identified where the control nature supports them

---

## Differences from Controls Pipeline

| Aspect | Controls | Evidence |
|--------|----------|---------|
| Input | Provisions from Postgres | Controls from DuckDB staging |
| Output per law | 3-15 controls | 3-15 evidence patterns (1:1 with controls) |
| Consolidation | Yes (Phase 3, HDBSCAN clustering) | No (patterns are per-control) |
| Policy predicate | Yes (law-level) | No (evidence is per-control, not per-law) |
| Provision index resolution | Yes (LLM returns indices, pipeline resolves) | No (controls are referenced by control_index) |
| Deterministic derivations | Few (most fields are LLM-estimated) | Several (needs_judgement, evidence_standard, staleness_tolerance) |
| Validation | Deontic lint + referent check + judgement check | Schema + enum + consistency checks |
| Chunking | Yes (large laws >20K tokens) | No (controls are pre-consolidated, fit in one call) |

---

## Open Questions

1. **Cross-control evidence reuse.** One penetration test report may serve as evidence for multiple controls. Should the pipeline suggest evidence reuse — "this artefact also serves controls [2, 4]"? Or leave reuse to the customer? For v0.1, leave to customer. The pattern is per-control; the customer's operational context determines reuse.

2. **Quantitative VoI.** The current design classifies VoI into four quadrants with text rationale. Should the pipeline also estimate numeric Expected Loss from control properties? The formula (`Uncertainty(info_distance, staleness) x Consequence(blast_radius)`) is well-defined. For v0.1, stick with quadrant classification — numeric EL requires customer-specific calibration of the staleness input.

3. **Flash vs Pro.** Evidence generation is simpler than controls generation (no structural analysis, no consolidation, no provision-level reasoning). Gemini Flash may be sufficient and significantly cheaper. The pilot should benchmark both models on the 5 reference laws and compare quality.

4. **Predicate-level evidence.** The policy predicate sits above specific controls. Should it get an evidence pattern too? The policy predicate is a goal-setting statement ("Work does not harm the people who do it"). Its evidence is the aggregate of all control-level evidence — the system can compute coverage from the control-level patterns. No per-predicate evidence generation needed.

5. **Artefact-to-Judgement link.** The L4 schema has a judgement-artefact join table (which artefacts the judge relied on). Should the evidence pattern suggest which artefacts feed into the judgement? For v0.1, the basis_guidance implicitly covers this — it tells the judge what to look at, which includes the artefacts.

---

## Related Documents

- [`COMPLIANCE-CONTROLS.md`](.claude/plans/compliance/COMPLIANCE-CONTROLS.md) -- L3 Controls pipeline design
- [`EVIDENCE-SCHEMA.md`](Desktop/sertantai-legal/docs/compliance/l4-evidence/EVIDENCE-SCHEMA.md) -- canonical L4 entity model
- [`DEFINITION-OF-EVIDENCE.md`](Desktop/sertantai-legal/docs/compliance/l4-evidence/DEFINITION-OF-EVIDENCE.md) -- evidence as credence change
- [`VALUE-OF-INFORMATION.md`](Desktop/sertantai-legal/docs/compliance/l4-evidence/VALUE-OF-INFORMATION.md) -- VoI framework, the 2x2
- [`LEGIBLE-vs-LOAD-BEARING.md`](Desktop/sertantai-legal/docs/compliance/l4-evidence/LEGIBLE-vs-LOAD-BEARING.md) -- the operationalisation paradox
- [`EVIDENCE-VAULT-PATTERNS.md`](Desktop/sertantai-legal/docs/compliance/l4-evidence/EVIDENCE-VAULT-PATTERNS.md) -- evidence strategy from control properties
- [`EVIDENCE-CALIBRATION.md`](Desktop/sertantai-legal/docs/compliance/l4-evidence/EVIDENCE-CALIBRATION.md) -- judgement vs calibration vs drift
- [`L4-EVIDENCE.md`](Desktop/sertantai-legal/docs/qq/L4-EVIDENCE.md) -- QQ brief for evidence
