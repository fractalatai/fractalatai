---
session: Integration & Validation
status: pending
opened: 2026-08-19
---

# Session: Integration & Validation (PENDING)

## Problem

Wire together the classifier pipeline and simulator into the fractalaw architecture: DuckDB persistence (`data/sif.duckdb`), Zenoh sync for results, classifier→simulator handoff (post-event screening → pre-task exploration), human review workflow for ELEVATED classifications, and QQ customer pilot.

## Todo

- ⬜ Create DuckDB schema (`events`, `classifications`, `reviews` tables)
- ⬜ Build ingest script: QQ CSV → `data/sif.duckdb` events table
- ⬜ Build load script: inference results JSONL → classifications table
- ⬜ Zenoh sync for SIF classification results (results only, not narratives)
- ⬜ Connect classifier output to simulator — pre-fill energy parameters from SLM extraction
- ⬜ Human review workflow for ELEVATED classifications
- ⬜ QQ pilot: run full pipeline on QQ incident data, compare to human SIFp labels
- ⬜ Feedback loop design: corrections → fine-tuning data → model update

## Dependencies

- ⬜ S3: SIF simulator (Product 2)
- ⬜ S5: Energy analyser (full classifier pipeline)
- ⬜ QQ incident data with SIFp labels
