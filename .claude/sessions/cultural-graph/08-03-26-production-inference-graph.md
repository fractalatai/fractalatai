---
session: Production Inference & Graph Architecture
status: closed
opened: 2026-08-03
closed: 2026-08-04
outcome: success

summary: >
  Ran production inference on full QQ corpus (10,666 narratives, 68 sites, 5 years), loaded into DuckDB,
  built site cultural profiles with Voice/Drift/Care executive dashboard, trained multi-label classifier
  (macro F1=0.447), and delivered a complete 4-skill monthly workflow with templated reports and PowerBI export.

decisions:
  - what: DuckDB for production analytics, LadybugDB deferred to pending session
    why: 50K edges is small enough for DuckDB. Kuzudb development stopped, fork LadybugDB needs evaluation. Graph queries not yet blocking.
    result: DuckDB handles all current analytics. LadybugDB evaluation scoped as pending session.
  - what: Voice/Drift/Care as three composite executive indicators
    why: C-suite needs simplicity. 12 edge types reduced to 3 governance dimensions without losing the underlying data. Not a score — reports what the data shows.
    result: Dashboard auto-flags outlier sites with HIGH/LOW markers. Validated on 11,170 narratives.
  - what: Multi-label binary classifier instead of continuous regression
    why: User correctly identified that continuous R2 regression is not classification. Binary per-type predictions (does this narrative contain speaks-up-to? yes/no) are more useful and easier to train.
    result: Macro F1=0.447. Top types viable as pre-filters (shares-info 0.646, speaks-up 0.630). Rare types need the full SLM.
  - what: Four-skill monthly workflow
    why: User specified no ad-hoc bash scripting. Formalised as Claude skills backed by Python scripts with YAML config for normalisation.
    result: /cultural-graph-ingest, /cultural-graph-runpod, /cultural-graph-load, /cultural-graph-report. Tested end-to-end on FY2027.
  - what: Schema v0.2 and C-suite executive summary
    why: Implementation findings needed to update the dialectic working schema. C-suite needs a 1-pager not a 12-page schema document.
    result: v0.2 published with change log. Executive summary written.

metrics:
  production_inference: { narratives: 10666, valid: 10346, error_pct: 3.0, sites: 68, years: 5 }
  duckdb: { narratives: 11170, entities: 69015, edges: 68219, cultural: 25286 }
  fy2027_ingest: { records: 848, valid: 824, sites: 45 }
  classifier_multilabel: { macro_f1: 0.447, shares_info_f1: 0.646, speaks_up_f1: 0.630, normalises_f1: 0.335 }
  classifier_regression: { voice_r2: 0.45, drift_r2: 0.08, care_r2: 0.22, verdict: "too low for filtering" }
  edge_normalisation: { normalised: 107, dropped: 70, clean_pct: 99.7 }

lessons:
  - title: Regression is not classification — frame the task correctly before training
    detail: Built a continuous regressor (R2=0.26) before realising the filtering use case needs binary classification (is speaks-up-to present? yes/no). Reframing to multi-label binary classification immediately produced more useful results (F1=0.65 for top types). Cost two training runs to learn this.
    tag: methodology
  - title: MiniLM fine-tuning overfits quickly — use early stopping
    detail: R2 peaked at epoch 7-8 then declined through epoch 15. Loss kept decreasing but test R2 dropped from 0.45 to 0.30. The model memorises training examples rather than generalising. Save best checkpoint, not last.
    tag: models
  - title: Voice/Drift/Care composites solve the simplicity deficit without scoring
    detail: The dialectic explicitly forbids scoring (not a Bradley Curve replacement). Three composites report what the data shows without placing sites on a maturity scale. Execs can scan 68 sites in 30 seconds. The key insight from Gemini — the biggest risk is the simplicity deficit, not the technical capability.
    tag: methodology
  - title: Production edge type normalisation is lightweight — 99.7% clean
    detail: Expected significant hallucination cleanup. Only 107 of ~62K edges needed normalisation, 70 dropped. The fine-tuned model produces canonical edge types reliably. The YAML normalisation config is a safety net, not a critical pipeline stage.
    tag: data
  - title: FY2027 speaks-up rebounded to 19.6% after declining to 16.4%
    detail: The 5-year declining trend (19% to 16.4%) reversed in FY2027. This is either a genuine cultural improvement or a data artifact (FY2027 is partial year, 848 records). Worth monitoring but not actionable yet.
    tag: data
  - title: Formalise workflows as Claude skills, not ad-hoc scripts
    detail: User specified no on-the-fly bash scripting. Four skills with backing Python scripts and YAML config produce a repeatable, documented workflow. Each skill has a SKILL.md with usage examples, prerequisites, and post-run checklists.
    tag: tooling

artifacts:
  - scripts/cultural-graph/production_inference.py
  - scripts/cultural-graph/ingest_qa.py
  - scripts/cultural-graph/runpod_inference.py
  - scripts/cultural-graph/load_results.py
  - scripts/cultural-graph/generate_report.py
  - scripts/cultural-graph/train_classifier.py
  - scripts/cultural-graph/train_multilabel_classifier.py
  - scripts/cultural-graph/config/normalise.yaml
  - data/cultural-graph.duckdb
  - data/cultural-graph-models/classifier-multilabel/
  - data/qq/cultural-graph/outputs/reports/cultural-graph-report-all.md
  - data/qq/cultural-graph/outputs/reports/cultural-graph-report-fy2027.md
  - data/qq/cultural-graph/outputs/reports/cultural-graph-powerbi.csv
  - data/qq/cultural-graph/outputs/briefs/site-cultural-profiles-brief.md
  - data/qq/cultural-graph/outputs/briefs/cultural-graph-executive-summary.md
  - data/qq/cultural-graph/outputs/briefs/cultural-graph-model-deployment-brief.md
  - data/qq/cultural-graph/docs/methodology.md
  - data/code-review/cultural-graph-strategy-kuzu-vs-duck.md
  - /var/home/jason/Desktop/dialectics/dialectics/output/safety-culture-dialectic/cultural-graph-working-schema-v0.2.md
  - .claude/skills/cultural-graph-ingest/SKILL.md
  - .claude/skills/cultural-graph-runpod/SKILL.md
  - .claude/skills/cultural-graph-load/SKILL.md
  - .claude/skills/cultural-graph-report/SKILL.md

depends_on:
  - 07-30-26-training-data-preparation.md
  - 07-30-26-hazard-reports-emergence.md
  - 07-30-26-near-miss-reports-emergence.md
  - 07-30-26-injury-reports-emergence.md

enables:
  - Monthly cultural graph processing via 4-skill workflow
  - C-suite reporting via templated reports and PowerBI CSV
  - LadybugDB graph storage evaluation (pending session)
  - Arrow IPC signal schema design (pending session)
  - Investigation reports processing (blames/silences)
  - Human review cycle (41-narrative sample ready)
---

# Session: Production Inference & Graph Architecture (CLOSED)

## Problem

The cultural graph extraction model (Qwen 3 8B, GGUF) is trained and tested. The full QQ corpus — 10,666 narratives across 68 sites, 5 years, 9 report types — is available. Two things need to happen: (1) run production inference on the full corpus via RunPod, and (2) decide how to store and query the resulting graph. The plan (Section 8, Q3) left graph storage unresolved: "Kuzudb vs DuckDB with recursive CTEs?" This needs answering before we build the signal output, because the graph structure determines the signal format.

Key architectural question: in the fractalaw model, the edge device processes narratives and emits a *signal* (structured graph triples). The raw narrative never leaves the edge. The hub receives only the signal and builds/queries the graph. What does that signal look like as Arrow IPC, and what receives it?

## Todo

- ✅ Profile full QQ corpus — 10,666 records, 68 sites, 9 report types, 5 sectors, cp1252 encoding
- ✅ Build production inference script — `scripts/cultural-graph/production_inference.py` (multi-worker, per-record persistence, resume support)
- ✅ Run production inference on RunPod (RTX 4090) — 10,346 valid / 320 errors (97%), ~5.5 hours
- ✅ Download + NAS backup — `data/qq/cultural-graph/production-output/` (14MB, 5 yearly JSONL files)
- ✅ Gemini strategic review — hybrid DuckDB+Kuzudb, Bradley Curve positioning (`data/code-review/cultural-graph-strategy-kuzu-vs-duck.md`)
- ✅ Load production results into DuckDB — `data/cultural-graph.duckdb` (10,346 narratives, 63,351 entities, 62,606 edges)
- ⏸️ Resolve graph storage architecture (deferred — pending session: LadybugDB evaluation)
- ✅ Build reusable data cleaning module — `scripts/cultural-graph/config/normalise.yaml` + `load_results.py`
- ⏸️ Define the signal schema (deferred — pending session: Arrow IPC signal design)
- ✅ Address the "simplicity deficit" — 3-indicator dashboard (Voice/Drift/Care) + "3 sites to watch" summary
- ✅ Build cross-source site profiling — 24 sites with 50+ narratives profiled, sector comparison, temporal trajectory
- ✅ Produce site cultural profiles brief — `outputs/briefs/site-cultural-profiles-brief.md`
- ✅ Train ONNX signal classifier v1 — Voice/Drift/Care regression, R²=0.26/0.05/0.10 (too low for filtering)
- ✅ Reframe as multi-label binary classifier — 12 per-type yes/no predictions instead of 3 continuous scores
- ✅ Train multi-label classifier — macro F1=0.447, top types: shares-info 0.646, speaks-up 0.630. Downloaded + RunPod workspace cleaned.
- ✅ Write methodology document — `data/qq/cultural-graph/docs/methodology.md`
- ✅ Build three-skill monthly workflow — tested end-to-end on FY2027 (848 records)
  - `/cultural-graph-ingest` → ingest_qa.py, profile, clean, dedup, output JSONL
  - `/cultural-graph-runpod` → runpod_inference.py, 4 workers, per-record persistence, resume
  - `/cultural-graph-load` → load_results.py, normalise from YAML config, append DuckDB, site profile
- ✅ Normalisation config — `scripts/cultural-graph/config/normalise.yaml` (52 edge, 6 entity mappings)
- ✅ FY2027 loaded into DuckDB — 11,170 total narratives (was 10,346)
- ✅ Skill #4: `/cultural-graph-report` — `generate_report.py` with --template and --csv modes
  - Part A: Templated report — tested for all-years and FY2027, auto-flags outlier sites
  - Part B: Bespoke analysis — DuckDB SQL queries, documented in skill
  - Part C: PowerBI CSV — 430 rows, per-site/year/report-type with all 12 edge type counts + Voice/Drift/Care
- ✅ Production deployment brief — rolled into `cultural-graph-executive-summary.md` and `site-cultural-profiles-brief.md`

## Dependencies

- ✅ Trained model — Qwen 3 8B v2, 4-bit adapter on RunPod `/workspace/cultural-graph/`
- ✅ GGUF for edge — `data/cultural-graph-models/qwen3-8b-cultural-graph-v2-q4.gguf`
- ✅ Full QQ corpus — `data/qq/cultural-graph/qq-data/Redactor_202{2-6}.csv` (10,666 records)
- ✅ Schema — 12 cultural types + operational
- ✅ Gemini graph storage review — hybrid dual-ingest recommended. Kuzudb dev stopped; fork LadybugDB is the candidate for the graph side.
- ⏸️ Graph storage implementation (deferred — same pending session as graph architecture)
