# Skill: Bulk Enrichment Operations

## When This Applies

When re-enriching the full corpus with `taxa enrich --gap-c --force`, or any large-scale enrichment that touches many making laws. This is an infrequent operation — typically after schema changes, pipeline updates, or table rebuilds.

## Key Facts

- **19,472 laws** in DuckDB, but only **~500 are making laws** with LanceDB provisions
- The other ~19,000 are non-making (amendments, SIs) — they skip instantly (no LanceDB text)
- **161,902 provisions** in LanceDB, **67,303 with actors data** after full enrichment
- A "Processed 1,020 laws" message mostly means fast no-ops for non-making laws

## The Fragment Bloat Problem

LanceDB `merge_insert` creates new data fragments for each batch update. During bulk enrichment:

- **Growth rate**: ~600 MB per 1,000 making-law provisions updated
- **Full corpus**: clean 374 MB table → 11+ GB after processing all families
- Without compaction, the table can hit 8–12 GB — enough to fill the disk

The `optimize()` / `cleanup_old_versions()` APIs require `pylance` (not installed). Use the rebuild-based compact instead.

## Procedure

### 1. Pre-flight

```bash
# Check disk — need at least 14 GB free to complete without compaction interrupts
df -h /var/home

# Back up to NAS first (see nas-backup skill)
# Back up to local Parquet as safety net
/usr/bin/python3 -c "
import lancedb, pyarrow.parquet as pq
db = lancedb.connect('data/lancedb')
table = db.open_table('legislation_text')
pq.write_table(table.to_arrow(), 'backups/legislation_text_pre_enrich.parquet')
print(f'Exported {table.count_rows():,} rows')
"
```

### 2. Enrich in per-family batches

**NEVER run `--force` without `--family`** — the global `--force` clears all DuckDB taxa columns first, and if the enrichment is interrupted you lose existing data.

```bash
# Extract family list
cargo run -p fractalaw-cli -- query \
  "SELECT DISTINCT family FROM legislation WHERE family IS NOT NULL AND family != '' ORDER BY family" \
  2>&1 | grep '|' | grep -v '^+' | grep -v '^ *| family' | sed 's/| *//;s/ *|$//' | sed 's/^ *//' > /tmp/families.txt

# Run all families
while IFS= read -r fam; do
  echo "=== Enriching family: $fam ==="
  cargo run -p fractalaw-cli -- taxa enrich --gap-c --force --family "$fam" 2>&1 | tail -1
done < /tmp/families.txt
```

### 3. Monitor disk during enrichment

Watch LanceDB size and disk free. The critical threshold is **3.5 GB free** — below that, kill the enrichment and compact.

```bash
# Monitor every 2 minutes
while true; do
  size=$(du -sh data/lancedb/ | cut -f1)
  free=$(df -h /var/home | tail -1 | awk '{print $4}')
  echo "LanceDB: $size | Free: $free"
  sleep 120
done
```

### 4. Kill-compact-resume workflow

When disk gets tight:

```bash
# 1. Kill the enrichment process
ps aux | grep "taxa enrich" | grep -v grep | awk '{print $2}' | xargs -r kill

# 2. Compact (rebuild-based — pylance not installed)
/usr/bin/python3 scripts/maintenance/compact_lance.py
# Typically reduces 11 GB → 374 MB, recovers ~10 GB disk

# 3. Find remaining families
grep "^=== Enriching" /path/to/output | sed 's/=== Enriching family: //' | sed 's/ ===//' > /tmp/done.txt
comm -23 <(sort /tmp/families.txt) <(sort /tmp/done.txt) > /tmp/remaining.txt

# 4. Resume from remaining families
while IFS= read -r fam; do
  echo "=== Enriching family: $fam ==="
  cargo run -p fractalaw-cli -- taxa enrich --gap-c --force --family "$fam" 2>&1 | tail -1
done < /tmp/remaining.txt
```

### 5. Final compact and backup

```bash
# Compact to clean state
/usr/bin/python3 scripts/maintenance/compact_lance.py

# Back up clean result to NAS
BACKUP_DIR=/mnt/nas/sertantai-data/data/fractalaw-backups/$(date +%Y%m%d)
mkdir -p "$BACKUP_DIR"
cp data/fractalaw.duckdb "$BACKUP_DIR/"
cp -r data/lancedb/ "$BACKUP_DIR/lancedb/"
```

## Enrichment Timing

- **Non-making families** (e.g., HEALTH: Coronavirus 554 laws): ~30 seconds (all skips)
- **Mixed families** (e.g., AGRICULTURE 1,020 laws): ~1-2 minutes
- **Heavy making-law families** (e.g., OH&S: Occupational 451 laws): ~10-15 minutes
- **Full corpus** (80 families): ~45-60 minutes with 2 compact interrupts

## Family Naming

DuckDB has both emoji-prefixed (`💚 AGRICULTURE`, 150 laws) and plain (`AGRICULTURE`, 13,322 laws) family names due to drift between sertantai and fractalaw. Both variants need enriching. Convention going forward: plain names (no emoji) in DuckDB.

## Empty-family laws

~6,000 laws have empty/NULL family. These are all non-making — `taxa enrich --gap-c` (without `--force` or `--family`) confirms "All laws with LanceDB text already have DRRP taxa data." No special handling needed.

## What NOT to do

- **Don't run `taxa enrich --force` without `--family`** — clears all DuckDB taxa globally
- **Don't manually delete LanceDB fragments** — corrupts the manifest
- **Don't ignore disk warnings** — LanceDB can fill the disk within minutes during heavy families
- **Don't skip the Parquet backup** — it's the safety net if the table gets corrupted mid-enrichment
