# Response to feedback on the PSIF proposal (v0.1)

Feedback from: QQ colleague, received 2026-11-19. Response compiled 2026-11-19.

| Item | Outcome | Summary |
|---|---|---|
| QQ-FB-01 | Answered | Yes on all five points. Additions: the AI model is optional; the core is the single fixed question the panel answers. |
| QQ-FB-02 | Answered | Thanks; the panel works through several judges, measured calibration and a generous triage. Cost: a monthly sitting and calibration tests. |
| QQ-FB-03 | Agreed; changed | Agreed: SIF, PSIF and Precursor (PSIF kept, not HiPo; see QQ-FB-04). A protected exposure is not a precursor. Conditions now count when a person could foreseeably be within reach. |
| QQ-FB-04 | Not adopted | Not adopted: I keep the published term. HiPo (IOGP) is fatality-only with an open counterfactual, so not equivalent to PSIF. QQ may name it as it wishes. |
| QQ-FB-05 | Agreed; changed | Plain wording yes, open question no. Reporters give facts only; triagers ask the definition's question in plain words with a low bar; the panel asks it calibrated. |
| QQ-FB-06 | Agreed; changed | Agreed: stakeholder, decision and needs table added to proposal §4.4. Senior leaders see three measures. To confirm during a pilot. |
| QQ-FB-07 | Answered | Agreed: the probability-impact grid is the problem, and I reject it in risk assessment too. Severity and control effectiveness are distributions; judgement is given as calibrated ranges. |
| QQ-FB-08 | Agreed; changed | Agreed: the scenario (the risk-assessment context) and its hazardous events are the common key; golden-thread section added to the scenario index (v0.2, §7). |
| QQ-FB-09 | Agreed; changed | Agreed, overlaid with Hubbard: measure where uncertainty could change the decision. Control coverage as the summary; value of information directs assurance (controls paper §5.4). |
| QQ-FB-10 | Agreed; changed | Agreed: a third safety net, a monthly random sample of unflagged reports sentenced by the panel, estimates the miss rate (proposal §4.1, §7). |
| QQ-FB-11 | Agreed; changed | Agreed: one source of truth. Following the gap analysis (QQ-01), reporting reads the Power BI layer; the panel works in the reporting layer, and its classification is the record there. |

---

## QQ-FB-01. Have you understood the proposal correctly?

**Your point.** You read the problem as inconsistent individual judgement of severity, likelihood and credibility, which makes PSIF data hard to trend, compare and learn from. You read the solution as five parts, and asked whether that was right.

**Response.** Yes, all five points are right. There are two additions. First, the AI model is optional: the human process stands without it. Second, the core of the change is the single fixed question the panel answers: *would the energy, had it reached a person with no protection, more likely than not have caused a life-ending, life-threatening or life-altering injury?* Triage, the panel and the reporting all hang off that question.

| Your reading | Proposal | Match |
|---|---|---|
| Triage becomes a broad filter | §4.1: "if in doubt, flag"; the triager's rating is a view, not the decision | ✓ |
| AI as second reading and extra candidates | §4.1 stage 2; optional and independent, and nothing in the human process depends on it | ✓ (*optional*) |
| A calibrated panel decides | §4.1 stage 3; its answer is the classification of record, held in the reporting layer | ✓ |
| Potential assessed apart from controls | Potential is judged as if there were no protection; the control state is recorded separately as protected, unprotected or unverified | ✓ |
| Events apart from precursors and exposures | Events count as PSIF, conditions as SIF exposure; reported side by side, never summed | ✓ |
| *(not mentioned)* | **The fixed question** above, with hands, fingers and eyes counted as life-altering | Addition |

The classification of record is the panel's, held in the reporting layer. Your point about two sources of truth is answered under QQ-FB-11.

---

## QQ-FB-02. Moving the decision to a calibrated panel

**Your point.** You support the direction of travel, above all moving the final decision from individual triagers to a calibrated panel using consistent criteria.

**Response.** Thank you. That shift is the heart of it: a decision made once, by people who are trained and measured, with the most information. It works because of three things:

- **Several judges.** Four or five members, each answering yes or no with a confidence level, averaged. In healthcare, a reliable potential-severity score needed at least four judges per case.
- **Measured calibration.** Members are tested on cases with known answers and scored over time with a Brier score (a standard accuracy score for probability judgements), so QQ can show its PSIF decisions are measured, not just made.
- **A generous triage.** Triage still matters: it feeds the panel, so its bar is "if in doubt, flag" (see QQ-FB-10).

The cost is a monthly sitting and periodic calibration tests. The pilot (proposal §7) will show how many cases a month the panel can handle.

---

## QQ-FB-03. Three core categories

**Your point.** Keep to at most three core categories: SIF; HiPo, an event that did not result in a SIF but could credibly have caused a life-ending, life-threatening or life-altering injury; and SIF precursor or exposure, a condition, control weakness or work activity where a credible SIF scenario exists but no event has occurred.

**Response.** Agreed on three categories: SIF, PSIF and Precursor. There are four refinements. One of them changed my definitions.

| Category | What it means | Refinement |
|---|---|---|
| **SIF** | A life-ending, life-threatening or life-altering injury occurred | It also counts as PSIF with P(serious) = 1, so it needs no judgement of potential |
| **PSIF** (your HiPo) | An event where, had the energy reached a person with no protection, a serious injury was more likely than not | I keep the published term rather than HiPo, which is not equivalent (see QQ-FB-04). The fixed question and its threshold define it |
| **Precursor** | A hazardous situation (energy and people within reach) where the direct control is absent or has failed, and no event has occurred | Precursor and exposure are not the same thing: a protected exposure is not a precursor. Protected exposures are reported separately, as a measure of control coverage (see QQ-FB-09) |

**Credible versus likely.** The threshold is "more likely than not". Cases where a serious injury is credible but not likely are not lost: they go to the panel as a review queue. They are not reported as a category.

**No one present.** Your wording included conditions and work activities, not only cases where a person was in reach. My definition had required a person within reach at the time. I have widened it: a condition now counts if a person could be within reach during intended use or reasonably foreseeable misuse, the test in ISO/IEC Guide 51. An unguarded running machine with nobody nearby is therefore a precursor if people normally work or pass there.

**Control state** (protected, unprotected or unverified) is recorded on every case, events and conditions alike. That is what separates a precursor from a protected exposure, and a protected PSIF from an unprotected one.

I have also mapped the SIF labels (precursor, exposure, success) to established risk vocabulary (ISO/IEC Guide 51, ISO 31000, WHO), since they describe the same ideas, and set out how HiPo differs from PSIF: definitions, Annex C.

---

## QQ-FB-04. HiPo in place of PSIF

**Your point.** You asked whether the SIFP or PSIF terminology helps or hinders, and suggested HiPo (high potential event) as a simpler term for operational teams, investigators and leaders, with the same method behind it.

**Response.** I keep PSIF. My approach is to follow the safety literature and published definitions strictly, and HiPo is not equivalent to PSIF:

- **The canonical HiPo definition is fatality only.** IOGP defines a high potential event as one that "could have, under slightly different circumstances, realistically resulted in a fatal incident". It notes that a permanent-impairment case "may or may not be a high potential event". PSIF also covers life-altering injury.
- **Its test is the open one.** "Under slightly different circumstances" is the open counterfactual the proposal replaces with a fixed question.
- **There is no single definition.** IOGP itself notes that "individual company definitions may differ".

Using HiPo for PSIF would put a published, fatality-only term on a different definition. It would also break comparison with the sources PSIF follows (EEI, ASTM E2920, the Campbell Institute).

How QQ names the idea in its own communications is QQ's decision. If QQ uses another name, I suggest every document that uses it states that it means PSIF as defined, so that QQ's figures are not read as IOGP HiPo figures.

---

## QQ-FB-05. Plain language at the front end

**Your point.** You agreed with the energy principle, but many operational users think in hazards and consequences rather than energy. You suggested a plain front-end question, "Could this realistically have resulted in a life-ending, life-threatening or life-altering injury?", with the energy method kept behind the scenes.

**Response.** Agreed that the front-end language should be plain and the energy method should stay behind the scenes, but the question itself does not change. Your suggested question, "could this realistically…", is the open counterfactual that gives different people different answers, so I have not adopted it. Instead:

| Who | Task | Question |
|---|---|---|
| **Reporters** | Facts only: no opinion, inference or judgement | What happened, what was involved, and its size (how high, how heavy, how fast, what voltage) |
| **Triagers** | Judgement, with a low bar ("if in doubt, flag"); not calibrated | The definition's question in plain words (below) |
| **The panel** | Calibrated judgement, with confidence levels | The same question |

The definition's question, in plain words:

> *If the thing that could have hurt someone (the fall, the load, the vehicle, the electricity, the machine) had reached a person with no protection, would a life-ending, life-threatening or life-altering injury have been more likely than not?*

Everyone asks one question, and only the rigour changes from stage to stage. The plain wording is now in the definitions (§7.1) and the proposal (§4.1).

---

## QQ-FB-06. Audiences and the decisions they make

**Your point.** Before the model is finalised, understand the intended audiences and the decisions each is expected to make. Safety professionals analysing trends need a different level of detail from senior leaders, and the decisions each supports determine how much complexity is genuinely needed.

**Response.** Agreed, and the proposal did not do this: it named only the safety director and EHS leadership. I have added a table of stakeholders, the decisions each makes and what each needs (proposal §4.4). The complexity sits with the panel and the analysts. Senior leaders see three measures: SIF, PSIF (unprotected) and the share of SIF exposures that are protected. The table is a proposal to confirm during a pilot.

| Stakeholder | Decision they make | What they need | What they don't need |
|---|---|---|---|
| **Senior leaders** | Where to put attention and resource; whether SIF-capable work is under control | SIF actual; PSIF (unprotected); the share of SIF exposures that are protected | Bands, probabilities, Elevated |
| **Safety director and EHS leadership** | Programme priorities, such as a critical-control campaign by energy type | PSIF and Precursor by mechanism and control state; the unverified share | Individual cases |
| **Site and operational managers** | What to fix locally: which work, which control | Their open Precursors and control failures, case by case | Trends across sites |
| **Investigators** | How deep to investigate | The PSIF class, closest approach and control state on each case | Aggregates |
| **Safety analysts** | Whether the data can be trusted and trended | Everything: Elevated, bands, panel calibration (Brier scores), agreement between the panel and the AI model | (none) |

---

## QQ-FB-07. The same subjectivity in risk assessment

**Your point.** Many of the problems the proposal identifies (likelihood, credibility and dependence on controls) mirror those in risk assessment. The proposal challenges subjectivity in SIF potential, but the same assumptions sit within the risk assessment process.

**Response.** Agreed. The weakness is the probability-impact grid itself, and it is well documented: Cox (2008) found such grids can be "worse than useless", and Chapman and Ward say they should be "scrapped in all contexts". My approach rejects them in risk assessment as well as in PSIF coding:

- **Severity** comes from the energy as a distribution, not a cell on a grid.
- **Controls** carry their own effectiveness distributions and are combined by simulation. "How much does this control help, and what if two fail together?" then has a quantified answer rather than a moved cell.
- **What remains a judgement**, such as likelihood, is given as a calibrated range.

| Subjectivity | Probability-impact grid | Replacement |
|---|---|---|
| Severity and credibility | An ordinal 1–5 bin: a 2 m and a 20 m fall can share a cell | A severity distribution from the energy, giving P(serious) |
| Dependence on controls | The cell moves when the assessor feels better about a control | Each control has an effectiveness distribution, combined by Monte Carlo simulation (a quantified Swiss cheese model), including controls that fail together |
| Likelihood | "Likely" means different things to different people | Calibrated range estimates |

The same engine runs before the work (pre-task planning) and after the event (screening). A scenario index applies the same rules to the scenario of each risk assessment, before anything happens.

References: *From PIGs to Probability Curves* (2026); PSIF scenario index (2026); Cox, L.A. (2008), What's wrong with risk matrices?, *Risk Analysis* 28(2); Hubbard, D. (2009), *The Failure of Risk Management*; Chapman, C. & Ward, S., *How to Manage Project Opportunity and Risk*.

---

## QQ-FB-08. A golden thread from risk assessment to learning

**Your point.** There is an opportunity for a stronger golden thread between risk assessment, SIF identification, investigation categorisation, assurance and control verification, and organisational learning.

**Response.** Agreed. This is where the approach is heading, and the thread needs a common key. I use the **scenario** in its risk-management sense: the context of a risk assessment (ISO 31000, 6.3: scope, context and criteria), the same frame as the scope of a HAZID or HAZOP. Within each scenario sit its credible **hazardous events**: PSIF events defined in advance, each with its energy, its potential and the direct controls that should stop it. A scenario ends in risk evaluation: is the remaining risk tolerable, giving the confidence to take the next step?

| Stage | What happens | Key carried |
|---|---|---|
| **Risk assessment** | The scenario is set; its hazardous events are coded and their direct controls listed; the scenario is evaluated: go, go with conditions, or no go | Scenario, hazardous event, control |
| **SIF identification** | Each report is coded by the same rules and matched to a hazardous event | Hazardous event |
| **Investigation categorisation** | Class, control state and closest approach set the depth of investigation | Hazardous event, control |
| **Assurance and control verification** | Observations check the listed direct controls; each result is protected or unprotected | Control |
| **Organisational learning** | Reports that match no hazardous event become new ones; SIF-class events that never appear in reports are reviewed; the scenario's evaluation is revisited with better evidence | All |

A pilot on range operations is drafted. The depth of investigation each class gets is QQ's process, and I would welcome working on it together. I have added a section setting out the thread (scenario index, §7).

---

## QQ-FB-09. Leading indicators on SIF-capable work and critical controls

**Your point.** Assurance activities should provide leading indicators on SIF-capable work and critical controls, measuring whether critical controls are present and performing as expected, not only classifying events after they occur.

**Response.** Agreed, and I would go one step further, following Hubbard: measure to make a better decision, and measure first where the uncertainty is greatest.

- **The decision.** Each scenario ends in a decision: is the remaining risk tolerable, so that the work can go ahead? Assurance exists to make that decision better.
- **The uncertainty.** Each direct control on a SIF-class hazardous event has an effectiveness distribution, not a yes or no. Some are well known, such as a fixed guard. Others are wide, such as a procedure.
- **What to measure.** The value of information shows which control's uncertainty could actually change the decision. Assurance goes there first. A few well-chosen observations of a wide, decision-critical control are worth more than many of a control already known to work. That is more focused, and cheaper.
- **The indicator.** Control coverage, the share of SIF-capable conditions where the direct control was in place and working (EEI's high-energy control assessment), remains the summary measure. One study links it to fewer SIFs (Bayona et al., 2026), but it is correlational, so it is a measure to learn from, not a target.
- **The thread.** Through the golden thread (QQ-FB-08), each observation updates the distribution of the control it checks, and the scenario's evaluation is revisited with less uncertainty. This follows Hopkins (2009) and ICMM (2015): measure whether the controls work, not only what happened.

I have added this to the controls paper (§5.4).

References: Hubbard, D.W. (2014), *How to Measure Anything*, 3rd ed.; Bayona et al. (2026); Hopkins, A. (2009), Thinking about process safety indicators, *Safety Science* 47(4); ICMM (2015), *Critical Control Management: Implementation Guide*.

---

## QQ-FB-10. Events never put forward by triage

**Your point.** If candidate cases progress only when the triager identifies them, some events may never be put forward for consideration at all.

**Response.** Agreed, and this is a real risk. The proposal has two safety nets. Triage has a deliberately low bar ("if in doubt, flag"), and the AI model, where used, reads every report and puts forward what triage missed. But the AI model is optional, and its own misses would go unseen. So I have added a third net that does not depend on either: each month the panel sentences a small random sample of reports that nobody flagged. That gives QQ an estimate of the miss rate, with its uncertainty. It is cheap, because a few dozen cases a month are enough to show whether misses are rare or common (Hubbard). If the rate is low, the sample can stay small. If it is high, the triage bar or the screen needs attention. The sample size is set during the pilot.

| Safety net | Catches |
|---|---|
| Low triage bar | Borderline cases |
| AI model screen (optional) | Triage misses, at scale |
| Missed-case audit: a monthly random sample of unflagged reports | Misses by triage and by the AI model, with an estimated miss rate |

The audit is now in the proposal (§4.1), and the miss rate is a pilot measure (§7).

---

## QQ-FB-11. Two sources of truth

**Your point.** There is a risk of two sources of truth if the classification captured in IMR differs from the one ultimately used for reporting and decision-making.

**Response.** Agreed that it is a risk. The IMR gap analysis has addressed it: its action QQ-01 recommends a stated two-tool boundary. Power BI is for corporate reporting and decisions; IMR (EcoOnline) is for capture, case workflow and site-level views. All reporting already reads the reporting layer (the data pipeline that feeds Power BI), never IMR directly. IMR holds the ratings as entered, the pipeline derives the one classification every dashboard uses, and errors are corrected at source in IMR. So there is one source of truth for reporting.

The panel works within that boundary:

| | Where |
|---|---|
| The panel's list of cases to sentence (triage flags, AI model candidates, the audit sample) | The reporting layer (an Excel workbook) |
| The details of each case | The IMR record, opened by the panel |
| The panel's classification, with control state | Recorded back in the reporting layer, joined to the IMR record by its report key |
| The triage rating | Stays in IMR, as the triager's view; kept alongside for comparison, not reported as the classification |

The panel's classification is the classification of record in the reporting layer. IMR needs no new field. The proposal (§4.1) now says so.
