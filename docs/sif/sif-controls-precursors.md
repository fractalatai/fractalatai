# Controls, precursors and conditions in SIF potential

Status: draft v0.3, 2026-11-19. A literature review in the series on defining serious injury and fatality (SIF) potential. Definitions follow [sif-definition.md](sif-definition.md); the companion paper on counterfactuals is [sif-counterfactual.md](sif-counterfactual.md).

| Version | Date | Change |
|---|---|---|
| 0.2 | 2026-11-17 | Draft for the definition papers |
| 0.3 | 2026-11-19 | Conditions count when a person could be within reach during intended use or reasonably foreseeable misuse (§4.2; QQ-FB-03). Assurance directed by value of information (§5.4; QQ-FB-09) |

## Executive summary

*For the safety director.*

### The problem

Two questions are tangled together in SIF programmes:

1. **How bad could the energy have been?** That is the SIF potential.
2. **Were the controls there, and did they work?** That is the control state.

Published definitions answer them in different orders, and some fold the second into the first. A BST/DEKRA-style *precursor* is defined by controls being "absent, ineffective, or not complied with". EEI's PSIF needs both high energy and the absence of a direct control. Campbell's "SIF potential" ignores the barriers that worked. These definitions are not interchangeable, so counts made under one cannot be compared with counts made under another.

A second tangle is about what gets counted. PSIF is an **event**: something happened, and a serious injury would have been the likely result. Many programmes also apply PSIF to every **observation**, whether a hazard spotted or work seen being done safely. But whether something is an event depends on what was seen, not on how it was reported:

- **A person seen under a suspended load** is an event. They entered the line of fire of live energy, whether or not anyone saw them go in.
- **A person seen working at an unguarded edge, or tag-lining a load,** is a **condition**. Live energy is present and they are within its reach, but nothing has happened.

The best-developed framework, EEI's Safety Classification and Learning (SCL) model, keeps events and conditions apart:

- a high-energy hazard with no direct control is *Exposure*;
- a high-energy hazard with a direct control is *Success*;
- only a **release** of energy is a candidate *PSIF*.

Hallowell & Spencer (2024) warn that broad definitions let "any incident or condition" be classified as PSIF, causing "overinclusion, which waters down SIF-specific learning".

EEI's events start from the energy source: something has to be released. That leaves a gap. A hand reaching into a running machine, or a worker under a load that holds, involves no release, so EEI can only record it as Exposure. The AI model counts events from both ends. The energy moves toward people (an **excursion**), or people move into the line of fire of live energy (an **incursion**). An incursion is PSIF. This is a deliberate departure from EEI (section 4.2).

### The fix

Keep the two questions apart, and give events and conditions their own labels.

| | Event (excursion or incursion) | Condition (nothing happened) |
|---|---|---|
| **Examples** | A load drops; a crane overturns; a person walks under a suspended load; a hand goes past an open guard into a running conveyor | Work at an unguarded edge; tag-lining a lift; work on isolated plant |
| **SIF potential** (controls set aside) | Would a serious injury have been more likely than not, had the energy reached a person? | The same question, asked of the energy present |
| **Label when the potential is SIF** | **PSIF** | **SIF exposure**: a *precursor* when the controls are not in place |
| **Control state** | *protected* (a direct control worked; EEI Capacity), *unprotected* (absent or failed; EEI PSIF), *unverified* | *protected* (direct control in place; EEI Success), *unprotected* (absent or failed; EEI Exposure, BST precursor), *unverified* |
| **What it is for** | Learning from what nearly happened | Finding and fixing precursors before anything happens; measuring control coverage |

### Recommendations

1. **Code by content, not by channel.** An event is PSIF however it is reported: a person seen in the line of fire of live energy is an incursion. A condition is **SIF exposure**, never PSIF. It is *unprotected* (a **precursor**) when no direct control is in place, and *protected* when one is. Energy with no person within reach is a hazard, not an exposure. Record any trigger seen with the exposure, such as a trip hazard at an unprotected edge. Triggers do not change the potential, but they show which precursors to fix first (section 4.3).
2. **Keep the two counts separate.** Report PSIF (events) and SIF exposure (conditions) side by side, never summed. The AI model's PSIF counts include incursions, and excursions with no one nearby, so they will be higher than EEI's for the same work. Do not benchmark one against the other directly.
3. **Use positive observations.** The share of SIF exposures with a direct control in place, EEI's high-energy control assessment (HECA), is the leading indicator the observation data can support.
4. **Count only direct or critical controls** when recording control state: targeted at the energy, verified in place, and effective even if someone makes an unintentional error. Training, rules, signs and general PPE are not direct controls. Where the report is silent, record the control state as *unverified*, not absent, and have the panel resolve it (section 7).
5. **Use PSIF and precursor data for learning, not targets.** Precursor events are good warnings but poor performance indicators (Hopkins, 2009). EEI's model "should be used for learning, not for the creation of new metrics".

## Contents

- [1. The question](#1-the-question)
- [2. Where controls sit in the definitions](#2-where-controls-sit-in-the-definitions)
- [3. What counts as a control](#3-what-counts-as-a-control)
- [4. Conditions and events](#4-conditions-and-events)
- [5. Measuring controls](#5-measuring-controls)
- [6. Critiques](#6-critiques)
- [7. The AI model's position](#7-the-ai-models-position)
- [8. Research agenda](#8-research-agenda)
- [References](#references)
- [Verification notes](#verification-notes)

## 1. The question

Every SIF programme has to decide how controls enter the classification. There are three possibilities:

- **In the definition of potential.** A situation is a precursor *because* controls are missing.
- **Set aside when judging potential.** Controls are ignored to judge how dangerous the energy is.
- **As a qualifier afterwards.** Potential is judged first; the control state is recorded beside it.

It also has to decide what may be classified: only events, or observed conditions too.

These choices decide what a PSIF count means, whether it can be compared with another site's, and what can be learned from it.

## 2. Where controls sit in the definitions

### 2.1 Controls as the defining feature: the precursor schools

BST introduced the precursor:

> "A precursor of a SIF is defined as a high risk situation in which management controls are either absent, ineffective, or not complied with, and if allowed to continue or repeat could reasonably result in a serious injury or fatality." (Krause, 2010)

Martin & Black (2015) firmed the wording to "will result in a serious or fatal injury if allowed to continue", and described precursors as "conditions, behaviors, practices, exposures, situations and factors". Their formula: "a high-risk situation must be present, the safety controls designed to protect against injury must fail and this combination must be allowed to continue."

DEKRA's current wording, as quoted by the Campbell Institute (2022), keeps the form: "A high-risk situation in which control measures are absent, ineffective or not complied with, and would potentially result in a fatality or serious injury if allowed to continue" (DEKRA, 2019).

The Campbell Institute adopted the two-part form and made the role of controls explicit:

> "it is a high-risk situation where controls are broken, absent or not complied with. This two-part definition of a precursor is what is tricky for organizations to understand … Working at height is not in and of itself a precursor, but working at height without fall protection is." (Campbell Institute, 2020)

It also moved from "management controls" to *critical* controls: "if not all the critical controls are in place or not functioning as intended, and if this situation is not mitigated, then a SIF precursor exists" (Campbell Institute, 2020).

In this school, **a precursor is high-risk energy plus a control failure**. Remove the control failure and there is no precursor.

### 2.2 Controls set aside: potential as the energy's

Other definitions judge potential with the controls removed, or ignored:

- **Campbell (2018):** an event has SIF potential "if it could have resulted in a serious injury or fatality if not for certain barriers or countermeasures, or if one factor around the event had been changed". The barriers that worked do not reduce the potential.
- **ICMM (2015):** a material unwanted event is sized by its consequence, and "it is usual to consider this in terms of the maximum foreseeable loss". Controls are then managed separately.
- **ARMS (2010):** the first question, "If this event had escalated into an accident, what would have been the most credible accident outcome?", is asked before any credit is given for barriers.
- **EEI SCL (2024):** the first question is whether high energy was present, judged by its magnitude, before any question about controls.

### 2.3 Controls as a qualifier

EEI's SCL model, and ARMS's second question, judge the energy first and then classify by control state.

| EEI SCL category | Definition (Hallowell & Spencer, 2024) | Event or condition |
|---|---|---|
| **PSIF** | "Incident with a release of high energy in the absence of a direct control where a serious injury is not sustained" | Event |
| **Capacity** | "Incident with a release of high energy in the presence of a direct control" | Event |
| **Exposure** | "Condition where high energy is present in the absence of a direct control. Unlike incidents, an exposure is an observable condition" | Condition |
| **Success** | "Condition where a high-energy incident does not occur because of the presence of a direct control" | Condition |

ARMS's second question asks: "What was the effectiveness of the remaining barriers between this event and the most credible accident outcome?" It specifies that "Barriers that already failed are ignored. Only the barrier which worked and any subsequent barriers still in place are taken into account" (ARMS Working Group, 2010).

EEI's working group arrived at this structure from a disagreement. "Some team members focused only on the seriousness of the hazard involved, others considered the presence and adequacy of relevant controls." It concluded that "cases where an adequate control was present were fundamentally different from those with an inadequate level of control" (Hallowell & Spencer, 2024).

### 2.4 Summary

| Framework | Potential judged… | Controls are… | Classifies conditions? |
|---|---|---|---|
| BST / DEKRA precursor (Krause, 2010; Martin & Black, 2015) | Implicitly, as "high risk" | The defining feature ("absent, ineffective, or not complied with") | Yes: precursors are situations |
| Campbell precursor (2020) | As "high-risk situation" or "SIF risk" | The defining feature (critical controls) | Yes |
| Campbell SIF potential (2018) | Ignoring the barriers that worked | Set aside | Mainly events (near misses) |
| ICMM CCM (2015) | As maximum foreseeable loss | Managed and verified separately | Controls are verified by observation |
| ARMS ERC (2010) | As the most credible accident outcome | A second, separate question | Events |
| EEI SCL (2024) | By energy magnitude (high energy) | A qualifier: direct control present or not | Yes: Exposure and Success |
| AI model ([sif-definition.md](sif-definition.md)) | With all controls set aside (separation to zero) | A qualifier: protected / unprotected / unverified | Yes: SIF exposure |

## 3. What counts as a control

The frameworks differ on what a "control" is. The ones that give credit for controls converge on a narrow definition.

| Source | Definition | Excluded |
|---|---|---|
| **BST / DEKRA** | "Management controls", not further defined (Krause, 2010) | Nothing explicit; behaviour and compliance count |
| **ICMM (2015)** | "An act, object (engineered) or system (combination of act and object) intended to prevent or mitigate an unwanted event." A *critical* control is one whose "absence or failure … would significantly increase the risk despite the existence of the other controls" | Anything whose performance is not "specified, observable, measurable and auditable": "The plan may contain controls, but the plan itself is not the control" |
| **CCPS / Energy Institute bowties** | "Each valid barrier should be effective, independent, and auditable. It must have the capability to completely stop the threat from leading to the top event or, if a mitigation barrier, significantly reducing or eliminating the consequence" (CCPS & EI, 2018, section 2.6.3). They are either prevention barriers ("stop threats from resulting in the top event") or mitigation barriers ("reduce the scale of … undesired consequences") Training and competence: listing them as barriers is a common mistake, because "these are degradation controls and would appear on a degradation pathway supporting the barrier to which they apply" (CCPS & EI, 2018, p. 43). Likewise "training, competence, MOC, audit, inspection and maintenance programme, contractor management … are not barriers in themselves" (Johnson et al., 2018) |
| **EEI direct control** | One that "1) is specifically targeted to the high-energy source; 2) effectively mitigates exposure to the high-energy source when installed, verified and used properly …; and 3) is effective even if there is unintentional human error" (Hallowell & Spencer, 2024). Direct controls are absolute (they eliminate exposure) or mitigating (they reduce it below the threshold) | Training, warning signs, rules, experience and most standard PPE. "A deficiency in any component of the system invalidates the entire direct control" (EEI, 2024) |

On human factors, the bowtie literature is blunt: "Most of the controls that organisations use to minimise human error cannot meet the criteria necessary to be considered active barriers in their own right" (Manton et al., 2017).

**What this means.** A precursor defined by "management controls … not complied with" lets behaviour and non-compliance define the category. A classification that credits only direct, critical or barrier-grade controls does not. The first casts a wide net and finds many precursors. The second is narrower and more consistent, because whether a guard was fitted is easier to agree on than whether a rule was "complied with".

## 4. Conditions and events

### 4.1 The distinction

An **event** is a happening: in the AI model's definition, the separation between energy and people shrinks, from either end. A **condition** is a state: energy and people are together and nothing changes. Several bodies of work draw the same line:

- **ISO 12100** distinguishes the *hazardous event*, "event that can cause harm", from the *hazardous situation*, a "circumstance in which a person is exposed to at least one hazard".
- **Reason** describes latent conditions as "the inevitable 'resident pathogens' within the system". They "may lie dormant within the system for many years before they combine with active failures and local triggers to create an accident opportunity", and "can be identified and remedied before an adverse event occurs" (Reason, 2000).
- **EEI SCL** separates incidents (PSIF, Capacity) from conditions (Exposure, Success): "Unlike incidents, an exposure is an observable condition" (Hallowell & Spencer, 2024).

### 4.2 Source-centric and two-sided events

EEI defines a high-energy incident as an "instance where the high-energy source was released and where the worker came in contact with or proximity to the high-energy source". A release is "an instance in time where there is a temporary loss of control of the high-energy, or the high-energy changes state while exposed to the work environment". Proximity means within 6 feet, measured as "the closest point of a worker and the energy for the duration of the energy transfer" (EEI, 2024, pp. 11, 19).

This is a **source-centric** definition: the energy has to do something. It works for dropped objects, falls and arc flash. It does not work for energy that is already moving under control. EEI's list of high-energy hazards includes "contact with moving components of stationary machinery". But a running conveyor is neither released nor out of control. A hand that reaches past an open guard into it, without contact, is no incident under EEI, only an Exposure. The same applies to a worker who walks under a suspended load that holds.

EEI makes one exception, for traffic: "For work zone traffic only, an incident occurs only when a vehicle departs from the intended path of travel and is within 6 ft of an exposed employee **or if an employee enters the traffic pattern**" (EEI, 2024, p. 20). That last clause is an incursion: the people move, not the energy. It is the only people-side event found in the reviewed frameworks. It is allowed for one hazard, where the source-centric definition most visibly fails.

The AI model makes the event **two-sided** ([sif-definition.md](sif-definition.md), section 3). The separation between energy and people can shrink from either end:

- **Excursion:** the energy changes. It is released, moves off its path or builds up. A load drops; a crane overturns; a vehicle leaves its route.
- **Incursion:** people enter the line of fire of **live** energy. Live energy is energy in its working state, with a line of fire to people. A suspended load is live until it is landed, and a running machine is live until it is stopped and isolated.

Three consequences follow for observations:

1. **Presence in the line of fire is the incursion.** A person seen under a suspended load got there somehow. The entry happened whether or not anyone saw it, so the observation records an event.
2. **Planned presence within reach of the energy is exposure.** Tag-liners at the edge of a lift are within reach of the crane, but not in the drop path the lift keeps clear. Their position is a condition. It becomes an event only if the crane fails, and that is an excursion.
3. **An excursion with no one nearby is still an event.** A load that drops into an empty, barriered area is an excursion with a wide closest approach. EEI's proximity test would not count it.
4. **When the person is the energy, the event is an excursion.** At an edge, the source (the person's mass at height) and the target are the same. A person cannot enter their own line of fire. Like any energy source, they can only leave the space in which they can operate safely. The fall is the signature of that excursion. Fall protection does not turn a fall into an exposure: an arrested fall is still an excursion, and the harness is recorded in the control state, as PSIF (protected). Vehicle occupants are the same case: the event is the vehicle leaving its route.
5. **A barrier that keeps people back from an edge marks a danger zone.** Crossing it, or defeating a travel-restraint lanyard, is an incursion into that zone and so an event. No one else's energy is needed. A person cannot enter their own line of fire, but they can enter the danger zone drawn around it. If they then fall, that is a second event, an excursion. Working at an edge where no zone is drawn is exposure.
6. **Losing balance is an excursion, whatever the setting.** A person who trips and falls on a platform has left the space they need to keep their balance. The nearby edge does not change that event. Its potential is that of a fall on the same level, and the person's exposure to a fall from height carries on as before. If one continuous happening takes them over the edge, that is the fall-from-height event. EEI codes a trip that ends 2 ft from an unguarded edge as a high-energy incident (EEI, 2024, Appendix 6, Case A). The AI model codes it as a same-level fall, alongside a continuing SIF exposure.

| Observed | AI model | EEI SCL |
|---|---|---|
| Worker under a suspended load; load holds | Incursion: **PSIF** (unprotected) | Exposure (no release) |
| Hand past an open guard into a running conveyor; no contact | Incursion: **PSIF** (unprotected) | Exposure (no release) |
| Visitor crosses the edge of the lift exclusion zone, clear of the drop path | Incursion, closest approach wide: **PSIF** (unprotected) | Exposure (no release) |
| Pedestrian walks into a live traffic route | Incursion: **PSIF** | Incident (the traffic exception) |
| Load drops into an empty, barriered area | Excursion, closest approach wide: **PSIF** | No incident (no one within 6 ft) |
| Crane overturns; tag-liners within the collapse radius, unhurt | Excursion: **PSIF** | PSIF or Capacity |
| Tag-liners at the edge of a lift; crane intact | Condition: **SIF exposure** | Success or Exposure |
| Work at an unguarded edge; no fall | Condition: **SIF exposure** (unprotected) | Exposure |
| Fall arrest in use at height | Condition: **SIF exposure** (protected) | Success |
| Worker slips at the edge; harness arrests the fall | Excursion: **PSIF** (protected) | Capacity |
| Worker climbs over the edge barrier, or unclips a travel restraint, to reach the edge; no fall | Incursion into the edge danger zone: **PSIF** (unprotected) | Exposure (no release) |
| Worker at 25 ft trips and falls on the platform, 2 ft from an unguarded edge | Excursion: a fall on the same level, judged on its own potential; SIF exposure to the edge continues | High-energy incident (Case A) |
| Guard missing on a running machine; no one nearby | Condition: **SIF exposure** (unprotected), if a person could be within reach during intended use or reasonably foreseeable misuse (Guide 51, 3.6–3.7); otherwise hazard only | Exposure (no person needed) |

### 4.3 Triggers, exposure and the receiving environment

Three things meet in any event, and the AI model keeps them apart. They follow the direction of the energy flow, and each maps to one term of the risk equation: risk = exposure × chance per unit of exposure × consequence.

| | Where it sits | Examples | Risk term | Counts toward SIF potential? |
|---|---|---|---|---|
| **Trigger** | Upstream: it starts the excursion or incursion | A lifting lug to trip on; a wet deck; a gust of wind; a jam that invites a reach-in | Chance | **No.** Potential assumes the energy reaches the person anyway (separation to zero) |
| **Exposure** | Energy and people together | Work at a 25 ft unguarded edge; tag-lining a lift | Exposure | It is what is being judged |
| **Receiving environment** | Downstream: where the energy is delivered | Rebar or plant below; a hard surface; a sharp edge to strike | Consequence | **Yes.** It can raise the potential: a fall onto rebar |

A trigger is not a precursor in itself. On its own, a lifting lug threatens no more than a fall on the same level. The **precursor** is the unprotected SIF exposure: the unguarded edge. The trigger is what could set it off. Together they are the pattern Reason (2000) describes, in which latent conditions "combine with active failures and local triggers to create an accident opportunity".

EEI's Case A, read this way:

- **Trigger:** the lifting lug.
- **Precursor:** the unguarded edge at 25 ft. This is SIF exposure, with the control state recorded separately (the worker's fall arrest made it protected).
- **Event:** the trip, a fall on the same level.
- **Receiving environment:** the platform. It would have been the ground below only if the fall had continued over the edge.

The edge is not part of the receiving environment for the trip. It is not downstream of the trip; it is the boundary of the height exposure.

For coding, triggers are worth recording alongside a SIF exposure. They do not change its potential. But a SIF exposure (unprotected) with a trigger present is the precursor most likely to turn into an event. That is how a panel decides which to fix first.

### 4.4 How conditions are labelled

| Source | Label for an observed condition | Is it called PSIF? |
|---|---|---|
| BST (Krause, 2010) | Precursor. Data come from "injuries, near misses, safety observations and audit findings" | No |
| Martin & Black (2015) | Precursor: "conditions, behaviors, practices, exposures, situations" | No |
| Campbell (2018) | Precursor: "a condition or behavior that if left uncorrected could lead to a serious injury or fatality" | No |
| EEI SCL (2024) | **Exposure** (no direct control) or **Success** (direct control). Exposures "are often referred to as good catches, stop work, or at-risk observations" | No: PSIF requires a release of energy |
| Busch et al. (2021) | "proactive/positive precursors that are found in observations of work performed safely" | No |
| Cooper (2019) | "a potential SIF is an unsafe behaviour and/or unsafe condition that could feasibly and reasonably have resulted in a life-threatening or life-altering injury" | **Yes** |

Only one published source found applies the PSIF label to conditions: Cooper (2019). It describes itself as an "opinion piece". It also recommends a "potential SIF metric (i.e. number of potential SIFs/hours worked)", the kind of rate EEI later argued against.

### 4.5 Why it matters

Coding conditions as PSIF has four consequences:

1. **It mixes two different populations.** An event says the separation shrank: the energy moved, or people moved into its line of fire. A condition says it could. Their counts move for different reasons: events with the hazard, conditions with how hard people look.
2. **It inflates the count with whatever is easiest to observe.** Busch et al. (2021) found "about five times as many low and moderate severity unsafe observation findings as high or life-threatening findings", with findings "heavily focused on easily observable behaviors and conditions, such as general PPE use".
3. **It risks overinclusion.** "any incident or condition could be classified as a PSIF … This increases the potential for overinclusion, which waters down SIF-specific learning" (Hallowell & Spencer, 2024).
4. **It makes positive observations look like near misses.** A positive observation (a direct control in place) is EEI's Success. Labelling it PSIF records good practice as a failure.

The conditions themselves are valuable. EEI argues that "if organizations wish to become predictive, they must collect success observations … Studying HSIF, PSIF, and LSIF alone would not reveal relevant predictors" (EEI, 2024). The problem is the label, not the data.

## 5. Measuring controls

### 5.1 High-energy control assessment (HECA)

HECA turns condition observations into a measure: "the percentage of high-energy hazards with a corresponding direct control … every condition observation is modeled only as success … or exposure". In formula form, HECA = success / (success + exposure). "A control is only considered present when it is installed, verified and used properly. If the control is not installed properly, inspected on schedule, maintained regularly or is misused, the control is considered absent" (Oguz Erkal & Hallowell, 2023).

When it was introduced, its authors were candid: "It is unclear whether HECA is predictive of future performance because it has yet to be empirically tested" (Oguz Erkal & Hallowell, 2023).

### 5.2 The evidence so far

One study links control presence to SIF rates. Bayona et al. (2026) collected HECA and pre-job briefing data from 31 utility and construction firms over three months in 2024, against 22 months of injury records. They found that "Every percentage increase in a company's baseline HECA score reduces the expected number of SIFs by 3%, holding work hours constant (CI = -0.05, −0.001; p = 0.03)". Inter-rater reliability for HECA was "ρ (55) = 0.68 … moderately strong reliability".

The authors list the limits:

- "this study did not establish causal relationships. It is unclear whether fewer injuries originate from more Direct Controls … or whether firms have more Direct Controls … because of lower injury rates";
- the firms were "a large, convenient sample of volunteering companies";
- 18 of the 22 months of injury records came before the HECA measurement;
- higher pre-job briefing quality was associated with *more* long-term SIFs, an unexpected result.

The 87% figure often cited for observations is weaker than it sounds. Martin & Black (2015) "examined a sample size of 55 SIF/SIF-potential cases and confirmed that the SIF precursors … would be discoverable through interviews and/or observations in 87% of the cases". That is analysts' judgement about past cases, not a test of an observation programme. It is sometimes attributed to Krause (2012), an unpublished presentation (Cooper, 2019).

### 5.3 Warning signs or performance indicators?

Hopkins (2009) separates the two uses:

> "These precursor events may be quite unusual and quite ill-defined and therefore not suitable for use as performance indicators … while there is very good reason to seek out and respond to precursor events, this is not necessarily an argument for treating them as performance indicators."

His alternative is to measure controls: "process safety indicators must be chosen so as to measure the effectiveness of the controls upon which the risk control system relies. Whether they be described a lead or lag is ultimately of little consequence" (Hopkins, 2009). ICMM makes the same point: "The absence of accidents or incidents must not be taken as evidence that controls are working adequately", and leaders should not "assume critical controls are working well in practice without direct observation and other evidence" (ICMM, 2015).

EEI applies the same caution to its own categories: "The SCL model should be used for learning, not for the creation of new metrics … PSIF rates are not unidirectional" (Hallowell & Spencer, 2024).

Practice is mixed. The Campbell Institute (2022) reports that Dow set an "annual reduction target for non-injury pSIFs", described as "a common leading indicator". It also notes that "some organizations choose not to create goals around SIF metrics because of the potential that these goals might create fear and discourage reporting". On capture, it reports that "including a check-box option to report a pSIF on an incident report has proven effective". That is the kind of reporter-selected label that section 4 recommends replacing with coding by content.

### 5.4 Measure where the uncertainty is

Coverage tells you how often a direct control was in place. It does not tell you where to look next. Hubbard (2014) treats a measurement as a reduction in uncertainty, made to inform a decision. It is worth its cost only where the uncertainty is large and could change the decision. Measuring everything evenly tends towards what he calls the *measurement inversion*: effort goes to what is easy to measure, not to what is worth measuring.

Applied to SIF-capable work:

- **The decision** is the scenario's risk evaluation: is the remaining risk tolerable, so that the work can go ahead ([sif-scenario-index.md](sif-scenario-index.md) §3)?
- **The uncertainty** sits in the controls. Each direct control on a SIF-class hazardous event has an effectiveness distribution, not a yes or no. A fixed guard is well known; a procedure is wide.
- **What to measure first** is the control whose uncertainty could change the decision, found by the value of information. A few observations of a wide, decision-critical control are worth more than many of a control already known to work.
- **Each observation updates** the distribution of the control it checks, and the scenario's evaluation is revisited with less uncertainty.

Coverage (HECA) stays as the summary measure. Value of information decides where the assurance effort goes. It is more focused and cheaper than even sampling.

## 6. Critiques

- **Bimodal thinking.** Safety-II argues that "Safety-I promotes a bimodal view of work" and that "seeing deficiencies in hindsight does nothing to explain the generation or persistence of those deficiencies" (Hollnagel, Wears & Braithwaite, 2015). A control coded only as present or absent is a bimodal construct. Recording control state does not explain why a control was absent; that needs investigation.
- **Hindsight in "not complied with".** Judging that a control was "not complied with" is often a judgement made after the outcome is known. The counterfactual paper discusses the bias ([sif-counterfactual.md](sif-counterfactual.md)). The narrow definitions (a fitted guard; a verified isolation) reduce the room for it.
- **Compliance as a control.** The precursor schools' "not complied with" counts behaviour as a control. The bowtie and EEI definitions do not, because a control that depends on unerring behaviour fails with ordinary error.
- **Weak evidence.** The definitions rest on expert consensus, the 87% claim on retrospective judgement, and the control–SIF link on one correlational study.

## 7. The AI model's position

The AI model's definitions ([sif-definition.md](sif-definition.md)) take a clear position on each question in this paper:

1. **Potential is judged with all controls set aside** (separation to zero). This matches the second group in section 2: Campbell 2018, ICMM, ARMS's first question and EEI's energy test. How dangerous the energy was does not depend on whether a guard was there.
2. **Control state is a qualifier**, as in EEI SCL and ARMS's second question. For events it is PSIF *protected* (EEI Capacity), *unprotected* (EEI PSIF) or *unverified*.
3. **Events are two-sided.** An event is an excursion (the energy changes) or an incursion (people enter the line of fire of live energy). Both can be PSIF. EEI counts only releases, plus traffic incursions (section 4.2).
4. **Conditions are not PSIF.** Exposure is a condition, judged without regard to controls, and an exposure to SIF-level energy is *SIF exposure*. With the control state it maps onto EEI: SIF exposure (unprotected) is Exposure, and SIF exposure (protected) is Success. SIF exposure (unprotected) is also a precursor in the BST and Campbell sense.
5. **Only direct controls earn credit**, as in EEI, the bowtie barrier criteria and ICMM critical controls.
6. **Credit for controls comes through modelling them**: the residual distribution, including the chance that each control is degraded or absent. This goes beyond the binary present/absent of SCL and HECA.

This crosswalk sets the precursor schools' definitions against our terms:

| Precursor-school term | AI model term |
|---|---|
| Precursor: high-risk situation + controls absent, ineffective or not complied with | SIF exposure (unprotected), a condition, where "controls" means direct controls |
| Near miss with SIF potential, barrier worked (Campbell 2018) | PSIF (protected) |
| Near miss with SIF potential, no barrier | PSIF (unprotected) |
| Positive precursor (Busch et al., 2021) | SIF exposure (protected) |

Against EEI SCL, the terms map one to one, but the populations differ:

| EEI SCL | AI model | Where the counts differ |
|---|---|---|
| PSIF | PSIF (unprotected) | The AI model adds incursions, and excursions with no one within 6 ft |
| Capacity | PSIF (protected) | As above |
| Exposure | SIF exposure (unprotected) | EEI includes hazards with no one nearby; the AI model needs a person within reach now, or foreseeably in intended use or misuse. Incursions EEI records as Exposure are PSIF in the AI model |
| Success | SIF exposure (protected) | EEI includes hazards with no one nearby |

**"Unverified" is kept.** EEI and HECA treat a control that is not verified as absent: "the control is considered absent" (Oguz Erkal & Hallowell, 2023). ICMM similarly warns against assuming controls work "without direct observation". No reviewed framework has a separate "unverified" state. The AI model adds one because incident narratives often do not say whether controls were in place. Treating silence as absent would overstate control failures and hide the gap in reporting. The rules are:

- **Record unverified when the report does not say** whether a direct control was in place or worked. Example: "Scaffold board fell into the barriered area below", where nothing shows whether the exclusion zone held.
- **Map unverified to unprotected** whenever a yes/no is needed: for EEI comparisons, and in HECA-style scores, where it counts as exposure.
- **Treat unverified as a work queue.** The panel should establish the control state before drawing lessons from the case.
- **Report the unverified share.** If it rises, reports are not capturing control state, and that is itself a finding.

## 8. Research agenda

- **Test whether control state is reliably coded.** Measure agreement on direct control present, absent or unverified, in narratives and in observations, separately from agreement on potential. Track how often the panel resolves unverified cases, and which way they fall.
- **HECA over time.** Collect HECA prospectively, before the injury window, to test the forward link that Bayona et al. (2026) could not.
- **Observation bias.** Measure how much of the observation stream is the "easily observable" low-severity findings Busch et al. (2021) describe, before and after restricting control credit to direct controls.
- **Incursions.** Measure the share of PSIF that are incursions, and agreement on whether the energy was live and the person was in its line of fire. This is the population EEI does not count.
- **Precursor versus PSIF counts.** Compare trends in condition-based (SIF exposure) and event-based (PSIF) counts at the same sites, to see whether they move together.
- **Residual versus binary credit.** Compare the binary present/absent classification with the residual distribution from modelling controls, on the same events.

## References

- ARMS Working Group (2010). *The ARMS Methodology for Operational Risk Assessment in Aviation Organisations*, v4.1, March 2010.
- Bayona, A., Hallowell, M.R., Bhandari, S. & Raheemy, Y. (2026). Balanced approach to SIF prevention: exploring empirical relationships between short and long-term measures of safety performance. *Journal of Safety Research*, 98, 175–189. doi:10.1016/j.jsr.2026.06.007
- Busch, C., Usrey, C., Loud, J., Goodell, N. & Carrillo, R.A. (2021). Serious injuries & fatalities: why are they constant while injury rates decrease? *Professional Safety*, 66(1), 26–31.
- Campbell Institute (2018). *Serious Injury and Fatality Prevention: Perspectives and Practices*. National Safety Council.
- Campbell Institute (2020). *Designing Strategy for Serious Injury and Fatality Prevention*. National Safety Council.
- Campbell Institute (2022). *Serious Injury and Fatality Prevention: Leading Indicators, Cumulative Risk and Safety Networks*. National Safety Council.
- CCPS & Energy Institute (2018). *Bow Ties in Risk Management: A Concept Book for Process Safety*. Wiley / AIChE.
- DEKRA (2019). Precursor definition, as quoted in Campbell Institute (2022).
- Cooper, M.D. (2019). The efficacy of industrial safety science constructs for addressing serious injuries & fatalities (SIFs). *Safety Science*, 120, 164–178. doi:10.1016/j.ssci.2019.06.038
- Edison Electric Institute (2024). *Safety Classification and Learning (SCL) Model*, rev. September 2024.
- Hallowell, M.R. & Spencer, C. (2024). Safety classification & learning model: defining & classifying potential serious injuries & fatalities. *Professional Safety*, 69(2), 18–26.
- Hollnagel, E., Wears, R.L. & Braithwaite, J. (2015). *From Safety-I to Safety-II: A White Paper*. Resilient Health Care Net.
- Hubbard, D.W. (2014). *How to Measure Anything: Finding the Value of "Intangibles" in Business*, 3rd ed. Wiley.
- Hopkins, A. (2009). Thinking about process safety indicators. *Safety Science*, 47(4), 460–465. doi:10.1016/j.ssci.2007.12.006
- ICMM (2015). *Critical Control Management: Implementation Guide*. International Council on Mining and Metals.
- ISO 12100:2010. *Safety of machinery – General principles for design – Risk assessment and risk reduction*.
- Johnson, M., Manton, M., Cowley, C. & Scanlon, M. (2018). Bow ties in risk management: using the new CCPS-EI book to avoid pitfalls. *IChemE Hazards 28*, Symposium Series 163.
- Krause, T.R. (2010). *New Findings on Serious Injuries and Fatalities: Validating a New Paradigm to Inform Prevention Initiatives*. BST.
- Manton, M. et al. (2017). Representing human factors in bow ties as per the new CCPS/EI book. *IChemE Hazards 27*, Symposium Series 162.
- Martin, D.K. & Black, A.A. (2015). Preventing serious injuries & fatalities: study reveals precursors & paradigms. *Professional Safety*, 60(9), 35–43.
- Oguz Erkal, E.D. & Hallowell, M.R. (2023). Moving beyond TRIR: measuring & monitoring safety performance with high-energy control assessments. *Professional Safety*, 68(5), 26–35.
- Reason, J. (2000). Human error: models and management. *BMJ*, 320(7237), 768–770.

## Verification notes

*For the author; remove before publication.*

- **Quotation check (2026-11-17).** Every quotation in sections 1 to 8 was matched word for word against the source text, allowing for line breaks and hyphenation in the PDFs. The executive summary repeats quotes from the body. No quotation is unverified. One wording was tightened: section 6 now quotes "not complied with", where it had loosely quoted "absent or not complied with".
  - **All source copies are in `docs/sif/lit/`.** Those obtained online during research were added on 2026-11-17: `eei_scl_model_2024.pdf`, `hallowell_spencer_2024_scl_psj.pdf`, `iso_12100_2010_extract.pdf` (a 15-page extract, not the full standard), `busch_et_al_2021_psj.pdf`, `cooper_2019_safety_science.pdf`, `hopkins_2009_safety_science.pdf`, `hollnagel_wears_braithwaite_2015_safety_I_II.pdf`, `reason_2000_bmj.html` (the PMC web page), `johnson_et_al_2018_hazards28_bowties.pdf` and `manton_et_al_2017_hazards27_bowties_hf.pdf`. Also added: `eei_heca_guide.pdf`, and `wachter_ferguson_2013_psj.pdf` (for the Cooper correction below).
  - **Page and section numbers** were checked only for EEI SCL (pp. 11, 19, 20) and CCPS & EI (2018) (p. 43, section 2.6.3).
- **Local file:** `docs/sif/lit/bayone_et_al_2026.pdf` is Bayona et al. (2026, *J. Safety Research*), the HECA study, not the 2023 LIFE paper.
- **EEI SCL (2024) quotes in section 4.2** are checked against the source text: the release and proximity definitions (p. 11), the glossary (p. 19), the traffic note (p. 20), and hazard #14 in the list of high-energy hazards.
- **DEKRA's own text** is not traceable. "Martin & Strikoff (2012)" could not be found outside DEKRA's own material, so DEKRA's definition is cited as DEKRA (2019) via Campbell Institute (2022).
- **Not obtained:** the ICMM *Good Practice Guide*; ANSI/ASSP Z16.1-2022; Conklin (2019); Hollnagel (2014); Reason (1997); Bayona et al. (2024, *JCEM*).
- **Corrections carried into this paper:**
  - Campbell (2020) reads "not in and of itself a precursor".
  - The 87% figure is from Martin & Black (2015), not Krause (2012).
  - Cooper's quoted Wachter & Ferguson (2013) definition was not found in their article, so it is not used.
- **ISO 12100 wording** is as verified for the definitions documents.
