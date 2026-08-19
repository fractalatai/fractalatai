---
session: Taxonomy & Data
status: pending
opened: 2026-08-19
---

# Session: Taxonomy & Data (PENDING)

## Problem

The classifier and simulator both need structured data before any model can be trained or any calibration curve fitted: ICD-11 taxonomy labels, ICECI→Energy Wheel mapping, OSHA injury narratives, severity calibration curves (energy magnitude → metalog coefficients), mitigation effectiveness library, and a 2,000+ event benchmark set including QQ SIFp data.

## Todo

- ⬜ Download ICD-11 Chapter 23 + Extension Codes via REST API → structured label definitions
- ⬜ Build ICECI C2 → Energy Wheel mapping table (mechanism → energy type)
- ⬜ Download OSHA severe injury reports → extract narratives + OIICS labels
- ⬜ Profile ICD-11 class frequencies in OSHA data → identify underrepresented classes
- ⬜ Build severity calibration curves: energy type × magnitude → metalog coefficients (from OSHA/RIDDOR)
- ⬜ Build mitigation effectiveness library: default metalog coefficients from published reliability data
- ⬜ Ingest QQ near-miss/accident CSV subset → `data/sif.duckdb` events table with SIFp labels
- ⬜ Generate synthetic training data (Gemini) for underrepresented ICD-11 classes (<30% of mix)
- ⬜ Build benchmark set (~2,000 events, 100% real-world data for test/validation)
- ⬜ Research AIS / insurance cost severity scales → choose principled ordinal-to-continuous mapping

## Dependencies

- ⬜ S1: SIPmath engine (need metalog fitting for calibration curves)
- ⬜ ICD-11 REST API access
- ⬜ OSHA ITA data download
- ⬜ QQ CSV with SIFp metadata columns
