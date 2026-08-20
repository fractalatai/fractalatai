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
- ✅ S4: Mechanism classifier — 5 runs, 0.80 F1 on OSHA, domain gap to QQ. Two-stage approach suspended.
- ✅ S4a: Zero-shot Qwen 8B — 0 errors on 2,744 events, mechanism extraction works, severity needs calibration
- ⬜ S5: Energy analyser — Stage 2, depends on S4a outcome
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
| S4 | `mechanism-classifier` | 3 | S2 | **CLOSED** | 0.80 F1 on OSHA, domain gap to QQ. Two-stage suspended. Key learnings captured. |
| S4a | `zero-shot-single-model` | 3.5 | S2, S4 | **CLOSED** | Mechanism extraction works zero-shot. Severity estimation needs calibration curves (S2a), not more model. |
| S5 | `energy-analyser` | 4 | S4a | PENDING | Depends on S4a outcome — may merge into single-model or remain Stage 2 |
| S6 | `integration` | 5 | S3, S5 | PENDING | DuckDB, Zenoh, classifier→simulator, QQ pilot |

## Dependency Graph

```
S1 (sipmath) ──→ S2 (data) ──→ S2a (calibration) ──→ S3 (simulator)
                    │                                        │
                    ▼                                        ▼
               S4 (mech) ──→ S4a (zero-shot) ──→ S5 ──→ S6 (integration)
               CLOSED         NEXT
```

- S4 CLOSED with key finding: domain gap kills single-source training
- **S4a is NEXT** — zero-shot Qwen 8B on QQ, tests single-model approach without fine-tuning
- S4a outcome determines S5 scope: if zero-shot works, S5 = prompt engineering; if not, S5 = fine-tune with energy annotations
- S2a/S3 (simulator chain) remains independent and unblocked
