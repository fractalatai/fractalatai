# Supplementary Incident Narrative Data Sources

*Researched 2026-08-19. Target: real-world incident narratives for 3 underrepresented SIF classifier classes (breathing/O2: 40 examples, pressure: 250, explosions: 461).*

## Tier 1 — Immediate CSV Downloads

### MSHA Part 50 Accident Injuries
- **URL**: https://catalog.data.gov/dataset/msha-accident-injuries-data-set
- **Alt**: https://arlweb.msha.gov/opengovernmentdata/ogimsha.asp
- **NIOSH reformatted**: https://www.cdc.gov/niosh/mining/data/ (SPSS/Access with merged narratives)
- **Format**: Text-delimited files + 4 Accident Narrative files per quarterly release
- **Narratives**: Yes — 384-char NARRATIVE field in ACCIDENTS table
- **Records**: 2000–present (quarterly), 1983–present via Part 50 archives. Tens of thousands.
- **Relevant events**: Mine explosions (methane, coal dust), suffocations, O2 deficiency
- **Target classes**: Breathing, Explosion

### PHMSA Hazardous Materials Incident Reports (5800.1)
- **URL**: https://github.com/data-liberation-project/phmsa-hazmat-incident-reports
- **Official**: https://www.phmsa.dot.gov/hazmat/library/data-stats/incidents
- **Format**: CSV (monthly files on GitHub, ready to combine). Updated nightly.
- **Narratives**: Yes — incident descriptions, cause of failure
- **Records**: Decades of hazmat transport incidents
- **Relevant events**: Compressed gas releases, pressure vessel failures, explosions
- **Target classes**: Pressure, Explosion

### PHMSA Pipeline Incident Data
- **URL**: https://www.phmsa.dot.gov/data-and-statistics/pipeline/pipeline-incident-flagged-files
- **Format**: Downloadable CSV-like files
- **Narratives**: Yes — incident descriptions
- **Records**: 1970–present
- **Relevant events**: Gas pipeline explosions, pressure failures, ruptures
- **Target classes**: Pressure, Explosion

### OSHA Severe Injury Reports (SIR) Dashboard
- **URL**: https://www.osha.gov/severeinjury
- **Kaggle mirror**: https://www.kaggle.com/datasets/krist0phersmith/osha-severe-incident-reports
- **Format**: CSV download from dashboard
- **Narratives**: Yes — free-text event descriptions
- **Records**: 2015–present. Tens of thousands (hospitalisations, amputations, fatalities only)
- **Target classes**: All three (filter by OIICS codes)

### NFIRS (National Fire Incident Reporting System)
- **URL**: https://www.usfa.fema.gov/nfirs/access-data/
- **OpenFEMA**: https://www.fema.gov/about/openfema/data-sets/fema-usfa-nfirs-annual-data
- **Format**: CSV (post-2012), dBASE (earlier). 20-table relational database.
- **Narratives**: Remarks/narrative module (not always completed). Structured fire cause codes reliable.
- **Records**: 1980–2024. Nation's largest fire incident database.
- **Target classes**: Explosion

### EPA RMP Accident History
- **URL**: https://rmpmap.org/api-docs (API, CSV export)
- **Data Liberation Project**: https://www.data-liberation-project.org/datasets/epa-risk-management-program-database/
- **Format**: CSV via API or DLP spreadsheets. Updated through Dec 2025.
- **Narratives**: Yes — accident descriptions, chemicals released, causes
- **Records**: RMP submissions from ~12,500 facilities
- **Target classes**: Explosion, Breathing (toxic releases, O2 displacement)

### ASRS Aviation Safety Reports
- **URL**: https://asrs.arc.nasa.gov/search/database.html
- **HuggingFace**: https://huggingface.co/datasets/elihoole/asrs-aviation-reports (47,723 reports)
- **Format**: CSV (up to 10K per download). HuggingFace pre-packaged.
- **Narratives**: Yes — free-text reporter narratives (de-identified)
- **Records**: 140,000+
- **Target classes**: Pressure (cabin pressurisation), Breathing (smoke, fumes, O2)

### NTSB Aviation Accident Database
- **URL**: https://www.ntsb.gov/Pages/AviationQueryHelp.aspx
- **Format**: CSV ("Download All Text"), MDB with separate narratives table
- **Narratives**: Yes — probable cause summaries + full factual narratives in MDB
- **Records**: Tens of thousands
- **Target classes**: Pressure (pressurisation), Explosion (fuel, engine)

## Tier 2 — Worth the Effort (scraping or manual extraction)

### OSHA IMIS Accident Investigations
- **URL**: https://www.osha.gov/ords/imis/accidentsearch.html
- **Scrapers**: https://github.com/OSHADataDoor/OshaScrapy, https://github.com/jwc20/fcisapi
- **Narratives**: Detailed multi-paragraph investigation abstracts
- **Records**: 1984–present, updated daily
- **Notes**: No bulk CSV. Web search interface. Keywords: "confined space", "explosion", "asphyxiation", "oxygen deficient", "hydraulic", "pressure"

### NIOSH FACE Reports
- **URL**: https://wwwn.cdc.gov/NIOSH-FACE/
- **CPWR Excel**: https://www.cpwr.com/research/data-center/construction-face-database/ (768 construction deaths)
- **Narratives**: Multi-page investigation narratives with recommendations
- **Target classes**: Breathing (confined space fatalities are a major FACE category)

### CSB Investigations
- **URL**: https://www.csb.gov/
- **Structured wiki**: https://incidents.tychodata.com/ (116 investigations, 1998–2024)
- **Notes**: Highest-quality explosion narratives but only ~116 reports. PDF format.

### MSHA Digital Library
- **URL**: https://www.msha.gov/data-and-reports/reports
- **Notes**: 30,000+ reports dating to 1840. Full-text PDF investigation reports. Not structured CSV.

### BSEE Offshore Incidents
- **URL**: https://www.bsee.gov/stats-facts/offshore-incident-statistics
- **Data center**: https://www.data.bsee.gov/
- **Notes**: Fires, explosions, blowouts. Investigation reports are PDF.

### ARIA Database (France)
- **URL**: https://www.aria.developpement-durable.gouv.fr/?lang=en
- **Bulk download**: https://www.data.gouv.fr/datasets/mise-a-disposition-de-lintegralite-de-la-base-aria
- **Format**: CSV (UTF-8, zipped) on data.gouv.fr
- **Records**: 60,000+ industrial accidents (1992–present)
- **Notes**: Narratives in French — needs translation. Very rich for explosions and toxic releases.

## Tier 3 — Academic / Pre-packaged Datasets

- **Construction-Safety-Dataset**: https://github.com/zhenhuiou/Construction-Safety-Dataset-CSDataset (50K+ incidents)
- **safetyhub/OSHA_Acc**: https://github.com/safetyhub/OSHA_Acc (OSHA narratives for text classification)
- **LemonDa/OSHA_Dataset**: https://github.com/LemonDa/OSHA_Dataset (16K+ construction accident reports)
- **ConstructCIE**: https://arxiv.org/abs/2608.06495 (530 annotated OSHA construction summaries)
- **Kaggle OSHA 2015-2017**: https://www.kaggle.com/datasets/ruqaiyaship/osha-accident-and-injury-data-1517 (22K)
- **Kaggle Injured Workers**: https://www.kaggle.com/datasets/jboysen/injured-workers (22K)

## Tier 4 — Restricted Access

| Source | Access | Notes |
|--------|--------|-------|
| CCPS/AIChE PSID | Members only | 710+ process safety incidents. Excellent but non-public. |
| EU eMARS | EU Login + 2FA | ~1,000 Seveso Directive major accidents. Register required. |
| UK HSE RIDDOR narratives | Research access only | Contact discoveringsafety@hse.gov.uk |
| HSE Discovering Safety | Collaboration with Lloyd's Register Foundation / NaCTeM Manchester | 40+ years of investigation data |
| ATF BATS | Law enforcement only | 797,000+ bombing/arson incidents |
| DAN Diving Database | Published summaries only | Pressure change / diving incidents |
| IAEA IRS | Member state access | 12-month rolling public window |

## Acquisition Strategy

**Breathing/O2 (40 → 500)**: MSHA suffocation + OSHA IMIS "confined space" + NIOSH FACE + EPA RMP toxic releases
**Pressure (250 → 500)**: PHMSA hazmat compressed gas (GitHub CSV) + PHMSA pipeline + ASRS pressurisation
**Explosions (461 → 500)**: MSHA mine explosions + NFIRS + PHMSA. Basically solved from real data.
