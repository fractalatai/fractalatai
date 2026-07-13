---
description: Pull and manage LRT (law-level metadata) records from sertantai. Ensures DuckDB has LRT rows before enrichment, fitness extraction, or publishing.
---

# LRT Sync

## When This Applies

- Before enriching or publishing a law that may not have an LRT record in DuckDB
- When fitness extraction or DRRP parse finds provisions in Postgres with no matching DuckDB row
- When onboarding new laws that arrived via batch import (bypassing sync-watch)
- When `taxa status` or `fitness status` reports laws missing from DuckDB

## Why This Matters

DuckDB's `legislation` table is the LRT (Law-level Registry/Taxonomy) — the master record for each law. Without an LRT row:
- Enrichment UPDATE hits zero rows (nothing to update)
- Publish has no data to send
- Fitness aggregation can't write law-level results

LRT rows are normally created by sync-watch when new LAT arrives. But any process that writes provisions to Postgres without going through sync-watch creates an orphan — provisions with no law-level metadata.

## Commands

### Pull LRT for specific laws

```bash
cargo run -p fractalaw-sync-cli -- pull-lrt \
  --tenant dev --connect tcp/localhost:7447 \
  --laws UK_ukpga_1981_69,UK_uksi_2017_1012
```

Queries sertantai's `fractalaw/@{tenant}/sertantai/lrt/{law_name}` queryable for each law. If the law already exists in DuckDB, merges (preserves enriched columns). If new, inserts.

### Pull LRT for QQ customer register

```bash
cargo run -p fractalaw-sync-cli -- pull-lrt \
  --tenant dev --connect tcp/localhost:7447 --qq
```

Reads `data/sertantai/qq-applicable-laws.csv` and pulls LRT for all laws in the register.

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--laws` | — | Comma-separated law names |
| `--qq` | false | Use QQ applicable laws CSV |
| `--timeout` | 30 | Query timeout in seconds per law |
| `--tenant` | — | Zenoh tenant (always `dev` for sertantai) |
| `--connect` | — | Zenoh router address (always `tcp/localhost:7447`) |

## Finding Orphan Laws

Laws in Postgres without DuckDB LRT records:

```sql
-- Laws with provisions but no LRT
SELECT DISTINCT split_part(section_id, ':', 1) as law_name
FROM legislation_text
WHERE law_name NOT IN (SELECT name FROM legislation);

-- Or via Python for cross-DB check:
```

```python
import psycopg2, duckdb

conn = psycopg2.connect(host='localhost', port=5433, user='fractalaw', password='fractalaw', dbname='fractalaw')
cur = conn.cursor()
cur.execute("SELECT DISTINCT law_name FROM legislation_text")
pg_laws = {r[0] for r in cur.fetchall()}
conn.close()

db = duckdb.connect('data/fractalaw.duckdb', read_only=True)
duck_laws = {r[0] for r in db.sql('SELECT name FROM legislation').fetchall()}
db.close()

missing = pg_laws - duck_laws
print(f"Orphan laws (provisions but no LRT): {len(missing)}")
for law in sorted(missing):
    print(f"  {law}")
```

## Data Flow

```
sertantai (source of truth for LRT)
  ↓ query_lrt via Zenoh queryable
  ↓ Arrow IPC response
fractalaw-sync pull-lrt
  ↓ upsert_legislation / merge_legislation
DuckDB legislation table (LRT)
  ↓ enrichment writes taxa/fitness/significance
  ↓ publish sends back to sertantai
```

## Troubleshooting

### "Arrow IPC decoding error: failed to fill whole buffer"

Sertantai returned data but it's not valid Arrow IPC. Causes:
- The law doesn't exist in sertantai's database (returns an error as raw bytes)
- Schema mismatch between sertantai's LRT format and fractalaw's decoder
- Network truncation

Check if the law exists in sertantai first. If it does, the issue is likely a schema change on sertantai's side.

### "no data"

Sertantai has no LRT record for this law. The law may not be in sertantai's database, or the queryable timed out.

### Merge vs Insert

- **New law** (not in DuckDB): `upsert_legislation` — INSERT
- **Existing law** (already in DuckDB): `merge_legislation` — UPDATE only the columns sertantai sends, preserving enriched columns (taxa, fitness, significance)

## Related

- `/publish` — publishes enriched data FROM DuckDB TO sertantai (opposite direction)
- `/customer-batch-parse` — the full enrichment pipeline, which assumes LRT exists
- `sync-watch` — the normal flow that creates LRT automatically during LAT ingestion
