---
session: SIF definition papers
status: active
opened: 2026-11-17
---

# Session: SIF definition papers (ACTIVE)

## Problem

The SIF definition work (discussion doc, definitions doc, counterfactuals paper) surfaced several contested topics that each need a thorough literature review in their own paper. Two come next. The first is how controls are treated across PSIF and SIF-precursor formulations; the safety director is keen on it. It also has to settle QQ's current practice of coding positive and hazard observations (conditions) as PSIF, which the definitions treat as *SIF exposure*, not events. The second is severity: the LIFE model (Bayona et al., 2023), who uses it, what else is used, and how it compares with healthcare trauma scales (AIS).

## Todo

- [x] Paper 1 literature research (agent): controls in PSIF/precursor formulations; conditions and observations (HECA, BST precursors, EEI Exposure/Success); ICMM critical controls, bowtie, ARMS barriers; Safety-II objections
- [x] Paper 1 draft v0.1: `docs/sif/sif-controls-precursors.md`, *Controls, precursors and conditions in SIF potential*, with an executive summary for the safety director
- [x] Paper 1: position on PSIF-coding observations and hazard conditions (QQ practice vs the *SIF exposure* definition)
- [x] Paper 1: verify quotes against primary sources (author supplies copies in `docs/sif/lit/`)
- [ ] Paper 2 literature research: LIFE model adoption; AIS/ISS/NISS; impairment scales; regulatory lists
- [ ] Paper 2 draft: `docs/sif/sif-severity.md`, LIFE vs AIS and the AI model's life-altering gap (parked Q6)
- [ ] Back-port any definition changes to `sif-definition.md` and `sif-definition-discussion.md`

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
