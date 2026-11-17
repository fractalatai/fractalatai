# Defining SIF: a discussion draft

Status: draft v1.6, 2026-11-16. All open questions answered or parked. A starting point to polish, not a settled position.

## Contents

- [Why we need this](#why-we-need-this)
- [1. What the literature says](#1-what-the-literature-says)
  - [1.1 Severity: what counts as "serious"](#11-severity-what-counts-as-serious)
  - [1.2 The "potential" counterfactual](#12-the-potential-counterfactual)
  - [1.3 Controls: the sharpest split](#13-controls-the-sharpest-split)
  - [1.4 Energy: harmful, unwanted or uncontrolled?](#14-energy-harmful-unwanted-or-uncontrolled)
  - [1.5 Event or condition](#15-event-or-condition)
  - [1.6 Event in the literature](#16-event-in-the-literature)
- [2. The definition our classifier holds implicitly](#2-the-definition-our-classifier-holds-implicitly)
  - [Where this sits against the literature](#where-this-sits-against-the-literature)
- [3. The wording problem: unwanted, unexpected or uncontrolled?](#3-the-wording-problem-unwanted-unexpected-or-uncontrolled)
  - ["Uncontrolled"](#uncontrolled)
  - ["Unexpected" (unplanned)](#unexpected-unplanned)
  - ["Unwanted" (unintended)](#unwanted-unintended)
  - [Proposed position](#proposed-position)
  - [Refinement: intentional releases and the energy envelope](#refinement-intentional-releases-and-the-energy-envelope)
  - [Refinement: energy state, not its history](#refinement-energy-state-not-its-history)
  - [Refinement: over-energy states (build-up without release)](#refinement-over-energy-states-build-up-without-release)
  - [Refinement: the envelope in Mass, Length, Time](#refinement-the-envelope-in-mass-length-time)
  - [Refinement: event, exposure, excursion and incursion](#refinement-event-exposure-excursion-and-incursion)
  - [Refinement: exposure as the denominator of risk](#refinement-exposure-as-the-denominator-of-risk)
  - [Refinement: exposure strata, demand and incursion](#refinement-exposure-strata-demand-and-incursion)
  - [Refinement: the target has an envelope too](#refinement-the-target-has-an-envelope-too)
  - [Refinement: the event as shrinking separation](#refinement-the-event-as-shrinking-separation)
  - [Refinement: live energy and the line of fire](#refinement-live-energy-and-the-line-of-fire)
  - [Refinement: cycles, breaking the circle](#refinement-cycles-breaking-the-circle)
- [4. Proposal: three layers, not one definition](#4-proposal-three-layers-not-one-definition)
  - [4.1 Where does the source end and the path begin?](#41-where-does-the-source-end-and-the-path-begin)
  - [4.2 Intentional releases and relief devices](#42-intentional-releases-and-relief-devices)
  - [4.3 Energy state and build-up cases](#43-energy-state-and-build-up-cases)
  - [4.4 Controls in the outcome distribution (answering Q2)](#44-controls-in-the-outcome-distribution-answering-q2)
- [5. A draft definition](#5-a-draft-definition)
- [6. Consequences for the classifier if we adopt this](#6-consequences-for-the-classifier-if-we-adopt-this)
- [7. Open questions](#7-open-questions)
  - [Answered in this draft](#answered-in-this-draft)
  - [Still open](#still-open)
  - [Parked: beyond this paper](#parked-beyond-this-paper)
- [8. Checks before polishing](#8-checks-before-polishing)
- [Appendix A. Line of Fire](#appendix-a-line-of-fire)
  - [A.1 Origin](#a1-origin)
  - [A.2 What the authoritative versions say](#a2-what-the-authoritative-versions-say)
  - [A.3 The underlying model](#a3-the-underlying-model)
  - [A.4 Line of Fire in our definition](#a4-line-of-fire-in-our-definition)
  - [A.5 Crosswalk: IOGP LoF categories to classifier mechanisms](#a5-crosswalk-iogp-lof-categories-to-classifier-mechanisms)
  - [A.6 What LoF adds, and what it leaves out](#a6-what-lof-adds-and-what-it-leaves-out)
  - [A.7 Checks](#a7-checks)
- [Appendix B. Glossary](#appendix-b-glossary)
- [References](#references)

## Why we need this

The SIF classifier already holds a definition of SIF potential. That definition is spread across a prompt, calibration files and design notes, and has never been written down in one place. Validation against human coders makes this urgent: we can't say whether a disagreement is the AI's error or a difference in definition until both sides' definitions are explicit.

The hardest part is **controls**. Is SIF potential judged:

- with controls ignored,
- conditional on controls having failed, or
- with working controls credited?

The published literature splits on exactly this question.

## 1. What the literature says

Full citations are at the end. Wording marked *(verbatim)* was read in the source. Wording marked *(secondary)* comes from summaries and needs checking before we quote it.

### 1.1 Severity: what counts as "serious"

The literature is converging on a three-part intensional definition: **life-ending, life-threatening or life-altering**.

- **LIFE model** (Bayona et al., 2023) *(verbatim)*: life-threatening is "a case that requires immediate medical intervention or lifesaving support (e.g., CPR, defibrillation) to save the injured person's life". Life-altering is "a case in which the injured worker will not fully recover and will most likely suffer permanent impairment from the loss of the use a major internal organ … body function or body part."
- **EEI SIF Criteria** (2025): the same life-threatening and life-altering definitions, plus a 15-item list. Examples include amputation involving bone, and traumatic brain injury or loss of consciousness over 30 minutes.
- **Campbell Institute** (2020, *secondary*): "a permanent impairment or life-altering state, or an injury that – if not immediately addressed – will lead to death or permanent or long-term impairment."
- **ASTM E2920-26** (2026, *secondary*): life-ending, life-threatening or life-altering consequences.
- **Regulatory lists** (outcome only, no "potential"):
  - RIDDOR specified injuries (fractures other than fingers, thumbs or toes; amputation; sight loss; crush to head or torso; serious burns; loss of consciousness, …).
  - OSHA severe-injury reporting (fatality, in-patient hospitalisation, amputation, loss of an eye).
  - RIDDOR *dangerous occurrences* are the regulatory analogue of "potential": a list of no-injury events that must be reported because of what they could have caused.

The lists are known to be unreliable at the margins. Bayona et al. show that a severed fingertip is classified differently by Cal/OSHA, WorkSafeBC, EEI, HSE and others.

### 1.2 The "potential" counterfactual

There are three anchors:

| Anchor | Who | Question asked |
|---|---|---|
| Credible / single-factor change | Campbell 2018; BST/DEKRA (Martin & Black 2015) | "Could it reasonably and realistically have been a SIF if one thing had been different?" ("inches, seconds and luck") |
| Most likely outcome, fixed by physics | EEI SCL (Hallowell & Spencer 2024) | "Is the most likely outcome of this event a serious injury or fatality?" Energy above about 500 ft-lbs counts as high. "Avoid 'what-if' scenarios … consider only the facts." |
| Company threshold | ICMM 2015 | A material unwanted event is one whose "potential or real consequence exceeds a threshold defined by the company" |

EEI's reason for banning the open "potential" question is the core argument in that model. Asked openly, the answer "is inevitably 'yes' if the analyst is creative enough". Before calibration, practitioners agreed only about 65% of the time; after 40 calibration cases the working group reached about 95% (percent agreement, not kappa).

### 1.3 Controls: the sharpest split

| Position | Who | Effect |
|---|---|---|
| **Ignored**: inherent hazard only | List approaches; plain "high energy present" | Control status doesn't enter the classification |
| **Conditional on absence or failure** | BST precursor; Campbell precursor; MORT/DOE "accident"; EEI PSIF | A SIF precursor or PSIF requires controls to be "absent, ineffective, or not complied with" (BST 2010, *verbatim*) |
| **Credited**: a working control removes PSIF status | EEI SCL ("Capacity", "Success") | High-energy release with a direct control present and no serious injury = **Capacity**, not PSIF |
| **Counted as potential despite the barrier** | Campbell 2018, p. 5: "could have resulted in SIF if not for one factor. This one factor could be a control that was in place…" | A barrier-saved event *is* a PSIF |

So the same event, a worker caught by a properly worn fall-arrest harness, is a PSIF under Campbell and Capacity under EEI. Before the SCL model existed, EEI's working group split 50/50 on exactly this case.

**What counts as a control** also differs:

- **EEI direct control** *(verbatim)*: "specifically targeted to the high-energy source; effectively mitigates exposure … when installed, verified, and used properly; and is effective even if there is unintentional human error." Training, rules, signs, experience and generic PPE (hard hats, gloves) are excluded, and "a deficiency in any component of the system invalidates the entire direct control."
- **ICMM** accepts acts as controls ("an act, object (engineered) or system").
- **BST** speaks of "management controls", the broadest notion of the three.

### 1.4 Energy: harmful, unwanted or uncontrolled?

This is the wording question we started with. The sources map onto it like this:

| Wording | Source | What it builds in |
|---|---|---|
| **Harmful / damaging** in amount and rate | Gibson / Haddon (1961–74): "transfer of energy in such ways and amounts, and at such rapid rates, that … structures are damaged" | Defined by the *effect* on the target; neutral about controls |
| **Unwanted** transfer | MORT (Johnson): "incidents [are] unwanted energy transfers" | Defined by the target's (or the system's) *interest*; neutral about controls |
| **Unwanted … due to absence or failure of barriers** | DOE-HDBK-1208-2012 (MORT descendant): "Accident: An unwanted transfer of energy … that, due to the absence or failure of barriers or controls, produces injury" | Control failure is part of the *definition of the event* |
| **Release** = change of state / loss of control | EEI SCL: "Instance when an energy source changes state when exposed to the work environment"; "a temporary loss of control of the high-energy" | *Uncontrolled* in the operational sense; plus contact with, or proximity to, a worker |
| **Uncontrolled flow of energy** | Attributed to Viner (1991) *(unverified)* | Ties the event to the source–path–target model with controls on the path |

Haddon's countermeasure list frames controls as acting on three things: the *release* (prevent or modify), the *path* (separate people from energy, barriers) and the *target* (make it more resistant). That is the source–path–target model in its original form.

### 1.5 Event or condition

- **Incident-based**: PSIF, Capacity, aSIF.
- **Condition-based**, needing no event: EEI Exposure and Success; the HECA score = Success / (Success + Exposure); ASTM SIFRisk; BST precursors "if allowed to continue". HECA explicitly excludes hypothetical situations.

### 1.6 Event in the literature

None of these definitions contradicts ours. ISO 31000 and ISO 45001 are broader: any change of circumstances, or any work occurrence. EEI's energy release and process safety's loss of containment are special cases of our event.

| Source | Definition *(verbatim unless marked)* | Relation to our event |
|---|---|---|
| ISO 31000:2018, 3.5 | Event: "occurrence or change of a particular set of circumstances". Notes: an event can have several causes and consequences, and can be "something that is expected which does not happen" *(via ERA terminology collection)* | The general class; ours makes the change specific to energy. A control failing to act is not an event for us until the energy does something |
| ISO Guide 73:2009, 3.5.1.3 | Same definition. Note 4: "An event without consequences … can also be referred to as a 'near miss'" | Near miss = event without harm |
| ISO 12100:2010 | Hazardous event: "event that can cause harm"; Note 1, it "can occur over a short period of time or over an extended period of time". Hazardous situation: "circumstance in which a person is exposed to at least one hazard" | **The closest analogue**: their event vs situation is our event vs exposure. We fix the moment as the crossing of a bound |
| ISO 45001:2018, 3.35 | Incident: "occurrence arising out of, or in the course of, work that could or does result in injury and ill health"; Note 2, without injury it "may be referred to as a 'near-miss', 'near-hit' or 'close call'" | Includes the outcome; our event is defined before it |
| HSG245 (HSE, 2004) | Accident: "an event that results in injury or ill health". Near miss: "an event that, while not causing harm, has the potential to cause injury or ill health". Undesired circumstance: "a set of conditions or circumstances that have the potential to cause injury or ill health" *(secondary)* | Undesired circumstance ≈ exposure (a condition); accident and near miss are events |
| RIDDOR 2013, reg. 2(1) | Dangerous occurrence: "an occurrence which arises out of or in connection with work and is of a class specified in" Schedule 2 | A listed subset of events, reportable for their potential |
| OSHA near-miss template | "… no property was damaged and no personal injury was sustained, but where, given a slight shift in time or position, damage and/or injury easily could have occurred" | "Time or position" = the T and L margins |
| EEI SCL (2024) | Energy release: "an instance in time where there is a temporary loss of control of the high-energy, or the high-energy changes state while exposed to the work environment" | Precedent for "the energy changes"; our four verbs spell it out |
| Bowtie top event (UK CAA) | "the point in time which describes the release or loss of control over a Hazard" | Precedent for "at a moment" |
| CCPS Process Safety Metrics v3.2 | Loss of primary containment: "an unplanned or uncontrolled release of material from primary containment" | A special case: an excursion in *where* |
| ICAO Annex 13 (10th ed.) | Accident and serious incident: "The difference … lies only in the result" | Near miss vs SIF actual |
| Haddon (1968, 1980) | Pre-event, event and post-event phases; injury as energy transfer beyond tolerance *(paraphrase)* | Pre-event ≈ exposure; event ≈ event |

**Incursion** has no direct precedent. It extends EEI's "contact or proximity" and the ISO 12100 hazard zone.

## 2. The definition our classifier holds implicitly

Reconstructed from `scripts/sif/zeroshot_sif.py`, `scripts/sif/band_selector.py`, `data/sif/calibration/*.json`, `.claude/plans/sif/SIF-CLASSIFIER.md` and [what-is-sif-potential.md](what-is-sif-potential.md):

| Element | Current implicit rule |
|---|---|
| Unit | One event narrative; the most severe mechanism per event |
| Event | Any event with an identifiable energy transfer mechanism (18 mechanisms, 10 energy types). No requirement that the energy was "released", "unwanted" or reached anyone |
| Counterfactual | **Chance P = 1**: assume the energy reached a person. Outcome is a distribution (P10/P50/P90), not a single worst case |
| Controls | **All engineered controls excluded**, both those that prevent the event (guardrail, LOTO, barriers) and those that limit harm (net, harness, PPE). Test: "was this put there to prevent harm, or is it just what's there?" |
| Included context | Source magnitude (height, mass, speed, voltage, …); carrier geometry (sharp, rotating); default receiving environment (concrete, rebar); body region |
| Reference person | Working-age adult (18–65), random orientation, hard surface by default |
| Severity scale | Mortality-based: P(death) on an AIS-derived scale; "serious injury" = P(death) ≥ 0.10 |
| SIF threshold | P(outcome ≥ serious) ≥ **0.50** → SIF; ≥ 0.10 → ELEVATED; otherwise NON_SIF. Stated as policy, customer-configurable |
| Missing detail | Least severe band ("if the narrative does not contain enough detail … select band 1") |
| Out of scope | Overexertion and abrasion automatically NON_SIF; chronic exposure and illness not modelled |

### Where this sits against the literature

- **Counterfactual: closer to EEI than it looks.** P(serious) ≥ 0.50 is EEI's "most likely outcome is a SIF". Our energy bands do the job of EEI's energy threshold, but continuously and for each mechanism. ELEVATED (≥ 0.10) is closer to the Campbell/DEKRA "could reasonably have been".
- **Controls: closer to Campbell than to EEI.** Because every control is excluded, a fall arrested by a harness scores the same as an unprotected fall. EEI would call it Capacity, not PSIF.
- **Proximity: broader than EEI.** EEI needs a worker in contact or within about 6 ft. Chance P = 1 assumes a person even when nobody was near (a dropped object in an empty exclusion zone).
- **Severity: narrower than LIFE.** A mortality scale captures life-ending and life-threatening injuries well but **under-counts life-altering ones**. An amputated finger, an eye injury or a crushed hand can be permanently disabling at low P(death). Every modern definition includes these.
- **Missing information: facts-only, like EEI.** This matches EEI's "consider only the facts", but contradicts our own [bayesian-priors.md](bayesian-priors.md), which argues that missing detail should fall back to the population prior, not to the least severe case.
- **Energy types: broader than EEI.** EEI's threshold is in physical units (ft-lbs). We also cover chemical, thermal, breathing and electrical energy, each with its own calibration curve. That breadth is a strength, but it needs saying.

## 3. The wording problem: unwanted, unexpected or uncontrolled?

This section is the argument trail. It shows why "unwanted", "unexpected" and "uncontrolled" were dropped: they are imprecise, and the refinements below replace them with exposure, excursion, incursion, event and near miss (see the Glossary). The definition in section 5 uses only the replacement terms.

### "Uncontrolled"

"Uncontrolled flow/release" puts control status *inside* the definition of the event. Three consequences follow:

1. **The narrative must say what the controls were.** Narratives rarely do, so classification depends on what the writer happened to mention.
2. **A barrier-saved event is no longer an event.** A harness that arrests a fall means the release was "controlled", so it falls out of scope. Unlike EEI's Capacity category, it is then not counted at all.
3. **It merges two questions**: "how bad could this be?" and "did the barriers work?". Section 4 argues they should be kept apart.

What "uncontrolled" gets right is the source–path–target structure: energy leaves its source, travels a path and reaches a target, and controls sit at the source or on the path.

### "Unexpected" (unplanned)

"Unexpected" defines the event against the work plan. Three problems:

- **Whose expectation?** The worker's, the planner's or the analyst's?
- **Planned exposure.** Work deliberately done within reach of high energy (live-line work, work at height, traffic management) involves *expected* energy. An event that goes wrong within that plan may not feel unexpected, and yet it is exactly what we want to capture.
- **Intent.** An assault is intended by the attacker. It isn't unexpected in that sense, but it is clearly unwanted by the victim.

### "Unwanted" (unintended)

"Unwanted" defines the event from the point of view of the target, and of the system as designed. It is MORT's original wording and makes no statement about controls. A near miss qualifies: the energy left its intended state, even though it reached nobody.

### Proposed position

**Starting point: define the event as an unwanted release or transfer of energy.** (Refined below: intentional releases need an envelope test.) "Unwanted" here means energy leaving its intended state, containment or path; "release" follows EEI's "change of state".

**Keep control status out of the event definition** and record it as a separate attribute. This keeps the source–path–target model, because that is how controls get described, without making the event depend on them.

### Refinement: intentional releases and the energy envelope

"Unwanted release" breaks on work whose *purpose* is to release energy: blasting in quarries, controlled demolition, live test firing on a range. There the release is wanted. It is made safe by an **exclusion zone** (danger area), a designed envelope that the energy may enter and people may not. Relief devices raise a related question: a dust explosion vent or a pressure relief valve releases energy deliberately, by design, in response to an upset.

What is unwanted in all of these is not the release but **energy and people meeting where the design says they should not**. So the event is better defined against an **intended energy envelope**:

- For **contained energy** (pressure vessel, live conductor, load on a crane), the envelope is the containment and the designed path.
- For **intentional releases** (blast, test round, relief discharge), the envelope is the designed release zone: the blast exclusion zone, the range danger area, the vent or relief discharge area.

**Revised event wording (layer 1).** An event is either:

- (a) energy outside its intended envelope, whether or not anyone was there, **or**
- (b) a person inside an envelope while the energy in it is live or being released (refined to *active* below).

Clause (b) is what catches the person in the exclusion zone during a blast. The release went to plan; the person was where the plan said nobody would be.

Without a gate like this, chance P = 1 would make every routine blast a SIF potential event. That shows the gate is a necessary part of the definition, not a refinement.

There is precedent for this framing in process safety. API RP 754 counts loss of primary containment as a process safety event, and treats a pressure-relief discharge as one only when it goes somewhere unsafe (e.g. liquid carryover, discharge to an unsafe location, shelter-in-place). A relief device lifting as designed counts as a *demand on a safety system* (Tier 3). That is the same distinction between "envelope exceeded" and "control called on" (wording to check against the standard).

### Refinement: energy state, not its history

Every case so far assumes a precondition: **the energy has to be present**. The vehicle has to be moving, the conductor live, the vessel pressurised, the load lifted. EEI's first question ("Was high energy present?") is the same test.

**Proposal: SIF potential is a property of the energy state at the time, not of how that state came about.** It is assessed on the energy actually present (or building), whatever its history. The steps that led there are out of scope for layers 1 and 2 in every event class, but not out of scope altogether:

| Upstream step | Where it belongs |
|---|---|
| Why the circuit was live (isolation missed, wrong circuit isolated, back-feed) | Layer 3: the isolation control failed |
| Why the vehicle was in the pedestrian area (route, signage, segregation) | Layer 3: the segregation control |
| Why pressure rose (blocked outlet, failed cooling, wrong feed) | Layer 3: the control that should have prevented or caught the build-up; plus investigation |
| A *different* energy released along the way (a dropped load hits a valve, which releases pressure) | A separate energy source: assess each one and keep the most serious (EEI's rule) |

So the history matters for **why** and for **which control failed**, never for **how bad**. This keeps layer 2 reproducible: two analysts looking at the same energy state should reach the same potential, however differently they reconstruct the cause.

The envelope then needs **three dimensions**, not one:

| Dimension | The envelope bounds… | Excursion examples |
|---|---|---|
| **Where** (space, L) | Where the energy may be: containment, path, release zone | Flyrock outside the zone; vehicle off its route; steam discharge onto a walkway |
| **How much** (magnitude, M) | How much energy the containment or system is designed for | Overpressure, overtemperature, overspeed, overload, reactivity excursion |
| **When** (state, T) | When the energy should be present at all | Unexpected energisation of an isolated circuit; stored energy left after isolation; a vehicle moving while it should be immobilised |

The "when" dimension covers isolation. The isolated part of a system has an intended envelope of **zero energy**. The energy is still present behind the isolator, so a person working dead is exposed (see the event/exposure refinement below). Unexpected energisation while someone works on it is an excursion in time, even though the energy is exactly where and as large as it would be in normal operation.

### Refinement: over-energy states (build-up without release)

Some events involve no release at all: **energy builds up inside its containment beyond the design bound**. Examples:

- a reactor power excursion, controlled (or not) by control rods,
- a runaway chemical reaction,
- pressure building behind a blocked outlet,
- a turbine overspeeding,
- a crane lifting beyond its rated load,
- a battery starting thermal runaway,
- snow load on a roof, or a dam level rising.

These are excursions in the **magnitude** dimension: clause **(c)** of the event.

**Revised event wording (layer 1).** An event is any of:

- (a) energy outside its intended envelope in space,
- (b) a person inside an envelope while the energy in it is live (refined to *active* in the event/exposure refinement below),
- (c) energy beyond its intended magnitude, or present when its intended state is zero, even if still contained.

**How big is the potential of a build-up?** At the moment it was caught, a build-up may have reached only a modest level. If layer 2 is assessed on the peak actually reached, a reactor excursion stopped by the rods would look harmless. That would be crediting the control, which layer 2 doesn't do.

Consistency with the control-blind rule gives the answer. **Assess the energy the build-up would reach if no control acted, up to the source's inherent physical limit.** That limit is the inventory, fuel or reactant quantity, maximum pump head, failure pressure of the containment, or stored kinetic energy at the maximum possible speed. It is the build-up counterpart of chance P = 1. It is still bounded by physics, so it avoids EEI's concern about "creative" what-ifs: the analyst may not imagine energy that isn't there, only let the energy that is there run to its limit.

**Inherent features versus controls.** Build-ups make it pressing to say where the source ends and the controls begin. The first draft's test was "no failure mode, no demand". Checked against the inherent-safety literature, its two halves match two different categories:

- **"Needs no demand or action"** is how CCPS and the IAEA define **passive** safety: "without the active functioning of any device" (Hendershot, 2012); "does not need any external input to operate" (IAEA-TECDOC-626).
- **"Has no failure mode"** is how the IAEA defines **inherent** safety: "an inherent safety characteristic is not subject to failure of any kind" (IAEA-TECDOC-626 §2.2).

Requiring both halves rightly puts bunds, blast walls, fire insulation and fixed guards on the *control* side. They need no demand, but they can fail. The sources agree on this:

- Kletz (1991): "Passive safety is not the same as inherent safety … Fire insulation is passive, as it does not have to be commissioned, but it is not inherently safe as it can fall off or be removed for maintenance and not replaced."
- Hendershot: a dike "can fail – dikes can crack, leak, be full of water".
- ISO 12100 3.20 defines inherently safe design measures as working "without the use of guards or protective devices".

The research also found two weaknesses in the wording:

1. **"Demand" clashes with LOPA and IEC 61511 usage.** In LOPA a bund *is* demanded (by the spill) and has a probability of failure on demand. The distinction we want is that a source feature acts **all the time in normal operation**, while a passive control is called on only after a deviation.
2. **"No failure mode" is too absolute.** A reactor's negative temperature coefficient is a property of the source, but it depends on the operating state (it can be near zero early in a fuel cycle, and the RBMK's positive void coefficient is the counter-example). Kletz's *limitation of effects* ("limiting the amount of energy available") covers it, and pump dead-head pressure, as magnitude limits rather than elimination. The better test is that the feature **can be lost only by changing the source itself**.

**Revised test (Q1d, answered):**

- A feature is **part of the energy source** when it is a property of the source's own materials, quantity, operating conditions or physics. It is in effect throughout normal operation, without detection, actuation, external power or human action, and it can be lost only by changing the source itself (a different material, a larger inventory, new operating conditions, a changed configuration). Its effect is judged for a named hazard within the stated operating envelope: "inherently *safer*", never absolutely safe (CCPS; IAEA-TECDOC-626). Examples: a reduced inventory, a weaker reagent, a pump's dead-head pressure, a reactor's negative temperature coefficient. It changes the potential (layer 2).
- A feature is a **control** when it is added to act on the source or on the path to people, and it can degrade, be defeated or be overwhelmed while the source stays unchanged. It is layer 3. Controls come in three kinds (the CCPS strategy categories):
  - **active**: they detect and respond. Trips, interlocks, safety instrumented functions, quench and dump systems, load-moment cut-outs.
  - **passive**: they act by their presence once a deviation has occurred. Bunds, guards, blast walls, fire insulation. Relief valves and bursting discs are passive with moving parts (IAEA category C). Gravity-drop control rods on a fail-safe trip are passive execution with active initiation (IAEA category D).
  - **procedural**: they depend on people following a procedure. LOTO, permits, exclusion-zone sentries.

This matches the hierarchy of controls. NIOSH: "Elimination removes the hazard at the source. Substitution is using a safer alternative to the source of the hazard." These change the source and so change the potential; everything below them is a control. It also matches CDM 2015 reg. 9: designers "eliminate, so far as is reasonably practicable, foreseeable risks", and only then reduce or control.

### Refinement: the envelope in Mass, Length, Time

The three envelope dimensions line up with the base dimensions of mechanics, **M, L, T**. That is a useful metaphor, and in places more than a metaphor.

| Base dimension | In the envelope | Varies how |
|---|---|---|
| **T, time** | When the energy is present, and how much is present at each moment | Presence and amount change over time: energised or isolated, build-up, decay, the firing sequence, the duty cycle |
| **L, length** | The distance between energy and target. Once the source point is marked, the envelope's size is a radius or boundary around it | Separation changes as the energy moves (flyrock, vehicle) or the person moves (entering the zone) |
| **M, mass** | The magnitude of the energy | Inventory, charge mass, pressure × volume, voltage, speed |

Energy itself has the dimensions **M L² T⁻²**, so magnitude, distance and rate are not independent. Three established practices already work this way:

- **Haddon** defines harm by energy transferred "in such ways and **amounts**, and at such rapid **rates**": magnitude (M) and rate (T).
- **Radiation protection** uses *time, distance, shielding*: limit T, increase L, put a barrier on the path. Source strength is M. This is the source–path–target model expressed in M, L, T.
- **Blast scaling** (Hopkinson–Cranz) sets the safe distance from the charge mass: the scaled distance Z = R / W^⅓ is constant for a given overpressure. So the size of the envelope in L is a function of M. Electrical limits of approach (distance by voltage), inverse-square fall-off for radiation and noise, and vehicle stopping distance by speed all follow the same pattern.

**What this buys us:**

1. **A completeness check.** An envelope is fully stated only when it has a bound in each of M, L and T. "Exclusion zone 500 m" is incomplete without the charge mass it was sized for (M) and the firing window (T). A narrative that gives only one dimension leaves the other two unknown, not satisfied.

2. **Simpler event clauses.** L is the *relative* distance between energy and target, so clauses (a) and (b) are the same loss of separation in L: an *excursion* if the energy moved out, an *incursion* if the person moved in. Clause (c) is an excursion in M (over-magnitude) or in T (present when it should be zero). So:

   | Clause | Dimension exceeded |
   |---|---|
   | (a) energy outside its envelope | L (energy moved) |
   | (b) incursion into a danger zone, or a change of state while a person is exposed | L (target moved, or energy moved onto an exposed target) |
   | (c) over-magnitude | M |
   | (c) unexpected presence, e.g. energisation of an isolated system | T |

3. **Dimensionless margins.** Each bound can be written as a ratio to its design value:
   - *m* = actual magnitude / design magnitude,
   - *l* = actual separation / required separation,
   - *t* = presence outside the intended time window (0 or 1, or a fraction of it).
   
   An excursion or incursion has happened when m > 1, l < 1 or t > 0. The ratios also say *how far* it went: flyrock at 1.1 × the zone radius is not the same as at 3 ×. They work across energy types, as dimensionless groups do in physics. This is a possible future field for layer 3, or for control-state analytics.

4. **Controls classified by dimension.**
   - **M controls** reduce or cap magnitude: smaller inventory, isolation (M → 0), relief valves, load cut-outs, speed limiters.
   - **L controls** keep or restore separation: exclusion zones, segregation, barriers, guarding. Shielding is an L control that absorbs energy on the path rather than adding distance.
   - **T controls** limit when, or how long: permits and firing windows, interlocks, exposure time limits, soft-start and controlled-descent rate limiters.
   
   That gives layer 3 a structure that works across energy types.

5. **T also sets severity, through rate and dose.** In layer 2, severity depends on M and on how fast the energy is delivered (T): a sudden release against a slow leak, the stopping distance in a fall. It also depends on dose (rate × duration) for thermal, chemical, radiation and noise exposure. The calibration bands already include some of this ("speed of release", "duration"). The M, L, T framing says it should be explicit for every mechanism.

**Limits of the metaphor.** Electrical, thermal and chemical energy need other SI base quantities (current, temperature, amount of substance). So "M" here means *magnitude in the energy's own units*, not literally mass. L also does two jobs: separation distance in the envelope, and contact area at the target (Hallowell's intensity, J/cm², is L⁻²). The first belongs to the envelope (layers 1 and 3); the second is a carrier property in layer 2.

"Line of Fire", the shop-floor phrase for keeping out of the energy's path, is essentially the L dimension on its own. Appendix A sets it against this definition.

### Refinement: event, exposure, excursion and incursion

Earlier drafts called any departure from the envelope a "breach" and treated every breach as an event. That has two problems.

**"Breach" has legal overtones.** It suggests breach of duty or statutory breach, and so fault. In a product that also handles legal registers, readers would take it that way. The SIF definition should describe what the energy did, not whether anyone was at fault.

**It hid an inconsistency.** Clause (b) said "a person inside an envelope while the energy in it is live". A suspended load is live (stored gravitational energy), so a person standing under it would be an *event*. Yet the cases (Appendix A, and HECA's "Exposure") treat that as a *condition*: nothing has happened yet. The same goes for a person inside the approach distance of a live conductor without contact, or a missing guardrail with nobody falling.

The fix separates two things that "breach" ran together: **whether energy and a person are together**, and **whether anything happened**.

**Exposure is the presence of energy and a person in the same time and space, with the person within the energy's reach, assessed without regard to controls.** Exposure is a condition: it has a duration, not a moment.

- **Energy counts as present if it is held, stored or available to be released.** That includes the fuel in a parked vehicle that could move, the supply held behind an isolator, a person's own height on a platform, and a suspended load.
- **Only elimination removes it**: the supply disconnected and removed, the vessel drained and vented, the load landed, the vehicle removed.
- **Isolation, immobilisation, guarding and guardrails are controls.** They have failure modes, by the test in the build-up refinement above. They don't end the exposure; they are its control state.

| Term | Meaning | Examples | Not this |
|---|---|---|---|
| **Exposure** (a condition) | Energy and a person present together in time and space, the person within reach, controls disregarded | Working on a platform at height, guarded or not; working dead behind an isolator; a pedestrian in a lane with parked, fuelled vehicles; standing under a suspended load | A missing handrail with nobody on the platform: that is a *control state*, not an exposure. A circuit whose supply has been removed: eliminated |
| **Excursion** | Energy goes beyond its envelope in L (outside its space), M (beyond its magnitude) or T (present where it should be zero) | Flyrock beyond the zone; overpressure; a re-energised isolated circuit | |
| **Incursion** (an event) | A person **enters a danger zone** while the energy is live: an area within reach of the energy that the plan keeps clear of people (exclusion zone, segregated route, drop zone, a machine's danger zone). The entry is the event; remaining there counts towards exposure, in the stratum where that control is absent. Entry under isolation is not an incursion | Walking into a segregated vehicle route; reaching into a running conveyor's danger zone; entering a blast zone | Planned exposure, e.g. a rigger at height doing the task |
| **Event** | Something happens at a moment: the energy **changes state** (release, transfer, build-up, energisation), producing an excursion or reaching an exposed person, or a person makes an **incursion** | The load drops; the vehicle reverses toward the pedestrian; the blast fires with someone in the zone; the reactor power rises; a hand goes into a running machine | Vehicles moving past a pedestrian at a steady clearance in a shared area: continued exposure |

**Exposure is control-blind**, like layer 2 and chance P = 1. Controls decide how an exposure is *classified*, not whether it exists. That gives the condition-side counterpart of PSIF and Capacity:

| Exposure + control state | EEI/HECA term |
|---|---|
| Direct control effective (guardrail intact, isolation verified) | **Success** |
| Direct control absent or failed (no guardrail, isolation unverified) | **Exposure** (EEI's sense) |

**Terminology clash.** EEI uses "Exposure" for the narrower case: a high-energy hazard *without* a direct control. Our exposure is the wider, control-blind condition, and EEI's Exposure is our exposure plus an absent or failed control. Either keep "exposure" and always qualify EEI's term ("uncontrolled exposure"), or choose another word for ours (e.g. "co-presence"). Q1f.

**Stored and active energy** still describe the energy usefully (held, contained, suspended or isolated; or moving, flowing, being released), echoing potential and kinetic energy. They no longer separate exposure from event, though. A person near a moving vehicle is exposed; it becomes an event when the vehicle changes state (swerves, reverses) or the person makes an incursion into its danger zone.

**Correction to v0.2.** v0.2 said a pedestrian in a route with every vehicle parked and immobilised "is not even an exposure". That was wrong: the fuel is available energy and immobilisation is a control, so it is an exposure with an effective control. The same reasoning reverses v0.2's treatment of successful isolation in 4.1. Working dead is an exposure, and isolation is its control.

Both "excursion" and "incursion" are already in use in neighbouring fields:

- **Excursion** is the process-safety term for leaving the safe operating envelope or limits (CCPS usage, to check).
- **Incursion** is the aviation term for an unauthorised entry into a protected area (runway incursion, ICAO usage, to check).

**Event clauses restated:**

- (a) energy changes state and goes **outside its envelope in space** (excursion in L), whether or not anyone is exposed,
- (b) a person makes an **incursion** into a danger zone, or the energy changes state while a person is exposed (the load drops near a rigger; a vehicle reverses toward a pedestrian in a shared area),
- (c) energy **builds beyond its magnitude, or appears where it should be zero** (excursion in M or T).

Exposures are observable before anything happens (HECA, BST's precursor "if allowed to continue", Appendix A's line-of-fire condition). Events are what the classifier reads in incident narratives.

**When the target moves.** Earlier wording ("the energy converges on the person") was imprecise when it is the *person* who moves, toward a moving vehicle or into running machinery. "How near is near?" had no answer. Making the **incursion** the event gives one: the entry into the danger zone, whose boundary the plan defines (exclusion zone, segregated route, machine guarding perimeter; ISO 12100 3.11 "hazard zone / danger zone: any space within and/or around machinery in which a person can be exposed to a hazard"). It works the same for vehicles and machinery. Where the plan defines no zone, only a change of energy state can make the event. Who should define zones is parked (Q1a).

**Near miss.** That gives a definition: **a near miss is an event (a change of energy state, an excursion or an incursion) in which nobody was hurt.** Exposure alone is not a near miss. Common definitions describe a near miss as an event where "given a slight shift in time or position" injury could have occurred (NSC/OSHA usage, to check). That is a statement about the T and L margins at the moment of the event. A **PSIF** is a near miss with SIF potential, qualified *protected*, *unprotected* or *unverified* by its control state.

### Refinement: exposure as the denominator of risk

"Exposure" has a long-established meaning in risk, and it anchors ours. The widely used form of risk is a product of three terms:

> **Risk = exposure × chance per unit of exposure × consequence**

- **Exposure** is measured in units that fit the context: hours of driving per year, take-offs and landings per year, lifts, blasts fired, confined-space entries, isolations. It is a **quantity**, not just a condition.
- **Chance** is a *rate*: the probability of a harmful event per unit of exposure (per hour, per take-off, per lift).
- **Consequence** is the severity of the outcome, given the event.

**Micromorts** make this concrete (Howard, 1980; Spiegelhalter & Blastland, 2013). A micromort is a one-in-a-million chance of death, quoted per unit of an activity, such as per mile driven, per parachute jump, or per general anaesthetic. Process safety uses the same structure as the **Fatal Accident Rate** (FAR: fatalities per 10⁸ exposed hours; Kletz). Even TRIR is one: its 200,000-hour denominator is a generic exposure. Illustration, figures to check: at about 1 micromort per 250 miles of UK driving, **1,000 hours a year** at an average 30 mph is about 30,000 miles, or about 120 micromorts a year (roughly 1.2 × 10⁻⁴).

**How this anchors the definition:**

1. **Our exposure is the risk formula's exposure.** Co-presence of energy and person, assessed without regard to controls, is exactly what the denominator must be. Controls belong in the *chance per unit* term, not in the denominator: a guardrail doesn't reduce the hours spent at height, it reduces the chance of a fall per hour. If exposure credited controls, a control would be counted twice. That is why exposure has to be control-blind.

2. **Exposure has a T dimension built in.** As a quantity, exposure is co-presence accumulated over time: the person-hours (or person-episodes) a person spends within reach of the energy. In M, L, T terms it is the time integral of "person within L of energy with M > 0", summed over people. For discrete tasks (a lift, a blast, an entry) the natural unit is the count of episodes, not hours.

3. **Each term maps onto a layer:**

   | Risk term | Our framework |
   |---|---|
   | Exposure (units) | The condition of section 3, now measured: person-hours or episodes within reach, per energy type |
   | Chance per unit exposure | Event rate per unit exposure: how often energy changes state near an exposed person, or a person makes an incursion. Driven by **controls (layer 3)**. This is the left side of the bowtie, which SIF potential deliberately sets aside (chance P = 1) |
   | Consequence | **Inherent SIF potential (layer 2)**: the outcome distribution given the event, P(serious) |

   So, for one energy type: **expected SIF = exposure × event rate per unit × P(serious | event)**. The classifier supplies the last term.

4. **The classifier's severity scale already speaks micromorts.** Layer 2 is calibrated in P(death), so E[P(death) | event] × event rate × exposure gives expected micromorts directly. The life-altering gap noted in section 2 applies here too: micromorts count deaths only. Spiegelhalter's *microlife* (a half-hour of life expectancy lost) is one way to weight non-fatal harm (to check).

5. **Rates need energy-specific denominators.** EEI advises against a PSIF *rate*. One reason is that the only denominator usually available is total hours worked, which mixes office hours with hours at height. With exposure anchored per energy type (hours at height, lifts, vehicle-km, isolations), a PSIF rate per unit of *that* exposure is meaningful. Without it, it isn't.

6. **Incident data gives numerators only.** Narratives tell us about events. Exposure denominators have to come from elsewhere: timesheets by task, lift logs, permits, telematics, blast records. That is a data requirement, not something the classifier can infer.

7. **Count-per-exposure data suits the Bayesian approach** in the validation work: event counts per unit of exposure are naturally modelled as Poisson with a Gamma prior on the rate.

This also settles Q1f in favour of keeping "exposure". Ours is the standard risk-formula meaning; EEI's "Exposure" (a hazard without a direct control) is the specialised use, so it should be the one that gets qualified.

### Refinement: exposure strata, demand and incursion

**Is incursion time the same as exposure?** No. Take an operator using a guarded machine.

- **Exposure is control-blind.** Without the guards, the moving parts would be within the operator's reach, so the whole time they use the machine is exposure. "Within reach" can be anchored in body-reach distances to a hazard zone (ISO 13857 safety distances, to check). Someone at the far end of the workshop is not exposed.
- **The guard is under constant demand.** A fixed guard on a running machine is a passive control in *continuous* demand for as long as someone is within reach (IEC 61508 distinguishes low-demand, high-demand and continuous modes, to check). It reduces the chance of harm per hour of exposure; it does not reduce the exposure.
- **Opening the guard while the machine is energised is an incursion.** The entry is the event, and each entry counts. The time spent inside counts towards exposure, in the stratum where the guard is absent. Running clean is the extreme case: the energy is active, the person is inside, and the main control is gone.
- **Opening the guard under isolation is not an incursion.** A danger zone's envelope has a T condition: kept clear *while energised*. A planned entry under LOTO is exposure with isolation as the control in demand. The energy is still present behind the isolator.

So **one control-blind exposure, split into strata by control state**:

| Stratum | Example | Control in demand | Chance per unit exposure |
|---|---|---|---|
| Guards in place | Normal operation | Guard (continuous demand) | Lowest |
| Isolated, guard open | Clearing a blockage under LOTO | Isolation | Low if verified |
| Energised, guard open | Running clean; reaching in | None: an incursion | Highest |

Risk then sums over the strata:

> **Risk = Σ (exposureₛ × chance per unit of exposureₛ × consequence)**

The total exposure is unchanged by the guard. The split shows *where* the risk sits: usually in the short, non-routine strata such as maintenance, clearing and cleaning, which Manuele identified as where SIFs concentrate. Incursion counts and incursion time are the measures of the highest stratum.

The earlier refinements above are kept as the argument trail. Section 4 onwards uses these terms.

### Refinement: the target has an envelope too

The M, L, T envelope was first applied only to the energy. That left two problems:

- **"Incursion" was spatial only.** Yet it is often a matter of time. Entering a firing range tomorrow, when no tests are planned, is not an incursion. It is separation in time: a scheduling control.
- **Event tables built on the energy alone came out sparse and lopsided.** In every version, one side carried most of the cases.

**The fix: the people (the target) have an envelope in the same three dimensions.**

| | **Energy** | **People (the target)** |
|---|---|---|
| **Where** | Where the energy is, or can reach | Where the people are |
| **When** | When the energy is live: energised, operating, the firing window | When the people are there: the shift, the schedule, an entry |
| **How much** | The magnitude of the energy | How many people: one person, a team, a public crowd |

**An event is a change, on either side, that brings the two envelopes together.** If the energy changes, it is an **excursion**; if the people change, it is an **incursion**. Each can happen in any of the three dimensions:

| Dimension | **Energy changes** (excursion) | **People change** (incursion) |
|---|---|---|
| **Where** | Released or moves beyond its envelope: flyrock; a load swings off its path; a person at height falls | People enter where the energy is: a segregated route; a running machine |
| **When** | Becomes live when it should not be: a back-fed circuit; a machine restarts; firing starts early | People present when the energy is live: on the range in the firing window; in the blast zone at firing |
| **How much** | Builds up beyond design: overpressure; overload; a runaway reaction | More people than planned: a crowd in the drop zone; a team where one person was planned |

**Exposure** is the envelopes overlapping with nothing changing. **Separation controls** keep the envelopes apart in where (distance, barriers) or when (scheduling, permits, firing windows).

**Event as the materialised chance term.** In risk = exposure × chance per unit of exposure × consequence, the event is the point where the **chance** term materialises: the preventive side, the left of the bowtie, has not kept energy and people apart. That makes the event the top event. SIF potential then assesses the **consequence** term, with chance P = 1. Earlier wording ("beyond what the plan allows") pointed at the same thing, but too cryptically.

**Target magnitude doesn't change SIF potential**, which is judged per person. It multiplies the exposure (person-hours) and the number of people who could be harmed in one event. That is the multiple-casualty, societal-risk view, and ICMM's material unwanted event.

This supersedes the event clauses (a)/(b)/(c) and the earlier event tables. They stay above as the argument trail.

### Refinement: the event as shrinking separation

The "energy and people come together" wording of v1.4 did not fit three cases that the event definition must cover:

1. an excursion into **unoccupied** space, where someone could have been present (the counterfactual);
2. an excursion when someone was **closer** in space and time: the near miss;
3. the event **leading to harm or loss**, which is what risk usually wants to measure.

A first attempt to fix this added a probability ("Contact P": the chance the energy reaches a person, given the event). Needing to adjust P was a sign that the event itself was not exact.

**The fix: the event is a shrinking of the separation margin.** Energy and people are kept apart by a margin: distance (where), time (when) or headroom to the design limit (how much). An event is a *happening*, not a condition: that margin shrinks toward zero, because the energy changes (excursion) or the people do (incursion). It is in effect a separation (control) failure, whether or not a control was meant to provide the separation. When someone is struck by flyrock, the distance is literally zero.

The three cases are then positions on one scale, the event's **closest approach**:

| Closest approach | Case |
|---|---|
| **Wide**: nobody near | Excursion to unoccupied space (RIDDOR's dangerous occurrence is the regulatory analogue) |
| **Close**: margin small, not zero | Near miss: "a slight shift in time or position" (OSHA) |
| **Contact**: margin zero | The energy reached a person, with or without harm |

**The counterfactual, restated.** "Chance P = 1" becomes **separation to zero**: assess SIF potential as if the margin had gone to zero. It is the same counterfactual, now expressed in the same terms as the event, and it no longer collides with the chance term of the risk equation.

Closest approach is the margins idea (m, l, t) applied to a single event. Recording it qualitatively (wide, close, contact) is part of the definition; numeric margins stay parked for the design specification (Q1e).

### Refinement: live energy and the line of fire

**The challenge.** A person enters the exclusion zone during a heavy lift. The person moved and the load didn't, so the separation shrank from the people's end. That is an incursion, and so an event. Yet v1.5 of the definitions document gave "standing under a suspended load" as an example of *exposure*, which is a condition and not an event.

**A wrong turn.** The first response was a current-path test: an incursion counts as an event only if the energy could reach the person by continuing what it was already doing. A held load would need to drop as well, so entering beneath it would be exposure. That test was rejected. Separation can shrink from either end, which is the agreed meaning of an event. The judgement is about *how far* the margin was eroded, which is the closest approach, not *whether* there was an event.

**The distinction that holds: live versus isolated.**

- **A suspended load is live.** Only the crane and rigging, which are controls that can fail, hold its potential energy. Its drop path is a **line of fire**, which is why the exclusion zone exists. Entering it is an incursion: an event. The closest approach records how far in the person went (wide at the zone edge; close under the load).
- **An isolated machine has no line of fire.** The energy is present behind the isolator, so the person is exposed, but nothing live points at them. Opening its guard is exposure under isolation, not an incursion.
- **Tag-liners are planned exposure.** They are within reach of the lift but positioned at the edge, out of the drop path, with the tag-line length as their separation. There is no event unless something changes: the load swings, or they are drawn in under it.

**Consequences:**

- Two new terms. *Live*: energy in its working state, not isolated, with a line of fire to people. *Line of fire*: the path live energy takes as it moves, or would take if released. Danger zones are drawn around it. This brings the shop-floor term (Appendix A) into the definition itself.
- "Standing under a suspended load" was a wrong exposure example. Getting there is an incursion; staying there counts as exposure.
- *Present* and *live* now do different jobs. Present (held, stored, available, isolated) decides exposure. Live (present and not isolated, with a line of fire) decides whether an entry is an incursion.

### Refinement: cycles, breaking the circle

Two questions from v0.4 turn out to be one problem.

- **Upstream steps.** Putting "how the energy came to be present" into layer 3 is **circular**. A layer-3 control failure in one cycle (the cooling pump trips) is what starts layer 1 in the next (the reaction runs away).
- **Build-up limits.** A build-up's "uncontrolled limit" often depends on a control rather than on physics. The inventory in a reactor is set by a feed control, and Kletz's point is that inventory is a design choice.
- **Bridge conditions** add to it. The fuel that gives a vehicle its motion can burn. The battery that drives an electric vehicle can go into thermal runaway. Motion becomes heat in braking.

**Proposal: analyse in cycles.**

- A **cycle** is one energy **source** taken through the three layers once: envelope, exposure, event, potential and control state. Per source is the default boundary. A source with several energy types (a vehicle has motion and fuel) is one source with several cycles only where a bridge is in scope.
- Cycles **chain** through **bridge conditions**. The control failure or event of cycle A sets the starting state of cycle B. Energy conversion links cycles of different energy types (chemical → motion, chemical → thermal, motion → thermal).
- **Within a cycle, layer 2 is control-blind. At the cycle's boundary, the starting state is taken as found.** The upstream controls that set it (a feed control limiting inventory, a governor limiting speed) belong to the upstream cycle. They are not set aside unless that cycle is opened.

That breaks the circle: no control is both set aside and relied on within the same cycle.

**The build-up limit, restated.** The uncontrolled limit is the limit *within the cycle under examination*, starting from the state as found (the batch as charged, the speed as governed). Asking "what if the feed control had failed too?" opens a further cycle, which must be labelled as a **counterfactual**.

**Scope is the investigator's choice, stated up front.** This works like the **scoping statement** of a HAZID or HAZOP study, which frames the enquiry before it starts: the study boundary, the nodes, what is in and out. For a SIF assessment the scoping statement names:

- the sources (cycles) in scope,
- the bridges to follow,
- whether counterfactual cycles are included.

There are two kinds of cycle:

- **Evidential cycles**: those the facts show happened, where the narrative evidences the bridge (the cooling did fail; the battery did ignite). These are precursors chained by evidence.
- **Counterfactual cycles** (what-ifs): extensions the facts don't show (had the relief also failed; had the fuel ignited). They are legitimate in investigation and risk assessment, and are always labelled *counterfactual*.

**Default for the classifier.** The implicit scoping statement is: per source, evidential cycles only, no counterfactuals. Each cycle is assessed separately and the most serious is kept (EEI's per-energy-source rule). A counterfactual cycle needs an analyst, and appears in output with the label `counterfactual`. This keeps EEI's "facts only" discipline at the cycle boundary while staying control-blind within each cycle.

| Narrative | Cycle A | Bridge | Cycle B | Default scope |
|---|---|---|---|---|
| Cooling-water pump trips on a batch reactor; temperature rises; the quench works | Cooling system (mechanical): control fails | Loss of cooling → heat accumulates | Runaway reaction (chemical → thermal, pressure). Limit = reactant inventory *as charged* | A and B, both evidential |
| The same, and the analyst asks what if the batch had been overcharged | — | — | Inventory beyond the charge | Counterfactual: analyst only, labelled |
| Forklift strikes racking; no fire | Motion | — | A fuel fire would be counterfactual | A only |
| Electric vehicle damaged in a collision; smoke, then thermal runaway in the yard | Motion | Damage → cell short circuit | Battery (chemical → thermal) | A and B, both evidential |
| Isolated circuit back-fed from a portable generator | Generator running outside the plan | Back-feed path | Work circuit energised (excursion in T) | A and B, both evidential |

Mapping the whole chain is causal analysis (bowtie, MORT). SIF classification only needs the cycles that carry SIF potential.

## 4. Proposal: three layers, not one definition

Treat SIF as three separate questions, each answerable on its own:

| Layer | Question | Source–path–target | Who answers |
|---|---|---|---|
| **1. Event** | Did something happen: did the energy change state so that (a) it went outside its envelope in space, (b) a person made an incursion into a danger zone, or the energy changed state while a person was exposed, or (c) it went beyond its magnitude, or appeared where it should be zero? Energy and a person together with nothing happening is an *exposure* (a condition), not an event | Source changes state, builds up, or target enters the path | Narrative (classifier Pass 1: mechanism, energy type). The envelope gate is not yet implemented |
| **2. Inherent SIF potential** | If that energy had reached a person with nothing protective between them, what is the outcome distribution, and is a serious outcome more likely than not? For a build-up, assess the energy it would reach with no control acting, up to the source's inherent limit | Source (incl. inherent features) + carrier + default environment + target; **controls set aside** | Classifier (bands → P(serious)) |
| **3. Control state** | Which controls acted on the source, the build-up or the path, and did they work? (effective / present but failed / absent / not stated). Upstream causes are recorded here | Source isolation, build-up controls (trips, relief, rods), path | Separate extraction; often "not stated" |

Combining layers 2 and 3 recovers the vocabulary of the literature without changing what the classifier does:

| Inherent potential | Control state | Outcome | Category (EEI term) |
|---|---|---|---|
| SIF | any | Serious injury | **SIF actual** (HSIF) |
| SIF | absent / failed | No serious injury | **PSIF (unprotected)** (EEI: PSIF; BST: precursor) |
| SIF | effective direct control | No serious injury | **PSIF (protected)** (EEI: Capacity): the control did its job. See 4.4 |
| SIF | not stated | No serious injury | **PSIF (unverified)**: our current SIF flag |
| not SIF | any | any | Low-severity (or LSIF if seriously injured) |

The same split applies to **exposures**, where nothing has happened. That is how HECA observes work:

| Exposure (no event), SIF-level energy | Control state | Category (EEI/HECA term) |
|---|---|---|
| Person within reach | Effective direct control | **Success** |
| Person within reach | Absent / failed | **Exposure** (EEI's narrower sense) |
| Person within reach | Not stated | Unverified exposure |

What this buys us:

- **The classifier stays control-blind.** That is intended, and it matches the narratives, which rarely say much about controls.
- **The Campbell/EEI split becomes a reporting choice**, not a definitional one. "All PSIF" (Campbell) and "PSIF (unprotected)" (EEI) are two views of the same data. Section 4.4 goes further: credit for controls belongs in a residual outcome distribution, not a binary label.
- **It matches the simulator split**: Product 2 models layer 3 (outcome-P controls on the path).
- **It gives a HECA-style measure** when controls are known: the share of SIF-potential events with an effective direct control.
- **It slots into the risk formula** (see the exposure-as-denominator refinement): exposure × event rate per unit (layer 3 drives the rate) × P(serious | event) (layer 2).

### 4.1 Where does the source end and the path begin?

The current test ("was this put there to prevent harm?") works for most cases but leaves three ambiguous:

- **Containment.** A pipe wall, cable insulation or a vehicle body is part of the source system. *Proposal:* containment defines the energy's intended state, and the release is the escape from it. Containment is never counted as a "control" that excludes the event.
- **Isolation (LOTO, de-energisation).** Isolation is a **control**: it has a failure mode (wrong isolator, back-feed, an isolator switched back on). The energy behind it is still present, so working dead is an *exposure* whose control state is "isolation effective". Only **elimination** (supply disconnected and removed, system drained and vented, stored energy discharged) changes the source and ends the exposure. *(v0.2 treated successful isolation as "no energy present". That is withdrawn.)*
- **Evasive action, luck, "nobody there".** These are not controls; EEI excludes acts and relies on error-tolerant barriers. *Proposal:* they leave potential unchanged. This is the "one draw from the distribution" reasoning already in the design.

Worked boundary cases:

| Case | Layer 1 event? | Layer 2 potential | Layer 3 control state |
|---|---|---|---|
| Fall from 6 m arrested by a harness | Yes (fall started) | SIF | Effective direct control → PSIF (protected) |
| Worker at 6 m behind an intact guardrail, no slip | No event; **exposure** (person at height) | SIF (if a fall occurred) | Guardrail effective → EEI "Success" |
| Guardrail missing, worker at 6 m, no fall | No event; **exposure** | SIF (if a fall occurred) | Control absent → EEI "Exposure"; BST precursor |
| Scaffold board falls 20 m into an empty exclusion zone | Yes | SIF (chance P = 1) | Exclusion zone effective? → PSIF (protected) or PSIF (unprotected) |
| Electrician works on a conductor that LOTO has de-energised | No event; **exposure** (energy held behind the isolator) | Per voltage band | Isolation effective → EEI "Success" |
| LOTO applied but stored hydraulic pressure releases | Yes | per band | Isolation failed → PSIF (unprotected) |
| Struck on the hard hat by a 2 kg tool from 10 m | Yes | per band (head) | Hard hat is generic PPE, not a direct control (EEI) |
| Near miss with a reversing vehicle; worker jumped clear | Yes | per band | Evasive action is not a control → PSIF (unprotected) |

### 4.2 Intentional releases and relief devices

Using the envelope wording from section 3:

| Case | Layer 1 event? | Layer 2 potential | Layer 3 control state |
|---|---|---|---|
| Quarry blast fired as planned; zone clear; no flyrock outside the zone | No: release stayed inside the envelope, nobody inside | n/a | n/a (HECA "Success" if observed) |
| Quarry blast; flyrock lands beyond the exclusion zone, nobody there | Yes (a): energy outside the envelope | SIF (fragment mass × velocity) | Zone sizing or blast design failed → PSIF (unprotected) |
| Controlled demolition; a person is found inside the exclusion zone after firing, unhurt | Yes (b): blast fired while a person was exposed (incursion) | SIF | Zone clearance and sentries failed → PSIF (unprotected) |
| Blast halted at the countdown because a person was seen in the zone | No event; **exposure** (incursion into a charged blast zone) | SIF | The clearance check worked; the zone control had failed → an incursion caught before firing |
| Live test firing; a range user enters the danger area during firing | Yes (b) | SIF | Range control failed → PSIF (unprotected) |
| Live test firing; a round or ricochet lands outside the danger area | Yes (a) | SIF | Danger-area design or weapon/ammunition failure → PSIF (unprotected) |
| Misfire: an unexploded charge or round remains after firing | Stored energy left where it was not meant to be (excursion in T). An **exposure** for anyone within reach; an event if it fires or is disturbed | Per charge / round | Misfire procedure is the control; handling the misfire is high-energy work |
| Dust explosion in a silo; the explosion vent opens and discharges to a safe area | Yes: the deflagration is an excursion (it escaped containment) | SIF | Vent = effective mitigating control → **PSIF (protected)** |
| Dust explosion; the vent discharges into an occupied area or walkway | Yes | SIF | Vent worked on the vessel but its discharge was outside a safe envelope → PSIF (unprotected) |
| Boiler overpressure; the relief valve lifts and discharges to a safe point | Yes: the overpressure is an upset, the containment envelope was challenged | SIF (vessel rupture energy) | Relief valve = effective direct control → **PSIF (protected)**. API RP 754 would call this a demand on a safety system |
| Relief valve lifts and discharges steam onto a walkway | Yes (a): the relief discharge is outside its intended envelope | per band (thermal / pressure at the discharge) | Relief design or discharge piping failed → PSIF (unprotected) |
| Relief valve fails to lift (blocked or gagged), but the vessel holds | Yes: overpressure beyond design | SIF | Direct control failed; the vessel margin held, which is luck rather than a control → PSIF (unprotected) |
| Planned blowdown or venting under a permit | No: inside the envelope | n/a | n/a |

These cases expose three points that the first table didn't:

1. **Relief devices are controls, not the event.** The upset (overpressure, deflagration) is the event; the relief device is a mitigating control that redirects energy into a safe envelope. A relief that works is PSIF (protected) (EEI: Capacity). A relief whose own discharge reaches people is a second excursion.
2. **Exclusion zones are separation controls on the path.** Whether one counts as an EEI *direct control* is unsettled. It is targeted and can be effective, but it depends on people (sentries, clearance checks), and EEI excludes controls that fail with unintentional human error. This feeds Q4.
3. **Planned high-energy work needs the envelope gate**, or chance P = 1 floods the SIF class. For the classifier this means Pass 1 must recognise "planned release, nobody inside the envelope" as no event, which it currently doesn't attempt.

### 4.3 Energy state and build-up cases

Clause (c) of the event, with build-ups assessed at their uncontrolled limit:

| Case | Layer 1 event? | Layer 2 potential | Layer 3 control state |
|---|---|---|---|
| Reactor power excursion; control rods insert (scram) and power falls | Yes (c): magnitude beyond design | Assessed at the excursion's uncontrolled limit, which for a reactor is set by inherent feedback and fuel inventory. The negative temperature coefficient is part of the source | Rods / scram = effective direct control → **PSIF (protected)** |
| Reactor excursion; rods fail to insert, but inherent feedback limits power | Yes (c) | As above: the inherent feedback is already in the potential | Direct control failed → PSIF (unprotected) |
| Runaway reaction in a batch reactor; quench or dump system works | Yes (c) | Reactant inventory → vessel rupture energy | Quench = effective control → **PSIF (protected)** |
| Runaway reaction; relief undersized; vessel ruptures, nobody hurt | Yes (c), then (a) | SIF | Relief failed → PSIF (unprotected) |
| Crane lift beyond rated load; the load-moment indicator cuts out the hoist | Yes (c): overload | Collapse of crane and load | LMI = effective control → **PSIF (protected)** |
| Crane overload with the LMI overridden; no collapse | Yes (c) | Collapse of crane and load | Control defeated → PSIF (unprotected) |
| Turbine overspeed; the overspeed trip works | Yes (c) | Disc burst at the inherent limit (steam supply) | Trip = effective control → **PSIF (protected)** |
| Electrician working on an isolated circuit; it is re-energised from another feed | Yes (c), "when": energy present when the intended state was zero | Per voltage band | Isolation failed (back-feed not identified) → PSIF (unprotected) |
| Hydraulic system isolated; trapped pressure remains in an accumulator | No event; an **exposure** for anyone working on it (energy held in the accumulator). An event (c) if the pressure releases | Per pressure band | Isolation incomplete → PSIF (unprotected) |
| Vehicle moving within its designated route, no pedestrian present | No: inside the envelope | n/a | n/a |
| Pedestrian walks into a segregated vehicle route while vehicles move | **Exposure** (incursion). The entry is itself an event (b): an incursion. With no contact it is a near miss | Per speed and mass band | Segregation failed → PSIF (unprotected) |
| Battery store; one cell starts thermal runaway, detected and isolated | Yes (c) | Store inventory (cascade to adjacent cells) | Detection and isolation = effective → **PSIF (protected)** |
| Roof snow load beyond design; no collapse | Yes (c), slow build | Collapse | No control acting (snow clearing not done) → PSIF (unprotected) |

Three points from these cases:

1. **Upstream causes never change the potential.** The re-energised circuit scores the same whether the back-feed came from a generator, a wrong label or a contractor. The cause goes into layer 3.
2. **Most build-up controls are trips and relief devices**, so build-up events with working controls usually come out as **PSIF (protected)** (EEI: Capacity). That is the process-safety idea of a "demand on a safety system".
3. **Slow build-ups have lead time.** Lead time affects whether people can get away (chance P), not how bad the outcome is if they don't. By the chance P = 1 rule it stays out of the potential. Evacuation is a control (layer 3).

The last three cases in 4.1 don't depend on the layer model. They depend on which control definition we adopt (EEI direct control, or something broader). Open question Q4.

### 4.4 Controls in the outcome distribution (answering Q2)

Q2 asked whether to credit working controls (EEI's "Capacity") or to count barrier-saved events as PSIF (Campbell), and which should be the headline. Two points change the question.

**"Capacity" is a misleading word.** In Safety-II and resilience engineering (Hollnagel; Woods's "adaptive capacity"), and in Human and Organizational Performance (Conklin: the capacity to fail safely), capacity is a property of people and systems: the ability to adapt and to absorb surprise. EEI uses the word for a single barrier that worked in a single event. That is much narrower, and readers from either tradition will hear the other meaning. EEI's HECA does draw on HOP; the Safety-II and HOP wording is still to check. This draft therefore makes PSIF the umbrella and qualifies it by control state: **PSIF (protected)** for EEI's Capacity, **PSIF (unprotected)** for EEI's PSIF, and **PSIF (unverified)** when the control state is not stated. *Protected* reuses the defined term "protective control" and states a physical fact (something stood between the energy and the person). It was preferred to *controlled*, which already means *planned* in this domain ("controlled demolition", "controlled release"), implies the risk was adequately managed, and pairs with *uncontrolled*, a word the definition dropped. A first version said "control held"; it was hard to parse.

**PSIF is a counterfactual.** Calling a barrier-saved event a PSIF means imagining it without the controls it actually had. What matters is how that counterfactual is built:

- **Campbell and DEKRA** build it by judgement ("if one thing had changed"). EEI showed this is unreliable: about 65% agreement among practitioners.
- **EEI** refuses it, and credits the control as a yes/no.
- **Our model** builds it by rule, and then models the controls separately.

**What the classifier and the simulator already do:**

1. **The classifier (layer 2) ignores controls by rule.** It builds the outcome distribution from the energy, the amplification mechanisms (carrier, receiving environment, body region) and a range of outcomes, then tests P(serious) against a threshold the company sets. That is the counterfactual "all controls absent", fixed by physics rather than imagined, so it is reproducible. This is the **inherent SIF potential**.
2. **The simulator (Product 2; see [monte-carlo-and-swiss-cheese.md](monte-carlo-and-swiss-cheese.md) and [controls-cost-benefit.md](controls-cost-benefit.md)) adds the controls.** Each control gets a metalog effectiveness distribution, drawn from the mitigation library (`data/sif/calibration/mitigations/`) or a job hazard analysis. Monte Carlo trials combine these with the inherent severity distribution to give a **residual** (controlled) outcome distribution. The trials include degraded and absent states: the trial where the escape route is blocked, and correlated degradation from a shared cause such as deferred maintenance. So the probability that the controls fail is *inside* the residual CDF.

The barrier-saved question is then answered by the CDF, not by a label. Take a fall arrested by a harness:

- Its **inherent** P(serious) is high.
- Its **residual** P(serious) is low, but not zero. The residual tail is the share of trials in which the harness, the anchor or the rescue fails.

**The counterfactual, sharpened.** In the residual distribution, degraded and absent control states are weighted by their probability. The counterfactual EEI warns against is different: **imagining controls that were not in the actual event** (or removing ones that were), with no probability attached. The model keeps three things apart:

| | Controls | What it tells you | Where |
|---|---|---|---|
| **Inherent potential** | All set aside, by rule | How bad the energy can be | Classifier, layer 2: **the headline** |
| **Residual potential** | As they were (or as designed), with failure states weighted by probability | How bad it is with these controls, including their unreliability | Simulator, using the layer 3 control state |
| **Counterfactual cycle** | Imagined differently from the facts | What if other controls had existed, or had failed | Analyst only, labelled `counterfactual` |

**Answer to Q2.**

- **The headline is inherent SIF potential**: the classifier, against the company threshold.
- **Credit for controls comes as the residual distribution**, not as a relabelling of the event.
- **The EEI and Campbell yes/no categories are projections**, for crosswalk and reporting:
  - Campbell's PSIF ≈ inherent P(serious) at or above the threshold, with no serious injury.
  - EEI's PSIF versus Capacity ≈ the same, split by whether a direct control was effective. In our terms, that is the residual P(serious) given the observed control state.

**Caveat.** The residual needs layer 3 information: which controls were present, and their effectiveness distributions. Narratives rarely give this, so the simulator gets it from the control library or a job hazard analysis, not from the incident report.

## 5. A draft definition

> **SIF potential.** An event has SIF potential when the energy involved, had it reached a person with no protective control between them, would more likely than not have caused a life-ending, life-threatening or life-altering injury.
>
> - *Event*: a happening, not a condition: the separation margin between energy and people, in distance, time or headroom, shrinks toward zero because the energy changes (an *excursion*) or the people do (an *incursion*). An event is defined before any outcome is known; SIF potential is assessed as if the separation had gone to zero.
> - *Near miss*: an event whose closest approach was *close* but not contact, or reached *contact* without harm: in OSHA's words, "given a slight shift in time or position" injury could have occurred. An excursion to unoccupied space is an event but not a near miss. As ICAO puts it, the difference from an accident "lies only in the result". Exposure alone is not a near miss.
> - *Exposure*: a condition, not an event. Energy (held, stored or available) and a person are together in time and space, the person within reach, assessed without regard to controls.
> - *Intended envelope*: bounds on the energy in magnitude (M: how much there may be), length (L: where it may be, and its required separation from people; for an intentional release, the release zone such as a blast exclusion zone, range danger area or relief discharge area) and time (T: when it may be present). An *excursion* is energy going beyond any one of them; an *incursion* is a person entering the envelope.
> - *Energy state*: potential is assessed on the energy present at the time, whatever its history. For a build-up, on the energy it would reach with no control acting, up to the source's inherent physical limit.
> - *Protective control*: anything added to act on the source or on the path to people (actively, passively or procedurally) that can degrade, be defeated or be overwhelmed while the source stays unchanged. Containment of the source, inherent features of the source (properties in effect throughout normal operation that can be lost only by changing the source, e.g. a reduced inventory or a negative temperature coefficient), the default environment and the person's own body are not controls.
> - *More likely than not*: P(serious outcome) ≥ 0.50, from the calibrated outcome distribution. A lower policy threshold (≥ 0.10) marks *elevated* potential.
> - *Life-ending, life-threatening, life-altering*: as in the LIFE model (Bayona et al., 2023).
>
> **PSIF.** An event with SIF potential and no serious injury, qualified by control state: **PSIF (unprotected)**, with no effective protective control (absent or failed; EEI's PSIF, BST's precursor); **PSIF (protected)**, where an effective direct control stood between the energy and the person (EEI: Capacity); or **PSIF (unverified)**, where the control state is not stated.
>
> **Residual SIF potential.** The outcome distribution with the controls as they were, or as designed. Each control has an effectiveness distribution that includes its degraded and absent states, and P(serious | controls) is read from the resulting CDF (see 4.4).
>

## 6. Consequences for the classifier if we adopt this

1. **Severity scale.** "Serious" is currently P(death) ≥ 0.10. The outcome distribution already gives the chance of non-fatal serious outcomes, with seriousness measured as threat to life (Q6). Life-altering injuries with low mortality (amputation, sight loss, permanent loss of function) need a second axis or a body-region rule. That is parked for the design specification.
2. **Missing information.** The classifier currently defaults to the least severe band (facts-only, as EEI). A population prior ([bayesian-priors.md](bayesian-priors.md)) is an *extension*, not an alternative. Parked for the design specification (Q7).
3. **Layer 3 extraction.** Add a control-state field (effective / failed / absent / not stated). An EEI direct-control flag is needed only for the EEI crosswalk (Q4). This is new work; until then, report the classifier's SIF as "PSIF (unverified)".
4. **Proximity.** Settled for reported events: chance P = 1 (Q5). A proactive use, measuring energy, exposure and exclusion zones and running the Monte Carlo as a risk assessment, is a future line of work.
5. **Out-of-scope mechanisms.** Confirm that overexertion and abrasion are automatically NON_SIF. Decide how medical events (e.g. cardiac arrest at work) and chronic exposures are handled; they can be fatal without any energy transfer.
6. **Envelope gate.** Pass 1 should recognise planned releases that stayed inside their envelope with nobody inside it, and record them as no event. It should also flag clause (b) events (incursions, and changes of state while a person is exposed) and clause (c) events (over-magnitude build-ups, energisation where the intended state is zero) even when nothing was released.
7. **Build-up bands.** The calibration bands describe released energy (fall height, voltage, speed). Build-up events need a way to choose a band from the uncontrolled limit (e.g. vessel burst energy from inventory and failure pressure), not from what the narrative says was reached.
8. **Threshold wording.** Keep both classes (Q3). Document them so that SIF = "most likely outcome is serious" (≥ 0.50) and ELEVATED = "credibly serious" (≥ 0.10). This lets each reader map the classes onto their preferred tradition.

## 7. Open questions

### Answered in this draft

- **Q1** (event wording and terms). **Answered.** "Unwanted", "unexpected" and "uncontrolled" are imprecise, and the model no longer needs them. Section 3 keeps the discussion as the argument for that choice, but the definition does not use them. The terms it does use (*exposure*, *excursion*, *incursion*, *event*, *near miss*) are understood in the domain and defined in the Glossary.
- **Q1b + Q1c** (upstream steps; build-up limits). One circular problem, broken by analysing in **cycles**: control-blind within a cycle, the starting state taken as found at its boundary, and the investigator scoping evidential versus counterfactual cycles. See the cycles refinement in section 3.
- **Q1d** ("no failure mode, no demand" as the test for an inherent feature). Accepted as the working definition of inherent safety / safety by design, and restated after checking against Kletz, CCPS/Hendershot, IAEA-TECDOC-626 and ISO 12100. The feature is in effect throughout normal operation and can be lost only by changing the source; controls are active, passive or procedural. See "Inherent features versus controls" in section 3.
- **Q1f** (exposure versus EEI's "Exposure"). Keep ours: it is the standard risk-formula meaning. Qualify EEI's as "Exposure (EEI)".
- **Q1g** (convergence when the target moves). The **incursion** is the event: entry into a danger zone defined by the plan. This works for vehicles and machinery, and gives a definition of **near miss** (an event in which nobody was hurt). See the event/exposure refinement.
- **Q1i** (how a cycle is bounded; how what-ifs are labelled). **Per source** by default. Scope is set by a **scoping statement**, as for a HAZID/HAZOP study (sources, bridges, counterfactuals in or out). What-ifs are labelled **counterfactual**. The classifier's implicit scope is per source, evidential only.
- **Q1j** (is incursion time the same as exposure?). **No.** Exposure is control-blind: the operator at a guarded machine is exposed for the whole time they use it, and the guard is in continuous demand. Incursion is entry into the danger zone while the energy is live; time inside is the highest exposure stratum. Entry under isolation is not an incursion. See "Refinement: exposure strata, demand and incursion".
- **Q1k** (why did event tables keep coming out sparse?). The target needs an envelope too, in where, when and how much. An event is a change on either side (excursion or incursion) that brings the two envelopes together; it is the materialised chance term of the risk equation (the top event). See "Refinement: the target has an envelope too".
- **Q1l** (does the event cover unoccupied excursions, near misses and harm events?). Yes, once the event is defined as a **shrinking of the separation margin**, a happening rather than a condition. Its closest approach (wide, close, contact) places it. "Chance P = 1" becomes **separation to zero**. See "Refinement: the event as shrinking separation".
- **Q1m** (is a person entering a heavy-lift exclusion zone an event, when the load is unchanged?). Yes: the separation shrank from the people's end. The deciding distinction is **live** (a suspended load has a line of fire) versus **isolated** (no line of fire). The degree of erosion is the closest approach. See "Refinement: live energy and the line of fire".
- **Q2** (credit working controls, or count barrier-saved events as PSIF?). **Headline = inherent SIF potential** (classifier, company threshold). Credit for controls comes as the **residual outcome distribution** from the simulator, whose Monte Carlo includes degraded and absent control states, so the answer is in the CDF. PSIF is the umbrella, qualified **protected / unprotected / unverified**; EEI's "Capacity" is PSIF (protected), avoiding confusion with Safety-II and HOP "capacity". See 4.4.
- **Q3** (anchor at 0.50 "more likely than not" or 0.10 "credible"?). **Both stay in use**: SIF ≥ 0.50, ELEVATED ≥ 0.10, with the threshold set by the company. No change; revisit when the validation against human coders has been reconciled.
- **Q4** (EEI's narrow direct control or a broader definition?). **No longer a definitional question.** EEI needs a narrow definition because its classification is yes/no: it must decide which controls "count". In our model every control (engineered, passive, procedural, training, PPE) enters the simulator through its effectiveness distribution. An error-tolerant engineered barrier has a high, narrow distribution; a rule or training has a low, wide one. EEI's three criteria are kept only for the EEI crosswalk (PSIF protected versus unprotected) and for HECA-style metrics.
- **Q5** (proximity requirement?). **Chance P = 1.** The classifier codes *reported* events, so there is no proximity test. A proactive mode is parked below.
- **Q6** (life-altering outcomes on a mortality-calibrated scale?). **Partly answered.** Layer 2's outcome is a distribution over the *seriousness of the injury*, measured as its probability of death on an AIS-derived scale (no injury, first aid, medical treatment, serious, fatal). So yes, it can be read as the chance *and* the seriousness of non-fatal outcomes: P(serious) counts non-fatal serious injuries as well as deaths, and the CDF gives the chance of each category. What it measures is **threat to life**, which covers life-ending and life-threatening outcomes. **Life-altering** injuries with low mortality (a finger amputation, sight loss, a crushed hand) score low on it. Adding them needs a second axis, e.g. an impairment scale such as the AAAM Injury Impairment Scale or the Functional Capacity Index (to check), or a body-region rule. Parked below.

### Still open

None in this draft.

### Parked: beyond this paper

These belong to a later design specification that wraps the model (risk-control design, data fields, metrics), not to the SIF definition.

- **Q1a.** Who defines the envelope (the design, the permit, the regulator), and what happens when none is documented? A question about the design of risk controls.
- **Q1e.** Making M, L, T operational: dimensionless margins (m, l, t) as fields, and controls classified as M, L or T. In this paper M, L, T is a framing device.
- **Q1h.** Standard exposure units per energy type, and where client data can supply the denominators. The paper shows there are several ways to measure exposure. Some energy types already converge on established metrics: chemical exposure as a time-weighted average against a workplace exposure limit (concentration × time), electromagnetic fields and ionising radiation as field-strength reference levels and dose (references to check).
- **Q5 (proactive mode).** Don't wait for reports. Measure where the energy is, where the exposure is, and where the exclusion zones are, then run the Monte Carlo as a forward-looking risk assessment. A future line of work.
- **Q6 (life-altering axis).** An impairment axis or a body-region rule alongside threat-to-life, so that LIFE's "life-altering" outcomes are counted.
- **Q7.** Missing detail. The classifier currently defaults to the least severe band. A population prior ([bayesian-priors.md](bayesian-priors.md)) would be an extension of the model, not an either/or choice.

## 8. Checks before polishing

- Campbell Institute (2018, p. 5) SIF potential wording verified from the author's copy. The 2020 serious-injury quote is still from a secondary source.
- ANSI/ASSP Z16.1-2022 definitions are unverified (paywalled).
- ASTM E2920-26 wording is from the store summary and a vendor blog.
- Viner (1991) "uncontrolled flow of energy" wording is unverified.
- Origin of the energy wheel graphic is unverified.
- **Units:** EEI's "500 ft-lbs" is about 680 J, *not* 1,500 J. EEI's own documents use both figures, so they are not a unit conversion. Our docs should state which one we mean wherever we cite either.
- Safety-II / resilience engineering (Hollnagel; Woods, "adaptive capacity") and HOP (Conklin) uses of "capacity" are cited from memory. Check them, and EEI's stated source for the term.
- The AAAM Injury Impairment Scale and the Functional Capacity Index are named from memory as candidate impairment axes. Check them. Also check the AIS level that P(death) ≥ 0.10 corresponds to.
- ISO 13857 (safety distances / reach to hazard zones) and the IEC 61508 low-demand, high-demand and continuous modes are cited from memory. Check both.
- Event literature: ISO 31000, Guide 73, ISO 45001 and ISO 12100 wordings come via secondary sources (ERA, nlfnorm.cz, certifico.com). Check them against licensed copies, including ISO 12100 clause numbers and ISO 31000 Note 1. Also check: HSG245 (secondary; its dangerous-occurrence wording is unverified), the CCPS/EI bowtie book's own top-event wording, and the lead-in to the OSHA near-miss template.
- Chemical exposure (TWA against workplace exposure limits) and EMF/ionising radiation metrics (reference levels, dose) are named as established exposure metrics without references.
- Micromort figures (e.g. about 1 per 250 miles of UK driving), the FAR definition (per 10⁸ exposed hours) and *microlife* are cited from memory. Check them against Spiegelhalter & Blastland (2013) and Kletz before quoting.
- Hopkinson–Cranz cube-root scaling and the radiation-protection triad (time, distance, shielding) are cited as standard practice. Add a primary reference for each (e.g. Baker, *Explosions in Air*; ICRP).
- Inherent-safety sources: quotes from Kletz (1991), Hendershot (2012), the CCPS Beacon (2010), IAEA-TECDOC-626, the IAEA Glossary (2018), ISO 12100 (preview), NIOSH and CDM 2015 were read verbatim. Not verified: the wording of the CCPS *Inherently Safer Chemical Processes* (2009) book, the text of Kletz (1978), and direct wording from Kletz & Amyotte (2010). Also unverified: the verbatim LOPA (2001) IPL definition, the IEC 61511/61508 demand-mode and SIF definitions, ISO 12100 clause 6.1 and the ANSI/ASSP Z590.3 definition. The NTC operating-state caveat is the research agent's own observation; check it with a nuclear source.
- "Excursion" (CCPS safe operating limits) and "incursion" (ICAO runway incursion) are cited as established usage from memory. Check both before relying on the parallel.
- API RP 754: check the exact Tier 1/2 treatment of pressure-relief discharges and the Tier 3 "demands on safety systems" wording (taken from memory, not yet checked in this review).
- BST precursor wording: Krause (2010) says "could reasonably result … if allowed to continue or repeat". The firmer "will result" is Martin & Black (2015). Quote the right one.

## Appendix A. Line of Fire

"Line of Fire" (LoF) is the phrase most used to explain energy-based safety on the shop floor. This appendix:

- traces where the phrase comes from and what it rests on,
- maps it onto the three-layer definition and the M, L, T envelope,
- says what it adds and what it leaves out.

### A.1 Origin

- **Military origin.** "Line of fire" is the path of a projectile from a weapon's muzzle. It is recorded from the 1850s, and "in the line of fire" figuratively means "in harm's way".
- **Industrial use.** We found no documented first use in industrial safety. Its institutional home is **oil and gas**:
  - Energy Safety Canada's *Line of Fire* programme. It was reportedly adapted from a Suncor Energy programme; the source documents are no longer online.
  - The **IOGP Life-Saving Rules**. Line of Fire was added as one of the nine rules in the 2018 major revision (IOGP Report 459, v3.0). Shell (2021) and ConocoPhillips (2019) adopted the IOGP set largely because it included Line of Fire.
- **Regulators.** Neither OSHA nor BLS uses the term. Their nearest categories are OSHA's Focus Four (falls, struck-by, caught-in/between, electrocution) and BLS OIICS "contact with objects and equipment" (struck by, struck against, caught in or compressed by). BSEE Safety Alert 468 (2023) uses it alongside "red zone".
- **Construction.** Usage there is mostly toolbox talks, not standards.

### A.2 What the authoritative versions say

**IOGP Life-Saving Rule, 2018** *(verbatim)*: "Keep yourself and others out of the line of fire.
- I position myself to avoid: moving objects, vehicles, pressure releases, dropped objects.
- I establish and obey barriers and exclusion zones.
- I take action to secure loose objects and report potential dropped objects."

**IOGP guidance** *(verbatim, extract)*: the rule "is intended to raise personal awareness of struck-by and caught-in-between hazards. Line of fire hazards are not always obvious or constant and can be introduced as the task progresses … At all times individuals need to continually monitor their surroundings and position themselves to avoid being in the line of fire."

**IOGP data:**
- 2008–2017: Line of Fire covered 135 of the 330 personal-safety fatalities that the nine rules address. It was the largest single rule.
- The 2025 supporting pack sorts 631 fatalities (2000–2024) into ten LoF hazard categories (table in A.5). It also states: "the difference between a high potential event outcome … and a fatal incident is often dependent on where people are positioned in relation to the surrounding hazards."

**Other definitions:**
- **BSEE** *(verbatim)*: LoF hazards "occur when a worker is in the direct path of an object if a release of energy from that object would cause the worker to be hit, struck, pinched, impaled, crushed, or caught between objects."
- **Energy Safety Canada** *(secondary, unverified)*: "You are in the line of fire when you are at risk of coming into contact with a force your body cannot endure."
- **Common toolbox wording**: LoF injuries happen "when the path of a moving object or the release of hazardous energy intersects with an individual's body". This is widely used, but we found no authoritative source, so it shouldn't be cited as one.

### A.3 The underlying model

No authority names a theory behind Line of Fire. The model it carries is **Haddon's energy–path–target**, reduced to the path:

- **Haddon countermeasure 5**: "separate in space or time the energy being released from the susceptible person". That is positioning.
- **Haddon countermeasure 6**: a material barrier between energy and person. That is "barriers and exclusion zones".

The link to Haddon is our inference; no source we read makes it explicitly.

Hallowell's energy-based approach does not use LoF as an organising idea. Hallowell & Spencer (2024) cite the Campbell/DEKRA list item "struck by or in the line of fire of a moving object with sufficient energy to cause SIF harm" as an example of an imprecise, list-based PSIF definition.

### A.4 Line of Fire in our definition

| Our concept | Line of Fire equivalent | Comment |
|---|---|---|
| Source–path–target | The "line" is the path; the "fire" is the energy | LoF covers path and target. It says nothing about the source's magnitude |
| Envelope, L dimension | Being *in* the line of fire means l < 1: separation less than the energy's reach | LoF is L only |
| Envelope, T dimension | "Not always obvious or constant and can be introduced as the task progresses" | IOGP's guidance recognises that the envelope changes over time |
| Envelope, M dimension | Absent. ESC's "a force your body cannot endure" is the only hint of a threshold | No magnitude threshold, which is Hallowell's criticism |
| Layer 1, clause (b) | Stepping *into* the line of fire (incursion), or the energy releasing while a person is in it | Clause (b) *is* the LoF event |
| Exposure (a condition) | Within reach of live energy with nothing happening, e.g. tag-liners at the edge of a lift. Stepping *into* the line of fire, such as walking under a suspended load, is an incursion (an event) | Our exposure, control-blind. With no direct control it is also EEI "Exposure" |
| Layer 2, inherent potential | **Chance P = 1 is the LoF question**: "if you had been in the line of fire, how bad would it be?" | LoF positioning changes chance P, not potential |
| Layer 3, control state | Positioning, barriers, exclusion zones, securing loose objects | All **L controls**. M controls (isolation, relief) and T controls (permits, interlocks) sit in other rules, e.g. Energy Isolation |
| EEI direct control | Barriers and engineered exclusion zones *may* qualify | Positioning by behaviour does not: it fails with unintentional human error. Feeds Q4 |

**The key alignment.** IOGP's own statement, that the gap between a high-potential event and a fatality "is often dependent on where people are positioned", is the chance P / outcome P split of section 2 in shop-floor language. Positioning decides the draw; the energy decides the distribution.

**"Line" is a simplification.** For dropped objects and projectiles the path is close to a line (a trajectory). For other energies it is a zone or a volume:

- the whip zone of a hose,
- the snap-back zone of a mooring line,
- the radius of a blast,
- the approach distance to a live conductor,
- the swept path of a reversing vehicle.

The envelope generalises the line, and blast and electrical scaling (section 3) give the size of the zone from the magnitude.

### A.5 Crosswalk: IOGP LoF categories to classifier mechanisms

The classifier's mechanisms can be rolled up into IOGP's LoF categories for shop-floor reporting:

| IOGP LoF hazard category (fatalities 2000–2024) | Classifier mechanism(s) | Energy type(s) |
|---|---|---|
| Moving vehicles / mobile equipment (142) | transport, struck | motion |
| Working at height: falls and dropped objects (94) | fall, struck | gravity |
| Objects with uncontrolled roll, slide or fall potential (89) | struck, caught_in, structural_collapse | gravity, motion |
| Mechanical lifting / hoisting (71) | struck, caught_in | gravity, mechanical |
| Electrical equipment / exposure (68) | electrical | electrical |
| Pressurised or vacuum systems / equipment (56) | pressure | pressure |
| Machinery and tools (42) | caught_in, struck, abrasion | mechanical |
| Projectiles / explosion / jet fire (34) | explosion, fire, struck | pressure, thermal, chemical, motion |
| Tensioned lines / spring-loaded devices / stored energy (31) | struck, pressure | mechanical, pressure |
| Natural environmental events / extreme weather (3) | structural_collapse, other | gravity, motion, thermal |
| *Not LoF* | breathing, chemical, thermal (contact), radiation_noise, overexertion, slip_no_fall, assault, animal_insect | chemical, thermal, radiation, biological |

The mapping is many-to-many, so a crosswalk needs the mechanism *and* the energy type, and sometimes the band, to place an event. Testing it against classified events is a follow-on task.

### A.6 What LoF adds, and what it leaves out

**Adds:**
- **Communication.** Its everyday phrase explains chance P = 1 well: "SIF potential asks how bad it would have been if you had been in the line of fire."
- **Exposure in a word.** "In the line of fire" is exposure, often an incursion. Stepping into it is an incursion (an event); being in it when the energy releases is an event too.
- **The T dimension in practice.** IOGP's "introduced as the task progresses" puts the time-varying envelope in shop-floor terms.
- **A reporting crosswalk** (A.5) that IOGP members already use.

**Leaves out:**
- **Magnitude (M).** There is no threshold. LoF can't tell a dropped spanner from a dropped beam, which is the job of layer 2.
- **Non-kinetic energies.** The 2018 rule text lists only kinetic and mechanical hazards. IOGP's 2025 categories and ConocoPhillips add electrical, explosion and weather. Chemical, thermal and toxic exposures sit with other rules.
- **Most controls.** It is deliberately framed around individual action ("I position myself"). IOGP itself puts design and engineering controls with the site organisation under the hierarchy of controls. Critics (e.g. La Duke, 2013) note that some procedures themselves place workers in the line of fire.
- **Process safety.** IOGP's fatality analysis behind the rules excluded process safety events, so LoF says little about clause (c) build-ups.

**Recommendation.** Use Line of Fire as the **communication layer** for the L dimension and for chance P = 1, and offer the A.5 crosswalk for reporting. Don't use it as a definition: it has no magnitude and only part of the control picture.

### A.7 Checks

- Earliest industrial use of the phrase: not found.
- Whether Line of Fire was among the 2012/13 OGP supplementary rules: not checked.
- Energy Safety Canada / Suncor programme documents: not retrievable. The ESC definition is from search snippets only.
- The common "path of a moving object or release of hazardous energy" definition has no authoritative source found.
- The Haddon link is our inference.

## Appendix B. Glossary

Terms as used in this draft. Where a term comes from a published source, the source is named; otherwise it is ours.

| Term | Meaning |
|---|---|
| **Active control** | (CCPS) A control that detects and responds: trips, interlocks, safety instrumented functions, sprinklers, quench and dump systems. Compare *passive control*, *procedural control* |
| **Active energy** | Energy that is moving, flowing or being released (a moving vehicle, a falling load, a blast). Descriptive; it does not by itself make an event. Compare *stored energy* |
| **Bridge condition** | A link from one cycle to the next: a control failure or event in one cycle that sets the starting state of another, often by energy conversion (fuel → motion, fuel or battery → fire, motion → heat) |
| **Capacity** | (EEI SCL) A high-energy release with an effective direct control present and no serious injury. This draft says *PSIF (protected)*, because "capacity" means something broader in Safety-II, resilience engineering and HOP (the ability of people and systems to adapt and fail safely) |
| **Chance P** | Earlier term: the probability that the energy reaches a person, "set to 1" for SIF potential. Replaced by *separation to zero*. It collided with the risk equation's chance term (the chance of the event per unit of exposure) |
| **Chance per unit exposure** | In the risk formula, the probability (rate) of a harmful event per unit of exposure (per hour, per lift, per take-off). Driven by controls |
| **Closest approach** | How far the separation margin shrank during an event: *wide* (excursion to unoccupied space), *close* (near miss) or *contact* |
| **Containment** | What holds energy in its intended state (pipe wall, cable insulation, vessel). Part of the source, not a control; release is escape from it |
| **Control (protective control)** | Anything added to act on the source or on the path to people that can degrade, be defeated or be overwhelmed while the source stays unchanged. Active, passive or procedural (CCPS); also classed as an M, L or T control. Compare *inherent feature* |
| **Counterfactual cycle** | A cycle the facts don't show happened, examined as a what-if (had the relief also failed). Legitimate in investigation and risk assessment; always labelled `counterfactual`. Compare *evidential cycle* |
| **Cycle** | One energy source analysed through all three layers once (envelope, exposure, event, potential, control state). Bounded per source by default; cycles chain through bridge conditions; each is *evidential* or *counterfactual* |
| **Danger zone** | An area within reach of an energy that the plan keeps clear of people. For machinery, ISO 12100 3.11: "any space within and/or around machinery in which a person can be exposed to a hazard". Entry is an incursion |
| **Demand** | (LOPA, IEC 61511) A deviation that calls on a protection layer to act; even passive barriers (a bund) are demanded and have a probability of failure on demand. A guard on a running machine is in *continuous* demand while someone is within reach (IEC 61508 modes). Not the test for an inherent feature |
| **Direct control** | (EEI) A control targeted to the high-energy source, effective when installed, verified and used properly, and effective even with unintentional human error. Training, rules, signs and generic PPE are excluded |
| **Elevated** | Classifier class: P(serious outcome) ≥ 0.10 and < 0.50; "credibly serious" |
| **Elimination** | Removing the energy from the source (supply disconnected and removed, system drained and vented, stored energy discharged). Changes the source and ends exposure. Compare *isolation* |
| **Envelope (intended energy envelope)** | The designed bounds on an energy in magnitude (M), space and separation (L) and time (T): containment and designed path for contained energy; the release zone for an intentional release |
| **Event** | A happening, not a condition: the separation margin between energy and people, in distance, time or headroom, shrinks toward zero because the energy changes (an *excursion*) or the people do (an *incursion*). An event is defined before any outcome is known; SIF potential is assessed as if the separation had gone to zero. Layer 1; the bowtie top event |
| **Evidential cycle** | A cycle the facts show happened, with the bridge to it evidenced in the narrative. The classifier's default scope |
| **Excursion** | Energy going beyond its envelope in L, M or T |
| **Exposure** | As a condition: energy (held, stored or available) and a person together in time and space, the person within reach, assessed **without regard to controls**. As a quantity: that co-presence measured in context-specific units (person-hours, lifts, take-offs, km), the denominator of the risk formula. Not the same as EEI's "Exposure", which is a high-energy hazard *without* a direct control (our exposure + control absent) |
| **Exposure stratum** | A part of total (control-blind) exposure with one control state, e.g. guards in place, isolated, or guard open while energised. Risk sums over strata |
| **FAR (Fatal Accident Rate)** | Fatalities per 10⁸ exposed hours (Kletz, process safety): a chance-per-unit-exposure measure |
| **HECA** | (EEI/CSRA) High-Energy Control Assessment: the share of observed high-energy hazards with a direct control in place, Success / (Success + Exposure) |
| **High energy** | (EEI) Over 500 ft-lbs (about 680 J); Hallowell et al. (2017) use > 1,500 J for "most likely a SIF". Not the same threshold |
| **Incursion** | An event: people entering the energy's envelope: into the line of fire of live energy, there when it is live, or in greater numbers than planned. Time inside counts as exposure. Entry under isolation, or when the energy is not live, is not an incursion: there is no line of fire |
| **Inherent feature** | A property of the source's own materials, quantity, operating conditions or physics. It is in effect throughout normal operation without detection, actuation, power or human action, and can be lost only by changing the source (e.g. a reduced inventory, pump dead-head pressure, a negative temperature coefficient). Part of the source; changes the potential. Judged for a named hazard within the operating envelope |
| **Inherent safety** | "The achievement of safety through the elimination or exclusion of inherent hazards through the fundamental conceptual design choices" (IAEA-TECDOC-626). In process safety, "eliminate or greatly reduce the hazard by changing the process to use materials and conditions that are nonhazardous or much less hazardous" (Hendershot, 2012). Kletz: "what you don't have, can't leak" |
| **Inherent SIF potential** | Layer 2: the outcome distribution if the energy reached a person with no protective control between them, assessed on the energy state at the time (or the uncontrolled limit of a build-up) |
| **Inherently safe design measure** | (ISO 12100 3.20, machinery) "protective measure which either eliminates hazards or reduces the risks associated with hazards by changing the design or operating characteristics of the machine without the use of guards or protective devices" |
| **Inherently safer design** | (CCPS) "The process of identifying and implementing inherent safety in a specific context". Always comparative: "A technology can only be described as inherently safer when compared to a different technology". Kletz's principles: intensification, substitution, attenuation, limitation of effects, simplification |
| **Isolation** | A control that holds energy away from the work (isolator, LOTO, immobilisation). It has failure modes, so the energy is still present and the person still exposed. A procedural control in CCPS terms. Compare *elimination* |
| **Layer 1, 2, 3** | Event; inherent SIF potential; control state |
| **LIFE** | (Bayona et al., 2023) Life-ending, life-threatening, life-altering: the three-part definition of a serious outcome |
| **Line of fire** | The path live energy takes as it moves, or would take if released (drop path, swing path, travel path, trajectory). Danger zones are drawn around it. Also the shop-floor term (IOGP); see Appendix A |
| **Live** | Energy in its working state, not isolated, with a line of fire to people: a suspended load, a running machine, an energised conductor, plant under pressure. Isolated energy is present (exposure) but not live |
| **M, L, T** | Magnitude, length (space and separation), time: the three dimensions of the envelope |
| **Margins m, l, t** | Dimensionless ratios of actual to design value: m = magnitude / design magnitude; l = separation / required separation; t = presence outside the intended window. Excursion or incursion when m > 1, l < 1 or t > 0 |
| **Micromort** | A one-in-a-million chance of death, quoted per unit of an activity (Howard; Spiegelhalter). Layer 2's P(death) scale lets expected micromorts be computed as exposure × event rate × E[P(death) given the event] |
| **Near miss** | An event whose closest approach was *close* but not contact, or reached *contact* without harm: in OSHA's words, "given a slight shift in time or position" injury could have occurred. An excursion to unoccupied space is an event but not a near miss. As ICAO puts it, the difference from an accident "lies only in the result". Exposure alone is not a near miss (ISO 45001 Note 2; HSG245) |
| **Outcome P** | The distribution of severity given the energy reaches a person. Layer 2 models this |
| **P(serious)** | Classifier output (`sif_p_sif`): probability that the outcome is serious or worse, from the calibrated band distribution |
| **Passive control** | (CCPS; IAEA) A control that acts by its presence, "without the active functioning of any device" (Hendershot, 2012), but can still fail: bunds, guards, blast walls, fire insulation. Relief valves and bursting discs are passive with moving parts (IAEA category C). "Passive safety is not the same as inherent safety" (Kletz) |
| **Prevention through Design (PtD)** | (NIOSH) "addressing occupational safety and health needs in the design process to prevent or minimize the work-related hazards and risks associated with the construction, manufacture, use, maintenance, and disposal of facilities, materials, and equipment". US counterpart of the CDM 2015 designers' duty |
| **Procedural control** | (CCPS) A control that depends on people following procedures: operating procedures, LOTO, permits, training, emergency response |
| **PSIF (potential SIF)** | An event with SIF potential and no serious injury, always qualified by control state: *unprotected* (no effective protective control; EEI's PSIF, BST's precursor), *protected* (an effective direct control worked; EEI's Capacity) or *unverified* (not stated) |
| **Residual SIF potential** | The outcome distribution with controls as they were or as designed. Each control has an effectiveness distribution including degraded and absent states, combined by Monte Carlo (the simulator); P(serious \| controls) is read from its CDF. Compare *inherent SIF potential* |
| **Risk** | Exposure × chance per unit of exposure × consequence |
| **Scoping statement** | The statement that frames a SIF assessment before it starts, as for a HAZID/HAZOP study: the sources (cycles) in scope, the bridges to follow, and whether counterfactual cycles are included |
| **Separation margin** | What keeps energy and people apart: distance, time, or headroom to the design limit. An event shrinks it toward zero |
| **Separation to zero** | The SIF potential counterfactual: assess as if the separation margin had gone to zero, so the energy reached a person |
| **SIF** | Serious injury or fatality. As a classifier class: P(serious) ≥ 0.50, "most likely outcome is serious" |
| **SIF actual (HSIF)** | An event in which a serious injury or fatality occurred |
| **Source–path–target** | The energy-damage model (Gibson, Haddon): energy leaves a source, travels a path, reaches a target. Controls act at the source or on the path |
| **STKY** | (Hallowell/CSRA) "Stuff That Kills You": high-energy hazard categories |
| **Stored energy** | Energy held, contained, suspended, isolated or available but not flowing (a suspended load, a pressurised vessel, a live conductor, the supply behind an isolator, fuel in a parked vehicle). Still counts as present for exposure. Compare *active energy* |
| **Success** | (EEI/HECA) A high-energy hazard with a corresponding direct control in place |
| **Target envelope** | Where people are, when they are there, and how many (one person, a team, a public crowd): the counterpart of the energy's envelope. An event brings the two envelopes together |
| **Uncontrolled limit** | For a build-up, the energy it would reach with no control acting, up to the source's physical limit (inventory, failure pressure, maximum speed) |

## References

- ANSI/ASSP Z16.1-2022. *Safety and Health Metrics and Performance Measures*. ASSP.
- API RP 754 (4th ed., 2026). *Process Safety Performance Indicators for the Refining and Petrochemical Industries*. American Petroleum Institute. [edition and wording to check](https://www.api.org/products-and-services/standards/important-standards-announcements/api-rp-754-4th) [3rd Edition](https://avadocuments2026.com/preview/ansi-api-rp-754-2021.pdf)
- [ASTM E2920-26 (2026). *Standard Guide for Recording Priority Occupational Injuries and Illnesses and SIF-Related …*](https://store.astm.org/e2920-26.html)
- Baker, S.P. & Haddon, W. Jr. (1974). Reducing injuries and their results: the scientific approach. *Milbank Memorial Fund Quarterly: Health and Society*, 52(4), 377–389.
- [Bayona, A., Bhandari, S., Hallowell, M., Sherratt, F., Bailey, J.M. & Upton, J. (2023). What is a serious injury? A model for defining serious injuries & fatalities. *Professional Safety*, 68(9)](https://www.assp.org/docs/default-source/psj-articles/f1bayona_0923.pdf)
- BSEE (2023). *Safety Alert 468: Red Zone Hazards Pose Increased Risk to Personnel During Lifting Operations*. Bureau of Safety and Environmental Enforcement.
- Campbell Institute (2018). *Serious Injury and Fatality Prevention: Perspectives and Practices*. National Safety Council.
- Campbell Institute (2020). *Designing Strategy for Serious Injury and Fatality Prevention*. National Safety Council.
- Campbell Institute (2022). *SIF Prevention: Leading Indicators, Cumulative Risk, and Safety Networks*. National Safety Council.
- CCPS (2001). *Layer of Protection Analysis: Simplified Process Risk Assessment*. Center for Chemical Process Safety / AIChE. (independent protection layer definition to check)
- [CCPS (2010). Passive safety devices. *Process Safety Beacon*, July 2010](https://www.aiche.org/sites/default/files/2010-07-Beacon-English.pdf)
- [CCPS (2018). *Process Safety Metrics: Guide for Selecting Leading and Lagging Indicators*, v3.2 AIChE](https://www.aiche.org/sites/default/files/docs/pages/ccps_process_safety_metrics_-_v3.2.pdf)
- [CDM 2015. *Construction (Design and Management) Regulations 2015*, regulation 9 (duties of designers)](https://www.legislation.gov.uk/uksi/2015/51/regulation/9)
- [ConocoPhillips (2025). *Life Saving Rules Standard*](https://static.conocophillips.com/files/resources/life-saving-rules-standard.pdf)
- DOE-HDBK-1208-2012. *Accident and Operational Safety Analysis*. US Department of Energy.
- Edison Electric Institute (2025). *SIF Criteria* (SCL model, Appendix 9).
- [Energy Safety Canada. *Line of Fire Program*](https://www.energysafetycanada.com/resource/line-fire-program)
- Haddon, W. Jr. (1973). Energy damage and the ten countermeasure strategies. *Journal of Trauma*, 13(4), 321–331. doi:10.1097/00005373-197304000-00011
- [Hallowell, M. & Oguz Erkal, E.D. (2024). *High-Energy Control Assessments (HECA)*. Edison Electric Institute](https://cms.eei.org/-/media/Project/EEI/Documents/Issues-and-Policy/Power-to-Prevent-SIF/EEI-HECA.pdf)
- [Hallowell, M. (2020, rev. 2024). *Safety Classification and Learning (SCL) Model*. Edison Electric Institute](https://cms.eei.org/-/media/Project/EEI/Documents/Issues-and-Policy/Power-to-Prevent-SIF/eeiSCLmodel.pdf)
- [Hallowell, M.R. & Spencer, C. (2024). Safety Classification & Learning Model: defining & classifying potential serious injuries & fatalities. *Professional Safety*, 69(2)](https://assp.org/docs/default-source/psj-articles/f1hallowellspencer_0224.pdf)
- [Hallowell, M.R., Alexander, D. & Gambatese, J.A. (2017). Energy-based safety risk assessment: does magnitude and intensity of energy predict injury severity? *Construction Management and Economics*, 35(1–2), 64–77. doi:10.1080/01446193.2016.1274418](https://www.tandfonline.com/doi/full/10.1080/01446193.2016.1274418)
- [Hendershot, D.C. (2012). Inherently safer design: the fundamentals. *Chemical Engineering Progress*, January 2012. AIChE/CCPS](https://www.aiche.org/sites/default/files/cep/20120140-1.pdf)
- Howard, R.A. (1980). On making life and death decisions. In R.C. Schwing & W.A. Albers (eds), *Societal Risk Assessment: How Safe Is Safe Enough?* Plenum. (micromort origin; to check)
- HSE (2004). *HSG245: Investigating Accidents and Incidents*. Health and Safety Executive.
- [IAEA (1991). *IAEA-TECDOC-626: Safety Related Terms for Advanced Nuclear Plants*](https://www-pub.iaea.org/MTCD/Publications/PDF/te_626_web.pdf)
- [IAEA (2018). *IAEA Safety Glossary*, 2018 ed](https://www-pub.iaea.org/MTCD/publications/PDF/PUB1830_web.pdf)
- [ICAO (2010). *Annex 13 to the Convention on International Civil Aviation: Aircraft Accident and Incident Investigation*, 10th ed](https://elibrary.icao.int/reader/475561/&returnUrl%3DaHR0cHM6Ly9lbGlicmFyeS5pY2FvLmludC9wcm9kdWN0LzQ3NTU2MQ%3D%3D)
- [ICMM (2015). *Critical Control Management: Implementation Guide*](https://pimcore.icmm.com/website/publications/pdfs/health-and-safety/2015/guidance_ccm-implementation.pdf)
- [IOGP (2018). *Report 459: IOGP Life-Saving Rules*, v3.0. International Association of Oil & Gas Producers](https://www.iogp.org/life-savingrules/)
- [IOGP (2025). *Life-Saving Rules: Line of Fire – Introduction to Supporting Materials*](https://www.iogp.org/wp-content/uploads/2025/10/IOGP-Life-Saving-Rules-Line-of-Fire-Introduction-to-Supporting-Materials.pdf)
- ISO 12100:2010. *Safety of machinery – General principles for design – Risk assessment and risk reduction*. ISO.
- ISO 31000:2018. *Risk management – Guidelines*. ISO Guide 73:2009, *Risk management – Vocabulary*.
- ISO 45001:2018. *Occupational health and safety management systems – Requirements with guidance for use*.
- Johnson, W.G. (1980). *MORT Safety Assurance Systems*. Marcel Dekker.
- Kletz, T. (1978). What you don't have, can't leak. *Chemistry & Industry*, 287–292.
- [Kletz, T. (1991). Inherently safer plants – recent progress. *IChemE Symposium Series* No. 124 (Hazards XI)](https://www.icheme.org/media/10434/xi-paper-14.pdf)
- Kletz, T. (1991, 2nd ed. 2010 with Amyotte). *Process Plants: A Handbook for Inherently Safer Design*. (edition to check)
- Kletz, T. (1999). *Hazop and Hazan: Identifying and Assessing Process Industry Hazards*, 4th ed. IChemE. (FAR; edition to check)
- [Krause, T.R. (2010). *New Findings on Serious Injuries and Fatalities*. BST](https://ppsa.memberclicks.net/assets/Webinars/Presentations/Krause%202010%20-%20New%20Findings%20on%20Serious%20Injury%20and%20Fatality%20Prevention.pdf)
- La Duke, P. (2013). Working in the line of fire. *ISHN*, 22 August 2013.
- [Manuele, F.A. (2008). Serious injuries & fatalities: a call for a new focus on their prevention. *Professional Safety*, 53(12), 32–39](https://static1.squarespace.com/static/5d12affb6c48770001d49960/t/5ee27d177f2406082d5cfeda/1591901464599/fred+manuele+serious+injuries+asse.pdf)
- [Manuele, F.A. (2013). Preventing serious injuries & fatalities: time for a sociotechnical model for an operational risk management system. *Professional Safety*, 58(5)](https://aeasseincludes.assp.org/professionalsafety/pastissues/058/05/F2Manu_0513.pdf)
- [Martin, D.K. & Black, A.A. (2015). Preventing serious injuries & fatalities: study reveals precursors & paradigms. *Professional Safety*, 60(9)](https://safetyfocus.assp.org/docs/Don%20Martin_Preventing%20Serios%20Injuries%20and%20Fatalities_September%202015.pdf)
- [National Academies (2012). *The Use and Storage of Methyl Isocyanate (MIC) at Bayer CropScience*, ch. on inherently safer processes](https://www.nationalacademies.org/read/13385/chapter/7)
- [NIOSH. *Hierarchy of Controls*; *Prevention through Design*](https://www.cdc.gov/niosh/hierarchy-of-controls/about/index.html)
- [Oguz Erkal, E.D. & Hallowell, M.R. (2023). Moving beyond TRIR: measuring & monitoring safety performance with high-energy control assessments. *Professional Safety*, 68(5), 26–35](https://www.assp.org/docs/default-source/psj-articles/f1hallowell_0523.pdf?sfvrsn=45416846_0)
- OSHA, 29 CFR 1904.39. Reporting fatalities, hospitalizations, amputations, and losses of an eye.
- [OSHA. *Construction Focus Four: Struck-By Hazards*, instructor guide](https://www.osha.gov/sites/default/files/struckby_ig.pdf)
- [OSHA. *Template for Near Miss Report Form*](https://www.osha.gov/sites/default/files/2021-07/Template%20for%20Near%20Miss%20Report%20Form.pdf)
- [RIDDOR 2013, regulation 4 (specified injuries) and Schedule 2 (dangerous occurrences)](https://www.legislation.gov.uk/uksi/2013/1471/regulation/4)
- Spiegelhalter, D. & Blastland, M. (2013). *The Norm Chronicles: Stories and Numbers About Danger*. Profile Books.
- [UK CAA. *BowTie: Top events*](https://www.caa.co.uk/safety-initiatives/working-with-industry/bowtie/bowtie-elements/top-events/)
- Viner, D. (1991). *Accident Analysis and Risk Control*. (wording unverified)
