---
session: SIF v0.1 Meta-Plan
status: active
opened: 2026-08-19
---

# Session: SIF v0.1 Meta-Plan (ACTIVE)

## Problem

Tracker session for the full SIF v0.1 build — two products (classifier + simulator) sharing a SIPmath/metalog engine. Stays ACTIVE through all build sessions. Design plan at `.claude/plans/sif/SIF-CLASSIFIER.md` (v0.3, reviewed 2x by Gemini).

## Todo

- ✅ S1: SIPmath engine — `fractalaw-sipmath` crate (1,055 lines, 21 tests, zero math deps)
- ✅ S2: Taxonomy & data — ICD-11, OSHA 1.6M rows, QQ 2,747 events ingested, P(death) scale
- ⬜ S2a: Calibration curves — energy × magnitude → severity metalog, mitigation effectiveness library
- ⬜ S3: SIF simulator — Product 2, CLI + WASM, energy params → severity → mitigations → residual P(SIF)
- ⬜ S4: Mechanism classifier — Stage 1, Qwen 3 0.6B fine-tune, ONNX, multi-label mechanism + object
- ⬜ S5: Energy analyser — Stage 2, Qwen 3 4B fine-tune, structured JSON extraction, severity quantiles
- ⬜ S6: Integration — DuckDB, Zenoh sync, classifier→simulator handoff, QQ pilot

## Data Roles — DO NOT CONFUSE

| Dataset | Role | Volume | Use |
|---------|------|--------|-----|
| OSHA ITA Case Detail | **TRAINING** | 1.6M rows (2023+2024) | Narratives + OIICS event codes = mechanism labels. The OIICS codes ARE the training labels via the mapping table. |
| Synthetic (Gemini) | **TRAINING (balance)** | ~1,680 events | Only for 5 underrepresented high-SIF classes (pressure, water, collapse, oxygen, fires). <30% of any class. Generated during S4. |
| QQ SIF events | **CORRELATION TEST** | 2,747 events | Human SIFp labels (subjective, ~65% inter-rater agreement). NEVER train on this. Measures classifier–human correlation and explores where/why they differ — NOT gold standard, NOT scored as accuracy. |
| OSHA (held-out split) | **VALIDATION** | ~10% of OSHA | Standard train/val split from OSHA data. Used during fine-tuning for early stopping. |

## Dependencies

- ✅ Design plan v0.3 (`.claude/plans/sif/SIF-CLASSIFIER.md`)
- ✅ Gemini review x2 (`data/code-review/sif-classifier-design-review.md`, `sif-classifier-v02-design-review.md`)
- ⬜ RunPod access for S4/S5 fine-tuning

## Build Sessions

| # | Session | Phase | Depends On | Status | Deliverable |
|---|---------|-------|------------|--------|-------------|
| S1 | `sipmath-engine` | 0 | — | **CLOSED** | `fractalaw-sipmath` crate, 1,055 lines, 21 tests. WASM deferred to S3 |
| S2 | `taxonomy-and-data` | 1 | S1 | **CLOSED** | ICD-11 taxonomy, OSHA 1.6M rows, QQ 2,747 events, P(death) scale |
| S2a | `calibration-curves` | 1.5 | S1, S2 | PENDING | Severity metalog curves per energy type × magnitude, mitigation library |
| S3 | `simulator` | 2 | S1, S2a | PENDING | CLI `sif sim` + WASM prototype. Validates calibration curves |
| S4 | `mechanism-classifier` | 3 | S2 | PENDING | Stage 1 ONNX classifier, CLI `sif classify`, F1 ≥ 0.85. Includes synthetic gen + benchmark |
| S5 | `energy-analyser` | 4 | S2, S4 | PENDING | Stage 2 Ollama extraction, full pipeline, SIF recall ≥ 0.90 |
| S6 | `integration` | 5 | S3, S5 | PENDING | DuckDB, Zenoh, classifier→simulator, QQ pilot |

## Dependency Graph

```
S1 (sipmath) ──→ S2 (data) ──→ S2a (calibration) ──→ S3 (simulator)
                    │                                        │
                    ▼                                        ▼
               S4 (mech) ──────→ S5 (energy) ──────────→ S6 (integration)
```

- S2a and S4 can run **in parallel** after S2 (different dependency chains)
- S3 is blocked on S2a (needs calibration curves)
- S4 is unblocked NOW (OSHA data + OIICS mapping = training data)
- S5 needs S4 (Stage 1 feeds Stage 2)
- S6 needs both S3 and S5
