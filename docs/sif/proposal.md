# Proposal: a defined, reliable PSIF for QQ

Status: draft v0.4, 2026-11-19. For QQ's safety director and EHS leadership. Internal.

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-11-18 | First draft |
| 0.2 | 2026-11-19 | §4.1: triagers ask the definition's question in plain words (QQ-FB-05). §4.4: stakeholders, their decisions and what they need (QQ-FB-06). §5.2: thresholds fixed by the definition (QQ-FB-03). Rec. 6: reporters give facts only (QQ-FB-05) |
| 0.3 | 2026-11-19 | §4.1: missed-case audit, a monthly random sample of unflagged reports sentenced by the panel. §7: miss rate added as a measure (QQ-FB-10) |
| 0.4 | 2026-11-19 | §4.1: the panel works in the reporting layer; its classification is the classification of record there, joined to IMR by the report key; one source of truth for reporting (QQ-FB-11) |

This proposal draws on four papers, which are available on request and referenced throughout:

| Paper | Question it answers |
|---|---|
| [SIF definitions and coding rules](sif-definition.md) (with its [discussion](sif-definition-discussion.md)) | What exactly is PSIF, and how is it coded, step by step? |
| [Counterfactuals](sif-counterfactual.md) | How do people judge "what could have happened" reliably? |
| [Controls, precursors and conditions](sif-controls-precursors.md) | Where do controls and observations fit? |
| [What counts as serious](sif-severity.md) | Which injuries count as serious, and how is that measured? |

---

## Summary

**The ask.** Give PSIF a single written definition, decide it with a fixed question answered by calibrated people, and record controls and observations separately. The classification stays human. An independent AI model, developed by Spongl Ltd, can screen every report and act as a second reader, but nothing in the human process depends on it.

**Why.** QQ's PSIF coding cannot currently be trusted to trend or compare, and the reasons are structural rather than a failing of the people doing it:

- **The question is open.** QQ's guidance asks how much worse an incident "realistically could have been, on average, if repeated several times", and in the same passage asks for "the most serious, credible outcome". Those are different questions, and each triager answers them in their own way.
- **The scale is mixed.** The one field in use carries two scales at once: a descriptor (Low to Very High) whose meanings are not defined, and a treatment scale (First Aid to Fatal) that measures harm, not potential.
- **Observations dominate.** 69% of FY2027 PSIF ratings are on positive and hazard observations, which are mostly conditions, not events.
- **The ratings don't track the energy.** On 3,006 QQ reports, the human potential rating and an energy-based reading of the same narratives barely line up (section 1.4).
- **The research says open questions fail.** Practitioners rating potential this way agree 64% of the time, where 50% is a coin toss, and knowing the outcome moves expert judgements by about 30 percentage points.

**What changes.**

| | Today | Proposed |
|---|---|---|
| **The definition** | "How much worse could this have been?" | Would the energy, had it reached a person with no protection, more likely than not have caused a life-ending, life-threatening or life-altering injury? |
| **What is counted** | Every report type, including observations | Events (something happened) as PSIF; conditions (nothing happened) as SIF exposure |
| **Controls** | Mixed into the rating, case by case | Recorded separately: protected, unprotected or unverified |
| **Who decides** | The triager, at triage | A calibrated monthly panel; triage becomes a generous filter |
| **Severity** | A mix of descriptors and treatment levels | LIFE: life-ending, life-threatening or life-altering, with hands, fingers and eyes counted |
| **Use** | Counted and trended as reported | Used for learning now; trended once the panel is calibrated; never targeted |

**Recommendations** (section 6): adopt the definition; code observations by content; record control state; simplify triage; set up a calibrated panel; ask reporters for the size of the energy; count life-altering injuries at the hand and eye; use PSIF for learning, not targets; pilot the AI model as a screen.

---

## Contents

- [1. Where QQ is today](#1-where-qq-is-today)
- [2. The definition](#2-the-definition)
- [3. The evidence behind it](#3-the-evidence-behind-it)
- [4. How people use it](#4-how-people-use-it)
- [5. The AI model](#5-the-ai-model)
- [6. Recommendations](#6-recommendations)
- [7. Pilot and measures](#7-pilot-and-measures)
- [Annex A. QQ's own examples, recoded](#annex-a-qqs-own-examples-recoded)
- [Annex B. Glossary](#annex-b-glossary)

---

## 1. Where QQ is today

### 1.1 The guidance

QQ's *Environment, Health & Safety Incident Classifications and Definitions* sets out the purpose of potential consequences well:

> The purpose of assessing potential consequences is to ensure that [QQ] responds appropriately to incidents with minor actual consequences, but high potential consequences, especially in the rigour with which they are investigated. A Near Miss that was a potential fatality must be investigated with the same rigour as if it were a fatality.

This proposal keeps that purpose. The difficulty is the method. The guidance asks for three things that pull in different directions:

1. **An average:** "how much worse the consequences of the incident realistically could have been, on average, if repeated several times".
2. **A worst credible case:** "the most serious, credible outcome of the event that can reasonably be foreseen".
3. **An imagined change:** "how much worse could this have been, on another day, in different circumstances?"

An average and a worst credible case give different answers for the same event. "On another day" leaves the triager to choose which circumstances to change. The guidance's own examples show this: a 240 V shock is a potential fatality because "a small change in body position … would have changed the current path", which is a single-factor change, while a 1 m fall is not, because a fatality is "not impossible here, merely unlikely", which is a likelihood judgement. Both are reasonable, but they apply different tests (Annex A).

### 1.2 The fields and scales

Since 2022, PSIF has been coded in three different IMR fields (a fourth, in the investigation form, has never been used), and the main field in use has changed several times. Today only *Triage/Potential Severity Rating_1* is in use. Counts in one field cannot be trended against counts in another.

That field carries two scales, and every value is shown to the triager because different report types use different scales:

| Scale | Values | What it measures | FY2027 ratings |
|---|---|---|---|
| Descriptor | Low, Medium, High, Very High | Not defined | 1,225 (89%) |
| Treatment | First Aid, Restricted Workday, Medical Treatment, Lost Workday, Fatal | Harm to a person, by treatment | 144 (11%) |

A treatment scale is an outcome scale. It describes how an injury was treated, so as a measure of *potential* it borrows the language of what happened. The descriptor scale has no stated meaning, so "High" on one site may be "Medium" on another.

### 1.3 What is being rated

FY2027 ratings by report type:

| Report type | Ratings | Share |
|---|---|---|
| Hazard and observation | 716 | 52% |
| Positive observation | 232 | 17% |
| Near miss | 209 | 15% |
| Injury | 98 | 7% |
| Property damage | 68 | 5% |
| Environmental | 46 | 3% |
| **Total** | **1,369** | |

Over two-thirds of PSIF ratings are on observations. Twelve ratings are "Fatal", and eight of those are on observations, including one positive observation. A positive observation records work done safely: it may show energy with SIF potential, but nothing happened. These ratings answer a real question (how dangerous was what was seen?), but it is a different question from PSIF, and counting both together makes the PSIF total mean little (section 3.2).

### 1.4 Do the ratings track the energy?

The AI model (section 5) read 3,006 QQ report narratives and assigned each a probability of serious injury from the energy described. Set against the human rating in the earlier potential-severity field (intermediate levels omitted):

| Human rating | Reports | AI model: SIF | Elevated | Not SIF |
|---|---|---|---|---|
| Fatal | 89 | 25% | 18% | 56% |
| High | 265 | 14% | 23% | 63% |
| Not SIF potential | 2,600 | 10% | 19% | 71% |

If both were measuring the same thing, the rows would look very different from one another. They don't: a human "Fatal" and a human "Not SIF potential" have similar energy profiles. That does not show which reading is right. The model can only use what the narrative says, and only about 3–5% of narratives state a height, voltage, speed or weight. Human triagers may know more than they write. But it does show that the current rating is not a consistent measure of the energy involved, and that about 260 reports rated *not* SIF potential describe energy the model reads as more likely than not to cause serious injury.

### 1.5 What the research says

These are not QQ-specific problems. They follow from asking an open question:

- **Agreement is barely better than chance.** When practitioners rate potential with an open question, they agree 64% of the time, where 50% is a coin toss (EEI). In healthcare, comparable "could this have been prevented?" judgements average about 0.33 reliability.
- **The outcome leaks in.** Changing only the outcome of otherwise identical cases moves expert judgements by about 30 percentage points.
- **Critics and promoters agree on the cause.** Busch calls PSIF "arbitrary" and "subjective". EEI, which promotes it, says that asked openly the answer is "inevitably 'yes' if the analyst is creative enough".
- **Broad definitions water down learning.** Letting "any incident or condition" be PSIF causes "overinclusion, which waters down SIF-specific learning" (Hallowell & Spencer, 2024).

The subjectivity comes from leaving the question open, not from PSIF itself ([Counterfactuals](sif-counterfactual.md), executive summary).

---

## 2. The definition

> **SIF potential.** An event has SIF potential when the energy involved, had it reached a person with no protective control between them, would more likely than not have caused a life-ending, life-threatening or life-altering injury.
>
> **PSIF.** An event with SIF potential and no serious injury. Every PSIF is recorded with its control state: *protected* (a direct control worked), *unprotected* (no effective control) or *unverified* (the report does not say).
>
> **SIF exposure.** A condition, not an event: a person within reach of energy that would have SIF potential if an event occurred. When no direct control is in place it is a *precursor*.
>
> **Elevated potential.** As SIF potential, but a serious injury is credible rather than more likely than not.

Each phrase does a job:

| Phrase | What it settles | Section |
|---|---|---|
| "the energy involved" | Potential comes from the physics, not from imagination | 3.1 |
| "an event" | Something happened; a condition is SIF exposure | 3.2 |
| "had it reached a person" | Assess as if the separation went to zero, whoever was nearby | 3.3 |
| "with no protective control between them" | Controls are set aside when judging potential, and recorded separately | 3.3, 3.5 |
| "more likely than not" | One threshold, the same for everyone: the most likely outcome is serious | 3.3 |
| "life-ending, life-threatening or life-altering" | The LIFE definition of serious, including hands, fingers and eyes | 3.4 |

The full coding rules, with seven steps, a determination table and worked examples, are in [SIF definitions and coding rules](sif-definition.md), section 7. The definition maps onto the published schemes (EEI, Campbell Institute, BST/DEKRA, ASTM E2920), so QQ can still benchmark ([definitions](sif-definition.md), section 6).

---

## 3. The evidence behind it

### 3.1 Energy: potential comes from the physics

Every injury is energy reaching a body faster than it can tolerate (Haddon). Energy-based safety (Hallowell and colleagues) builds on this: the amount of energy, and what it lands on, set the range of likely outcomes. A 10 m fall onto concrete has a distribution of outcomes; the outcome on the day is one draw from it. A person who walks away from that fall was lucky, not safe.

This gives PSIF a fixed anchor. Instead of imagining "another day", the coder identifies the energy source, its type and size, and the receiving environment (what the energy would land on or strike: concrete, rebar, a sharp edge). Published thresholds and injury data then say how likely a serious outcome is. EEI's SCL model works the same way: "was high energy present?" comes first ([definitions](sif-definition.md), sections 4 and 5.1).

**Why it matters for QQ:** two triagers looking at the same report reach the same answer, because they are reading the same physics. It is also why the size of the energy must be reported (recommendation 6).

### 3.2 Event: something happened, from either side

PSIF is about **events**. An event is a change that shrinks the separation between energy and people, and it can start from either side:

| | **The energy changes** (excursion) | **The people change** (incursion) |
|---|---|---|
| **Examples** | A load drops; a crane overturns; a person loses their balance; a circuit is back-fed; pressure builds up | A person walks under a suspended load; a hand goes past an open guard into a running machine; someone crosses an edge barrier |

A **condition** is different: energy and people are together and nothing changes. Tag-liners holding a load from the edge of the zone, work at an unguarded edge, or work on isolated plant are conditions. They are **SIF exposure**, not PSIF.

Two points follow:

- **Code by content, not by report type.** A hazard observation of a person under a suspended load is an event: being there means they went in, whether or not anyone saw them enter. A positive observation of a correctly barriered lift is a condition. The report type does not decide; what was seen does.
- **Incursions count.** The best-developed published scheme (EEI's SCL) counts only *releases* of energy, so a hand in a running machine or a worker under a load that holds can only be recorded as exposure. This definition counts them as events. It is a deliberate departure, and it means QQ's PSIF counts will run higher than EEI's for the same work ([controls paper](sif-controls-precursors.md), section 4.2).

**Why it matters for QQ:** 69% of today's ratings are on observations. Under this definition, observations still count, but in the right column: events seen on observation cards are PSIF, and conditions are SIF exposure, which is the raw material for a leading indicator (section 4.4).

### 3.3 The counterfactual: one fixed question

Judging PSIF means judging something that did not happen. The research shows how to make that reliable: fix the question, and take out the parts that invite imagination ([Counterfactuals](sif-counterfactual.md)).

The question:

> *If this energy (as described) had reached a person, with no protection, would a serious injury be more likely than not?*

Three rules make it fixed:

1. **Separation to zero.** Assess as if the energy reached a person, however close anyone actually came. Distance, timing and luck are recorded as the *closest approach* (wide, close or contact), but they do not change the potential.
2. **Set controls aside.** Harnesses, guards, exclusion zones, PPE, training and evasive action count for nothing when judging potential. They are recorded separately (section 3.5).
3. **What-ifs are separate.** A chain of events the report does not show ("the fuel could have ignited") is recorded and labelled for investigation, but never changes the classification.

"More likely than not" replaces both the guidance's average and its worst credible case with one threshold everyone shares. Events below it but where a serious injury is still credible are recorded as **Elevated**, so they are not lost: Elevated cases are reviewed, and the panel can move them.

**Why it matters for QQ:** this is the change that addresses the 64% agreement problem. The guidance's purpose (investigate high-potential near misses with the same rigour as a fatality) is kept; the answer to "was it high potential?" no longer depends on who is asking.

### 3.4 Severity: what counts as serious

"Serious" follows the **LIFE** model (Bayona et al., 2023): an injury is serious if it is **life-ending, life-threatening or life-altering**. LIFE is the most widely adopted definition. EEI, ASTM E2920 and the Campbell Institute use it or close variants ([What counts as serious](sif-severity.md), sections 2–3).

Two findings matter for QQ:

- **Treatment levels are not severity.** Hospital admission, days lost and medical treatment depend on local practice and on the person. LIFE deliberately rejects them as a test, so the treatment scale should leave the potential field (recommendation 4).
- **Hands, fingers and eyes need deliberate attention.** Trauma scales rate injuries by threat to life, so a finger amputation or the loss of an eye scores low, though it changes a life. Published lists also disagree at the hand: RIDDOR counts any finger amputation, while EEI's 2025 criteria exclude finger fractures and most fingertip amputations. The recommendation is to follow LIFE: the loss of any body part, or any permanent loss of function, is life-altering (recommendation 7).

### 3.5 Controls: recorded, not mixed in

Published schemes disagree most about controls. A BST/DEKRA precursor is defined by controls being "absent, ineffective, or not complied with"; EEI's PSIF needs both high energy and no direct control; Campbell's SIF potential ignores the barriers that worked. Counts made under one cannot be compared with counts under another ([controls paper](sif-controls-precursors.md)).

The proposal keeps two questions apart:

1. **How dangerous was the energy?** That is the potential, with controls set aside.
2. **Were the controls there, and did they work?** That is the control state, recorded alongside.

| Control state | Meaning | Example |
|---|---|---|
| **Protected** | A direct control stood between the energy and the person, and worked | A harness arrested a 5 m fall |
| **Unprotected** | No direct control, or it failed or was defeated | A guard was open and the belt running |
| **Unverified** | The report does not say | Most reports today |

Only **direct controls** count: aimed at the energy, verified in place, and effective even if someone makes an unintentional error. Training, rules, signs, general PPE, evasive action and luck are not direct controls. "Unverified" is not treated as absent: the panel resolves it before lessons are drawn, and its share is itself a measure of reporting quality.

**Why it matters for QQ:** the same split gives every published category as a view of the same data. "All PSIF" is Campbell's count; "PSIF (unprotected)" is EEI's. A protected PSIF says the control did its job, which is good news to learn from, not a near-failure to hide.

---

## 4. How people use it

The process is human. It needs no change to IMR beyond tidying the triage scale.

### 4.1 Three stages

| Stage | Who | Job | What it produces |
|---|---|---|---|
| **1. Triage** | Triagers, as now | A coarse filter, with a deliberately low bar: "if in doubt, flag". Triagers ask the definition's question in plain words (definitions §7.1), uncalibrated | A list of candidate cases. The rating is the triager's view, not the decision |
| **2. Screening** (optional) | The AI model | Reads every report and puts forward likely PSIF that triage did not flag; gives a second reading of every flagged case | Added candidates, with energy, band, probability and a one-line rationale |
| **3. Sentencing** | A calibrated panel of four or five people from across the operation, monthly | Answers the fixed question for every candidate | The classification of record, with control state |

**Missed-case audit.** Each month the panel also sentences a small random sample of reports that neither triage nor the AI model put forward. It estimates the miss rate, with its uncertainty, without depending on either filter. A few dozen cases a month are enough to show whether misses are rare or common (Hubbard). If the rate is low, the sample stays small. If it is high, the triage bar or the screen needs attention. The sample size is set in the pilot.

**Where the panel works.** The panel's inputs and outputs flow through the reporting layer, an Excel workbook on the ETL and Power BI side. Its list of cases to sentence (triage flags, the AI model's candidates and the audit sample) comes from the workbook. Members open the IMR record for the details of each case, and their classification is recorded back in the workbook, joined to the IMR record by its report key. IMR needs no new field. This follows the two-tool boundary recommended in the IMR gap analysis (action QQ-01): Power BI for corporate reporting and decisions, IMR for capture, case workflow and site-level views.

The classification of record is the panel's decision, held in the reporting layer. The IMR triage rating stays as the triager's view, captured with the case, and the IMR-derived rating is kept alongside for comparison. Reporting reads one place. Triagers are no longer asked to decide PSIF at the point of triage, when they have the least information and the outcome is freshest.

### 4.2 The panel's answer

Each member answers the fixed question **yes or no, with their confidence**, from 50% (a guess) to 100% (certain). For example: *Yes, 70%*. "Yes, 70%" counts as 0.70 and "No, 70%" as 0.30; the panel's answer is the average, and 0.5 or more is PSIF. Split cases are discussed before recording.

This is the format used to train and measure calibrated assessors (Hubbard). Averaging judges is itself a gain: in healthcare, a reliable potential-severity score needed at least four judges per case. Where facts are missing, members give a range ("the fall was between 1.5 and 4 m") and answer accordingly, rather than guessing a single value.

### 4.3 Calibration

Panel members are trained and then tested periodically on the same question, using cases whose answers are known from the injury data behind the energy bands (for example, "a 3–6 m fall onto concrete"). Each member's calibration is tracked with a Brier score. Members learn what "70% confident" really means, and the organisation can show that its PSIF decisions are measured, not just made.

### 4.4 What gets reported

| Measure | What it shows | Use |
|---|---|---|
| **PSIF**, by control state | Events where a serious injury was the likely outcome | Learning from what nearly happened; investigation rigour |
| **SIF exposure**, by control state | Conditions seen on observations, protected or not | The share protected is a leading indicator of control coverage (EEI's high-energy control assessment) |
| **Elevated** | Credible but not likely serious | Review queue |
| **Closest approach** | Wide, close or contact | Separates a dropped load in a cleared zone from one that missed someone by a metre |
| **Unverified share** | How often reports don't state controls | Reporting quality |

PSIF and SIF exposure are reported side by side, never summed.

**Who uses what.** Each stakeholder gets the measures for the decision they make. The complexity sits with the panel and the analysts. This table is a proposal to confirm during the pilot.

| Stakeholder | Decision they make | What they need | What they don't need |
|---|---|---|---|
| **Senior leaders** | Where to put attention and resource; whether SIF-capable work is under control | SIF actual; PSIF (unprotected); the share of SIF exposures that are protected | Bands, probabilities, Elevated |
| **Safety director and EHS leadership** | Programme priorities, such as a critical-control campaign by energy type | PSIF and Precursor by mechanism and control state; the unverified share | Individual cases |
| **Site and operational managers** | What to fix locally: which work, which control | Their open Precursors and control failures, case by case | Trends across sites |
| **Investigators** | How deep to investigate | The PSIF class, closest approach and control state on each case | Aggregates |
| **Safety analysts** | Whether the data can be trusted and trended | Everything: Elevated, bands, panel calibration (Brier scores), agreement between the panel and the AI model | (none) |

---

## 5. The AI model

### 5.1 What it is

The AI SIF classification model was developed by Spongl Ltd, independently of QQ's process.[^spongl] It reads an incident narrative and produces, for each energy source:

- the **mechanism** (fall, struck by, caught in, electrical, and so on) and **energy type**;
- an **energy band** for the size of the energy described (for example, `fall/medium`, 3–6 m);
- a **probability of serious injury**, P(serious), from that band's calibrated outcome distribution;
- a **class**: SIF (P ≥ 0.50), Elevated (0.10–0.50) or Not SIF;
- a short **rationale**, so a person can see why.

It applies the same definition and rules as the human coders in section 2. The bands and probabilities are published in the coding rules ([definitions](sif-definition.md), Annex B), so humans and the model use the same scale.

### 5.2 Why it moves the discipline forward

| Today's practice | What the model adds |
|---|---|
| A label chosen from a pick list | A **probability**, from injury data, that the definition's thresholds turn into a class. Elevated cases are visible, not lost between "Medium" and "High" |
| Potential imagined case by case | Potential set **by rule**: separation to zero, controls set aside, energy physics. The same report gives the same answer every time |
| Different methods for different hazards | **One method across every energy type**, including chemical, thermal and breathing hazards |
| Controls mixed into potential | **Potential and controls kept apart**, so control effectiveness can be measured, not assumed |
| Events counted only when energy is released | **Two-sided events**: incursions into the line of fire count, closing the gap in EEI's scheme |
| Outcome definitions with no potential element | LIFE defines serious outcomes but leaves potential to "future work". The model's separation-to-zero test and band table **are that missing potential layer** ([severity paper](sif-severity.md), section 8) |
| Reliability unknown | Every case gets an **independent second reading**, and disagreements between the panel and the model become a structured learning conversation |

It also points to where the field is going: from yes/no flags to outcome distributions, and from there to *residual* potential, the outcome distribution with the controls as they were. That allows control effectiveness to be modelled rather than argued about ([discussion](sif-definition-discussion.md), section 4.4).

### 5.3 How people use it

- **As a screen.** Every report is read, not only those triage flagged. The panel sees the candidates triage missed.
- **As a second reader.** For each case the panel sentences, the model's reading is shown alongside. When they disagree, the panel discusses why. The model does not decide.
- **As a calibration source.** The band table supplies the known-answer questions for panel calibration (section 4.3).
- **As a reporting prompt.** When the model gives a low band because the narrative didn't state a height or voltage, that is a signal to ask the reporter.

### 5.4 Its limits, stated plainly

- **Thin narratives are under-called.** With no height, voltage or mass stated, the model takes the lowest band consistent with the text. Only about 3–5% of QQ narratives state a magnitude. Better reporting is the fix (recommendation 6).
- **Life-altering injuries are under-weighted.** Its scale measures threat to life, so hand, finger and eye injuries score lower than LIFE intends. The fix (a body-region rule now, an impairment axis later) is part of Spongl Ltd's development plan ([severity paper](sif-severity.md), sections 8–9). Until then, the panel applies the LIFE rule at the hand and eye.
- **Its agreement with people is not yet a measure of accuracy.** Human ratings agree with each other only about 64% of the time, so there is no reliable "right answer" to score against yet. A calibrated panel creates one. A pilot (section 7) measures agreement between panel and model on the same cases.
- **It does not see controls.** By design it judges potential only. Control state comes from the report and the panel.

---

## 6. Recommendations

1. **Adopt the definition.** Replace "how much worse could this have been, on average" with the definition in section 2 and the coding rules in [sif-definition.md](sif-definition.md). Keep the guidance's purpose: high-potential near misses are investigated with the rigour of the serious outcome they nearly were. *(Sections 1.1, 2, 3.3)*

2. **Code by content, not by report type.** An event seen on an observation card (a person under a suspended load, a hand past an open guard) is PSIF. A condition (work at an edge, a barriered lift) is SIF exposure, protected or unprotected, never PSIF. *(Sections 1.3, 3.2)*

3. **Record control state separately.** Protected, unprotected or unverified, counting only direct controls. Report PSIF and SIF exposure side by side, never summed, and report the share of SIF exposures with a direct control in place as a leading indicator. *(Sections 3.5, 4.4)*

4. **Make triage a filter, with one scale.** Remove the treatment levels (First Aid to Fatal) from the potential field; they describe outcomes, not potential. Give the descriptor scale stated meanings, or reduce it to "flag / don't flag". Ask triagers to flag generously. *(Sections 1.2, 4.1)*

5. **Set up a calibrated monthly sentencing panel.** Four or five people, answering the fixed question yes or no with confidence. The panel's average is the classification of record. Train and test members on known-answer cases, and track calibration with a Brier score. *(Sections 3.3, 4.2, 4.3)*

6. **Ask reporters for the size of the energy.** Facts only, no opinion or judgement: height, voltage, speed, weight, pressure, and what it would have landed on. These decide the answer, and today only about 3–5% of narratives give them. *(Sections 1.4, 3.1)*

7. **Count life-altering injuries at the hand and eye.** Follow LIFE: the loss of any body part, or any permanent loss of function or sight, is serious. Record the EEI criterion alongside for benchmarking. *(Section 3.4)*

8. **Use PSIF for learning, not targets.** Use it to find and investigate cases now. Trend it only once the panel is calibrated and its rules are fixed. Do not set targets for PSIF rates, which suppresses reporting. Never reclassify history after one serious event. *(Sections 1.5, 4.4)*

9. **Pilot the AI model as an independent screen and second reader.** Run it alongside the panel for the pilot period and review its value against the measures in section 7. Its use is a separate decision from recommendations 1–8, which stand on their own. *(Section 5)*

---

## 7. Pilot and measures

**Scope.** Three months of reports from one or two business units, sentenced by a panel trained before the pilot starts.

**What to measure:**

| Measure | Why |
|---|---|
| Agreement within the panel, per case | Shows whether the fixed question is working |
| Panel members' calibration (Brier score) | Shows whether confidence means what it says |
| Agreement between panel and AI model, and the reasons for disagreement | Tests the model as a screen and second reader |
| Cases the model put forward that triage missed, and how the panel sentenced them | The screen's value |
| Miss rate from the unflagged sample, with its uncertainty | What triage and the screen both miss |
| Share of PSIF *unverified* | Reporting quality on controls |
| Share of reports stating a magnitude | Reporting quality on energy |
| Panel time per case | Cost |

**Decision at the end of the pilot:** whether to extend the panel across QQ, whether to change IMR's triage field, and whether to continue the AI model screen.

---

## Annex A. QQ's own examples, recoded

The guidance's examples, coded with the rules in [sif-definition.md](sif-definition.md), section 7. Band values are from Annex B of that document.

| Guidance example | Guidance's answer | Proposed coding | Agree? |
|---|---|---|---|
| A person falls 10 m onto hard ground; first aid only | Credible potential fatality | Gravity, fall from height. `fall/high` or `fall/very_high` (6–20 m), P(serious) 0.50–0.90: **SIF potential**. No serious injury, so **PSIF**, with its control state | Yes |
| A person falls from a 1 m wall; medical treatment | Potential lost time; fatality unlikely | `fall/standing` or `fall/low` at the 1 m boundary, P(serious) 0.001–0.10: **Not SIF** or **Elevated**. The receiving surface decides which | Yes |
| 240 V shock, current path missed the heart; lost-time burns | Potential fatality | `electrical/low` (50–240 V), P(serious) 0.10: **Elevated**. A serious injury is credible but not more likely than not. It is flagged high energy and reviewed by the panel | Partly: the guidance uses a single-factor change ("a small change in body position"); the proposal uses likelihood. The case still reaches the panel |
| Fire in a building used only for storage, never occupied | No injury potential, property only | The fire is an excursion with nobody near. Under separation to zero, `fire/structural_fire`, P(serious) 0.72: **SIF potential**, recorded with closest approach **wide** | No: under the proposal, luck in where people were is not a control. Closest approach lets QQ separate it from fires near people. If the building is never entered, the panel may judge it outside scope |
| Fire at a weekend in a building occupied on weekdays | Potential injuries or fatalities | As above: **SIF potential**, closest approach **wide** | Yes |

The two fire examples show the change most clearly. The guidance treats them differently because of who might have been there; the proposal treats them the same, because the energy was the same, and records who was there as the closest approach.

---

## Annex B. Glossary

| Term | Meaning |
|---|---|
| **Closest approach** | How near the energy came to a person: wide, close or contact. Recorded for learning; does not change the potential |
| **Condition** | Energy and people together, with nothing happening. Not an event |
| **Control state** | Whether a direct control was in place and worked: protected, unprotected or unverified |
| **Direct control** | A control aimed at the energy, verified in place, and effective even if someone makes an unintentional error. Not training, rules, signs or general PPE |
| **Elevated potential** | A serious injury was credible but not more likely than not (P(serious) 0.10–0.50) |
| **Event** | A change that shrinks the separation between energy and people: an excursion or an incursion |
| **Excursion** | The energy changes: it is released, moves, becomes live or builds up |
| **Incursion** | The people change: they enter a danger zone or the line of fire of live energy |
| **LIFE** | The definition of serious: life-ending, life-threatening or life-altering (Bayona et al., 2023) |
| **P(serious)** | The probability that the outcome is serious, from the energy band's calibrated outcome distribution |
| **Precursor** | SIF exposure with no direct control in place |
| **PSIF** | An event with SIF potential and no serious injury |
| **Separation to zero** | Assess as if the energy reached a person, however close anyone came |
| **SIF exposure** | A condition in which a person is within reach of energy with SIF potential |

[^spongl]: The AI model is copyright © Spongl Ltd 2026. It is not part of QQ's core process. Terms for its use are a separate matter.
