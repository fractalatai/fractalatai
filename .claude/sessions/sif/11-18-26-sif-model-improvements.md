---
session: SIF AI model improvements
status: active
opened: 2026-11-18
---

# Session: SIF AI model improvements (ACTIVE)

## Problem

The SIF definition papers, the QQ proposal and the scenario index have fixed the definitions, and the AI model (Spongl Ltd) now lags them. Its output has no event type, energy state or control state. Its severity scale under-counts life-altering injuries at the hand and eye. It has no ballistic band. It may also conflict with the AIS licence. This session brings the model into line with the papers before the QQ pilot (proposal rec. 9, §7) and the range-index pilot.

## Todo

- [ ] AIS licence check: does AAAM's generative-AI prohibition cover the model's AIS-derived bands? If so, plan an ICISS recalibration (severity rec. 6, §4.4, §9)
- [ ] Body-region rule: flag hand, finger and eye bands, set P(life-altering), and combine with threat to life by taking the maximum (severity §8; proposal §5.4)
- [ ] Impairment axis groundwork: body-region distributions per band from incident data, mapped to RPMI-type tables (severity §9)
- [ ] Band gaps: a `ballistic` mechanism (air weapon, backsplash, handgun, centrefire rifle) and `explosion/fragmenting_munition` and `demolition_charge` bands; wildland fire (scenario index §5)
- [ ] Build-up bands: choose the band from the uncontrolled limit, not the level the narrative says was reached (discussion §6.7)
- [ ] Out-of-scope gate: overexertion and abrasion are Not SIF; medical events and chronic exposure are out of scope (discussion §6.5)
- [ ] Event output: excursion, incursion or both; where, when or how much; closest approach; SIF exposure as its own output, coded by content not channel (definitions §3, §7.4; controls paper rec. 1)
- [ ] Energy-state recognition: live, at-rest or zero-energy; entering near at-rest energy is not an incursion; planned releases inside their envelope are no event (definitions §2.1; discussion §6.6)
- [ ] Control-state extraction: effective, failed or absent, or unverified, with a direct-control flag, giving PSIF protected, unprotected or unverified (discussion §6.3; controls paper §7)
- [ ] Optional trigger field, kept apart from the receiving environment, which does count (definitions §3.4, §7.4)
- [ ] Missing magnitude: lowest-band default against a Bayesian prior or calibrated ranges, tested on thin QQ narratives (only 3–5% state a magnitude) (discussion Q7; counterfactual §8)
- [ ] Scenario matching: map reports to scenario-index IDs, with `none` marking a candidate new scenario (scenario index §2.5)
- [ ] Panel support: known-answer calibration items generated from the band distributions, for Brier scoring (counterfactual §6.4; proposal rec. 5)
- [ ] Validation set: regulatory-list cases (RIDDOR, EEI) as labelled audit cases; check the distributions against outcomes (severity rec. 5; counterfactual §8)
- [ ] Re-run the October QQ batch (3,006 reports) after the changes and compare with v1 and the human ratings (proposal §1.4)
- [ ] Update `sif-definition.md` §4 and Annex B, and the proposal §5.4 limits, to match the changed model

## Dependencies

- [x] SIF definition papers: `11-17-26-sif-definition-papers.md` (closed). Definitions v1.8, controls paper v0.2, severity paper v0.2, proposal v0.1, scenario index v0.1
- [x] October QQ batch baseline: `10-29-26-qq-batch-oct.md`
- [x] Model code and calibration: `scripts/sif/zeroshot_sif.py`, `scripts/sif/band_selector.py`, `data/sif/calibration/*.json`; plan at `.claude/plans/sif/SIF-CLASSIFIER.md`
- [ ] AAAM AIS licence terms (need the current licence text)
- [ ] Related pending sessions: `08-19-26-energy-analyser.md`, `08-19-26-integration.md`, `08-19-26-simulator.md` (the control-state and simulator handoff overlap with these)
