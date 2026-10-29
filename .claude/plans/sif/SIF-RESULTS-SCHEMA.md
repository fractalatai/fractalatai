# SIF Results File — Schema v3

What fractalaw sends back to QQ: `data/qq/sif/sif-powerbi.csv` (+ `.csv.gz`), produced by
`scripts/sif/generate_sif_export.py --csv`. Uploaded to QQ, ingested into an .xlsx that feeds the
QQ Dashboards PowerBI model (.pbip, built on synthetic data by the dashboards-be session).

Pairs with the import contract `SIF-IMPORT-SCHEMA.md`.

## File

- CSV, UTF-8, header row, one row per **report** (`row_sig`), all report types
- Cumulative: every report ever imported, minus those ECO_ETL has merged away (`vanished_at`)
- Newlines inside text are replaced with spaces (PowerBI import safety); reasoning columns last
- Empty field = no value (no model result, or the narrative didn't state the magnitude)

## Format

`.` decimal point, no thousands separators, numbers never quoted, no scientific notation, no booleans.
Text with commas/quotes uses standard CSV quoting. Fixed file name every run.

## Band lookup — `sif-bands.csv`

Written next to every export. One row per calibration band (87): `mechanism, band, band_order`
(1 = least severe), `band_description, cal_p10, cal_p50, cal_p90, p_sif, sif_classification`.
Join on (`sif_mechanism`, `sif_band`) and sort by `band_order`.

**Stability contract (ECO_ETL v0.20 consumes both files):** keep `sif-bands.csv` column names and order
fixed, and don't rename bands in `data/sif/calibration/*.json`. A renamed band leaves old results
rows showing as "not in sif-bands.csv" in ECO_ETL's ETL Status. If a band must change, regenerate the
results file in the same run and tell the QQ Dashboards side. Synthetic sample of the results file:
`data/qq/sif/sif-powerbi.sample.csv`.

## Join

`row_sig` = ECO_ETL `ReportTable.RowSignatureII` (Id×100 + exported report-type code).
`id` = EcoOnline case `Id` (a case can hold several reports).

The file carries **only the keys and the SIF analysis**. Every source column (dates, site, sector,
report types, hazards, human SIFp labels, narrative, action) comes from ECO_ETL via the join, so no
redacted text travels in this file.

## Columns (21)

| Column | Type | Definition |
|---|---|---|
| `row_sig` | integer | Key: ECO_ETL `RowSignatureII` |
| `id` | integer | EcoOnline case `Id` |

### SIF analysis (new — created by fractalaw)

Pipeline: **Pass 1** Qwen 3 8B reads the narrative → mechanism + energy. **Pass 2** Qwen picks a
magnitude band from that mechanism's calibration curve. **Calibration** turns the band's severity
quantiles into P(SIF) with a metalog fit. Severity is measured as P(death) on [0, 1]
(no_injury 0, first_aid 0.001, medical_treatment 0.005, serious_injury 0.10, fatality 1.0).
SIF = P(death) ≥ 0.10. It rates SIF **potential**: the energy present if it reached a person with no
engineered controls, regardless of the actual outcome.

| Column | Type | Definition |
|---|---|---|
| `sif_mechanism` | text, 18 values | Physical injury mechanism (normalised): transport, fall, struck, caught_in, explosion, fire, electrical, thermal, chemical, breathing, pressure, structural_collapse, assault, overexertion, slip_no_fall, abrasion, radiation_noise, animal_insect. Empty if the model gave something unmappable |
| `sif_energy_types` | text, `\|`-separated | Energy types present, normalised to this list and order: gravity, motion, mechanical, electrical, pressure, thermal, chemical, radiation, sound, biological (aliases: kinetic → motion, light → radiation, radiation_noise → radiation\|sound; anything else dropped) |
| `sif_band` | text | Magnitude band chosen within the mechanism's calibration curve, e.g. fall → standing / low / …, electrical → extra_low / low / …. 3–9 bands per mechanism, ordered least → most severe; defaults to the least severe band when the narrative lacks detail |
| `sif_band_confidence` | high / medium / low | Model's confidence in the band choice |
| `sif_p_sif` | decimal 0–1, 3 dp | **Headline measure.** Probability that the outcome reaches serious injury or worse (P(death) ≥ 0.10), from the band's calibrated severity distribution |
| `sif_classification` | SIF / ELEVATED / NON_SIF | Banded `sif_p_sif`: SIF ≥ 0.50, ELEVATED ≥ 0.10, NON_SIF < 0.10 |
| `sif_height_m` | number | Fall/drop height in metres, if stated in the narrative (non-numeric model output dropped, as for the other magnitudes) |
| `sif_voltage_v` | number | Voltage, if stated |
| `sif_speed_kmh` | number | Speed in km/h, if stated |
| `sif_temperature_c` | number | Temperature in °C, if stated |
| `sif_mass_kg` | number | Mass in kg, if stated |
| `sif_source_properties` | text | Pass 1 free-text description of the energy source |
| `sif_body_vulnerability` | text, `\|`-separated | Body region(s) exposed, normalised by keyword to: head, neck, spine, torso, limbs, respiratory, skin; `unknown` if none named |
| `sif_qwen_p50` | text | Pass 1 model's own median outcome guess (no_injury … fatality). Context only: uncalibrated and conservative; use `sif_p_sif` |
| `sif_cal_p10`, `sif_cal_p50`, `sif_cal_p90` | decimal 0–1 | The chosen band's calibrated severity quantiles in P(death) — the inputs to `sif_p_sif` (audit) |
| `sif_reasoning_pass1` | text | Pass 1 one-sentence rationale |
| `sif_reasoning_pass2` | text | Pass 2 band-choice rationale |

## Notes for the dashboard

- `ed_report_type`, `SIFP` etc. come from ReportTable via `row_sig`. Filter or slice by `ed_report_type` to compare like with like. Report types beyond Injury/Near Miss
  (Property Damage, Environmental Incident, Occupational Illness) are included from 2026-10 as an experiment.
- `SIFP` (human, ECO_ETL) vs `sif_classification` (model) is a comparison, not an accuracy score: humans tend
  to rate the actual outcome, the model rates the energy.
- A few rows have empty `sif_*` values (unmappable mechanism, or a near-empty narrative).

## History

- v3 (2026-10-29): slimmed to keys + `sif_*` (21 columns); source columns come from ECO_ETL via the join.
- v2 (2026-10-29): `event_id` → `row_sig`, added `id`. All report types.
- v1 (2026-08-20): 34 columns keyed on the legacy RowSignatureII hash, Injury/Near Miss only.
