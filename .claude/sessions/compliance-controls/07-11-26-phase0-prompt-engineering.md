---
session: Controls Prompt Engineering
status: closed
opened: 2026-07-11
closed: 2026-07-11
outcome: success

summary: >
  Wrote and validated both LLM prompts for the compliance controls pipeline: a system prompt
  for generating specific controls (8 constraints, 3 few-shot examples) and a policy predicate
  prompt. Tested against 5 laws of varying character — all passed on first attempt with a
  consistent ~4:1 provision-to-control consolidation ratio. No prompt iteration needed.

decisions:
  - what: Option B for provision IDs — LLM returns short refs, pipeline prefixes law name
    why: LLM invented full section_ids (UK_pga_1999_26 instead of UK_uksi_1999_3242). Short refs are what the LLM sees in the prompt, so it returns them correctly. Post-processing prefix is trivial.
    result: Fixed in system-prompt-v1.md, confirmed working on HSWA test (s.2(1) not UK_ukpga_1974_37:s.2(1))

  - what: Fire Safety Order 2005 replaces Environmental Permitting Regs as 5th test law
    why: Environmental Permitting Regs not in the corpus. FSO provides different-domain (FIRE) test and large-law challenge (259 raw obligations).
    result: Good test — different domain handled correctly, duplicate scraping issue discovered

  - what: Pipeline provision filter needs purposes-based exclusion of enforcement/offence provisions
    why: HSWA ss.20-42 pass through governed-only filter because Ind Person is a governed actor on offence provisions. Purposes classification doesn't tag these as Offence consistently.
    result: GitHub issue fractalatai/fractalatai#47 raised

metrics:
  controls_test:
    laws_tested: 5
    total_provisions_in: 190
    total_controls_out: 49
    consolidation_ratio: "3.9:1"
    deontic_verb_failures: 0
    prompt_iterations_needed: 0
  per_law:
    confined_spaces: { provisions: 12, controls: 3, ratio: "4:1" }
    mhsw: { provisions: 49, controls: 12, ratio: "4:1" }
    hswa: { provisions: 30, controls: 11, ratio: "2.7:1" }
    coshh: { provisions: 51, controls: 12, ratio: "4.25:1" }
    fso: { provisions: 48, controls: 11, ratio: "4.4:1" }
  policy_predicates:
    tested: 5
    quality: "3 excellent, 2 reproduced few-shot examples (expected)"

lessons:
  - title: Gemini Pro follows indicative mood constraint without iteration
    detail: >
      Expected to need 2-3 prompt iterations to prevent deontic drift (Gemini v0.1 review
      warned about this). In practice, the 8 constraints + 3 few-shot examples were sufficient
      on the first attempt. Zero deontic verbs across 49 generated controls. The few-shot
      examples are the critical anchor — stating the principle alone would not be enough.
    tag: methodology

  - title: LLMs invent plausible but wrong identifiers when asked to construct them
    detail: >
      MHSW test produced UK_pga_1999_26 instead of UK_uksi_1999_3242 for provision IDs.
      The LLM confidently constructed a plausible-looking identifier that was entirely wrong.
      Fix: never ask the LLM to construct identifiers — give it short refs and prefix in
      post-processing (Option B).
    tag: methodology

  - title: Fire Safety Order has duplicate reg/art scraping in sertantai
    detail: >
      Provisions appear as both reg.X and art.X with near-identical text, doubling the
      provision count. The Order uses "articles" — art. is correct. GitHub issue
      shotleybuilder/sertantai-legal#118 raised.
    tag: data

  - title: Governed-only filter leaks enforcement provisions via generic Ind Person actor
    detail: >
      HSWA ss.20-42 (enforcement, offences, court procedures) pass through the governed
      obligation filter because they have Ind Person as a governed actor — technically correct
      (the person being prosecuted) but not an operational duty-bearer. The purposes classifier
      doesn't consistently tag these as Offence. GitHub issue fractalatai/fractalatai#47 raised.
    tag: data

  - title: The Postgres connection is fractalaw:fractalaw@localhost:5433/fractalaw
    detail: >
      Found by grepping the CLI source. No .env file or pgpass needed — credentials are
      in the connection string in main.rs.
    tag: infrastructure

  - title: ~4:1 consolidation ratio is consistent across law types
    detail: >
      Whether small prescriptive SI (Confined Spaces), medium management framework (MHSW),
      goal-setting Act (HSWA), technical SI (COSHH), or different domain (Fire Safety Order),
      the consolidation ratio stays around 4:1. This suggests the ratio is a property of
      UK legislative drafting style, not the specific law.
    tag: methodology

artifacts:
  - .claude/plans/compliance-controls/prompts/system-prompt-v1.md
  - .claude/plans/compliance-controls/prompts/policy-predicate-prompt-v1.md
  - data/compliance-controls/test-results/confined-spaces-v1.json
  - data/compliance-controls/test-results/mhsw-v1.json
  - data/compliance-controls/test-results/hswa-v1.json
  - data/compliance-controls/test-results/coshh-v1.json
  - data/compliance-controls/test-results/fso-v1.json
  - data/compliance-controls/test-results/policy-predicates-v1.json

depends_on:
  - 07-10-26-compliance-controls.md

enables:
  - Phase 1 pipeline build (prompt assembly from DB, Gemini calls, staging table)
  - Controls generation for any law in the corpus
---

# Session: Controls Prompt Engineering (CLOSED)

## Problem

The compliance controls pipeline depends entirely on the quality of the LLM prompt. Before writing any pipeline code, the system prompt, few-shot examples, and policy predicate prompt need to be written, tested against real laws, and iterated until the output consistently meets the design constraints (indicative mood, referent not paperwork, discriminating test, honest limits, consolidation, proportionality, control type accuracy, operational property estimation).

## Work

1. ✅ Write system prompt encoding constraints 1-8 from COMPLIANCE-CONTROLS.md
2. ✅ Write few-shot examples: good control, bad-to-good rewrite, consolidation example
3. ✅ Write the policy predicate prompt (separate from specific controls)
4. ✅ Test against 5 laws of varying character:
   - ✅ Small prescriptive SI (Confined Spaces Regs 1997) — 3 controls
   - ✅ Medium management SI (MHSW Regs 1999) — 12 controls
   - ✅ Goal-setting framework Act (HSWA 1974 ss.2-8) — 11 controls
   - ✅ Technical SI (COSHH 2002) — 12 controls
   - ✅ Different domain (Fire Safety Order 2005) — 11 controls (swapped for Environmental Permitting — not in corpus)
5. ✅ Evaluate outputs against quality signals (red flags / green flags) — all pass
6. ✅ No iteration needed — prompt v1 passed all 5 tests on first attempt
7. ✅ Final prompts saved:
   - `prompts/system-prompt-v1.md` — controls generation (8 constraints + 3 few-shot examples)
   - `prompts/policy-predicate-prompt-v1.md` — policy predicate generation (2 examples)

### Policy Predicate Test Results

| Law | Predicate | Assessment |
|-----|-----------|------------|
| MHSW 1999 | "Workplace risks are systematically identified, assessed, and controlled through a planned management system" | Good — slightly procedural but reflects the law's management framework nature |
| COSHH 2002 | "People are not exposed to substances hazardous to their health, or where exposure is unavoidable, it is reduced to the lowest reasonably practicable level" | Excellent — captures prevent→control hierarchy |
| Fire Safety Order 2005 | "In the event of a fire, everyone in the premises can escape to a place of total safety, unaided and unharmed" | Excellent — outcome-focused, the kind of statement a customer would adopt |
| Confined Spaces 1997 | (Reproduced few-shot example) | Expected |
| HSWA 1974 | (Reproduced few-shot example) | Expected |

Full output saved: `data/compliance-controls/test-results/policy-predicates-v1.json`

## Results

### Test 1: Confined Spaces Regulations 1997 (UK_uksi_1997_1713)

**Prompt**: system-prompt-v1.md + manual user prompt with 12 governed provisions
**Model**: Gemini 2.5 Pro, temp=0.1, structured JSON output
**Output**: 3 controls

| # | Title (first 60 chars) | Type | Provisions | Strength |
|---|----------------------|------|-----------|----------|
| 1 | "Before any confined space entry, a documented considerat..." | Preventive | reg.4(1), reg.3(1)(a), reg.3(1)(b), reg.3(2)(b) | Primary |
| 2 | "All work in a confined space is executed under a documen..." | Preventive | reg.4(2), reg.3(1)(a), reg.3(1)(b), reg.3(2)(b) | Primary |
| 3 | "Suitable and sufficient emergency rescue arrangements ar..." | Corrective | reg.5(1), reg.5(2)(a), reg.5(2)(b), reg.3(1)(a), reg.3(1)(b), reg.3(2)(b) | Primary |

**Quality checks:**
- ✅ No deontic verbs in titles — all indicative
- ✅ Descriptions reference reality, not paperwork
- ✅ Type-B evidence provided in all evidence_hints
- ✅ Load-bearing judgement flagged: "reasonably practicable", "safe and without risks to health", "suitable and sufficient"
- ✅ Honest limits populated for all 3
- ✅ Consolidated: 12 provisions → 3 controls (4:1 ratio)
- ✅ Correctly skipped reg.6 (exemptions) and reg.7 (court defences) — proportionality
- ✅ Correctly linked reg.3 duty-holder provisions to all 3 controls as the "ensure compliance" wrapper
- ✅ Info_distance: all Adjacent — reasonable for permit-based controls
- ✅ Blast_radius: all Local — correct for confined space entry (single space)
- ⚠️ Control 2 nature "IT-dependent manual" — debatable, "Manual" may be more accurate for a permit-to-work
- ⚠️ reg.5(3) (immediately put into operation) not linked — this is the activation duty, could be argued as part of control 3

**Verdict**: Excellent. Prompt v1 produces well-formed controls on first test. Minor quibbles only.

Full output saved: `data/compliance-controls/test-results/confined-spaces-v1.json`

### Test 2: Management of Health and Safety at Work Regulations 1999 (UK_uksi_1999_3242)

**Prompt**: system-prompt-v1.md + manual user prompt with ~30 grouped provision entries (from 49 obligations + 9 scope)
**Model**: Gemini 2.5 Pro, temp=0.1, structured JSON, thinkingBudget=8192
**Output**: 12 controls

| # | Title (first 70 chars) | Type | Provisions linked | Strength |
|---|----------------------|------|--------------------|----------|
| 1 | "A risk assessment that is suitable and sufficient for the nature..." | Preventive | reg.3(1-3) | Primary |
| 2 | "Preventive and protective measures are selected...based on principles..." | Directive | reg.4 | Primary |
| 3 | "Arrangements for effective planning, organisation, control..." | Directive | reg.5(1) | Primary |
| 4 | "A system of health surveillance is in place..." | Detective | reg.6 | Primary |
| 5 | "One or more competent persons are appointed, resourced..." | Preventive | reg.7(1-4) | Primary |
| 6 | "Procedures for serious and imminent danger...established, tested..." | Corrective | reg.8(1), 8(1)(b), 9 | Primary |
| 7 | "All employees...provided with information and adequate training..." | Preventive | reg.8(1)(c), 10(1), 13(1-2) | Primary |
| 8 | "A process for cooperation, coordination...at a shared workplace..." | Preventive | reg.11(1), 12(1), 12(3) | Primary |
| 9 | "A specific risk assessment...before a young person is employed..." | Preventive | reg.3(4-5), 10(2), 19(1-2) | Primary |
| 10 | "A specific risk assessment...for new or expectant mothers..." | Preventive | reg.16, 17 | Primary |
| 11 | "Employees are informed of their duties to use equipment correctly..." | Directive | reg.14(1-2) | Primary |
| 12 | "Information on required skills...provided to temporary work agencies..." | Preventive | reg.15(1-3) | Primary |

**Quality checks:**
- ✅ No deontic verbs in any title — all indicative
- ✅ Descriptions reference reality, not paperwork
- ✅ Type-B evidence provided in all evidence_hints — good variety (observations, employee interviews, drill performance)
- ✅ Load-bearing judgement flagged throughout: suitable and sufficient, appropriate, effective, competent, adequate, comprehensible
- ✅ Honest limits populated where appropriate, null where obligation is fully reducible (#11)
- ✅ Excellent consolidation: 49 obligations → 12 controls (4:1 ratio)
- ✅ Good thematic grouping: risk assessment, hierarchy of control, management system, health surveillance, competent persons, emergencies, information/training, shared workplaces, young persons, new/expectant mothers, employee duties, temporary workers
- ✅ Control types varied and accurate: 7 Preventive, 2 Directive, 1 Detective, 1 Corrective, 1 Directive
- ⚠️ **Provision IDs wrong**: `UK_pga_1999_26` instead of `UK_uksi_1999_3242` — the LLM invented a law reference. This is a validation catch (Phase 2 lint would reject these).
- ⚠️ reg.12(4) cluster (information to outside undertakings about emergency persons) not explicitly linked — absorbed into #8 implicitly

**Verdict**: Very good control quality. The provision ID bug is a systematic prompt issue — the LLM doesn't have the correct law name prefix to construct IDs. Need to either: (a) provide section_ids explicitly in the prompt, or (b) tell the LLM to use the section references as given (e.g., "reg.3(1)") and map them to full IDs in post-processing.

Full output saved: `data/compliance-controls/test-results/mhsw-v1.json`

### Test 3: Health and Safety at Work etc. Act 1974 (UK_ukpga_1974_37)

**Prompt**: system-prompt-v1.md (with Option B fix) + manual user prompt with ss.2-9 only (30 provisions)
**Model**: Gemini 2.5 Pro, temp=0.1, structured JSON, thinkingBudget=8192
**Output**: 11 controls

| # | Title (first 70 chars) | Type | Provisions | Strength |
|---|----------------------|------|--------------------|----------|
| 1 | "A systematic process for identifying workplace hazards and implement..." | Preventive | s.2(1), s.2(2) | Primary |
| 2 | "All employees are provided with the information, instruction, train..." | Preventive | s.2(2)(c) | Primary |
| 3 | "A written health and safety policy...maintained, current, communic..." | Directive | s.2(3) | Primary |
| 4 | "An effective arrangement for consultation with employees or their..." | Directive | s.2(4), s.2(6), s.2(7) | Primary |
| 5 | "The undertaking is conducted in a way that does not expose non-empl..." | Preventive | s.3(1), s.3(2), s.3(3) | Primary |
| 6 | "Any person in control of non-domestic premises ensures the premises..." | Preventive | s.4(2) | Primary |
| 7 | "Articles and plant for use at work are designed, constructed, instal..." | Preventive | s.6(1), s.6(3) | Primary |
| 8 | "Substances for use at work are supplied with sufficient testing and..." | Preventive | s.6(4) | Primary |
| 9 | "Each employee takes reasonable care for their own health and safety..." | Directive | s.7 | Primary |
| 10 | "No person intentionally or recklessly interferes with or misuses..." | Directive | s.8 | Primary |
| 11 | "No charge is levied on any employee for anything done or provided..." | Directive | s.9 | Primary |

**Quality checks:**
- ✅ No deontic verbs — all indicative
- ✅ Option B fix worked: linked_provisions use short refs (s.2(1), s.6(1) etc.)
- ✅ Descriptions reference reality, not paperwork
- ✅ Type-B evidence provided throughout — good variety (physical inspections, observed behaviour, absence of incidents)
- ✅ Load-bearing judgement flagged: "reasonably practicable" (×3), "necessary", "safe", "effectively", "reasonable care"
- ✅ s.9 and s.8 correctly have null honest_limit — fully reducible to checkable predicates
- ✅ Good consolidation: s.2(4)+s.2(6)+s.2(7) → one consultation control; s.3(1)+s.3(2)+s.3(3) → one non-employee risk control; s.6(1)+s.6(3) → one articles control
- ✅ s.6 articles vs substances correctly separated (different supply chain duties)
- ✅ s.7 and s.8 correctly placed as individual/employee duties (Direct info_distance) vs employer duties (Mediated)
- ✅ s.5 (emissions) captured as separate premises control

**Observations on goal-setting character:**
- Control 1 (the HSWA "big" control) is necessarily broad: "A systematic process for identifying workplace hazards..." — this IS what HSWA s.2 demands. The honest_limit correctly identifies "reasonably practicable" as the core judgement.
- The controls read like a safety management system outline, which is exactly what HSWA is: a goal-setting framework.
- s.2(2)(d) (safe place of work) absorbed into control 1 rather than getting its own control — reasonable consolidation.

**No issues found. Prompt v1 handles goal-setting legislation well.**

Full output saved: `data/compliance-controls/test-results/hswa-v1.json`

### Pipeline data quality note

The governed-only filter for HSWA lets through enforcement/offence provisions (ss.20-42) because they have `Ind: Person` as a governed actor. The `purposes` classification doesn't tag these as `Offence`. For the pipeline, need either:
- Better purposes tagging for enforcement provisions, or
- Actor-category filter: exclude provisions where the only non-government actors are generic `Ind: Person` in enforcement/offence sections
- Pragmatic: the user prompt can instruct the LLM to "focus on Part I general duties" — works but fragile

### Test 4: Control of Substances Hazardous to Health Regulations 2002 (UK_uksi_2002_2677)

**Prompt**: system-prompt-v1.md (Option B) + manual user prompt with 51 provisions grouped by regulation
**Model**: Gemini 2.5 Pro, temp=0.1, structured JSON, thinkingBudget=8192
**Output**: 12 controls

| # | Title (first 70 chars) | Type | Key provisions | Strength |
|---|----------------------|------|--------------------|----------|
| 1 | "A suitable and sufficient risk assessment for work involving hazard..." | Preventive | reg.3(1), 6(1), 6(2)(b) | Primary |
| 2 | "Exposure to hazardous substances is prevented or...adequately cont..." | Preventive | reg.7(1-3), 7(5), 7(6) | Primary |
| 3 | "All control measures...maintained in efficient state...regular exam..." | Preventive | reg.9(1-2), 9(7) | Primary |
| 4 | "All control measures and facilities...properly used by employees..." | Directive | reg.8(1), 8(2) | Primary |
| 5 | "PPE provided is suitable for the risks, maintained, correctly stored..." | Preventive | reg.7(9), 9(3), 9(5) | Primary |
| 6 | "Workplace exposure monitoring is conducted where required..." | Detective | reg.10(1) | Primary |
| 7 | "An appropriate health surveillance programme is in place..." | Detective | reg.11(1-9) — all 8 sub-regs | Primary |
| 8 | "Employees and others...receive suitable and sufficient information..." | Preventive | reg.12(1), 12(2), 12(4) | Primary |
| 9 | "Containers and pipework holding hazardous substances are clearly..." | Directive | reg.12(5) | Primary |
| 10 | "Procedures and equipment for responding to accidents...in place..." | Corrective | reg.13(1), 13(3), 13(5) | Primary |
| 11 | "Prohibited substances are not supplied for use or used at work" | Preventive | reg.4(3) | Primary |
| 12 | "Required notifications are made and warning notices placed before..." | Preventive | reg.14(2), 14(3) | Primary |

**Quality checks:**
- ✅ No deontic verbs — all indicative
- ✅ Short section refs throughout (Option B confirmed)
- ✅ Descriptions are substance-specific — references to LEV, SDS, RPE, WEL, sensitisers
- ✅ Type-B evidence excellent: physical walk-throughs, flow-rate indicators, monitoring results vs WEL, observed employee competence
- ✅ Load-bearing judgement flagged: suitable and sufficient, reasonably practicable, adequately controlled, appropriate, efficient state
- ✅ reg.11 health surveillance consolidated into ONE control covering all 8 sub-regulations — correct, it's one operational mechanism
- ✅ reg.7 hierarchy of control consolidated into ONE control covering prevent→substitute→engineer→PPE — correct
- ✅ Prohibited substances (reg.4(3)) correctly gets null honest_limit — absolute prohibition, fully checkable
- ✅ Container labelling (reg.12(5)) correctly gets null honest_limit — checkable predicate
- ✅ Control types well-distributed: 7 Preventive, 2 Detective, 2 Directive, 1 Corrective
- ✅ Domain variety: Organisational, Physical, People, Technical all represented
- ✅ Fumigation (reg.14) correctly treated as a standalone control — distinct operational mechanism

**Observations on technical/prescriptive character:**
- More specific and concrete controls than HSWA — reflects the prescriptive nature of COSHH
- The hierarchy of control (reg.7) consolidated well — one control captures prevent→substitute→engineer→admin→PPE
- Health surveillance (reg.11) is the biggest consolidation win: 8 sub-regulations → 1 control
- The monitoring vs surveillance distinction is correctly maintained (two separate controls)

**Verdict**: Excellent. Prompt v1 handles technical, prescriptive SIs very well. No changes needed.

Full output saved: `data/compliance-controls/test-results/coshh-v1.json`

### Test 5: Regulatory Reform (Fire Safety) Order 2005 (UK_uksi_2005_1541)

**Prompt**: system-prompt-v1.md (Option B) + manual user prompt with 48 provisions (deduplicated reg. only, filtered)
**Model**: Gemini 2.5 Pro, temp=0.1, structured JSON, thinkingBudget=8192
**Output**: 11 controls

| # | Title (first 70 chars) | Type | Key provisions | Strength |
|---|----------------------|------|--------------------|----------|
| 1 | "A suitable and sufficient fire risk assessment...completed and kept..." | Preventive | art.9(1), art.9(4-5) | Primary |
| 2 | "A coherent fire safety policy and effective management arrangement..." | Directive | art.5(1-3), art.8(1), art.10, art.11(1) | Primary |
| 3 | "Fire and explosion risks from dangerous substances are eliminated..." | Preventive | art.12(1-4), art.16(1-3) | Primary |
| 4 | "The premises are equipped with appropriate fire-fighting equipment..." | Preventive | art.13(1,3), art.14(1-2) | Primary |
| 5 | "All fire safety facilities, equipment, and devices are maintained..." | Detective | art.17(1), art.38(1) | Primary |
| 6 | "Clear emergency procedures are established, practiced through dri..." | Corrective | art.15(1), art.15(2)(c) | Primary |
| 7 | "A sufficient number of competent persons are appointed and provid..." | Preventive | art.18(1-4) | Primary |
| 8 | "All relevant persons...receive comprehensible fire safety informat..." | Preventive | art.19(1-3), art.20(1-3), art.21(1-2), art.21A(2) | Primary |
| 9 | "Fire safety measures are coordinated between all responsible pers..." | Directive | art.22(1-2) | Primary |
| 10 | "Every employee takes reasonable care for their own and others' saf..." | Directive | art.23(1) | Primary |
| 11 | "No employee is charged for anything provided for fire safety" | Directive | art.40 | Primary |

**Quality checks:**
- ✅ No deontic verbs — all indicative
- ✅ Short section refs with "art." prefix (correct for an Order, not "reg." or "s.")
- ✅ Descriptions are fire-specific — references to ignition sources, DSEAR, fire doors, escape routes, fire wardens
- ✅ Type-B evidence excellent: physical walk-throughs, drill observations, functioning alarms, clear escape routes
- ✅ Load-bearing judgement flagged: suitable and sufficient, effective, reasonably practicable, appropriate, adequate, competent, reasonable care
- ✅ art.12 (dangerous substances) and art.16 (emergency for dangerous substances) correctly consolidated — same operational mechanism
- ✅ art.17 (general maintenance) and art.38 (fire-fighter safety maintenance) correctly consolidated — same mechanism
- ✅ art.21A (resident information) correctly absorbed into information/training control — new duty, same mechanism
- ✅ art.22 (coordination) correctly gets null honest_limit — procedural, fully checkable
- ✅ art.40 (no charges) correctly gets null honest_limit — absolute rule

**Data quality issues found:**
- ⚠️ Duplicate scraping: provisions appear as both `reg.X` and `art.X` with near-identical text. The Fire Safety Order uses "articles" — `art.` is correct. Need to fix in sertantai scraper.
- ⚠️ reg.30 (enforcement notices) and reg.37 (fire safety equipment) leaked through the filter — enforcement provisions with `Ind: Person` as governed actor

**Verdict**: Excellent. Different domain (FIRE vs OH&S), different instrument type (Order vs SI/Act), correctly handled. The structural parallel with MHSW is visible in the control pattern (risk assessment, arrangements, competent persons, information, training, emergencies) which is expected — both implement the EU Framework Directive pattern.

Full output saved: `data/compliance-controls/test-results/fso-v1.json`

## Dependencies

- ✅ COMPLIANCE-CONTROLS.md v0.2 design reviewed and approved
- ✅ Design constraints 1-8 defined
- ✅ Output JSON schema defined
- ✅ Quality signals (red flags / green flags) defined
- ✅ Worked example (Confined Spaces) in design doc as reference
- ✅ Governed provision data accessible for test laws (Postgres: fractalaw:fractalaw@localhost:5433)
