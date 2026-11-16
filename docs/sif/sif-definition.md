# SIF definitions and coding rules

Status: v1.0, 2026-11-16. This document states the definitions; the reasoning behind them is in [sif-definition-discussion.md](sif-definition-discussion.md).

## Contents

- [1. Scope](#1-scope)
- [2. Terms](#2-terms)
- [3. The fractalaw definition](#3-the-fractalaw-definition)
- [4. Other published definitions](#4-other-published-definitions)
- [5. Crosswalk](#5-crosswalk)
- [6. Coding rules: determining PSIF](#6-coding-rules-determining-psif)
- [Annex A. Mechanisms](#annex-a-mechanisms)
- [Annex B. Band table](#annex-b-band-table)
- [References](#references)

## 1. Scope

- **What it covers.** Serious injury and fatality (SIF) potential in workplace events that involve the transfer of energy to, or toward, a person.
- **Unit of assessment.** One reported event, assessed per energy source.
- **Out of scope.** Chronic exposure and occupational illness; medical events without an energy transfer (e.g. cardiac arrest); property and environmental harm.

## 2. Terms

### 2.1 Energy and its envelope

| Term | Definition |
|---|---|
| **Energy source** | Anything that holds, stores or can release energy: gravity (height, suspended loads), motion, mechanical, electrical, pressure, thermal, chemical, radiation, sound, biological |
| **Energy present** | Energy that is held, stored or available to be released. This includes the supply behind an isolator and the fuel in a parked vehicle. Only *elimination* removes it |
| **Elimination** | Removing the energy from the source: supply disconnected and removed, system drained and vented, stored energy discharged, load landed |
| **Envelope** | The designed bounds on an energy, in three dimensions: magnitude (M), how much may be present; length (L), where it may be and how far it must be kept from people; and time (T), when it may be present. For contained energy, the envelope is the containment and its designed path. For an intentional release (a blast, live firing, a relief discharge), it is the designed release zone |
| **Danger zone** | An area within reach of an energy that the plan keeps clear of people while the energy is live: an exclusion zone, a segregated route, a drop zone, or a machine's hazard zone behind its guards (ISO 12100 3.11) |
| **Containment** | What holds energy in its intended state (vessel, pipe, cable insulation). It is part of the source, not a control |
| **Inherent feature** | A property of the source's own materials, quantity, operating conditions or physics. It is in effect throughout normal operation without detection, actuation, power or human action, and can be lost only by changing the source (e.g. a reduced inventory, pump dead-head pressure, a negative temperature coefficient). It is part of the source |

### 2.2 Exposure and events

| Term | Definition |
|---|---|
| **Exposure** | A condition: energy present and a person within its reach, together in time and space, **assessed without regard to controls**. Reach is judged as if guards and barriers were absent, so an operator using a guarded machine is exposed for the whole time of use. As a quantity, exposure is measured in context units (person-hours, lifts, entries) |
| **Exposure stratum** | A part of total exposure with one control state: e.g. guards in place (the guard in continuous demand), isolated under LOTO, or guard open while energised. Risk sums over strata |
| **Excursion** | Energy going beyond its envelope: outside its space (L), beyond its magnitude (M), or present where its intended state is zero (T) |
| **Incursion** | A person entering a danger zone while the energy is live (e.g. opening a guard on a running machine). The entry is an event; remaining there counts towards exposure, in the stratum where that control is absent. Entry under isolation is not an incursion |
| **Event** | Something that happens at a moment. It is one of three things: **(a)** the energy changes state and goes outside its envelope in space; **(b)** a person makes an incursion, or the energy changes state while a person is exposed; or **(c)** the energy builds beyond its magnitude, or appears where its intended state is zero |
| **Near miss** | An event in which nobody was hurt. Exposure alone is not a near miss |
| **Cycle** | One energy source taken once through event, potential and control state. Cycles are bounded per source by default |
| **Bridge condition** | A control failure or event in one cycle that sets the starting state of another, often by energy conversion (fuel → fire, motion → heat) |
| **Evidential cycle** | A cycle the facts show happened |
| **Counterfactual cycle** | A cycle the facts do not show (a what-if). It is examined only by an analyst and is always labelled `counterfactual` |
| **Scoping statement** | What an assessment covers: the sources in scope, the bridges followed, and whether counterfactual cycles are included |

### 2.3 Outcomes and potential

| Term | Definition |
|---|---|
| **Serious outcome** | A life-ending, life-threatening or life-altering injury (LIFE model; Bayona et al., 2023) |
| **P(serious)** | The probability that the outcome is serious or worse, read from the outcome distribution of the chosen energy band |
| **Chance P** | The probability that the energy reaches a person. Set to 1 when assessing potential |
| **Inherent SIF potential** | The outcome distribution if the energy had reached a person with no protective control between them |
| **Residual SIF potential** | The outcome distribution with the controls as they were, or as designed, each including its degraded and absent states. It is produced by the simulator |
| **SIF (class)** | P(serious) ≥ 0.50: the most likely outcome is serious |
| **Elevated (class)** | 0.10 ≤ P(serious) < 0.50: a serious outcome is credible |
| **Not SIF (class)** | P(serious) < 0.10 |

### 2.4 Controls

| Term | Definition |
|---|---|
| **Control** | Anything added to act on the source, or on the path to people, that can degrade, be defeated or be overwhelmed while the source stays unchanged |
| **Active control** | Detects and responds: trips, interlocks, safety instrumented functions, quench systems |
| **Passive control** | Acts by its presence but can still fail: guards, guardrails, bunds, blast walls, relief valves |
| **Procedural control** | Depends on people following a procedure: LOTO, permits, exclusion-zone sentries, training |
| **Direct control** | (EEI) A control targeted to the high-energy source, effective when installed, verified and used properly, and effective even with unintentional human error. Training, rules, signs and generic PPE do not qualify |
| **Control state** | For each control: effective, failed, absent, or not stated |

## 3. The fractalaw definition

### 3.1 Definitions

> **SIF potential.** An event has SIF potential when, had the energy reached a person with no protective control between the energy and the person, a life-ending, life-threatening or life-altering injury would be more likely than not: P(serious) ≥ 0.50.

> **Elevated potential.** As above, with 0.10 ≤ P(serious) < 0.50.

> **SIF actual.** An event in which a serious injury or fatality occurred.

> **PSIF (potential SIF).** An event with SIF potential, no serious injury, and no effective direct control. If the control state is not stated, it is recorded as *PSIF (controls unverified)*.

> **Control held.** An event with SIF potential, no serious injury, and an effective direct control that worked (EEI: *Capacity*).

> **SIF exposure.** Exposure to energy that would have SIF potential if an event occurred. It is a condition, not an event. With an effective direct control it is EEI's *Success*; without one, EEI's *Exposure*.

### 3.2 Assessment rules

1. **Chance P = 1.** For a reported event, the energy is assumed to have reached a person.
2. **Controls are set aside** when assessing potential: active, passive, procedural and PPE alike. Credit for controls comes only through the residual distribution.
3. **Not controls**: the source's containment, its inherent features, the default receiving environment (a concrete floor, rebar), and the person's own body.
4. **Assess the energy state at the time**, whatever its history. For a build-up, assess the level the energy would reach with no control acting, up to the source's physical limit, starting from the state as found at the cycle's boundary.
5. **One cycle per source, evidential cycles only.** Keep the most serious.
6. **Missing detail** goes to the lowest band consistent with what the report states.
7. **Thresholds**: 0.50 (SIF) and 0.10 (Elevated) by default; the company may set its own.

### 3.3 Pros and cons

**Pros**

- The counterfactual is built by rule ("all controls absent", energy physics), not by imagination. That makes it reproducible.
- It is calibrated to injury data for each mechanism and band, and gives a continuous probability with a company-set threshold.
- One method covers every energy type, including chemical, thermal and breathing.
- Exposure is control-blind, so it is a valid denominator for rates (risk = exposure × chance per unit × consequence).
- It separates inherent potential (how bad the energy is) from residual potential (how bad it is with these controls, failure states included).
- It maps onto EEI, Campbell and BST categories (section 5).

**Cons**

- It depends on how much detail the narrative gives. The lowest-band default can under-call thin reports.
- The scale measures threat to life. Life-altering injuries with low mortality (amputation, sight loss) are under-counted.
- Narratives rarely state controls, so most PSIFs are "controls unverified".
- Build-up limits and bridge conditions need an analyst's judgement.
- The strength of the calibration evidence varies between bands.

## 4. Other published definitions

Wording is verbatim unless marked *(secondary)*.

### 4.1 EEI Safety Classification and Learning (SCL) model

Hallowell & Spencer (2024); EEI (2020, rev. 2024).

The model asks four questions: "Was high energy present? Did a high-energy incident occur? Was a serious injury sustained? Was a direct control present?"

- **High energy**: "more than 500 ft-lbs of physical energy" (about 680 J).
- **High-energy incident**: "the high-energy source was released and … the worker came in contact with or proximity to the high-energy source". Proximity means within 6 ft.
- **HSIF**: "release of high energy where a serious injury was sustained."
- **PSIF**: "release of high energy in the absence of a direct control where a serious injury was not sustained."
- **Capacity**: "release of high energy in the presence of a direct control where a serious injury was not sustained."
- **Exposure**: "high-energy hazard was present without a corresponding direct control."
- **Success**: "high-energy hazard was present and had a corresponding direct control."

**Pros:** rule-based and fast; high agreement after calibration (about 95% within the working group); a single energy threshold; separates successes from failures.

**Cons:** the threshold is in physical units only and is awkward for chemical, thermal and toxic energy; the energy test is yes/no, with no range of outcomes; controls are credited as yes/no; the proximity rule ties classification to where people happened to be; "Capacity" clashes with Safety-II and HOP usage.

### 4.2 Campbell Institute (National Safety Council)

Campbell Institute (2018, 2020). *(Secondary.)*

- **SIF potential**: an event that "could have resulted in a serious injury or fatality if not for certain barriers or countermeasures, or if one factor around the event had been changed."
- **Precursor**: "a high-risk situation in which control measures are absent, ineffective or not complied with, and would potentially result in a fatality or serious injury if allowed to continue."

**Pros:** easy to explain; counts barrier-saved events, so no high-potential event is missed.

**Cons:** the counterfactual is open-ended ("one factor changed"); agreement between raters is low (about 65% for list and judgement methods); no energy or severity threshold.

### 4.3 BST / DEKRA precursor

Krause (2010); Martin & Black (2015).

- **Precursor**: "a high risk situation in which management controls are either absent, ineffective, or not complied with, and if allowed to continue or repeat could reasonably result in a serious injury or fatality" (Krause, 2010).
- **SIF exposure potential**: the exposure "could have reasonably and realistically resulted in a fatality or serious injury outcome if repeated" (Martin & Black, 2015, *secondary*).

**Pros:** covers conditions as well as events; ties potential to failures of management control; >90% agreement reported for trained analysts.

**Cons:** "reasonably and realistically" depends on judgement; "management controls" is broad and undefined; reliability is reported only within the firm.

### 4.4 ASTM E2920-26

ASTM (2026). *(Secondary.)*

- **aSIF**: an incident with a "clear workplace connection" and "life-ending, life-threatening, or life-altering consequences."
- **pSIF**: an overlay on any tier: an incident with a "likelihood of resulting in life-ending, life-threatening, or life-altering consequences."
- **SIFRisk**: system conditions likely to produce a SIF if not addressed, with no incident needed.

**Pros:** a standard; uses LIFE severity; separates actual, potential and condition-based risk.

**Cons:** "likelihood" is not quantified; no energy threshold; the full wording is behind a paywall.

### 4.5 LIFE model (severity)

Bayona et al. (2023).

- **Life-ending.**
- **Life-threatening**: "requires immediate medical intervention or lifesaving support (e.g., CPR, defibrillation) to save the injured person's life."
- **Life-altering**: "the injured worker will not fully recover and will most likely suffer permanent impairment from the loss of the use a major internal organ … body function or body part."

**Pros:** intensional (rule-based) rather than a list; consistent at the margins where lists disagree; widely adopted.

**Cons:** defines severity only, with no potential or control element; still needs judgement about "will not fully recover".

### 4.6 Regulatory lists

- **RIDDOR 2013, reg. 4 (specified injuries).** Fractures other than to fingers, thumbs and toes; amputation; permanent loss or reduction of sight; crush injury to the head or torso; serious burns; scalping; loss of consciousness from head injury or asphyxia; enclosed-space hypothermia or heat illness. *Dangerous occurrences* (Schedule 2) are a list of no-injury events that must be reported.
- **OSHA 29 CFR 1904.39.** Fatality; in-patient hospitalisation; amputation; loss of an eye.

**Pros:** legally defined; auditable; already collected.

**Cons:** outcome only, with no potential (apart from RIDDOR's dangerous occurrences); lists disagree at the margins; hospitalisation depends on local practice.

### 4.7 ICMM material unwanted event

ICMM (2015).

- **Material unwanted event (MUE)**: "An unwanted event where the potential or real consequence exceeds a threshold defined by the company as warranting the highest level of attention."

**Pros:** the company sets the threshold; links directly to critical control management.

**Cons:** the threshold is not standardised, so results can't be compared across companies; "potential" is undefined.

## 5. Crosswalk

| fractalaw | EEI SCL | Campbell | BST / DEKRA | ASTM E2920 |
|---|---|---|---|---|
| SIF actual | HSIF | SIF | SIF | aSIF |
| PSIF | PSIF | SIF potential | SIF exposure | pSIF |
| PSIF (controls unverified) | (no category) | SIF potential | SIF exposure | pSIF |
| Control held | Capacity | SIF potential | (not stated) | pSIF |
| SIF exposure, direct control absent | Exposure | Precursor | Precursor | SIFRisk |
| SIF exposure, direct control effective | Success | (none) | (none) | (none) |
| Elevated potential | (no category) | (judgement) | (judgement) | pSIF (possible) |
| Not SIF | Low-severity | (none) | (none) | Priority / recordable |

## 6. Coding rules: determining PSIF

### 6.1 Before you start

- **You need:** the event narrative, Annex A (mechanisms) and Annex B (bands).
- **Apply these throughout:**
  - **Chance P = 1.** Assume the energy reached a person, whether or not anyone was hit.
  - **Ignore all controls when judging potential.** Guardrails, harnesses, PPE, LOTO, interlocks, exclusion zones, training, evasive action and luck all count for nothing at that step.
  - **Do count** the default environment (the surface someone would land on, nearby edges and rebar) and the body region at risk.
  - **Use only facts in the report.** Don't imagine controls that weren't there, or energy that wasn't there.
  - **Never let the actual outcome lower the potential.** A minor injury from a high-energy event is still high potential.

### 6.2 Steps

Work through steps 1–7 once **for each energy source** the report shows was involved.

**Step 1. Identify the energy source.** Name the source and its energy type: gravity, motion, mechanical, electrical, pressure, thermal, chemical, radiation, sound, biological.
- If the report describes no energy transfer (e.g. a medical event, or illness from long-term exposure), **stop: out of scope.**

**Step 2. Check the energy was present.** Was the energy held, stored or available? Isolated or immobilised energy counts as present.
- If it had been eliminated (removed, drained, discharged), **stop: no exposure.**

**Step 3. Decide whether an event occurred.** Did any of these happen?
- (a) The energy changed state and went outside its envelope in space (flyrock beyond the zone, a dropped load, steam discharging onto a walkway).
- (b) A person entered a danger zone while the energy was live (an incursion: reaching past a guard on a running machine, walking into a segregated route), or the energy changed state while a person was within reach (a fall, a load swinging toward a rigger, a vehicle reversing toward a pedestrian).
- (c) The energy built up beyond its design level, or appeared where it should have been zero (overpressure, overload, overspeed, a re-energised isolated circuit).

If none of these happened:
- With a person within reach, record **SIF exposure** if the energy would have SIF potential (assess it with steps 4–5). **Stop: not an event, not a PSIF.**
- Otherwise, **stop: no event.**

A planned release that stayed inside its envelope with nobody inside it (a blast fired with the zone clear) is **not an event**.

**Step 4. Choose the mechanism and band.**
- Choose the mechanism from Annex A: the physical process that caused, or could have caused, harm.
- Choose the band from Annex B that matches the energy **as stated in the report**.
- If the report gives no magnitude, choose the lowest band consistent with the facts. Example: "fell from a ladder" with no height is `fall/low` if the ladder implies 1–3 m, or `fall/standing` if nothing suggests height.
- For a build-up, choose the band for the level the energy would reach if no control had acted, starting from the state as found (the batch as charged, the load as lifted).
- Body region and environment: if the report puts the head, neck or spine at risk, or the landing on concrete, steel or rebar, and two bands fit, take the higher.

**Step 5. Read the potential.** Take P(serious) and the class from Annex B for that band.
- P(serious) ≥ 0.50: **SIF potential.**
- 0.10 to under 0.50: **Elevated.**
- Under 0.10: **Not SIF.**
- If your company uses different thresholds, apply them here.

**Step 6. Record the outcome.** Did anyone sustain a life-ending, life-threatening or life-altering injury (LIFE definitions, section 4.5)? Answer yes or no.

**Step 7. Record the control state.** Was a **direct control** in place on this energy, and did it work? Record one of:
- **effective**: a direct control was in place and stopped or contained the energy (a harness arrested the fall; a relief valve lifted to a safe point; a trip worked);
- **failed or absent**: there was no direct control, or one was in place and failed or was defeated;
- **not stated**: the report doesn't say.

Training, rules, signs, generic PPE, evasive action and luck are **not** direct controls.

### 6.3 Determination

| Step 5 potential | Step 6 serious injury? | Step 7 control state | Determination |
|---|---|---|---|
| any | Yes | any | **SIF actual** (record the potential as well) |
| SIF | No | failed or absent | **PSIF** |
| SIF | No | not stated | **PSIF (controls unverified)** |
| SIF | No | effective | **Control held** |
| Elevated | No | any | **Elevated**, not PSIF |
| Not SIF | No | any | **Not SIF** |
| Not SIF or Elevated | Yes | any | **SIF actual, low energy**: check the band choice |

**More than one energy source.** Record each source separately. The event's determination is the most serious, in this order: SIF actual, PSIF, PSIF (controls unverified), control held, Elevated, Not SIF.

**Counterfactuals.** If you want to examine a source or bridge the report does *not* show (e.g. "the fuel could have ignited"), record it separately, labelled `counterfactual`. It never changes the determination.

### 6.4 What to record

| Field | Content |
|---|---|
| Source | Energy source and type |
| Event clause | a, b or c; or *exposure only*; or *no event* |
| Mechanism / band | From Annexes A and B |
| Stated magnitude | Height, voltage, speed, mass or temperature, if given |
| P(serious) and class | From Annex B |
| Serious injury | Yes / no |
| Control state | Effective / failed or absent / not stated, naming the control |
| Determination | From 6.3 |
| Rationale | One sentence |

### 6.5 Worked examples

| Report | Steps | Determination |
|---|---|---|
| "Scaffold board fell from the 4th lift (about 8 m) into the barriered area below. Nobody hurt." | Gravity; present; event (a). `struck/heavy_dropped` (5–50 kg from 3–15 m): SIF. No injury. The barrier is an exclusion zone, a procedural/passive control. Its effectiveness isn't stated | **PSIF (controls unverified)** |
| "Operative slipped at the edge of a 5 m platform. Harness and lanyard arrested the fall. No injury." | Gravity; event (b). `fall/medium`: SIF. No injury. Fall arrest = effective direct control | **Control held** |
| "Electrician found the circuit live while working on it; it had been back-fed from a generator. No shock." | Electrical, 230 V; event (c), present where it should be zero. `electrical/low`: Elevated. No injury | **Elevated** |
| "Pedestrian walked into the segregated forklift route; the forklift braked. No contact." | Motion; incursion, event (b). `transport/yard_speed`: SIF (0.50). No injury. Segregation (direct control) failed | **PSIF** |
| "Worker strained back lifting a box." | Overexertion; event (b). `overexertion/manual_handling`: Not SIF | **Not SIF** |
| "Rigger standing under a suspended load; nothing happened." | Gravity; present; no event | **SIF exposure** (not a PSIF) |
| "Operator cleared a jam on the conveyor with the guard open and the belt running; no injury." | Mechanical; incursion while energised, event (b). `caught_in/medium_machinery`: SIF (0.50). No injury. Guard removed; no isolation | **PSIF** |
| "Jam cleared with the guard open after LOTO was applied and verified." | Mechanical; energy present behind the isolator; no incursion and no event | **SIF exposure** with isolation effective (EEI: Success) |

## Annex A. Mechanisms

| Mechanism | Energy types | Covers |
|---|---|---|
| `transport` | motion | Vehicles and mobile plant striking people or overturning |
| `fall` | gravity | A person falling, from height or on the same level |
| `struck` | gravity, motion, mechanical | Struck by a falling, flying, swinging or moving object |
| `caught_in` | mechanical | Caught in or between machinery or objects |
| `structural_collapse` | gravity | Collapse of structures, excavations, scaffolds, formwork |
| `explosion` | pressure, chemical | Explosions and blast |
| `fire` | thermal, chemical | Fire, flash fire, clothing ignition |
| `electrical` | electrical | Electric shock, arc flash |
| `thermal` | thermal | Hot or cold surfaces, molten material, hot or cold environments |
| `chemical` | chemical | Corrosive and toxic contact or ingestion |
| `breathing` | chemical | Toxic or oxygen-deficient atmospheres, drowning, confined spaces |
| `pressure` | pressure | Release of pressurised fluid or gas, injection injuries |
| `radiation_noise` | radiation, sound | Ionising and non-ionising radiation, laser, arc UV, impulse noise |
| `assault` | motion, mechanical | Violence by another person |
| `animal_insect` | biological, motion | Bites, stings, large-animal impact |
| `overexertion` | (body) | Manual handling, repetitive strain |
| `slip_no_fall` | (body) | A slip or trip without a fall |
| `abrasion` | mechanical | Friction and rubbing injuries |

## Annex B. Band table

Bands are ordered from least to most severe within each mechanism. P(serious) comes from each band's calibrated outcome distribution, assuming a working-age adult, no controls, and the default (hard) environment. The class uses the default thresholds (0.50 and 0.10). **High energy** marks the bands that meet Hallowell's "Stuff That Kills You" (STKY) high-energy criterion. Exception: `fall/low` (1–3 m) counts as high energy only when a height of 1.8 m or more is stated. Source: `data/sif/calibration/` (the classifier's calibration).

**abrasion**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `minor_friction` | Minor friction/rubbing injury | 0.000 | Not SIF |  |
| 2 | `power_tool_graze` | Glancing contact with powered rotating surface | 0.011 | Not SIF |  |
| 3 | `high_speed_friction` | Sustained contact with high-speed surface | 0.100 | Elevated |  |

**animal_insect**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `insect_sting_no_allergy` | Insect sting or minor bite without allergic response | 0.000 | Not SIF |  |
| 2 | `anaphylaxis` | Anaphylactic reaction to sting or bite in sensitised individual | 0.100 | Elevated |  |
| 3 | `venomous_bite` | Venomous snake or spider bite | 0.100 | Elevated |  |
| 4 | `dog_bite` | Dog bite or attack | 0.011 | Not SIF |  |
| 5 | `large_animal` | Large animal kick, charge, crush, or trample | 0.100 | Elevated |  |
| 6 | `large_animal_confined` | Large animal attack in confined space — cattle crush, horse stable, bull pen | 0.500 | SIF |  |

**assault**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `verbal_threat` | Verbal abuse, threats, intimidation without physical contact | 0.000 | Not SIF |  |
| 2 | `unarmed_physical` | Unarmed physical assault — punches, kicks, pushing, biting, scratching, restraint | 0.011 | Not SIF |  |
| 3 | `blunt_weapon` | Assault with blunt weapon or improvised weapon | 0.100 | Elevated |  |
| 4 | `edged_weapon` | Assault with knife, blade, or sharp weapon | 0.500 | SIF |  |
| 5 | `firearm` | Assault with firearm | 0.720 | SIF |  |

**breathing**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `nuisance_dust_fume` | Nuisance dust, welding fume, mild irritant gas below acute threshold | 0.000 | Not SIF |  |
| 2 | `toxic_gas_acute` | Acute toxic gas or vapour exposure above IDLH | 0.500 | SIF | yes |
| 3 | `oxygen_deficient` | Oxygen-deficient atmosphere (<19.5% O2) | 0.720 | SIF | yes |
| 4 | `drowning_submersion` | Drowning or submersion | 0.720 | SIF | yes |
| 5 | `confined_space_combined` | Confined space with combined hazards — oxygen deficiency + toxic gas + limited escape | 0.900 | SIF | yes |

**caught_in**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `light_machinery` | Light powered equipment with limited torque | 0.100 | Elevated |  |
| 2 | `medium_machinery` | Medium powered machinery with significant torque | 0.500 | SIF | yes |
| 3 | `heavy_machinery` | Heavy industrial machinery with high torque | 0.720 | SIF | yes |
| 4 | `caught_between` | Caught between or crushed between moving and fixed objects | 0.500 | SIF | yes |

**chemical**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `irritant_mild` | Mild irritant or sensitiser | 0.000 | Not SIF |  |
| 2 | `corrosive_localised` | Corrosive contact to localised area | 0.100 | Elevated |  |
| 3 | `corrosive_large_area` | Corrosive contact to large body area | 0.500 | SIF | yes |
| 4 | `acute_toxic_dermal` | Acute systemic toxicity from skin absorption | 0.500 | SIF | yes |
| 5 | `acute_toxic_ingestion` | Accidental ingestion of acutely toxic substance | 0.500 | SIF | yes |
| 6 | `hf_special` | Hydrofluoric acid exposure — any concentration, any route | 0.500 | SIF | yes |

**electrical**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `extra_low` | Extra-low voltage (0–50 V) | 0.000 | Not SIF |  |
| 2 | `low` | Low voltage (50–240 V) | 0.100 | Elevated | yes |
| 3 | `medium` | Medium voltage (240–600 V) | 0.500 | SIF | yes |
| 4 | `high` | High voltage (600–15000 V) | 0.720 | SIF | yes |
| 5 | `very_high` | Very high voltage (≥ 15000 V) | 0.900 | SIF | yes |

**explosion**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `minor_confined` | Minor confined explosion | 0.100 | Elevated |  |
| 2 | `significant_industrial` | Significant industrial explosion | 0.500 | SIF | yes |
| 3 | `major_catastrophic` | Major catastrophic explosion | 0.900 | SIF | yes |

**fall**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `standing` | Standing height / same level (0–1 m) | 0.001 | Not SIF |  |
| 2 | `low` | Low elevation (1–3 m) | 0.100 | Elevated |  |
| 3 | `medium` | Medium elevation (3–6 m) | 0.500 | SIF | yes |
| 4 | `high` | High elevation (6–10 m) | 0.500 | SIF | yes |
| 5 | `very_high` | Very high elevation (10–20 m) | 0.900 | SIF | yes |
| 6 | `extreme` | Extreme elevation (≥ 20 m) | 1.000 | SIF | yes |

**fire**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `flash_momentary` | Brief flash or flare | 0.100 | Elevated |  |
| 2 | `clothing_ignition` | Clothing catches fire | 0.500 | SIF | yes |
| 3 | `structural_fire` | Established structural or industrial fire | 0.720 | SIF | yes |
| 4 | `explosion_fireball` | Explosion with fireball or BLEVE | 0.900 | SIF | yes |

**overexertion**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `manual_handling` | Lifting, carrying, pushing, pulling | 0.001 | Not SIF |  |
| 2 | `repetitive_strain` | Repetitive motion, sustained awkward posture, vibration exposure | 0.007 | Not SIF |  |
| 3 | `extreme_physical_stress` | Extreme physical exertion in hostile environment | 0.011 | Not SIF |  |

**pressure**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `low_pressure_pneumatic` | Low-pressure pneumatic or air system (<10 bar) | 0.011 | Not SIF |  |
| 2 | `tyre_inflation` | Tyre/wheel explosion during inflation or servicing | 0.500 | SIF | yes |
| 3 | `hydraulic_injection` | High-pressure hydraulic fluid injection (100-700 bar) | 0.500 | SIF | yes |
| 4 | `high_pressure_gas` | High-pressure gas release (>10 bar) | 0.500 | SIF | yes |
| 5 | `high_pressure_water_jet` | High-pressure water jet (100-3000 bar) | 0.500 | SIF | yes |

**radiation_noise**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `chronic_low_dose` | Chronic low-level ionising radiation | 0.000 | Not SIF |  |
| 2 | `acute_moderate` | Acute moderate ionising radiation dose (0.25-1 Sv) | 0.100 | Elevated |  |
| 3 | `acute_high` | Acute high ionising radiation dose (1-6 Sv) | 0.720 | SIF | yes |
| 4 | `acute_lethal` | Acute lethal ionising radiation dose (>6 Sv) | 0.900 | SIF | yes |
| 5 | `laser_high_power` | High-power laser exposure (Class 3B/4) | 0.100 | Elevated | yes |
| 6 | `uv_arc_flash` | Intense UV exposure | 0.001 | Not SIF |  |
| 7 | `rf_microwave` | RF/microwave exposure above occupational limits | 0.011 | Not SIF |  |
| 8 | `impulse_noise` | Impulse noise above 140 dB peak | 0.001 | Not SIF |  |

**slip_no_fall**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `indoor_level` | Indoor same-level slip/trip on flat surface | 0.001 | Not SIF |  |
| 2 | `outdoor_uneven` | Outdoor same-level slip/trip on uneven or hazardous surface | 0.011 | Not SIF |  |
| 3 | `near_secondary_hazard` | Same-level slip/trip adjacent to a secondary hazard | 0.100 | Elevated |  |

**struck**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `hand_tool` | Hand tool, small falling object (<2kg from <3m), low-speed contact | 0.011 | Not SIF |  |
| 2 | `light_dropped` | Light object from low-moderate height (<5kg from <5m) | 0.100 | Elevated |  |
| 3 | `heavy_dropped` | Heavy object from height (5-50kg from 3-15m) | 0.500 | SIF | yes |
| 4 | `swinging_projectile` | Swinging load, pressurised release, high-speed projectile | 0.439 | Elevated | yes |
| 5 | `heavy_object_fall` | Heavy object from height (>50kg from >3m) | 0.720 | SIF | yes |

**structural_collapse**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `minor_partial` | Partial collapse of light structure or material | 0.100 | Elevated |  |
| 2 | `trench_excavation` | Trench or excavation collapse | 0.900 | SIF | yes |
| 3 | `scaffold_formwork` | Scaffold or formwork collapse | 0.500 | SIF | yes |
| 4 | `building_structure` | Building or major structure collapse | 0.900 | SIF | yes |

**thermal**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `warm` | Warm surface or liquid (40-60°C) (40–60 °C) | 0.001 | Not SIF |  |
| 2 | `hot_surface` | Hot surface or pressurised liquid (60-150°C) (60–150 °C) | 0.100 | Elevated |  |
| 3 | `high_temp_surface` | High temperature surface or process (150-500°C) (150–500 °C) | 0.439 | Elevated | yes |
| 4 | `molten_material` | Molten material or extreme heat (>500°C) (≥ 500 °C) | 0.720 | SIF | yes |
| 5 | `hot_environment_moderate` | Hot working environment (WBGT 28-32°C) | 0.011 | Not SIF |  |
| 6 | `hot_environment_extreme` | Extreme heat environment (WBGT >32°C) or sustained heavy exertion in heat | 0.500 | SIF |  |
| 7 | `cold_contact` | Contact with cold surface or substance | 0.100 | Elevated |  |
| 8 | `cold_environment_moderate` | Cold working environment (0 to -10°C) with wind/wet exposure | 0.001 | Not SIF |  |
| 9 | `cold_environment_extreme` | Extreme cold environment (<-10°C) or cold water immersion | 0.500 | SIF |  |

**transport**

| # | Band | Description | P(serious) | Class | High energy |
|---|---|---|---|---|---|
| 1 | `yard_speed` | Yard/site speed (0–20 km/h) | 0.500 | SIF | yes |
| 2 | `road_speed` | Urban/road speed (20–60 km/h) | 0.720 | SIF | yes |
| 3 | `highway_speed` | Highway/dual carriageway speed (≥ 60 km/h) | 0.900 | SIF | yes |

## References

- ASTM E2920-26 (2026). *Standard Guide for Recording Priority Occupational Injuries and Illnesses and SIF-Related …* https://store.astm.org/e2920-26.html
- Bayona, A., Bhandari, S., Hallowell, M., Sherratt, F., Bailey, J.M. & Upton, J. (2023). What is a serious injury? A model for defining serious injuries & fatalities. *Professional Safety*, 68(9).
- Campbell Institute (2018). *Serious Injury and Fatality Prevention: Perspectives and Practices*. National Safety Council.
- Campbell Institute (2020). *Designing Strategy for Serious Injury and Fatality Prevention*. National Safety Council.
- Edison Electric Institute (2020, rev. 2024). *Safety Classification and Learning (SCL) Model*.
- Hallowell, M.R. & Spencer, C. (2024). Safety Classification & Learning Model. *Professional Safety*, 69(2).
- Hendershot, D.C. (2012). Inherently safer design: the fundamentals. *Chemical Engineering Progress*, January 2012.
- ICMM (2015). *Critical Control Management: Implementation Guide*.
- ISO 12100:2010. *Safety of machinery – General principles for design – Risk assessment and risk reduction*.
- Krause, T.R. (2010). *New Findings on Serious Injuries and Fatalities*. BST.
- Martin, D.K. & Black, A.A. (2015). Preventing serious injuries & fatalities: study reveals precursors & paradigms. *Professional Safety*, 60(9).
- OSHA, 29 CFR 1904.39.
- RIDDOR 2013, regulation 4 and Schedule 2.

Full references and source notes are in [sif-definition-discussion.md](sif-definition-discussion.md).
