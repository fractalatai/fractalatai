---
session: Mitigation Library
status: pending
opened: 2026-08-20
---

# Session: Mitigation Library (PENDING)

## Problem

The SIF simulator (S3) needs a mitigation effectiveness library: for each common barrier/control, a metalog distribution representing its effectiveness at reducing P(death). The simulator applies mitigations as a Swiss cheese chain — each barrier has a probability of working (Bernoulli gate) and a severity reduction when it does (bounded metalog). The SIPmath engine already implements `chain_mitigations` and `gated_effectiveness`. This session builds the data that feeds them — published reliability and effectiveness data for common workplace controls.

## Todo

- ⬜ Define mitigation data format (JSON — name, mechanism applicability, effectiveness P10/P50/P90, reliability/PFD, source)
- ⬜ Fall protection mitigations — harness/lanyard (PFD from AS/NZS 1891, EN 365 inspection data), guardrails, safety nets, edge protection
- ⬜ Electrical mitigations — LOTO (PFD from LOPA data), insulated gloves, arc-rated PPE (NFPA 70E categories), GFCI, permits
- ⬜ Struck-by mitigations — hard hat (reduction factor from EN 397 test data), exclusion zones, catch nets, tool lanyards
- ⬜ Transport mitigations — barriers, speed controls, spotters, segregation, high-vis
- ⬜ Thermal/fire mitigations — FR clothing (prevents clothing ignition escalation), sprinklers, deluge, gas suppression
- ⬜ Confined space mitigations — atmospheric monitoring, SCBA standby rescue, permit-to-work, ventilation
- ⬜ Caught-in mitigations — fixed guarding, interlocked guards, light curtains, LOTO, e-stop
- ⬜ Chemical mitigations — emergency shower (decontamination speed), chemical-resistant PPE, ventilation, calcium gluconate (HF)
- ⬜ Generic mitigations — permit-to-work, competent person, method statement/RAMS, supervision
- ⬜ Validate: unmitigated STKY hazard at SIF → add appropriate mitigations → residual P(SIF) drops below threshold
- ⬜ Store as JSON in data/sif/calibration/mitigations/

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
