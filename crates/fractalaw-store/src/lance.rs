//! LanceDB storage layer for legislation text and amendment annotations.
//!
//! The semantic path stores full legal text with embeddings for similarity search.
//! Two tables: `legislation_text` (97K structural units) and `amendment_annotations`
//! (19K change annotations).

use std::path::Path;
use std::sync::Arc;

use arrow::array::{RecordBatchIterator, new_null_array};
use arrow::compute::cast;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use tracing::info;

use crate::StoreError;

const LEGISLATION_TEXT_TABLE: &str = "legislation_text";
const AMENDMENT_ANNOTATIONS_TABLE: &str = "amendment_annotations";

/// LanceDB store for the semantic path (legislation text + annotations).
///
/// Manages two Lance tables:
/// - `legislation_text`: 97K structural units with text and embeddings
/// - `amendment_annotations`: 19K amendment footnotes linked to text sections
pub struct LanceStore {
    db: lancedb::Connection,
}

impl LanceStore {
    /// Connect to a LanceDB database at the given path.
    ///
    /// Creates the database directory if it doesn't exist.
    pub async fn open(path: &Path) -> Result<Self, StoreError> {
        let uri = path
            .to_str()
            .ok_or_else(|| StoreError::Other("non-UTF8 database path".into()))?;
        let db = lancedb::connect(uri).execute().await?;
        Ok(Self { db })
    }

    /// Create (or replace) the `legislation_text` table from a Parquet file.
    pub async fn create_legislation_text(&self, parquet_path: &Path) -> Result<(), StoreError> {
        self.create_table_from_parquet(LEGISLATION_TEXT_TABLE, parquet_path)
            .await
    }

    /// Create (or replace) the `amendment_annotations` table from a Parquet file.
    pub async fn create_amendment_annotations(
        &self,
        parquet_path: &Path,
    ) -> Result<(), StoreError> {
        self.create_table_from_parquet(AMENDMENT_ANNOTATIONS_TABLE, parquet_path)
            .await
    }

    /// Load both tables from a data directory containing the Parquet files.
    pub async fn load_all(&self, data_dir: &Path) -> Result<(), StoreError> {
        self.create_legislation_text(&data_dir.join("legislation_text.parquet"))
            .await?;
        self.create_amendment_annotations(&data_dir.join("amendment_annotations.parquet"))
            .await?;
        Ok(())
    }

    /// Open the `legislation_text` table.
    pub async fn legislation_text(&self) -> Result<lancedb::Table, StoreError> {
        let table = self.db.open_table(LEGISLATION_TEXT_TABLE).execute().await?;
        Ok(table)
    }

    /// Open the `amendment_annotations` table.
    pub async fn amendment_annotations(&self) -> Result<lancedb::Table, StoreError> {
        let table = self
            .db
            .open_table(AMENDMENT_ANNOTATIONS_TABLE)
            .execute()
            .await?;
        Ok(table)
    }

    /// Count rows in the `legislation_text` table.
    pub async fn legislation_text_count(&self) -> Result<usize, StoreError> {
        let table = self.legislation_text().await?;
        let count = table.count_rows(None).await?;
        Ok(count)
    }

    /// Count rows in the `amendment_annotations` table.
    pub async fn amendment_annotations_count(&self) -> Result<usize, StoreError> {
        let table = self.amendment_annotations().await?;
        let count = table.count_rows(None).await?;
        Ok(count)
    }

    /// Vector similarity search on the `legislation_text` embedding column.
    ///
    /// Returns the nearest `limit` rows to the query vector, ordered by distance.
    /// Requires embeddings to have been populated (Task 4).
    pub async fn search_text(
        &self,
        query_vector: &[f32],
        limit: usize,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        let table = self.legislation_text().await?;
        let results: Vec<RecordBatch> = table
            .vector_search(query_vector)?
            .limit(limit)
            .execute()
            .await?
            .try_collect()
            .await?;
        Ok(results)
    }

    /// Query the `legislation_text` table with a SQL filter.
    pub async fn query_legislation_text(
        &self,
        filter: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        let table = self.legislation_text().await?;
        let results: Vec<RecordBatch> = table
            .query()
            .only_if(filter)
            .limit(limit)
            .offset(offset)
            .execute()
            .await?
            .try_collect()
            .await?;
        Ok(results)
    }

    /// Query provision-level taxa and fitness columns for a law.
    ///
    /// Returns only enriched provisions (where `drrp_types IS NOT NULL`)
    /// with a column projection that excludes text, embeddings, and token_ids.
    /// Used by `sync publish --provisions` to build the zenoh payload.
    pub async fn query_provision_taxa(
        &self,
        law_name: &str,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        let table = self.legislation_text().await?;
        let filter = format!(
            "law_name = '{}' AND drrp_types IS NOT NULL",
            law_name.replace('\'', "''")
        );
        let columns: Vec<String> = [
            "section_id",
            "drrp_types",
            // governed_actors and government_actors omitted — replaced by actors struct.
            // Sertantai retains old flat data for comparison.
            "duty_family",
            "duty_sub_type",
            "popimar",
            "purposes",
            "clause_refined",
            "taxa_confidence",
            "taxa_classified_at",
            "fitness_polarity",
            "fitness_person",
            "fitness_process",
            "fitness_place",
            "fitness_plant",
            "fitness_property",
            "fitness_sector",
            "extraction_method",
            "holder_inferred_from",
            "ancestor_distance",
            "actors",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let results: Vec<RecordBatch> = table
            .query()
            .only_if(filter)
            .select(Select::Columns(columns))
            .execute()
            .await?
            .try_collect()
            .await?;
        Ok(results)
    }

    /// Ensure Gap C provenance columns exist on the `legislation_text` table.
    ///
    /// LanceDB doesn't support `ALTER TABLE ADD COLUMN IF NOT EXISTS`, so we
    /// check the schema first and only add missing columns. Idempotent.
    pub async fn ensure_gap_c_columns(&self) -> Result<(), StoreError> {
        let table = self.legislation_text().await?;
        let schema = table.schema().await.map_err(|e| {
            StoreError::Other(format!("failed to get legislation_text schema: {e}"))
        })?;

        // Simple scalar columns can use SQL CAST. List columns need a
        // different approach — we add them via an Arrow transform instead.
        let scalar_needed: Vec<(&str, &str)> = vec![
            ("extraction_method", "CAST(NULL AS STRING)"),
            ("ancestor_distance", "CAST(NULL AS INT)"),
        ];

        for (col_name, default_expr) in &scalar_needed {
            if schema.field_with_name(col_name).is_err() {
                table
                    .add_columns(
                        lancedb::table::NewColumnTransform::SqlExpressions(vec![(
                            col_name.to_string(),
                            default_expr.to_string(),
                        )]),
                        None,
                    )
                    .await
                    .map_err(|e| StoreError::Other(format!("add column {col_name}: {e}")))?;
                info!(column = col_name, "added Gap C column to legislation_text");
            }
        }

        // holder_inferred_from: stored as Utf8 (scalar column, SQL-safe)
        if schema.field_with_name("holder_inferred_from").is_err() {
            table
                .add_columns(
                    lancedb::table::NewColumnTransform::SqlExpressions(vec![(
                        "holder_inferred_from".to_string(),
                        "CAST(NULL AS STRING)".to_string(),
                    )]),
                    None,
                )
                .await
                .map_err(|e| StoreError::Other(format!("add column holder_inferred_from: {e}")))?;
            info!(
                column = "holder_inferred_from",
                "added column to legislation_text"
            );
        }
        // actors: native List<Struct> — created during table rebuild, not via add_columns
        // Position classifier output is stored in the actors struct `reason` field
        // (only when classifier disagrees with regex position).

        // drrp_history: JSON string (was List<Struct>, changed to avoid Lance offset panics).
        // Records what each tier (regex, classifier, llm) predicted for this provision.
        if schema.field_with_name("drrp_history").is_err() {
            tracing::warn!(
                "drrp_history column missing from legislation_text — \
                 run scripts/migrate_drrp_history.py to add it"
            );
        }

        Ok(())
    }

    /// Query provisions that have taxa data but no AI refinement yet.
    pub async fn query_unpolished(
        &self,
        law_name: &str,
        limit: usize,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        let filter = format!(
            "law_name = '{}' AND drrp_types IS NOT NULL AND ai_clause IS NULL",
            law_name.replace('\'', "''")
        );
        self.query_legislation_text(&filter, limit, 0).await
    }

    /// Write taxa classification results for provisions.
    ///
    /// Uses `merge_insert` keyed on `section_id` — updates only the provided
    /// columns for matched rows. The batch must contain `section_id` plus any
    /// taxa columns to update.
    pub async fn update_taxa(&self, batch: RecordBatch) -> Result<(), StoreError> {
        let table = self.legislation_text().await?;
        let schema = batch.schema();
        let reader = RecordBatchIterator::new(vec![Ok(batch)], schema);
        let mut builder = table.merge_insert(&["section_id"]);
        builder.when_matched_update_all(None);
        builder
            .execute(Box::new(reader))
            .await
            .map_err(|e| StoreError::Other(format!("merge_insert taxa: {e}")))?;
        Ok(())
    }

    /// Write AI-polished results for provisions.
    ///
    /// Uses `merge_insert` keyed on `section_id` — updates only `ai_*` columns
    /// for matched rows.
    pub async fn update_polished(&self, batch: RecordBatch) -> Result<(), StoreError> {
        let table = self.legislation_text().await?;
        let schema = batch.schema();
        let reader = RecordBatchIterator::new(vec![Ok(batch)], schema);
        let mut builder = table.merge_insert(&["section_id"]);
        builder.when_matched_update_all(None);
        builder
            .execute(Box::new(reader))
            .await
            .map_err(|e| StoreError::Other(format!("merge_insert polished: {e}")))?;
        Ok(())
    }

    /// Upsert legislation text (LAT) data received from sertantai.
    ///
    /// If the `legislation_text` table does not exist, creates it from the
    /// incoming batches. If it exists, uses `merge_insert` keyed on
    /// `section_id` to update/insert rows. Never calls `drop_table`.
    ///
    /// Incoming batches are normalized to handle Polars/Explorer type
    /// differences (LargeUtf8→Utf8, Null→nullable Utf8, timezone strings).
    pub async fn upsert_lat(&self, batches: Vec<RecordBatch>) -> Result<usize, StoreError> {
        if batches.is_empty() {
            return Ok(0);
        }

        let batches: Vec<RecordBatch> = batches
            .iter()
            .map(normalize_polars_batch)
            .collect::<Result<_, _>>()?;

        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        let schema = batches[0].schema();

        let existing = self.db.table_names().execute().await?;

        if !existing.contains(&LEGISLATION_TEXT_TABLE.to_string()) {
            info!(
                table = LEGISLATION_TEXT_TABLE,
                rows = total_rows,
                "creating legislation_text table from LAT pull"
            );
            let reader = RecordBatchIterator::new(batches.into_iter().map(Ok), schema);
            self.db
                .create_table(LEGISLATION_TEXT_TABLE, Box::new(reader))
                .execute()
                .await?;
        } else {
            let table = self.legislation_text().await?;
            let reader = RecordBatchIterator::new(batches.into_iter().map(Ok), schema);
            let mut builder = table.merge_insert(&["section_id"]);
            builder.when_matched_update_all(None);
            builder.when_not_matched_insert_all();
            builder
                .execute(Box::new(reader))
                .await
                .map_err(|e| StoreError::Other(format!("merge_insert LAT: {e}")))?;
        }

        info!(rows = total_rows, "upserted LAT data");
        Ok(total_rows)
    }

    /// Upsert embeddings for provisions by section_id.
    ///
    /// The input batch must contain `section_id` (Utf8) and `embedding`
    /// (FixedSizeList<Float32, 384>) columns. Only these columns are
    /// updated — all other columns remain untouched.
    pub async fn upsert_embeddings(&self, batch: &RecordBatch) -> Result<(), StoreError> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let table = self.legislation_text().await?;
        let schema = batch.schema();
        let reader = RecordBatchIterator::new(std::iter::once(Ok(batch.clone())), schema);
        let mut builder = table.merge_insert(&["section_id"]);
        builder.when_matched_update_all(None);
        // Don't insert new rows — only update existing provisions
        builder
            .execute(Box::new(reader))
            .await
            .map_err(|e| StoreError::Other(format!("merge_insert embeddings: {e}")))?;
        info!(rows = batch.num_rows(), "upserted embeddings");
        Ok(())
    }

    /// Compact the legislation_text table to reduce fragment bloat.
    ///
    /// Merges small data fragments created by merge_insert operations.
    /// Should be called periodically during batch enrichment to prevent
    /// disk usage from spiralling (merge_insert creates ~25x write amplification).
    pub async fn compact(&self) -> Result<(), StoreError> {
        let table = self.legislation_text().await?;
        let stats = table
            .optimize(lancedb::table::OptimizeAction::All)
            .await
            .map_err(|e| StoreError::Other(format!("compact: {e}")))?;
        if let Some(c) = stats.compaction {
            info!(
                added = c.files_added,
                removed = c.files_removed,
                "compacted legislation_text"
            );
        }
        Ok(())
    }

    /// Delete all legislation text rows for a given law.
    ///
    /// Uses LanceDB row-level deletion. Returns the number of rows that
    /// existed before deletion. No-op if the table doesn't exist or the
    /// law has no rows (idempotent).
    pub async fn delete_law_lat(&self, law_name: &str) -> Result<usize, StoreError> {
        self.delete_by_law(LEGISLATION_TEXT_TABLE, law_name).await
    }

    /// Delete all amendment annotation rows for a given law.
    ///
    /// Same semantics as [`delete_law_lat`](Self::delete_law_lat).
    pub async fn delete_law_annotations(&self, law_name: &str) -> Result<usize, StoreError> {
        self.delete_by_law(AMENDMENT_ANNOTATIONS_TABLE, law_name)
            .await
    }

    /// List table names in the database.
    pub async fn table_names(&self) -> Result<Vec<String>, StoreError> {
        let names = self.db.table_names().execute().await?;
        Ok(names)
    }

    /// Create (or replace) a table from pre-built RecordBatches.
    ///
    /// Used by the embedding pipeline to write batches with populated embedding columns.
    pub async fn create_table_from_batches(
        &self,
        table_name: &str,
        batches: Vec<RecordBatch>,
    ) -> Result<(), StoreError> {
        if batches.is_empty() {
            return Err(StoreError::Other("no record batches provided".into()));
        }

        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        let schema = batches[0].schema();
        let reader = RecordBatchIterator::new(batches.into_iter().map(Ok), schema);

        let existing = self.db.table_names().execute().await?;
        if existing.contains(&table_name.to_string()) {
            self.db.drop_table(table_name, &[]).await?;
        }

        self.db
            .create_table(table_name, Box::new(reader))
            .execute()
            .await?;

        info!(
            table = table_name,
            rows = total_rows,
            "created LanceDB table from batches"
        );
        Ok(())
    }

    // ── Internal ──

    /// Delete all rows matching `law_name` from the given table.
    async fn delete_by_law(&self, table_name: &str, law_name: &str) -> Result<usize, StoreError> {
        let existing = self.db.table_names().execute().await?;
        if !existing.contains(&table_name.to_string()) {
            return Ok(0);
        }

        let filter = format!("law_name = '{}'", law_name.replace('\'', "''"));
        let table = self.db.open_table(table_name).execute().await?;
        let count = table.count_rows(Some(filter.clone())).await?;

        if count > 0 {
            table
                .delete(&filter)
                .await
                .map_err(|e| StoreError::Other(format!("delete from {table_name}: {e}")))?;
            info!(table = table_name, law_name, rows = count, "deleted rows");
        }

        Ok(count)
    }

    async fn create_table_from_parquet(
        &self,
        table_name: &str,
        parquet_path: &Path,
    ) -> Result<(), StoreError> {
        if !parquet_path.exists() {
            return Err(StoreError::ParquetNotFound(parquet_path.to_path_buf()));
        }

        let batches = read_parquet(parquet_path)?;
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();

        if batches.is_empty() {
            return Err(StoreError::Other(format!(
                "no record batches in {parquet_path:?}"
            )));
        }

        let schema = batches[0].schema();
        let reader = RecordBatchIterator::new(batches.into_iter().map(Ok), schema);

        // Drop existing table if it exists, then create fresh.
        let existing = self.db.table_names().execute().await?;
        if existing.contains(&table_name.to_string()) {
            self.db.drop_table(table_name, &[]).await?;
        }

        self.db
            .create_table(table_name, Box::new(reader))
            .execute()
            .await?;

        info!(
            table = table_name,
            rows = total_rows,
            "created LanceDB table"
        );
        Ok(())
    }
}

/// Normalize a RecordBatch from Polars/Explorer to be LanceDB-compatible.
///
/// Handles three common mismatches:
/// - `LargeUtf8` → `Utf8` (Polars default string type)
/// - `Null`-typed columns → nullable `Utf8` (all-null columns in Polars)
/// - Timezone `Etc/UTC` → `UTC` (Polars timezone string convention)
fn normalize_polars_batch(batch: &RecordBatch) -> Result<RecordBatch, StoreError> {
    let schema = batch.schema();
    let mut fields = Vec::with_capacity(schema.fields().len());
    let mut columns = Vec::with_capacity(batch.num_columns());

    for (i, field) in schema.fields().iter().enumerate() {
        let col = batch.column(i);
        match field.data_type() {
            DataType::LargeUtf8 => {
                fields.push(Arc::new(Field::new(
                    field.name(),
                    DataType::Utf8,
                    field.is_nullable(),
                )));
                columns.push(cast(col, &DataType::Utf8).map_err(|e| {
                    StoreError::Other(format!("cast LargeUtf8→Utf8 for `{}`: {e}", field.name()))
                })?);
            }
            DataType::Null => {
                // All-null column — promote to nullable Utf8.
                fields.push(Arc::new(Field::new(field.name(), DataType::Utf8, true)));
                columns.push(new_null_array(&DataType::Utf8, batch.num_rows()));
            }
            DataType::Timestamp(unit, Some(tz)) if tz.as_ref() != "UTC" => {
                let target = DataType::Timestamp(*unit, Some("UTC".into()));
                fields.push(Arc::new(Field::new(
                    field.name(),
                    target.clone(),
                    field.is_nullable(),
                )));
                columns.push(cast(col, &target).map_err(|e| {
                    StoreError::Other(format!("cast timezone for `{}`: {e}", field.name()))
                })?);
            }
            _ => {
                fields.push(Arc::new(field.as_ref().clone()));
                columns.push(col.clone());
            }
        }
    }

    let new_schema = Arc::new(Schema::new(fields));
    RecordBatch::try_new(new_schema, columns)
        .map_err(|e| StoreError::Other(format!("normalize batch: {e}")))
}

/// Read a Parquet file into Arrow RecordBatches.
pub fn read_parquet(path: &Path) -> Result<Vec<RecordBatch>, StoreError> {
    let file = std::fs::File::open(path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;
    let batches: Result<Vec<RecordBatch>, _> = reader.collect();
    Ok(batches?)
}

#[async_trait::async_trait]
impl crate::ProvisionStore for LanceStore {
    async fn query_legislation_text(
        &self,
        law_name: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        let filter = if law_name.is_empty() {
            "true".to_string()
        } else {
            format!("law_name = '{}'", law_name.replace('\'', "''"))
        };
        self.query_legislation_text(&filter, limit, offset).await
    }

    async fn query_provision_taxa(
        &self,
        law_name: &str,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        self.query_provision_taxa(law_name).await
    }

    async fn upsert_lat(&self, batches: Vec<RecordBatch>) -> Result<usize, StoreError> {
        self.upsert_lat(batches).await
    }

    async fn upsert_embeddings(&self, batch: &RecordBatch) -> Result<(), StoreError> {
        self.upsert_embeddings(batch).await
    }

    async fn update_taxa(&self, batch: RecordBatch) -> Result<(), StoreError> {
        self.update_taxa(batch).await
    }

    async fn update_polished(&self, batch: RecordBatch) -> Result<(), StoreError> {
        self.update_polished(batch).await
    }

    async fn compact(&self) -> Result<(), StoreError> {
        self.compact().await
    }

    async fn ensure_gap_c_columns(&self) -> Result<(), StoreError> {
        self.ensure_gap_c_columns().await
    }

    async fn delete_law_lat(&self, law_name: &str) -> Result<usize, StoreError> {
        self.delete_law_lat(law_name).await
    }

    async fn legislation_text_count(&self) -> Result<usize, StoreError> {
        self.legislation_text_count().await
    }

    async fn delete_law_annotations(&self, law_name: &str) -> Result<usize, StoreError> {
        self.delete_law_annotations(law_name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn data_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("data")
    }

    fn require_lat_data() -> PathBuf {
        let dir = data_dir();
        let lat = dir.join("legislation_text.parquet");
        let ann = dir.join("amendment_annotations.parquet");
        if !lat.exists() || !ann.exists() {
            panic!(
                "LAT data not found. Run: duckdb < data/export_lat.sql\n  Expected: {:?}",
                dir
            );
        }
        dir
    }

    #[tokio::test]
    async fn open_creates_database() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();
        let names = store.table_names().await.unwrap();
        assert!(names.is_empty());
    }

    #[tokio::test]
    async fn create_legislation_text() {
        let dir = require_lat_data();
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();

        store
            .create_legislation_text(&dir.join("legislation_text.parquet"))
            .await
            .unwrap();

        let count = store.legislation_text_count().await.unwrap();
        assert!(
            count > 90_000,
            "expected >90K legislation_text rows, got {count}"
        );

        let names = store.table_names().await.unwrap();
        assert!(names.contains(&"legislation_text".to_string()));
    }

    #[tokio::test]
    async fn create_amendment_annotations() {
        let dir = require_lat_data();
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();

        store
            .create_amendment_annotations(&dir.join("amendment_annotations.parquet"))
            .await
            .unwrap();

        let count = store.amendment_annotations_count().await.unwrap();
        assert!(count > 15_000, "expected >15K annotation rows, got {count}");
    }

    #[tokio::test]
    async fn load_all_creates_both_tables() {
        let dir = require_lat_data();
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();

        store.load_all(&dir).await.unwrap();

        let names = store.table_names().await.unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"legislation_text".to_string()));
        assert!(names.contains(&"amendment_annotations".to_string()));
    }

    #[tokio::test]
    async fn query_legislation_text_by_law() {
        let dir = require_lat_data();
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();
        store
            .create_legislation_text(&dir.join("legislation_text.parquet"))
            .await
            .unwrap();

        let batches = store
            .query_legislation_text("law_name = 'UK_ukpga_1974_37'", 1000, 0)
            .await
            .unwrap();

        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert!(
            total_rows > 100,
            "HSWA 1974 should have >100 text rows, got {total_rows}"
        );
    }

    #[tokio::test]
    async fn missing_parquet_errors() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();

        let result = store
            .create_legislation_text(Path::new("/nonexistent/file.parquet"))
            .await;
        assert!(matches!(result, Err(StoreError::ParquetNotFound(_))));
    }

    #[tokio::test]
    async fn reload_replaces_table() {
        let dir = require_lat_data();
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();

        // Load once.
        store
            .create_legislation_text(&dir.join("legislation_text.parquet"))
            .await
            .unwrap();
        let count1 = store.legislation_text_count().await.unwrap();

        // Load again — should replace, not append.
        store
            .create_legislation_text(&dir.join("legislation_text.parquet"))
            .await
            .unwrap();
        let count2 = store.legislation_text_count().await.unwrap();

        assert_eq!(count1, count2);
    }

    #[tokio::test]
    async fn legislation_text_schema_has_expected_columns() {
        let dir = require_lat_data();
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();
        store
            .create_legislation_text(&dir.join("legislation_text.parquet"))
            .await
            .unwrap();

        let table = store.legislation_text().await.unwrap();
        let schema = table.schema().await.unwrap();

        // Key columns must exist.
        assert!(schema.field_with_name("section_id").is_ok());
        assert!(schema.field_with_name("law_name").is_ok());
        assert!(schema.field_with_name("text").is_ok());
        assert!(schema.field_with_name("sort_key").is_ok());
        assert!(schema.field_with_name("embedding").is_ok());
    }

    #[tokio::test]
    async fn delete_law_lat_removes_only_target_law() {
        let dir = require_lat_data();
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();
        store
            .create_legislation_text(&dir.join("legislation_text.parquet"))
            .await
            .unwrap();

        let total_before = store.legislation_text_count().await.unwrap();
        let law = "UK_ukpga_1974_37";

        // Count rows for this specific law.
        let law_rows = store
            .query_legislation_text(&format!("law_name = '{law}'"), 100_000, 0)
            .await
            .unwrap()
            .iter()
            .map(|b| b.num_rows())
            .sum::<usize>();
        assert!(
            law_rows > 100,
            "HSWA 1974 should have >100 rows, got {law_rows}"
        );

        // Delete.
        let deleted = store.delete_law_lat(law).await.unwrap();
        assert_eq!(deleted, law_rows);

        // Verify target law is gone.
        let remaining = store
            .query_legislation_text(&format!("law_name = '{law}'"), 100_000, 0)
            .await
            .unwrap()
            .iter()
            .map(|b| b.num_rows())
            .sum::<usize>();
        assert_eq!(remaining, 0, "deleted law should have 0 rows");

        // Verify other laws remain.
        let total_after = store.legislation_text_count().await.unwrap();
        assert_eq!(total_after, total_before - law_rows);

        // Idempotent re-delete returns 0.
        let deleted_again = store.delete_law_lat(law).await.unwrap();
        assert_eq!(deleted_again, 0);
    }

    #[tokio::test]
    async fn delete_law_lat_no_table_returns_zero() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test_lancedb");
        let store = LanceStore::open(&db_path).await.unwrap();

        // No table exists — should return 0, not error.
        let deleted = store.delete_law_lat("UK_ukpga_1974_37").await.unwrap();
        assert_eq!(deleted, 0);
    }
}
