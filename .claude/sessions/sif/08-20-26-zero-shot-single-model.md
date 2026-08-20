---
session: Zero-Shot Single Model
status: active
opened: 2026-08-20
---

# Session: Zero-Shot Single Model (ACTIVE)

## Problem

S4 showed that the two-stage mechanism classifier doesn't generalise across domains (US OSHA → UK QQ). Instead of training a separate mechanism classifier, test whether a base model (Qwen 3 4B or 8B) can do mechanism + energy analysis in a single pass via prompting alone — zero-shot or few-shot, no fine-tuning. If the base model handles 80% of the task, we focus on prompt engineering rather than fine-tuning. If it can't, we know what fine-tuning data to build.

## Todo

- ✅ Design structured output schema (mechanism, energy types, source properties, severity P10/P50/P90, reasoning)
- ✅ Write zero-shot prompt with energy wheel reference + severity scale
- ✅ Test Qwen 3 8B zero-shot on 43 QQ events (stratified sample)
- ✅ Fix Qwen 3 thinking mode (`think: false` needed for JSON output)
- ✅ Full Qwen 3 8B zero-shot on all 2,744 QQ events — 0 errors, ~1s/event on RTX 5090
- ✅ Gemini gold annotation on 200 stratified QQ events — 199/200 valid, ~8s/event
- ✅ Cross-analysis: Qwen vs Gemini vs QQ SIFp
- ⬜ Test Qwen 3 4B zero-shot (edge deployment target)
- ⬜ Deeper disagreement pattern analysis (narrative length, hazard category, report type)
- ⬜ Decision: zero-shot sufficient, or fine-tuning needed?

## Dependencies

- ✅ S2: QQ data in sif.duckdb (2,747 events with narratives + SIFp labels)
- ✅ S4: Domain gap finding — single-source training doesn't generalise
- ✅ SIPmath engine — for P(SIF) from severity quantiles
- ✅ RunPod with Ollama + Qwen 3 8B (RTX 5090, 32GB)

## Initial Results (43-event sample)

### Qwen 3 8B zero-shot — first attempt
- `format: "json"` + Qwen thinking mode = 77% empty responses (33/43 errors)
- Fix: `think: false` in Ollama request → 0 errors

### Qwen 3 8B zero-shot — with thinking disabled
- 43 events, 0 errors, ~1s per event on RTX 5090
- Mechanism distribution diverse and plausible (struck 11, electrical 4, thermal 4, overexertion 4, assault 4, fall 3, fire 3, slip 3)
- No single-class collapse (unlike fine-tuned 1.7B runs 3+4)
- Cross-domain generalisation works — UK defence narratives classified correctly without any UK training data
- Reasoning quality strong: lathe entanglement → caught_in + mechanical, scaffold fall → fall + gravity, transformer → electrical
- Model correctly identifies "no incident" narratives (scheduling decisions, permit denials → unknown/no_injury)

### Hazard category comparison (43 events)
- 44% exact match against QQ hazard→mechanism mapping
- But many "mismatches" are the model being more precise:
  - Vehicle collision → model says "struck" (correct — kinetic energy transfer), hazard says "transport"
  - Lithium battery → model says "thermal" (correct — fire risk), hazard says "electrical"
  - Machine contact → model says "struck" (correct if hit, not entangled), hazard says "caught_in"
- Model classifies by mechanism (physical process), QQ classifies by hazard (situation type) — different valid frames

### Key insight
The base 8B model handles cross-domain narratives zero-shot. The fine-tuned 1.7B failed at this because it learned OSHA-specific patterns. The 8B has enough general language understanding to reason about energy and mechanism from any English narrative.

## Gold Benchmark Strategy

200 events stratified by narrative length (short/medium/long) and SIFp (50% SIFp, 50% Not SIFp).

| Band | Events | SIFp | Not SIFp | Rationale |
|------|--------|------|----------|-----------|
| Short (<100 chars) | 20 | 10 | 10 | Test model on low-signal narratives |
| Medium (100-300) | 60 | 30 | 30 | Bulk of typical reports |
| Long (300+) | 120 | 60 | 60 | Richest narratives, best signal |

Gemini 2.5 Flash annotates all 200 with the same schema — independent "expert" classification.
Three-way comparison: Qwen 8B vs Gemini vs QQ hazard categories.
QQ SIFp labels used for exploration, NOT as ground truth.

## Full Results

### Qwen 8B zero-shot (2,744 events)

- **0 errors** across all 2,744 UK defence/industrial narratives — cross-domain generalisation works
- Inference: ~1s/event on RTX 5090, persisted every 10 events
- Mechanism distribution plausible and diverse:
  - struck 25%, overexertion 15%, slip 15%, assault 10%, fall 8%, electrical 5%, structural_collapse 5%
- P50 severity: serious_injury 32%, no_injury 32%, first_aid 31%, medical_treatment 4%, fatality 0.4%

### Gemini gold annotations (200 events)

- **199/200 valid** (1 error), confidence: 157 high, 32 medium, 10 low
- SIF potential: 103 SIF, 67 ELEVATED, 29 NON_SIF
- Mechanism: struck 45, transport 38, fall 21, unknown 15, fire 14, electrical 12

### Cross-analysis: Three-way comparison

**Qwen-Gemini mechanism agreement: 48%** on the 199 overlapping events. Different but both valid — Gemini prefers "transport" for vehicle events, Qwen prefers "struck" (energy transfer vs situation type).

**Gemini SIF_POTENTIAL vs QQ human SIFp:**

| QQ SIFp | Gemini: SIF | Gemini: ELEVATED | Gemini: NON_SIF | n |
|---------|-------------|------------------|-----------------|---|
| Fatal | 15 (75%) | 2 (10%) | 3 (15%) | 20 |
| Very High | 12 (92%) | 1 (8%) | 0 | 13 |
| High | 35 (53%) | 20 (30%) | 11 (17%) | 66 |
| Not SIFp | 41 (41%) | 44 (44%) | 14 (14%) | 99 |

**Key finding:** Gemini rates 86% of human "Not SIFp" events as SIF or ELEVATED. Either Gemini over-rates, or humans systematically under-rate SIF potential — which is what the safety literature predicts (humans anchor on actual outcome, not potential energy). This is the value of the classifier: consistent physics-based assessment vs subjective human judgement.

**Events where both models agree no incident occurred:** Scheduling decisions, permit denials, administrative reports — correctly classified as unknown/no_injury by both Qwen and Gemini, despite some being labelled "Fatal" by human raters (who likely had context not in the narrative).

### RunPod workspace

Artefacts on `/workspace/sif/` (persists):
- `/workspace/sif/output/zeroshot-qwen3-8b/predictions.json` — full 2,744 event results
- `/workspace/sif/output/zeroshot-qwen3-8b/run.log` — inference log
- `/workspace/sif/models/mechanism-run5/` — fine-tuned 1.7B (kept for comparison)
- `/workspace/sif/scripts/zeroshot_sif.py` — zero-shot script
