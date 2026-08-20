---
session: Taxonomy & Data
status: closed
opened: 2026-08-19
closed: 2026-08-19
outcome: success

summary: >
  Acquired and structured all source data for the SIF classifier and simulator: ICD-11 Chapter 23
  taxonomy (990 codes), OSHA ITA Case Detail (1.6M rows across 2023+2024), QQ SIF events (2,747
  with human SIFp labels ingested into data/sif.duckdb), and P(death) severity scale from AIS
  mortality data. Identified 8 underrepresented high-SIF classes for synthetic balancing.

decisions:
  - what: Use ICD-11 Simple Tabulation bulk download instead of REST API
    why: The ICD-11 REST API requires OAuth2 registration and token management. The Simple Tabulation ZIP (3.3 MB) contains the full 36K-row classification hierarchy as a TSV — same data, no auth.
    result: Chapter 23 extracted (990 rows, 9 intent blocks, 48 mechanism blocks, 879 leaf codes) plus Extension Codes (2,425 rows, 18 dimensions)
  - what: P(death) from AIS empirical mortality data as the continuous severity scale
    why: The naive ordinal mapping (1,2,3,4) assumes equal spacing. AIS data shows geometric spacing — each level is a 4-6x increase in mortality. P(death) is bounded [0,1], directly interpretable, and free to use (no AAAM licence needed for published mortality rates).
    result: 5-point scale from 0.0 (no injury) to 1.0 (fatality), with serious_injury at P(death)=0.10. Corroborated by HSE appraisal costs (£1,190 to £2.185M).
  - what: Defer calibration curves and mitigation library to a dedicated sub-session (S2a)
    why: Building energy magnitude → severity metalog curves requires LLM-assisted extraction of heights/speeds/voltages from OSHA narratives, then curve fitting. This is a substantial analytical task better scoped as its own session.
    result: S2a calibration-curves session created as PENDING
  - what: Move synthetic generation and benchmark curation to S4 (mechanism classifier)
    why: Both depend on the finalised classifier label set — which OIICS→mechanism groupings become training labels. Can't generate synthetic data for labels that don't exist yet.
    result: S4 todo list updated with 3 new items
  - what: SIFP column is the consensus SIF label (I > T > ED cascade)
    why: QQ data has multiple assessors (reporter RE_, triager ED_/T_, investigator I_). SIFP aggregates them with investigator taking precedence. User confirmed this is the label to use.
    result: 5-level scale — Not SIFp (85.7%), High (9.3%), Fatal (3.2%), Very High (1.7%), Massive (0.1%)

metrics:
  icd11_taxonomy: { chapter23_rows: 990, intent_blocks: 9, mechanism_blocks: 48, leaf_codes: 879, extension_dimensions: 18, extension_codes: 2363 }
  osha_data: { year_2023_rows: 890944, year_2024_rows: 688652, total_rows: 1579596, narrative_coverage_pct: 100, oiics_coverage_pct: 90.4, unique_event_codes: 31 }
  qq_sif: { total_sif_records: 2753, matched_to_narratives: 2747, unmatched: 6, not_sifp: 2352, high: 255, fatal: 88, very_high: 48, massive: 4 }
  underrepresented_classes: { electrical: 1813, explosions: 461, fires: 481, pressure_change: 250, water_vessel: 63, structural_collapse: 29, oxygen_deficiency: 40, synthetic_needed: 1680 }
  data_sizes: { osha_zipped_mb: 207, icd11_taxonomy_mb: 16, sif_duckdb_events: 2747 }

lessons:
  - title: ICD-11 bulk download bypasses OAuth entirely
    detail: >
      The ICD-11 REST API requires OAuth2 client credentials (register, get client ID/secret,
      request token). But the Simple Tabulation ZIP on the browse page contains the exact same
      hierarchy as a TSV with Foundation URIs, codes, block IDs, titles, depth, and chapter numbers.
      No auth, no rate limits, 3.3 MB download. Always check for bulk exports before implementing
      API clients.
    tag: data
  - title: OSHA ITA Case Detail only exists from 2023 onwards
    detail: >
      The ITA Case Detail data (individual incidents with narratives and OIICS codes) started
      collection in 2023. Years 2016-2022 have only Summary Data (Form 300A — aggregate counts
      per establishment, no individual incidents). The Severe Injury Reports and fatality
      investigation data go back to 2011 but aren't available as bulk downloads with narratives.
      This caps the real training data at ~1.6M rows across 2023+2024.
    tag: data
  - title: OIICS code assignments shifted between 2023 and 2024
    detail: >
      Code 65 ("Struck, caught, or crushed in collapsing structure") had 29 records in 2023 but
      15,384 in 2024 — the BLS autocoder apparently recalibrated. Code 25 similarly jumped from
      41 to 2,529. When pooling across years, verify that code definitions are stable, not just
      counts.
    tag: data
  - title: QQ SIF CSV uses cp1252 encoding with CR line endings
    detail: >
      Same encoding pattern as the cultural graph Redactor CSVs. The csv module's DictReader
      works with cp1252 but the trailing \r on the last column value needs stripping.
      Standard pattern for Windows-origin CSVs from QQ.
    tag: data
  - title: 34% of OSHA events are auto-non-SIF by mechanism alone
    detail: >
      Overexertion (22.2%), other bodily reactions (5.6%), repetitive motions (2.7%),
      slips without fall (2.5%), and abrasion (1.2%) sum to 34.2% of all events. These
      are the auto-non-SIF gate — filtering them before Stage 2 reduces the energy
      analysis workload by a third.
    tag: methodology

artifacts:
  - scripts/sif/build_taxonomy.py
  - scripts/sif/ingest_qq.py
  - data/sif/taxonomy/icd11-chapter23-taxonomy.json
  - data/sif/taxonomy/icd11-extension-codes-taxonomy.json
  - data/sif/taxonomy/mechanism-energy-mapping.json
  - data/sif/taxonomy/oiics-to-mechanism-mapping.json
  - data/sif/taxonomy/severity-scale.json
  - data/sif/sources/osha/ITA_Case_Detail_2023.zip
  - data/sif/sources/osha/ITA_Case_Detail_2024.zip
  - data/sif.duckdb

depends_on:
  - 08-19-26-sipmath-engine.md

enables:
  - 08-19-26-calibration-curves.md (severity curves need OSHA data + sipmath engine)
  - 08-19-26-simulator.md (needs calibration curves + taxonomy)
  - 08-19-26-mechanism-classifier.md (needs OSHA training data + taxonomy labels + benchmark)
  - 08-19-26-energy-analyser.md (needs OSHA training data + severity scale)
  - 08-19-26-integration.md (needs sif.duckdb schema + QQ events)
---

# Session: Taxonomy & Data (CLOSED)

## Problem

The classifier and simulator both need structured data before any model can be trained or any calibration curve fitted: ICD-11 taxonomy labels, ICECI→Energy Wheel mapping, OSHA injury narratives, severity calibration curves (energy magnitude → metalog coefficients), mitigation effectiveness library, and a 2,000+ event benchmark set including QQ SIFp data.

## Todo

- ✅ Download ICD-11 Chapter 23 + Extension Codes (Simple Tabulation bulk download, no API auth needed)
- ✅ Build ICECI C2 → Energy Wheel mapping table (mechanism → energy type)
- ✅ Download OSHA ITA Case Detail 2023+2024 (891K + ~700K rows, 5 narrative fields, OIICS codes)
- ✅ Profile ICD-11 class frequencies in OSHA data → 8 underrepresented high-SIF classes identified
- ⏸️ Build severity calibration curves — deferred to new sub-session `calibration-curves`
- ⏸️ Build mitigation effectiveness library — deferred to `calibration-curves`
- ✅ Ingest QQ SIF CSV → `data/sif.duckdb` (2,747 events, joined with Redactor narratives)
- ⏸️ Generate synthetic training data — moved to S4 (needs finalised classifier label set)
- ⏸️ Build benchmark set — moved to S4 (curation depends on label set)
- ✅ Research AIS / insurance cost severity scales → P(death) chosen as continuous latent variable

## Dependencies

- ✅ S1: SIPmath engine (closed — `fractalaw-sipmath` crate with metalog fitting)
- ✅ ICD-11 data (used bulk Simple Tabulation download — no OAuth needed)
- ✅ OSHA ITA data download (2023: 891K rows, 2024: ~700K rows, 207MB zipped)
- ✅ QQ CSV with SIFp metadata columns (data/qq/sif/SIF.csv — 2,753 events, 5-level SIFp scale, joins to Redactor via RowSignatureII)
