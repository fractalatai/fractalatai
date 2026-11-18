# What counts as serious: LIFE, trauma scales and the life-altering gap

Status: draft v0.1, 2026-11-17. A literature review in the series on defining serious injury and fatality (SIF) potential. Definitions follow [sif-definition.md](sif-definition.md). Companion papers: [sif-counterfactual.md](sif-counterfactual.md) and [sif-controls-precursors.md](sif-controls-precursors.md).

## Executive summary

*For the safety director.*

### The problem

"Serious" is defined in three different ways, and the AI model uses a mixture of them:

1. **The LIFE model** (Bayona et al., 2023) defines a serious injury as one that is life-ending, life-threatening or life-altering. These are three yes/no gates, applied to the actual outcome. Most SIF definitions now use LIFE or close variants, including EEI, ASTM E2920 and the Campbell Institute.
2. **Regulatory lists** (RIDDOR specified injuries, OSHA severe-injury reporting, EEI's 15 SIF criteria) name injuries that count. They disagree with each other at the margins, most of all at the hand.
3. **Trauma scales** (AIS, ISS) rate injuries by their **threat to life**. The AI model's severity scale comes from this family: each energy band has an outcome distribution calibrated to the chance of death.

The AI model's definition says "life-ending, life-threatening or life-altering", but its scale measures only the first two. An injury that rarely kills but changes a life (a finger or hand amputation, sight loss, a crushed hand) scores low. In the AI model, the bands where such injuries are common are often *Elevated*, not *SIF*:

| Band | P(serious) | Class | Typical life-altering outcome |
|---|---|---|---|
| `caught_in/light_machinery` | 0.100 | Elevated | Finger amputation at a drill, bench saw or press |
| `chemical/corrosive_localised` | 0.100 | Elevated | Sight loss from a splash to the eye |
| `radiation_noise/laser_high_power` | 0.100 | Elevated | Permanent retinal damage |

Under LIFE, these outcomes are serious. The trauma literature has the same problem: the ISS > 15 threshold for "major trauma" is criticised as "arbitrary (as it addresses only one dimension of injury severity …)" (Palmer, 2007).

### The fix

Treat severity as **two axes**, threat to life and permanent impairment, and count an outcome as serious if it crosses either.

| Axis | What it covers | Current state in the AI model | Evidence base |
|---|---|---|---|
| **Threat to life** | Life-ending and life-threatening | Calibrated (AIS-derived) | Strong: AIS is the standard trauma scale |
| **Permanent impairment** | Life-altering | Missing | Moderate: Swedish insurance data give the risk of permanent impairment by body region and AIS level (Malm et al., 2008) |

### Recommendations

1. **Keep LIFE as the definition of a serious outcome.** It is the most widely adopted definition, and its three gates cover the right ground. Use it as the target the AI model is calibrated against, not as a scale.
2. **Add a body-region rule now.** Flag the bands where energy is usually delivered to the hand, fingers or eyes, and set a probability of a life-altering outcome for them. This is transparent, needs no new data and matches how WorkSafeBC and Australian regulators single out hands and eyes.
3. **Build an impairment axis later.** For each band, model the chance of permanent impairment using published tables of impairment risk by body region and AIS level (Malm et al., 2008). Take P(serious) as the larger of the two axes until the joint distribution is known.
4. **Follow LIFE at the hand, not EEI.** LIFE counts the loss of any body part and any permanent loss of function. EEI's 2025 criteria exclude all finger and toe fractures, and fingertip amputations other than the thumb, index finger or great toe. Code to LIFE, and record the EEI criterion alongside it for benchmarking.
5. **Use regulatory lists for calibration and audit, not as the definition.** They are outcome-only and disagree with each other. They are useful as labelled cases to test the AI model against.
6. **Check the AIS licence.** AAAM, which owns AIS, prohibits "the use of all content in any way with generative AI, including training, scraping, or ingestion". The AI model uses AIS severity levels and published mortality rates, not dictionary content. Confirm that this is outside the restriction before any impairment axis draws on AIS-coded material.

## Contents

- [1. The question](#1-the-question)
- [2. The LIFE model](#2-the-life-model)
- [3. Who uses LIFE](#3-who-uses-life)
- [4. Trauma scales: threat to life](#4-trauma-scales-threat-to-life)
- [5. Impairment scales: life-altering](#5-impairment-scales-life-altering)
- [6. Regulatory lists](#6-regulatory-lists)
- [7. Comparison](#7-comparison)
- [8. The AI model's position](#8-the-ai-models-position)
- [9. Research agenda](#9-research-agenda)
- [References](#references)
- [Verification notes](#verification-notes)

## 1. The question

SIF potential asks whether an injury would more likely than not have been serious, had the energy reached a person. That needs a definition of "serious", and a scale on which to measure the chance of it.

There are two kinds of answer in the literature:

- **Intensional definitions** state the property that makes an injury serious (LIFE: it ends, threatens or forever changes a life).
- **Extensional definitions** list the injuries that count (RIDDOR, OSHA, EEI's criteria).

Bayona et al. (2023) argue for the first: "extensional definitions are generally less mature and must be updated every time a new incident type is encountered" (p. 23), and "As history shows, extensional definitions yield conflict and discomfort" (p. 29).

Neither kind is a scale. A probabilistic model needs a third thing: a measure of severity with known links to outcomes. Trauma medicine has one for threat to life (AIS). It has weaker ones for impairment.

## 2. The LIFE model

### 2.1 Definitions

Bayona et al. (2023) define "the life-centered injury and fatality evaluation (LIFE) model" (p. 25). "The general philosophy behind the LIFE model is that the incident ends, threatens or forever changes the life of the injured person" (p. 25).

- **Life-ending:** "A case that results in the death of the injured person" (p. 25).
- **Life-threatening:** "A case that requires immediate medical intervention or lifesaving support (e.g., CPR, defibrillation) to save the injured person's life. Any incident where a lifesaving intervention or medical care without which death would have been imminent is considered life-threatening" (p. 25). The authors anchor this to trauma-centre triage (Tanabe et al., 2007, the Emergency Severity Index), not to AIS.
- **Life-altering:** "A case in which the injured worker will not fully recover and will most likely suffer permanent impairment from the loss of the use a major internal organ (e.g., brain, heart, lungs, liver, kidneys), body function or body part" (p. 26). "The expert panel recommended considering the loss of any body part as serious for two main reasons: 1. doing so maintains the simplicity in the model and 2. this type of injury is irreversible" (p. 26).

A body function is "a psychological or physiological function of the body system such as vision, range of motion or spatial orientation", and body parts are "anatomical parts of the body (e.g., organs, limbs and their components) that support body functions" (p. 26).

The Construction Safety Research Alliance (CSRA) publishes a one-page version that adds two provisions. Life-altering must be confirmed by a "QUALIFIED PERSON: Individual with reasonable knowledge to make a determination about the IP's condition (e.g., a medical professional)", and "Cases should be reclassified if new information becomes available" (CSRA, 2022).

### 2.2 Scope

LIFE is for actual outcomes only:

- "the expert panel opted to focus on actual outcomes of the incident only … Assessing potential severity is recommended for future work" (p. 25). The CSRA handout adds: "Use the LIFE Model for determining if an ACTUAL injury is serious or not. You may use other models available for assessing the POTENTIAL severity of an incident (e.g., EEI SCL Model)."
- "pain, illnesses and mental health concerns were not in the scope of this study" (p. 25).
- Activities of daily living were rejected because "the information required to assess ADLs is not accessible by most companies because of the need to protect the privacy of the injured person" (p. 25).
- Treatment-based definitions were rejected because of "the variability in medical practitioners, healthcare and medical systems, and workforces across different geographies" (p. 24).

### 2.3 How it was developed and tested

LIFE came from "four focus group discussions … held over the course of a year", managed by "a panel of three academic researchers" (p. 24). Panellists from the US, Canada and the UK averaged 16 years in safety. They "chose to start anew rather than adopt or adapt traditional approaches" (p. 24). The paper does not mention AIS, ISS or any impairment scale.

The test used 18 cases from the OSHA severe-injury database, nine SIF and nine not, before and after a short training video (pp. 26–27). Fatalities were excluded. Forty-nine practitioners took part before training and 39 after. Results:

- "the level of variation among participants decreased significantly (23% overall reduction from a standard deviation of 1.41 to 1.08)" (p. 28);
- internal consistency (KR-20) rose from 0.23 to 0.53 on one survey and from −0.44 to 0.21 on the other (Table 4, p. 29);
- several cases stayed split after training, one at 39/61 (Table 3, p. 28);
- alignment with company philosophy fell significantly (Table 5, p. 29).

The authors note that "The residual variability may be explained, in part, because only a brief introductory video was used" (p. 28). There is no kappa or other agreement statistic, and no independent replication was found.

### 2.4 The fingertip

The paper opens with a test case: "A construction worker lost the tip of a finger when unstable materials unexpectedly shifted in the bed of a truck. The injury involves bone damage, and the piece of finger cannot be reattached. In other words, the consequences of this injury will be permanent but there is technically no disablement because the functionality of the finger will not be affected" (p. 23). Agencies and companies split on it (Table 1, p. 25), which shows "the need for a more precise threshold" (p. 23). LIFE's rule that the loss of any body part is serious makes it a SIF.

## 3. Who uses LIFE

| Body | Definition | LIFE? |
|---|---|---|
| **EEI SIF criteria** (effective 2025) | "Life-Threatening: A physical injury that if not immediately addressed is likely to lead to the death of the affected individual …"; "Life-Altering: A physical injury that results in permanent loss of use of an internal organ, body function, or body part" (EEI, 2024, Appendix 9, p. 39). Plus 15 criteria | LIFE-like wording, list-based in practice |
| **ASTM E2920-26** | aSIF: "life-ending, life-threatening, or life-altering consequences" (*unverified*: seen only in the ASTM store summary, not saved) | The LIFE terms; LIFE not named |
| **Campbell Institute** (2020) | "Most Institute members have adopted the definition of a serious injury as a permanent impairment or life-altering state, or an injury that if not immediately addressed will lead to death or permanent or long-term impairment" | Close |
| **Cooper** (2019) | Life-altering: "Probable survival - Permanent or long‐term impairment or loss of use of an internal organ, body function, or body part" (Table 1, p. 166) | Predates LIFE with similar wording |
| **ANSI/ASSP Z16.1-2022** | No single definition by design (*secondary*: ASSP, 2026) | No |

The EEI SCL model defers to the EEI criteria: "Creating a definition of serious injury and fatality (SIF) was outside the scope of this work. The team deferred to the existing EEI SIF criteria" (EEI, 2024, p. 11).

**EEI and LIFE now disagree at the hand.** EEI's 2025 criteria cover amputations involving bone, but exclude the distal phalanx "unless thumb, index or great toe". They cover fractures requiring surgery, but not of fingers or toes: "All fractures of the fingers and toes are excluded" (EEI, 2024, p. 40). For injuries not listed, EEI asks coders to "select only if an employee is unable to engage in prior level of work functional ability" (p. 42). Two of Bayona's own cases are SIF under LIFE but not under EEI: the fingertip (section 2.4), and two broken fingers with "permanently lost motor function" (Table 2, p. 27).

## 4. Trauma scales: threat to life

### 4.1 AIS

The Abbreviated Injury Scale is maintained by the Association for the Advancement of Automotive Medicine (AAAM). "It represents an assessment of the severity of injury including the threat to life associated with the injury." Its severity runs "1 to 6, one being a minor injury and six being maximal", and "The AIS scale is a measurement tool for single injuries" (AAAM, n.d.). The current edition is the 2015 revision.

AIS is "mainly a threat-to-life scale and only assesses a single injury" (Malm et al., 2008). Early data showed how steeply mortality rises with AIS (Baker et al., 1974, as reproduced by Dehouche, 2021):

| Maximum AIS | Mortality |
|---|---|
| 1 | 0% |
| 2 | 0.5% |
| 3 | 3% |
| 4 | 16% |
| 5 | 64% |

The AI model's threshold for a serious injury (about 10% mortality) falls between AIS 3 and AIS 4. The same AIS level does not carry the same risk in every body region: "the risks to mortality in the patients with a given AIS value of serious to critical injury in different injured body regions were not the same" (Rau et al., 2017).

### 4.2 ISS and NISS

The Injury Severity Score combines AIS across body regions (Baker et al., 1974). The New ISS sums "the squares of the Abbreviated Injury Scale scores of each of a patient's three most severe Abbreviated Injury Scale injuries regardless of the body region", and predicts mortality better (Osler et al., 1997).

"Major trauma is commonly defined using an Injury Severity Score (ISS) threshold of 15", a threshold "first described by Boyd et al in 1987 as being predictive of 10% mortality". Palmer calls it "both arbitrary (as it addresses only one dimension of injury severity …) and changeable", and warns that it "may exclude a substantial body of data relating to significantly morbid trauma patients" (Palmer, 2007). That is the life-altering gap, stated by trauma researchers.

### 4.3 AIS in occupational work

No occupational SIF definition found uses AIS. It appears in occupational epidemiology. Sears et al. (2015) estimated AIS from workers' compensation data and note that AIS "rates injury severity based on threat to life and does not take comorbidity or complications into account". The EU adopted MAIS3+ as its definition of a seriously injured road casualty in 2013 (*secondary*).

## 5. Impairment scales: life-altering

| Scale | What it measures | Status | Fit as a life-altering axis |
|---|---|---|---|
| **Injury Impairment Scale** (AAAM, 1994) | Predicted impairment one year after a single injury, 0–6, across "mobility, cognitive, cosmetic interfering with function, sensory, sexual /reproduction and pain" (Barnes & Morris, 2009) | "has not been widely adopted"; tied to AIS 1990 | Right idea, obsolete |
| **Functional Capacity Index** (MacKenzie et al., 1996) and **pFCI** | "expected levels of reduced functional capacity at 1 year after injury", mapped from AIS codes | pFCI was in the AIS 2005 dictionary; status in AIS 2015 not confirmed | Weak: "A modest correlation was found between FCI scores and actual levels of impairment observed at 1 year" |
| **Risk of Permanent Medical Impairment** (Malm et al., 2008) | Empirical risk of ≥1%, 5% or 10% permanent impairment by body region and AIS level, from 20,484 insurance claimants followed for at least 5 years | Published tables; road-crash data | **Best fit**: probabilistic, keyed to the same AIS structure as the AI model |
| **GBD disability weights** (Salomon et al., 2012) | Health loss on a 0–1 scale, where "1 implies that a state is equivalent to death" | Maintained (IHME) | Not mapped to injuries; inconsistent for the hand |
| **AMA Guides**, sixth edition (2025) | Whole-person impairment at maximum medical improvement | Maintained (AMA) | The reference for after-the-fact audit; not usable before the outcome |

The two axes really are different. Malm et al. (2008) found that "A person surviving an AIS4+ thoracic injury (excluding the spine) usually recovers without functional reduction", while an AIS 3 upper-limb injury carries a 15% risk of at least 10% impairment, and "almost 10% of all car occupants with AIS1 injuries sustained a permanent medical impairment". Their impairment examples include "37% for loss of one hand, 68% for total blindness".

Predicting impairment is hard. IIS and pFCI disagreed four-fold on the same crash data (6% against 24% impaired), and "the simplicity of assigning one single score to every injury does not take into account the fact that some individuals will recover without impairment" (Barnes & Morris, 2009). "there is a poor correlation between immediate assessment of the severity of an injury and the long-term consequences" (Malm et al., 2008).

GBD weights show the problem with a single health-loss number: amputation of fingers other than the thumb is weighted 0.030, the thumb 0.013 and one leg, treated, 0.021 (Salomon et al., 2012, Table 2).

## 6. Regulatory lists

| | LIFE | EEI 2025 | RIDDOR | OSHA | Cal/OSHA | WorkSafeBC | Australia (WA) |
|---|---|---|---|---|---|---|---|
| Fingertip amputation | Yes | Only thumb, index, great toe | "amputation of … finger" | Yes: "fingertip amputations with or without bone loss" | Yes (amputation) | Yes, if bone or joint | Yes: "the tip of a finger" |
| Finger fracture, permanent loss of movement | Yes | No | No: fractures of fingers excluded | Only if hospitalised | Only if hospitalised | Not listed | "loss of movement of a limb" |
| Sight loss | Yes | Yes | "likely to cause permanent blinding or reduction in sight" | "loss of an eye" | "loss of an eye" | Yes | Yes |
| Resuscitation needed | Yes (life-threatening) | Yes | Yes, in enclosed spaces: "requires resuscitation" | Only if hospitalised | Only if hospitalised | "critical lifesaving intervention, such as … CPR" | Not as such |
| Hospital admission alone | No | No | Over 24 hours, enclosed spaces only | Yes | Yes | No | Yes, "for any duration" |

Sources: RIDDOR 2013 reg. 4(1); 29 CFR 1904.39; California Labor Code §6302(h); WorkSafeBC guideline G-P2-68-1; WorkSafe WA guidance on the WHS Act s.36.

Most lists mix life-altering items (amputation, sight, disfigurement) with treatment triggers (hospital admission, days off work), which LIFE rejects. WorkSafeBC is the closest regulatory analogue to SIF *potential*: employers must report when "the circumstances of an accident reasonably indicate that an injury may endanger a worker's life or cause a worker significant physical impairment", including by mechanism, "even where symptoms are not yet evident" (WorkSafeBC, G-P2-68-1).

## 7. Comparison

| Scheme | Measures | Output | When it can be coded | Role for the AI model |
|---|---|---|---|---|
| LIFE | Threat to life and permanent impairment | Three yes/no gates | After the outcome; life-altering may need a medical view and later revision | The target definition |
| EEI SIF criteria | LIFE-like, applied through 15 items | Yes/no per item | At diagnosis or surgery | Benchmark label |
| AIS / ISS / NISS | Threat to life | Ordinal (AIS 1–6; ISS 1–75) | At diagnosis | The current axis |
| IIS / FCI / pFCI | Predicted impairment | Ordinal | From the AIS code | Cross-check only |
| RPMI | Risk of permanent impairment | Probability by body region and AIS | From region and AIS | The candidate second axis |
| GBD weights / AMA Guides | Health loss / whole-person impairment | 0–1 / % | Health state / months later | Audit |
| Regulatory lists | Mixed | Reportable or not | Within days | Calibration and audit labels |

## 8. The AI model's position

1. **The definition is LIFE.** A serious outcome is life-ending, life-threatening or life-altering ([sif-definition.md](sif-definition.md), section 5.5). This matches EEI, ASTM and Campbell.
2. **The scale is threat to life.** Each band's outcome distribution is AIS-derived and mortality-calibrated, and P(serious) is the chance of an outcome at the serious level or worse (about 10% mortality). This captures life-ending and life-threatening outcomes. It under-counts life-altering ones.
3. **LIFE has no potential element, and the AI model supplies it.** LIFE judges actual outcomes only, and leaves potential severity to "future work". The AI model's separation-to-zero counterfactual and band table are the potential layer that LIFE lacks.

**Closing the gap.** There are three options:

| Option | How it works | For | Against |
|---|---|---|---|
| **Body-region rule** (interim) | Flag bands where energy usually reaches the hand, fingers or eyes (light machinery, hand-held cutting tools, hydraulic injection, corrosives and lasers to the eye, molten metal on the hands) and set P(life-altering) for them | Transparent; needs no AIS content; matches WorkSafeBC and Australian lists; supported by Hallowell et al. (2017): "This residual variability can be primarily attributed to the vulnerability of the body part affected" (p. 73) | Brings back list-like choices; thresholds at the hand are contested |
| **Impairment axis** (long term) | For each band, model the spread of body region and injury level, then map it to P(permanent impairment) using RPMI-type tables | Empirical and probabilistic, like the current calibration. Two-axis ratings have precedent in Swedish road safety (*secondary*) | Road-crash data; the impairment threshold is a value choice (is 10% life-altering?); needs body-region distributions per band; weak prediction validity |
| **List overlay** | Apply EEI or RIDDOR items as extra SIF triggers | Aligns with how users benchmark | Outcome-only; the lists disagree with each other and with LIFE |

**Combining the axes.** The joint distribution is unknown, and the two axes are correlated, so 1 − (1 − a)(1 − b) would overstate the chance. Take P(serious) as the larger of P(threat to life) and P(life-altering) as a conservative floor, and state that this is the rule used.

**Two series fixes** (applied in definitions v1.8 and discussion v1.8, 2026-11-18):

- The discussion document says "serious" is "P(death) ≥ 0.10". The definitions document says SIF is P(serious) ≥ 0.50. Both are right, but they are different thresholds: the first defines the serious level on the scale, and the second is the probability of reaching it. The wording now makes that explicit.
- The definitions document's summary of RIDDOR (section 5.6) now adds reg. 4(1)(h)(ii), "requires resuscitation", and quotes the sight item as "likely to cause permanent blinding or reduction in sight".

## 9. Research agenda

- **Size the gap.** Code a sample of actual life-altering injuries (amputations, sight loss, permanent loss of hand function) and compare them with the AI model's P(serious) for the same events.
- **Body-region distributions.** Estimate, for each band, how often the energy reaches the hand, eyes, head or trunk, from incident data.
- **The impairment threshold.** Decide what level of permanent impairment counts as life-altering (Sweden uses ≥1% for seriously injured and ≥10% for very seriously injured, *secondary*).
- **Coder agreement on life-altering.** Measure agreement on the life-altering gate, with and without a medical view, since LIFE's own test reported no agreement statistic.
- **Hand definitions.** Compare LIFE, EEI and RIDDOR coding on the same hand-injury cases, and record where they diverge.

## References

- AAAM (n.d.). *AIS frequently asked questions*. Association for the Advancement of Automotive Medicine. aisinjuryscale.org.
- Baker, S.P., O'Neill, B., Haddon, W. & Long, W.B. (1974). The Injury Severity Score: a method for describing patients with multiple injuries and evaluating emergency care. *Journal of Trauma*, 14(3), 187–196.
- Barnes, J. & Morris, A. (2009). A study of impairing injuries in real world crashes using the Injury Impairment Scale (IIS) and the predicted Functional Capacity Index (PFCI-AIS). *Annals of Advances in Automotive Medicine*, 53, 195ff. PMID 20184844.
- Bayona, A., Bhandari, S., Hallowell, M., Sherratt, F., Bailey, J.M. & Upton Jr., J. (2023). What is a serious injury? A model for defining serious injuries and fatalities. *Professional Safety*, 68(9), 22–30.
- California Labor Code §6302(h).
- Campbell Institute (2020). *Designing Strategy for Serious Injury and Fatality Prevention*. National Safety Council.
- Cooper, M.D. (2019). The efficacy of industrial safety science constructs for addressing serious injuries & fatalities (SIFs). *Safety Science*, 120, 164–178. doi:10.1016/j.ssci.2019.06.038
- CSRA (2022). *The LIFE Model*. Construction Safety Research Alliance, University of Colorado.
- Dehouche, N. (2021). On some statistical and axiomatic properties of the Injury Severity Score. arXiv:2102.04233.
- Edison Electric Institute (2024). *Safety Classification and Learning (SCL) Model*, rev. September 2024. Appendix 9: EEI SIF criteria, approved changes effective 1 January 2025.
- Hallowell, M.R., Alexander, D. & Gambatese, J.A. (2017). Energy-based safety risk assessment: does magnitude and intensity of energy predict injury severity? *Construction Management and Economics*, 35(1–2), 64–77. doi:10.1080/01446193.2016.1274418
- MacKenzie, E.J., Damiano, A., Miller, T. & Luchter, S. (1996). The development of the Functional Capacity Index. *Journal of Trauma*, 41(5), 799–807. doi:10.1097/00005373-199611000-00006
- Malm, S., Krafft, M., Kullgren, A., Ydenius, A. & Tingvall, C. (2008). Risk of permanent medical impairment (RPMI) in road traffic accidents. *Annals of Advances in Automotive Medicine*, 52, 93ff. PMID 19026226.
- Osler, T., Baker, S.P. & Long, W. (1997). A modification of the Injury Severity Score that both improves accuracy and simplifies scoring. *Journal of Trauma*, 43(6), 922–925. doi:10.1097/00005373-199712000-00009
- OSHA, 29 CFR 1904.39. Reporting fatalities, hospitalizations, amputations, and losses of an eye.
- Palmer, C. (2007). Major trauma and the Injury Severity Score: where should we set the bar? *Annual Proceedings of the Association for the Advancement of Automotive Medicine*, 51, 13ff.
- Rau, C.-S. et al. (2017). Same Abbreviated Injury Scale values may be associated with different risks to mortality in trauma patients: a cross-sectional retrospective study based on the trauma registry system in a level I trauma center. *International Journal of Environmental Research and Public Health*, 14, 1552. doi:10.3390/ijerph14121552
- RIDDOR: The Reporting of Injuries, Diseases and Dangerous Occurrences Regulations 2013, reg. 4.
- Salomon, J.A. et al. (2012). Common values in assessing health outcomes from disease and injury: disability weights measurement study for the Global Burden of Disease Study 2010. *The Lancet*, 380, 2129–2143. doi:10.1016/S0140-6736(12)61680-8
- Sears, J.M., Bowman, S.M., Rotert, M. & Hogg-Johnson, S. (2015). A new method to classify injury severity by diagnosis: validation using workers' compensation and trauma registry data. *Journal of Occupational Rehabilitation*, 25, 742–751. doi:10.1007/s10926-015-9582-5
- WorkSafe WA. *Serious injury or illness*. Guidance on the Work Health and Safety Act 2020 (WA) s.36.
- WorkSafeBC. Guideline G-P2-68-1, preliminary revision 17 September 2026.

## Verification notes

*For the author; remove before publication.*

- **Research:** by a research agent on 2026-11-17. Its downloads (41 files) are in `docs/sif/lit/severity/` (gitignored), including the primary LIFE paper `bayona_2023_psj.pdf`, the CSRA LIFE handout, the EEI 2025 criteria and the regulation texts.
- **Bayona et al. (2023)** has no DOI. Free PDF: assp.org/docs/default-source/psj-articles/f1bayona_0923.pdf.
- **Secondary only:** ASTM E2920-26 (store summary); ANSI/ASSP Z16.1-2022 (via an ASSP article); the EU MAIS3+ definition; Sweden's impairment thresholds; Gustafsson's two-axis rating (via Malm et al.); Boyd et al. (1987) (via Palmer).
- **Not obtained:** the AIS 2015 dictionary; the AAAM IIS manual; Baker (1974) and Boyd (1987) full texts; the Australian WHS Act s.36 text (regulator guidance used); Bayona et al. (2022, CRC).
- **Quotation check (2026-11-17).** Every quotation was matched word for word against the saved source text, allowing for line breaks and hyphenation. The exceptions:
  - **Unverified:** the ASTM E2920-26 wording (section 3), which was seen only in the ASTM store summary.
  - **Not source quotes:** "life-ending, life-threatening or life-altering" in the executive summary, and "P(death) ≥ 0.10" in section 8, quote the series' own documents.
- **Page numbers** for Bayona et al. (2023) are the journal's printed pages. They were not separately checked.
