//! Hub side of LAT sync with sertantai-legal (fractalatai #62): per-law sync
//! state, a reversible archive, and a transactional diff-apply that refuses to
//! commit if tier data on carried rows didn't survive.
//!
//! Tier data lives in three places: the enrichment columns on
//! `legislation_text`, and `provision_actors` / `fitness_mentions`, which
//! cascade on delete but not on id change. A rename therefore copies the row
//! to its new id, re-points the child rows, then deletes the old row.

use std::collections::HashMap;

use arrow::array::RecordBatch;
use chrono::{DateTime, Utc};
use fractalaw_core::lat_sync::{DiffPlan, LatRow, ManifestEntry};
use sqlx::Row;

use crate::pg::upsert_lat_rows;
use crate::{PgStore, StoreError};

fn db(ctx: &'static str) -> impl Fn(sqlx::Error) -> StoreError {
    move |e| StoreError::Other(format!("{ctx}: {e}"))
}

/// What the hub last applied for one law.
#[derive(Debug, Clone)]
pub struct LatSyncState {
    pub law_name: String,
    pub lat_hash: String,
    pub struct_hash: Option<String>,
    pub row_count: i64,
    /// Latest legal rename-log `created_at` consumed for this law
    pub renames_through: Option<DateTime<Utc>>,
    /// Hub-only rows left for review (ambiguous matches)
    pub held_section_ids: Vec<String>,
    /// Text changed or rows inserted since the last parse
    pub reparse_needed: bool,
    pub applied_at: DateTime<Utc>,
}

/// Tier data on a set of provisions. Diff-apply requires the carried rows'
/// counts to be identical before (old ids) and after (new ids).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TierCounts {
    pub rows: i64,
    pub drrp_types: i64,
    pub embedding: i64,
    pub scope: i64,
    pub provision_actors: i64,
    pub fitness_mentions: i64,
}

#[derive(Debug, Clone)]
pub struct LatApplyReport {
    pub law_name: String,
    pub committed: bool,
    pub before: TierCounts,
    pub after: TierCounts,
    /// Rows written to `lat_archive` (removed + text-changed snapshots)
    pub archived_rows: u64,
    pub upserted: usize,
}

impl LatApplyReport {
    pub fn gate_passed(&self) -> bool {
        self.before == self.after
    }
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS lat_sync_state (
    law_name          TEXT PRIMARY KEY,
    lat_hash          TEXT NOT NULL,
    struct_hash       TEXT,
    row_count         INTEGER NOT NULL,
    renames_through   TIMESTAMPTZ,
    held_section_ids  TEXT[] NOT NULL DEFAULT '{}',
    reparse_needed    BOOLEAN NOT NULL DEFAULT false,
    applied_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS lat_archive (
    id                BIGSERIAL PRIMARY KEY,
    law_name          TEXT NOT NULL,
    section_id        TEXT NOT NULL,
    reason            TEXT NOT NULL,
    archived_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    row_data          JSONB NOT NULL,
    provision_actors  JSONB,
    fitness_mentions  JSONB
);
CREATE INDEX IF NOT EXISTS idx_lat_archive_law ON lat_archive (law_name, archived_at);
";

/// Snapshot rows + their child tier rows into `lat_archive`.
const ARCHIVE_SQL: &str = "
INSERT INTO lat_archive (law_name, section_id, reason, row_data, provision_actors, fitness_mentions)
SELECT lt.law_name, lt.section_id, $2, to_jsonb(lt),
       (SELECT jsonb_agg(to_jsonb(pa)) FROM provision_actors pa WHERE pa.section_id = lt.section_id),
       (SELECT jsonb_agg(to_jsonb(fm)) FROM fitness_mentions fm WHERE fm.section_id = lt.section_id)
FROM legislation_text lt WHERE lt.section_id = ANY($1)";

async fn tier_counts(conn: &mut sqlx::PgConnection, ids: &[String]) -> Result<TierCounts, StoreError> {
    let r = sqlx::query(
        "SELECT count(*),
                count(*) FILTER (WHERE drrp_types IS NOT NULL),
                count(*) FILTER (WHERE embedding IS NOT NULL),
                count(*) FILTER (WHERE scope IS NOT NULL),
                (SELECT count(*) FROM provision_actors WHERE section_id = ANY($1)),
                (SELECT count(*) FROM fitness_mentions WHERE section_id = ANY($1))
         FROM legislation_text WHERE section_id = ANY($1)",
    )
    .bind(ids)
    .fetch_one(&mut *conn)
    .await
    .map_err(db("tier counts"))?;
    Ok(TierCounts {
        rows: r.get(0),
        drrp_types: r.get(1),
        embedding: r.get(2),
        scope: r.get(3),
        provision_actors: r.get(4),
        fitness_mentions: r.get(5),
    })
}

impl PgStore {
    /// Create the sync-state and archive tables if missing (additive).
    pub async fn ensure_lat_sync_tables(&self) -> Result<(), StoreError> {
        sqlx::raw_sql(SCHEMA).execute(self.pool()).await.map_err(db("lat sync schema"))?;
        Ok(())
    }

    /// Sync state for every law the hub has applied from a manifest.
    pub async fn lat_sync_states(&self) -> Result<HashMap<String, LatSyncState>, StoreError> {
        let rows = sqlx::query(
            "SELECT law_name, lat_hash, struct_hash, row_count, renames_through,
                    held_section_ids, reparse_needed, applied_at
             FROM lat_sync_state",
        )
        .fetch_all(self.pool())
        .await
        .map_err(db("lat_sync_state"))?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let s = LatSyncState {
                    law_name: r.get(0),
                    lat_hash: r.get(1),
                    struct_hash: r.get(2),
                    row_count: r.get::<i32, _>(3) as i64,
                    renames_through: r.get(4),
                    held_section_ids: r.get(5),
                    reparse_needed: r.get(6),
                    applied_at: r.get(7),
                };
                (s.law_name.clone(), s)
            })
            .collect())
    }

    /// Distinct laws with LAT in the hub, with row counts.
    pub async fn hub_law_row_counts(&self) -> Result<HashMap<String, i64>, StoreError> {
        let rows: Vec<(String, i64)> =
            sqlx::query_as("SELECT law_name, count(*) FROM legislation_text GROUP BY law_name")
                .fetch_all(self.pool())
                .await
                .map_err(db("hub laws"))?;
        Ok(rows.into_iter().collect())
    }

    /// The hub's current (section_id, sort_key, text) rows for one law.
    pub async fn hub_lat_rows(&self, law_name: &str) -> Result<Vec<LatRow>, StoreError> {
        let rows: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT section_id, sort_key, text FROM legislation_text WHERE law_name = $1",
        )
        .bind(law_name)
        .fetch_all(self.pool())
        .await
        .map_err(db("hub lat rows"))?;
        Ok(rows
            .into_iter()
            .map(|(section_id, sort_key, text)| LatRow { section_id, sort_key, text })
            .collect())
    }

    /// Apply a planned diff for one law in a single transaction:
    /// archive removed rows; snapshot text-changed rows and clear their tier
    /// data; move renamed rows (and child tier rows) to their new ids; upsert
    /// legal's LAT columns for every row; then check that tier data on carried
    /// rows is unchanged. If the check fails, nothing is committed.
    pub async fn apply_lat_diff(
        &self,
        law_name: &str,
        legal: &[RecordBatch],
        plan: &DiffPlan,
        manifest: &ManifestEntry,
        renames_through: Option<DateTime<Utc>>,
    ) -> Result<LatApplyReport, StoreError> {
        let mut tx = self.pool().begin().await.map_err(db("begin"))?;
        let carried = plan.carried();
        let old_ids: Vec<String> = carried.iter().map(|(o, _)| o.clone()).collect();
        let new_ids: Vec<String> = carried.iter().map(|(_, n)| n.clone()).collect();
        let before = tier_counts(&mut tx, &old_ids).await?;
        let mut archived_rows = 0u64;

        // Removed rows: archive, then delete (children cascade)
        if !plan.archived.is_empty() {
            archived_rows += sqlx::query(ARCHIVE_SQL)
                .bind(&plan.archived)
                .bind("removed")
                .execute(&mut *tx)
                .await
                .map_err(db("archive removed"))?
                .rows_affected();
            sqlx::query("DELETE FROM legislation_text WHERE section_id = ANY($1)")
                .bind(&plan.archived)
                .execute(&mut *tx)
                .await
                .map_err(db("delete removed"))?;
        }

        // Text changed: snapshot, drop child tier rows, null enrichment columns
        if !plan.text_changed.is_empty() {
            archived_rows += sqlx::query(ARCHIVE_SQL)
                .bind(&plan.text_changed)
                .bind("text_changed")
                .execute(&mut *tx)
                .await
                .map_err(db("archive text_changed"))?
                .rows_affected();
            for child in ["provision_actors", "fitness_mentions"] {
                sqlx::query(&format!("DELETE FROM {child} WHERE section_id = ANY($1)"))
                    .bind(&plan.text_changed)
                    .execute(&mut *tx)
                    .await
                    .map_err(db("clear child tier rows"))?;
            }
            let lat_cols: Vec<String> = legal
                .first()
                .map(|b| b.schema().fields().iter().map(|f| f.name().clone()).collect())
                .unwrap_or_default();
            let cols: Vec<(String,)> = sqlx::query_as(
                "SELECT column_name::text FROM information_schema.columns
                 WHERE table_name = 'legislation_text' AND is_nullable = 'YES'
                   AND column_name <> ALL($1)
                   AND column_name NOT IN ('section_id', 'law_name', 'created_at', 'updated_at')",
            )
            .bind(&lat_cols)
            .fetch_all(&mut *tx)
            .await
            .map_err(db("enrichment columns"))?;
            if !cols.is_empty() {
                let set: Vec<String> = cols.iter().map(|(c,)| format!("{c} = NULL")).collect();
                sqlx::query(&format!(
                    "UPDATE legislation_text SET {} WHERE section_id = ANY($1)",
                    set.join(", ")
                ))
                .bind(&plan.text_changed)
                .execute(&mut *tx)
                .await
                .map_err(db("clear enrichment"))?;
            }
        }

        // Renames: copy row to the new id, re-point children, drop the old row
        for (old, new, _) in &plan.renamed {
            sqlx::query(
                "INSERT INTO legislation_text
                 SELECT (jsonb_populate_record(NULL::legislation_text,
                         to_jsonb(lt) || jsonb_build_object('section_id', $2::text))).*
                 FROM legislation_text lt WHERE lt.section_id = $1",
            )
            .bind(old)
            .bind(new)
            .execute(&mut *tx)
            .await
            .map_err(db("rename copy"))?;
            for child in ["provision_actors", "fitness_mentions"] {
                sqlx::query(&format!("UPDATE {child} SET section_id = $2 WHERE section_id = $1"))
                    .bind(old)
                    .bind(new)
                    .execute(&mut *tx)
                    .await
                    .map_err(db("rename children"))?;
            }
            sqlx::query("DELETE FROM legislation_text WHERE section_id = $1")
                .bind(old)
                .execute(&mut *tx)
                .await
                .map_err(db("rename delete old"))?;
        }

        let upserted = upsert_lat_rows(&mut tx, legal).await?;
        let after = tier_counts(&mut tx, &new_ids).await?;
        let report = LatApplyReport {
            law_name: law_name.to_string(),
            committed: before == after,
            before,
            after,
            archived_rows,
            upserted,
        };
        if !report.committed {
            tx.rollback().await.map_err(db("rollback"))?;
            return Ok(report);
        }

        sqlx::query(
            "INSERT INTO lat_sync_state
                (law_name, lat_hash, struct_hash, row_count, renames_through, held_section_ids, reparse_needed, applied_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, now())
             ON CONFLICT (law_name) DO UPDATE SET
                lat_hash = EXCLUDED.lat_hash, struct_hash = EXCLUDED.struct_hash,
                row_count = EXCLUDED.row_count,
                renames_through = COALESCE(EXCLUDED.renames_through, lat_sync_state.renames_through),
                held_section_ids = EXCLUDED.held_section_ids,
                reparse_needed = lat_sync_state.reparse_needed OR EXCLUDED.reparse_needed,
                applied_at = now()",
        )
        .bind(law_name)
        .bind(&manifest.lat_hash)
        .bind(&manifest.struct_hash)
        .bind(manifest.row_count as i32)
        .bind(renames_through)
        .bind(&plan.held)
        .bind(!plan.text_changed.is_empty() || !plan.inserted.is_empty())
        .execute(&mut *tx)
        .await
        .map_err(db("lat_sync_state"))?;

        tx.commit().await.map_err(db("commit"))?;
        Ok(report)
    }

    /// Archive every hub row of a law legal no longer holds (approved deletes
    /// only), then remove them. Reversible with [`Self::restore_archived_law`].
    pub async fn archive_law(&self, law_name: &str, reason: &str) -> Result<u64, StoreError> {
        let mut tx = self.pool().begin().await.map_err(db("begin"))?;
        let ids: Vec<(String,)> = sqlx::query_as("SELECT section_id FROM legislation_text WHERE law_name = $1")
            .bind(law_name)
            .fetch_all(&mut *tx)
            .await
            .map_err(db("law ids"))?;
        let ids: Vec<String> = ids.into_iter().map(|(s,)| s).collect();
        let n = sqlx::query(ARCHIVE_SQL)
            .bind(&ids)
            .bind(reason)
            .execute(&mut *tx)
            .await
            .map_err(db("archive law"))?
            .rows_affected();
        sqlx::query("DELETE FROM legislation_text WHERE law_name = $1")
            .bind(law_name)
            .execute(&mut *tx)
            .await
            .map_err(db("delete law"))?;
        sqlx::query(
            "INSERT INTO lat_sync_state (law_name, lat_hash, row_count, applied_at)
             VALUES ($1, 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', 0, now())
             ON CONFLICT (law_name) DO UPDATE SET lat_hash = EXCLUDED.lat_hash,
                struct_hash = NULL, row_count = 0, held_section_ids = '{}', applied_at = now()",
        )
        .bind(law_name)
        .execute(&mut *tx)
        .await
        .map_err(db("lat_sync_state"))?;
        tx.commit().await.map_err(db("commit"))?;
        Ok(n)
    }

    /// Restore a law's most recent archive batch with the given reason
    /// (rows and child tier rows). Rows whose section_id exists again are skipped.
    pub async fn restore_archived_law(&self, law_name: &str, reason: &str) -> Result<u64, StoreError> {
        let mut tx = self.pool().begin().await.map_err(db("begin"))?;
        let batch = "(SELECT max(archived_at) FROM lat_archive WHERE law_name = $1 AND reason = $2)";
        let n = sqlx::query(&format!(
            "INSERT INTO legislation_text
             SELECT (jsonb_populate_record(NULL::legislation_text, a.row_data)).*
             FROM lat_archive a WHERE a.law_name = $1 AND a.reason = $2 AND a.archived_at = {batch}
             ON CONFLICT (section_id) DO NOTHING"
        ))
        .bind(law_name)
        .bind(reason)
        .execute(&mut *tx)
        .await
        .map_err(db("restore rows"))?
        .rows_affected();
        for child in ["provision_actors", "fitness_mentions"] {
            sqlx::query(&format!(
                "INSERT INTO {child}
                 SELECT (jsonb_populate_record(NULL::{child}, e)).*
                 FROM lat_archive a, jsonb_array_elements(a.{child}) e
                 WHERE a.law_name = $1 AND a.reason = $2 AND a.archived_at = {batch}
                 ON CONFLICT DO NOTHING"
            ))
            .bind(law_name)
            .bind(reason)
            .execute(&mut *tx)
            .await
            .map_err(db("restore children"))?;
        }
        tx.commit().await.map_err(db("commit"))?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    //! Runs against the scratch database `fractalaw_lat_test` (never the hub),
    //! seeded with UK_ssi_2016_88, UK_uksi_2016_614 and UK_wsi_2021_77.
    //! Skips when it isn't available. Each test uses its own law name.
    use super::*;
    use arrow::array::{ArrayRef, Int32Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use fractalaw_core::lat_sync::{lat_hash, plan_diff};
    use std::sync::Arc;

    const TEST_DB: &str = "postgres://fractalaw:fractalaw@localhost:5433/fractalaw_lat_test";

    async fn store() -> Option<PgStore> {
        let s = PgStore::connect(TEST_DB).await.ok()?;
        s.ensure_lat_sync_tables().await.ok()?;
        Some(s)
    }

    fn batch(law: &str, rows: &[(&str, &str, &str)]) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("law_name", DataType::Utf8, false),
            Field::new("section_id", DataType::Utf8, false),
            Field::new("sort_key", DataType::Utf8, true),
            Field::new("text", DataType::Utf8, true),
            Field::new("position", DataType::Int32, true),
        ]));
        let cols: Vec<ArrayRef> = vec![
            Arc::new(StringArray::from(vec![law; rows.len()])),
            Arc::new(StringArray::from(rows.iter().map(|r| format!("{law}:{}", r.0)).collect::<Vec<_>>())),
            Arc::new(StringArray::from(rows.iter().map(|r| r.1).collect::<Vec<_>>())),
            Arc::new(StringArray::from(rows.iter().map(|r| r.2).collect::<Vec<_>>())),
            Arc::new(Int32Array::from((0..rows.len() as i32).collect::<Vec<_>>())),
        ];
        RecordBatch::try_new(schema, cols).unwrap()
    }

    async fn seed(s: &PgStore, law: &str, rows: &[(&str, &str, &str)]) {
        let p = s.pool();
        sqlx::query("DELETE FROM legislation_text WHERE law_name = $1").bind(law).execute(p).await.unwrap();
        sqlx::query("DELETE FROM lat_archive WHERE law_name = $1").bind(law).execute(p).await.unwrap();
        sqlx::query("DELETE FROM lat_sync_state WHERE law_name = $1").bind(law).execute(p).await.unwrap();
        s.upsert_lat(vec![batch(law, rows)]).await.unwrap();
        // Tier data: enrichment column + an actor row on every provision
        sqlx::query("UPDATE legislation_text SET drrp_types = '{Obligation}', scope = 'substantive' WHERE law_name = $1")
            .bind(law).execute(p).await.unwrap();
        sqlx::query(
            "INSERT INTO provision_actors (section_id, actor_label, actor_category)
             SELECT section_id, 'Ind: Person', 'governed' FROM legislation_text WHERE law_name = $1",
        )
        .bind(law).execute(p).await.unwrap();
    }

    async fn actors_of(s: &PgStore, sid: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM provision_actors WHERE section_id = $1")
            .bind(sid).fetch_one(s.pool()).await.unwrap()
    }

    fn manifest(law: &str, legal: &[RecordBatch]) -> ManifestEntry {
        let rows = fractalaw_core::lat_sync::lat_rows_from_batches(legal).unwrap();
        ManifestEntry { law_name: law.into(), row_count: rows.len() as u64, lat_hash: lat_hash(&rows), struct_hash: None }
    }

    #[tokio::test]
    async fn diff_apply_carries_renames_archives_and_clears_changed() {
        let Some(s) = store().await else { eprintln!("test DB unavailable, skipping"); return };
        let law = "TEST_lat_apply";
        seed(&s, law, &[
            ("reg.1", "1", "The operator must keep records"),   // unchanged
            ("reg.39(e)", "2", "any other prescribed matter"),  // renamed (text match)
            ("reg.4(4)", "3", "4.—(1) Paragraphs (2) and (3) apply"), // text changed
            ("reg.5(5)", "4", "5.—(1) Ministers may issue"),    // removed
        ]).await;
        let legal = vec![batch(law, &[
            ("reg.1", "1b", "The operator  must keep records"),
            ("reg.39(2)(e)", "2", "any other prescribed matter"),
            ("reg.4(4)", "3", "Paragraphs (2) and (3) do not apply"),
            ("reg.4(2)", "3a", "A person must not deploy any fishing gear"),
        ])];
        let hub = s.hub_lat_rows(law).await.unwrap();
        let plan = plan_diff(&hub, &fractalaw_core::lat_sync::lat_rows_from_batches(&legal).unwrap(), &[]);
        assert_eq!(plan.renamed.len(), 1);
        let r = s.apply_lat_diff(law, &legal, &plan, &manifest(law, &legal), None).await.unwrap();
        assert!(r.committed && r.gate_passed(), "{r:?}");
        assert_eq!(r.before.provision_actors, 2);
        assert_eq!(r.archived_rows, 2);

        assert_eq!(actors_of(&s, &format!("{law}:reg.1")).await, 1);
        assert_eq!(actors_of(&s, &format!("{law}:reg.39(2)(e)")).await, 1, "tier data follows rename");
        assert_eq!(actors_of(&s, &format!("{law}:reg.39(e)")).await, 0);
        assert_eq!(actors_of(&s, &format!("{law}:reg.4(4)")).await, 0, "changed text loses stale actors");
        let (drrp, text): (Option<Vec<String>>, String) = sqlx::query_as(
            "SELECT drrp_types, text FROM legislation_text WHERE section_id = $1")
            .bind(format!("{law}:reg.4(4)")).fetch_one(s.pool()).await.unwrap();
        assert!(drrp.is_none());
        assert!(text.starts_with("Paragraphs (2) and (3) do not"));
        let sort_key: String = sqlx::query_scalar("SELECT sort_key FROM legislation_text WHERE section_id = $1")
            .bind(format!("{law}:reg.1")).fetch_one(s.pool()).await.unwrap();
        assert_eq!(sort_key, "1b", "metadata refreshed on unchanged rows");

        // Hub now hashes like legal, and state records it
        let hub_after = s.hub_lat_rows(law).await.unwrap();
        assert_eq!(lat_hash(&hub_after), manifest(law, &legal).lat_hash);
        let st = &s.lat_sync_states().await.unwrap()[law];
        assert!(st.reparse_needed);

        // Removed row is restorable, with its actor
        let n = s.restore_archived_law(law, "removed").await.unwrap();
        assert_eq!(n, 1);
        assert_eq!(actors_of(&s, &format!("{law}:reg.5(5)")).await, 1);
    }

    #[tokio::test]
    async fn gate_rolls_back_when_tier_data_would_be_lost() {
        let Some(s) = store().await else { eprintln!("test DB unavailable, skipping"); return };
        let law = "TEST_lat_gate";
        seed(&s, law, &[("reg.1", "1", "x must y"), ("reg.2", "2", "z must w")]).await;
        let legal = vec![batch(law, &[("reg.1", "1", "x must y"), ("reg.2", "2", "z must w")])];
        // A corrupt plan that claims reg.2 carries but also archives it
        let mut plan = plan_diff(&s.hub_lat_rows(law).await.unwrap(),
            &fractalaw_core::lat_sync::lat_rows_from_batches(&legal).unwrap(), &[]);
        plan.archived.push(format!("{law}:reg.2"));
        let r = s.apply_lat_diff(law, &legal, &plan, &manifest(law, &legal), None).await.unwrap();
        assert!(!r.committed);
        assert_eq!(actors_of(&s, &format!("{law}:reg.2")).await, 1, "rolled back");
        assert!(!s.lat_sync_states().await.unwrap().contains_key(law));
        let archived: i64 = sqlx::query_scalar("SELECT count(*) FROM lat_archive WHERE law_name = $1")
            .bind(law).fetch_one(s.pool()).await.unwrap();
        assert_eq!(archived, 0);
    }

    #[tokio::test]
    async fn archive_and_restore_whole_law() {
        let Some(s) = store().await else { eprintln!("test DB unavailable, skipping"); return };
        let law = "TEST_lat_archive";
        seed(&s, law, &[("reg.1", "1", "a"), ("reg.2", "2", "b")]).await;
        assert_eq!(s.archive_law(law, "revoked").await.unwrap(), 2);
        assert!(s.hub_lat_rows(law).await.unwrap().is_empty());
        assert_eq!(s.lat_sync_states().await.unwrap()[law].row_count, 0);
        assert_eq!(s.restore_archived_law(law, "revoked").await.unwrap(), 2);
        assert_eq!(actors_of(&s, &format!("{law}:reg.2")).await, 1);
    }

    #[tokio::test]
    async fn real_law_noop_apply_keeps_everything() {
        // Seeded copy of Wester Ross: applying the hub's own rows is a gate-clean no-op
        let Some(s) = store().await else { eprintln!("test DB unavailable, skipping"); return };
        let law = "UK_wsi_2021_77";
        let before: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM provision_actors pa JOIN legislation_text lt USING (section_id) WHERE lt.law_name = $1")
            .bind(law).fetch_one(s.pool()).await.unwrap();
        let hub = s.hub_lat_rows(law).await.unwrap();
        let legal_rows: Vec<(String, Option<String>, Option<String>)> =
            hub.iter().map(|r| (r.section_id.clone(), r.sort_key.clone(), r.text.clone())).collect();
        let schema = Arc::new(Schema::new(vec![
            Field::new("law_name", DataType::Utf8, false),
            Field::new("section_id", DataType::Utf8, false),
            Field::new("sort_key", DataType::Utf8, true),
            Field::new("text", DataType::Utf8, true),
        ]));
        let legal = vec![RecordBatch::try_new(schema, vec![
            Arc::new(StringArray::from(vec![law; legal_rows.len()])) as ArrayRef,
            Arc::new(StringArray::from(legal_rows.iter().map(|r| r.0.clone()).collect::<Vec<_>>())),
            Arc::new(StringArray::from(legal_rows.iter().map(|r| r.1.clone()).collect::<Vec<_>>())),
            Arc::new(StringArray::from(legal_rows.iter().map(|r| r.2.clone()).collect::<Vec<_>>())),
        ]).unwrap()];
        let plan = plan_diff(&hub, &hub, &[]);
        assert!(plan.is_noop());
        let r = s.apply_lat_diff(law, &legal, &plan, &manifest(law, &legal), None).await.unwrap();
        assert!(r.committed);
        assert_eq!(r.before.provision_actors, before);
        assert_eq!(r.after, r.before);
    }
}
