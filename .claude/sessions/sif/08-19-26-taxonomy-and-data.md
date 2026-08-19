---
session: Taxonomy & Data
status: active
opened: 2026-08-19
---

# Session: Taxonomy & Data (ACTIVE)

## Problem

The classifier and simulator both need structured data before any model can be trained or any calibration curve fitted: ICD-11 taxonomy labels, ICECI→Energy Wheel mapping, OSHA injury narratives, severity calibration curves (energy magnitude → metalog coefficients), mitigation effectiveness library, and a 2,000+ event benchmark set including QQ SIFp data.

## Todo

- ✅ Download ICD-11 Chapter 23 + Extension Codes (Simple Tabulation bulk download, no API auth needed)
- ✅ Build ICECI C2 → Energy Wheel mapping table (mechanism → energy type)
- ✅ Download OSHA ITA Case Detail 2023+2024 (891K + ~700K rows, 5 narrative fields, OIICS codes)
- ⬜ Profile ICD-11 class frequencies in OSHA data → identify underrepresented classes
- ⬜ Build severity calibration curves: energy type × magnitude → metalog coefficients (from OSHA/RIDDOR)
- ⬜ Build mitigation effectiveness library: default metalog coefficients from published reliability data
- ⬜ Ingest QQ near-miss/accident CSV subset → `data/sif.duckdb` events table with SIFp labels
- ⬜ Generate synthetic training data (Gemini) for underrepresented ICD-11 classes (<30% of mix)
- ⬜ Build benchmark set (~2,000 events, 100% real-world data for test/validation)
- ✅ Research AIS / insurance cost severity scales → P(death) chosen as continuous latent variable

## Dependencies

- ✅ S1: SIPmath engine (closed — `fractalaw-sipmath` crate with metalog fitting)
- ✅ ICD-11 data (used bulk Simple Tabulation download — no OAuth needed)
- ✅ OSHA ITA data download (2023: 891K rows, 2024: ~700K rows, 207MB zipped)
- ⬜ QQ CSV with SIFp metadata columns
