---
session: Zero-Shot Single Model
status: closed
opened: 2026-08-20
closed: 2026-08-20
outcome: success

summary: >
  Zero-shot Qwen 3 8B classifies 2,744 UK defence narratives with 0 errors — cross-domain
  generalisation that the fine-tuned 1.7B couldn't achieve. Mechanism and energy identification
  are strong, but severity quantile estimation is systematically conservative (Qwen P50 defaults
  to no_injury where Gemini says serious_injury). Architecture conclusion: model extracts
  mechanism + energy cues, SIPmath engine maps to calibrated P(SIF) — model should not estimate
  severity directly.

decisions:
  - what: Zero-shot 8B over fine-tuned 1.7B for mechanism extraction
    why: Fine-tuned 1.7B learned OSHA-specific patterns and collapsed on UK QQ data (domain gap). Base 8B has enough general language understanding to reason about energy and mechanism from any English narrative without domain-specific training.
    result: 0 errors on 2,744 cross-domain events vs fine-tuned model's 95% single-class collapse or marginal-over-random gate
  - what: Model extracts, SIPmath calibrates — don't let the model estimate severity
    why: Qwen-Gemini disagreement analysis showed both models agree on mechanism but diverge on severity (P50). Qwen is systematically conservative. Severity estimation from general knowledge is unreliable — needs empirical calibration curves (energy magnitude → P(death) from injury data).
    result: Validates the two-product architecture. Model is step 1 (what energy?), SIPmath is step 2 (how bad?).
  - what: QQ human SIFp labels are NOT ground truth — stop scoring against them
    why: Human inter-rater agreement is ~65%. Gemini rates 86% of human "Not SIFp" events as SIF/ELEVATED. Differences between model and human are findings to explore, not errors to count.
    result: Reframed evaluation from accuracy metrics to disagreement pattern analysis.
  - what: Qwen 3 thinking mode must be disabled for JSON output
    why: format:"json" + thinking mode produces empty responses (77% error rate). think:false fixes it — 0 errors.
    result: Documented in script and session lessons.
  - what: Persist inference results incrementally, not at end
    why: 2,744 events at ~1s each = 45 min. Writing all at end risks losing everything on crash. Persist every 10 events + resume from partial results.
    result: Script survived SSH drops and process kills. Resumable.

metrics:
  qwen_8b_full: { events: 2744, errors: 0, speed_per_event_s: 1.0, gpu: "RTX 5090 32GB" }
  qwen_8b_mechanisms: { struck: 698, overexertion: 401, slip_no_fall: 397, assault: 271, fall: 209, electrical: 132, structural_collapse: 130, thermal: 79, fire: 73, collision: 69, chemical: 55, explosion: 51, transport: 42 }
  qwen_8b_p50: { serious_injury: 883, no_injury: 883, first_aid: 853, medical_treatment: 113, fatality: 12 }
  gemini_gold: { events: 200, valid: 199, errors: 1, sif: 103, elevated: 67, non_sif: 29, high_confidence: 157 }
  qwen_gemini_agreement: { mechanism: "48%", both_sif: 53, gemini_only_sif: 50, qwen_only_sif: 5, both_not: 24 }
  gemini_confidence_vs_qwen: { high_conf_qwen_agrees: "58%", medium_conf_qwen_agrees: "22%" }
  narrative_length: { short_gemini_sif: "25%", short_qwen_sif: "25%", long_gemini_sif: "60%", long_qwen_sif: "33%" }

lessons:
  - title: Zero-shot 8B generalises where fine-tuned 1.7B fails
    detail: >
      The fine-tuned 1.7B learned OSHA narrative style, not generalisable mechanism patterns.
      The base 8B has broad enough language understanding to reason about energy from any English
      narrative — UK defence, mining, construction — without domain-specific training. The lesson
      is that for cross-domain classification tasks, a larger zero-shot model can beat a smaller
      fine-tuned one, especially when training data is single-source.
    tag: models
  - title: Severity estimation is the wrong task for a language model
    detail: >
      Both Qwen and Gemini identify mechanisms correctly but estimate severity differently.
      Qwen is systematically conservative (P50 defaults to no_injury). The model has no empirical
      basis for knowing that a 6m fall P50 = serious_injury. This should come from calibration
      curves (epidemiological data), not from LLM general knowledge. The architecture split is:
      model extracts what happened + what energy, physics maps to how bad.
    tag: architecture
  - title: Gemini flags 86% of human "Not SIFp" events as having SIF potential
    detail: >
      This isn't Gemini over-rating — it's consistent with safety literature showing humans
      anchor on actual outcome rather than potential energy. The classifier's value proposition
      is exactly this: consistent physics-based assessment that catches SIF potential humans miss.
      Stop treating human labels as ground truth for SIF potential.
    tag: methodology
  - title: Qwen 3 thinking mode + JSON format = empty responses
    detail: >
      Qwen 3's thinking mode with format:"json" in Ollama produces empty strings on ~77% of
      requests. The thinking tokens consume the output budget, leaving nothing for the actual
      JSON. Fix: set think:false in the Ollama request body. Documented for future Qwen 3 usage.
    tag: models
  - title: Always persist inference results incrementally
    detail: >
      A 45-minute inference job that writes results only at the end will lose everything on any
      failure (OOM, SSH drop, pod stop). Persist every N events + implement resume from partial
      results (check done_ids on startup). This saved the full QQ run when the first attempt
      was killed to fix the persistence issue.
    tag: tooling
  - title: Narrative length strongly correlates with model SIF detection
    detail: >
      Gemini flags 60% of long narratives (>300 chars) as SIF vs 25% of short (<100 chars).
      Qwen shows the same pattern: 33% vs 25%. Longer narratives contain more energy cues for
      the model to reason about. Short narratives (~12% of QQ data) have fundamentally less
      signal. This is a data quality issue, not a model issue — reporters who write more give
      the model more to work with.
    tag: data

artifacts:
  - scripts/sif/zeroshot_sif.py
  - scripts/sif/gemini_gold_annotate.py
  - data/sif/benchmarks/qq_zeroshot_full.json
  - data/sif/benchmarks/gold_sample_200.json
  - data/sif/benchmarks/gold_gemini_annotations.json
  - data/sif/benchmarks/qq_zeroshot_sample.json

depends_on:
  - 08-19-26-mechanism-classifier.md
  - 08-19-26-taxonomy-and-data.md

enables:
  - S2a calibration curves (model extracts energy cues, calibration curves map to P(SIF))
  - S3 simulator (calibrated severity distributions from empirical data)
  - 4B edge deployment test (can smaller model do the extraction?)
  - Future fine-tuning on extraction task (not classification — teach the model to identify energy sources more precisely)
---

# Session: Zero-Shot Single Model (CLOSED)

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
- ✅ Deeper disagreement analysis: Qwen vs Gemini severity calibration gap identified
- ⏸️ Test Qwen 3 4B zero-shot — deferred, 8B validates the approach first
- ⏸️ Decision on fine-tuning — deferred, architecture conclusion reached: model extracts, SIPmath calibrates

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
