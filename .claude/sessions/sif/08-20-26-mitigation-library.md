---
session: Mitigation Library
status: active
opened: 2026-08-20
---

# Session: Mitigation Library (ACTIVE)

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
- ⬜ Validate: unmitigated STKY hazard at SIF → add mitigations → residual P(SIF) drops below threshold

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
