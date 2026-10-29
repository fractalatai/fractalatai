# SIF Import File — Schema v3 (proposed)

One joined, SIF-only file per batch, exported from ECO_ETL.xlsx (QQ Dashboards workbook).
Replaces the old two-file import (`SIF.csv` labels + the cultural graph's `Redactor_<FY>.csv`
narratives joined on `RowSignatureII`). The SIF workflow no longer reads the cultural graph's files.

Status: proposed to the QQ Dashboards session (`dashboards-be`); shipping it is that user's call
(later ECO_ETL version). Writing the CSV is a manual Save As (or small script) at QQ.

## Key — `row_sig` (one row per report); `id` groups reports into a case

`RowSignatureII` changed meaning on 2026-10-07 (ECO_ETL ETL-015):

- **Legacy** (August export, our 2,747 events): hash of `Id|~|ED_ReportType`
- **ECO_ETL**: `Id × 100 + report-type code` (e.g. `31388204` = Id 313882, Injury)

So old and new `RowSignatureII` values never match.

An EcoOnline **case** (one `Id`) can hold several **reports** of different types; ECO_ETL keeps one
row per report, keyed by `RowSignatureII` (confirmed by dashboards-be / owner). Therefore:

- **Primary key = `row_sig`** (ECO_ETL `RowSignatureII`), used as-is — not `(id, ed_report_type)`.
  It's computed from the report type *as exported*, before the Event Type override, so a later
  re-type changes `ed_report_type` but not `row_sig`.
- **`id` = case**, kept for grouping (EcoOnline's own counts are per case).
- **Merges:** when an override re-types a report to a type its case already has, ECO_ETL merges the
  two rows (ETL-020), so a `row_sig` can vanish. Treat a vanished `row_sig` as merged, not deleted.

The existing 2,747 events only hold the legacy hash. Re-key: legacy hash → (`Id`, exported type)
(see Legacy re-key) → new `row_sig = Id×100 + type code`.

## File

- **Location:** `data/qq/sif/incoming/SIF_<YYYYMMDD>.csv` (the extract date)
- **Format:** CSV, UTF-8 (no BOM), header row, RFC 4180 quoting (narratives contain commas,
  quotes, newlines — quote them, don't strip them)
- **Rows:** one per report; at-work reports only (`IsAtWork = TRUE` is the row filter);
  unsubmitted reports already excluded by ECO_ETL (v0.15)
- **Scope:** delta or cumulative — both work. Ingest keys on `row_sig`: new ones are inserted, known
  ones are updated when `date_updated` is newer (labels like `I_SIFP` arrive after investigation)
- **Redaction:** narratives and actions from ECO_ETL's `Redacted` table

## Columns (in this order — long text last)

| # | Column | ECO_ETL source | Type / values | Required |
|---|--------|----------------|---------------|----------|
| 1 | `row_sig` | `RowSignatureII` | Id×100 + exported-type code. **Primary key** (one row per report) | yes |
| 2 | `id` | `Id` | EcoOnline case Id, integer (a case can hold several reports) | yes |
| 3 | `event_date` | `RE_When` | ISO `YYYY-MM-DD` (drop the time) | yes |
| 4 | `date_updated` | `DateUpdated` | ISO `YYYY-MM-DD` (or `YYYY-MM-DDTHH:MM:SS`) | yes |
| 5 | `fy` | `AP_FY` | FY ending year, e.g. `2027` | yes |
| 6 | `site` | `R_SiteCode` | e.g. `6.54 WIN` | yes |
| 7 | `sector` | `R_OrgL1` | Org L1 | yes |
| 8 | `sub_sector` | `R_OrgL2CodefSite` | Org L2 | no |
| 9 | `report_type` | `RE_ReportType` | reporter's type: Near Miss, Injury, Hazard & Observations, … | yes |
| 10 | `ed_report_type` | `ED_ReportType` | triager's type: `Near Miss` / `Injury` | yes |
| 11 | `event_type_class` | `ED_EventTypeClass` | e.g. `Potential Injury`; may be empty | no |
| 12 | `hazard_category` | `ED_Hazards` | e.g. `Manual Handling` | no |
| 13 | `sifp` | `SIFP` | consensus label (I > T > ED): `1 Fatal`, `2 Massive`, `3 Very High`, `4 High`, `9 Not SIFp` | yes |
| 14 | `ed_sifp` | `ED_SIFP` | triager-adjusted; may be empty | no |
| 15 | `t_sifp` | `T_SIFP` | triager matrix code `A1`…`C4`; may be empty | no |
| 16 | `i_sifp` | `I_SIFP` | investigator potential; may be empty | no |
| 17 | `i_sifa` | `I_SIFA` | investigator actual, matrix code; may be empty | no |
| 18 | `narrative` | `Redacted.RE_What_Redacted` | redacted free text | yes |
| 19 | `action` | `Redacted.RE_ActionImmediate_Redacted` | redacted free text; may be empty | no |

All non-Redacted sources are `ReportTable` columns. Empty = empty field (`,,`), not `NULL`/`N/A`.

## Dropped (not used anywhere in the SIF workflow)

| Source column | Why dropped |
|---------------|-------------|
| `T_SIFP2` | never read |
| `IsAtWork` | becomes the row filter |
| Redactor `Type` / `AtWork` / `AP` | duplicates or never read |
| `R_SiteName`, `fk_ApId`, `ChangeKey`, `RedactedOn` | not needed (`date_updated` covers change tracking) |

## Example (illustrative values)

```csv
row_sig,id,event_date,date_updated,fy,site,sector,sub_sector,report_type,ed_report_type,event_type_class,hazard_category,sifp,ed_sifp,t_sifp,i_sifp,i_sifa,narrative,action
31388204,313882,2026-08-04,2026-08-11,2027,6.07 BCE,Defence,Engineering,Injury,Injury,,Manual Handling,9 Not SIFp,,B1,,,"Lifting a box, felt strain in lower back.","Reviewed lifting technique with xxxxx."
```

## Legacy re-key (resolved 2026-10-29)

Legacy `RowSignatureII` algorithm (from dashboards-be): string `Text.From(Id) & "|~|" & ED_ReportType`,
then `h = (h*31 + ord(c)) mod 2147483647` per character, starting at 0. Not collision-free across
report types (`313882|~|Near Miss` and `100618|~|Hazard & Observations` both give 428175302).

Inverted by brute force over Ids 1–10M for each type → `data/sif/legacy_rowsig_to_id.csv`
(`legacy_row_sig,row_sig,id,ed_report_type`). Result: **all 2,753 SIF.csv rows mapped**, none
ambiguous, each verified by re-hashing; 2,753 distinct new `row_sig`s.

New `row_sig = Id×100 + code`, code from ECO_ETL `ReportTypeCodes` (fixed, never renumbered;
exact-string lookup on the exported ED_ReportType, before override):

| Code | Type | Code | Type | Code | Type |
|---|---|---|---|---|---|
| 1 | Positive Observations | 4 | Injury | 7 | Environmental Incident |
| 2 | Hazard & Observations | 5 | Occupational Illness | 8 | Near Miss |
| 3 | Workers Compensation | 6 | Property Damage | 9 | Incident (Other) |

e.g. 428 Injury → `42804`; 3411 → `341104` (Injury) + `341108` (Near Miss); 20772 → `2077207`.
If 20772's override to Near Miss collided with an existing Near Miss report in its case, ETL-020 merged
it and `2077207` won't appear in ECO_ETL — the "vanished = merged" case.

- Ids run 1 → 24,860 and rise with event date (corr 0.99): they're EcoOnline's sequential report Ids.
- `887282585` → Id 20772 hashed as `Environmental Incident`, not the `Near Miss` in SIF.csv (its
  report type was overridden after hashing).
- Id 3411 has two legacy rows, both in `events`: `971229388` = `3411|~|Near Miss`, `1797205421` =
  `3411|~|Injury`. Same narrative and action; the Injury row has hazard `Slips/Trips/Falls`, the Near Miss
  row has none. Real: case 3411 holds two reports (Near Miss + Injury). Keep both rows.

Pending: spot-check of Ids against the real ECO_ETL. Only possible on the QQ company machine
(dashboards-be holds synthetic data only), so it's Jason's / the QQ user's lookup.

## History

- v3 (2026-10-29): key moved from `id` to `row_sig` (cases hold several reports); `id` kept as case.
- v2 (2026-10-29): re-keyed on `id`, added `date_updated`, ECO_ETL source mapping.
- v1 (2026-10-29): 17 columns keyed on `RowSignatureII` — superseded same day by the re-key.
- Last two-file batch: 2,747 events, `event_date` 2021-05-05 → 2026-07-31 (ingested 2026-08-20).
  The next batch starts at 2026-08-01.
