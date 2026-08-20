---
session: OSHA Validation
status: pending
opened: 2026-08-20
---

# Session: OSHA Validation (PENDING)

## Problem

The calibration curves (S2a) are built from published literature — trauma registries, IEC standards, biomechanics studies. They're defensible but untested against real-world injury data at scale. OSHA ITA Case Detail has 690K+ events (2024) with narratives, OIICS event codes, injury nature/part codes, and outcome severity (fatality, hospitalisation, amputation, eye loss). This data can validate the severe end of the curves: for falls where we can extract height from the narrative, does the outcome distribution match what the calibration predicts? Where curves and data disagree, adjust the curves or document why the disagreement is expected (e.g., OSHA reporting bias — only severe cases reported).

## Todo

- ⬜ Design extraction prompt for OSHA narratives — per-mechanism, extract magnitude (height, voltage, speed, temperature, mass) from narrative text
- ⬜ Run extraction on stratified OSHA sample (~1,000 events per SIF-relevant mechanism) using Qwen on RunPod
- ⬜ Cross-tab: extracted magnitude × OSHA outcome code → empirical severity distribution per band
- ⬜ Compare empirical vs literature curves — where do they agree, where do they diverge?
- ⬜ Adjust curves where OSHA data provides stronger evidence than literature (unlikely for well-studied mechanisms like falls, possible for less-studied ones like pressure, caught_in)
- ⬜ Document expected divergences (OSHA reporting bias: only captures severe cases, so the mild end of curves cannot be validated)
- ⬜ Re-run STKY validation and full evaluation after any curve adjustments

## Dependencies

- ✅ S2a: Calibration curves (18 files, 87 bands)
- ✅ S2b: Band selection working (needed to place OSHA events on correct band)
- ✅ OSHA ITA Case Detail 2024 downloaded (690K+ events in data/sif/sources/osha/)
- ⬜ RunPod for Qwen extraction (~5,000-10,000 OSHA events)

## Notes

- OSHA outcome codes: 1=Fatality, 2=Hospitalisation, 3=Amputation, 4=Loss of eye. All are severe — there is no "first aid" or "no injury" in this data. The curves' lower bands cannot be validated from OSHA alone.
- OSHA OIICS event codes map to our mechanism labels via `oiics-to-mechanism-mapping.json` — this gives us mechanism without needing model inference.
- The most valuable validation is falls (code 41, 10,576 events) and struck (code 64, 54,649 events) — large sample sizes with height/weight cues in narratives.
- Electrical (code 51, 835 events) is small but well-characterised — voltage is often stated.
