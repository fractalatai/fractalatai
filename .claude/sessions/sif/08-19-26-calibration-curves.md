---
session: Calibration Curves
status: pending
opened: 2026-08-19
---

# Session: Calibration Curves (PENDING)

## Problem

The SIF simulator (S3) needs severity calibration curves: for each energy type × magnitude range, a metalog distribution parameterised by P(death) that represents the outcome distribution. These are the lookup tables that map "6m fall onto concrete" to a severity metalog. Building them requires extracting energy magnitudes (heights, speeds, voltages) from OSHA narratives, correlating with outcomes, and fitting metalog coefficients. Also includes the mitigation effectiveness library (default metalog coefficients for common barriers from published reliability data).

## Todo

- ⬜ Extract energy magnitude cues from OSHA narratives (heights, speeds, voltages, temperatures) — LLM-assisted on a sample
- ⬜ Correlate extracted magnitudes with OSHA outcome codes → empirical severity-by-magnitude curves per energy type
- ⬜ Fit metalog coefficients for each energy type × magnitude band using P(death) scale
- ⬜ Validate calibration curves: 13 STKY hazards at published thresholds should give P(SIF) > 0.50
- ⬜ Build mitigation effectiveness library: default metalog coefficients from LOPA, fall protection, NFPA 70E data
- ⬜ Store calibration curves + mitigation library as JSON in data/sif/calibration/

## Dependencies

- ✅ S1: SIPmath engine (metalog fitting)
- ✅ S2: OSHA data downloaded and profiled, P(death) severity scale decided
- ⬜ Gemini API for LLM-assisted magnitude extraction from narratives
