---
description: Keep the Postgres hub's LAT (provision text) in step with sertantai-legal using the per-law LAT manifest (lat_hash/struct_hash), diff-apply with tier-data carry-over, and verified, reversible deletion of laws legal no longer holds.
---

# LAT Sync (fractalatai #62)

## When This Applies

- Checking whether the hub's provision text has drifted from legal's ("is the hub stale?")
- After legal re-parses laws, fixes sort keys or ids, or deletes LAT
- Before QA, re-parse or enrichment of a law whose text may be out of date
- Handling laws the hub holds but legal no longer serves (revoked, duplicates)

## How It Works

- **Legal serves a manifest:** `lat-manifest/{law|*}` with `row_count`, `lat_hash` and `struct_hash`.
  - `lat_hash` covers `section_id`, `sort_key` and normalised text.
  - `struct_hash` covers the 18 structural columns.
  - Vectors: `crates/fractalaw-core/data/lat_hash_vectors.json`.
- **Legal also serves a rename log:** `lat-renames/{law}`.
- **The hub records what it applied** in `lat_sync_state`. A law is stale when either hash differs.
- **Stale laws are pulled** (`lat/{law}`), checked against the manifest, and planned (`fractalaw_core::lat_sync::plan_diff`):

  | Bucket | Meaning | Effect |
  |---|---|---|
  | unchanged | same id; same text, or only numbering/markers/spacing differ (`match_key`) | tier data kept, LAT columns refreshed (sort_key-only changes land here) |
  | renamed | legal's rename map first, then a unique match on exact text, then on `match_key` (leading `[F…`, `(n)`, `27 `, `4.—` stripped) | tier data moves to the new id |
  | grown | same id, new text contains the old (≥ 15 chars) | tier data kept; needs re-parse for the added text |
  | text_changed | same id, other new text, including a parent that has shed its children's text | old row snapshotted to `lat_archive`; tier data cleared; needs re-parse. (Parse never removes actor rows it doesn't find again, so stale tier data isn't carried.) |
  | inserted | new rows | needs parse |
  | archived | hub rows whose text is gone | moved to `lat_archive` with their actor/fitness rows |
  | held | ambiguous (duplicate text, colliding renames, legal `ambiguous`) | left untouched for review |

- **Gate:** each law is applied in one transaction. If tier data on carried rows (actors, fitness mentions, drrp_types, embeddings, scope) isn't identical before and after, it rolls back and the run stops.
- **`sync watch` (with `--pg`)** applies `lat` events the same way, never deletes on `lat_deleted`, and re-compares the full manifest at startup and every `--manifest-interval-mins` (default 60).

## Runbook

Set `PG=--pg postgres://fractalaw:fractalaw@localhost:5433/fractalaw` and `Z="--tenant dev --connect tcp/localhost:7447"`.

1. **Back up first** (first run, and before any large batch): run the NAS backup skill (`/nas-backup`) for DuckDB + Postgres.
2. **Dry run:** `fractalaw-sync $PG pull-lat $Z --stale`. This writes `data/lat-sync/pull_lat_<ts>.csv`, `delete_candidates_<ts>.csv` and `reparse_<ts>.txt`. Review the counts: text_changed, archived, held.
3. **Pilot:** `pull-lat $Z --laws <1 unenriched law>,<1 enriched law with renames> --apply`, then check the report: `actors_carried`, and no `gate_failed`.
4. **Batches:** `pull-lat $Z --stale --apply --limit 30`, then repeat. Benchmark laws are report-only unless `--allow-benchmark`, which needs Jason's approval.
5. **Re-parse:** the laws in `reparse_<ts>.txt` go through the existing pipeline: `taxa parse` → reconcile → backfill (and fitness). `taxa parse` rewrites law-level DuckDB DRRP, so restore held/zero-actor laws from a snapshot per the #57/#58 lessons.
6. **Held rows:** look at them per law (`lat_sync_state.held_section_ids`) and decide manually.

## Delete Verification

A law the hub holds that legal serves no LAT for is a **delete candidate**. **Being absent from the manifest never deletes anything on its own.**

1. **Deterministic pass** (automatic, in `delete_candidates_<ts>.csv`):
   - `verified_revoked`: legal LRT `live` is revoked, DuckDB `status = revoked`, `rescinded_by` is non-empty, there is a rescind date, and there's no status conflict;
   - `legal_gap_in_force`: legal says in force. It's legal's gap (it needs a LAT parse). **Never delete.**
   - `needs_review`: the records disagree (e.g. DuckDB `partial`), or there's no audit trail or a conflict;
   - `unknown_to_legal`: legal has no LRT row (e.g. a regnal-year duplicate of a modern-named law).
2. **Agent review** for `needs_review` and `unknown_to_legal` (spawn an agent per batch). For each law:
   - Read the LRT audit trail first, in DuckDB `legislation`: `status`, `status_conflict_detail`, `rescinded_by` (names, dates), `latest_rescind_date`, `amended_by`. Check whether the revoking instruments revoke the whole law or part of it, and whether they're in force.
   - Only if that's inconclusive, check the source. Use **legislation.gov.uk's changes table** (`https://www.legislation.gov.uk/changes/affected/{type}/{year}/{number}`), the one legal reads. `data.xml` effects are incomplete: GPSR 1994's revocation is only in the changes table.
     - A whole-instrument revoked/repealed row counts **whether or not it has been applied**: legislation.gov.uk is often years behind in applying effects.
     - **Not whole revocations:** commencement rows ("appointed day for spec. repeals"); revocations whose extent is only part of the UK (e.g. Scotland-only repeal, E+W-only revocation); revocations with savings (note them).
     - Titles marked "(revoked)" / "(repealed)" confirm an applied whole revocation.
     - **Matching rules** (from legal's `ChangesFeed`/`LiveStatus.whole?`, 2026-09-28; the feed is `/changes/affected/{type}/{year}/{number}/data.feed`):
       - Any revocation type counts: `rev`, `rep`, `revoked`, `repealed`, `revoked (with savings)`, `revoked in so far as not already revoked`, …
       - Not whole, and not UK: `partial repeal`, `revoked in part`/`in pt`, overseas types such as `revoked (Pitcairn)`.
       - Blank `AffectedProvisions` = the whole instrument, as are "Regulations"/"Act"/"Order", and for EU law "Regulation"/"Directive"/"Decision".
       - `Applied="false"` still counts: a revocation not yet worked into the text.
       - `AffectingEffectsExtent="S+A+M+E+A+S+A+F+F+E+C+T+E+D"` means *same as affected*. Don't parse it as regions.
     - Once legal's status fix has covered a law, its `live_evidence` JSONB already carries the decision (kind `revoked`/`revoked_unapplied`/`territorial`/`part_revoked`, with_savings, revokers, regions). Prefer reading that to re-deriving it.
   - Regnal-year names: find the modern-named law in legal (`legal_register`, read-only) and mark it `duplicate_of:<name>`.
   - Write `data/lat-sync/delete_review_<ts>.csv` with columns `law,verdict,evidence,source`. The verdict is one of `revoked` | `keep` | `legal_gap` | `duplicate_of:<law>`.
3. **Jason approves** the list: `verified_revoked` plus agent-confirmed `revoked`/`duplicate_of`.
4. **Archive:** `fractalaw-sync $PG pull-lat $Z --archive-laws <file>`. It refuses benchmark laws and any law legal still serves. Rows and their actor/fitness rows go to `lat_archive`.
5. **Undo:** `pull-lat --restore-laws <laws>` restores the most recent archive batch.

## Notes

- Removed and changed rows are always archived (`lat_archive`, JSONB snapshots), never hard-deleted.
- An empty manifest (legal server restarting) aborts the pass rather than treating every law as a delete candidate.
- Without `--pg` (LanceDB edge), `pull-lat --laws` keeps the old plain upsert.

## Tests

The store tests (`cargo test -p fractalaw-store --features pg pg_lat_sync pg_provenance`) run against a scratch database, **never the hub**. They skip if it's missing. To recreate it (schema + three real laws):

```bash
export PGPASSWORD=fractalaw; P="-h localhost -p 5433 -U fractalaw"
psql $P fractalaw -c "create database fractalaw_lat_test"
pg_dump $P --schema-only --no-owner fractalaw | psql $P -q fractalaw_lat_test
LAWS="'UK_ssi_2016_88','UK_uksi_2016_614','UK_wsi_2021_77'"
for t in legislation_text provision_actors fitness_mentions; do
  W=$([ $t = legislation_text ] && echo "law_name in ($LAWS)" || echo "section_id in (select section_id from legislation_text where law_name in ($LAWS))")
  psql $P fractalaw -c "\\copy (select * from $t where $W) to '/tmp/$t.copy'"
  psql $P fractalaw_lat_test -c "\\copy $t from '/tmp/$t.copy'"
done
```

