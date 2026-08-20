---
session: Calibration Curves
status: closed
opened: 2026-08-19
closed: 2026-08-20
outcome: success

summary: >
  Built complete severity calibration library: 18 JSON files covering all 18 mechanism labels
  with 87 magnitude bands, all bounded [0,1] metalog feasible. Literature-based dose-response
  (not data-mining). Validated against 13 STKY hazards (all pass). ICD-11 reconciliation
  confirms zero mechanism gaps. End-to-end evaluation of Qwen QQ results through curves
  exposed band selection as the bottleneck — second-pass prompt approach validated with Gemini.

decisions:
  - what: Literature-first calibration, not OSHA data-mining
    why: OSHA only captures severe cases (fatality, hospitalisation, amputation, eye loss) — can't build full severity distribution from biased reporting data. Published trauma registry dose-response gives the complete curve.
    result: All 87 bands have published source citations. OSHA validates the severe end, doesn't define the shape.
  - what: Bounded [0,1] metalog for P(death) distributions
    why: P(death) is naturally bounded. Unbounded metalog produces infeasible fits for the heavily skewed distributions near 0 and 1 that dominate high-energy hazards.
    result: All 87 bands feasible after applying epsilon offsets (0.0001 for 0, 0.999 for 1) and ensuring no two quantiles are equal near a boundary.
  - what: Split struck/transport into separate calibration files
    why: The model produces separate mechanism labels for struck (object hits person) and transport (vehicle event). Calibration must align with model output labels.
    result: Two files with different magnitude bands — kinetic energy range vs vehicle speed.
  - what: Thermal covers 4 ICD-11 sub-mechanisms in one file
    why: ICD-11 separates hot contact, environmental heat, cold contact, environmental cold. Model outputs single "thermal" label. Sub-mechanism field on bands enables lookup.
    result: 9 bands with sub_mechanism field. Hypo/hyperthermia properly covered alongside contact burns.
  - what: Default to least-severe band when narrative lacks magnitude detail
    why: If it was a 10m fall the reporter would have said so. Short/vague narratives correlate with low-severity events (confirmed in S4a — narrative length correlates with SIF detection).
    result: Validated in 5-event Gemini test — struck→hand_tool, assault→verbal_threat instead of defaulting to middle bands.
  - what: Two-pass extraction architecture (mechanism then band selection)
    why: Single prompt can't present band options without knowing the mechanism first. Band options are mechanism-specific (87 bands across 18 mechanisms).
    result: Pass 1 (Qwen zero-shot) extracts mechanism. Pass 2 (per-mechanism prompt auto-generated from calibration JSON) selects band. Validated with Gemini — correct band selection with honest confidence.

metrics:
  calibration_library: { files: 18, bands: 87, all_feasible: true }
  stky_validation: { hazards_tested: 13, passed: 13, failed: 0 }
  icd11_reconciliation: { codes_checked: 162, mapped_to_existing: 24, not_applicable: 5, residual: 2, gaps: 0 }
  qwen_evaluation: { events: 2744, calibrated: 2740, magnitude_extracted: 217, default_band: 2523 }
  band_selection_test: { events: 5, correct_band: 5, model: "gemini-2.5-flash" }

lessons:
  - title: SPT metalog infeasible when two of three quantiles are equal near a boundary
    detail: >
      P10=P50=0.10 or P50=P90=0.999 produces infeasible bounded metalog because the
      skewness coefficient (a3) overwhelms the spread coefficient (a2) in the derivative.
      Fix: spread equal values slightly (0.0001 vs 0.0003, 0.90 vs 0.999). Discovered on
      falls high band, applied to all subsequent calibration files. Design constraint for
      all future calibration data.
    tag: methodology
  - title: Don't use Gemini for structured JSON output — thinking budget consumes output tokens
    detail: >
      Gemini 2.5 Flash with responseMimeType=application/json and thinkingConfig truncates
      output when thinkingBudget + maxOutputTokens is insufficient. The thinking tokens
      consume the budget first, leaving nothing for the JSON. Either increase maxOutputTokens
      substantially or reduce thinkingBudget. Plain text output works fine.
    tag: models
  - title: Band selection, not calibration, is the bottleneck in the pipeline
    detail: >
      Calibration curves are correct when given the right band. But 92% of Qwen events
      fell to a default middle band because source_properties was free text that couldn't
      be parsed for magnitude. This systematically over-rated struck (all went to heavy_dropped)
      and under-rated thermal (all went to hot_environment_moderate). The fix is a second-pass
      prompt that presents band options and asks the model to select, not better calibration.
    tag: architecture
  - title: Default to least severe band, not middle, when magnitude is unknown
    detail: >
      User insight: if it was a severe event, the reporter would have described the magnitude.
      Vague narratives correlate with low-severity events. Defaulting to middle band
      systematically over-rates. Defaulting to band[0] (least severe) is conservative and
      correct — the calibration should under-rate rather than over-rate when uncertain.
    tag: methodology
  - title: ICD-11 Chapter 23 has complete coverage — no mechanism gaps in our 18-label taxonomy
    detail: >
      Full reconciliation of all unintentional cause codes (162 codes) against our 18 labels.
      Automated keyword matching flagged 31 as unmapped, but manual review resolved all of them.
      Key corrections: PA80 (firearm projectile) is struck not fire, PB32 (CO) is breathing
      not chemical. Pharmaceutical/drug codes (PB20-PB27) are clinical context, not workplace SIF.
    tag: methodology
  - title: Calibration curves can be built from training data knowledge alone
    detail: >
      User corrected the approach of calling Gemini API for literature synthesis — Claude's
      training data contains the published dose-response data (AIS mortality, IEC 60479,
      Tefft 2013, etc.). No API call needed. Faster and avoids Gemini JSON truncation issues.
    tag: tooling
  - title: HF acid deserves its own calibration band
    detail: >
      Hydrofluoric acid has disproportionate lethality relative to burn area — a palm-sized
      HF burn can kill from cardiac arrest via systemic hypocalcaemia (fluoride ion binds
      calcium). No other common industrial chemical has this property. Kirkpatrick 1995
      documents 2.5% BSA concentrated HF burn as fatal.
    tag: data
  - title: Clothing ignition is THE escalation boundary for fire SIF potential
    detail: >
      Analogous to 3m for falls and 240V for electrical. Once clothing catches fire, BSA
      involvement increases rapidly (seconds), mortality jumps from ~1% to ~20%. FR clothing
      prevents this transition — the single most effective fire mitigation.
    tag: methodology

artifacts:
  - data/sif/calibration/falls_gravity.json
  - data/sif/calibration/electrical.json
  - data/sif/calibration/motion_struck.json
  - data/sif/calibration/motion_transport.json
  - data/sif/calibration/thermal.json
  - data/sif/calibration/fire.json
  - data/sif/calibration/slip_no_fall.json
  - data/sif/calibration/overexertion.json
  - data/sif/calibration/explosion.json
  - data/sif/calibration/structural_collapse.json
  - data/sif/calibration/breathing.json
  - data/sif/calibration/pressure.json
  - data/sif/calibration/radiation_noise.json
  - data/sif/calibration/chemical.json
  - data/sif/calibration/assault.json
  - data/sif/calibration/animal_insect.json
  - data/sif/calibration/abrasion.json
  - data/sif/calibration/caught_in.json
  - scripts/sif/validate_calibration.py
  - scripts/sif/validate_stky.py
  - scripts/sif/evaluate_calibrated.py
  - scripts/sif/band_selector.py

depends_on:
  - 08-19-26-sipmath-engine.md
  - 08-19-26-taxonomy-and-data.md
  - 08-20-26-zero-shot-single-model.md

enables:
  - S2b second-pass band selection (Qwen on RunPod, per-mechanism prompts from calibration JSON)
  - S2c OSHA magnitude extraction and curve validation
  - S2d mitigation effectiveness library (LOPA, fall protection, NFPA 70E)
  - S3 simulator (calibrated severity distributions now available)
---

# Session: Calibration Curves (CLOSED)

## Problem

The SIF simulator (S3) needs severity calibration curves: for each energy type × magnitude range, a metalog distribution parameterised by P(death) that represents the outcome distribution. These are the lookup tables that map "6m fall onto concrete" to a severity metalog. Building them requires extracting energy magnitudes (heights, speeds, voltages) from OSHA narratives, correlating with outcomes, and fitting metalog coefficients. Also includes the mitigation effectiveness library (default metalog coefficients for common barriers from published reliability data).

## Todo

- ✅ Define calibration data format (JSON with P10/P50/P90 quantiles, bounded [0,1] metalog, modifiers, sources, validation notes)
- ✅ Build validation script (`scripts/sif/validate_calibration.py`) — checks monotonicity, bounded metalog feasibility, roundtrip, P(SIF) classification
- ✅ Falls/gravity calibration — 6 bands (standing → extreme), literature-based dose-response from Lau 2005, Lapostolle 2005, LD50 at ~12-15m
- ✅ Electrical calibration — 5 bands (extra_low → very_high), IEC 60479 thresholds, AC/DC modifier added
- ✅ Struck calibration — 5 bands (hand_tool → heavy_object_fall), split dropped objects into light/heavy, added swinging_projectile
- ✅ Transport calibration — 3 bands (yard → highway), split from struck to match model's mechanism labels, Tefft 2013 pedestrian dose-response
- ✅ Thermal calibration — 9 bands covering 4 sub-mechanisms (hot contact, hot environment, cold contact, cold environment), ICD-11 aligned
- ✅ Fire calibration — 4 bands (flash → explosion_fireball), clothing ignition as the critical escalation boundary
- ✅ Slip/no-fall calibration — 3 bands, mostly NON_SIF, secondary hazard band for slip-near-edge scenarios
- ✅ Overexertion calibration — 3 bands, all NON_SIF, quantitatively confirms AUTO_NON_SIF classification
- ✅ Explosion calibration — 3 bands (minor → major/VCE/BLEVE), overlaps with fire/struck/structural_collapse documented
- ✅ Structural collapse calibration — 4 bands (minor → building), trench collapse as the #1 killer in this category
- ✅ Breathing calibration — 5 bands (nuisance → confined space combined), drowning/submersion, O2 deficiency, cascade rescue pattern
- ✅ Pressure calibration — 5 bands (pneumatic → water jet), tyre inflation and hydraulic injection as distinct bands
- ✅ Radiation/noise calibration — 8 bands across ionising (4), non-ionising (3), noise (1), ARS dose thresholds
- ✅ Chemical calibration — 6 bands, HF acid as its own band (disproportionate lethality), corrosive vs systemic toxic routes
- ✅ Assault calibration — 5 bands (verbal → firearm), bimodal severity distribution
- ✅ Animal/insect calibration — 6 bands, large_animal_confined as agricultural SIF equivalent of caught_between
- ✅ ICD-11 reconciliation — all Chapter 23 unintentional mechanism codes map to our 18 labels. Zero genuine gaps.
- ✅ Gitignore exceptions for data/sif/taxonomy/ and data/sif/calibration/
- ✅ Committed and pushed (29 files)
- ✅ Validate calibration curves: 13 STKY hazards at published thresholds should give P(SIF) > 0.50
- ✅ Store calibration curves as JSON in data/sif/calibration/ (committed and pushed)
- ⏸️ Extract energy magnitude cues from OSHA narratives — deferred to S2c (OSHA validation session)
- ⏸️ Correlate extracted magnitudes with OSHA outcome codes — deferred to S2c
- ⏸️ Build mitigation effectiveness library — deferred to S2d (own session, substantial scope)

## Dependencies

- ✅ S1: SIPmath engine (metalog fitting)
- ✅ S2: OSHA data downloaded and profiled, P(death) severity scale decided
- ✅ S4a: Zero-shot model validates the architecture — model extracts energy cues, calibration curves map to P(SIF)
- ⬜ Gemini API for LLM-assisted magnitude extraction from narratives

## Results

### Calibration Library v0.1

18 files, 87 bands, all bounded [0,1] metalog feasible:

| File | Mechanism | Bands | Energy Type |
|------|-----------|-------|-------------|
| falls_gravity.json | fall | 6 | gravity |
| electrical.json | electrical | 5 | electrical |
| motion_struck.json | struck | 5 | motion |
| motion_transport.json | transport | 3 | motion |
| thermal.json | thermal | 9 | thermal |
| fire.json | fire | 4 | thermal |
| slip_no_fall.json | slip_no_fall | 3 | gravity |
| overexertion.json | overexertion | 3 | biological |
| explosion.json | explosion | 3 | pressure |
| structural_collapse.json | structural_collapse | 4 | gravity |
| breathing.json | breathing | 5 | chemical |
| pressure.json | pressure | 5 | pressure |
| radiation_noise.json | radiation_noise | 8 | radiation |
| chemical.json | chemical | 6 | chemical |
| assault.json | assault | 5 | biological |
| animal_insect.json | animal_insect | 6 | biological |
| abrasion.json | abrasion | 3 | mechanical |
| caught_in.json | caught_in | 4 | mechanical |

### Metalog Feasibility Constraint

SPT 3-term bounded [0,1] metalog requires all three quantiles (P10, P50, P90) to be distinct and spread in logit space. When two quantiles are equal near a boundary (e.g., P10=P50=0.10 or P50=P90=0.999), the metalog is infeasible. Fix: spread the equal values slightly (0.0001 vs 0.0003, or 0.90 vs 0.999). This is a design constraint for all future calibration data.

### ICD-11 Reconciliation

All ICD-11 Chapter 23 unintentional mechanism codes (BlockL2-PA0 through PB6Z) map to our 18 labels. The 31 codes that appeared "unmapped" in automated keyword scan all resolve on manual review:
- 24 map to existing labels (PA74 bitten-by-person → assault, PA80 firearm projectile → struck, PB32 CO → breathing, etc.)
- 5 are non-workplace/non-acute (privation, neglect, abandonment)
- 2 are residual/unspecified catch-all codes

### Approach

**Literature-first, not data-mining.** Published dose-response data (AIS mortality, biomechanics, IEC standards, trauma registries) defines the curves. OSHA data validates the severe end but doesn't define the shape — OSHA only captures severe cases (fatality, hospitalisation, amputation, eye loss), so it can't show the full severity distribution. This is more defensible than pure OSHA data-mining.

**Iterative by energy type.** Started with falls (strongest literature), learned the metalog feasibility constraint, applied it to all subsequent types. Each type informed the next.

### Key Design Decisions

1. **Bounded [0,1] metalog**: P(death) is naturally bounded. Unbounded metalog produces infeasible fits for skewed distributions near the boundaries.
2. **Struck/transport split**: Model produces separate mechanism labels, so calibration must match. Same physics (kinetic energy) but different magnitude bands (object weight vs vehicle speed).
3. **Thermal sub-mechanisms**: ICD-11 separates hot contact, environmental heat, cold contact, environmental cold. All map to `thermal` label but need distinct bands because the dose-response is completely different (burn depth vs core temperature vs frostbite).
4. **Clothing ignition as escalation boundary**: For fire, the tipping point is whether clothing catches fire — analogous to 3m for falls and 240V for electrical.
5. **HF acid as its own band**: Disproportionate lethality (palm-sized burn can kill from cardiac arrest via hypocalcaemia). No other common industrial chemical has this property.
6. **Large animal confined**: Agricultural equivalent of caught_between — bull pins worker against gate. Same physics as vehicle pinning. Captures the #1 agricultural SIF mechanism.
7. **Non-SIF mechanisms stay non-SIF**: Overexertion (162K events/year, 0 fatalities) and abrasion produce NON_SIF across all bands. The calibration's job is to quantitatively suppress noise.
