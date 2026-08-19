---
session: SIF Simulator
status: pending
opened: 2026-08-19
---

# Session: SIF Simulator (PENDING)

## Problem

Product 2: a pre-task JHA tool where safety professionals dial in energy parameters, see a severity distribution, add outcome P mitigations, and see residual P(SIF). No SLM required — pure physics + metalog math. Validates the calibration curves before the classifier is built. CLI + WASM browser prototype.

## Todo

- ⬜ Implement `fractalaw-core::sif` types: energy wheel enum, mechanism codes, severity scale (AIS-weighted)
- ⬜ Implement `severity.rs` — energy parameters → severity metalog via calibration curves
- ⬜ Implement `mitigation.rs` — mitigation library with Bernoulli gates + copula for CCF
- ⬜ CLI command `sif sim` — interactive energy type selection, parameter input, mitigation stacking
- ⬜ Validate 13 STKY hazards produce P(SIF) > 0.50 at published thresholds
- ⬜ WASM build for browser-based simulator prototype
- ⬜ Gemini review of simulator design + calibration curve quality

## Dependencies

- ⬜ S1: SIPmath engine crate
- ⬜ S2: Calibration curves + mitigation library
