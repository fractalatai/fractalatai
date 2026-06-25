---
description: Publish enriched taxa to sertantai via Zenoh — enrichment (LRT) and provisions (LAT)
---

# Skill: Publish

## When This Applies

When the user wants to publish DRRP taxa data to sertantai. Publishing sends two types of data over Zenoh:
- **Enrichment** (`/taxa/enrichment/{law_name}`) — law-level metadata from DuckDB (duty_holders, rights_holders, taxa_hash etc.)
- **Provisions** (`/taxa/provisions/{law_name}`) — per-provision taxa from Postgres/LanceDB (drrp_types, actors, extraction_method etc.)

Both are needed for a complete publish.

## Prerequisites

- Sertantai must be running with Zenoh listener on port 7447
- Use `--connect tcp/localhost:7447` (client mode) — sertantai owns the listener
- Use `--tenant dev` for the dev environment
- For provisions from Postgres: `--pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw`
- **NEVER publish benchmark laws with --force re-processing first** — check `is_benchmark` in DuckDB

## Usage

### Check what's ready to publish

```bash
fractalaw taxa status --law-file data/qq-applicable-laws.csv --summary
```

### Publish specific laws (both enrichment + provisions)

```bash
# Enrichment (law-level LRT)
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --laws UK_ukpga_1974_37

# Provisions (per-provision LAT taxa) — needs --pg for Postgres data
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --laws UK_ukpga_1974_37 --provisions --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw
```

### Publish changed laws only

```bash
# Laws where taxa_hash != published_hash
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --changed
```

### Publish all laws with taxa

```bash
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --all
```

### Publish by family

```bash
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --family "OH&S: Occupational / Personal Safety"
```

## Two-channel publish pattern

A complete publish for a set of laws requires **two commands**:

```bash
LAWS="UK_ukpga_1974_37,UK_uksi_1999_3242"

# 1. Enrichment (reads from DuckDB)
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --laws "$LAWS"

# 2. Provisions (reads from Postgres)
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --laws "$LAWS" \
  --provisions --pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw
```

Sertantai subscribes to both channels:
- `fractalaw/@dev/taxa/enrichment/*` — law-level metadata
- `fractalaw/@dev/taxa/provisions/*` — per-provision taxa

## Reconciling before publish

Before publishing a batch, verify the laws have data:

```bash
# Check which laws have enriched provisions in Postgres
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "
SELECT law_name, count(*) FILTER (WHERE extraction_method IS NOT NULL) as enriched
FROM legislation_text
WHERE law_name IN ('UK_ukpga_1974_37','UK_uksi_1999_3242')
GROUP BY law_name
HAVING count(*) FILTER (WHERE extraction_method IS NOT NULL) > 0;
"
```

Only publish laws that return rows — otherwise provisions publish sends 0 rows.

## Notes

- **Tenant matters**: `--tenant dev` for sertantai dev. CLI defaults to `local` which sertantai ignores.
- **Connection mode**: always `--connect tcp/localhost:7447` — sertantai owns the Zenoh listener. Peer mode (default) tries to bind the same port and fails.
- **Actor dictionary**: published automatically with each publish (24KB YAML).
- **Large batches**: if publishing 200+ laws, watch sertantai's disk — ElectricSQL WAL can grow rapidly. Publish in batches of 20-50 if needed.
- **Provisions without --pg**: reads from LanceDB. If provisions are only in Postgres, you'll get "0 provisions" without `--pg`.
- **Published hash**: after enrichment publish, DuckDB sets `published_hash = taxa_hash`. `--changed` won't re-publish until taxa changes again.
