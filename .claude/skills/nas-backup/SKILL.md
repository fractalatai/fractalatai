# Skill: NAS Backup

## When This Applies

Before any destructive operation on LanceDB or DuckDB — table rebuilds, schema migrations, bulk enrichment with `--force`, or any operation that calls `drop_table()`. Also for periodic snapshots after significant enrichment work.

## Backup Modes

### Quick mode (end-of-session, pipeline work)

Only backs up data that changes during normal pipeline work. Takes ~30 seconds.

| Source | Size | Changes when |
|--------|------|-------------|
| Postgres (pg_dump) | ~400 MB | Every parse/classify/reconcile/backfill |
| DuckDB | ~200 MB | Every enrich/publish |

### Full mode (before destructive ops, weekly, after retraining)

Backs up everything. Takes ~5 minutes.

| Source | Size | Changes when |
|--------|------|-------------|
| Postgres (pg_dump) | ~400 MB | Every pipeline run |
| DuckDB | ~200 MB | Every enrich/publish |
| LanceDB | 370 MB–1.4 GB | New law ingestion, re-embed |
| Classifiers | ~60 KB | Retrain classifier |
| SLM adapter | ~125 MB | Retrain SLM on RunPod |
| GGUF model | ~2.4 GB | Retrain SLM on RunPod |

**Do NOT back up `target/`** — it's 29+ GB of build artifacts.

## NAS Details

- **Mount**: `/mnt/nas/sertantai-data` (UGREEN DXP2800, SMB3 automount via fstab)
- **Backup dir**: `/mnt/nas/sertantai-data/data/fractalaw-backups/`
- **Space**: 5.5 TB total, typically <1% used

## Quick Backup

```bash
BACKUP_DIR=/mnt/nas/sertantai-data/data/fractalaw-backups/$(date +%Y%m%d)
mkdir -p "$BACKUP_DIR"

# Postgres (the primary store — provision_actors, legislation_text, gold_benchmarks)
PGPASSWORD=fractalaw pg_dump -h localhost -p 5433 -U fractalaw -Fc fractalaw > "$BACKUP_DIR/fractalaw.pgdump"

# DuckDB (LRT metadata, taxa hashes, publish state)
cp data/fractalaw.duckdb "$BACKUP_DIR/"

echo "Quick backup complete"
du -sh "$BACKUP_DIR/fractalaw.pgdump" "$BACKUP_DIR/fractalaw.duckdb"
```

## Full Backup

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

# Postgres (pg_dump — custom format for fast restore)
PGPASSWORD=fractalaw pg_dump -h localhost -p 5433 -U fractalaw -Fc fractalaw > "$BACKUP_DIR/fractalaw.pgdump"

# DuckDB
cp data/fractalaw.duckdb "$BACKUP_DIR/"

# LanceDB (copy entire directory — binary fragments, not individual files)
cp -r data/lancedb/ "$BACKUP_DIR/lancedb/"

# Classifier models (gitignored — only live in data/)
cp data/drrp_classifier_v*.pkl "$BACKUP_DIR/" 2>/dev/null

# SLM adapter (only if exists)
[ -d data/slm-adapter ] && cp -r data/slm-adapter/ "$BACKUP_DIR/slm-adapter/"

# GGUF model (only if exists)
[ -f data/gemma3-position-q4.gguf ] && cp data/gemma3-position-q4.gguf "$BACKUP_DIR/"
```

### 3. Verify

```bash
du -sh "$BACKUP_DIR"/*

# Verify Postgres dump
PGPASSWORD=fractalaw pg_restore -l "$BACKUP_DIR/fractalaw.pgdump" | head -5

# Verify Postgres row count
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "SELECT count(*) FROM legislation_text;"
```

## Compaction Before Backup

If LanceDB has grown large due to merge_insert fragment bloat, compact first to reduce backup size:

```bash
/usr/bin/python3 scripts/maintenance/compact_lance.py
```

## Restore

```bash
# From NAS backup
BACKUP_DIR=/mnt/nas/sertantai-data/data/fractalaw-backups/YYYYMMDD
cp "$BACKUP_DIR/fractalaw.duckdb" data/
cp -r "$BACKUP_DIR/lancedb/" data/lancedb/

# Restore Postgres from pg_dump
PGPASSWORD=fractalaw pg_restore -h localhost -p 5433 -U fractalaw -d fractalaw --clean --if-exists "$BACKUP_DIR/fractalaw.pgdump"

# Restore SLM adapter + GGUF (if needed)
[ -d "$BACKUP_DIR/slm-adapter" ] && cp -r "$BACKUP_DIR/slm-adapter/" data/slm-adapter/
[ -f "$BACKUP_DIR/gemma3-position-q4.gguf" ] && cp "$BACKUP_DIR/gemma3-position-q4.gguf" data/

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
- Postgres is the hub primary store (188K+ rows) — `pg_dump -Fc` is fast and compresses well
- Postgres container: `systemctl --user start fractalaw-pg.service` (port 5433)
- Embeddings take ~9 hours to recompute on CPU (161K rows × 384-dim) — the backup is the safety net
- Multiple dated backups can coexist on the NAS (5.5 TB available)
- Local Parquet backups in `backups/` are a secondary safety net (~175 MB each)
- **NEVER write directly to NAS** — NAS block-pads binary files. Write locally first, copy to NAS
