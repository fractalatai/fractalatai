---
session: Band Selection
status: active
opened: 2026-08-20
---

# Session: Band Selection (ACTIVE)

## Problem

The calibration curves (S2a) are correct but the pipeline can't select the right band. Qwen Pass 1 extracts mechanism (works well — 2,740/2,744 mapped), but source_properties is free text that can't be parsed for magnitude. 92% of events default to a middle band, systematically over-rating struck (all → heavy_dropped) and under-rating thermal (all → hot_environment_moderate). A second-pass prompt, auto-generated from the calibration JSON, presents the available bands and asks the model to select. Validated with Gemini on 5 events — correct band selection with default-to-least-severe when narrative lacks detail. Needs to run on Qwen/RunPod for the full 2,744 QQ events.

## Todo

- ✅ Implement Qwen/Ollama mode in `scripts/sif/band_selector.py` — same Ollama pattern as zeroshot_sif.py, band_name validation with fallback to band_number then default-to-least-severe
- ⬜ Spin up RunPod with Qwen 3 8B, verify Ollama + model ready
- ⬜ Run Pass 2 on full 2,744 QQ events (~1s/event, ~45 min)
- ⬜ Re-run `evaluate_calibrated.py` with proper band selection — compare against default-to-middle baseline
- ⬜ Cross-tab: calibrated P(SIF) vs QQ human SIFp vs Qwen own severity — does band selection fix the over-rating?
- ⬜ Identify any mechanisms where band selection is poor — may need prompt refinement
- ⬜ Commit results, update session doc

## Dependencies

- ✅ S2a: Calibration curves (18 files, 87 bands, all feasible)
- ✅ S4a: Qwen zero-shot results (2,744 events with mechanism + source_properties)
- ✅ `band_selector.py` scaffolded with Gemini mode working, Qwen mode stub
- ✅ `evaluate_calibrated.py` baseline run (default-to-middle: 45.9% SIF)
- ⬜ RunPod with Qwen 3 8B + Ollama
