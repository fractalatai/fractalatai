# Counterfactuals in SIF potential

Status: draft v0.7, 2026-11-17. A literature review in a series on the challenges, differences and points of consensus in defining serious injury and fatality (SIF) potential. Definitions follow [sif-definition.md](sif-definition.md).

## Executive summary

*For the safety director.*

### The problem

PSIF means classifying an event by what did not happen. Today, report triagers decide PSIF by choosing a descriptive severity from a pick list ("low", "high", "very high", "massive" and so on). That asks each triager to imagine an outcome and label it. The pick list does not say:

- **what to imagine:** the worst case, the most likely outcome, or something in between;
- **whether controls count:** for example a harness that worked, or a barrier that held;
- **what the labels mean:** where "very high" ends and "massive" begins.

The triager also knows the actual outcome. That is known to pull the rating: changing only the outcome of otherwise identical cases moves expert judgements by about 30 percentage points.

The research is consistent on what happens next:

- When practitioners rate potential this way, agreement is little better than chance: 64%, where 50% is a coin toss (EEI).
- In healthcare, comparable "would this have been prevented?" judgements average about 0.33 reliability.
- Critics such as Busch call PSIF "arbitrary" and "subjective". Promoters such as EEI agree that, asked openly, the answer is "inevitably 'yes' if the analyst is creative enough".

The consequences differ by use:

- **Flagging cases for review:** the pick list does a reasonable job, if triagers flag generously.
- **Deciding PSIF, trending it, or comparing sites:** it is unreliable. A change in the numbers may be a change in who did the triage, not in the risk.

### The fix

The subjectivity comes from leaving the question open, not from PSIF itself. Fix the question, decide it with calibrated people, check it against the AI model, and record controls separately. This needs no change to the reporting software: triagers keep the pick list, but it is no longer the classification of record.

**Three stages:**

1. **Triage: a coarse filter.** Triagers use the existing pick list, with a deliberately low bar ("if in doubt, flag"). Their job is to miss nothing, not to decide.
2. **Screening: the AI model.** The AI model reads every case on the list. It puts forward likely PSIF candidates that the triagers did not flag, so under-calls at triage are caught.
3. **Sentencing: a calibrated panel, monthly.** A small panel sentences every flagged and put-forward case. Its decision, recorded outside the reporting software, is the classification of record.

**The panel answers one fixed question for each case:**

> *If this energy (as described) had reached a person, with no protection, would a serious injury be more likely than not?*

Each member answers yes or no, with their confidence that the answer is correct, from 50% (a guess) to 100% (certain). For example: **Yes, 70%**. This is the format Hubbard uses to train and measure calibrated assessors.

| | Today | Proposed | Why |
|---|---|---|---|
| **Who decides** | The triager, at triage | A calibrated monthly panel. Triagers filter; the AI model adds missed candidates | Separates finding cases from deciding them |
| **The question** | "How severe could this have been?" (open) | One fixed question, the same for every case (above) | An open question gives a different answer for each person |
| **The answer** | A descriptive label | Yes or no, with 50–100% confidence, from each member. The panel's answer is the average | Confidence shows where the panel is sure and where it is guessing |
| **Facts that are missing** | Filled in by imagination | Members give a range ("the fall was between 1.5 and 4 m") and answer accordingly | A calibrated range is honest about uncertainty; a guess is not |
| **Checking the answer** | Not measured | Compared with the AI model's classification for the same case. Disagreements are discussed | Two independent readers catch each other's errors |
| **Calibrating the panel** | Not done | Training and periodic tests on the same question, with answers known from the injury data behind the AI model (e.g. "a 3–6 m fall onto concrete") | Members learn what 70% confidence really means, and are scored on it |
| **Controls** | Mixed in, or ignored, case by case | Recorded separately: PSIF *protected* (a control worked), *unprotected* (absent or failed) or *unverified* (not stated) | Keeps "how dangerous was the energy" apart from "did our controls work" |
| **What-ifs** | Part of the rating | Recorded separately and labelled, for investigation only | What-ifs help learning but make classification inconsistent |

**How the panel's answers combine.** "Yes, 70%" counts as 0.70 and "No, 70%" as 0.30. The panel's answer is the average: 0.5 or more is yes (PSIF), and below 0.5 is no. Cases where members are split, or where everyone is near 50%, are discussed before the result is recorded. Averaging several judges is itself a gain: in healthcare, a reliable potential-severity score needed at least four judges averaged per case.

### Recommendations

1. **Adopt the definition.** Use the SIF potential definition in [sif-definition.md](sif-definition.md) as the panel's question.
2. **Keep triage as a filter.** No software change. Ask triagers to flag generously; the pick-list rating is their view, not the decision.
3. **Use the AI model to screen all cases.** It adds likely candidates that triage missed, and gives the panel a second, independent reading of every case it sentences.
4. **Set up a monthly sentencing panel** of four or five people from across the operation. Members answer yes or no with confidence, and the panel records the average and the control state.
5. **Calibrate the panel.** Give initial calibration training, then test periodically with questions whose answers come from the injury data. Track each member's calibration with a Brier score.
6. **Ask reporters for the size of the energy.** Height, speed, voltage, weight and pressure decide the answer. Thin reports are under-called.
7. **Use PSIF to find cases now, and trend later.** Trend only once the panel is calibrated and its rules are fixed. Do not set targets for PSIF rates, because that suppresses reporting. Never re-classify history after one serious event.

## Contents

- [Executive summary](#executive-summary)
- [Abstract](#abstract)
- [1. The problem](#1-the-problem)
- [2. What a counterfactual is](#2-what-a-counterfactual-is)
- [3. Why open counterfactuals are subjective](#3-why-open-counterfactuals-are-subjective)
- [4. Counterfactuals in safety practice](#4-counterfactuals-in-safety-practice)
  - [4.4 In healthcare: preventability and potential harm](#44-in-healthcare-preventability-and-potential-harm)
- [5. The case against PSIF](#5-the-case-against-psif)
- [6. How practice responds](#6-how-practice-responds)
- [7. Two counterfactuals, not one](#7-two-counterfactuals-not-one)
- [8. Case surfacing versus trending](#8-case-surfacing-versus-trending)
- [9. Research agenda](#9-research-agenda)
- [10. Conclusion](#10-conclusion)
- [References](#references)
- [Verification notes](#verification-notes)

## Abstract

PSIF (potential serious injury or fatality) rests on a counterfactual: an event is classified by what did not happen but could have. Critics and promoters of SIF programmes agree that this invites subjectivity. Critics argue that almost any event can be imagined into a fatality, and measured agreement between practitioners supports them. Promoters respond in three ways: they restrict who decides, restrict the question, or recast the question as a probability.

This paper traces counterfactual reasoning through philosophy, causal inference, psychology and law, then through accident investigation and near-miss learning. It argues that the subjectivity comes from leaving the counterfactual **open**. Fixing the antecedent by rule and taking the answer from calibrated outcome data removes most of it. What remains is the error of reading a report, which can be measured. That remaining error matters little when PSIF is used to surface cases for review. It matters a great deal when PSIF is used for trending or benchmarking.

## 1. The problem

PSIF is built on a counterfactual: things that did not happen. That raises two problems:

- **Objectivity:** is the classification a fact about the event, or about the assessor?
- **Consistency:** do two assessors, or one assessor on two days, reach the same answer?

How much this matters depends on what PSIF is used for:

- **Surfacing individual cases for closer examination.** A false positive costs a review. A false negative may be caught another way. Subjectivity is tolerable.
- **Trending and benchmarking.** A PSIF rate is compared across months, sites or companies. Inconsistency in classification becomes indistinguishable from change in the underlying risk.

Organisations often tackle this by restricting who may sentence PSIF. At one organisation, five staff from across the operation met every week to sentence every case of every event type, in meetings that took as long as needed. That buys consistency through a shared, practised judgement. It does not make the judgement objective, and it does not carry over to another organisation.

The scale of the problem has been measured. When EEI's working group classified 14 real cases, "the average level of agreement was only 64%, where 50% represents perfect disagreement (i.e., a 50/50 split)". Before the exercise, "no two organizations on the team used the same method to classify PSIF events" (Hallowell & Spencer, 2024).

Detractors and promoters both address the subjectivity of PSIF head on.

## 2. What a counterfactual is

A counterfactual is a conditional about something contrary to fact: *had X been the case, Y would have followed*. Four traditions treat it.

**Philosophy.** Lewis (1973a, p. 1) opens his analysis with an example:

> "'If kangaroos had no tails, they would topple over' seems to me to mean something like this: in any possible state of affairs in which kangaroos have no tails, and which resembles our actual state of affairs as much as kangaroos having no tails permits it to, the kangaroos topple over."

He then concedes the difficulty, and says how to live with it:

> "Counterfactuals are notoriously vague. That does not mean that we cannot give a clear account of their truth conditions. It does mean that such an account must either be stated in vague terms—which does *not* mean ill-understood terms—or be made relative to some parameter that is fixed only within rough limits on any given occasion of language use. … It will be a relation of comparative similarity." (Lewis, 1973a, p. 1)

Lewis builds causation on the same foundation. He starts from Hume's second definition, a cause is that "where, if the first object had not been, the second never had existed" (Lewis, 1973b, p. 556). He then states the common intuition he sets out to analyse:

> "We think of a cause as something that makes a difference, and the difference it makes must be a difference from what would have happened without it. Had it been absent, its effects—some of them, at least, and usually all—would have been absent as well." (Lewis, 1973b, p. 557)

He notes why such intuitions had not been trusted: "We have learned all too well that counterfactuals are ill understood" (p. 557). His answer is to take them "as statements about possible alternatives to the actual situation, somewhat vaguely specified" (pp. 557–558). Everything turns on the similarity parameter. Lewis later proposed priority rules to tighten it (Lewis, 1979).

PSIF is a Lewis-type counterfactual: *had the energy reached a person, a SIF would have occurred*. Its truth depends on which worlds count as closest, and an open what-if leaves that parameter to each analyst. Lewis's own remedy points the way for SIF practice: a counterfactual can have clear truth conditions if the parameter is fixed. The fixed rule in section 7 fixes it.

**Causal inference.** Pearl places counterfactuals at the top of a three-level hierarchy, "1. Association, 2. Intervention, and 3. Counterfactual". The third level's typical questions are "Why? Was it X that caused Y? What if I had acted differently?" It sits "at the top of the hierarchy because [counterfactuals] subsume interventional and associational questions". Crucially, counterfactual sentences "can be computed only when the model is based on functional relations or structural" (Pearl, 2019; Pearl & Mackenzie, 2018). In Pearl's framework a counterfactual is not free imagination. It is a defined intervention applied to a model of how the world works, and its output is a probability.

**Psychology.** People generate counterfactuals readily and with predictable biases.

- They judge how nearly something happened by how easily an alternative can be mentally simulated (Kahneman & Tversky, 1982).
- Each event "creates its own norm" after the fact, and exceptional events are more readily "undone" than normal ones (Kahneman & Miller, 1986).
- Reality has "fault lines", the aspects people most readily change in imagination: exceptional over normal events, actions over inactions, controllable over uncontrollable events, and forbidden over permitted acts (Byrne, 2005).
- Upward counterfactuals ("it could have been better") prepare people to improve. Downward ones ("it could have been worse") mainly make people feel better (Roese, 1997).

**Law.** But-for causation is a counterfactual disciplined by a probability threshold. As Pearl (2019) puts it: "a defendant is considered responsible for an injury if, but for the defendant's action, it is more likely than not the injury would not have occurred."

Taken together, these sources say that a counterfactual is only as objective as the rule that fixes its antecedent and the model that computes its consequent. Left open, it follows whatever the assessor finds easiest to imagine.

## 3. Why open counterfactuals are subjective

The literature gives five reasons.

1. **Unbounded possible worlds.** "Counterfactuals are notoriously vague" (Lewis, 1973a, p. 1). "Could this have been a SIF?" is true in *some* world for nearly every hazard. EEI reached the same point empirically: "every hazard has the remote possibility of causing a SIF" (Hallowell & Spencer, 2024).
2. **Mutability biases.** Open what-ifs follow narrative fault lines, often the worker's act or a broken rule, not the physics of the energy (Byrne, 2005; Kahneman & Miller, 1986).
3. **Closeness mistaken for probability.** After the event, people judge probability by how physically, temporally or conceptually close the alternative was (Teigen, 1998). "Almost" judgements neglect prior probabilities (Kahneman & Varey, 1990). Practice builds this heuristic in: "The only real difference between an 'actual SIF' and a 'potential SIF' is inches, seconds, and luck" (Martin, dss+).
4. **Hindsight and outcome bias.** Knowing the outcome changes judged probability and relevance, and judges are largely unaware of it (Fischhoff, 1975). "Once people have knowledge of an outcome, they tend to view the outcome as having been more probable than other possible outcomes", and this persists "even when judges are warned about the phenomenon" (Woods & Cook, 1999; Baron & Hershey, 1988). In medicine the effect has been measured directly. Anaesthesiologists rated matched cases that were "identical to the original in every respect except that a plausible outcome of opposite severity was substituted". The proportion rating the care as appropriate "decreased by 31 percentage points when the outcome was changed from temporary to permanent" (Caplan, Posner & Cheney, 1991). A benign outcome biases judged potential downward.
5. **Measured noise.** EEI's agreement was 64% against a chance level of 50% (Hallowell & Spencer, 2024). Healthcare has measured the same kind of judgement more thoroughly (section 4.4):
   - Single raters scoring the potential harm of radiotherapy near misses reached only "fair agreement" (Krippendorff's α = 0.376; Mullen et al., 2016).
   - In a multi-institution test of a radiotherapy incident learning system, likelihood of harm was the least reliable data element (κ = 0.326, against 0.659 on average; Kapur et al., 2016, conference abstract).
   - Judgements of whether a death was preventable, a pure counterfactual, had a median reliability of 0.33 across studies (Manaseki-Holland et al., 2019).
   
   A study at an Australian port found 76% of incidents with SIF potential had been rated moderate risk or lower (Fernando et al., 2024).

## 4. Counterfactuals in safety practice

### 4.1 In accident investigation

Investigators work backwards from a known outcome, looking for "critical assessments or actions that, if they had been different, would have avoided that outcome" (Woods & Cook, 1999). Dekker (2002) names these directly: "These are counterfactuals—quite common in accident analysis." He gives the objection: "saying what people could have done in order to prevent a particular outcome does not explain why they did what they did", and "counterfactuals are a powerful tributary to the hindsight bias."

Busch makes the same point:

> "[C]ounterfactual reasoning. This means that one assumes that everything that could have prevented an accident is a cause … is often characterised by thinking in terms of 'if only…', and verbs like 'should'." (Busch, 2019, "Sifting Through SIF")

Dekker's objection is aimed at counterfactuals about **human action** used as **explanation**. He accepts them looking forward: "Counterfactual reasoning may be a fruitful exercise when trying to uncover potential countermeasures against such failures in the future" (Dekker, 2002). This distinction matters for PSIF (section 7).

### 4.2 In near-miss learning

Near misses are counterfactual by nature. March, Sproull & Tamuz (1991) describe learning from "near-histories—events that almost happened" and "hypothetical histories—events that might have happened under certain unrealized but plausible conditions". They recall that the National Research Council "defined a safety 'incident' as an event that, under slightly different circumstances, could have been an accident". They state the goal of such learning: "identifying the underlying distribution from which that realization was drawn rather than explaining the particular draw."

They also name the ambiguity at the heart of near-miss learning:

> "every time a pilot avoids a collision, the event provides evidence both for the threat and for its irrelevance. It is not clear whether the learning should emphasize how close the organization came to a disaster, thus the reality of danger in the guise of safety, or the fact that disaster was avoided, thus the reality of safety in the guise of danger." (March, Sproull & Tamuz, 1991)

Later work shows which reading people tend to choose, and with what effect:

- Near misses tend to be read as successes. They lower *perceived* risk without lowering the statistical risk (Dillon & Tinsley, 2008).
- Tinsley, Dillon & Cronin (2012) separate two readings. A **vulnerable** near miss is one "wherein a disaster almost happened and results in the perceived vulnerability of the system". A **resilient** near miss is one "wherein a disaster could have but did not happen and results in the perceived resilience of the system". The resilient reading leads people to "illegitimately underestimate the danger of subsequent hazardous situations and make riskier decisions". Highlighting "the harm the event could have caused" counteracts it.
- Learning follows **upward, self-focused** counterfactuals ("I could have reduced the danger by acting differently"). Accountability to organisational audiences suppresses them (Morris & Moore, 2000).

### 4.3 In SIF potential classification

SIF definitions make the counterfactual explicit. They differ in how they bound it:

| Source | The counterfactual | Bound |
|---|---|---|
| Campbell Institute (2018, p. 5) | "commonly used criteria by Campbell Institute members is that an event has SIF potential if the situation could have been worse and could have resulted in SIF if not for one factor. This one factor could be a control that was in place, the location of a person or piece of equipment, the time of day, or even weather conditions" | One factor changed. Which factor is left open |
| BST (Krause, 2010) | "could reasonably result in a serious injury or fatality" if allowed to continue or repeat | "Reasonably": judgement |
| Cooper (2019) | A potential SIF is "an incident that resulted in a minor injury that could reasonably have resulted in a life-threatening or life-altering injury", or a near miss with "human exposure and a release of some type of stored energy" | "Feasibly and reasonably": judgement; exposure and energy release as entry conditions |
| dss+ (Martin) | "The only real difference between an 'actual SIF' and a 'potential SIF' is inches, seconds, and luck" | Closeness |
| EEI SCL (Hallowell & Spencer, 2024) | "Is the most likely outcome of this event a SIF?" What-ifs excluded | Most likely outcome; energy over 500 ft-lb |
| ASTM E2920-26 | pSIF: likelihood of life-ending, life-threatening or life-altering consequences | Likelihood, not quantified |
| ARMS Working Group (2010, p. 18) | "If this event had escalated into an accident, what would have been the most credible accident outcome?" | Most credible accident outcome, neither the most probable nor the worst; barriers assessed separately |
| AI model (sif-definition.md) | Had the energy reached a person with no protective control between them, a serious outcome would be more likely than not | One fixed antecedent; probability from calibrated injury data |

### 4.4 In healthcare: preventability and potential harm

Healthcare has studied counterfactual judgement for longer, and measured it more carefully, than occupational safety. Two judgements in its incident and case review are counterfactual:

- **Potential harm:** how badly a near miss could have hurt the patient.
- **Preventability:** whether the harm would have been avoided with optimal care.

**Preventability is the least reliable.** A systematic review found a "median reliability of 0.33 and an interquartile range of 0.27–0.45" for judging whether hospital deaths were preventable. It concluded that "a Kappa of 0.35 would require an average of 8 to 17 reviews of a single case" (Manaseki-Holland et al., 2019).

- Two reviewers agreed on whether care was negligent at κ 0.19–0.23, against κ 0.40–0.41 on whether an adverse event had occurred at all (Thomas et al., 2002).
- Disagreement was worst "for adverse events attributed to failure to diagnose or lack of therapy" (Localio et al., 1996). These are omissions: the most counterfactual cases, because they require imagining an act that did not happen.
- Hayward & Hofer (2001) called it plainly: "preventability is in the eye of the reviewer."

**Potential harm is fair for single raters and better with structure:**

- Rating the potential harm of radiotherapy near misses gave "fair agreement" (α = 0.376). It rose to "moderate agreement" (α = 0.501) among the most experienced raters, and to α = 0.607 when rated in weekly meetings, with "significant differences … between professions" (Mullen et al., 2016).
- Potential severity of medication errors: "Most of the variance was attributable to differences in the cases", but "to achieve a generalizability coefficient of more than 0.8, at least four judges would have to score each case … with the mean score used" (Dean & Barber, 1999).
- With an explicit algorithm, agreement on the "severity of potential harm" was "at least substantial (ICC>0.60) and in most of cases almost perfect (ICC>0.80)" (Doerper et al., 2015).
- Rule-based severity scoring reached κ 0.83–0.90, against 0.63–0.84 for judgement-based scoring of the same errors (Snyder et al., 2007).

No study was found that compares the reliability of *actual* and *potential* harm ratings on the same incidents, so the evidence does not establish that potential is always rated less consistently than actual. What it does establish is the pattern this paper argues for:

- the more a judgement depends on imagining what did not happen, the less reliable it is;
- consensus and averaging help;
- explicit rules and algorithms help most.

## 5. The case against PSIF

Busch states the objection most directly:

> "S for Subjective? Building the SIF triangle is not without other problems … who is to decide upon potential, and how … The major question is, however, how to assess potential. Is this not rather arbitrary? … I can think up a fatal accident for almost any event, but how realistic are those scenarios? And where does one draw the line then? We need some assessment probability for this … Even if there were, it is going to be pretty tricky and subjective … SIF programs require you to report counterfactuals: things that did not happen, but could have happened." (Busch, 2019, "Sifting Through SIF")

The objection has three parts:

1. **No limit.** With enough imagination, any event becomes a fatality.
2. **No line.** Even if only "realistic" scenarios count, nothing says where realistic ends.
3. **Probability is needed but unavailable.** Busch names the remedy, "some assessment probability", and doubts it can be had without becoming "tricky and subjective" in turn.

Busch is not against potential as such. In 2021 he co-authored a recommendation to "classify injuries not only by outcome, but also by outcome potential (e.g., SIF potential or not)", noting that "grouping data according to control(s) and risk potential is more useful in identifying SIF precursors than the common practice of grouping by chance consequence" (Busch et al., 2021). His objection is to *how* potential is judged.

Cooper, writing for practitioners, lists the same problem among the "problems to overcome with an SIF program":

> "How can people reliably assess the potential for an SIF without the assessment becoming arbitrary and subjective? – What does feasible and reasonable mean? – Can we use severity x likelihood of impact? – i.e. Risk Ratings" (Cooper, 2017)

He also raises a hindsight problem that matters for trending: "If a low probability/serious injury event occurs, should all such prior and future events be re-categorized into SIFs?" (Cooper, 2017).

EEI, a promoter, agrees with the diagnosis:

> "One of the most challenging tasks observed in the test cases was determining if an incident or observation had SIF potential because existing methods involve a great deal of subjectivity … analysts should not ask the question: does this incident have potential? The answer to this question is inevitably 'yes' if the analyst is creative enough (e.g., a fall to the same level could be fatal if the worker struck his or her head just the right way). Rather, the analyst should ask: Is the most likely outcome of this event a serious injury or fatality? … 'what-if' scenarios should not be considered in the formal classification of the event or observation. Instead, the analyst should consider only the facts." (Edison Electric Institute, *SCL Model*) *[page]*

Aviation reached the same diagnosis. The ARMS Working Group found that analysts asked to rate severity answered the question "severity of what?" differently:

> "Some refer to the severity of the actual event and its actual, real outcome. Others think of the severity of the potential outcome, an 'imaginary but realistic' outcome, 'the most probable type of accident' outcome or the 'worst case scenario'." (ARMS Working Group, 2010, p. 8)

Some analysts "were rating based on the severity of the actual outcome of the event, some on the potential outcome and some on what is considered a credible outcome. This is all very subjective" (p. 40). Its worked example, a near-overrun with only blown tyres, lists the candidates side by side: the actual outcome, "overrun with some injuries & major aircraft damage", or "overrun with 100% fatalities" (p. 41).

Detractor and promoter agree on the diagnosis. They differ on whether it can be treated.

## 6. How practice responds

Three responses appear in the literature and in practice. The third runs through all of them.

### 6.1 Restrict who decides

PSIF sentencing is confined to a small, trained cohort, as in the weekly five-person panel. EEI did this as calibration. Through "training and the opportunity to calibrate", and after "every decision criterion was carefully revised to remove ambiguity and potential subjectivity", agreement rose to "greater than 95%" after four rounds of 40 test cases (Hallowell & Spencer, 2024). BST reported more than 90% agreement among its trained analysts (Martin & Black, 2015).

Cooper recommends the same: "get agreement among the team to ensure consistency", and "ensure every single precursor situation and exposure category is clearly defined with concrete examples" (Cooper, 2014; 2017). Healthcare shows why consensus helps and what it costs. Consensus meetings raised agreement on near-miss potential from α 0.38 to 0.61 (Mullen et al., 2016). A reliable potential-severity score needed "at least four judges" averaged per case (Dean & Barber, 1999).

This works within the cohort. It moves the subjectivity from the individual to the group without removing it. March, Sproull & Tamuz (1991) warn that agreement (reliability) can be bought at the cost of being right (validity). A panel can be perfectly consistent and consistently wrong. Agreement is also reported as percentages within the calibrated group, not as chance-corrected statistics across organisations.

### 6.2 Restrict the question

Replace "could it have been a SIF?" with a narrower question tied to facts:

- **Most likely, not worst case.** EEI found "two key philosophical differences", the first "assessing the worst possible outcome versus the most likely outcome", and settled on the most likely (Hallowell & Spencer, 2024). ARMS takes a third position. It asks for the most credible accident outcome, and rejects both alternatives:

  > "This question is not asking for the most probable outcome, as that is usually 'nothing' and therefore ignores any risk that the event carries, but neither is it necessarily looking for the worst possible outcome as the worst case scenario would often not be the most obvious accident to expect." (ARMS Working Group, 2010, p. 18)

  ARMS's objection to "most probable" applies to the event as a whole: most events end in nothing. EEI's "most likely" and the AI model's "more likely than not" avoid that objection, because both are conditioned on the energy reaching a person.
- **An energy threshold.** EEI admits only releases of high energy, set where "the most likely outcome associated with the hazard is a SIF". This draws on Hallowell, Alexander & Gambatese (2017), who found that energy magnitude predicts injury severity.
- **Explicit rules.** In healthcare, rule-based severity scoring reached κ 0.83–0.90, against 0.63–0.84 for judgement (Snyder et al., 2007). An algorithm for potential harm reached ICC > 0.80 in most cases (Doerper et al., 2015).
- **Facts only.** "Avoid what-if scenarios … the analyst should consider only the facts and avoid fabrications" (Hallowell & Spencer, 2024).

### 6.3 Appeal to probability

Every response that claims objectivity ends up invoking a probability:

- EEI's "most likely outcome" is a probability above one half.
- ASTM's "likelihood" is a probability. ARMS turns its second question, barrier effectiveness, into "the probability of that accident outcome" (ARMS Working Group, 2010, p. 18).
- The law's "more likely than not" is a probability threshold.
- Busch's own remedy is "some assessment probability".

**The inherent subjectivity of PSIF is resolved by appealing to probability.**

The question is then where the probability comes from. If it comes from the assessor's judgement, the subjectivity has only been renamed: Teigen (1998) shows such judgements track closeness, not probability. If it comes from data, such as injury outcomes by energy type and magnitude, it is anchored outside the assessor. March, Sproull & Tamuz (1991) say the aim is to identify "the underlying distribution from which that realization was drawn". Pearl (2019) says a counterfactual's output is a probability computed from a model.

Collapsing the probability to a category also loses information. Risk-matrix bins have poor resolution and invite opposite ratings of the same risk (Cox, 2008). A "most likely" cut at 50% hides an outcome with a 30% chance of being serious.

### 6.4 Calibrated assessors

If the answer is a probability, the people giving it can be trained and measured like any other instrument. Hubbard's work on calibrated estimation is the most practical route (Hubbard, 2014; 2020). It draws on decades of research showing that people are typically overconfident, and that calibration improves with training and feedback (Lichtenstein, Fischhoff & Phillips, 1982).

Hubbard's binary format matches the PSIF question exactly. A statement is presented, and the assessor answers true or false with a confidence of 50% to 100%. A calibrated assessor is right about 70% of the time on the answers they give at 70% confidence. For PSIF:

> *If this energy had reached a person, with no protection, would a serious injury be more likely than not?* **Yes, 70%.**

Four points make this work for PSIF:

1. **Confidence is not P(serious).** "Yes, 70%" means "I am 70% confident a serious injury is more likely than not". It does not mean "P(serious) = 0.70". The format stays a yes/no decision against the threshold, with honesty about how sure the assessor is.
2. **The answer key comes from data.** A counterfactual cannot be checked against what happened. But the question can be put in training and tests for energies whose outcome distributions are known: the injury data behind the AI model's band table. Assessors are then calibrated on the live question, not only on general-knowledge exercises.
3. **Calibration is scored.** Each assessor's answers are compared with the answer key using a Brier score, the mean squared error of their stated probabilities (Brier, 1950). The score shows both accuracy and over- or under-confidence.
4. **Ranges handle missing facts.** When a report omits the height, speed or weight, the assessor gives a 90% confidence interval for it (Hubbard's range estimate) and answers in light of that range. This addresses the thin-report under-call, where the AI model defaults to the lowest band.

**Aggregation adds reliability.** Averaging several judges' probabilities is more reliable than any one judge. Dean & Barber (1999) needed at least four judges per case for a reliable potential-severity score. Forecasting research finds that averaged, calibrated, trained judges outperform individuals (Tetlock & Gardner, 2015).

**Where this sits among the responses.** Calibrated assessment combines all three responses: it restricts who decides (a trained panel), restricts the question (one fixed antecedent), and answers it with a probability. Unlike an uncalibrated panel, its consistency can be measured, and so can its accuracy, against data.

## 7. Two counterfactuals, not one

The critics' examples ("if only…", "should have…", "if he had struck his head just the right way") mix two different counterfactuals.

| | Control counterfactual | Outcome counterfactual |
|---|---|---|
| **Question** | What if the controls, or the people, had acted differently? | What would this energy have done to a person? |
| **Answer comes from** | Imagination about the system and its people | Physics and injury data |
| **Search space** | Open: any change to any barrier or action | Closed: one change, applied to every event alike |
| **Typical bias** | Hindsight; mutability (undoing the exceptional and the controllable) | Calibration error, which can be measured |
| **Where it belongs** | Investigation and learning, labelled as a what-if | Classification |

EEI's SCL model does not abolish the counterfactual. It fixes one. Its "high energy" test asks for the most likely outcome *if the energy is released onto a person*, which is a rule-based counterfactual with a probability threshold. What it bans is variable what-ifs about circumstances and proximity. ARMS does the same. Its first question fixes the antecedent ("If this event had escalated into an accident…"), and its second asks separately: "What was the effectiveness of the remaining barriers between this event and the most credible accident outcome?" (ARMS Working Group, 2010, p. 18). ARMS is candid about what remains subjective, and says how it expects that to be removed:

> "There is likely to be some subjectivity between users in the answer to the first question depending upon how they consider the factors causing the event. … In the longer term it is likely that organisations will identify the outcomes associated with types of events and hence remove the subjectivity associated with the first question for most incidents." (ARMS Working Group, 2010, p. 18)

Elsewhere it concedes that "picking the most credible of the listed scenarios is a subjective judgment" (p. 49). The calibrated outcome distributions in the AI model are the step ARMS anticipated: outcomes identified by type of event, in this case by energy mechanism and band, instead of chosen case by case.

The AI model's definition takes this to its conclusion:

- **One fixed antecedent.** *Separation to zero*: assume the energy reached a person. In Pearl's terms, this is a defined intervention, not a search over similar worlds.
- **All controls set aside, by rule.** Controls are not imagined failing one at a time. They are set aside for every event alike.
- **An answer from data.** The outcome is a calibrated distribution for the energy band, generalising Hallowell, Alexander & Gambatese (2017). "More likely than not" is P(serious) ≥ 0.50, read from that distribution. An Elevated class (P ≥ 0.10) keeps what a single "most likely" cut would hide.
- **Facts only for the inputs.** The band is chosen from the energy the report states.
- **Controls on their own axis.** Credit for controls comes from modelling them, including their failure rates, not from imagining their absence. This matches EEI ("was a direct control present?"), ARMS's barrier question, and Busch et al.'s "grouping … according to control(s) and risk potential".
- **Campbell's "one factor", closed.** Campbell's single changed factor may be "a control that was in place, the location of a person or piece of equipment, the time of day, or even weather conditions" (Campbell Institute, 2018, p. 5). Each is a different counterfactual, and the analyst chooses which one. The AI model changes the same things together and by rule: separation to zero takes location and time to contact, and setting controls aside removes the control. What is a free choice of one factor in Campbell becomes a fixed antecedent.
- **What-ifs kept separate.** What-ifs about other sources or chains of events are recorded separately, labelled counterfactual, and never change the classification.

This answers each part of the objection in section 5:

- **The limit** is one antecedent, applied the same way to every event.
- **The line** is a probability threshold, set by the company.
- **The probability** comes from injury data, not from the assessor.

It also sits on the side of Dekker's line that he accepts. The counterfactual concerns energy reaching a person, not what the worker should have done, and it is used to find cases worth learning from, not to explain why people acted as they did.

What is left is the subjectivity of choosing the band from a narrative. That is reading, not imagining, and agreement between readers can be measured.

## 8. Case surfacing versus trending

| Use | Tolerance for subjectivity | What it needs |
|---|---|---|
| **Surfacing cases** for review or investigation | High. A reviewer looks at each case anyway | Sensitivity: few true SIF-potential events missed |
| **Trending** within an organisation | Low. Changes in classification look like changes in risk | A fixed, versioned rule set; measured agreement; stable reporting detail |
| **Benchmarking** across organisations | Very low | All of the above, plus shared definitions and exposure denominators |

The literature favours surfacing:

- Near-histories enlarge the sample from which an organisation learns (March, Sproull & Tamuz, 1991).
- Decision rules should "look for incidents with SIF Risk Potential" (Martin, dss+).
- A fixed counterfactual with a stated probability makes every qualifying near miss a *vulnerable* one by construction, because it states the harm the event could have caused (Tinsley, Dillon & Cronin, 2012).

It is cautious about metrics:

- "PSIF rates are not unidirectional … a high PSIF rate could equally be the result of poor safety performance or the presence of a strong reporting culture … if PSIF rates are tracked and directly or indirectly incentivized, learning may be severely compromised" (Hallowell & Spencer, 2024).
- Accountability pressure suppresses the counterfactuals that drive learning (Morris & Moore, 2000).

Cooper's question, whether a serious event should prompt re-categorising "all such prior and future events" (Cooper, 2017), shows how hindsight breaks a trend: the classification of past cases moves with the latest outcome. A fixed, versioned rule set avoids this. Past cases are re-run only when the rules change, and every case is re-run at once.

An organisation that sentences by panel can trend internally while the panel stays stable. It cannot benchmark against another organisation's panel. A rule-based counterfactual with a calibrated probability can be applied identically across organisations. Summing the probabilities gives a more stable measure than counting yes/no flags. Even then, differences in report detail and in reporting culture move the results. A PSIF count measures what was reported, not how safe the operation is.

## 9. Research agenda

- **Measure agreement properly.** Report chance-corrected agreement (kappa) between human coders, and between coders and the AI model, on the same reports, with and without the coding rules.
- **Test whether a fixed antecedent reduces variance.** Compare open "could it have been a SIF?" judgements with the fixed counterfactual on identical cases.
- **Separate reading error from imagining error.** Where coders disagree, establish whether they disagree about the facts (the band) or about the counterfactual.
- **Measure outcome bias directly.** Present the same narrative with a benign and with a serious outcome, and compare the judged potential, with and without the fixed rule.
- **Report detail as a confounder.** Measure how classification shifts with narrative completeness, which matters for trending.
- **Framing and learning.** Test whether a probabilistic read-out ("P(serious) = 0.72") moves near-miss learning toward the vulnerable reading.
- **Calibrate the panel, and measure it.** Track each panel member's Brier score on questions with data-derived answers. Test whether calibration training improves agreement with the AI model and between members.
- **Ranges versus the lowest-band default.** Compare classifications from calibrated range estimates with the AI model's lowest-band default on thin reports.
- **Validity, not just reliability.** Check the calibrated distributions against outcomes, so that a consistent rule is also a correct one.

## 10. Conclusion

PSIF is a counterfactual. Its critics are right that an open counterfactual is subjective. Philosophy explains why (similarity is vague), psychology explains how (mutability, closeness, hindsight), and EEI measured it (64% agreement, against a chance level of 50%). Promoters agree: the open question is "inevitably 'yes' if the analyst is creative enough".

The literature's remedies converge on probability. They differ in where the probability comes from. The subjectivity is not in having a counterfactual. It is in leaving the antecedent open and the answer to imagination. To make PSIF consistent enough to trend:

- fix the antecedent by rule,
- take the answer from calibrated outcome data,
- report the distribution rather than a single cut,
- keep what-ifs about controls in investigation, where Dekker accepts them.

Panels and calibration then become a check on reading reports, not a substitute for a definition.

## References

- ARMS Working Group (2010). *The ARMS Methodology for Operational Risk Assessment in Aviation Organisations*, v4.1, March 2010.
- Baron, J. & Hershey, J.C. (1988). Outcome bias in decision evaluation. *Journal of Personality and Social Psychology*, 54(4), 569–579.
- Brier, G.W. (1950). Verification of forecasts expressed in terms of probability. *Monthly Weather Review*, 78(1), 1–3.
- Busch, C. (2019). *If You Can't Measure It… Maybe You Shouldn't: Reflections on Measuring Safety, Indicators, and Goals*. Mind the Risk. Chapter "Sifting Through SIF".
- Busch, C., Usrey, C., Loud, J., Goodell, N. & Carrillo, R.A. (2021). Serious injuries & fatalities: why are they constant while injury rates decrease? *Professional Safety*, 66(1).
- Byrne, R.M.J. (2005). *The Rational Imagination: How People Create Alternatives to Reality*. MIT Press.
- Campbell Institute (2018). *Serious Injury and Fatality Prevention: Perspectives and Practices*. National Safety Council.
- Caplan, R.A., Posner, K.L. & Cheney, F.W. (1991). Effect of outcome on physician judgments of appropriateness of care. *JAMA*, 265(15), 1957–1960.
- Cooper, D. (2014). Reducing serious injuries & fatalities: link SIF programs to your safety culture. *ISHN*, 3 September 2014.
- Cooper, D. (2017). *Practical Steps to Reduce Serious Injuries & Fatalities (SIFs)*. B-Safe Management Solutions (presentation).
- Cooper, M.D. (2019). The efficacy of industrial safety science constructs for addressing serious injuries & fatalities (SIFs). *Safety Science*, 120, 164–178. doi:10.1016/j.ssci.2019.06.038
- Cox, L.A. Jr. (2008). What's wrong with risk matrices? *Risk Analysis*, 28(2), 497–512. doi:10.1111/j.1539-6924.2008.01030.x
- Dean, B.S. & Barber, N.D. (1999). A validated, reliable method of scoring the severity of medication errors. *American Journal of Health-System Pharmacy*, 56(1), 57–62.
- Dekker, S. (2014). *The Field Guide to Understanding 'Human Error'*, 3rd ed. Ashgate.
- Dekker, S.W.A. (2002). Reconstructing human contributions to accidents: the new view on error and performance. *Journal of Safety Research*, 33, 371–385.
- Dillon, R.L. & Tinsley, C.H. (2008). How near-misses influence decision making under risk: a missed opportunity for learning. *Management Science*, 54(8), 1425–1440.
- Doerper, S., Godet, J., et al. (2015). Development and multi-centre evaluation of a method for assessing the severity of potential harm of medication reconciliation errors at hospital admission in elderly. *European Journal of Internal Medicine*, 26(7), 491–497. doi:10.1016/j.ejim.2015.07.014
- Edison Electric Institute (2020, rev. 2024). *Safety Classification and Learning (SCL) Model*.
- Fernando, A., Smith, W., Hawkes, A. & Mee, K. (2024). Meta incident analysis: an analysis of incident and near miss reporting within a major Australian commodity port. *International Mine Health and Safety Conference Proceedings 2024*. AusIMM.
- Fischhoff, B. (1975). Hindsight ≠ foresight: the effect of outcome knowledge on judgment under uncertainty. *Journal of Experimental Psychology: Human Perception and Performance*, 1(3), 288–299. doi:10.1037/0096-1523.1.3.288
- Hallowell, M.R. & Spencer, C. (2024). Defining & classifying potential serious injuries & fatalities: the safety classification and learning model. *Professional Safety*, 69(2), 18–26.
- Hallowell, M.R., Alexander, D. & Gambatese, J.A. (2017). Energy-based safety risk assessment: does magnitude and intensity of energy predict injury severity? *Construction Management and Economics*, 35(1–2), 64–77.
- Hayward, R.A. & Hofer, T.P. (2001). Estimating hospital deaths due to medical errors: preventability is in the eye of the reviewer. *JAMA*, 286(4), 415–420.
- Hubbard, D.W. (2014). *How to Measure Anything: Finding the Value of Intangibles in Business*, 3rd ed. Wiley.
- Hubbard, D.W. (2020). *The Failure of Risk Management: Why It's Broken and How to Fix It*, 2nd ed. Wiley.
- Kahneman, D. & Miller, D.T. (1986). Norm theory: comparing reality to its alternatives. *Psychological Review*, 93(2), 136–153. doi:10.1037/0033-295X.93.2.136
- Kahneman, D. & Tversky, A. (1982). The simulation heuristic. In Kahneman, Slovic & Tversky (eds), *Judgment under Uncertainty: Heuristics and Biases*, 201–208. Cambridge University Press.
- Kahneman, D. & Varey, C.A. (1990). Propensities and counterfactuals: the loser that almost won. *Journal of Personality and Social Psychology*, 59(6), 1101–1110.
- Kapur, A., Evans, S., Brown, D., et al. (2016). TU-D-201-04: Veracity of data elements in radiation oncology incident learning systems. *Medical Physics*, 43(6) (conference abstract).
- Krause, T.R. (2010). *New Findings on Serious Injuries and Fatalities*. BST.
- Lewis, D. (1973a). *Counterfactuals*. Blackwell / Harvard University Press.
- Lewis, D. (1973b). Causation. *Journal of Philosophy*, 70(17), 556–567. doi:10.2307/2025310
- Lewis, D. (1979). Counterfactual dependence and time's arrow. *Noûs*, 13(4), 455–476.
- Lichtenstein, S., Fischhoff, B. & Phillips, L.D. (1982). Calibration of probabilities: the state of the art to 1980. In Kahneman, Slovic & Tversky (eds), *Judgment under Uncertainty: Heuristics and Biases*, 306–334. Cambridge University Press.
- Localio, A.R., Weaver, S.L., Landis, J.R., et al. (1996). Identifying adverse events caused by medical care: degree of physician agreement in a retrospective chart review. *Annals of Internal Medicine*, 125(6), 457–464.
- Manaseki-Holland, S., Lilford, R.J., Te, A.P., et al. (2019). Ranking hospitals based on preventable hospital death rates: a systematic review with implications for both direct measurement and indirect measurement through standardized mortality rates. *Milbank Quarterly*, 97(1), 228–284. doi:10.1111/1468-0009.12375
- March, J.G., Sproull, L.S. & Tamuz, M. (1991). Learning from samples of one or fewer. *Organization Science*, 2(1), 1–13. doi:10.1287/orsc.2.1.1
- Martin, D.K. & Black, A.A. (2015). Preventing serious injuries & fatalities: study reveals precursors & paradigms. *Professional Safety*, 60(9), 35–43.
- Martin, D.K. (c. 2022). *What do you really "know" about your company's SIF risk potential?* dss+.
- Morris, M.W. & Moore, P.C. (2000). The lessons we (don't) learn: counterfactual thinking and organizational accountability after a close call. *Administrative Science Quarterly*, 45(4), 737–765. doi:10.2307/2667018
- Mullen, T., Nyflot, M., Zeng, J., et al. & Ford, E.C. (2016). Interrater reliability of a near-miss risk index for incident learning systems in radiation oncology. *Practical Radiation Oncology*, 6(6), 429–435. doi:10.1016/j.prro.2016.04.002
- Pearl, J. & Mackenzie, D. (2018). *The Book of Why*. Basic Books.
- Pearl, J. (2009). *Causality*, 2nd ed. Cambridge University Press.
- Pearl, J. (2019). The seven tools of causal inference, with reflections on machine learning. *Communications of the ACM*, 62(3), 54–60.
- Roese, N.J. (1997). Counterfactual thinking. *Psychological Bulletin*, 121(1), 133–148. doi:10.1037/0033-2909.121.1.133
- Snyder, R.A., Abarca, J., Meza, J.L., et al. (2007). Reliability evaluation of the adapted National Coordinating Council Medication Error Reporting and Prevention (NCC MERP) index. *Pharmacoepidemiology and Drug Safety*, 16(9), 1006–1013.
- Teigen, K.H. (1998). When the unreal is more likely than the real: post hoc probability judgements and counterfactual closeness. *Thinking & Reasoning*, 4(2), 147–177.
- Tetlock, P.E. & Gardner, D. (2015). *Superforecasting: The Art and Science of Prediction*. Crown.
- Thomas, E.J., Lipsitz, S.R., Studdert, D.M. & Brennan, T.A. (2002). The reliability of medical record review for estimating adverse event rates. *Annals of Internal Medicine*, 136(11), 812–816.
- Tinsley, C.H., Dillon, R.L. & Cronin, M.A. (2012). How near-miss events amplify or attenuate risky decision making. *Management Science*, 58(9), 1596–1613. doi:10.1287/mnsc.1120.1517
- Tinsley, C.H., Dillon, R.L. & Madsen, P.M. (2011). How to avoid catastrophe. *Harvard Business Review*, 89(4), 90–97.
- Woods, D.D. & Cook, R.I. (1999). Perspectives on human error: hindsight biases and local rationality. In F.T. Durso et al. (eds), *Handbook of Applied Cognition*, 141–171. Wiley.

## Verification notes

*For the author; remove before publication.*

- **Quotes read in full text:** Hallowell & Spencer (2024), March et al. (1991), Tinsley et al. (2012), Dekker (2002), Woods & Cook (1999, preprint), Pearl (2019), Busch et al. (2021), Martin (dss+).
- **Busch (2019), chapter "Sifting Through SIF":** both passages are from the author's Kindle copy, which has no page numbers, so they are cited by chapter. If a print copy is available, add page numbers. Check the subtitle and publisher against the book.
- **Krause (2010) and Martin & Black (2015):** checked against local PDFs. Krause's precursor definition is on p. 7. Martin & Black's ">90%" inter-rater reliability is on PDF p. 4; confirm the journal page.
- **EEI SCL quote:** add the page number from the EEI report.
- **Campbell Institute (2018, p. 5):** verified from the author's copy.
- **From abstracts or secondary sources, not yet read in full:** Kahneman & Tversky (1982), Kahneman & Miller (1986), Kahneman & Varey (1990), Roese (1997), Byrne (2005), Teigen (1998), Morris & Moore (2000), Dillon & Tinsley (2008), Fischhoff (1975), Cox (2008), Fernando et al. (2024).
- **Lewis (1973a, p. 1; 1973b, pp. 556–558):** verified from the author's copies. The "makes a difference" passage is on p. 557 of *Causation*. Lewis states it as the common intuition he goes on to analyse, not as his definition. The PDF is a scan, so text search does not find it.
- **Healthcare:** the earlier "κ ≈ 0.2 for potential harm fields" claim is replaced. Its sources were a three-student conference abstract (Pappas et al., 2015) and an actual-harm nurse vignette study (Lee et al., 2020), so neither supported it. Read in full: Hogan et al. (2012), Manaseki-Holland et al. (2019). Read as abstracts: Mullen et al. (2016), Kapur et al. (2016; conference abstract), Dean & Barber (1999), Doerper et al. (2015), Snyder et al. (2007), Thomas et al. (2002), Localio et al. (1996), Hayward & Hofer (2001), Caplan et al. (1991). Hayward & Hofer's 0.34 is a reliability for two reviewers, not explicitly a κ.
- **Cooper:** Cooper (2019, *Safety Science*) describes itself as an opinion piece. Cooper (2017) is a slide deck; Cooper (2014) is a trade article. Quotes read in full text.
- **ARMS Working Group (2010), v4.1, March 2010:** pp. 8, 18, 40, 41 and 49 verified from the author's copy. The Baines Simmons blog quote has been removed.
- **Page numbers and DOIs:** confirm for Lewis (1979), Kahneman & Varey (1990), Dillon & Tinsley (2008), Busch et al. (2021) and Dekker (2002).
- **Calibration sources (6.4):** Hubbard (2014, 2020), Lichtenstein, Fischhoff & Phillips (1982), Brier (1950) and Tetlock & Gardner (2015) are cited for their general findings, from memory. Check editions and pages before quoting. The binary 50–100% confidence format is as described by the author from Hubbard's training method.
