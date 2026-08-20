---
session: Mitigation Library
status: closed
opened: 2026-08-20
closed: 2026-08-20
outcome: success

summary: >
  Built complete mitigation effectiveness library: 12 JSON files, 74 mitigations covering all
  SIF-relevant mechanisms plus generic controls. Each mitigation classified on a 3×3 matrix
  (exposure/chance/severity × design/engineered/managed) with p_active and effectiveness
  P10/P50/P90. STKY validation with Monte Carlo confirms all 13 hazards reduce P(SIF) when
  mitigated. Critical conceptual correction: exposure and chance controls are preventative
  (left of bow-tie), only severity controls belong in the Chance P = 1 mitigation chain.

decisions:
  - what: Classify mitigations as exposure, chance, or severity controls
    why: User identified that exposure/chance controls (guardrails, LOTO, segregation) were leaking into the severity mitigation chain. These prevent the event — they don't reduce severity once the energy reaches the person. Only severity controls belong in the Chance P = 1 framework.
    result: 45 preventative (24 exposure + 21 chance) vs 29 protective (severity). Only the 29 severity controls feed the simulator's Swiss cheese chain.
  - what: 3×3 matrix — exposure/chance/severity × design/engineered/managed
    why: User framework — the first 6 cells are preventative, last 3 are protective. Design controls can't be defeated (only fail structurally), engineered can be bypassed, managed can be forgotten. Only exposure can reach zero.
    result: Clean taxonomy that maps directly to bow-tie risk model. Design×chance cell is empty (0) — makes sense conceptually.
  - what: Design mitigations are inherent safety, not hazard elimination
    why: User correction — design doesn't mean "eliminate the hazard" (that's exposure→0). Design means the system is built to survive the energy (train crash structure, blast-resistant control room). The energy event still occurs, the receiving system absorbs it.
    result: PFD ≈ 0 for design mitigations — they can't be defeated, only fail to perform (structural failure mode).

metrics:
  mitigation_library: { files: 12, total_mitigations: 74, exposure: 24, chance: 21, severity: 29 }
  stky_mitigated: { scenarios: 13, all_reduce: true, max_reduction_pct: 99, min_reduction_pct: 19 }
  stky_examples: { fall_harness: "SIF→NON 83%", guardrail_harness: "ELEV→NON 99%", hard_hat_alone: "SIF→ELEV 19%", hard_hat_toe_board: "SIF→NON 90%" }

lessons:
  - title: Exposure/chance controls leak into severity chain if not explicitly classified
    detail: >
      The initial mitigation library included guardrails, LOTO, and segregation in the
      Swiss cheese chain alongside harnesses, hard hats, and FR clothing. These are
      fundamentally different controls — guardrails prevent the fall (exposure), harnesses
      arrest it (severity). Without explicit control_type classification, the simulator
      would double-count the benefit. User caught this during review.
    tag: architecture
  - title: Hard hat alone gives only 19% severity reduction for heavy dropped objects
    detail: >
      STKY validation showed a hard hat reduces P(SIF) from 0.51 to 0.41 for a scaffold
      tube drop — a 19% reduction. Intuitive but surprising when quantified. A hard hat
      is necessary but insufficient for heavy dropped objects. It takes the layered approach
      (toe boards + hard hat = 90% reduction) to reach NON_SIF. Good illustration of why
      the hierarchy of controls matters.
    tag: methodology
  - title: The design×chance cell of the 3×3 matrix is naturally empty
    detail: >
      No mitigations classified as design + chance. A design control that reduces
      chance-per-exposure is better described as either severity (blast-resistant structure
      absorbs energy) or exposure (SELV voltage eliminates the hazard). There's no
      "designed-in probability reduction" that isn't one of the other two.
    tag: architecture
  - title: Risk = Event P × Severity P (mitigated) over a reference period
    detail: >
      The full risk model crystallised during this session. SIF classifier does the severity
      side (Chance P = 1). Event P (exposure × chance per exposure) is the frequency side.
      The 45 preventative controls reduce Event P. The 29 protective controls reduce
      Severity P. The simulator (S3) composes both for annual SIF risk per scenario.
    tag: architecture

artifacts:
  - data/sif/calibration/mitigations/fall_protection.json
  - data/sif/calibration/mitigations/electrical_protection.json
  - data/sif/calibration/mitigations/struck_protection.json
  - data/sif/calibration/mitigations/transport_protection.json
  - data/sif/calibration/mitigations/fire_protection.json
  - data/sif/calibration/mitigations/thermal_protection.json
  - data/sif/calibration/mitigations/confined_space.json
  - data/sif/calibration/mitigations/caught_in_protection.json
  - data/sif/calibration/mitigations/chemical_protection.json
  - data/sif/calibration/mitigations/explosion_protection.json
  - data/sif/calibration/mitigations/structural_collapse_protection.json
  - data/sif/calibration/mitigations/generic_controls.json
  - scripts/sif/validate_stky_mitigated.py

depends_on:
  - 08-19-26-sipmath-engine.md
  - 08-19-26-calibration-curves.md

enables:
  - S3 simulator (consumes calibration curves + mitigation library for SIF risk modelling)
  - Annual SIF risk calculation (Event P × mitigated Severity P over reference period)
---

# Session: Mitigation Library (CLOSED)

## Problem

The SIF simulator (S3) needs a mitigation effectiveness library: for each common barrier/control, a metalog distribution representing its effectiveness at reducing P(death). The simulator applies mitigations as a Swiss cheese chain — each barrier has a probability of working (Bernoulli gate) and a severity reduction when it does (bounded metalog). The SIPmath engine already implements `chain_mitigations` and `gated_effectiveness`. This session builds the data that feeds them — published reliability and effectiveness data for common workplace controls.

## Todo

- ✅ Define mitigation data format — p_active (Bernoulli gate), effectiveness P10/P50/P90 (bounded [0,1] metalog), type (design/engineered/managed), timing (pre/post-impact)
- ✅ Fall protection — 7 mitigations (elimination, guardrail, net, harness, restraint, permit, rescue)
- ✅ Electrical — 7 mitigations (LV design, LOTO, insulated gloves, arc PPE, GFCI, permit, emergency response)
- ✅ Struck-by — 5 mitigations (toe boards, tool lanyards, hard hat, exclusion zone, lift plan)
- ✅ Transport — 5 mitigations (segregation, speed controls, reversing aids, high-vis, banksman)
- ✅ Fire — 6 mitigations (fire-resistant design, sprinklers, FR clothing, detection, escape routes, fire brigade)
- ✅ Thermal — 7 mitigations (insulation, thermal PPE, WBGT monitoring, hydration, cold clothing, PFD/immersion suit, emergency cooling)
- ✅ Confined space / breathing — 7 mitigations (elimination, monitoring, ventilation, SCBA, permit, rescue team, retrieval line)
- ✅ Caught-in — 6 mitigations (design guarding, fixed guard, interlock, light curtain, LOTO, e-stop)
- ✅ Chemical — 6 mitigations (substitution, closed system, chemical PPE, emergency shower, calcium gluconate, labelling)
- ✅ Explosion — 6 mitigations (inerting, blast-resistant design, suppression, venting, gas detection, ATEX zoning)
- ✅ Structural collapse — 6 mitigations (trench box, sloping, scaffold design, TWC, competent person, rescue)
- ✅ Generic controls — 6 mitigations (risk assessment, competent person, training, stop work, emergency plan, first aid)
- ✅ Store as JSON in data/sif/calibration/mitigations/ — 12 files, 74 mitigations
- ✅ Validate: 13 STKY hazards all reduce P(SIF) with mitigations — Monte Carlo 10K trials, 19-99% reduction range

## Dependencies

- ✅ S1: SIPmath engine (`chain_mitigations`, `gated_effectiveness`, bounded metalog)
- ✅ S2a: Calibration curves (unmitigated severity — the baseline the mitigations reduce from)
- ⬜ S3: Simulator (consumes the mitigation library — but library can be built before the simulator)

## Notes

### Mitigation taxonomy

Each mitigation has three classification dimensions:

**Type** (hierarchy of controls):
- **Design**: inherent safety baked into the system. Not a barrier that can be removed — it IS the system. Crash structure on a train, blast-resistant control room, low-voltage design, anti-climb glazing. The energy event still occurs but the receiving system is designed to survive it. PFD ≈ 0 (can't be defeated, only fail to perform — structural failure mode, not human/procedural).
- **Engineered**: physical barriers and controls added to the system. Guards, interlocks, nets, GFCI, pressure relief valves, harnesses. Can be removed, bypassed, or poorly maintained. PFD 0.001-0.0001 from LOPA.
- **Managed**: procedural and administrative controls. Permits, LOTO, RAMS, training, supervision, competent person. Human-dependent — reliability degrades with time, fatigue, complacency. PFD 0.01-0.1 from LOPA.

The reliability hierarchy is about **whether someone can defeat it**: design can't be defeated (only fail structurally), engineered can be bypassed, managed can be forgotten.

**Timing** (within Outcome P — Chance P is always 1 in SIF framework):
- **Pre-impact**: reduces energy transfer during the event. Harness arrests fall, hard hat distributes force, FR clothing prevents ignition escalation, insulation limits current path, safety net catches the person.
- **Post-impact**: reduces consequence after energy has transferred. Confined space rescue team, emergency shower/decontamination, first aid response, medical treatment, rehabilitation. Rescue speed is often the primary survival variable (Golden & Hervey for immersion, calcium gluconate for HF, SCBA rescue for confined space).

### Data model

Three dimensions per mitigation:
- **Reliability** — P(barrier works) as a Bernoulli gate. From LOPA PFD data. E.g., harness PFD 0.01 (1 in 100 failure from improper inspection/fit/connection).
- **Effectiveness** — how much it reduces P(death) when it does work. Bounded [0,1] metalog from P10/P50/P90. E.g., harness effectiveness ~0.95 (arrests the fall, residual P(death) ~0.005 from harness trauma).
- **Timing** — pre-impact or post-impact. Determines where in the Swiss cheese chain it sits.

### Sources

- Published LOPA data (CCPS) provides PFD for procedural and engineered barriers.
- EN/ISO standards provide test performance data for PPE and engineered controls.
- NFPA 70E provides arc flash PPE categories with incident energy ratings.
- The modifiers already documented in calibration files (surface, PPE, duration, rescue_time etc.) are reference notes that will be formalised into the mitigation library.
