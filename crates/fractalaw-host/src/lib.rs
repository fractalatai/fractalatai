//! Wasmtime host runtime: micro-app lifecycle, instance pooling, WIT interface bridge.

use std::path::Path;
use wasmtime::component::{Component, HasSelf, ResourceTable};
use wasmtime::{Config, Engine, InstanceAllocationStrategy, PoolingAllocationConfig, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

#[cfg(feature = "duckdb")]
use fractalaw_store::DuckStore;
#[cfg(feature = "lancedb")]
use fractalaw_store::LanceStore;

wasmtime::component::bindgen!({
    world: "micro-app",
    path: "../../wit",
    imports: { default: async },
    exports: { default: async },
});

/// Host-side audit entry with timestamp added by the host.
#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub event_type: String,
    pub resource: String,
    pub detail: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Result of running a micro-app component.
pub struct RunResult {
    pub output: Result<String, String>,
    pub audit_entries: Vec<AuditRecord>,
    pub fuel_consumed: u64,
}

/// State held in the Wasmtime [`Store`](wasmtime::Store) for each guest execution.
pub struct HostState {
    pub audit_entries: Vec<AuditRecord>,
    pub wasi_ctx: WasiCtx,
    pub table: ResourceTable,
    #[cfg(feature = "duckdb")]
    pub duck: Option<DuckStore>,
    #[cfg(feature = "lancedb")]
    pub lance: Option<LanceStore>,
    #[cfg(feature = "onnx")]
    pub extractor: Option<fractalaw_ai::DrrpExtractor>,
}

impl Default for HostState {
    fn default() -> Self {
        Self::new()
    }
}

impl HostState {
    pub fn new() -> Self {
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build();
        Self {
            audit_entries: Vec::new(),
            wasi_ctx,
            table: ResourceTable::new(),
            #[cfg(feature = "duckdb")]
            duck: None,
            #[cfg(feature = "lancedb")]
            lance: None,
            #[cfg(feature = "onnx")]
            extractor: None,
        }
    }

    /// Attach a DuckDB store for data-query and data-mutate host functions.
    #[cfg(feature = "duckdb")]
    pub fn with_duck(mut self, store: DuckStore) -> Self {
        self.duck = Some(store);
        self
    }

    /// Attach a LanceDB store for legislation_text queries.
    #[cfg(feature = "lancedb")]
    pub fn with_lance(mut self, store: LanceStore) -> Self {
        self.lance = Some(store);
        self
    }

    /// Attach an ONNX DRRP extraction model for local inference.
    #[cfg(feature = "onnx")]
    pub fn with_extractor(mut self, extractor: fractalaw_ai::DrrpExtractor) -> Self {
        self.extractor = Some(extractor);
        self
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.table,
        }
    }
}

// ── Audit log host function ──

impl fractal::app::audit_log::Host for HostState {
    async fn record_event(&mut self, entry: fractal::app::audit_log::AuditEntry) {
        let record = AuditRecord {
            event_type: entry.event_type,
            resource: entry.resource,
            detail: entry.detail,
            timestamp: chrono::Utc::now(),
        };
        tracing::info!(
            event_type = %record.event_type,
            resource = %record.resource,
            "audit event recorded"
        );
        self.audit_entries.push(record);
    }
}

// ── Data query host function ──

impl fractal::app::data_query::Host for HostState {
    async fn query(
        &mut self,
        sql: String,
    ) -> Result<Vec<u8>, fractal::app::data_query::QueryError> {
        // Route legislation_text queries to LanceDB when available.
        #[cfg(feature = "lancedb")]
        if sql.contains("legislation_text")
            && let Some(ref lance) = self.lance
        {
            tracing::info!(sql = %sql, "Routing query to LanceDB");
            return lance_query_impl(lance, &sql).await;
        }
        tracing::info!(sql = %sql, "Routing query to DuckDB");
        self.query_impl(&sql)
    }
}

/// Route a legislation_text query to LanceDB.
///
/// Parses the guest's SQL minimally to extract WHERE and LIMIT clauses,
/// then queries LanceDB and returns Arrow IPC bytes.
///
/// Free function (not a method on HostState) to avoid borrowing `&self`
/// across `.await` points, which would trigger Send/Sync requirements
/// on HostState that DuckDB's Connection cannot satisfy.
#[cfg(feature = "lancedb")]
async fn lance_query_impl(
    lance: &LanceStore,
    sql: &str,
) -> Result<Vec<u8>, fractal::app::data_query::QueryError> {
    let sql_upper = sql.to_uppercase();

    // Extract WHERE clause.
    let filter = if let Some(where_pos) = sql_upper.find("WHERE ") {
        let after_where = &sql[where_pos + 6..];
        // WHERE clause ends at LIMIT, ORDER BY, GROUP BY, or end of string.
        let end = ["LIMIT ", "ORDER ", "GROUP "]
            .iter()
            .filter_map(|kw| after_where.to_uppercase().find(kw))
            .min()
            .unwrap_or(after_where.len());
        after_where[..end].trim().to_string()
    } else {
        String::new()
    };

    // Extract LIMIT.
    let limit = if let Some(limit_pos) = sql_upper.find("LIMIT ") {
        let after_limit = &sql[limit_pos + 6..];
        after_limit
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1000)
    } else {
        1000
    };

    // Extract OFFSET.
    let offset = if let Some(offset_pos) = sql_upper.find("OFFSET ") {
        let after_offset = &sql[offset_pos + 7..];
        after_offset
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0)
    } else {
        0
    };

    // Query LanceDB.
    tracing::debug!(filter = %filter, limit = %limit, offset = %offset, "LanceDB query");
    let batches = if filter.is_empty() {
        lance.query_legislation_text("true", limit, offset).await
    } else {
        lance.query_legislation_text(&filter, limit, offset).await
    }
    .map_err(|e| fractal::app::data_query::QueryError {
        code: 2,
        message: format!("LanceDB query failed: {e}"),
    })?;
    tracing::debug!(num_batches = %batches.len(), total_rows = %batches.iter().map(|b| b.num_rows()).sum::<usize>(), "LanceDB query result");

    // Check if the guest wants a to_json() result — single-string IPC.
    if sql_upper.contains("TO_JSON(") {
        return lance_to_json_result(&batches);
    }

    // Check if the guest wants a count.
    if sql_upper.contains("COUNT(") {
        let total: i64 = batches.iter().map(|b| b.num_rows() as i64).sum();
        let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(vec![
            arrow::datatypes::Field::new("count", arrow::datatypes::DataType::Int64, false),
        ]));
        let batch = arrow::record_batch::RecordBatch::try_new(
            schema,
            vec![std::sync::Arc::new(arrow::array::Int64Array::from(vec![
                total,
            ]))],
        )
        .map_err(|e| fractal::app::data_query::QueryError {
            code: 3,
            message: format!("failed to build count result: {e}"),
        })?;
        return encode_ipc(&[batch]).map_err(|e| fractal::app::data_query::QueryError {
            code: 3,
            message: e.to_string(),
        });
    }

    // Return raw Arrow IPC.
    encode_ipc(&batches).map_err(|e| fractal::app::data_query::QueryError {
        code: 3,
        message: e.to_string(),
    })
}

/// Serialise a LanceDB result row to JSON and return as single-string Arrow IPC.
///
/// This handles the guest's `SELECT to_json(struct_pack(...)) FROM ...` pattern.
/// The guest's IPC parser only handles single-row, single-column results.
#[cfg(feature = "lancedb")]
fn lance_to_json_result(
    batches: &[arrow::record_batch::RecordBatch],
) -> Result<Vec<u8>, fractal::app::data_query::QueryError> {
    use arrow::array::Array;

    if batches.is_empty() || batches[0].num_rows() == 0 {
        // Return empty string result.
        let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(vec![
            arrow::datatypes::Field::new("json", arrow::datatypes::DataType::Utf8, false),
        ]));
        let batch = arrow::record_batch::RecordBatch::try_new(
            schema,
            vec![std::sync::Arc::new(arrow::array::StringArray::from(vec![
                "",
            ]))],
        )
        .map_err(|e| fractal::app::data_query::QueryError {
            code: 3,
            message: format!("failed to build empty JSON result: {e}"),
        })?;
        return encode_ipc(&[batch]).map_err(|e| fractal::app::data_query::QueryError {
            code: 3,
            message: e.to_string(),
        });
    }

    // Build JSON object from first row of first batch.
    let batch = &batches[0];
    let schema = batch.schema();
    let mut map = serde_json::Map::new();

    for (i, field) in schema.fields().iter().enumerate() {
        let col = batch.column(i);
        if col.is_null(0) {
            map.insert(field.name().clone(), serde_json::Value::Null);
            continue;
        }
        let value = match field.data_type() {
            arrow::datatypes::DataType::Utf8 => {
                let arr = col
                    .as_any()
                    .downcast_ref::<arrow::array::StringArray>()
                    .unwrap();
                serde_json::Value::String(arr.value(0).to_string())
            }
            arrow::datatypes::DataType::Float32 => {
                let arr = col
                    .as_any()
                    .downcast_ref::<arrow::array::Float32Array>()
                    .unwrap();
                serde_json::json!(arr.value(0))
            }
            arrow::datatypes::DataType::Int32 => {
                let arr = col
                    .as_any()
                    .downcast_ref::<arrow::array::Int32Array>()
                    .unwrap();
                serde_json::json!(arr.value(0))
            }
            arrow::datatypes::DataType::List(_) => {
                // List<Utf8> — serialize as JSON array of strings.
                let list_arr = col
                    .as_any()
                    .downcast_ref::<arrow::array::ListArray>()
                    .unwrap();
                let values = list_arr.value(0);
                tracing::debug!(field = %field.name(), list_len = %values.len(), "processing List column");
                if let Some(str_arr) = values.as_any().downcast_ref::<arrow::array::StringArray>() {
                    let items: Vec<serde_json::Value> = (0..str_arr.len())
                        .map(|j| serde_json::Value::String(str_arr.value(j).to_string()))
                        .collect();
                    serde_json::Value::Array(items)
                } else {
                    tracing::warn!(field = %field.name(), "List column is not StringArray");
                    serde_json::Value::Null
                }
            }
            _ => {
                // Fallback: try to format as string.
                serde_json::Value::String(format!("{:?}", col))
            }
        };
        map.insert(field.name().clone(), value);
    }

    // Debug: log drrp_types from JSON
    if let Some(serde_json::Value::Array(arr)) = map.get("drrp_types") {
        tracing::debug!(drrp_types_len = %arr.len(), "lance_to_json drrp_types");
    }

    let json_str = serde_json::to_string(&serde_json::Value::Object(map)).map_err(|e| {
        fractal::app::data_query::QueryError {
            code: 3,
            message: format!("JSON serialization failed: {e}"),
        }
    })?;

    // Wrap in single-string Arrow IPC.
    let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(vec![
        arrow::datatypes::Field::new("json", arrow::datatypes::DataType::Utf8, false),
    ]));
    let batch = arrow::record_batch::RecordBatch::try_new(
        schema,
        vec![std::sync::Arc::new(arrow::array::StringArray::from(vec![
            json_str.as_str(),
        ]))],
    )
    .map_err(|e| fractal::app::data_query::QueryError {
        code: 3,
        message: format!("failed to build JSON IPC result: {e}"),
    })?;
    encode_ipc(&[batch]).map_err(|e| fractal::app::data_query::QueryError {
        code: 3,
        message: e.to_string(),
    })
}

/// Route an UPDATE on legislation_text to LanceDB.
///
/// Parses `UPDATE legislation_text SET col1 = val1, ... WHERE filter`.
///
/// Free function to avoid Send/Sync issues (same reason as lance_query_impl).
#[cfg(feature = "lancedb")]
async fn lance_execute_impl(
    lance: &LanceStore,
    sql: &str,
) -> Result<u64, fractal::app::data_mutate::MutateError> {
    let sql_upper = sql.to_uppercase();

    if !sql_upper.starts_with("UPDATE ") {
        // Only UPDATE is supported for legislation_text mutations via LanceDB.
        // DDL like CREATE TABLE is silently accepted (no-op) since the table already exists.
        if sql_upper.starts_with("CREATE ") || sql_upper.starts_with("ALTER ") {
            return Ok(0);
        }
        return Err(fractal::app::data_mutate::MutateError {
            code: 2,
            message: format!(
                "unsupported LanceDB mutation: {}",
                &sql[..sql.len().min(80)]
            ),
        });
    }

    // Parse SET clause.
    let set_pos = sql_upper
        .find("SET ")
        .ok_or(fractal::app::data_mutate::MutateError {
            code: 2,
            message: "UPDATE without SET clause".into(),
        })?;
    let after_set = &sql[set_pos + 4..];

    // WHERE clause.
    let (set_part, filter) = if let Some(where_pos) = after_set.to_uppercase().find("WHERE ") {
        (&after_set[..where_pos], after_set[where_pos + 6..].trim())
    } else {
        (after_set, "true")
    };

    let table =
        lance
            .legislation_text()
            .await
            .map_err(|e| fractal::app::data_mutate::MutateError {
                code: 2,
                message: format!("failed to open legislation_text: {e}"),
            })?;

    let mut update = table.update().only_if(filter);

    // Parse comma-separated SET assignments: col = val, col = val, ...
    // Handle quoted strings containing commas by tracking quote state.
    let mut assignments = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in set_part.chars() {
        match ch {
            '\'' if !in_quotes => {
                in_quotes = true;
                current.push(ch);
            }
            '\'' if in_quotes => {
                in_quotes = false;
                current.push(ch);
            }
            ',' if !in_quotes => {
                assignments.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        assignments.push(current.trim().to_string());
    }

    for assignment in &assignments {
        let parts: Vec<&str> = assignment.splitn(2, '=').collect();
        if parts.len() != 2 {
            continue;
        }
        let col_name = parts[0].trim();
        let col_value = parts[1].trim();
        update = update.column(col_name, col_value);
    }

    let result = update
        .execute()
        .await
        .map_err(|e| fractal::app::data_mutate::MutateError {
            code: 2,
            message: format!("LanceDB update failed: {e}"),
        })?;

    Ok(result.rows_updated)
}

impl HostState {
    fn query_impl(&self, sql: &str) -> Result<Vec<u8>, fractal::app::data_query::QueryError> {
        #[cfg(feature = "duckdb")]
        {
            let duck = self
                .duck
                .as_ref()
                .ok_or(fractal::app::data_query::QueryError {
                    code: 1,
                    message: "no DuckDB store attached".into(),
                })?;
            let batches =
                duck.query_arrow(sql)
                    .map_err(|e| fractal::app::data_query::QueryError {
                        code: 2,
                        message: e.to_string(),
                    })?;
            encode_ipc(&batches).map_err(|e| fractal::app::data_query::QueryError {
                code: 3,
                message: e.to_string(),
            })
        }

        #[cfg(not(feature = "duckdb"))]
        {
            let _ = sql;
            Err(fractal::app::data_query::QueryError {
                code: 1,
                message: "DuckDB support not compiled in".into(),
            })
        }
    }
}

// ── Data mutate host function ──

impl fractal::app::data_mutate::Host for HostState {
    async fn insert(
        &mut self,
        table: String,
        data: Vec<u8>,
    ) -> Result<u64, fractal::app::data_mutate::MutateError> {
        self.insert_impl(&table, &data)
    }

    async fn execute(
        &mut self,
        sql: String,
    ) -> Result<u64, fractal::app::data_mutate::MutateError> {
        // Route legislation_text mutations to LanceDB when available.
        #[cfg(feature = "lancedb")]
        if sql.contains("legislation_text")
            && let Some(ref lance) = self.lance
        {
            return lance_execute_impl(lance, &sql).await;
        }
        self.execute_impl(&sql)
    }
}

impl HostState {
    fn insert_impl(
        &self,
        table: &str,
        data: &[u8],
    ) -> Result<u64, fractal::app::data_mutate::MutateError> {
        #[cfg(feature = "duckdb")]
        {
            let duck = self
                .duck
                .as_ref()
                .ok_or(fractal::app::data_mutate::MutateError {
                    code: 1,
                    message: "no DuckDB store attached".into(),
                })?;
            let batches = decode_ipc(data).map_err(|e| fractal::app::data_mutate::MutateError {
                code: 2,
                message: format!("failed to decode Arrow IPC: {e}"),
            })?;
            let mut total_rows = 0u64;
            for batch in &batches {
                duck.insert_batch(table, batch).map_err(|e| {
                    fractal::app::data_mutate::MutateError {
                        code: 3,
                        message: e.to_string(),
                    }
                })?;
                total_rows += batch.num_rows() as u64;
            }
            Ok(total_rows)
        }

        #[cfg(not(feature = "duckdb"))]
        {
            let _ = (table, data);
            Err(fractal::app::data_mutate::MutateError {
                code: 1,
                message: "DuckDB support not compiled in".into(),
            })
        }
    }

    fn execute_impl(&self, sql: &str) -> Result<u64, fractal::app::data_mutate::MutateError> {
        #[cfg(feature = "duckdb")]
        {
            let duck = self
                .duck
                .as_ref()
                .ok_or(fractal::app::data_mutate::MutateError {
                    code: 1,
                    message: "no DuckDB store attached".into(),
                })?;
            duck.execute(sql)
                .map_err(|e| fractal::app::data_mutate::MutateError {
                    code: 2,
                    message: e.to_string(),
                })?;
            Ok(0)
        }

        #[cfg(not(feature = "duckdb"))]
        {
            let _ = sql;
            Err(fractal::app::data_mutate::MutateError {
                code: 1,
                message: "DuckDB support not compiled in".into(),
            })
        }
    }
}

// ── AI embeddings host function (stub) ──

impl fractal::app::ai_embeddings::Host for HostState {
    async fn embed(
        &mut self,
        _text: String,
    ) -> Result<Vec<f32>, fractal::app::ai_embeddings::AiError> {
        Err(fractal::app::ai_embeddings::AiError {
            code: 1,
            message: "embeddings not configured — use fractalaw embed CLI instead".into(),
        })
    }

    async fn embed_batch(
        &mut self,
        _texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, fractal::app::ai_embeddings::AiError> {
        Err(fractal::app::ai_embeddings::AiError {
            code: 1,
            message: "embeddings not configured — use fractalaw embed CLI instead".into(),
        })
    }
}

// ── AI inference host function ──

impl fractal::app::ai_inference::Host for HostState {
    async fn generate(
        &mut self,
        request: fractal::app::ai_inference::GenerateRequest,
    ) -> Result<fractal::app::ai_inference::GenerateResponse, fractal::app::ai_embeddings::AiError>
    {
        self.generate_impl(request).await
    }
}

impl HostState {
    async fn generate_impl(
        &mut self,
        request: fractal::app::ai_inference::GenerateRequest,
    ) -> Result<fractal::app::ai_inference::GenerateResponse, fractal::app::ai_embeddings::AiError>
    {
        // Try ONNX backend first (local-first).
        #[cfg(feature = "onnx")]
        if let Some(ref mut extractor) = self.extractor {
            tracing::debug!(prompt_len = %request.user_prompt.len(), "attempting to parse DRRP prompt");

            // Debug: dump first prompt to file
            static FIRST_PROMPT: std::sync::Once = std::sync::Once::new();
            FIRST_PROMPT.call_once(|| {
                let _ = std::fs::write("/tmp/drrp_prompt.txt", &request.user_prompt);
                eprintln!("Dumped first prompt to /tmp/drrp_prompt.txt");
            });

            if let Some(parsed) = parse_drrp_prompt(&request.user_prompt) {
                tracing::debug!(drrp_type = %parsed.drrp_type, holder = %parsed.holder, "parsed DRRP prompt successfully");
                let extraction = extractor
                    .extract(
                        &parsed.drrp_type,
                        &parsed.holder,
                        &parsed.source_text,
                        &parsed.article,
                    )
                    .map_err(|e| fractal::app::ai_embeddings::AiError {
                        code: 4,
                        message: format!("ONNX extraction failed: {e}"),
                    })?;

                let confidence = extraction.confidence;
                let text = extraction.to_json();

                tracing::info!(
                    holder = %extraction.holder,
                    confidence,
                    "ONNX DRRP extraction complete"
                );

                return Ok(fractal::app::ai_inference::GenerateResponse {
                    text,
                    tokens_used: 0,
                    confidence,
                });
            }
            // Prompt doesn't match DRRP format — no other backend available.
            tracing::debug!(
                "prompt does not match DRRP format, no other inference backend available"
            );
        }

        // No ONNX extractor or prompt not in DRRP format — error.
        let _ = request;
        Err(fractal::app::ai_embeddings::AiError {
            code: 1,
            message: "no inference backend configured (ONNX model not loaded)".into(),
        })
    }
}

// ── ONNX prompt parsing ──

/// Structured fields parsed from a DRRP polisher user prompt.
#[cfg(feature = "onnx")]
struct ParsedDrrpPrompt {
    drrp_type: String,
    holder: String,
    article: String,
    source_text: String,
}

/// Parse a DRRP polisher user prompt into structured fields.
///
/// Supports both Phase B format (single DRRP type + holder) and Phase C format
/// (taxa context with DRRP types list, actors, refined clause).
///
/// Returns `None` if the prompt doesn't match the expected format.
#[cfg(feature = "onnx")]
fn parse_drrp_prompt(user_prompt: &str) -> Option<ParsedDrrpPrompt> {
    let mut drrp_types = Vec::new();
    let mut holder = None;
    let mut article = None;
    let mut source_text = None;
    let mut refined_clause = None;

    let mut lines = user_prompt.lines().peekable();
    while let Some(line) = lines.next() {
        // Phase B format: "DRRP type: Duty"
        if let Some(val) = line.strip_prefix("DRRP type: ") {
            drrp_types.push(val.to_string());
        }
        // Phase C format: "DRRP types: Duty, Right"
        else if let Some(val) = line.strip_prefix("DRRP types: ") {
            drrp_types.extend(val.split(',').map(|s| s.trim().to_string()));
        }
        // Phase B format: "Article reference: s.2(1)"
        else if let Some(val) = line.strip_prefix("Article reference: ") {
            article = Some(val.to_string());
        }
        // Phase C format: "Provision: s.2(1)"
        else if let Some(val) = line.strip_prefix("Provision: ") {
            article = Some(val.to_string());
        }
        // Phase B format: "- Holder: every employer"
        else if let Some(val) = line.strip_prefix("- Holder: ") {
            holder = Some(val.to_string());
        }
        // Phase C format: "Governed actors: Org: Employer, ..."
        else if let Some(val) = line.strip_prefix("Governed actors: ") {
            if holder.is_none() {
                holder = Some(
                    val.split(',')
                        .next()
                        .unwrap_or("unknown")
                        .trim()
                        .to_string(),
                );
            }
        }
        // Phase C format: "Government actors: Gov: Minister, ..."
        else if let Some(val) = line.strip_prefix("Government actors: ") {
            if holder.is_none() {
                holder = Some(
                    val.split(',')
                        .next()
                        .unwrap_or("unknown")
                        .trim()
                        .to_string(),
                );
            }
        }
        // Phase C format: "Regex-refined clause:"
        else if line.starts_with("Regex-refined clause:") {
            // Read until we hit "Full section text:" or end
            let mut clause_lines = Vec::new();
            while let Some(next_line) = lines.peek() {
                if next_line.starts_with("Full section text:") {
                    break;
                }
                clause_lines.push(lines.next().unwrap().to_string());
            }
            refined_clause = Some(clause_lines.join("\n").trim().to_string());
        }
        // Both formats: "Full section text:"
        else if line.starts_with("Full section text:") {
            let rest: String = lines.collect::<Vec<_>>().join("\n");
            source_text = Some(rest.trim().to_string());
            break;
        }
    }

    // Use refined clause as source if available (Phase C), otherwise fall back to full text
    let text = refined_clause.or(source_text)?;

    // If no DRRP types found, return None (can't run ONNX model without a type)
    let drrp_type = drrp_types.first()?.clone();

    // If no holder found from actors, can't run ONNX model
    let holder = holder?;

    Some(ParsedDrrpPrompt {
        drrp_type,
        holder,
        article: article.unwrap_or_else(|| "unknown".to_string()),
        source_text: text,
    })
}

// ── Arrow IPC encoding/decoding ──

/// Encode Arrow RecordBatches into IPC streaming format bytes.
#[allow(dead_code)]
fn encode_ipc(
    batches: &[arrow::record_batch::RecordBatch],
) -> Result<Vec<u8>, arrow::error::ArrowError> {
    use arrow::ipc::writer::StreamWriter;

    if batches.is_empty() {
        // Return empty IPC stream with no schema — caller handles empty results.
        return Ok(Vec::new());
    }
    let schema = batches[0].schema();
    let mut buf = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut buf, &schema)?;
        for batch in batches {
            writer.write(batch)?;
        }
        writer.finish()?;
    }
    Ok(buf)
}

/// Decode Arrow IPC streaming format bytes into RecordBatches.
#[cfg(feature = "duckdb")]
fn decode_ipc(
    data: &[u8],
) -> Result<Vec<arrow::record_batch::RecordBatch>, arrow::error::ArrowError> {
    use arrow::ipc::reader::StreamReader;
    use std::io::Cursor;

    if data.is_empty() {
        return Ok(Vec::new());
    }
    let reader = StreamReader::try_new(Cursor::new(data), None)?;
    reader.into_iter().collect()
}

/// Create an [`Engine`] configured for micro-app execution.
///
/// - Pooling allocator with pre-allocated instance slots
/// - Fuel metering for deterministic execution budgets
/// - Epoch interruption for wall-clock timeouts
/// - Component model + async support
pub fn create_engine() -> anyhow::Result<Engine> {
    let mut pool = PoolingAllocationConfig::new();
    pool.total_component_instances(16);
    pool.total_memories(32);
    pool.max_memory_size(64 * 1024 * 1024); // 64 MiB per instance

    let mut config = Config::new();
    config.async_support(true);
    config.wasm_component_model(true);
    config.consume_fuel(true);
    config.epoch_interruption(true);
    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pool));

    Engine::new(&config)
}

/// Load and compile a WASM component from disk.
pub async fn load_component(engine: &Engine, path: &Path) -> anyhow::Result<Component> {
    let bytes = tokio::fs::read(path).await?;
    Component::new(engine, &bytes)
}

/// Create a [`wasmtime::component::Linker`] with host functions wired up.
pub fn create_linker(engine: &Engine) -> anyhow::Result<wasmtime::component::Linker<HostState>> {
    let mut linker = wasmtime::component::Linker::new(engine);
    // Wire up our fractal:app host functions
    MicroApp::add_to_linker::<HostState, HasSelf<HostState>>(&mut linker, |state| state)?;
    // Wire up WASI p2 interfaces (cli, io, filesystem, clocks) required by the wasip1 adapter
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    Ok(linker)
}

/// Optional host resources to attach when running a micro-app.
#[derive(Default)]
pub struct RunOptions {
    #[cfg(feature = "duckdb")]
    pub duck: Option<DuckStore>,
    #[cfg(feature = "lancedb")]
    pub lance: Option<LanceStore>,
    #[cfg(feature = "onnx")]
    pub extractor: Option<fractalaw_ai::DrrpExtractor>,
}

/// Load, instantiate, and execute a micro-app component.
///
/// Pass host resources via [`RunOptions`] to enable data and inference host functions.
pub async fn run_component(
    wasm_path: &Path,
    fuel: u64,
    #[allow(unused_variables)] opts: RunOptions,
) -> anyhow::Result<RunResult> {
    let engine = create_engine()?;
    let component = load_component(&engine, wasm_path).await?;
    let linker = create_linker(&engine)?;

    #[allow(unused_mut)]
    let mut state = HostState::new();
    #[cfg(feature = "duckdb")]
    if let Some(store) = opts.duck {
        state = state.with_duck(store);
    }
    #[cfg(feature = "lancedb")]
    if let Some(store) = opts.lance {
        state = state.with_lance(store);
    }
    #[cfg(feature = "onnx")]
    if let Some(extractor) = opts.extractor {
        state = state.with_extractor(extractor);
    }

    let mut store = Store::new(&engine, state);
    store.set_fuel(fuel)?;
    // Allow 3600 epoch ticks before interruption (= 1 hour with 1s ticker).
    // Long-running guests (e.g. polisher processing thousands of provisions) need this.
    store.set_epoch_deadline(3600);

    // Spawn a background task to increment the epoch every second.
    let epoch_engine = engine.clone();
    let epoch_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            epoch_engine.increment_epoch();
        }
    });

    let instance = MicroApp::instantiate_async(&mut store, &component, &linker).await?;
    let output = instance.call_run(&mut store).await?;

    epoch_handle.abort();

    let fuel_consumed = fuel.saturating_sub(store.get_fuel()?);
    let state = store.into_data();

    Ok(RunResult {
        output,
        audit_entries: state.audit_entries,
        fuel_consumed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn hello_world_wasm() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../guests/hello-world/target/wasm32-wasip1/release/hello_world.wasm")
    }

    /// Helper: run hello-world guest with no host resources attached.
    async fn run_hello_world(fuel: u64) -> RunResult {
        run_component(&hello_world_wasm(), fuel, RunOptions::default())
            .await
            .expect("run_component failed")
    }

    #[tokio::test]
    async fn run_returns_ok() {
        let result = run_hello_world(1_000_000_000).await;
        assert_eq!(
            result.output,
            Ok("Hello from the first Fractalaw micro-app!".to_string())
        );
    }

    #[tokio::test]
    async fn audit_entry_recorded() {
        let result = run_hello_world(1_000_000_000).await;
        assert_eq!(result.audit_entries.len(), 1);
        let entry = &result.audit_entries[0];
        assert_eq!(entry.event_type, "app-started");
        assert_eq!(entry.resource, "hello-world");
        assert_eq!(entry.detail, "Bootstrap test — first micro-app execution");
    }

    #[tokio::test]
    async fn fuel_consumed() {
        let budget = 1_000_000_000u64;
        let result = run_hello_world(budget).await;
        assert!(result.fuel_consumed > 0, "should have consumed some fuel");
        assert!(
            result.fuel_consumed < budget,
            "should not have exhausted the full budget"
        );
    }

    // ── Data host function unit tests ──

    #[cfg(feature = "duckdb")]
    mod data_tests {
        use super::*;
        use fractalaw_store::DuckStore;

        fn state_with_duck() -> HostState {
            let store = DuckStore::open().unwrap();
            store
                .execute("CREATE TABLE test_data (id INTEGER, name VARCHAR)")
                .unwrap();
            store
                .execute("INSERT INTO test_data VALUES (1, 'alpha'), (2, 'beta'), (3, 'gamma')")
                .unwrap();
            HostState::new().with_duck(store)
        }

        #[tokio::test]
        async fn query_returns_ipc_bytes() {
            use fractal::app::data_query::Host;

            let mut state = state_with_duck();
            let bytes = state
                .query("SELECT id, name FROM test_data ORDER BY id".into())
                .await
                .expect("query failed");

            assert!(!bytes.is_empty(), "IPC bytes should not be empty");

            // Decode and verify
            let batches = decode_ipc(&bytes).unwrap();
            let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(total_rows, 3);
        }

        #[tokio::test]
        async fn query_without_duck_errors() {
            use fractal::app::data_query::Host;

            let mut state = HostState::new(); // no DuckStore
            let err = state.query("SELECT 1".into()).await.unwrap_err();
            assert_eq!(err.code, 1);
            assert!(err.message.contains("no DuckDB store"));
        }

        #[tokio::test]
        async fn query_invalid_sql_errors() {
            use fractal::app::data_query::Host;

            let mut state = state_with_duck();
            let err = state
                .query("SELECT * FROM nonexistent_table".into())
                .await
                .unwrap_err();
            assert_eq!(err.code, 2);
        }

        #[tokio::test]
        async fn execute_runs_ddl() {
            use fractal::app::data_mutate::Host;

            let mut state = state_with_duck();
            state
                .execute("CREATE TABLE new_table (x INTEGER)".into())
                .await
                .expect("execute failed");

            // Verify via query
            use fractal::app::data_query::Host as QHost;
            let bytes = state
                .query("SELECT count(*)::BIGINT AS cnt FROM new_table".into())
                .await
                .expect("query failed");
            assert!(!bytes.is_empty());
        }

        #[tokio::test]
        async fn insert_arrow_ipc_roundtrip() {
            use arrow::array::{Int32Array, StringArray};
            use arrow::datatypes::{DataType, Field, Schema};
            use arrow::ipc::writer::StreamWriter;
            use arrow::record_batch::RecordBatch;
            use fractal::app::data_mutate::Host;
            use std::sync::Arc;

            let mut state = state_with_duck();

            // Build an Arrow IPC payload
            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Int32, true),
                Field::new("name", DataType::Utf8, true),
            ]));
            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(Int32Array::from(vec![10, 20])),
                    Arc::new(StringArray::from(vec!["delta", "epsilon"])),
                ],
            )
            .unwrap();

            let mut buf = Vec::new();
            {
                let mut writer = StreamWriter::try_new(&mut buf, &schema).unwrap();
                writer.write(&batch).unwrap();
                writer.finish().unwrap();
            }

            let rows = state
                .insert("test_data".into(), buf)
                .await
                .expect("insert failed");
            assert_eq!(rows, 2);

            // Verify total count is now 5 (3 original + 2 inserted)
            use fractal::app::data_query::Host as QHost;
            let bytes = state
                .query("SELECT count(*)::BIGINT AS cnt FROM test_data".into())
                .await
                .unwrap();
            let batches = decode_ipc(&bytes).unwrap();
            let col = batches[0]
                .column(0)
                .as_any()
                .downcast_ref::<arrow::array::Int64Array>()
                .unwrap();
            assert_eq!(col.value(0), 5);
        }

        #[tokio::test]
        async fn insert_without_duck_errors() {
            use fractal::app::data_mutate::Host;

            let mut state = HostState::new();
            let err = state.insert("test".into(), vec![]).await.unwrap_err();
            assert_eq!(err.code, 1);
        }

        // ── Integration test: data-test guest with DuckDB ──

        fn data_test_wasm() -> PathBuf {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../guests/data-test/target/wasm32-wasip1/release/data_test.wasm")
        }

        #[tokio::test]
        async fn data_test_guest_end_to_end() {
            let duck = DuckStore::open().unwrap();
            let opts = RunOptions {
                duck: Some(duck),
                #[cfg(feature = "lancedb")]
                lance: None,
                #[cfg(feature = "onnx")]
                extractor: None,
            };
            let result = run_component(&data_test_wasm(), 1_000_000_000, opts)
                .await
                .expect("run_component with data-test guest failed");

            // Guest should return Ok with a summary message
            let output = result.output.expect("guest returned Err");
            assert!(
                output.contains("Data test passed"),
                "unexpected output: {output}"
            );
            assert!(
                output.contains("IPC bytes"),
                "should mention IPC bytes: {output}"
            );

            // Should have 3 audit entries: app-started, ddl-complete, query-complete
            assert_eq!(
                result.audit_entries.len(),
                3,
                "expected 3 audit entries, got: {:?}",
                result
                    .audit_entries
                    .iter()
                    .map(|e| &e.event_type)
                    .collect::<Vec<_>>()
            );
            assert_eq!(result.audit_entries[0].event_type, "app-started");
            assert_eq!(result.audit_entries[1].event_type, "ddl-complete");
            assert_eq!(result.audit_entries[2].event_type, "query-complete");

            assert!(result.fuel_consumed > 0);
        }

        // ── DRRP polisher integration tests ──

        fn drrp_polisher_wasm() -> PathBuf {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../guests/drrp-polisher/target/wasm32-wasip1/release/drrp_polisher.wasm")
        }

        #[tokio::test]
        async fn drrp_polisher_no_data() {
            let duck = DuckStore::open().unwrap();
            // legislation_text with no taxa data → nothing to polish.
            duck.execute(
                "CREATE TABLE legislation_text (
                    section_id VARCHAR NOT NULL,
                    law_name VARCHAR,
                    provision VARCHAR,
                    text VARCHAR,
                    drrp_types VARCHAR[],
                    ai_clause VARCHAR
                )",
            )
            .unwrap();
            duck.execute(
                "INSERT INTO legislation_text VALUES (
                    'UK_ukpga_1974_37:s.2', 'UK_ukpga_1974_37', '2',
                    'Some text here', NULL, NULL
                )",
            )
            .unwrap();
            let opts = RunOptions {
                duck: Some(duck),
                #[cfg(feature = "lancedb")]
                lance: None,
                #[cfg(feature = "onnx")]
                extractor: None,
            };
            let result = run_component(&drrp_polisher_wasm(), 1_000_000_000, opts)
                .await
                .expect("run_component failed");

            let output = result.output.expect("guest returned Err");
            assert!(
                output.contains("No provisions need DRRP polishing"),
                "expected empty batch message, got: {output}"
            );
        }

        #[tokio::test]
        async fn drrp_polisher_reports_inference_errors() {
            let duck = DuckStore::open().unwrap();
            // legislation_text with taxa data but no AI refinement → needs polishing.
            duck.execute(
                "CREATE TABLE legislation_text (
                    section_id VARCHAR NOT NULL,
                    law_name VARCHAR,
                    provision VARCHAR,
                    text VARCHAR,
                    drrp_types VARCHAR[],
                    governed_actors VARCHAR[],
                    government_actors VARCHAR[],
                    duty_family VARCHAR,
                    clause_refined VARCHAR,
                    ai_clause VARCHAR
                )",
            )
            .unwrap();
            duck.execute(
                "INSERT INTO legislation_text VALUES (
                    'UK_ukpga_1974_37:s.2', 'UK_ukpga_1974_37', '2',
                    'It shall be the duty of every employer to ensure, so far as is reasonably practicable, the health, safety and welfare at work of all his employees.',
                    ['Duty'], ['Org: Employer'], ['Gvt: Minister'], 'Governed',
                    'the duty of every employer to ensure', NULL
                )",
            )
            .unwrap();

            let opts = RunOptions {
                duck: Some(duck),
                #[cfg(feature = "lancedb")]
                lance: None,
                #[cfg(feature = "onnx")]
                extractor: None, // no ONNX model → inference calls will error
            };
            let result = run_component(&drrp_polisher_wasm(), 1_000_000_000, opts)
                .await
                .expect("run_component failed");

            // Guest should succeed overall but report 1 error (inference not configured).
            let output = result.output.expect("guest returned Err");
            assert!(
                output.contains("1 errors"),
                "expected 1 inference error, got: {output}"
            );
            assert!(
                output.contains("Polished 0 provisions"),
                "expected 0 polished, got: {output}"
            );
        }
    }

    // ── AI host function unit tests ──

    mod ai_tests {
        use super::*;

        #[tokio::test]
        async fn embed_returns_not_configured() {
            use fractal::app::ai_embeddings::Host;

            let mut state = HostState::new();
            let err = state.embed("test".into()).await.unwrap_err();
            assert_eq!(err.code, 1);
            assert!(err.message.contains("not configured"));
        }

        #[tokio::test]
        async fn embed_batch_returns_not_configured() {
            use fractal::app::ai_embeddings::Host;

            let mut state = HostState::new();
            let err = state
                .embed_batch(vec!["a".into(), "b".into()])
                .await
                .unwrap_err();
            assert_eq!(err.code, 1);
        }

        #[tokio::test]
        async fn generate_without_config_errors() {
            use fractal::app::ai_inference::Host;

            let mut state = HostState::new();
            let request = fractal::app::ai_inference::GenerateRequest {
                system_prompt: None,
                user_prompt: "Hello".into(),
                max_tokens: 100,
                temperature: 0.0,
            };
            let err = state.generate(request).await.unwrap_err();
            assert_eq!(err.code, 1);
            assert!(
                err.message.contains("ONNX model not loaded"),
                "unexpected error message: {}",
                err.message,
            );
        }
    }
}
