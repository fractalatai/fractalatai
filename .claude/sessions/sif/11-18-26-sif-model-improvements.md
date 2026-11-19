---
session: SIF AI model improvements
status: active
opened: 2026-11-18
---

# Session: SIF AI model improvements (ACTIVE)

## Problem

The SIF definition papers, the QQ proposal and the scenario index have fixed the definitions, and the AI model (Spongl Ltd) now lags them. Its output has no event type, energy state or control state. Its severity scale under-counts life-altering injuries at the hand and eye. It has no ballistic band. It may also conflict with the AIS licence. This session brings the model into line with the papers before the QQ pilot (proposal rec. 9, §7) and the range-index pilot.

## Todo

- [x] AIS licence check: does AAAM's generative-AI prohibition cover the model's AIS-derived bands? If so, plan an ICISS recalibration (severity rec. 6, §4.4, §9)
- [x] Body-region rule: flag hand, finger and eye bands, set P(life-altering), and combine with threat to life by taking the maximum (severity §8; proposal §5.4)
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
- [ ] QQ colleague feedback (2026-11-19): respond to QQ-FB-01 to 11 in `docs/sif/feedback/register.md`; link changes by `Feedback:` trailer
- [ ] Update `sif-definition.md` §4 and Annex B, and the proposal §5.4 limits, to match the changed model

## Dependencies

- [x] SIF definition papers: `11-17-26-sif-definition-papers.md` (closed). Definitions v1.8, controls paper v0.2, severity paper v0.2, proposal v0.1, scenario index v0.1
- [x] October QQ batch baseline: `10-29-26-qq-batch-oct.md`
- [x] Model code and calibration: `scripts/sif/zeroshot_sif.py`, `scripts/sif/band_selector.py`, `data/sif/calibration/*.json`; plan at `.claude/plans/sif/SIF-CLASSIFIER.md`
- [x] AAAM AIS licence terms (fetched 2026-11-18)
- [ ] Related pending sessions: `08-19-26-energy-analyser.md`, `08-19-26-integration.md`, `08-19-26-simulator.md` (the control-state and simulator handoff overlap with these)

## AIS licence check (2026-11-18)

**AAAM wording.** The AIS License page and the AIS FAQs, fetched on 2026-11-18, are saved in `docs/sif/lit/severity/` (gitignored).
- Licence page, under both the commercial and the healthcare/individual licence: "AAAM prohibits the use of this content in any way with generative AI, including AI training, scraping, or ingestion."
- FAQ: "If you intend to sell your product that contains AIS copyrighted material, then you will require a license from AAAM. AAAM prohibits the use of all content in any way with generative AI, including training, scraping, or ingestion."
- What is licensed is the AIS dictionary: codes and descriptors, distributed through an API. Modification is not licensed.

**How the model uses AIS.** No dictionary codes or descriptors appear anywhere in the model.
- The LLM prompt (`scripts/sif/zeroshot_sif.py`) uses its own five outcome labels: no_injury, first_aid, medical_treatment, serious_injury, fatality.
- The calibration JSONs cite Baker et al. (1974) for AIS mortality rates, the published facts behind the P(death) scale, and Dickinson et al. (2012) for AIS distributions by fall height.
- "AIS" appears only in citations, in descriptions and in one export label (`generate_sif_export.py:527`).

**Provisional view: outside the restriction today.** The model holds no AIS content, only published statistics about AIS levels.

**The restriction would bite:**
- if the LLM were asked to assign or output AIS codes or severities;
- if AIS-coded registry data or dictionary text were used in prompts, fine-tuning or retrieval;
- if an RPMI-style impairment axis were built from AIS-coded tables inside the generative pipeline.

ICISS (ICD-based) keeps the model AIS-free on those routes. Jason is reviewing; whether to confirm with AAAM is his decision.

**Decided (Jason): the model does not use AIS.** It cites published mortality statistics, some of them reported by AIS level, to anchor the serious level, and it holds no AIS content. Severity paper v0.3 rec. 6 was reworded from "uses AIS severity levels" to "Keep the AI model AIS-free". The discussion doc's "AIS-derived scale" was reworded to match. The remaining risk is RPMI, which is AIS-keyed: the impairment axis needs an AIS-free route or AAAM's permission.

## Body-region rule (2026-11-18, done — provisional values)

**Proposed design:** a `life_altering` block per candidate band, giving the region split, P(life-altering | region) and a band-level value. The model extracts `body_region_at_risk` from the narrative and falls back to the band value when no region is stated. P(serious) = max(threat to life, life-altering), with a `driven_by` flag. **Decided (Jason): region-conditional, with the band value as the fallback.**

**Evidence (agent, 2026-11-18):** saved in `docs/sif/lit/severity/body-region/` (gitignored). No unselected occupational source exists. Every value sits between emergency-department and clinic series (mostly minor) and referral series (mostly severe), and the sources define "permanent" as acuity or amputation, not by LIFE's test.

Provisional P(life-altering | region reached):

| Band and region | Value | Range |
|---|---|---|
| Light machinery → hand | 0.15 | 0.10–0.30 |
| Corrosive → eye | 0.05 | 0.02–0.10 |
| Laser 3B/4 → eye | 0.5 | 0.3–0.7 |
| Hot surface → hand | 0.25 | 0.10–0.50 |
| Friction → hand | 0.3 | 0.2–0.8 |
| Power-tool graze → hand | 0.15 | |
| Struck → eye | 0.08 | nail gun about 0.4 |
| Cutting tools → hand | 0.03 | no band yet |

**Finding:** under max() with the 0.50 and 0.10 thresholds, the rule changes few classes:
- the laser band moves to SIF;
- `power_tool_graze` moves from Not SIF to Elevated;
- nail-gun-to-eye cases become Elevated.

Most hand and eye bands stay Elevated, because a life-altering outcome is credible but not the most likely one. The life-altering gap is smaller than the severity paper implied.

BLS cases by source × nature × body part would give proper denominators, but they need a manual download (bls.gov blocks scripts).

**Decided (Jason): no panel review of Spongl Ltd model values.** The AI model is Spongl Ltd's, and Jason decides its calibration values. The calibrated panel is QQ's process, not the model's. Jason is pulling the BLS profiles (biennial 2023–24 DART numbers, or 2011–20 days-away numbers if cells are suppressed) into `docs/sif/lit/severity/body-region/` for denominators. Key point (Jason): what matters for PSIF is how seriousness and life-altering outcomes are correlated.

**BLS denominators (2023–24 biennial DART counts, all US, OIICS v3).** Files: `bls_{machinery,handtools,parts-materials,chemicals}.xlsx` in `docs/sif/lit/severity/body-region/`. Nature and body part are marginals, not crossed. Amputations are assumed to fall at the hand or finger, which gives an upper bound.

| Source | DART cases | Hand/finger share | Eye share | Amputations | Amputations ÷ hand cases |
|---|---|---|---|---|---|
| Machinery | 166,990 | 47% | 0.4% | 1,880 (1.1%) | ≤2.4% |
| Hand tools | 187,410 | 59% | 0.6% | 400 (0.2%) | ≤0.4% |
| Parts and materials | 365,130 | 25% | 8.1% | 1,220 (0.3%) | ≤1.3% |
| Chemicals | 36,510 | 11% | 25% | – | chemical burns 32% |

DART already excludes first-aid and medical-only cases, so these shares are upper bounds for the band populations. Amputation is the floor of life-altering harm, not the whole of it: fractures, tendon or nerve damage and loss of vision add to it. Even so, the agent's referral-series values (light machinery → hand 0.15, friction 0.3) look high by a factor of about 3 to 6 against an unselected base.

**Decided (Jason, 2026-11-19): BLS-adjusted values.**

| Band and region | Value | Range |
|---|---|---|
| Light machinery → hand | 0.06 | 0.03–0.15 |
| Friction → hand | 0.10 | 0.05–0.3 |
| Power-tool graze → hand | 0.05 | |
| Cutting tools → hand | 0.03 | no band yet |
| Hot surface → hand | 0.25 | 0.10–0.50 |
| Corrosive → eye | 0.05 | 0.02–0.10 |
| Laser 3B/4 → eye | 0.5 | 0.3–0.7 |
| Struck → eye | 0.08 | nail gun 0.4 |

Net class changes: the laser band moves to SIF, and nail gun to eye moves to Elevated. `power_tool_graze` stays Not SIF.

**Implemented (2026-11-19):**
- Calibration: `life_altering` blocks on 8 bands (`caught_in/light_machinery`, `abrasion/high_speed_friction`, `abrasion/power_tool_graze`, `thermal/hot_surface`, `chemical/corrosive_localised`, `radiation_noise/laser_high_power`, `struck/hand_tool`, `struck/light_dropped`). Each gives the region value, the region split, its source and a band-level fallback (split × value).
- `scripts/sif/life_altering.py`: `p_serious()` = max(threat to life, life-altering), with `driven_by`. A stated region the block does not list scores 0 on this axis.
- `generate_sif_export.py`: new columns `sif_p_threat_to_life`, `sif_p_life_altering`, `sif_driven_by`, `sif_body_region_at_risk`, `sif_region_source`. The band table gains `p_threat_to_life` and `p_life_altering_band`. The "AIS 3-4" methodology label is gone.
- `band_selector.py`: a new `body_region_at_risk` extraction field. Until the pod re-run, the region falls back to a keyword scan of the pass-1 `body_vulnerability` text. That text is coarse: pass 1 offered only "limbs", so only 4 hand/finger cases are found.
- Smoke test on the October batch (3,006 reports): 1 class change (laser to the eye: Elevated → SIF); 36 records are driven by the life-altering value with no change of class.
- Papers: `sif-definition.md` §7.2 step 5 gains the hand/eye line. Severity paper v0.3: the executive summary now says the bands stay Elevated, and rec. 2 is marked done (provisional).
- Deferred: nail gun to eye (about 0.4) needs its own band, so it goes with the ballistic/projectile band item.

## QQ colleague feedback (2026-11-19, open)

The feedback was on proposal v0.1: 11 points, logged as QQ-FB-01 to 11 in `docs/sif/feedback/register.md`.
- 2 confirm: the reading of the proposal, and support for the panel.
- 4 design: three categories; HiPo for PSIF; a plain-language front end; audiences and decisions.
- 5 gap: risk-assessment subjectivity; the golden thread; leading indicators on critical controls; triage letting events through the gaps; two sources of truth between IMR and reporting.

**Mechanism (decided, Jason):** GitHub issues were rejected because the repo is public.
- The verbatim text is in `docs/sif/research/qq-feedback/` (gitignored).
- The register holds neutral paraphrases in YAML frontmatter.
- Commits carry a `Feedback: QQ-FB-nn` trailer.
- `session_index.py` now builds `feedback` and `feedback_links` tables from the register's `changes` lists and from the git trailers. It was tested on a throwaway repo, where two trailer IDs resolved to their commit.
