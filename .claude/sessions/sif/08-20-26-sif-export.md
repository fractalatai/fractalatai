---
session: SIF Export
status: closed
opened: 2026-08-20
closed: 2026-08-20
outcome: success

summary: >
  Built PowerBI-ready CSV export and standard markdown report for SIF analysis results.
  Joins original QQ event records with Qwen mechanism extraction, band selection, and
  calibrated P(SIF). 2,747 rows × 34 columns, gzip compressed for upload. Repeatable
  via single script invocation for monthly data refreshes.

decisions:
  - what: Use triager's ED_ReportType instead of reporter's RE_ReportType for report breakdowns
    why: User correction — the triager reclassifies events into a cleaner taxonomy (Near Miss / Injury only, vs 8 reporter categories). Both columns included in CSV for PowerBI flexibility.
    result: Report type breakdown reduced from 8 categories to 2. Both RE and ED columns in CSV.
  - what: Strip newlines from text fields in CSV
    why: Embedded newlines break PowerBI CSV import — 677 of 2,747 narratives contained newlines.
    result: All newlines replaced with spaces. Zero import issues.
  - what: Auto-generate gzip alongside CSV
    why: User needs to upload the file — 3.2MB CSV compresses to 975KB (70% smaller).
    result: Both .csv and .csv.gz produced on every run.
  - what: SIF annotations as extension columns, not separate file
    why: The SIF analysis is an annotation dataset — extends the original records. One row per event with original QQ fields + sif_ prefixed columns. PowerBI joins on event_id.
    result: 34-column CSV with clean namespace separation (qq_ for human labels, sif_ for model output).

metrics:
  csv_output: { rows: 2747, columns: 34, size_mb: 3.2, gz_size_kb: 977, compression_pct: 70 }
  report_sections: { summary: 1, by_mechanism: 18, by_sector: 5, by_report_type: 2, cross_tab: 5, top_sif: 20 }

lessons:
  - title: SIF.csv source file uses latin-1 encoding, not UTF-8
    detail: >
      The QQ SIF.csv export contains non-breaking spaces (0xA0) that break Python's default
      UTF-8 csv reader. Must open with encoding='latin-1'. Same issue likely applies to
      other QQ exports.
    tag: data
  - title: Long text columns go last in PowerBI CSVs
    detail: >
      Narrative and action columns (up to 4,500 chars) should be the rightmost columns in
      the CSV. PowerBI column preview truncates, and having long text in the middle makes
      the data preview unusable. Same pattern used in cultural-graph-powerbi.csv.
    tag: tooling

artifacts:
  - scripts/sif/generate_sif_export.py
  - data/qq/sif/sif-powerbi.csv
  - data/qq/sif/sif-powerbi.csv.gz
  - data/qq/sif/reports/sif-report.md

depends_on:
  - 08-19-26-calibration-curves.md
  - 08-20-26-band-selection.md
  - 08-20-26-zero-shot-single-model.md

enables:
  - PowerBI SIF dashboard for QQ organisation
  - Monthly SIF analysis refresh (re-run script on new data)
---

# Session: SIF Export (CLOSED)

## Problem

We have end-to-end SIF analysis for 2,740 QQ events (Qwen mechanism extraction + band selection + calibrated P(SIF)) but no way to deliver it. The organisation needs the results as a PowerBI-ready CSV that joins the SIF annotations back to the original QQ records, plus a standard report that can be regenerated each month when new data arrives — same pattern as the cultural graph deliverables in `data/qq/cultural-graph/reports/`.

## Todo

- ✅ Build export script (`scripts/sif/generate_sif_export.py`): joins DuckDB events + Qwen Pass 1 + Pass 2 + calibrated P(SIF). 33 columns, newlines stripped for PowerBI.
- ✅ Output PowerBI CSV to `data/qq/sif/sif-powerbi.csv` — 2,747 rows, 3.2MB
- ✅ Build report script (markdown) — by mechanism, sector, report type, calibrated vs QQ cross-tab, top 20 SIF events
- ✅ Generate initial report (`data/qq/sif/reports/sif-report.md`)
- ✅ Test: CSV loads in PowerBI, all columns present, join to original records works
- ✅ Commit scripts (not data outputs)

## Dependencies

- ✅ S4a: Qwen zero-shot results (2,744 events with mechanism + energy + severity)
- ✅ S2b: Band selection results (2,740 events with band + confidence + extracted values)
- ✅ S2a: Calibration curves (P(SIF) computation)
- ✅ sif.duckdb events table (2,747 rows with original QQ fields: site, narrative, report_type, fy, sector, hazard_category)
- ✅ Cultural graph report pattern to follow (`scripts/cultural-graph/generate_report.py`, `data/qq/cultural-graph/reports/`)
