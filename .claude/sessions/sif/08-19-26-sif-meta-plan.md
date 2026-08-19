---
session: SIF v0.1 Meta-Plan
status: active
opened: 2026-08-19
---

# Session: SIF v0.1 Meta-Plan (ACTIVE)

## Problem

Tracker session for the full SIF v0.1 build — two products (classifier + simulator) sharing a SIPmath/metalog engine. Stays ACTIVE through all build sessions. Design plan at `.claude/plans/sif/SIF-CLASSIFIER.md` (v0.3, reviewed 2x by Gemini).

## Todo

- ⬜ S1: SIPmath engine — `fractalaw-sipmath` crate (metalog, HDR, SIP composition, SIPmath 3.0 JSON)
- ⬜ S2: Taxonomy & data — ICD-11 taxonomy, OSHA data, calibration curves, mitigation library, benchmark set
- ⬜ S3: SIF simulator — Product 2, CLI + WASM, energy params → severity → mitigations → residual P(SIF)
- ⬜ S4: Mechanism classifier — Stage 1, Qwen 3 0.6B fine-tune, ONNX, multi-label mechanism + object
- ⬜ S5: Energy analyser — Stage 2, Qwen 3 4B fine-tune, structured JSON extraction, severity quantiles
- ⬜ S6: Integration — DuckDB, Zenoh sync, classifier→simulator handoff, QQ pilot

## Dependencies

- ✅ Design plan v0.3 (`.claude/plans/sif/SIF-CLASSIFIER.md`)
- ✅ Gemini review x2 (`data/code-review/sif-classifier-design-review.md`, `sif-classifier-v02-design-review.md`)
- ⬜ RunPod access for S4/S5 fine-tuning

## Build Sessions

| # | Session | Phase | Depends On | Status | Deliverable |
|---|---------|-------|------------|--------|-------------|
| S1 | `sipmath-engine` | 0 | — | PENDING | Publishable `fractalaw-sipmath` crate, ~500 lines, WASM |
| S2 | `taxonomy-and-data` | 1 | S1 (metalog for calibration) | PENDING | ICD-11 labels, OSHA data, calibration curves, mitigation library, 2K benchmark |
| S3 | `simulator` | 2 | S1, S2 | PENDING | CLI `sif sim` + WASM prototype, validates calibration curves |
| S4 | `mechanism-classifier` | 3 | S2 (training data) | PENDING | Stage 1 ONNX classifier, CLI `sif classify`, F1 ≥ 0.85 |
| S5 | `energy-analyser` | 4 | S2, S4 | PENDING | Stage 2 Ollama extraction, full pipeline, SIF recall ≥ 0.90 |
| S6 | `integration` | 5 | S3, S5 | PENDING | DuckDB, Zenoh, classifier→simulator, QQ pilot |

```
S1 (sipmath) ──→ S2 (data) ──→ S3 (simulator)
                    │                    │
                    ▼                    ▼
               S4 (mech) ──→ S5 (energy) ──→ S6 (integration)
```

S1 is the foundation — no dependencies, well-defined scope, standalone value.
S2 blocks everything else — data and calibration are prerequisites.
S3 and S4 can run in parallel after S2.
S5 needs both S2 (training data) and S4 (Stage 1 output feeds Stage 2).
S6 brings it all together.
