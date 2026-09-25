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
| DuckDB (fractalaw) | ~200 MB | Every enrich/publish |
| DuckDB (cultural-graph) | ~50 MB | Monthly cultural graph load |
| DuckDB (sif) | ~TBD | SIF classification runs |
| LanceDB | 370 MB–1.4 GB | New law ingestion, re-embed |
| SIF taxonomy + sources | ~220 MB | ICD-11 download, OSHA data acquisition |
| SIF calibration + models | ~TBD | Calibration curve fitting, model training |
| Classifiers | ~60 KB (JSON in crates/fractalaw-cli/config/) | Retrain classifier |
| SLM adapter | ~125 MB | Retrain SLM on RunPod |
| GGUF model | ~2.4 GB | Retrain SLM on RunPod |

**Do NOT back up `target/`** — it's 29+ GB of build artifacts.

## NAS Details

- **Mount**: `/mnt/nas/sertantai-data` (UGREEN DXP2800, SMB3 automount via fstab)
- **Backup dir**: `/mnt/nas/sertantai-data/data/fractalaw-backups/`
- **Space**: 5.5 TB total, typically <1% used

## The rule: stage locally, then copy, then verify

**Never write backup files directly to the NAS.** The SMB mount block-pads binary files with trailing zeros. `correct_gold_standard.py` once wrote benchmark Parquet straight to the NAS and **13 of 15 files were corrupted**, with no second copy. Every procedure below therefore:

1. **Stages** into `/mnt/ssd/fractalaw-backups/nas-stage-YYYYMMDD/` (local SSD, ~800 GB free; `/var/home` is nearly full).
2. **Validates** the staged files locally.
3. **Writes** a `SHA256SUMS` manifest.
4. **Copies** to the NAS with `rsync`, then runs `sync`.
5. **Verifies** the NAS copies against the manifest (`sha256sum -c`) and re-opens them.

The staging directory stays on the SSD as a second copy. Prune old ones by hand.

## Quick Backup

```bash
D=$(date +%Y%m%d)
STAGE=/mnt/ssd/fractalaw-backups/nas-stage-$D
NAS=/mnt/nas/sertantai-data/data/fractalaw-backups/$D
mkdir -p "$STAGE"

# 1. Stage locally
# Postgres (the primary store — provision_actors, legislation_text, fitness_mentions, gold_benchmarks)
PGPASSWORD=fractalaw pg_dump -h localhost -p 5433 -U fractalaw -Fc fractalaw > "$STAGE/fractalaw.pgdump"
# DuckDB (LRT metadata, trees, application, publish state). Nothing may hold the lock:
pgrep -af "target/debug/fractalaw" && echo "WARNING: fractalaw process running — DuckDB copy may be inconsistent"
cp data/fractalaw.duckdb "$STAGE/"

# 2. Validate locally
PGPASSWORD=fractalaw pg_restore -l "$STAGE/fractalaw.pgdump" | grep -c "TABLE DATA"   # expect 8
duckdb -readonly "$STAGE/fractalaw.duckdb" "SELECT count(*) FROM legislation"

# 3. Manifest
(cd "$STAGE" && sha256sum fractalaw.pgdump fractalaw.duckdb > SHA256SUMS)

# 4. Copy to NAS
mkdir -p "$NAS"
rsync -a "$STAGE/fractalaw.pgdump" "$STAGE/fractalaw.duckdb" "$STAGE/SHA256SUMS" "$NAS/"
sync

# 5. Verify NAS copies
(cd "$NAS" && sha256sum -c SHA256SUMS)                        # both must say OK
PGPASSWORD=fractalaw pg_restore -l "$NAS/fractalaw.pgdump" | grep -c "TABLE DATA"
duckdb -readonly "$NAS/fractalaw.duckdb" "SELECT count(*) FROM legislation"
du -sh "$NAS"/*
```

## Full Backup

### 1. Pre-flight checks

```bash
# Verify NAS is mounted
ls /mnt/nas/sertantai-data/data/

# Check local data sizes
du -sh data/fractalaw.duckdb data/lancedb/ data/cultural-graph.duckdb data/sif/ 2>/dev/null

# Check Postgres size
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "SELECT pg_size_pretty(pg_database_size('fractalaw'));"

# Check free space: SSD staging and NAS
df -h /mnt/ssd /mnt/nas/sertantai-data/

# Nothing may hold DuckDB/LanceDB open
pgrep -af "target/debug/fractalaw"
```

### 2. Stage everything locally

```bash
D=$(date +%Y%m%d)
STAGE=/mnt/ssd/fractalaw-backups/nas-stage-$D
NAS=/mnt/nas/sertantai-data/data/fractalaw-backups/$D
mkdir -p "$STAGE"

# Postgres (pg_dump — custom format for fast restore)
PGPASSWORD=fractalaw pg_dump -h localhost -p 5433 -U fractalaw -Fc fractalaw > "$STAGE/fractalaw.pgdump"

# DuckDB
cp data/fractalaw.duckdb "$STAGE/"

# LanceDB (copy entire directory — binary fragments, not individual files)
rsync -a data/lancedb/ "$STAGE/lancedb/"

# Classifier models are JSON in crates/fractalaw-cli/config/ (in git) — no backup needed
# Active versions: drrp_classifier_v8.json, position_classifier_v3.json

# Cultural graph DuckDB
[ -f data/cultural-graph.duckdb ] && cp data/cultural-graph.duckdb "$STAGE/"

# SIF data (taxonomy, sources, calibration, models, benchmarks, DuckDB)
mkdir -p "$STAGE/sif"
[ -f data/sif.duckdb ] && cp data/sif.duckdb "$STAGE/sif/"
for d in taxonomy sources calibration models benchmarks; do
  [ -d data/sif/$d ] && rsync -a data/sif/$d/ "$STAGE/sif/$d/"
done

# SLM adapter + GGUF (only if they exist)
[ -d data/slm-adapter ] && rsync -a data/slm-adapter/ "$STAGE/slm-adapter/"
[ -f models/gemma3-position-q4.gguf ] && cp models/gemma3-position-q4.gguf "$STAGE/"
```

### 3. Validate locally, write manifest

```bash
PGPASSWORD=fractalaw pg_restore -l "$STAGE/fractalaw.pgdump" | grep -c "TABLE DATA"
duckdb -readonly "$STAGE/fractalaw.duckdb" "SELECT count(*) FROM legislation"
[ -f "$STAGE/cultural-graph.duckdb" ] && duckdb -readonly "$STAGE/cultural-graph.duckdb" "SELECT 1"
PGPASSWORD=fractalaw psql -h localhost -p 5433 -U fractalaw -d fractalaw -c "SELECT count(*) FROM legislation_text;"

(cd "$STAGE" && find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 sha256sum > SHA256SUMS)
wc -l "$STAGE/SHA256SUMS"
```

### 4. Copy to NAS and verify

```bash
mkdir -p "$NAS"
rsync -a "$STAGE/" "$NAS/"
sync

(cd "$NAS" && sha256sum --quiet -c SHA256SUMS) && echo "NAS copy verified"   # any FAILED line = re-copy
PGPASSWORD=fractalaw pg_restore -l "$NAS/fractalaw.pgdump" | grep -c "TABLE DATA"
duckdb -readonly "$NAS/fractalaw.duckdb" "SELECT count(*) FROM legislation"
du -sh "$NAS"/*
```

## Compaction Before Backup

If LanceDB has grown large due to merge_insert fragment bloat, compact first to reduce backup size:

```bash
/usr/bin/python3 scripts/maintenance/compact_lance.py
```

## Restore

```bash
# From NAS backup (reads are safe; check the manifest first if present)
BACKUP_DIR=/mnt/nas/sertantai-data/data/fractalaw-backups/YYYYMMDD
[ -f "$BACKUP_DIR/SHA256SUMS" ] && (cd "$BACKUP_DIR" && sha256sum --quiet -c SHA256SUMS)
# Or restore from the local SSD staging copy: /mnt/ssd/fractalaw-backups/nas-stage-YYYYMMDD/
cp "$BACKUP_DIR/fractalaw.duckdb" data/
cp -r "$BACKUP_DIR/lancedb/" data/lancedb/

# Restore Postgres from pg_dump
PGPASSWORD=fractalaw pg_restore -h localhost -p 5433 -U fractalaw -d fractalaw --clean --if-exists "$BACKUP_DIR/fractalaw.pgdump"

# Restore cultural graph DuckDB
[ -f "$BACKUP_DIR/cultural-graph.duckdb" ] && cp "$BACKUP_DIR/cultural-graph.duckdb" data/

# Restore SIF data
[ -d "$BACKUP_DIR/sif" ] && rsync -a "$BACKUP_DIR/sif/" data/sif/
[ -f "$BACKUP_DIR/sif/sif.duckdb" ] && cp "$BACKUP_DIR/sif/sif.duckdb" data/

# Restore SLM adapter + GGUF (if needed)
[ -d "$BACKUP_DIR/slm-adapter" ] && cp -r "$BACKUP_DIR/slm-adapter/" data/slm-adapter/
[ -f "$BACKUP_DIR/gemma3-position-q4.gguf" ] && cp "$BACKUP_DIR/gemma3-position-q4.gguf" models/

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
- **NEVER write directly to NAS** — NAS block-pads binary files (13/15 benchmark Parquet files corrupted once). Stage on the SSD, validate, `rsync`, `sync`, then `sha256sum -c` the NAS copy. See "The rule" above.
- Backups before 2026-09-25 were written directly to the NAS without a manifest. Verify them before relying on one (open the DuckDB, `pg_restore -l` the dump).
