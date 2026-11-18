---
session: SIF definition papers
status: closed
opened: 2026-11-17
closed: 2026-11-18
outcome: success

summary: >
  Wrote the controls/precursors paper (v0.2, two-sided events) and the severity paper (v0.2, LIFE vs AIS,
  life-altering gap). Back-ported both into the definitions and discussion docs (v1.8). Agreed the
  live / at-rest / zero-energy energy states. Drafted the QQ proposal (v0.1) and the PSIF scenario index
  (v0.1, risk-assessment side, range-operations pilot from DSA 03.OME Part 3).

decisions:
  - what: Code by content, not by channel; observations of an incursion are PSIF; conditions are SIF exposure
    why: Presence in the line of fire is the incursion whether or not it was seen; EEI's source-centric event definition gives running machinery no route to PSIF
    result: Paper 1 rec. 1; definitions v1.7 §3.2; proposal rec. 2
  - what: Keep "unverified" as a third control state
    why: Maps to unprotected for EEI/HECA; works as the panel's queue; its share measures reporting quality
    result: Paper 1 §7; definitions §7.3
  - what: Energy states live / at-rest / zero-energy (hyphenated defined terms)
    why: Energised/de-energised is electrical-only; test is whether anything must keep working to hold the energy; gravity never reaches zero-energy
    result: Definitions and discussion v1.8; landing a load removed from elimination
  - what: Two severity axes (threat to life, permanent impairment), combined by maximum; body-region rule now, RPMI later
    why: AIS is a threat-to-life scale and under-counts hand, finger and eye injuries; burden-of-injury data (EUROCOST/INTEGRIS) scores hand/eye at 0% lifelong
    result: Paper 2 v0.2; ICISS recorded as the AIS-free fallback
  - what: AI model positioned as an optional Spongl Ltd screen; the human panel process stands alone
    why: Model is Spongl Ltd IP, not QQ's core process; recs 1–8 must not depend on it
    result: Proposal rec. 9; Spongl fixes referenced only in passing
  - what: Index PSIF scenarios proactively, linked to coded reports
    why: Risk assessment needs no actual event; ranges have numeric envelopes (WDA, BSD) and written controls
    result: sif-scenario-index.md v0.1, 36 range scenarios

metrics:
  papers: { controls_precursors: v0.2, severity: v0.2, definitions: v1.8, discussion: v1.8, proposal: v0.1, scenario_index: v0.1 }
  qq_comparison: { reports: 3006, human_fatal_model_sif: "25%", not_sifp_read_as_sif: "~260", narratives_with_magnitude: "3-5%" }
  range_scenarios: { total: 36, psif: 26, elevated: 4, sif_exposure: 6 }

lessons:
  - title: Verify agent-supplied quotes against saved primary texts
    detail: Paper 1 research found Campbell 2020 wording, the 87% figure (Martin & Black, not Krause) and a Cooper quote all wrong in secondary use; the mislabelled bayone_et_al_2023.pdf was really Bayona 2026. Every quote was then checked word for word.
    tag: methodology
  - title: Source-centric event definitions miss incursions
    detail: EEI needs a release to make an event, so a person under a holding load is only Exposure. Treating events as two-sided (excursion or incursion) fixed the sparse event tables and the observation-coding problem in one move.
    tag: methodology
  - title: Burden-of-injury data does not close the life-altering gap
    detail: The ChatGPT-suggested EQ-5D/WHODAS/ICF/DALY measure realised burden, and EUROCOST/INTEGRIS scores hand and eye injury at 0% lifelong. Listing them as considered and set aside showed thoroughness without changing the recommendation.
    tag: models
  - title: The band table has no ballistic band
    detail: Building the range index showed a stray rifle round would fall into struck/swinging_projectile (0.439, Elevated). Running the rules forwards over a new domain exposes calibration gaps that report-led coding hides.
    tag: models
  - title: Client research notes must be gitignored before they are committed
    detail: sif_qq_schema.md held the client's full name and a signed portal URL; docs/sif/research/ was added to .gitignore.
    tag: data
  - title: Check that regulatory sources are current
    detail: JSP 403 is withdrawn; the current MoD range code is DSA 03.OME Part 3. Its "Line of Fire" (barrel to target) clashes with the SIF term.
    tag: data

artifacts:
  - docs/sif/sif-controls-precursors.md
  - docs/sif/sif-severity.md
  - docs/sif/sif-definition.md
  - docs/sif/sif-definition-discussion.md
  - docs/sif/proposal.md
  - docs/sif/sif-scenario-index.md
  - .gitignore

depends_on:
  - 10-29-26-qq-batch-oct.md

enables:
  - SIF AI model improvements session (deferred model actions from the papers, proposal and scenario index)
  - QQ pilot of the calibrated panel
  - Range-operations scenario index pilot
---

# Session: SIF definition papers (CLOSED)

## Problem

The SIF definition work (discussion doc, definitions doc, counterfactuals paper) surfaced several contested topics that each need a thorough literature review in their own paper. Two come next. The first is how controls are treated across PSIF and SIF-precursor formulations; the safety director is keen on it. It also has to settle QQ's current practice of coding positive and hazard observations (conditions) as PSIF, which the definitions treat as *SIF exposure*, not events. The second is severity: the LIFE model (Bayona et al., 2023), who uses it, what else is used, and how it compares with healthcare trauma scales (AIS).

## Todo

- [x] Paper 1 literature research (agent): controls in PSIF/precursor formulations; conditions and observations (HECA, BST precursors, EEI Exposure/Success); ICMM critical controls, bowtie, ARMS barriers; Safety-II objections
- [x] Paper 1 draft v0.1: `docs/sif/sif-controls-precursors.md`, *Controls, precursors and conditions in SIF potential*, with an executive summary for the safety director
- [x] Paper 1: position on PSIF-coding observations and hazard conditions (QQ practice vs the *SIF exposure* definition)
- [x] Paper 1: verify quotes against primary sources (author supplies copies in `docs/sif/lit/`)
- [x] Paper 2 literature research: LIFE model adoption; AIS/ISS/NISS; impairment scales; regulatory lists
- [x] Paper 2 draft: `docs/sif/sif-severity.md`, LIFE vs AIS and the AI model's life-altering gap (parked Q6)
- [x] Back-port any definition changes to `sif-definition.md` and `sif-definition-discussion.md`
- [x] QQ proposal draft v0.1: `docs/sif/proposal.md`, the argument for the recommendations, landing the PSIF definition and the Spongl Ltd AI model
- [x] PSIF scenario index v0.1: `docs/sif/sif-scenario-index.md`, the risk-assessment side; schema and a range-operations pilot from DSA 03.OME Part 3
- [ ] ⏸️ Add a `ballistic` mechanism and fragmentation/demolition explosion bands to the band table and AI model calibration (index §5) (deferred — moved to the SIF AI model improvements session)

## Dependencies

- [x] SIF definition discussion doc: `docs/sif/sif-definition-discussion.md` (v1.6)
- [x] SIF definitions and coding rules: `docs/sif/sif-definition.md` (v1.6, with "AI model" naming and copyright footnote)
- [x] Counterfactuals paper: `docs/sif/sif-counterfactual.md` (v0.7, pushed at `38c1874`). Establishes control vs outcome counterfactuals and the triage → AI screening → calibrated panel process
- [x] Literature PDFs kept local: `docs/sif/lit/` (gitignored)

## Paper 1: controls, precursors and conditions

**Literature research.** Primary texts read for Krause 2010, Martin & Black 2015, Campbell 2018/2020, EEI SCL 2024, Hallowell & Spencer 2024, HECA (Oguz Erkal & Hallowell 2023), Bayona et al. 2026 (*JSR*), ICMM 2015, the CCPS/EI bowtie papers, ARMS 2010, Busch et al. 2021, Hopkins 2009, Reason 2000 and Cooper 2019.

**Corrections found:**
- Campbell 2020 reads "not in and of itself a precursor".
- The 87% observation figure is Martin & Black 2015 (analyst judgement on 55 cases), not Krause 2012.
- Cooper's Wachter & Ferguson quote was not found in their article.
- The local file `bayone_et_al_2023.pdf` is actually Bayona et al. 2026 (HECA study).

**Key findings:**
- Controls sit in three places across the frameworks: they define a precursor (BST, DEKRA, Campbell), are set aside when judging potential (Campbell 2018, ICMM, ARMS Q1, EEI's energy test), or qualify the result (EEI SCL, ARMS Q2).
- Only Cooper 2019, an opinion piece, applies the PSIF label to conditions. EEI SCL keeps conditions as Exposure and Success.
- Only one study links control presence to SIF rates (Bayona 2026: HECA, −3% SIFs per point). It is correlational, uses a convenience sample, and most of its injury window predates the measurement.

**Draft v0.1** has an executive summary recommending that observations be recorded as SIF exposure (protected or unprotected), not PSIF.

**Decided (Jason): keep "unverified".** It maps to unprotected for EEI and HECA comparisons, works as the panel's work queue, and its share is reported as a measure of reporting quality. Written into paper 1 §7.

**Draft v0.2: two-sided events.** The v0.1 premise that "observations are conditions" was wrong under the AI model. Presence in the line of fire *is* the incursion, seen or not, so items are coded by what was observed, not by how they were reported. EEI's incident definition is source-centric: it needs a release ("temporary loss of control … or changes state", p. 11). That gives running machinery no route to PSIF, and a person under a load that holds is only Exposure. The one exception is EEI's traffic note, "or if an employee enters the traffic pattern" (p. 20): an incursion allowed for one hazard. v0.2 adds §4.2 (source-centric vs two-sided events, with a comparison table against EEI), rewrites recommendation 1 as "code by content, not by channel", and adds an EEI crosswalk showing where the counts differ.

**Agreed (Jason), to back-port into the definitions docs:**
- A suspended load is live until landed. An incursion into the lift exclusion zone is an event. A crane collapse is an excursion.
- Tag-liners are exposed to the crane (a condition), and are an event only if the crane fails.
- Incursion wording: planned presence within reach of the energy is exposure. Incursion means entering the line of fire or danger zone that the work keeps clear. Line of fire: "the path live energy takes as it moves, or the path a foreseeable release would take".
- Choose a word for "not live" (Jason: "dead?"). Candidate: *at rest*, covering landed, stopped and isolated, with isolation as one way to bring energy to rest.
- When the source and the target are the same (a person at height, a vehicle occupant), the event is an excursion: like any energy source, the person leaves the space in which they can operate safely, and the fall is the signature. An arrested fall is PSIF (protected).
- An edge barrier or travel restraint marks a danger zone. Crossing or defeating it is an incursion (an event), and falling afterwards is a second event. Working at an edge where no zone is drawn is exposure. `sif-definition.md:102` ("a person climbs past edge protection and falls") stays correct as a both-sides example.
- A trip or fall on a platform near an edge is an excursion (loss of balance) with same-level-fall potential. The edge does not count, and the SIF exposure to a fall from height continues. Only a continuous fall over the edge is the fall-from-height event. This diverges from EEI Case A. Check `sif-definition.md:298` "nearby edges" in the receiving environment, which may contradict it.
- Triggers, exposure and receiving environment follow the direction of the energy flow and map onto the risk equation: trigger → chance, exposure → exposure, receiving environment → consequence. Only the receiving environment counts toward potential. A trigger is not a precursor; the precursor is the unprotected SIF exposure. Added as paper 1 §4.3, with triggers as an optional coding field. Back-port the `sif-definition.md:298` wording: "Count the receiving environment: what the energy would land on or strike (the surface below, rebar, sharp edges)".
- Sources added in v0.2: CCPS & EI (2018) bowtie book (barrier criteria §2.6.3; training and competence are degradation controls, p. 43); Campbell Institute (2022) (DEKRA 2019 precursor definition; Dow pSIF reduction target; pSIF check-box on incident reports). Martin & Strikoff (2012) cannot be traced outside DEKRA, so it is dropped.
- Quotes verified (2026-11-17): every quotation matched word for word against the source text (8 sources local, 10 downloaded). None is unverified. Page numbers were checked only for EEI and CCPS/EI. The 10 downloaded copies, plus the EEI HECA guide and Wachter & Ferguson, were copied into `docs/sif/lit/` (gitignored).
- Back-ported (2026-11-17): `sif-definition.md` v1.7 and `sif-definition-discussion.md` v1.7. Covers live/at rest, line of fire, danger zone (edge zone), exposure (planned presence), incursion, triggers and receiving environment. Adds new §3.3 (person as the energy) and §3.4 (triggers), an EEI population note under the crosswalk, rule 7.1 wording, step 3/7 changes, an optional trigger field, and 3 worked examples. The discussion doc gets a new refinement section and glossary rows, and its old "standing under a suspended load" exposure example is corrected.

## Paper 2: severity (LIFE vs trauma scales)

**Research (agent, 2026-11-17).** The primary LIFE paper is Bayona et al. (2023), *PSJ* 68(9):22–30, with no DOI. Its reliability test is one self-run pre/post study (KR-20 0.21–0.53, no kappa). LIFE judges actual outcomes only, and "potential" is left to future work. The EEI 2025 criteria use LIFE-like wording but exclude finger fractures and most fingertip amputations, so EEI and LIFE disagree on Bayona's own fingertip case. AIS is "mainly a threat-to-life scale". The best candidate for a second axis is RPMI (Malm et al., 2008), the risk of permanent impairment by body region × AIS. IIS and pFCI are poorly validated. AAAM prohibits using AIS content with generative AI; this needs checking against the AI model. Downloads are in `docs/sif/lit/severity/`.

**Draft v0.1** `docs/sif/sif-severity.md` recommends:
- two axes (threat to life, permanent impairment), counting an outcome as serious if either axis is crossed;
- a body-region rule now (hands, fingers, eyes) and an RPMI-style axis later, combined by taking the maximum;
- following LIFE at the hand, not EEI;
- using lists for calibration only;
- checking the AIS licence.

Quotes are verified except the ASTM E2920-26 wording, which is marked unverified inline.

**v0.2 (2026-11-18): burden-of-injury route checked.** Jason's ChatGPT note (`docs/sif/research/ais_plus_chatgpt.md`) proposed EQ-5D, WHODAS 2.0, ICF and DALY. These were added to §5.3 as considered and set aside, because they measure realised burden, not potential. A second agent checked EUROCOST/INTEGRIS (Haagsma 2012 Table 1): it is AIS-free, but scores hand/finger fracture and eye injury at 0% lifelong, ED and admitted alike, with no amputation group and severe burns excluded. It is therefore rejected as a life-altering axis (§5.2), and RPMI stays the candidate. GBD treats amputation as P(long-term) = 1, which supports the body-region rule, but is non-commercial-licensed. None gives mortality by injury group; ICISS (Osler 1996, ICD-based survival risk ratios) is the AIS-free fallback for threat to life (§4.4). Downloads in `docs/sif/lit/severity/burden/`.

**Series fixes (applied 2026-11-18, definitions and discussion v1.8):**
- Make the difference between "P(death) ≥ 0.10" (the serious level on the scale) and "P(serious) ≥ 0.50" (the SIF class) explicit.
- `sif-definition.md` §5.6 RIDDOR: add reg. 4(1)(h)(ii) "requires resuscitation", and use "likely to cause" for sight.
- The Campbell 2020 quote in the discussion doc §1.1 is verified primary, without the dashes.

## Energy states: live, at-rest, zero-energy (2026-11-18)

**Decided (Jason):** three hyphenated defined terms, hyphenated to aid searching. The test is whether anything has to keep working to hold the energy where it is.
- **Live:** moving, or held by something that can fail or be undone.
- **At-rest:** present and stable; moves only if released (landed load, isolated machine with stored energy, isolated system not yet bled).
- **Zero-energy:** isolated and with stored energy drained or blocked (lockout's "zero energy state"). Gravity never reaches it.

Energised/de-energised was rejected as electrical-only. Landing a load is no longer listed as elimination, because landed loads topple, roll and slide. Applied to definitions and discussion v1.8.

## QQ proposal (2026-11-18)

**Draft v0.1** `docs/sif/proposal.md` (about 5,400 words). It is built from the four papers and from QQ's own data: the guidance text and FY2027 *Rating_1* counts from `docs/sif/research/sif_qq_schema.md`, plus the October batch comparison on 3,006 reports (human Fatal / High / Not SIFp against the model's SIF/Elevated/Not SIF).

- **Argument:** the guidance mixes "on average" with "most serious credible"; the field mixes two scales (89% descriptor, 11% treatment); 69% of ratings are on observations; human levels don't separate on energy.
- **Recommendations:** 9 for QQ; the AI model is optional (rec. 9) and the human process stands alone. Spongl Ltd ownership is in a footnote; Spongl's development fixes are referenced only in passing.
- **Annex A** recodes the guidance's own examples. Divergence: fire in an unoccupied storage building becomes PSIF with closest approach wide (separation to zero).
- `research/sif_qq_schema.md` names the client in full and holds a signed portal URL, so it must not be committed as is.

## PSIF scenario index (2026-11-18)

**Idea (Jason):** run the coding rules from the risk-assessment side. Each credible excursion or incursion for an activity becomes a scenario, a PSIF event defined before it happens. Coded reports link back to scenarios, giving frequency, closest approach and control failure by scenario.

**Draft v0.1** `docs/sif/sif-scenario-index.md`:
- Six-table schema: scenario, activity, source, control, scenario_control, report_scenario. Scenario IDs take the form `RNG-EX-WN-02`.
- Range pilot from DSA 03.OME Part 3 (JSP 403 is withdrawn). PDFs are in `docs/sif/lit/ranges/` (gitignored).
- 36 scenarios: 26 PSIF, 4 Elevated, 6 SIF exposures.

**Findings:**
- Wide approaches dominate.
- Excursions in time are hidden: found loaded, live round in blank ammunition.
- Muzzle sweep has no direct control.
- Closed impact areas are standing danger zones, because blinds are live.

**Gaps and fit with MoD terms:**
- The band table has no ballistic band, and `struck/swinging_projectile` (0.439) would under-rate a rifle round.
- The MoD already uses "incursion" in our sense, and the WDA matches our envelope.
- The MoD's "Line of Fire" (barrel to target) clashes with our term.
