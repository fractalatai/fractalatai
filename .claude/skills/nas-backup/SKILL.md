# Skill: NAS Backup

## When This Applies

Before any destructive operation on LanceDB or DuckDB — table rebuilds, schema migrations, bulk enrichment with `--force`, or any operation that calls `drop_table()`. Also for periodic snapshots after significant enrichment work.

## NAS Details

- **Mount**: `/mnt/nas/sertantai-data` (UGREEN DXP2800, SMB3 automount via fstab)
- **Backup dir**: `/mnt/nas/sertantai-data/data/fractalaw-backups/`
- **Space**: 5.5 TB total, typically <1% used

## What to Back Up

| Source | Size (typical) | Notes |
|--------|---------------|-------|
| `data/fractalaw.duckdb` | ~175 MB | DuckDB — LRT metadata, taxa, publish hashes |
| `data/lancedb/` | 370 MB–1.4 GB (clean) | LanceDB — provisions, embeddings, actors. Copy entire directory. |
| Postgres (pg_dump) | ~200–400 MB | PgStore — 183K+ provisions, embeddings, taxa. Hub primary store. |
| `data/drrp_classifier_v*.pkl` | ~5 MB each | Trained classifier models (gitignored, not in repo) |

**Do NOT back up `target/`** — it's 29+ GB of build artifacts.

## Procedure

### 1. Pre-flight checks

```bash
# Verify NAS is mounted
ls /mnt/nas/sertantai-data/data/

# Check local data sizes
du -sh data/fractalaw.duckdb data/lancedb/

# Check Postgres size
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "SELECT pg_size_pretty(pg_database_size('fractalaw'));"

# Check NAS free space
df -h /mnt/nas/sertantai-data/
```

### 2. Create dated backup

```bash
BACKUP_DIR=/mnt/nas/sertantai-data/data/fractalaw-backups/$(date +%Y%m%d)
mkdir -p "$BACKUP_DIR"

# DuckDB (fast, ~175 MB)
cp data/fractalaw.duckdb "$BACKUP_DIR/"

# LanceDB (copy entire directory — binary fragments, not individual files)
cp -r data/lancedb/ "$BACKUP_DIR/lancedb/"

# Postgres (pg_dump — custom format for fast restore)
PGPASSWORD=fractalaw pg_dump -h localhost -p 5433 -U fractalaw -Fc fractalaw > "$BACKUP_DIR/fractalaw.pgdump"

# Classifier models (gitignored — only live in data/)
cp data/drrp_classifier_v*.pkl "$BACKUP_DIR/" 2>/dev/null
```

### 3. Verify

```bash
# Check sizes match
du -sh "$BACKUP_DIR"/*

# Verify Postgres dump
PGPASSWORD=fractalaw pg_restore -l "$BACKUP_DIR/fractalaw.pgdump" | head -5

# Optionally verify LanceDB row count from backup
python3 -c "
import lancedb
db = lancedb.connect('$BACKUP_DIR/lancedb')
table = db.open_table('legislation_text')
print(f'Backup rows: {table.count_rows():,}')
"

# Verify Postgres row count
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "SELECT count(*) FROM legislation_text;"
```

## Compaction Before Backup

If LanceDB has grown large due to merge_insert fragment bloat, compact first to reduce backup size:

```bash
/usr/bin/python3 scripts/compact_lance.py
```

This rebuilds the table from an in-memory Arrow export (pylance not installed, so native compaction unavailable). Reduces LanceDB from potentially 8+ GB of fragments down to ~370 MB.

## Restore

```bash
# From NAS backup
BACKUP_DIR=/mnt/nas/sertantai-data/data/fractalaw-backups/YYYYMMDD
cp "$BACKUP_DIR/fractalaw.duckdb" data/
cp -r "$BACKUP_DIR/lancedb/" data/lancedb/

# Restore Postgres from pg_dump
PGPASSWORD=fractalaw pg_restore -h localhost -p 5433 -U fractalaw -d fractalaw --clean --if-exists "$BACKUP_DIR/fractalaw.pgdump"

# From Parquet backup (if LanceDB is corrupted)
/usr/bin/python3 -c "
import lancedb, pyarrow.parquet as pq
arrow = pq.read_table('backups/legislation_text_mid_enrich.parquet')
db = lancedb.connect('data/lancedb')
db.drop_table('legislation_text')
db.create_table('legislation_text', data=arrow)
print(f'Restored: {arrow.num_rows:,} rows')
"
```

## Notes

- LanceDB is binary fragments — always copy the entire `data/lancedb/` directory, never individual files
- Postgres is the hub primary store (183K+ rows) — `pg_dump -Fc` is fast and compresses well
- Postgres container: `systemctl --user start fractalaw-pg.service` (port 5433)
- Embeddings take ~9 hours to recompute on CPU (161K rows × 384-dim) — the backup is the safety net
- Multiple dated backups can coexist on the NAS (5.5 TB available)
- Local Parquet backups in `backups/` are a secondary safety net (~175 MB each)
- **NEVER write directly to NAS** — NAS block-pads binary files. Write locally first, copy to NAS
