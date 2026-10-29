---
session: QQ SIF batch (Oct 2026) — single-file import, row_sig re-key, ECO_ETL results
status: closed
opened: 2026-10-29
closed: 2026-10-29
outcome: success

summary: >
  Processed the second QQ SIF batch (316 reports, events 2026-06-29 → 2026-10-29) and moved the
  workflow onto files agreed with the QQ Dashboards session (dashboards-be): one SIF-only import file
  exported from ECO_ETL, and a slim results file (keys + sif_*) plus a band lookup that ECO_ETL
  v0.20 reads directly. RowSignatureII changed meaning in ECO_ETL (2026-10-07), so the 2,747 August
  events were re-keyed by inverting the legacy hash. sif.duckdb now holds 3,007 reports; the export
  carries 3,006 (one merged away in ECO_ETL).

decisions:
  - what: One joined, SIF-only import file instead of SIF.csv + the cultural graph's Redactor CSVs
    why: Jason — simpler, drops unused columns, and the SIF workflow stops reading culture files.
    result: SIF-IMPORT-SCHEMA.md v3, 19 columns; ECO_ETL writes data/qq/sif/incoming/SIF_<YYYYMMDD>.csv
  - what: Key on row_sig (ECO_ETL RowSignatureII = Id x 100 + report-type code); id kept as the case
    why: An EcoOnline case (Id) can hold several reports of different types; RowSignatureII is computed
      from the exported type before the Event Type override, so it survives re-typing. Id alone isn't unique.
    result: events.event_id = row_sig; case_id, legacy_event_id added
  - what: Recover the August events' Ids by brute-forcing the legacy hash
    why: The legacy key was a hash of Id|~|ED_ReportType — not reversible, but Ids are small (≤ ~25K) and
      types few, so trying Ids 1–10M per type is seconds of numpy. No transition work needed in ECO_ETL.
    result: all 2,753 legacy keys mapped, none ambiguous, each verified by re-hashing (data/sif/legacy_rowsig_to_id.csv)
  - what: Process all report types, not only Injury + Near Miss
    why: Jason — experiment; SIF coding is row-level and the dashboard slices by report type.
    result: Property Damage (65), Environmental Incident (41), Occupational Illness (1) added
  - what: Results file slimmed to row_sig + id + sif_* (21 columns)
    why: Jason — ECO_ETL already holds every source column; the join brings them back and no redacted
      text travels in a second file.
    result: SIF-RESULTS-SCHEMA.md v3; source columns come from ReportTable via row_sig
  - what: Normalise energy types and body regions to fixed vocabularies in the export
    why: Pass 1 free generation produced out-of-list values (kinetic, radiation_noise, light, "torso, limbs",
      eyes, …) — unstable for PowerBI slicers.
    result: energy 10 values, body 7 regions + unknown, "|"-joined in fixed order
  - what: Human SIFP stays binary in comparisons — no "4 High = ELEVATED" mapping
    why: The August analysis treated SIFP 1–4 as SIF potential and 9 as not (evaluate_calibrated.qq_is_sif,
      the 14.3% vs 10.4% comparison). The levels don't separate in the model's classes (4 High splits
      14/23/63% SIF/ELEVATED/NON_SIF, close to 9 Not SIFp's 10/19/71%).
    result: dashboard headline = human SIFp (1–4) vs model SIF (p_sif ≥ slicer, default 0.50); 5×3 matrix as detail
  - what: High energy = STKY strict, per band, with a fall straddle rule
    why: Jason. Sources give per-energy thresholds (Hallowell 2024 STKY; Hallowell 2017 >1,500 J; EEI ~680 J),
      not a per-row joule line, and magnitudes are stated in only ~3–5% of narratives. Strict reading
      flags only clear bands; fall "low" (1–3 m) counts only when sif_height_m ≥ 1.8 is stated.
    result: 42/87 bands flagged; sif_high_energy per row computed in our export (thresholds in one place,
      not duplicated in the dashboard); 411 high-energy reports
  - what: Add sif-bands.csv (band_order + calibrated quantiles) and keep it stable
    why: dashboards-be asked for least→most severe band sorting; ECO_ETL flags mechanism/band pairs
      missing from it, so band names and columns are now a contract.
    result: 87 bands, written with every export

metrics:
  import_file: { rows: 316, new: 260, updated: 56, text_changed: 5, vanished: 1, event_dates: "2026-06-29 → 2026-10-29" }
  rekey: { legacy_keys: 2753, mapped: 2753, ambiguous: 0, events_rekeyed: 2747, cases_with_two_reports: 1 }
  runpod_batch: { gpu: "RTX 4090", model: "qwen3:8b", pass1_events: 268, pass1_errors: 0, pass2_events: 264, speed_s_per_event: 1.0, wall_min: 10 }
  results: { events: 3007, exported: 3006, non_sif: 2090, elevated: 589, sif: 319, no_score: 8 }
  batch_classes: { reports: 316, sif: 39, elevated: 79, non_sif: 194, no_score: 4 }
  high_energy: { bands_flagged: 42, bands_total: 87, reports_true: 411, reports_false: 2587, blank: 8, sif_true: 317, sif_false: 2, elevated_true: 94, non_sif_true: 0 }

lessons:
  - title: A hash key with small inputs is reversible by brute force
    detail: >
      The legacy RowSignatureII was a polynomial hash of "Id|~|ED_ReportType". With Ids under ~25K
      and a handful of report types, hashing every Id 1–10M per type (vectorised) and matching against
      the stored keys recovered every Id uniquely in seconds. Check the input space before asking the
      upstream system to add a transition column.
    tag: data
  - title: Check which report-type column you're reading
    detail: >
      Id 3411's two legacy keys looked like a duplicate because both rows said "Near Miss" — that was
      RE_ReportType (reporter). The keys hash ED_ReportType (triager): one Injury, one Near Miss. Two
      real reports in one case. The reporter/triager distinction matters for keys, not just reports.
    tag: data
  - title: pkill -f matches your own SSH command line
    detail: >
      `ssh pod 'pkill -f "ollama serve"; ... ollama serve ...'` killed the remote shell, because its
      command line contains "ollama serve". Kill by a pattern that can't match the invoking command, or
      run the kill in a separate SSH call. Start servers with setsid nohup … < /dev/null so they detach.
    tag: tooling
  - title: Wait loops should match terminal markers only
    detail: >
      A wait loop grepping "Error" stopped at Pass 1's "Errors: 0/268" summary. Match an explicit
      BATCH_DONE marker or Traceback, not words that appear in normal summaries.
    tag: tooling
  - title: Pod scripts must not be hand-patched copies
    detail: >
      The August band run used a sed-patched band_selector.py on the pod that read narratives from a JSON
      — never committed, so the repo version silently fell back to 100-char previews without DuckDB.
      Paths and narrative sources are now CLI options; batches upload the repo version into their own folder.
    tag: tooling

artifacts:
  - .claude/plans/sif/SIF-IMPORT-SCHEMA.md
  - .claude/plans/sif/SIF-RESULTS-SCHEMA.md
  - scripts/sif/ingest_qq.py
  - scripts/sif/sif_schema.sql
  - scripts/sif/merge_batch_results.py
  - scripts/sif/generate_sif_export.py
  - scripts/sif/zeroshot_sif.py
  - scripts/sif/band_selector.py
  - scripts/migrations/sif_rekey_rowsig.py
  - data/sif/legacy_rowsig_to_id.csv
  - data/sif/batches/20261029/
  - data/qq/sif/sif-powerbi.csv
  - data/qq/sif/sif-bands.csv
  - data/qq/sif/sif-powerbi.sample.csv

depends_on:
  - 08-20-26-sif-export.md
  - 08-20-26-band-selection.md
  - 08-20-26-zero-shot-single-model.md
---

# Session: QQ SIF batch (Oct 2026) (CLOSED)

## Problem

A new batch of QQ records needed the SIF workflow, which hadn't run since August. The August
import joined two files (SIF.csv labels + the cultural graph's Redactor narratives) on a
RowSignatureII hash that ECO_ETL has since redefined. The import needed replacing with one SIF-only file, the
existing 2,747 events had to be re-keyed, the new reports run through Qwen on RunPod, and the
results handed back in a shape the QQ Dashboards session (dashboards-be) can build ECO_ETL and the
PowerBI dashboard on.

## Todo

- [x] Recap: import shape (SIF.csv + Redactor) and processed range (2021-05-05 → 2026-07-31)
- [x] Import schema with dashboards-be: v1 (17 cols) → v2 (re-key on Id) → v3 (row_sig key, 19 cols)
- [x] Invert the legacy RowSignatureII hash → Id for all 2,753 legacy keys; add new row_sig
- [x] Validate SIF_20261029.csv (first landing header-only; second 316 rows, valid)
- [x] Back up sif.duckdb + result JSON; re-key events and results to row_sig (`93f78dd`)
- [x] Rewrite ingest for the single file (insert / update on newer date_updated / vanished); ingest batch
- [x] Scope: all report types (Jason)
- [x] RunPod batch in /workspace/sif/batches/20261029: pilot 10, then Pass 1 (268) + Pass 2 (264), 0 errors
- [x] Merge batch results; regenerate export
- [x] Results schema to dashboards-be: v2 (35 cols) → v3 slim (keys + sif_*, 21 cols); answer their 6 questions
- [x] sif-bands.csv band lookup; vocab normalisation; synthetic sample; bands stability contract
- [x] Commit code (`bd36607`); confirm pod artefacts are on /workspace before stop
- [x] Answer dashboards-be: SIFP mapping (binary, no 4 High = ELEVATED); headline SIF = p_sif ≥ 0.50
- [x] High-energy flag, STKY strict + fall 1.8 m rule (Jason): sif-bands `high_energy`, results `sif_high_energy` (`a739bf6`)

## Import file and re-key

- `SIF-IMPORT-SCHEMA.md` v3: 19 columns from ECO_ETL ReportTable + Redacted. At-work reports only,
  UTF-8, ISO dates, keyed on `row_sig`, `id` = case, `date_updated` drives updates.
- ECO_ETL RowSignatureII = Id × 100 + report-type code (1 Positive Observations, 2 Hazard &
  Observations, 3 Workers Compensation, 4 Injury, 5 Occupational Illness, 6 Property Damage,
  7 Environmental Incident, 8 Near Miss, 9 Incident (Other)), computed from the exported type before
  the Event Type override. When an override collides with an existing type in the case, ECO_ETL merges
  the two reports (ETL-020), so a vanished row_sig means merged, not deleted.
- Legacy hash: `h = (h*31 + ord(c)) mod 2147483647` over `"{Id}|~|{ED_ReportType}"`. Ids recovered
  1 → 24,860, rising with event date (corr 0.99). 887282585 → Id 20772 hashed as Environmental Incident
  (re-typed to Near Miss later). Id 3411 = two real reports (Injury + Near Miss).
- Re-key validated on real data: the 56 updated reports from the new file all matched on event date
  (3 site recodes).

## Ingest (SIF_20261029.csv)

- 316 rows: 260 new, 56 updated (5 with changed narrative/action), 1 vanished (`2371008`, Nuneaton
  station near miss, 2026-07-15. No report from its case is in the file, so it's not a merge. Possibly no
  longer at-work, or withdrawn). One row (`2527704`) has no site.
- sif.duckdb: 3,007 events, 2021-05-05 → 2026-10-29. By ed_report_type: Near Miss 2,052,
  Injury 848, Property Damage 65, Environmental Incident 41, Occupational Illness 1.

## RunPod batch

- RTX 4090 pod, Ollama 0.40.13, qwen3:8b. Everything under `/workspace/sif/batches/20261029/`
  (data, scripts, output, logs). The August outputs were not touched.
- Input: 268 events (260 new + 5 text-changed + 3 old events never classified).
- Pass 1: 268, 0 errors, 1.0 s/event after an 87 s model load. Pass 2: 264 (4 unmappable mechanisms);
  confidence high 145 / medium 111 / low 8.
- Container-local only: Ollama binary + model weights (4.9 GB in /root/.ollama) — re-pull on next pod.

## Results and hand-off

- `sif-powerbi.csv`: 3,006 rows × 22 columns (row_sig, id, 20 sif_*). NON_SIF 2,090 / ELEVATED 589 /
  SIF 319 / no score 8. New batch: SIF 39, ELEVATED 79, NON_SIF 194, no score 4.
- `sif-bands.csv`: 87 rows (mechanism, band, band_order, band_description, cal_p10/p50/p90, p_sif,
  sif_classification).
- `sif_high_energy` (22nd column, appended) and `high_energy` in sif-bands.csv — see SIF-RESULTS-SCHEMA.md
  v3.1 for the definition. The 2 SIF reports not flagged are thermal/hot_environment_extreme (borderline,
  excluded by the strict reading).
- Dashboard comparison: human SIFp (SIFP 1–4) vs model SIF; on the 3,006 rows, Fatal 89 → 25% SIF /
  18% ELEVATED / 56% NON_SIF; 4 High 265 → 14/23/63%; 9 Not SIFp 2,600 → 10/19/71%.
- ECO_ETL v0.20 (dashboards-be) reads both fixed-name files from its inputs and joins on row_sig.
  ETL Status reports unmatched row_sigs, rows without analysis, and band pairs missing from sif-bands.csv.
- Next batch: land `SIF_<YYYYMMDD>.csv` in `data/qq/sif/incoming/` → `ingest_qq.py` → build batch input
  (events without results or with narrative_changed_at) → RunPod batch in its own
  `/workspace/sif/batches/<date>/` → `merge_batch_results.py` → `generate_sif_export.py --csv --report`.
  Batch input building is still an inline snippet. Worth a `--build-batch` script before the next run.
