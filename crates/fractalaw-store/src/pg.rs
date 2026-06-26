//! PostgreSQL + pgvector store for hub-side provision data.
//!
//! Replaces LanceDB for write-heavy hub operations. LanceDB remains
//! for edge (embedded, read-only synced slices).

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, Float32Array, Float32Builder, Int32Builder,
    StringBuilder, ListBuilder, FixedSizeListBuilder,
    RecordBatch,
};
use arrow::datatypes::{DataType, Field, Schema};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tracing::info;

use crate::StoreError;

/// PostgreSQL + pgvector provision store.
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    /// Connect to PostgreSQL.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(|e| StoreError::Other(format!("pg connect: {e}")))?;

        info!(url = url, "connected to PostgreSQL");
        Ok(Self { pool })
    }

    /// Count rows in legislation_text.
    pub async fn legislation_text_count(&self) -> Result<usize, StoreError> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM legislation_text")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StoreError::Other(format!("count: {e}")))?;
        Ok(row.0 as usize)
    }

    /// Query provisions with a WHERE clause, returning Arrow RecordBatch.
    pub async fn query_legislation_text(
        &self,
        law_name: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        // Use parameterised query — no raw SQL injection
        let rows = if law_name.is_empty() {
            sqlx::query("SELECT * FROM legislation_text ORDER BY sort_key LIMIT $1 OFFSET $2")
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query(
                "SELECT * FROM legislation_text WHERE law_name = $1 ORDER BY sort_key LIMIT $2 OFFSET $3"
            )
            .bind(law_name)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| StoreError::Other(format!("query_legislation_text: {e}")))?;

        if rows.is_empty() {
            return Ok(vec![]);
        }

        let batch = pg_rows_to_record_batch(&rows)?;
        Ok(vec![batch])
    }

    /// Query provision taxa for zenoh publish.
    pub async fn query_provision_taxa(
        &self,
        law_name: &str,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        let rows = sqlx::query(
            "SELECT section_id, drrp_types, duty_family, duty_sub_type, popimar, purposes, \
             clause_refined, taxa_confidence, taxa_classified_at, \
             fitness_polarity, fitness_person, fitness_process, fitness_place, \
             fitness_plant, fitness_property, fitness_sector, \
             extraction_method, holder_inferred_from, ancestor_distance, actors \
             FROM legislation_text \
             WHERE law_name = $1 AND extraction_method IS NOT NULL"
        )
        .bind(law_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StoreError::Other(format!("query_provision_taxa: {e}")))?;

        if rows.is_empty() {
            return Ok(vec![]);
        }

        let batch = pg_rows_to_record_batch(&rows)?;
        Ok(vec![batch])
    }

    /// Vector similarity search.
    pub async fn search_text(
        &self,
        query_embedding: &[f32],
        law_name: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        let vec = pgvector::Vector::from(query_embedding.to_vec());

        let rows = if let Some(law) = law_name {
            sqlx::query(
                "SELECT *, 1 - (embedding <=> $1::vector) AS similarity \
                 FROM legislation_text \
                 WHERE law_name = $2 AND embedding IS NOT NULL \
                 ORDER BY embedding <=> $1::vector \
                 LIMIT $3"
            )
            .bind(&vec)
            .bind(law)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                "SELECT *, 1 - (embedding <=> $1::vector) AS similarity \
                 FROM legislation_text \
                 WHERE embedding IS NOT NULL \
                 ORDER BY embedding <=> $1::vector \
                 LIMIT $2"
            )
            .bind(&vec)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| StoreError::Other(format!("search_text: {e}")))?;

        if rows.is_empty() {
            return Ok(vec![]);
        }

        let batch = pg_rows_to_record_batch(&rows)?;
        Ok(vec![batch])
    }

    /// Upsert provisions (LAT from sertantai).
    pub async fn upsert_lat(&self, batches: Vec<RecordBatch>) -> Result<usize, StoreError> {
        let mut total = 0usize;
        for batch in &batches {
            // Filter out rows where law_name is null (Postgres NOT NULL constraint)
            if let Some(law_col) = batch.column_by_name("law_name") {
                let not_null = arrow::compute::is_not_null(law_col)
                    .map_err(|e| StoreError::Other(format!("filter null law_name: {e}")))?;
                let filtered = arrow::compute::filter_record_batch(batch, &not_null)
                    .map_err(|e| StoreError::Other(format!("filter batch: {e}")))?;
                if filtered.num_rows() > 0 {
                    total += upsert_record_batch(&self.pool, &filtered, "section_id").await?;
                }
            } else {
                total += upsert_record_batch(&self.pool, batch, "section_id").await?;
            }
        }
        Ok(total)
    }

    /// Update embeddings for existing provisions.
    pub async fn upsert_embeddings(&self, batch: &RecordBatch) -> Result<(), StoreError> {
        update_record_batch(&self.pool, batch, "section_id").await?;
        Ok(())
    }

    /// Upsert taxa classification results.
    pub async fn update_taxa(&self, batch: RecordBatch) -> Result<(), StoreError> {
        update_record_batch(&self.pool, &batch, "section_id").await?;
        Ok(())
    }

    /// Upsert polished AI results.
    pub async fn update_polished(&self, batch: RecordBatch) -> Result<(), StoreError> {
        update_record_batch(&self.pool, &batch, "section_id").await?;
        Ok(())
    }

    /// No-op: Postgres doesn't need compaction.
    pub async fn compact(&self) -> Result<(), StoreError> {
        Ok(())
    }

    /// Ensure columns exist (no-op if schema is already correct).
    pub async fn ensure_gap_c_columns(&self) -> Result<(), StoreError> {
        // Postgres schema is created upfront via pg_schema.sql.
        // All columns already exist. No-op.
        Ok(())
    }

    /// Write classifier actor predictions to cls_actors column.
    /// Input: Vec of (section_id, json_string) pairs.
    pub async fn write_cls_actors(&self, updates: &[(String, String)]) -> Result<(), StoreError> {
        for (sid, json) in updates {
            sqlx::query("UPDATE legislation_text SET cls_actors = $1::jsonb WHERE section_id = $2")
                .bind(json)
                .bind(sid)
                .execute(&self.pool)
                .await
                .map_err(|e| StoreError::Other(format!("write_cls_actors: {e}")))?;
        }
        Ok(())
    }

    /// Upsert per-actor signals into provision_actors.
    /// Each tuple: (section_id, actor_label, actor_category, drrp, position, tier)
    pub async fn upsert_provision_actors(
        &self,
        actors: &[(String, String, String, Option<String>, String, String)],
    ) -> Result<(), StoreError> {
        for (sid, label, category, drrp, position, tier) in actors {
            let (drrp_col, pos_col) = match tier.as_str() {
                "regex" => ("regex_drrp", "regex_position"),
                "classifier" => ("cls_drrp", "cls_position"),
                "llm" => ("llm_drrp", "llm_position"),
                _ => continue,
            };
            let sql = format!(
                "INSERT INTO provision_actors (section_id, actor_label, actor_category, {drrp_col}, {pos_col}) \
                 VALUES ($1, $2, $3, $4, $5) \
                 ON CONFLICT (section_id, actor_label) DO UPDATE SET \
                 actor_category = COALESCE(EXCLUDED.actor_category, provision_actors.actor_category), \
                 {drrp_col} = EXCLUDED.{drrp_col}, {pos_col} = EXCLUDED.{pos_col}"
            );
            sqlx::query(&sql)
                .bind(sid)
                .bind(label)
                .bind(category)
                .bind(drrp.as_deref())
                .bind(position)
                .execute(&self.pool)
                .await
                .map_err(|e| StoreError::Other(format!("upsert_provision_actors: {e}")))?;
        }
        Ok(())
    }

    /// Copy drrp_types/actors → regex_drrp/regex_actors for a law.
    /// Snapshots ALL provisions with taxa data (not just extraction_method=regex),
    /// because the classifier may change extraction_method later.
    pub async fn snapshot_regex_signals(&self, law_name: &str) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE legislation_text SET regex_drrp = drrp_types, regex_actors = actors \
             WHERE law_name = $1 AND (drrp_types IS NOT NULL OR actors IS NOT NULL)"
        )
        .bind(law_name)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Other(format!("snapshot_regex: {e}")))?;
        Ok(())
    }

    /// Copy drrp_types/actors → cls_drrp/cls_actors after classifier runs.
    /// Snapshots all provisions that have classifier-tier or higher extraction_method.
    pub async fn snapshot_classifier_signals(&self, law_name: &str) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE legislation_text SET cls_drrp = drrp_types, cls_actors = actors, \
             cls_confidence = taxa_confidence \
             WHERE law_name = $1 AND extraction_method IN ('classifier', 'pending_llm') \
             AND (drrp_types IS NOT NULL OR actors IS NOT NULL)"
        )
        .bind(law_name)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Other(format!("snapshot_classifier: {e}")))?;
        Ok(())
    }

    /// Copy drrp_types/actors → llm_drrp/llm_actors for LLM-validated provisions.
    pub async fn snapshot_llm_signals(&self, law_name: &str) -> Result<(), StoreError> {
        sqlx::query(
            "UPDATE legislation_text SET llm_drrp = drrp_types, llm_actors = actors \
             WHERE law_name = $1 AND extraction_method IN ('agentic', 'agentic_unvalidated')"
        )
        .bind(law_name)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Other(format!("snapshot_llm: {e}")))?;
        Ok(())
    }

    /// Delete provisions for a law.
    pub async fn delete_law_lat(&self, law_name: &str) -> Result<usize, StoreError> {
        let result = sqlx::query("DELETE FROM legislation_text WHERE law_name = $1")
            .bind(law_name)
            .execute(&self.pool)
            .await
            .map_err(|e| StoreError::Other(format!("delete: {e}")))?;
        Ok(result.rows_affected() as usize)
    }
}

// ── Arrow ↔ Postgres conversion ─────────────────────────────────────

/// Convert sqlx PgRows to an Arrow RecordBatch.
///
/// Dynamically builds the schema from the column metadata in the first row.
/// Handles TEXT, INTEGER, REAL, TEXT[], JSONB, vector(384), TIMESTAMPTZ.
fn pg_rows_to_record_batch(rows: &[sqlx::postgres::PgRow]) -> Result<RecordBatch, StoreError> {
    use sqlx::Column;
    use sqlx::TypeInfo;

    if rows.is_empty() {
        return Err(StoreError::Other("no rows".into()));
    }

    let columns = rows[0].columns();
    let mut fields = Vec::with_capacity(columns.len());
    let mut arrays: Vec<ArrayRef> = Vec::with_capacity(columns.len());

    for col in columns {
        let name = col.name();
        let type_name = col.type_info().name();

        match type_name {
            "TEXT" | "VARCHAR" => {
                let mut builder = StringBuilder::new();
                for row in rows {
                    let val: Option<String> = row.try_get(name)
                        .unwrap_or(None);
                    match val {
                        Some(v) => builder.append_value(v),
                        None => builder.append_null(),
                    }
                }
                fields.push(Field::new(name, DataType::Utf8, true));
                arrays.push(Arc::new(builder.finish()));
            }
            "INT4" | "INT8" => {
                let mut builder = Int32Builder::new();
                for row in rows {
                    let val: Option<i32> = row.try_get(name).unwrap_or(None);
                    match val {
                        Some(v) => builder.append_value(v),
                        None => builder.append_null(),
                    }
                }
                fields.push(Field::new(name, DataType::Int32, true));
                arrays.push(Arc::new(builder.finish()));
            }
            "FLOAT4" | "FLOAT8" => {
                let mut builder = Float32Builder::new();
                for row in rows {
                    let val: Option<f32> = row.try_get(name).unwrap_or(None);
                    match val {
                        Some(v) => builder.append_value(v),
                        None => builder.append_null(),
                    }
                }
                fields.push(Field::new(name, DataType::Float32, true));
                arrays.push(Arc::new(builder.finish()));
            }
            "TEXT[]" => {
                let item = Arc::new(Field::new("item", DataType::Utf8, true));
                let mut builder = ListBuilder::new(StringBuilder::new());
                for row in rows {
                    let val: Option<Vec<String>> = row.try_get(name).unwrap_or(None);
                    match val {
                        Some(v) => {
                            for s in &v {
                                builder.values().append_value(s);
                            }
                            builder.append(true);
                        }
                        None => builder.append_null(),
                    }
                }
                fields.push(Field::new(name, DataType::List(item), true));
                arrays.push(Arc::new(builder.finish()));
            }
            "JSONB" | "JSON" => {
                // Store as Utf8 string (serialised JSON)
                let mut builder = StringBuilder::new();
                for row in rows {
                    let val: Option<serde_json::Value> = row.try_get(name).unwrap_or(None);
                    match val {
                        Some(v) => builder.append_value(v.to_string()),
                        None => builder.append_null(),
                    }
                }
                fields.push(Field::new(name, DataType::Utf8, true));
                arrays.push(Arc::new(builder.finish()));
            }
            t if t.starts_with("vector") => {
                // pgvector → FixedSizeList<Float32, 384>
                let item = Arc::new(Field::new("item", DataType::Float32, true));
                let mut builder = FixedSizeListBuilder::new(Float32Builder::new(), 384);
                for row in rows {
                    let val: Option<pgvector::Vector> = row.try_get(name).unwrap_or(None);
                    match val {
                        Some(v) => {
                            let values = builder.values();
                            for &f in v.as_slice() {
                                values.append_value(f);
                            }
                            builder.append(true);
                        }
                        None => {
                            let values = builder.values();
                            for _ in 0..384 {
                                values.append_null();
                            }
                            builder.append(false);
                        }
                    }
                }
                fields.push(Field::new(
                    name,
                    DataType::FixedSizeList(item, 384),
                    true,
                ));
                arrays.push(Arc::new(builder.finish()));
            }
            "TIMESTAMPTZ" | "TIMESTAMP" => {
                // Store as Utf8 ISO string for simplicity
                let mut builder = StringBuilder::new();
                for row in rows {
                    let val: Option<chrono::DateTime<chrono::Utc>> =
                        row.try_get(name).unwrap_or(None);
                    match val {
                        Some(v) => builder.append_value(v.to_rfc3339()),
                        None => builder.append_null(),
                    }
                }
                fields.push(Field::new(name, DataType::Utf8, true));
                arrays.push(Arc::new(builder.finish()));
            }
            "INT4[]" => {
                let item = Arc::new(Field::new("item", DataType::Int32, true));
                let mut builder = ListBuilder::new(Int32Builder::new());
                for row in rows {
                    let val: Option<Vec<i32>> = row.try_get(name).unwrap_or(None);
                    match val {
                        Some(v) => {
                            for i in &v {
                                builder.values().append_value(*i);
                            }
                            builder.append(true);
                        }
                        None => builder.append_null(),
                    }
                }
                fields.push(Field::new(name, DataType::List(item), true));
                arrays.push(Arc::new(builder.finish()));
            }
            _ => {
                // Fallback: read as string
                let mut builder = StringBuilder::new();
                for row in rows {
                    let val: Option<String> = row.try_get(name).unwrap_or(None);
                    match val {
                        Some(v) => builder.append_value(v),
                        None => builder.append_null(),
                    }
                }
                fields.push(Field::new(name, DataType::Utf8, true));
                arrays.push(Arc::new(builder.finish()));
            }
        }
    }

    let schema = Arc::new(Schema::new(fields));
    RecordBatch::try_new(schema, arrays)
        .map_err(|e| StoreError::Other(format!("record_batch: {e}")))
}

/// Upsert an Arrow RecordBatch into legislation_text.
///
/// Converts each row to a dynamic SQL INSERT...ON CONFLICT.
/// The RecordBatch may contain any subset of columns.
async fn upsert_record_batch(
    pool: &PgPool,
    batch: &RecordBatch,
    conflict_key: &str,
) -> Result<usize, StoreError> {
    if batch.num_rows() == 0 {
        return Ok(0);
    }

    let schema = batch.schema();
    let col_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
    let update_set: Vec<String> = col_names
        .iter()
        .filter(|c| **c != conflict_key)
        .map(|c| format!("{c} = EXCLUDED.{c}"))
        .collect();

    // Build per-row SQL with literal values (simple but safe for internal data)
    let mut inserted = 0usize;
    for row in 0..batch.num_rows() {
        let mut values = Vec::with_capacity(col_names.len());
        for (idx, field) in schema.fields().iter().enumerate() {
            let col = batch.column(idx);
            values.push(arrow_value_to_sql(col, row, field.data_type()));
        }

        let sql = format!(
            "INSERT INTO legislation_text ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
            col_names.join(", "),
            values.join(", "),
            conflict_key,
            update_set.join(", "),
        );

        sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|e| StoreError::Other(format!("upsert row {row}: {e}")))?;
        inserted += 1;
    }

    Ok(inserted)
}

/// Update existing rows in legislation_text by key column (e.g. section_id).
/// Only sets columns present in the batch — does not touch other columns.
async fn update_record_batch(
    pool: &PgPool,
    batch: &RecordBatch,
    key_col: &str,
) -> Result<usize, StoreError> {
    if batch.num_rows() == 0 {
        return Ok(0);
    }

    let schema = batch.schema();
    let col_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
    let key_idx = schema
        .index_of(key_col)
        .map_err(|_| StoreError::Other(format!("key column '{key_col}' not in batch")))?;

    let mut updated = 0usize;
    for row in 0..batch.num_rows() {
        let key_val = arrow_value_to_sql(batch.column(key_idx), row, schema.field(key_idx).data_type());

        let set_clauses: Vec<String> = col_names
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != key_idx)
            .map(|(idx, name)| {
                let val = arrow_value_to_sql(batch.column(idx), row, schema.field(idx).data_type());
                format!("{name} = {val}")
            })
            .collect();

        let sql = format!(
            "UPDATE legislation_text SET {} WHERE {key_col} = {key_val}",
            set_clauses.join(", "),
        );

        let result = sqlx::query(&sql)
            .execute(pool)
            .await
            .map_err(|e| StoreError::Other(format!("update row {row}: {e}")))?;
        updated += result.rows_affected() as usize;
    }

    Ok(updated)
}

/// Convert a single Arrow value to a SQL literal string.
fn arrow_value_to_sql(col: &ArrayRef, row: usize, data_type: &DataType) -> String {
    use arrow::array::AsArray;

    if col.is_null(row) {
        return "NULL".to_string();
    }

    match data_type {
        DataType::Utf8 => {
            let val = col.as_string::<i32>().value(row);
            format!("'{}'", val.replace('\'', "''"))
        }
        DataType::LargeUtf8 => {
            let val = col.as_string::<i64>().value(row);
            format!("'{}'", val.replace('\'', "''"))
        }
        DataType::Int32 => {
            let val = col.as_primitive::<arrow::datatypes::Int32Type>().value(row);
            val.to_string()
        }
        DataType::Float32 => {
            let val = col.as_primitive::<arrow::datatypes::Float32Type>().value(row);
            format!("{val}")
        }
        DataType::List(inner) => {
            let list = col.as_list::<i32>();
            let values = list.value(row);
            if values.is_empty() {
                return "NULL".to_string();
            }
            // List<Struct> → JSONB (e.g., actors column)
            if matches!(inner.data_type(), DataType::Struct(_)) {
                let struct_arr = values
                    .as_any()
                    .downcast_ref::<arrow::array::StructArray>()
                    .unwrap();
                let mut json_items = Vec::new();
                for i in 0..struct_arr.len() {
                    let mut obj = serde_json::Map::new();
                    for (fi, field) in struct_arr.fields().iter().enumerate() {
                        let col = struct_arr.column(fi);
                        if col.is_null(i) {
                            obj.insert(field.name().clone(), serde_json::Value::Null);
                        } else if let Some(sa) = col.as_any().downcast_ref::<arrow::array::StringArray>() {
                            obj.insert(field.name().clone(), serde_json::Value::String(sa.value(i).to_string()));
                        }
                    }
                    json_items.push(serde_json::Value::Object(obj));
                }
                let json = serde_json::Value::Array(json_items).to_string();
                return format!("'{}'::JSONB", json.replace('\'', "''"));
            }
            // TEXT[]
            if let Some(sa) = values.as_any().downcast_ref::<arrow::array::StringArray>() {
                let items: Vec<String> = (0..sa.len())
                    .filter(|&i| !sa.is_null(i))
                    .map(|i| format!("'{}'", sa.value(i).replace('\'', "''")))
                    .collect();
                format!("ARRAY[{}]::TEXT[]", items.join(","))
            } else {
                "NULL".to_string()
            }
        }
        DataType::FixedSizeList(_, _) => {
            // vector(384)
            let fsl = col
                .as_any()
                .downcast_ref::<arrow::array::FixedSizeListArray>()
                .unwrap();
            let values = fsl.value(row);
            let floats = values
                .as_any()
                .downcast_ref::<Float32Array>()
                .unwrap();
            let nums: Vec<String> = floats.values().iter().map(|f| format!("{f}")).collect();
            format!("'[{}]'::vector", nums.join(","))
        }
        DataType::Timestamp(unit, _tz) => {
            use arrow::datatypes::TimeUnit;
            // Handle native timestamp arrays (TimestampNanosecondArray etc.)
            let nanos = match unit {
                TimeUnit::Nanosecond => col
                    .as_any()
                    .downcast_ref::<arrow::array::TimestampNanosecondArray>()
                    .map(|a| a.value(row)),
                TimeUnit::Microsecond => col
                    .as_any()
                    .downcast_ref::<arrow::array::TimestampMicrosecondArray>()
                    .map(|a| a.value(row) * 1000),
                TimeUnit::Millisecond => col
                    .as_any()
                    .downcast_ref::<arrow::array::TimestampMillisecondArray>()
                    .map(|a| a.value(row) * 1_000_000),
                TimeUnit::Second => col
                    .as_any()
                    .downcast_ref::<arrow::array::TimestampSecondArray>()
                    .map(|a| a.value(row) * 1_000_000_000),
            };
            if let Some(ns) = nanos {
                let secs = ns / 1_000_000_000;
                let subsec_ns = (ns % 1_000_000_000) as u32;
                if let Some(dt) = chrono::DateTime::from_timestamp(secs, subsec_ns) {
                    return format!("'{}'::TIMESTAMPTZ", dt.to_rfc3339());
                }
            }
            // Fallback: try as string
            if let Some(sa) = col.as_any().downcast_ref::<arrow::array::StringArray>() {
                let val = sa.value(row);
                format!("'{}'::TIMESTAMPTZ", val.replace('\'', "''"))
            } else {
                "NULL".to_string()
            }
        }
        _ => {
            // Fallback: try as string
            if let Some(s) = col.as_any().downcast_ref::<arrow::array::StringArray>() {
                format!("'{}'", s.value(row).replace('\'', "''"))
            } else {
                "NULL".to_string()
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::ProvisionStore for PgStore {
    async fn query_legislation_text(
        &self,
        law_name: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RecordBatch>, StoreError> {
        self.query_legislation_text(law_name, limit, offset).await
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

    async fn write_cls_actors(&self, updates: &[(String, String)]) -> Result<(), StoreError> {
        self.write_cls_actors(updates).await
    }

    async fn upsert_provision_actors(
        &self,
        actors: &[(String, String, String, Option<String>, String, String)],
    ) -> Result<(), StoreError> {
        self.upsert_provision_actors(actors).await
    }

    async fn snapshot_regex_signals(&self, law_name: &str) -> Result<(), StoreError> {
        self.snapshot_regex_signals(law_name).await
    }

    async fn snapshot_classifier_signals(&self, law_name: &str) -> Result<(), StoreError> {
        self.snapshot_classifier_signals(law_name).await
    }

    async fn snapshot_llm_signals(&self, law_name: &str) -> Result<(), StoreError> {
        self.snapshot_llm_signals(law_name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect() {
        // Skip if Postgres not running
        let result = PgStore::connect(
            "postgres://fractalaw:fractalaw@localhost:5433/fractalaw"
        ).await;
        if result.is_err() {
            eprintln!("Postgres not available, skipping test");
            return;
        }
        let store = result.unwrap();
        let count = store.legislation_text_count().await.unwrap();
        assert!(count > 0, "expected rows in legislation_text");
        eprintln!("PgStore: {count} rows");
    }
}
