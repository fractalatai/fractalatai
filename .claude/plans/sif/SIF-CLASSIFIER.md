# SIF Classifier: Two-Stage Energy-Based Serious Injury & Fatality Potential Classification

**Status**: Design v0.3
**Date**: 2026-08-19
**Scope**: Two products — (1) SLM classifier for SIF potential from narrative text, (2) SIPmath simulator for pre-task JHA — sharing a metalog/SIPmath engine
**Reviewed by**: Gemini 2.5 Pro (2026-08-19) — v0.1 review: `data/code-review/sif-classifier-design-review.md`, v0.2 review: `data/code-review/sif-classifier-v02-design-review.md`

---

## The Problem

Traditional safety metrics (TRIR, lost-time injury rate) count outcomes. They fail to distinguish between a twisted ankle and a near-miss that was one second from a fatality. The empirical evidence (Campbell Institute, Hallowell, DEKRA) shows that reducing minor injuries does not reduce SIF rates — SIF events have *different precursors and root causes* from the bulk of workplace incidents.

The SIF potential question is: **given the energy present in this event, could it have killed or permanently injured someone — regardless of actual outcome?** A near-miss where a scaffold board fell 20m but hit nobody has the same SIF potential as a fatality from a scaffold board falling 20m onto someone. The energy was identical; only the path differed.

Current practice: safety teams manually review every incident report and subjectively judge SIF potential. Agreement rates are ~65% without structured frameworks (Hallowell & Spencer, 2024). This is slow, inconsistent, and scales poorly.

**The classifier's job**: take a free-text incident/near-miss narrative (plus any structured fields) and produce a SIF-potential classification with structured justification (mechanism, energy type, magnitude, amplification factors).

---

## Why Two Classification Schemes

A single-pass approach conflates two distinct questions:

1. **What happened?** — the mechanism of injury, the objects involved, the physical process. This is a *classification* problem with an established taxonomy (ICECI / ICD-11 Chapter 23).

2. **How much energy was involved?** — the physics question. Same mechanism (fall) can range from trivial (step off a kerb) to fatal (fall from roof). This requires *estimation* of energy magnitude and intensity, which the Energy-Based Safety (EBS) framework provides.

Separating them gives us:
- **Stage 1 (ICECI)**: coarse filter — identifies *what kind* of energy transfer occurred. Some mechanisms are almost always SIF-potential (electrical contact, confined space asphyxiation). Others require Stage 2 to discriminate (struck-by, fall).
- **Stage 2 (EBS)**: fine discriminator — estimates energy magnitude and applies amplification factors to determine SIF potential. Explicitly **no consideration of mitigation/barriers** because SIF potential asks "what if the barriers failed?" — and in a near-miss, they nearly did.

This mirrors the SCL model's decision tree (EEI/Hallowell): first ask "was high energy present?", then classify based on that answer.

---

## Chance P vs Outcome P (v0.2)

Every incident involves two distinct probabilities:

- **Chance P** — the probability the energy reaches a person at all. Did the dropped object hit someone? Did the worker actually fall? Did the chemical splash make contact? This is the left side of a bowtie — threats through barriers to the top event.

- **Outcome P** — given the event DID happen (chance P = 1), what is the distribution of severity outcomes? This is the right side of the bowtie — top event through consequences.

**SIF potential defines chance P = 1.** The near-miss didn't actually strike anyone — but SIF analysis asks "IF it had, what would the outcome distribution look like?" The point of SIF is precisely to look past the actual draw (a miss) and examine the outcome distribution that was in play.

This distinction has cascading consequences:

### For the classifier (Product 1)
- The classifier models **outcome P only**. It asks: "given this energy was transferred to a person, what is P(severity ≥ SIF)?"
- Chance P factors are irrelevant. Whether a dropped tool was likely to hit someone (busy area vs empty) doesn't change the severity IF it hits.
- This resolves the calibration bias concern from Gemini review: OSHA data records events that DID happen to people (chance P already = 1 in the data). This IS the right data for calibrating outcome curves.

### For the mitigation boundary
Mitigations split cleanly into two types:

| Type | Controls | Examples | Relevant to SIF potential? |
|------|----------|----------|---------------------------|
| **Chance P controls** | Prevent the event from happening | Guardrails (prevent fall), barriers (block entry), LOTO (prevent energisation), housekeeping (prevent slip) | **No** — SIF potential defines event P=1 |
| **Outcome P controls** | Reduce severity given the event happens | Safety nets (catch after fall), hard hats (reduce head strike severity), arc flash PPE (reduce burn severity) | **No** — SIF potential assesses unmitigated outcome |

The classifier excludes BOTH types. The simulator (Product 2) models outcome P controls — "given the fall happens, how much does a net reduce P(SIF)?"

### For the simulator (Product 2)
- The simulator models **outcome P mitigations** (right side of bowtie). Each mitigation has an effectiveness distribution reducing the severity of the outcome.
- Chance P mitigations (left side of bowtie) are a different question ("how likely is the event?") and could be a future extension, but are not in scope for v0.2.
- Oil on the floor is a chance P factor — it changes the probability of slipping, not the severity of the fall. It has nothing to do with SIF outcome.

### For the severity scale

**v0.2 change (Gemini v0.2 review):** The naive ordinal mapping (first_aid=1, medical=2, serious=3, fatal=4) assumes equal spacing between categories. The distance between first aid and medical treatment is not the same as between serious injury and fatality.

Established injury severity scales address this:
- **AIS (Abbreviated Injury Scale)**: 6-point scale (1=minor, 2=moderate, 3=serious, 4=severe, 5=critical, 6=unsurvivable). Non-linear spacing — each level represents roughly a doubling of threat to life.
- **ISS (Injury Severity Score)**: Composite score from AIS across body regions. 0–75 scale.
- **Insurance / actuarial cost scales**: Convert severity to £/$ — natural continuous scale. Published conversion tables exist for workers' compensation, tort liability, and health economics (QALY-based).

The metalog should be fitted to a **continuous severity measure** (AIS-weighted or cost-based), not to naive ordinal integers. This makes the P(SIF) threshold meaningful rather than arbitrary.

### Missing data and Bayesian priors

When the SLM cannot extract a quantitative magnitude from the narrative ("worker fell from ladder" — no height stated), the system uses a **Bayesian prior** — the distribution of ladder fall heights from OSHA data. A small clue ("top of extension ladder") narrows the prior substantially. Full specification ("6m scaffold") collapses it to a near-point estimate.

This provides a natural feedback loop: the system can report "P(SIF) = 0.45, but height was estimated from prior — specify actual height for more precise classification." This improves data quality over time as reporters learn what information matters.

---

## Classification Scheme 1: ICECI / ICD-11 Mechanism Taxonomy

### Background

The International Classification of External Causes of Injury (ICECI v1.2, 2004) was developed by WHO collaborating centres. It uses a multi-axial system where each injury event is coded across independent axes. ICECI is no longer maintained — its concepts have been absorbed into **ICD-11 Chapter 23** (External Causes of Morbidity or Mortality) and the ICD-11 Extension Codes for dimensions of external causes.

### Axes Used by the Classifier

We use two ICECI axes (three if occupation context is available):

**Axis 1: Mechanism of Injury (C2)** — the physical process that caused or could have caused harm.

| Code | Level 1 Mechanism | SIF Gate | Rationale |
|------|-------------------|----------|-----------|
| 1.1 | Transport injury event | → Stage 2 | Energy varies hugely (parking lot vs motorway) |
| 1.2 | Contact with object/animal | → Stage 2 | Depends on object mass, velocity |
| 1.3 | Contact with person | → Stage 2 | Usually low energy, but crowd crush exists |
| 1.4 | Crushing (caught-in/between) | → Stage 2 | Spans finger-in-drawer to industrial press — energy discriminates |
| 1.5 | Fall | → Stage 2 | Critical: height discriminates |
| 1.6 | Abrading/rubbing | **Auto-non-SIF** | Low energy by definition |
| 2.1 | Cutting/severing | → Stage 2 | Amplification-dependent (sharp = high intensity) |
| 2.2 | Puncturing/stabbing | → Stage 2 | Depends on depth, body region |
| 3.1 | Explosive blast | → Stage 2 | Usually high energy, but firecrackers exist |
| 3.2 | Contact with machinery | → Stage 2 | Spans scrape on de-energised housing to rotating shaft entanglement |
| 4.1 | Burns/scalds (heating) | → Stage 2 | Temperature, duration, area |
| 4.2 | Cooling/hypothermia | → Stage 2 | Duration-dependent |
| 5.1 | Mechanical threat to breathing | → Stage 2 | Usually high severity, but brief partial obstruction exists |
| 5.2 | Drowning/near-drowning | → Stage 2 | Usually high severity, but shallow-water slip exists |
| 5.3 | Oxygen-deficient confinement | → Stage 2 | Confined space atmosphere varies |
| 6.1 | Poisoning | → Stage 2 | Dose/concentration dependent |
| 6.2 | Corrosion by chemical | → Stage 2 | Concentration, body area |
| 7.1 | Acute over-exertion | **Auto-non-SIF** | High prevalence, almost never SIF (rare cardiac exception) |
| 98.2 | Electricity/radiation | → Stage 2 | Voltage/dose discriminates — static shock vs mains vs HV |

**v0.2 change (Gemini review):** Auto-SIF gates removed. Mechanism alone is almost never sufficient to determine SIF potential — a "caught-in" event spans finger-in-drawer to industrial press. The mechanism informs the energy analysis but does not bypass it. **Auto-non-SIF** gates are retained for mechanisms that are inherently low-energy (over-exertion, abrasion) and are consistently high-prevalence in accident data — keeping these out of Stage 2 reduces noise.

**Axis 2: Object/Substance (C3)** — proxy for energy source and magnitude.

The C3 code identifies what was involved. This directly informs the Stage 2 energy estimate:
- Land vehicle → kinetic energy (mass × velocity²/2)
- Ladder/scaffold → gravitational potential energy (mass × g × height)
- Electrical equipment → electrical energy (voltage × current × time)
- Chemical substance → chemical energy (concentration, volume, toxicity)
- Machinery → mechanical energy (rotational speed, torque)

**Axis 3: Activity (C5)** — context filter (optional).

If available, filters for occupational context (paid work vs leisure). Not required for the core classification but useful for routing and reporting.

### ICD-11 Mapping

ICECI C2 maps cleanly to ICD-11 Chapter 23 stem codes:

| ICECI C2 | ICD-11 Block | Codes |
|----------|-------------|-------|
| 1.1 Transport | Transport injury event | PA00–PA5Z |
| 1.5 Fall | Unintentional fall | PA60–PA6Z |
| 1.2–1.4 Contact/crush | Contact with object/person | PA70–PA8Z |
| 5.2 Drowning | Immersion/submersion | PA90–PA9Z |
| 5.1/5.3 Breathing | Threat to breathing | PB00–PB0Z |
| 4.x Thermal | Thermal mechanism | PB10–PB1Z |
| 6.x Chemical | Exposure to substances | PB20–PB36 |
| 3.x/98.x Other | Other mechanisms | PB50–PB5B |

The ICD-11 taxonomy is API-accessible, actively maintained, and available in structured JSON — making it the better source for label definitions.

### Data Source for ICECI Labels

- **ICD-11 REST API** — bulk download of Chapter 23 + Extension Codes (structured, machine-readable)
- **ICECI v1.2 PDF** — full taxonomy document from WHO FIC Netherlands
- **EU Injury Database (IDB) FDS Data Dictionary** — operationalised ICECI codes for data collection
- **BioPortal ICECI ontology** — formal OWL representation (REST API)

---

## Classification Scheme 2: Energy-Based Safety (EBS)

### Background

Energy-Based Safety, developed from Haddon's energy transfer theory (1973) and operationalised by Hallowell (2024), models injury as **unwanted energy transfer from source to target (person)**. The Energy Wheel organises workplace hazards into 10 energy types.

### The Energy Wheel — 10 Categories

| # | Energy Type | Typical Sources | SIF Threshold |
|---|-------------|-----------------|---------------|
| 1 | **Gravity** | Falls from height, falling objects, collapse | ≥ 1.8m fall height (~500 ft-lb for avg worker) |
| 2 | **Motion** (kinetic) | Vehicles, mobile plant, projectiles, recoiling cables | ≥ 500 ft-lb (~680J); vehicles ≥ 10 mph near pedestrians |
| 3 | **Mechanical** | Rotating equipment, presses, conveyors, shearing | Any unguarded rotating/reciprocating machinery |
| 4 | **Electrical** | Power lines, switchgear, arc flash, generators | ≥ 50V |
| 5 | **Pressure** | Hydraulic/pneumatic systems, compressed gas, steam, excavations | ≥ 30 psi; any trench/excavation |
| 6 | **Thermal** | Hot surfaces, molten material, fire, welding, cryogenic | Sustained fuel + ignition source; steam; molten material |
| 7 | **Chemical** | Toxic gas, corrosives, oxygen-displacing agents, reactive substances | Exceeds workplace exposure limits; any IDLH atmosphere |
| 8 | **Radiation** | Ionising (X-ray, gamma), non-ionising (UV, laser, RF) | Any ionising source; Class 3B+ laser |
| 9 | **Sound** (acoustic) | Explosions, pneumatic tools, heavy machinery | Blast overpressure |
| 10 | **Biological** | Pathogens, venomous animals, blood-borne agents | Rarely SIF alone; exception: anaphylaxis, sepsis |

### Energy Magnitude and Severity Distributions

#### Reference Thresholds (Hallowell et al., 2017)

| Band | Joules | Foot-pounds | Most Likely Outcome |
|------|--------|-------------|---------------------|
| Low | < 500 J | < 370 ft-lb | First aid |
| Medium | 500–1,500 J | 370–1,100 ft-lb | Medical treatment |
| High | > 1,500 J | > 1,100 ft-lb | Serious injury or fatality |

The EEI/SCL model uses a more conservative threshold of **500 ft-lb (~680 J)**. These thresholds inform but do not define the model output.

#### From Bands to Probability Curves (v0.2)

**v0.2 change:** Discrete magnitude bands (LOW/MEDIUM/HIGH) replaced by **probability distributions over severity outcomes**, following SIPmath / metalog distribution principles (Sam Savage).

Every incident is a sample from a distribution of possible outcomes. Strike 100 people with the same car holding all variables steady and they'll have a *distribution* of outcomes — some walk away, some are hospitalised, some die. The variables that shift the distribution are the source energy, carrier properties, receiving environment, and body vulnerability. The controlled outcome (what actually happened) is one draw.

The classifier's job is not to output a band — it's to **characterise the severity distribution** and report P(SIF):

```
P(SIF) = P(severity ≥ SIF_threshold | energy, carrier, environment, vulnerability)
```

**Model output** for Stage 2 becomes severity quantile estimates:

| Quantile | Meaning | Example (6m fall onto concrete) | Example (1m fall onto grass) |
|----------|---------|--------------------------------|------------------------------|
| P10 | 10th percentile severity | Medical treatment | No injury |
| P50 | Median severity | Serious injury | First aid |
| P90 | 90th percentile severity | Fatality | Medical treatment |

From 3+ quantiles, a **metalog distribution** is fitted (closed-form, no simulation needed). P(SIF) is then read directly from the CDF. This replaces the brittle `MEDIUM AND amplification → SIF` logic with a continuous probability that naturally incorporates the interaction between all factors.

**SIF potential classification** is then a threshold on P(SIF):

| P(SIF) | Classification | Action |
|--------|---------------|--------|
| ≥ 0.50 | **SIF** | Flag for SIF investigation |
| 0.10–0.50 | **Elevated** | Review, may warrant investigation |
| < 0.10 | **Non-SIF** | Standard incident processing |

The threshold is a policy decision (customer-configurable), not a model parameter. Conservative customers lower it; others raise it.

### Amplification, Vulnerability, and the Mitigation Boundary

Hallowell's key finding: **energy intensity** (energy / contact area, J/cm²) predicts injury severity better than raw magnitude. A sharp object concentrating the same total energy into a smaller area increases intensity by 20×+.

But considering what someone lands *on* or is struck *by* opens a boundary problem: amplification of the energy transfer allows control/mitigation to leak into the assessment. Falling onto rebar (amplification — the rebar concentrates force) vs falling onto a net (mitigation — the net absorbs energy) are both properties of the receiving environment. We need clear rules.

**The boundary: source/carrier/receiver vs engineered controls**

| Category | What It Covers | Include? | Rationale |
|----------|---------------|----------|-----------|
| **Source properties** | Mass, height, speed, voltage, concentration, temperature | **Yes** | Define the energy available at the source |
| **Carrier properties** | Sharp/pointed object, rotating shaft, pressurised fluid | **Yes** | Properties of whatever carries the energy — a chisel concentrates force, this is the carrier's geometry, not a control |
| **Receiver vulnerability** | Body part struck (head/neck/spine vs forearm), age/fitness | **Yes** | Inherent vulnerability of the target — nobody chose to install a skull |
| **Default environment** | Concrete floor, rebar, water depth, ambient temperature | **Yes** | What is *there* — the uncontrolled receiving surface. This is context, not mitigation |
| **Engineered controls** | Safety nets, barriers, PPE, LOTO, interlocks, guardrails, ventilation | **No** | Deliberate mitigations someone *put there* for safety. SIF potential asks: what if these failed or weren't present? |

The test: *"Was this put there to prevent harm, or is it just what's there?"* Concrete is the default surface — it's context. A safety net is an engineered control — it's mitigation. Rebar sticking up is the ambient hazard — it's context (and amplification). A guardrail is an engineered control — it's mitigation.

**Factors the classifier must detect:**

| Factor | Category | Effect | Example |
|--------|----------|--------|---------|
| **Sharp/pointed contact** | Carrier | Concentrates force → high intensity | Chisel: 24.5 J/cm² vs tape measure: 1.14 J/cm² |
| **Body vulnerability** | Receiver | Head/neck/spine more severe at same energy | Falling object striking head vs forearm |
| **Speed of release** | Source | Sudden release → less time to absorb | Pressurised line burst vs slow leak |
| **Mass** | Source | Heavier objects → more gravitational/kinetic energy | Scaffold tube vs hand tool |
| **Height** | Source | Direct multiplier for gravitational PE (mgh) | Each metre adds ~750 J for 75 kg person |
| **Velocity** | Source | Quadratic multiplier for kinetic energy (½mv²) | Vehicle speed doubles → 4× energy |
| **Concentration** (chemical) | Source | Higher concentration → more tissue destruction | Concentrated acid vs dilute solution |
| **Duration** (thermal/chemical) | Source | Longer exposure → more energy transferred | Flash contact vs sustained immersion |
| **Receiving surface** | Environment | Hard/uneven surface increases energy transfer to body | Concrete, rebar, machinery edges |

### What We Explicitly Exclude: Engineered Controls

The classifier assesses **potential SIF** — the energy that *was present* and the *uncontrolled* receiving environment, not whether engineered barriers prevented harm. This is deliberate:

- In a near-miss, controls either failed, nearly failed, or weren't present
- Controls are not 100% reliable — their presence doesn't eliminate SIF potential
- The question is: "given the energy source, the carrier, the default environment, and human vulnerability — what is the probability distribution of possible outcomes?"
- The controlled outcome (what actually happened) is one sample from this distribution — "nearly got struck by a car but was able to take evasive action" is a single draw from a range of possible outcomes. We're trying to characterise the distribution, not the draw.

---

## Pipeline Architecture

### Overview

```
                        ┌─────────────────────────────────────────────┐
                        │           Incident Narrative Text           │
                        │  + optional structured fields (if avail.)   │
                        └─────────────────────┬───────────────────────┘
                                              │
                                              ▼
                        ┌─────────────────────────────────────────────┐
                        │         STAGE 1: ICECI Mechanism            │
                        │                                             │
                        │  Multi-label classification:                │
                        │  • C2 mechanism code (L1 + L2)              │
                        │  • C3 object/substance (L1)                 │
                        │  • Confidence score per label               │
                        └─────────────────────┬───────────────────────┘
                                              │
                              ┌───────────────┴───────────────┐
                              │                               │
                              ▼                               ▼
                         → Stage 2                    Auto-non-SIF
                      (most mechanisms)            (over-exertion,
                                                    abrasion — high
                                                    prevalence, near-
                                                    zero SIF potential)
                              │                               │
                              ▼                               │
                 ┌────────────────────────┐                   │
                 │  STAGE 2: EBS Energy   │                   │
                 │                        │                   │
                 │  Structured extraction:│                   │
                 │  • Energy type(s)      │                   │
                 │  • Source properties    │                   │
                 │  • Carrier properties  │                   │
                 │  • Environment context │                   │
                 │  • Body vulnerability  │                   │
                 │  • Severity quantiles  │                   │
                 │    (P10, P50, P90)     │                   │
                 │  • P(SIF) from metalog │                   │
                 └────────────┬───────────┘                   │
                              │                               │
                              ▼                               ▼
                        ┌─────────────────────────────────────────────┐
                        │              OUTPUT RECORD                  │
                        │                                             │
                        │  mechanism_code: "1.5" (fall)               │
                        │  energy_types: ["gravity"]                  │
                        │  source_cues: ["6 metre scaffold"]          │
                        │  environment_cues: ["concrete floor"]       │
                        │  vulnerability: ["head exposure"]           │
                        │  severity_p10: "medical_treatment"          │
                        │  severity_p50: "serious_injury"             │
                        │  severity_p90: "fatality"                   │
                        │  p_sif: 0.72                                │
                        │  classification: "SIF" (P≥0.50)            │
                        │  reasoning: "Fall from scaffold (est. 6m)   │
                        │    onto concrete. Gravitational PE ≈ 4,400J.│
                        │    Hard surface, head exposure. Distribution │
                        │    of outcomes centres on serious injury     │
                        │    with substantial fatality tail."          │
                        └─────────────────────────────────────────────┘
```

### Stage 1: ICECI Mechanism Classifier

**Task type**: Multi-label text classification.

**Input**: Free-text narrative (typically 1–5 sentences). Optionally structured fields (location, equipment, activity).

**Output per event**:
- `mechanism_codes`: list of ICECI C2 codes at Level 1 and Level 2 (an event can involve multiple mechanisms — e.g., explosion + thermal)
- `object_substance`: ICECI C3 Level 1 code — what was involved
- `sif_gate`: AUTO_SIF | NEEDS_ENERGY_ASSESSMENT | NON_SIF — based on the mechanism's inherent SIF potential
- `confidence`: per-label confidence score

**Label set**: ~25 labels at C2 Level 2 granularity (collapsing the full ~40 ICECI L2 codes to the subset relevant to occupational settings). Plus ~15 C3 Level 1 object categories.

**Approach**: Fine-tuned SLM with a classification head. This is a well-understood task — the Bejaoui et al. (2024) paper achieved good results with BERT + XGBoost on a similar problem. For edge deployment, we fine-tune a small model (Qwen 3 0.6B or 1.7B) with a classification head rather than using generative output.

### Stage 2: EBS Energy Analyser

**Task type**: Structured extraction with estimation.

**Input**: Original narrative + Stage 1 output (mechanism code, object/substance).

**Output per energy source identified**:
- `energy_type`: one of the 10 Energy Wheel categories
- `source_cues`: extracted text spans for source properties (e.g., "6 metre scaffold", "240V supply")
- `carrier_cues`: extracted text spans for carrier properties (e.g., "sharp edge", "rotating shaft")
- `environment_cues`: extracted text spans for default environment (e.g., "concrete floor", "rebar")
- `body_vulnerability`: body region if mentioned (head/neck/spine = high vulnerability)
- `severity_quantiles`: estimated severity at P10, P50, P90 (ordinal: no_injury / first_aid / medical_treatment / serious_injury / fatality)
- `reasoning`: model's reasoning chain explaining the severity distribution

**Approach**: This is closer to structured extraction than pure classification. The model needs to:
1. Identify energy sources from the narrative
2. Extract magnitude indicators (heights, speeds, voltages, temperatures, concentrations)
3. Detect amplification cues (sharp, pointed, heavy, fast, concentrated, head, neck)
4. Map to energy bands using the threshold table

Two viable architectures:
- **Option A: SLM structured output** — fine-tune a generative SLM (Qwen 3 4B–8B) to produce JSON-structured output with energy analysis. More flexible, handles novel situations, provides reasoning.
- **Option B: Classification + extraction pipeline** — NER for magnitude cues, then rule-based mapping to energy bands. Simpler, faster, but brittle for edge cases.

**Recommendation: Option A** (generative structured output). The energy estimation task requires reasoning about physical quantities from natural language — "fell from the second floor" needs to be mapped to ~6m → ~4,400 J → HIGH. This is closer to the cultural graph extraction pattern (structured output from narrative text) than to the DRRP classification pattern (label assignment).

### Combined Output

**For auto-non-SIF mechanisms** (over-exertion, abrasion): P(SIF) = 0.0, classification = NON_SIF. No Stage 2 needed.

**For all other mechanisms**: Stage 2 outputs severity quantiles (P10, P50, P90) as ordinal categories. These are mapped to a **continuous severity scale** (AIS-weighted or insurance cost-based — see "Chance P vs Outcome P" section) and a **metalog distribution** is fitted:

```
Severity scale (AIS-weighted example):
  no_injury=0, first_aid=0.5, medical_treatment=1.5, serious_injury=3.0, fatality=6.0
SIF threshold: severity ≥ 3.0 (AIS "serious" or above)

From 3 quantiles → fit metalog CDF → P(SIF) = 1 - CDF(3.0)
```

The specific scale mapping (AIS vs insurance cost vs custom) is a design decision to validate empirically. The key requirement is non-equal spacing that reflects the actual distance between severity levels.

**Classification** is a policy threshold on P(SIF):

| P(SIF) | Classification | Action |
|--------|---------------|--------|
| ≥ 0.50 | **SIF** | Flag for SIF investigation |
| 0.10–0.50 | **Elevated** | Review, may warrant investigation |
| < 0.10 | **Non-SIF** | Standard incident processing |

The threshold is customer-configurable. The model outputs the probability; the policy decides the response. This separates the scientific question (what is the severity distribution?) from the operational question (what warrants investigation?).

---

## The 13 STKY Hazards as Validation Anchors

Hallowell's "Stuff That Kills You" (STKY) list provides 13 empirically-derived high-energy hazard categories that account for ~75% of SIF events. These serve as validation anchors — any classifier that fails to flag these as SIF is broken:

| # | STKY Hazard | ICECI Mechanism | Energy Type | Known Threshold |
|---|------------|-----------------|-------------|-----------------|
| 1 | Fall from elevation | 1.5 | Gravity | ≥ 1.8m (6 ft) |
| 2 | Suspended load | 1.2/1.4 | Gravity + Mechanical | Any mechanically lifted load |
| 3 | Mobile equipment (worker on foot) | 1.1/1.2 | Motion | Any speed, worker in proximity |
| 4 | Motor vehicle (occupant) | 1.1 | Motion | ≥ 50 km/h (30 mph) |
| 5 | Heavy rotating equipment | 3.2 | Mechanical | Unguarded rotating machinery |
| 6 | Electrical contact | 98.2 | Electrical | ≥ 50V |
| 7 | Arc flash | 98.2 | Electrical + Thermal | Energised equipment |
| 8 | High temperature | 4.1 | Thermal | Steam, molten material |
| 9 | Fire with sustained fuel | 4.1 | Thermal + Chemical | Fuel + ignition source |
| 10 | Explosion | 3.1 | Pressure + Thermal | Explosive atmosphere |
| 11 | Steam | 4.1 | Thermal + Pressure | Pressurised steam systems |
| 12 | Excavation/trenching | 1.4/5.3 | Gravity + Pressure | Any trench/excavation |
| 13 | Toxic chemical/radiation | 6.1/98.2 | Chemical/Radiation | ≥ workplace exposure limits |

---

## Model Architecture

### Edge Deployment Profile

Target device: 32 GB corporate laptop, Intel Core Ultra 5 135H (from memory: edge device spec). Consistent with fractalaw's edge-AI pattern.

### Stage 1 Model: Mechanism Classifier

| Aspect | Choice | Rationale |
|--------|--------|-----------|
| Base model | Qwen 3 0.6B | Classification task; small model sufficient. Matches DRRP classifier pattern |
| Architecture | Fine-tuned with classification head | Multi-label, not generative |
| Quantisation | Q8_0 GGUF | Classification head needs precision |
| Inference | ONNX Runtime or Ollama | ONNX for batch; Ollama for interactive |
| Latency target | < 100ms per event | Real-time screening |
| Label count | ~25 mechanism + ~15 object = ~40 labels | Two-head: mechanism head + object head |

### Stage 2 Model: Energy Analyser

| Aspect | Choice | Rationale |
|--------|--------|-----------|
| Base model | Qwen 3 4B (or 8B if quality demands) | Structured extraction needs reasoning |
| Architecture | Fine-tuned for structured JSON output | Generative, schema-constrained |
| Quantisation | Q4_K_M GGUF | 4B fits comfortably; 8B needs Q4 |
| Inference | Ollama with JSON mode | Structured output, temperature 0 |
| Latency target | < 2s per event | Batch-friendly, not real-time critical |
| Output schema | JSON with energy_type, magnitude, amplification, reasoning | Structured extraction |

### Alternative: Single-Model Approach

A single 8B model could do both stages in one pass. Trade-offs:

| | Two-Stage | Single-Model |
|--|-----------|-------------|
| Latency | Stage 1 fast (100ms), Stage 2 only when needed | Always full inference (~2s) |
| Accuracy | Each stage specialised for its task | One model covering both tasks |
| Interpretability | Clear mechanism → energy → SIF chain | Black box |
| Training data | Separate datasets per stage | Needs combined labels |
| Skip rate | ~30% events skip Stage 2 (auto-SIF or non-SIF) | No skip benefit |

**Recommendation**: Start with two-stage. If the Stage 1 classifier proves insufficiently accurate, fall back to single-model.

---

## Training Data Strategy

### Source 1: OSHA Severe Injury Reports (Public)

OSHA publishes severe injury reports (hospitalisations, amputations, fatalities) with free-text narratives and structured fields. Available via OSHA ITA (Injury Tracking Application) and the Severe Injury Reports dashboard.

- **Volume**: Tens of thousands of events per year
- **Quality**: Narrative + OIICS codes (nature, body part, event, source)
- **Labels**: All are actual serious injuries → positive examples for SIF
- **Limitation**: Only actual injuries, not near-misses. No non-SIF examples.

### Source 2: OSHA Form 300 Data (Public)

OSHA establishment-specific data includes all recordable injuries, not just severe ones. The lower-severity events provide non-SIF examples.

- **Volume**: Hundreds of thousands of records
- **Quality**: Less narrative detail; more structured fields
- **Labels**: Mix of SIF and non-SIF outcomes

### Source 3: Synthetic Generation from ICD-11 Taxonomy

Use a large LLM (Gemini Pro) to generate synthetic incident narratives for each ICD-11 Chapter 23 mechanism × energy magnitude combination. Primary purpose: **balance the less common ICD-11 classes** that are underrepresented in OSHA public data (e.g., pressure release, radiation, biological, oxygen-deficient confinement). Also addresses:
- Coverage (ensure every mechanism code has training examples)
- Edge cases (generate scenarios where amplification converts medium-energy to SIF)

Pipeline:
1. Profile ICD-11 class frequencies in OSHA data → identify underrepresented classes
2. For each underrepresented ICD-11 mechanism × energy band (LOW/MEDIUM/HIGH):
   - Generate 50–100 realistic incident narratives
   - Vary: industry, equipment, location, narrative style, length
   - For MEDIUM band: generate with and without amplification factors
3. For well-represented classes: generate only edge cases and amplification variants
4. Human review of a 10% sample for realism and label correctness
5. Augment with paraphrasing for linguistic diversity

### Source 4: UK HSE Data

UK Health and Safety Executive publishes RIDDOR (Reporting of Injuries, Diseases and Dangerous Occurrences) statistics and some investigation reports. Less accessible as structured data but valuable for UK-context narratives.

### Source 5: QQ SIFp Classifications (Benchmark Only)

QQ have human SIFp classifications on their incident records. These provide a real-world benchmark for testing end-to-end classifier accuracy against human judgement.

- **Use for**: Benchmark evaluation, not training
- **Why not training**: Quality of human SIFp labels is variable — inter-rater agreement is ~65% without structured frameworks (Hallowell & Spencer, 2024). Training on inconsistent labels would teach the model to replicate the inconsistency.
- **Benchmark value**: Measures whether the classifier agrees with human judgement at the same rate humans agree with each other. If classifier–human agreement ≥ human–human agreement (~65%), the classifier is performing at parity. Target: significantly exceed this baseline by applying consistent ICECI + EBS logic.

### Source 6: Customer Data (Future)

Once deployed, customer incident data (with consent) becomes the highest-value training source — it matches the actual input distribution. Per-customer fine-tuning adapts the model to:
- Customer-specific vocabulary and narrative style
- Industry-specific equipment and hazard profiles
- Site-specific terminology and abbreviations

### Label Generation for Stage 2

Stage 2 labels (energy type, magnitude, amplification) need structured annotation. Strategy:

1. **Rule-based pre-annotation**: Extract numeric cues (heights in metres, voltages, speeds) and map to magnitude bands automatically
2. **LLM-assisted annotation**: Use Gemini to annotate the energy analysis fields on real OSHA narratives
3. **Human validation**: Safety professional reviews a sample (same human-in-the-loop pattern as DRRP validation)

---

## Output Schema (Arrow RecordBatch)

Consistent with fractalaw's Arrow-everywhere convention:

```
sif_event {
    event_id:               utf8        -- unique identifier
    narrative:              utf8        -- original text
    
    // Stage 1 outputs
    mechanism_codes:        list<utf8>  -- ICECI C2 codes ["1.5", "1.2"]
    mechanism_labels:       list<utf8>  -- human-readable ["Fall", "Struck by object"]
    object_codes:           list<utf8>  -- ICECI C3 codes
    object_labels:          list<utf8>  -- human-readable
    sif_gate:               utf8        -- NEEDS_ASSESSMENT | AUTO_NON_SIF
    stage1_confidence:      float32     -- classifier confidence
    
    // Stage 2 outputs (null if sif_gate = AUTO_NON_SIF)
    energy_types:           list<utf8>  -- ["gravity", "motion"]
    source_cues:            list<utf8>  -- ["6 metre scaffold", "75 kg beam"]
    carrier_cues:           list<utf8>  -- ["sharp edge", "rotating shaft"]
    environment_cues:       list<utf8>  -- ["concrete floor", "rebar"]
    body_vulnerability:     utf8        -- body region: "head", "neck", "spine", "torso", "limb", null
    severity_p10:           utf8        -- 10th percentile: no_injury|first_aid|medical|serious|fatal
    severity_p50:           utf8        -- median severity
    severity_p90:           utf8        -- 90th percentile severity
    energy_reasoning:       utf8        -- model's reasoning chain
    stage2_confidence:      float32     -- extraction confidence
    
    // Final output (deterministic from quantiles + metalog fit)
    p_sif:                  float32     -- P(severity ≥ serious_injury) from metalog CDF
    sif_classification:     utf8        -- SIF | ELEVATED | NON_SIF (policy threshold)
    review_flag:            bool        -- true if low confidence or ELEVATED
}
```

---

## SIPmath Engine

### Overview

The metalog distribution and SIPmath standard provide the mathematical foundation for both the SIF classifier output and a standalone simulator. The engine is small (~500 lines of Rust), has no external dependencies beyond basic linear algebra, and compiles to WASM for browser/edge deployment.

**No Rust or JavaScript implementation currently exists** — crates.io and npm have no metalog or SIPmath packages. Reference implementations exist only in R (`rmetalog`) and Python (`metalog_jax`). This is a publishable crate.

### Metalog Distribution

The metalog (Keelin, 2016) is a distribution defined by its **quantile function** (inverse CDF). Unlike traditional distributions where you pick a shape (normal, lognormal, beta) and fit parameters, the metalog directly interpolates quantile points you provide.

**SPT (Symmetric Percentile Triplet)**: 3 quantile points (P10, P50, P90) → 3-term metalog. Closed-form coefficient formulas — no OLS, no iteration:

```
Given: x10 (10th percentile), x50 (median), x90 (90th percentile)
       y10 = 0.10, y50 = 0.50, y90 = 0.90

Logit:  L(y) = ln(y / (1-y))
        L(0.10) = -2.197,  L(0.50) = 0,  L(0.90) = 2.197

Coefficients:
  a1 = x50
  a2 = (x90 - x10) / (L(0.90) - L(0.10))    = (x90 - x10) / 4.394
  a3 = (x90 + x10 - 2*x50) / (L(0.90) - L(0.10))  [skewness term]
```

**Quantile function** (generates samples):
```
M(y) = a1 + a2 * ln(y/(1-y)) + a3 * (y - 0.5) * ln(y/(1-y))
```

This is the entire computation. To generate 10,000 Monte Carlo trials: feed 10,000 uniform random numbers through this formula.

**Bounded variants** for different domains:

| Domain | Boundedness | Transform | Use Case |
|--------|-----------|-----------|----------|
| (-∞, +∞) | Unbounded | z = x | Severity score (ordinal) |
| [0, +∞) | Semi-bounded lower | z = ln(x) | Energy in joules, severity ≥ 0 |
| [0, 1] | Bounded | z = ln(x/(1-x)) | Mitigation effectiveness |

### HDR Random Number Generator

The HDR (Hubbard Decision Research) PRNG uses a **5-component seed** to produce reproducible, independent uniform random streams:

```
hdr_uniform(counter, entity, varId, seed3, seed4) → u ∈ (0, 1)
```

- `counter` (PM_Index): trial number (1, 2, ..., 10000)
- `entity`: organisational unit (prevents accidental correlation across customers)
- `varId`: unique per variable in the model (severity vs effectiveness get different streams)
- `seed3, seed4`: optional additional seeds

Two variables with different `varId` produce independent streams. Two variables sharing the same `varId` and `entity` produce identical streams — use a **Gaussian copula** layer to introduce desired correlations.

### SIP Composition — Distributions That Work Like Numbers

A SIP (Stochastic Information Packet) is an array of Monte Carlo trials. Two SIPs with the same dimension compose via **element-wise arithmetic**:

```rust
// Generate severity SIP (semi-bounded lower [0, ∞))
let severity: Vec<f64> = (0..N_TRIALS)
    .map(|i| metalog_quantile(hdr_uniform(i, entity, VAR_SEVERITY), &severity_coeffs))
    .collect();

// Generate mitigation effectiveness SIP (bounded [0, 1])
let net_eff: Vec<f64> = (0..N_TRIALS)
    .map(|i| metalog_quantile(hdr_uniform(i, entity, VAR_NET), &net_coeffs))
    .collect();

// Compose: residual severity = severity * (1 - effectiveness)
let residual: Vec<f64> = severity.iter().zip(net_eff.iter())
    .map(|(s, e)| s * (1.0 - e))
    .collect();

// P(SIF) = fraction of trials where residual severity ≥ SIF_threshold
let p_sif = residual.iter().filter(|&&r| r >= SIF_THRESHOLD).count() as f64 / N_TRIALS as f64;
```

No convolution integrals. No simulation engine. The result array **is** the output distribution.

**Multiple mitigations chain multiplicatively** (quantitative Swiss cheese model):

```
residual[i] = severity[i] * (1 - eff_net[i]) * (1 - eff_harness[i]) * (1 - eff_hardhat[i])
```

Each mitigation has its own effectiveness distribution — nets don't always catch, harnesses aren't always clipped in, hard hats aren't always worn correctly. The multiplication naturally handles partial effectiveness and independence.

**Dynamic risk allocation** is handled automatically. In trial #47 where the escape route is blocked (gate = 0, effectiveness = 0), the remaining controls face the full unmitigated severity. The multiplication does the right thing: `residual[47] = severity[47] * 1.0 * (1 - eff_harness[47])`. This IS the cost-benefit argument for additional controls — you can compute the marginal reduction in P(SIF) from adding each barrier, accounting for the existing controls and their failure modes.

**Common cause failures** (multiple barriers degraded by a single root cause — deferred maintenance, extreme weather, schedule pressure) are modelled via the **Gaussian copula** layer in SIPmath. Effectiveness SIPs for correlated mitigations share HDR seeds coupled through a copula matrix. In trials where one barrier is degraded, correlated barriers are also degraded. This avoids the dangerous assumption of independence that simple multiplication implies.

**Bernoulli gates** for barriers with a distinct "fails to activate" mode:

```
// LOTO: 95% probability of being applied, when applied 99.9% effective
let loto_applied: Vec<bool> = (0..N_TRIALS)
    .map(|i| hdr_uniform(i, entity, VAR_LOTO_GATE) < 0.95)
    .collect();
let loto_eff: Vec<f64> = (0..N_TRIALS)
    .map(|i| if loto_applied[i] {
        metalog_quantile(hdr_uniform(i, entity, VAR_LOTO_EFF), &loto_coeffs)
    } else {
        0.0  // not applied = zero effectiveness
    })
    .collect();
```

### SIPmath 3.0 JSON Serialization

The output format follows the SIPmath 3.0 standard — portable, interoperable:

```json
{
  "libraryType": "SIPmath_3_0",
  "name": "fall_from_scaffold_6m",
  "sips": [
    {
      "name": "severity_unmitigated",
      "function": "Metalog_1_0",
      "arguments": {
        "aCoefficients": [3.0, 0.456, 0.228],
        "boundedness": "sl",
        "lowerBound": 0
      }
    },
    {
      "name": "safety_net_effectiveness",
      "function": "Metalog_1_0",
      "arguments": {
        "aCoefficients": [0.90, 0.046, -0.012],
        "boundedness": "b",
        "lowerBound": 0,
        "upperBound": 1
      }
    }
  ],
  "rng": [
    {
      "function": "HDR_2_0",
      "arguments": {
        "counter": "PM_Index",
        "entity": 1,
        "varId": 1
      }
    }
  ]
}
```

### Rust Crate Structure

```rust
// crate: fractalaw-sipmath (or publishable as standalone `sipmath`)

pub mod metalog {
    /// SPT (3-term) metalog from P10, P50, P90
    pub fn fit_spt(x10: f64, x50: f64, x90: f64) -> [f64; 3];

    /// OLS metalog from n quantile points (k-term, k ≤ n)
    pub fn fit_ols(quantiles: &[(f64, f64)], k: usize) -> Vec<f64>;

    /// Evaluate quantile function: u ∈ (0,1) → x
    pub fn quantile(u: f64, coeffs: &[f64], bounds: Bounds) -> f64;

    /// Numerical CDF: x → P(X ≤ x)  (bisection on quantile function)
    pub fn cdf(x: f64, coeffs: &[f64], bounds: Bounds) -> f64;

    /// Feasibility check: is the quantile function monotonically increasing?
    pub fn is_feasible(coeffs: &[f64], bounds: Bounds) -> bool;

    pub enum Bounds {
        Unbounded,
        SemiLower(f64),
        SemiUpper(f64),
        Bounded(f64, f64),
    }
}

pub mod hdr {
    /// HDR PRNG: 5-component seed → uniform in (0, 1)
    pub fn uniform(counter: u64, entity: u32, var_id: u32, seed3: u32, seed4: u32) -> f64;
}

pub mod sip {
    /// Generate N trials from a metalog SIP
    pub fn generate(n: usize, coeffs: &[f64], bounds: Bounds, entity: u32, var_id: u32) -> Vec<f64>;

    /// Element-wise composition: residual = severity * product(1 - eff_i)
    pub fn chain_mitigations(severity: &[f64], mitigations: &[&[f64]]) -> Vec<f64>;

    /// P(X ≥ threshold) from trial array
    pub fn exceedance_probability(trials: &[f64], threshold: f64) -> f64;

    /// Percentiles from trial array
    pub fn percentile(trials: &[f64], p: f64) -> f64;
}

pub mod io {
    /// Serialize to SIPmath 3.0 JSON
    pub fn to_sipmath_json(sips: &[SipDef]) -> String;

    /// Deserialize from SIPmath 3.0 JSON
    pub fn from_sipmath_json(json: &str) -> Vec<SipDef>;
}
```

Estimated size: ~500 lines of Rust. Dependencies: `nalgebra` or `ndarray` for OLS (only needed for >3 term fits; SPT is closed-form). Compiles to ~10-15 KB WASM.

---

## SIF Simulator (Product 2)

### Concept

The SIF simulator is a **pre-task planning and JHA (Job Hazard Analysis) tool** — no SLM required. A safety professional dials in the energy scenario parameters and gets a severity distribution. They then add **outcome P mitigations** (controls that reduce severity given the event happens), each with its own effectiveness distribution, and see how the residual P(SIF) changes.

This is the **right side of the bowtie quantified with real distributions** instead of point estimates. The simulator defines chance P = 1 (the event happens) and models the outcome distribution with and without barriers.

**Scope boundary**: the simulator models outcome P controls (nets, PPE, energy-absorbing systems). Chance P controls (guardrails, LOTO, barriers that prevent the event from happening at all) are a different question ("how likely is the event?") and are out of scope — they belong on the left side of the bowtie.

### User Workflow

```
1. SELECT ENERGY TYPE           2. SET SOURCE PROPERTIES         3. SEE SEVERITY DISTRIBUTION
   ○ Gravity                       Height: [6] metres               ┌──────────────────┐
   ● Motion                        Mass:   [75] kg                  │    ╱╲             │
   ○ Mechanical                    Surface: Concrete                │   ╱  ╲            │
   ○ Electrical                                                     │  ╱    ╲           │
   ○ Pressure                    Auto-calculated:                   │ ╱      ╲──        │
   ○ Thermal                       PE ≈ 4,414 J                    │╱         ╲─────── │
   ○ Chemical                      → severity metalog              │ P(SIF) = 0.78     │
   ○ Radiation                       P10: medical                  └──────────────────┘
                                     P50: serious
                                     P90: fatal

4. ADD MITIGATIONS               5. SEE RESIDUAL DISTRIBUTION
   [+] Safety net                    ┌──────────────────┐
       Effectiveness:                │      ╱╲          │
       P10: 0.75  P50: 0.90         │     ╱  ╲         │
       P90: 0.97                     │    ╱    ╲        │
   [+] Fall harness                  │   ╱      ╲───    │
       Effectiveness:                │  ╱         ╲──── │
       P10: 0.80  P50: 0.92         │ P(SIF) = 0.02    │
       P90: 0.98                     └──────────────────┘
   [+] Add mitigation...
                                   "Two barriers reduce P(SIF)
                                    from 0.78 → 0.02"
```

### Energy Type → Severity Metalog Mapping

For each energy type, the simulator provides a **parameterised severity metalog** based on the source properties. These are deterministic physics-based mappings, not ML:

| Energy Type | Key Parameters | Severity Mapping |
|-------------|---------------|------------------|
| **Gravity** | height (m), mass (kg), surface hardness | PE = mgh → lookup severity quantiles from empirical fall data |
| **Motion** | mass (kg), velocity (m/s) | KE = ½mv² → lookup severity quantiles |
| **Mechanical** | rotating speed (rpm), torque (Nm), gap (mm) | Power = τω → lookup severity quantiles |
| **Electrical** | voltage (V), available current (A) | Energy = VIt → lookup severity quantiles from electrocution data |
| **Pressure** | pressure (psi/bar), volume (L) | Stored energy = PV → lookup severity quantiles |
| **Thermal** | temperature (°C), duration (s), area (cm²) | Energy = mcΔT → lookup severity quantiles from burn data |
| **Chemical** | substance, concentration, route (inhalation/skin/ingestion) | IDLH/LD50 lookup → severity quantiles |
| **Radiation** | dose (mSv), type (α/β/γ/neutron) | Dose lookup → severity quantiles |

The "lookup severity quantiles" step uses **empirical calibration curves** — published injury severity data for each energy type at different magnitudes. These are built once from epidemiological data (OSHA, RIDDOR, medical literature) and calibrated as metalog coefficients indexed by energy magnitude.

### Mitigation Library

Each mitigation is a **bounded [0, 1] metalog** representing effectiveness. The library provides defaults from published reliability data; users can override with site-specific values.

| Mitigation | Default P10 | Default P50 | Default P90 | Source |
|------------|-------------|-------------|-------------|--------|
| Safety net | 0.75 | 0.90 | 0.97 | Fall protection studies |
| Fall harness (worn + attached) | 0.80 | 0.92 | 0.98 | OSHA data |
| Hard hat | 0.40 | 0.60 | 0.80 | Head strike data (limited protection) |
| LOTO (applied correctly) | 0.95 | 0.99 | 0.999 | LOPA reliability data |
| Machine guarding | 0.85 | 0.95 | 0.99 | Machinery safety standards |
| Arc flash PPE | 0.70 | 0.85 | 0.95 | NFPA 70E |
| Guardrail | 0.80 | 0.92 | 0.98 | Fall protection studies |
| Spotter/banksman | 0.50 | 0.70 | 0.85 | Human reliability analysis |
| Ventilation (confined space) | 0.75 | 0.90 | 0.97 | Atmospheric monitoring data |

Each mitigation can optionally have a **Bernoulli gate** representing the probability of it being present/activated at all (P(worn), P(applied), P(in place)). This separates "how often is it used" from "how well does it work when used."

### Connection to the SIF Classifier

The two products share the metalog engine and the severity distribution model. The classifier **seeds** the simulator:

```
SIF Classifier (Product 1)              SIF Simulator (Product 2)
─────────────────────────               ─────────────────────────
Narrative text                          Manual parameter input
    │                                       │
    ▼                                       ▼
SLM extracts:                           User dials in:
  mechanism = "fall"                      energy = gravity
  height ≈ 6m                             height = 6m
  surface = concrete                      surface = concrete
    │                                       │
    ▼                                       ▼
┌──────────────────────────────────────────────────────┐
│              SHARED METALOG ENGINE                    │
│                                                      │
│  severity_coeffs = energy_to_metalog(gravity, 6m)    │
│  severity_sip = generate(10000, severity_coeffs)     │
│  p_sif = exceedance(severity_sip, SIF_THRESHOLD)     │
└──────────────────────────────────────────────────────┘
    │                                       │
    ▼                                       ▼
P(SIF) = 0.78                          P(SIF) = 0.78
"Flag for SIF investigation"               │
                                            ▼
                                    User adds mitigations:
                                      net_sip = generate(...)
                                      harness_sip = generate(...)
                                      residual = chain(severity, [net, harness])
                                      P(SIF) = 0.02
                                    "Residual risk acceptable"
```

After the classifier flags an event as SIF, the user can open it in the simulator, verify the energy parameters the SLM extracted, adjust if needed, and then explore "what mitigations would have reduced P(SIF)?" This closes the loop between post-event screening and pre-task planning.

### Deployment

| Platform | Implementation | Use Case |
|----------|---------------|----------|
| **Rust crate** | `fractalaw-sipmath` | Embedded in CLI, batch processing |
| **WASM module** | Compiled from Rust | Browser-based simulator UI |
| **SIPmath 3.0 JSON** | Standard serialization | Export/import with Excel tools, other SIPmath-compatible systems |
| **Edge device** | Rust native | Pre-task JHA on corporate laptop |

The WASM module enables a **browser-based simulator** — no install, no server, no data leaves the browser. The safety professional opens a web page, dials in parameters, sees the distribution. This is the lowest-friction deployment path.

---

## Integration with Fractalaw

### Where It Lives

```
crates/
  fractalaw-sipmath/          -- STANDALONE CRATE (publishable)
    src/
      lib.rs                -- public API
      metalog.rs            -- SPT fit, OLS fit, quantile eval, CDF, feasibility
      hdr.rs                -- HDR PRNG (5-component seed → uniform)
      sip.rs                -- SIP generation, composition, exceedance
      io.rs                 -- SIPmath 3.0 JSON serialization
    Cargo.toml              -- minimal deps: nalgebra (OLS only), serde_json (IO)

  fractalaw-core/
    src/sif/
      mod.rs                -- SIF types, energy wheel enum, ICECI codes
      mechanism.rs          -- ICECI mechanism taxonomy (C2 codes, SIF gates)
      energy.rs             -- Energy wheel types, source/carrier/environment factors
      severity.rs           -- Energy magnitude → severity metalog calibration curves
      mitigation.rs         -- Mitigation library (default effectiveness metalogs)
      schema.rs             -- Arrow schema definition for sif_event

  fractalaw-ai/
    src/sif/
      classifier.rs         -- Stage 1 ONNX classifier (mechanism + object)
      energy_analyser.rs    -- Stage 2 structured extraction via Ollama
      pipeline.rs           -- Two-stage pipeline orchestration

  fractalaw-cli/
    src/commands/
      sif.rs                -- CLI: sif classify, sif batch, sif validate
      sif_sim.rs            -- CLI: sif sim (interactive simulator)

scripts/
  sif/
    generate_training.py    -- Synthetic data generation from ICD-11 taxonomy
    annotate_energy.py      -- LLM-assisted Stage 2 annotation
    evaluate.py             -- Benchmark evaluation (precision, recall, F1)
    train_mechanism.py      -- Stage 1 fine-tuning
    train_energy.py         -- Stage 2 fine-tuning
    calibrate_severity.py   -- Build severity calibration curves from OSHA/epidemiological data

data/
  sif/                      -- gitignored, runtime data
    training/               -- OSHA downloads, synthetic data, annotations
    models/                 -- GGUF/ONNX model files
    benchmarks/             -- Gold standard test set
    calibration/            -- Severity metalog calibration curves per energy type

.claude/plans/sif/          -- this plan + future design docs
```

### Persistence Architecture

**Dedicated DuckDB** at `data/sif.duckdb` — same pattern as cultural graph (`data/cultural-graph.duckdb`). SIF events are a distinct domain from legislation (fractalaw.duckdb) and cultural relationships (cultural-graph.duckdb). Three databases, three domains, shared CSV source:

```
QQ CSV (Redactor format)
  ├─ narratives (What, Action) ──→ cultural-graph.duckdb  (relationship extraction)
  └─ near-miss/accident subset ──→ sif.duckdb             (SIF potential classification)
      + supplementary metadata        (mechanism, energy, amplification)
         (SIFp labels from QQ)
```

**Current workflow** (pre-edge deployment):
```
QQ CSV ──→ ingest script ──→ data/sif.duckdb (events table)
                                    │
                                    ▼
                            RunPod inference ──→ results JSONL
                                    │
                                    ▼
                            load script ──→ data/sif.duckdb (classifications table)
```

**Future workflow** (edge deployment — Option A):
```
Customer EHS system ──→ edge device ──→ classify locally ──→ local sif.duckdb
                                                           ──→ Zenoh sync (results only, not narratives)
                                                                    │
                                                                    ▼
                                                              sertantai dashboard
```

### DuckDB Schema

```sql
-- data/sif.duckdb

-- Source events (ingested from CSV or customer system)
CREATE TABLE events (
    event_id          VARCHAR PRIMARY KEY,  -- QQ: original Id from CSV
    site              VARCHAR,              -- site code
    narrative         VARCHAR,              -- What column (free text)
    action            VARCHAR,              -- Action column (if available)
    report_type       VARCHAR,              -- 'Near Miss', 'Accident', etc.
    fy                VARCHAR,              -- financial year
    sector            VARCHAR,
    sub_sector        VARCHAR,
    -- Supplementary metadata (from QQ when available)
    qq_sifp           VARCHAR,              -- QQ's human SIFp label (benchmark only)
    qq_severity       VARCHAR,              -- QQ severity rating (if available)
    qq_mechanism      VARCHAR,              -- QQ mechanism category (if available)
    ingested_at       TIMESTAMP DEFAULT current_timestamp
);

-- Classification results (populated by model inference)
CREATE TABLE classifications (
    event_id          VARCHAR PRIMARY KEY REFERENCES events(event_id),
    -- Stage 1: ICECI mechanism
    mechanism_codes   VARCHAR[],        -- ["1.5", "1.2"]
    mechanism_labels  VARCHAR[],        -- ["Fall", "Struck by object"]
    object_codes      VARCHAR[],        -- ICECI C3
    object_labels     VARCHAR[],
    sif_gate          VARCHAR,          -- NEEDS_ASSESSMENT | AUTO_NON_SIF
    stage1_confidence FLOAT,
    -- Stage 2: EBS energy analysis (null if AUTO_NON_SIF)
    energy_types      VARCHAR[],        -- ["gravity", "motion"]
    source_cues       VARCHAR[],        -- ["6 metre scaffold"]
    carrier_cues      VARCHAR[],        -- ["sharp edge"]
    environment_cues  VARCHAR[],        -- ["concrete floor"]
    body_vulnerability VARCHAR,         -- body region or null
    severity_p10      VARCHAR,          -- ordinal severity quantile
    severity_p50      VARCHAR,
    severity_p90      VARCHAR,
    energy_reasoning  VARCHAR,          -- model's reasoning chain
    stage2_confidence FLOAT,
    -- Final output (deterministic from quantiles + metalog)
    p_sif             FLOAT,            -- P(severity ≥ serious_injury)
    sif_class         VARCHAR,          -- SIF | ELEVATED | NON_SIF
    review_flag       BOOLEAN,
    classified_at     TIMESTAMP DEFAULT current_timestamp,
    model_version     VARCHAR
);

-- Human review decisions (for UNCERTAIN or QA samples)
CREATE TABLE reviews (
    event_id          VARCHAR REFERENCES events(event_id),
    reviewer          VARCHAR,
    decision          VARCHAR,          -- SIF | NON_SIF (human override)
    notes             VARCHAR,
    reviewed_at       TIMESTAMP DEFAULT current_timestamp,
    PRIMARY KEY (event_id, reviewer)
);
```

Separating `events` from `classifications` means:
- Events can be ingested before any model runs (ingest and classify are independent steps)
- Re-classification (new model version) inserts/updates `classifications` without touching `events`
- QQ's human SIFp label lives on `events` (ground truth), model output lives on `classifications` — benchmark comparison is a simple join

### Sync (Future — Edge Deployment)

SIF classification results (not raw narratives) sync to sertantai via Zenoh Arrow IPC. Key expression: `fractalaw/@{tenant}/sif/{site}/{event_id}`. Narratives stay on the customer's edge device — only structured classification output crosses the network boundary.

---

## Implementation Phases

### Phase 0: SIPmath Engine (Foundation)

Build first — both products depend on it, it's small, and it's independently valuable.

1. Implement `fractalaw-sipmath` crate: metalog (SPT + OLS), HDR PRNG, SIP composition
2. SIPmath 3.0 JSON serialization/deserialization
3. Feasibility checking for metalog coefficient sets
4. Unit tests against R `rmetalog` reference outputs
5. WASM compilation target (`wasm32-unknown-unknown`)

**Deliverable**: Publishable `fractalaw-sipmath` crate with WASM support. ~500 lines of Rust.

### Phase 1: Taxonomy, Calibration & Training Data

1. Download and parse ICD-11 Chapter 23 + Extension Codes via API → structured label definitions
2. Build ICECI C2 → Energy Wheel mapping table (mechanism → energy type)
3. **Build severity calibration curves**: for each energy type × magnitude range, fit metalog coefficients from epidemiological data (OSHA severity outcomes, RIDDOR, medical literature). These are the lookup tables the simulator uses.
4. **Build mitigation effectiveness library**: default metalog coefficients for common mitigations from published reliability data (LOPA, fall protection studies, NFPA 70E)
5. Download OSHA severe injury reports → extract narratives + labels
6. Generate synthetic training data (Gemini) for underrepresented ICD-11 classes (<30% of training mix)
7. Build gold-standard benchmark set (~2,000 annotated events covering all 13 STKY hazards + QQ SIFp data as real-world benchmark)

**Deliverable**: Training dataset + benchmark + taxonomy files + calibration curves + mitigation library

### Phase 2: SIF Simulator (Product 2 — no SLM needed)

Build before the classifier — it's simpler, immediately useful, and validates the calibration curves.

1. Implement `fractalaw-core::sif` types: energy wheel enum, mechanism codes, severity scale
2. Implement `fractalaw-core::sif::severity` — energy parameters → severity metalog via calibration curves
3. Implement `fractalaw-core::sif::mitigation` — mitigation library with Bernoulli gates
4. CLI command `sif sim` — interactive: select energy type, set parameters, add mitigations, see P(SIF)
5. Validate against known scenarios (13 STKY hazards should produce P(SIF) > 0.50 at published thresholds)
6. WASM build for browser-based simulator prototype

**Deliverable**: Working SIF simulator, CLI + WASM. Usable for pre-task JHA without any ML.

### Phase 3: Stage 1 Classifier

1. Fine-tune Qwen 3 0.6B for multi-label mechanism + object classification
2. Export to ONNX for edge inference
3. Evaluate against benchmark (target: F1 ≥ 0.85 for mechanism, ≥ 0.80 for object)
4. Implement `fractalaw-ai::sif::classifier`
5. Add SIF gate logic (auto-non-SIF routing for over-exertion/abrasion)

**Deliverable**: Working Stage 1 classifier with CLI `sif classify` command

### Phase 4: Stage 2 Energy Analyser

1. Annotate OSHA narratives with energy analysis labels (LLM-assisted + human validation)
2. Fine-tune Qwen 3 4B for structured JSON extraction (energy type, source/carrier/environment cues, severity quantiles)
3. Implement `fractalaw-ai::sif::energy_analyser` with Ollama integration
4. Build the two-stage pipeline: Stage 1 → Stage 2 → metalog fit → P(SIF)
5. Evaluate end-to-end against benchmark (target: SIF recall ≥ 0.90, precision ≥ 0.80)
6. Evaluate against QQ SIFp benchmark (target: classifier–human agreement > 65% baseline)

**Deliverable**: Full two-stage SIF classifier pipeline with CLI `sif batch` command

### Phase 5: Integration & Validation

1. DuckDB schema + storage (`data/sif.duckdb`)
2. Zenoh sync for SIF classification results
3. Human review workflow for ELEVATED classifications
4. Connect classifier output to simulator — "investigate this event" opens in simulator with pre-filled parameters
5. Customer pilot with QQ incident data
6. Feedback loop: corrections → fine-tuning data → model update

**Deliverable**: Production-ready SIF classification + simulation service

---

## Key Design Decisions to Validate

| Decision | Options | Current Lean | Needs Validation |
|----------|---------|-------------|-----------------|
| Stage 1 model size | 0.6B / 1.7B / 4B | 0.6B | Benchmark F1 after fine-tuning |
| Stage 2 model size | 4B / 8B | 4B | Quality of severity quantile estimation |
| Single vs two-stage | Two-stage / single 8B | Two-stage | Compare end-to-end accuracy. Two models = two interpretable outputs = product feature |
| ICECI vs ICD-11 labels | ICECI C2 / ICD-11 Ch 23 | ICD-11 (maintained) | API availability + label granularity |
| P(SIF) threshold | 0.50 / 0.30 / customer-configurable | 0.50 default, configurable | False positive rate on QQ benchmark data |
| Severity quantile count | 3 (P10/P50/P90) / 5 (add P25/P75) | 3 | Metalog fit quality with 3 vs 5 quantiles |
| Metalog vs simpler CDF | Metalog / triangular / empirical | Metalog | Implementation complexity vs fit quality |
| SIPmath crate scope | Internal module / publishable crate | Publishable | No Rust metalog crate exists — is the market worth maintaining? |
| Trial count | 1,000 / 10,000 / 100,000 | 10,000 | Precision vs speed trade-off on edge device |
| Mitigation composition | Multiplicative / Bernoulli-gated / both | Both | Does multiplicative alone capture barrier failure modes? |
| Calibration curve source | OSHA only / OSHA + RIDDOR + medical lit | OSHA + RIDDOR | Do UK-context curves differ materially from US? |
| Severity scale | Naive ordinal / AIS-weighted / insurance cost / custom | AIS-weighted | Sensitivity analysis on scale mapping → P(SIF) impact |
| Simulator UI | CLI only / WASM browser / both | Both | Browser prototype before committing to full UI |
| Copula structure | Independent / single-factor CCF / full copula | Single-factor CCF | How often do correlated barrier failures matter in practice? |

---

## References

### Classification Schemes
- WHO. ICECI Version 1.2 (2004). https://www.whofic.nl/sites/default/files/2018-05/ICECI%20in%20English.pdf
- WHO. ICD-11 Chapter 23: External Causes. https://icd.who.int/browse11
- EU Injury Database FDS Data Dictionary (2017). https://www.eurosafe.eu.com

### Energy-Based Safety
- Haddon, W. (1973). Energy Damage and the Ten Countermeasure Strategies. *Human Factors*, 15(4), 355–366.
- Hallowell, M.R. (2024). *Energy-Based Safety*. CRC Press/Routledge. ISBN 9781041076339.
- Hallowell, M.R. et al. (2017). Energy-based safety risk assessment. *Construction Management and Economics*, 35(1-2). DOI: 10.1080/01446193.2016.1274418
- EEI. Safety Classification and Learning (SCL) Model. https://www.eei.org (Power to Prevent SIF)
- EEI. High-Energy Control Assessments (HECA). https://www.eei.org

### SIF Frameworks
- ASTM E2920-26. Standard Practice for Recording Occupational Injuries and Illnesses.
- Campbell Institute & DEKRA (2017). Perspectives on SIF Prevention.
- Campbell Institute (2020). Designing Strategy for SIF Prevention.

### ML/NLP for Safety
- Bejaoui, I. et al. (2024). BERT + XGBoost for PSIF classification. *Nature Scientific Reports*. DOI: 10.1038/s41598-024-58824-y
- CHASNZ. Energy Wheel. https://chasnz.org/energy-wheel
- Hallowell & Spencer (2024). Energy-Based Safety in *Professional Safety* (ASSP).

### Injury Severity Scales
- AAAM. Abbreviated Injury Scale (AIS). Association for the Advancement of Automotive Medicine.
- Baker, S.P. et al. (1974). The Injury Severity Score. *Journal of Trauma*, 14(3), 187–196.

### Probability / Distribution Modelling
- Savage, S. (2009). *The Flaw of Averages*. John Wiley & Sons.
- Keelin, T.W. (2016). The Metalog Distributions. *Decision Analysis*, 13(4), 243–277. DOI: 10.1287/deca.2016.0338
- ProbabilityManagement.org. SIPmath 3.0 Standard. https://www.probabilitymanagement.org/30-standard
- ProbabilityManagement.org. HDR Random Number Generator. https://www.probabilitymanagement.org/hdr
- ProbabilityManagement.org. Canonical SIPmath Libraries. https://www.probabilitymanagement.org/canonical-libraries
- Faber, I. `rmetalog` R package. https://cran.r-project.org/web/packages/rmetalog/
- Jefferies, T. `metalog_jax` Python package. https://github.com/tjefferies/metalog_jax
- FAIR Institute. FAIR Meets SIPmath (3-part series). https://www.fairinstitute.org/blog/fair-meets-sipmath-part-1

---

## Gemini Review Feedback (2026-08-19)

Full review: `data/code-review/sif-classifier-design-review.md`

**Actioned in v0.2:**
1. Auto-SIF gates removed — mechanism alone is insufficient (desk-drawer vs industrial press). Auto-non-SIF retained for inherently low-energy mechanisms (over-exertion, abrasion).
2. Discrete magnitude bands (LOW/MEDIUM/HIGH) replaced by severity probability distributions (SIPmath metalog from P10/P50/P90 quantiles). P(SIF) is continuous, classification threshold is customer-configurable policy.
3. Amplification/mitigation boundary unpacked — clear rules for what counts as source/carrier/environment (include) vs engineered controls (exclude).
4. Benchmark increased from 200 to 2,000+ events.
5. Combined output logic rewritten — no more brittle `MEDIUM AND amplification → SIF` rule.

**Noted but not actioned:**
- Gemini advocated single-model over two-stage — we keep two-stage because two interpretable outputs (mechanism + energy analysis) are a product feature, not just an implementation detail. Cost is pennies.
- Gemini advocated scrapping synthetic data — we keep it for class balancing (<30% of training mix) but acknowledge the stylistic artifact risk.
- Gemini advocated large cloud model first — contradicts edge deployment requirement and privacy constraint (incident narratives don't leave customer network).
- Gemini raised ICECI as post-hoc taxonomy — valid observation, but we use it as a label ontology (vocabulary for "what happened"), not as a clinical coding process.
- Gemini raised control effectiveness — we deliberately exclude engineered controls (SIF potential = energy present regardless of barriers). The controlled outcome is one draw from the severity distribution; we're characterising the distribution.

### Gemini v0.2 Review Feedback (2026-08-19)

Full review: `data/code-review/sif-classifier-v02-design-review.md`

Gemini acknowledged the SIPmath shift as "a quantum leap in sophistication" and conceded the controls exclusion point entirely ("the authors are unequivocally correct").

**Actioned in v0.2 (continued):**
1. Ordinal-to-numeric severity scale → replaced with AIS-weighted or insurance cost-based continuous scale. Sensitivity analysis added as design decision to validate.
2. Common cause failures → modelled via Gaussian copula layer in SIPmath. Correlated mitigations share coupled HDR seeds.
3. Calibration bias → resolved by Chance P vs Outcome P framing. OSHA data has chance P = 1 (events that happened), which IS the right data for outcome curves.
4. Validation/test set → must be 100% real-world data. Synthetic only in training (<30% of mix).
5. Dynamic risk allocation → already handled by Monte Carlo multiplication. Documented explicitly.
6. Missing data → Bayesian priors from OSHA population distributions, narrowed by available cues. Feedback loop for data quality improvement.

**Noted for later:**
- SLM uncertainty propagation (Bayesian treatment of model output uncertainty) → v2.0 enhancement.
- Mitigation library anchoring → UI design concern. Defaults labelled as starting points.
- Bimodal severity distributions → monitor whether 3-term SPT is sufficient; fall back to 5-term OLS if needed.
