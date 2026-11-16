# SIF definitions and coding rules

Version 1.6, 2026-11-16. For safety directors, safety teams and anyone coding incident reports for serious injury and fatality (SIF) potential. The supporting rationale is in [sif-definition-discussion.md](sif-definition-discussion.md).

## Contents

- [1. Scope](#1-scope)
- [2. Terms](#2-terms)
- [3. Events](#3-events)
- [4. The fractalaw definition](#4-the-fractalaw-definition)
- [5. Other published definitions](#5-other-published-definitions)
- [6. Crosswalk](#6-crosswalk)
- [7. Coding rules: determining PSIF](#7-coding-rules-determining-psif)
- [Annex A. Mechanisms](#annex-a-mechanisms)
- [Annex B. Band table](#annex-b-band-table)
- [References](#references)

## 1. Scope

- **Covers:** SIF potential in workplace events where energy is transferred to, or toward, a person.
- **Unit of assessment:** one reported event, with each energy source in it assessed separately.
- **Excludes:** chronic exposure and occupational illness; medical events with no energy transfer (e.g. cardiac arrest); property and environmental harm.

## 2. Terms

### 2.1 Energy

| Term | Definition |
|---|---|
| **Energy source** | Anything that holds, stores or can release energy: gravity (height, suspended loads), motion, mechanical, electrical, pressure, thermal, chemical, radiation, sound, biological |
| **Energy present** | Energy that is held, stored or available to be released, including the supply behind an isolator and the fuel in a parked vehicle. Only elimination removes it |
| **Elimination** | Removing the energy from the source: supply disconnected and removed, system drained and vented, stored energy discharged, load landed |
| **Envelope** | The designed bounds on an energy: **where** it may be and how far it must be kept from people; **when** it may be live; and **how much** may be present. For contained energy, the envelope is the containment and its designed path. For an intentional release (a blast, live firing, a relief discharge), it is the designed release zone |
| **Live** | Energy in its working state, not isolated, with a line of fire to people: a suspended load, a running machine, an energised conductor, plant under pressure. Isolated energy is present (people near it are exposed) but not live |
| **Line of fire** | The path live energy takes as it moves, or would take if released: the drop path of a suspended load, the swing path of a slewed load, the travel path of a vehicle, the trajectory of flyrock. Danger zones are drawn around it |
| **Danger zone** | An area around the line of fire of live energy that is kept clear of people: an exclusion zone, a segregated route, a drop zone, or a machine's hazard zone behind its guards (ISO 12100) |
| **Containment** | What holds energy in its intended state: a vessel, pipe or cable insulation. Containment is part of the source, not a control |
| **Inherent feature** | A property of the source's own materials, quantity, operating conditions or physics. It works throughout normal operation without detection, actuation, power or human action, and can be lost only by changing the source. Examples: a reduced inventory, pump dead-head pressure, a reactor's negative temperature coefficient. It is part of the source, not a control |

### 2.2 People, exposure and events

| Term | Definition |
|---|---|
| **Target envelope** | Where people are, when they are there, and how many: one person, a team, a public crowd |
| **Exposure** | A **condition**: energy is present and a person is within its reach, judged **as if no controls were in place**. An operator at a guarded machine is exposed for the whole time they use it. Exposure is measured in units that suit the work: person-hours, lifts, entries |
| **Separation margin** | What keeps energy and people apart: distance, time, or the margin to the design limit |
| **Event** | A **happening**, not a condition: the separation margin between energy and people shrinks toward zero, because the energy changes (an excursion) or the people do (an incursion). An event is defined before any outcome is known |
| **Excursion** | The energy goes beyond its envelope: it is released or moves (where), becomes live when it should not be (when), or builds up beyond its design (how much) |
| **Incursion** | People enter the energy's envelope: they go into the line of fire of live energy, are there when it is live, or are more than planned. Time spent inside counts as exposure. Entry under isolation, or when the energy is not live, is not an incursion: there is no line of fire |
| **Closest approach** | How near the energy came to people during an event: **wide** (nobody near), **close** (near miss) or **contact** (the energy reached a person) |
| **Near miss** | An event whose closest approach was close, or that reached contact without harm. An excursion with nobody near is an event, not a near miss |

### 2.3 Outcomes and potential

| Term | Definition |
|---|---|
| **Serious outcome** | A life-ending, life-threatening or life-altering injury (the LIFE model, section 5.5) |
| **P(serious)** | The probability that the outcome is serious or worse, read from the outcome distribution of the energy band (Annex B) |
| **Separation to zero** | The basis for assessing potential: assume the separation margin went to zero, so the energy reached a person |
| **Residual SIF potential** | The outcome distribution with the controls as they were or as designed, including the chance that each control is degraded or absent. Produced by modelling the controls |
| **SIF (class)** | P(serious) ≥ 0.50: a serious outcome is the most likely outcome |
| **Elevated (class)** | 0.10 ≤ P(serious) < 0.50: a serious outcome is credible |
| **Not SIF (class)** | P(serious) < 0.10 |

### 2.4 Controls

| Term | Definition |
|---|---|
| **Control** | Anything added to act on the source, or on the path to people, that can degrade, be defeated or be overwhelmed while the source stays unchanged |
| **Active control** | Detects and responds: trips, interlocks, safety instrumented functions, quench systems |
| **Passive control** | Works by being there, and can still fail: guards, guardrails, bunds, blast walls, relief valves |
| **Procedural control** | Depends on people following a procedure: LOTO, permits, exclusion-zone sentries, training |
| **Direct control** | A control targeted at the specific energy, effective when installed, verified and used properly, and effective even if someone makes an unintentional error (EEI). Training, rules, signs and generic PPE are not direct controls |
| **Control state** | For each control: effective, failed, absent, or not stated |

## 3. Events

An event is a **happening**: something changes and the separation margin between energy and people shrinks. An exposure is a **condition**: energy and people are together and nothing changes.

In risk terms (risk = exposure × chance per unit of exposure × consequence), the event is where the **chance** term materialises. It is the top event of a bowtie: the preventive side has not kept energy and people apart. SIF potential then assesses the **consequence** term.

### 3.1 Two envelopes

The energy and the people each have an envelope in the same three dimensions:

| | **Energy** | **People** |
|---|---|---|
| **Where** | Where the energy is, or can reach | Where the people are |
| **When** | When the energy is live: energised, operating, the firing window | When the people are there: the shift, the schedule, an entry |
| **How much** | The magnitude of the energy | How many people: one person, a team, a crowd |

### 3.2 What makes an event

An event is a change on either side that shrinks the separation between the two envelopes.

| | **The energy changes** (excursion) | **The people change** (incursion) |
|---|---|---|
| **Where** | It is released or moves beyond its envelope: flyrock beyond the zone; a load swings off its path; a vehicle leaves its route; a person at height falls | People enter the line of fire of live energy: walking into a segregated route; reaching into a running machine; walking under a suspended load |
| **When** | It becomes live when it should not be: a back-fed isolated circuit; a machine restarts; firing starts early | People are present when the energy is live: entering the range during the firing window; still in the blast zone at firing |
| **How much** | It builds up beyond its design: overpressure; overload; overspeed; a runaway reaction | More people than planned: a crowd gathers in the drop zone; a team works where one person was planned |

Both sides can change at once: a person climbs past edge protection and falls; an isolated system is re-energised while a fitter is inside the guard.

### 3.3 Closest approach

Every event has a closest approach. It is recorded for learning and reporting; it does not change the potential.

| Closest approach | Case | Example |
|---|---|---|
| **Wide**: nobody near | Excursion with nobody near | Flyrock lands in an empty area; a load drops into a cleared drop zone |
| **Close**: small margin, no contact | Near miss | Flyrock lands a metre from someone; a forklift stops short of a pedestrian |
| **Contact**: no margin | The energy reached a person, with or without harm | A person is struck by flyrock; a harness arrests a fall |

### 3.4 Not events

- **Exposure:** energy and people together, with nothing changing. Examples: tag-liners holding their lines at the edge of a lift, outside the drop path; an operator at a guarded, running machine; working dead behind an isolator.
- **Separation held:** the envelopes are kept apart in where (distance, barriers) or when (scheduling, permits, firing windows). Entering the range tomorrow, when no firing is planned, is separation in time.
- **A planned release that stays inside its envelope** with nobody in it, such as a blast fired with the zone clear.
- **A control failing on its own**, such as a relief valve sticking. The event is what the energy does next (the overpressure); the stuck valve is its control state.

The moment of an event is when the separation margin is lost, even if the change was slow (snow load, rising pressure, a crowd gathering).

The number of people does not change SIF potential, which is judged per person. It multiplies the exposure and the number of people who could be harmed in a single event.

## 4. The fractalaw definition

### 4.1 Definitions

> **SIF potential.** An event has SIF potential when the energy involved, had it reached a person with no protective control between them, would more likely than not have caused a life-ending, life-threatening or life-altering injury.

> **Elevated potential.** An event has elevated potential when the energy involved, had it reached a person with no protective control between them, would credibly, but not more likely than not, have caused a life-ending, life-threatening or life-altering injury.

> **SIF actual.** An event in which a serious injury or fatality occurred.

> **PSIF (potential SIF).** An event with SIF potential and no serious injury. Every PSIF carries one of three control-state qualifiers:
>
> - **PSIF (unprotected):** no effective protective control stood between the energy and the person, because it was absent or it failed.
> - **PSIF (protected):** an effective direct control stood between the energy and the person, and worked.
> - **PSIF (unverified):** the report does not state the control state.

> **SIF exposure.** Exposure to energy that would have SIF potential if an event occurred. It is a condition, not an event.

### 4.2 Assessment rules

1. **Separation to zero.** Assess every event as if the energy reached a person, whatever its closest approach.
2. **Set controls aside.** Ignore active, passive and procedural controls and PPE when assessing potential. Controls are credited only in residual SIF potential.
3. **These are not controls:** the source's containment, its inherent features, the receiving environment (a concrete floor, rebar), and the person's own body. Count them.
4. **Assess the energy as it was at the time.** How it came to be there does not change the potential.
5. **Energy that builds up** (pressure, load, a reaction) is assessed at the level it would have reached if nothing had stopped it, starting from conditions as found (the batch as charged, the load as lifted), up to the source's physical limit.
6. **Assess each energy source separately** and keep the most serious result. Use only what the report shows happened.
7. **Missing detail** takes the lowest band consistent with what the report states.
8. **Thresholds:** SIF at 0.50 and Elevated at 0.10. A company may set its own.

### 4.3 Strengths and limitations

**Strengths**

- Potential is set by rule (separation to zero, controls set aside, energy physics), not by an assessor's imagination. Two coders reach the same answer.
- Each mechanism and band is calibrated to injury data and gives a probability, which the company's threshold turns into a class.
- One method covers every energy type, including chemical, thermal and breathing hazards.
- Exposure ignores controls, so it is a sound denominator for rates.
- SIF potential (how dangerous the energy is) is kept separate from residual SIF potential (how dangerous it is with these controls).
- Results map directly onto the EEI, Campbell, BST and ASTM categories (section 6).

**Limitations**

- A thin narrative gives a low band. Reports that omit height, speed, voltage or mass will be under-called. Good reporting is the fix.
- The scale measures threat to life. Life-altering injuries with low mortality, such as finger amputation and sight loss, are under-weighted.
- Most reports do not state controls, so most PSIFs will be *unverified* until reporting captures control state.
- Energy that builds up needs an analyst to set its limit.

## 5. Other published definitions

### 5.1 EEI Safety Classification and Learning (SCL) model

Edison Electric Institute (2020, rev. 2024); Hallowell & Spencer (2024).

The model asks four questions: "Was high energy present? Did a high-energy incident occur? Was a serious injury sustained? Was a direct control present?"

- **High energy**: "more than 500 ft-lbs of physical energy" (about 680 J).
- **High-energy incident**: "the high-energy source was released and … the worker came in contact with or proximity to the high-energy source". Proximity is within 6 ft.
- **HSIF**: "release of high energy where a serious injury was sustained."
- **PSIF**: "release of high energy in the absence of a direct control where a serious injury was not sustained."
- **Capacity**: "release of high energy in the presence of a direct control where a serious injury was not sustained."
- **Exposure**: "high-energy hazard was present without a corresponding direct control."
- **Success**: "high-energy hazard was present and had a corresponding direct control."

**Pros:** rule-based and quick to apply; about 95% agreement after calibration; one energy threshold; separates successes from failures.

**Cons:** the threshold is in mechanical units, which fit chemical, thermal and toxic energy poorly; energy is yes/no, with no range of outcomes; controls are credited yes/no; the proximity rule makes the classification depend on where people happened to be; "Capacity" means something different in Safety-II and HOP.

### 5.2 Campbell Institute (National Safety Council)

Campbell Institute (2018, 2020).

- **SIF potential**: an event that could have resulted in a serious injury or fatality but for the barriers or countermeasures in place, or if one factor had been different.
- **Precursor**: a high-risk situation where control measures are absent, ineffective or not complied with, and which would potentially result in a serious injury or fatality if allowed to continue.

**Pros:** easy to explain; counts barrier-saved events, so high-potential events are not missed.

**Cons:** "one factor different" is open-ended; agreement between raters is low (about 65% for list and judgement methods); no energy or severity threshold.

### 5.3 BST / DEKRA precursor

Krause (2010); Martin & Black (2015).

- **Precursor**: "a high risk situation in which management controls are either absent, ineffective, or not complied with, and if allowed to continue or repeat could reasonably result in a serious injury or fatality" (Krause, 2010).
- **SIF exposure potential**: an exposure that could reasonably and realistically have resulted in a fatality or serious injury if repeated (Martin & Black, 2015).

**Pros:** covers conditions as well as events; ties potential to failures of management control; more than 90% agreement among trained analysts.

**Cons:** "reasonably and realistically" rests on judgement; "management controls" is broad; reliability is reported only within the firm.

### 5.4 ASTM E2920-26

ASTM (2026).

- **aSIF**: an incident with a clear workplace connection and life-ending, life-threatening or life-altering consequences.
- **pSIF**: an overlay on any incident tier, for incidents likely to result in life-ending, life-threatening or life-altering consequences.
- **SIFRisk**: system conditions likely to produce a SIF if not addressed. No incident is needed.

**Pros:** a recognised standard; uses LIFE severity; separates actual, potential and condition-based risk.

**Cons:** "likely" is not quantified; no energy threshold.

### 5.5 LIFE model (severity)

Bayona et al. (2023).

- **Life-ending.**
- **Life-threatening**: "requires immediate medical intervention or lifesaving support (e.g., CPR, defibrillation) to save the injured person's life."
- **Life-altering**: "the injured worker will not fully recover and will most likely suffer permanent impairment from the loss of the use a major internal organ … body function or body part."

**Pros:** rule-based rather than a list; consistent at the margins where lists disagree; widely adopted.

**Cons:** defines severity only, with no potential or control element; "will not fully recover" needs judgement.

### 5.6 Regulatory lists

- **RIDDOR 2013, reg. 4 (specified injuries):** fractures other than to fingers, thumbs and toes; amputation; permanent loss or reduction of sight; crush injury to the head or torso; serious burns; scalping; loss of consciousness from head injury or asphyxia; enclosed-space hypothermia or heat illness. RIDDOR's *dangerous occurrences* (Schedule 2) are a list of no-injury events that must be reported.
- **OSHA 29 CFR 1904.39:** fatality; in-patient hospitalisation; amputation; loss of an eye.

**Pros:** legally defined; auditable; already collected.

**Cons:** outcome only, with no potential (apart from RIDDOR's dangerous occurrences); the lists disagree at the margins; hospitalisation depends on local practice.

### 5.7 ICMM material unwanted event

ICMM (2015).

- **Material unwanted event (MUE)**: "An unwanted event where the potential or real consequence exceeds a threshold defined by the company as warranting the highest level of attention."

**Pros:** the company sets the threshold; links directly to critical control management.

**Cons:** thresholds differ between companies, so results are not comparable; "potential" is not defined.

### 5.8 Event and near miss

The fractalaw event is consistent with the standard definitions and is more specific. Each one below is either a broader form of it or a special case.

| Source | Definition | Relation to the fractalaw event |
|---|---|---|
| ISO 31000:2018 | Event: "occurrence or change of a particular set of circumstances" | Broader. fractalaw specifies the change: the separation between energy and people |
| ISO 12100:2010 | Hazardous event: "event that can cause harm". Hazardous situation: "circumstance in which a person is exposed to at least one hazard" | The same split as fractalaw *event* and *exposure* |
| ISO 45001:2018 | Incident: "occurrence arising out of, or in the course of, work that could or does result in injury and ill health" | Includes the outcome. The fractalaw event is defined before the outcome |
| EEI SCL | Energy release: "a temporary loss of control of the high-energy, or the high-energy changes state while exposed to the work environment" | A special case: an excursion |
| Bowtie top event | "the point in time which describes the release or loss of control over a Hazard" (UK CAA) | The same point in the risk chain |
| CCPS / API RP 754 | Loss of primary containment: "an unplanned or uncontrolled release of material from primary containment" | A special case: an excursion in where |
| OSHA | Near miss: injury "easily could have occurred" "given a slight shift in time or position" | A close closest approach |
| ICAO Annex 13 | Accident and serious incident: "The difference … lies only in the result" | The relation between near miss and SIF actual |

## 6. Crosswalk

| fractalaw | EEI SCL | Campbell | BST / DEKRA | ASTM E2920 |
|---|---|---|---|---|
| SIF actual | HSIF | SIF | SIF | aSIF |
| PSIF, all qualifiers | PSIF + Capacity | SIF potential | SIF exposure | pSIF |
| PSIF (unprotected) | PSIF | SIF potential | SIF exposure | pSIF |
| PSIF (protected) | Capacity | SIF potential | (none) | pSIF |
| PSIF (unverified) | (none) | SIF potential | SIF exposure | pSIF |
| SIF exposure, no effective direct control | Exposure | Precursor | Precursor | SIFRisk |
| SIF exposure, effective direct control | Success | (none) | (none) | (none) |
| Elevated potential | (none) | (judgement) | (judgement) | pSIF (possible) |
| Not SIF | Low-severity | (none) | (none) | Priority / recordable |

## 7. Coding rules: determining PSIF

### 7.1 Before you start

You need the incident report, Annex A (mechanisms) and Annex B (bands).

Apply these rules throughout:

- **Separation to zero.** Assess as if the energy reached a person, whether or not anyone was hit.
- **Ignore all controls when judging potential.** Guardrails, harnesses, PPE, LOTO, interlocks, exclusion zones, training, evasive action and luck count for nothing at this stage.
- **Count the receiving environment** (the surface someone would land on, nearby edges and rebar) and the body region at risk.
- **Use only facts in the report.** Do not add controls or energy the report does not describe.
- **Never let the actual outcome lower the potential.** A minor injury from a high-energy event is still high potential.

### 7.2 Steps

Work through steps 1–7 for **each energy source** in the report.

**Step 1. Identify the energy source.** Name it and its energy type: gravity, motion, mechanical, electrical, pressure, thermal, chemical, radiation, sound or biological.
- No energy transfer (a medical event, illness from long-term exposure): **stop, out of scope.**

**Step 2. Check the energy was present.** Held, stored, available, isolated or immobilised energy all count as present.
- Energy eliminated (removed, drained, discharged): **stop, no exposure.**

**Step 3. Decide whether an event occurred.** Use section 3.2. Ask: **did the energy or the people change, so that the separation between them shrank?**
- **Excursion:** the energy was released, moved, became live or built up. Examples: a dropped load, flyrock beyond the zone, a fall from height, a re-energised isolated circuit, overpressure.
- **Incursion:** people entered the line of fire of live energy, were there when it was live, or were more than planned. Entering near isolated energy is not an incursion. Examples: reaching past a guard on a running machine, walking into a segregated route, being on the range during firing.

If yes, it is an **event**. Record whether it was an excursion, an incursion or both; the dimension (where, when, how much); and the closest approach (wide, close, contact). Go to step 4.

If no:
- Energy and a person were together with nothing changing: assess steps 4–5, and if the result is SIF, record **SIF exposure**. **Stop: not an event, not a PSIF.**
- Otherwise: **stop, no event.**

**Step 4. Choose the mechanism and band.**
- **Mechanism:** from Annex A, the physical process that caused or could have caused harm.
- **Band:** from Annex B, the band that matches the energy **as the report states it**.
- **No magnitude given:** take the lowest band consistent with the facts. "Fell from a ladder" with no height is `fall/low` if the ladder implies 1–3 m, and `fall/standing` if nothing suggests height.
- **Energy that built up:** take the band for the level it would have reached if nothing had stopped it, starting from the conditions as found.
- **Two bands fit:** take the higher if the head, neck or spine was at risk, or the landing was concrete, steel or rebar.

**Step 5. Read the potential.** Take P(serious) and the class from Annex B.
- P(serious) ≥ 0.50: **SIF potential.**
- 0.10 ≤ P(serious) < 0.50: **Elevated.**
- P(serious) < 0.10: **Not SIF.**
- If your company sets different thresholds, use them.

**Step 6. Record the outcome.** Did anyone suffer a life-ending, life-threatening or life-altering injury (section 5.5)? Yes or no.

**Step 7. Record the control state.** Was a **direct control** in place on this energy, and did it work?
- **Effective:** a direct control stopped or contained the energy. Examples: a harness arrested the fall; a relief valve lifted to a safe point; a trip worked.
- **Failed or absent:** there was no direct control, or it failed or was defeated.
- **Not stated:** the report does not say.

Training, rules, signs, generic PPE, evasive action and luck are **not** direct controls.

### 7.3 Determination

| Potential (step 5) | Serious injury (step 6) | Control state (step 7) | Determination |
|---|---|---|---|
| any | Yes | any | **SIF actual** (record the potential too) |
| SIF | No | failed or absent | **PSIF (unprotected)** |
| SIF | No | effective | **PSIF (protected)** |
| SIF | No | not stated | **PSIF (unverified)** |
| Elevated | No | any | **Elevated** |
| Not SIF | No | any | **Not SIF** |

If a serious injury occurred but the band is Elevated or Not SIF, recheck the band choice before recording.

**More than one energy source.** Record each source. The event takes the most serious determination, in this order: SIF actual, PSIF (unprotected), PSIF (unverified), PSIF (protected), Elevated, Not SIF.

**What-ifs.** A source or chain of events the report does not show (e.g. "the fuel could have ignited") is recorded separately, labelled `counterfactual`. It never changes the determination.

### 7.4 What to record

| Field | Content |
|---|---|
| Source | Energy source and type |
| Event type | Excursion, incursion or both, with the dimension (where, when, how much); or *exposure only*; or *no event* |
| Closest approach | Wide, close or contact |
| Mechanism / band | From Annexes A and B |
| Stated magnitude | Height, voltage, speed, mass or temperature, if given |
| P(serious) and class | From Annex B |
| Serious injury | Yes or no |
| Control state | Effective, failed or absent, or not stated, naming the control |
| Determination | From 7.3 |
| Rationale | One sentence |

### 7.5 Worked examples

| Report | Coding | Determination |
|---|---|---|
| "Scaffold board fell from the 4th lift (about 8 m) into the barriered area below. Nobody hurt." | Gravity. Excursion, where; closest approach wide. `struck/heavy_dropped` (5–50 kg from 3–15 m): SIF. No injury. The report does not say whether the exclusion zone was effective | **PSIF (unverified)** |
| "Operative slipped at the edge of a 5 m platform. Harness and lanyard arrested the fall. No injury." | Gravity. Excursion, where; contact. `fall/medium`: SIF. No injury. Fall arrest is an effective direct control | **PSIF (protected)** |
| "Pedestrian walked into the segregated forklift route; the forklift braked. No contact." | Motion. Incursion, where; close. `transport/yard_speed`: SIF. No injury. Segregation failed | **PSIF (unprotected)** |
| "Operator cleared a jam on the conveyor with the guard open and the belt running. No injury." | Mechanical. Incursion, where; close. `caught_in/medium_machinery`: SIF. No injury. Guard removed, no isolation | **PSIF (unprotected)** |
| "Electrician found the circuit live while working on it; it had been back-fed from a generator. No shock." | Electrical, 230 V. Excursion, when; close. `electrical/low`: Elevated. No injury | **Elevated** |
| "Worker tripped over a cable in the office and bruised a knee." | Gravity. Excursion, where; contact. `fall/standing`: Not SIF | **Not SIF** |
| "During a heavy lift, a labourer walked under the load to fetch a tool." | Gravity, suspended load (live). Incursion into the drop path; close. `struck/heavy_object_fall`: SIF. No injury. Exclusion zone failed | **PSIF (unprotected)** |
| "Visitor crossed the outer edge of the lift exclusion zone, well clear of the drop path." | Gravity, suspended load (live). Incursion, where; wide. `struck/heavy_object_fall`: SIF. No injury. Exclusion zone failed | **PSIF (unprotected)** |
| "Tag-liners controlled the load from the edge of the zone throughout the lift." | Gravity, suspended load (live). Planned exposure outside the drop path; nothing changed | **SIF exposure** |
| "Jam cleared with the guard open after LOTO was applied and verified." | Mechanical. Energy present behind the isolator; no incursion | **SIF exposure**, isolation effective |

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

Bands run from least to most severe within each mechanism. P(serious) comes from each band's calibrated outcome distribution, assuming a working-age adult, no controls and a hard receiving surface. The class uses the default thresholds (0.50 and 0.10). **High energy** marks bands that meet the high-energy ("Stuff That Kills You") criterion; `fall/low` (1–3 m) counts as high energy only when a height of 1.8 m or more is stated. The bands and probabilities are the fractalaw classifier's calibration, so human and AI coding use the same scale.

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

- API RP 754 (2021). *Process Safety Performance Indicators for the Refining and Petrochemical Industries*, 3rd ed. American Petroleum Institute.
- ASTM E2920-26 (2026). *Standard Guide for Recording Priority Occupational Injuries and Illnesses and SIF-Related …* https://store.astm.org/e2920-26.html
- Bayona, A., Bhandari, S., Hallowell, M., Sherratt, F., Bailey, J.M. & Upton, J. (2023). What is a serious injury? A model for defining serious injuries & fatalities. *Professional Safety*, 68(9).
- Campbell Institute (2018). *Serious Injury and Fatality Prevention: Perspectives and Practices*. National Safety Council.
- Campbell Institute (2020). *Designing Strategy for Serious Injury and Fatality Prevention*. National Safety Council.
- CCPS (2018). *Process Safety Metrics: Guide for Selecting Leading and Lagging Indicators*, v3.2. AIChE.
- Edison Electric Institute (2020, rev. 2024). *Safety Classification and Learning (SCL) Model*.
- Hallowell, M.R. & Spencer, C. (2024). Safety Classification & Learning Model. *Professional Safety*, 69(2).
- ICAO (2010). *Annex 13 to the Convention on International Civil Aviation: Aircraft Accident and Incident Investigation*, 10th ed.
- ICMM (2015). *Critical Control Management: Implementation Guide*.
- ISO 12100:2010. *Safety of machinery – General principles for design – Risk assessment and risk reduction*.
- ISO 31000:2018. *Risk management – Guidelines*.
- ISO 45001:2018. *Occupational health and safety management systems*.
- Krause, T.R. (2010). *New Findings on Serious Injuries and Fatalities*. BST.
- Martin, D.K. & Black, A.A. (2015). Preventing serious injuries & fatalities: study reveals precursors & paradigms. *Professional Safety*, 60(9).
- OSHA, 29 CFR 1904.39.
- OSHA. *Template for Near Miss Report Form*.
- RIDDOR 2013, regulation 4 and Schedule 2.
- UK Civil Aviation Authority. *BowTie: Top events*.
