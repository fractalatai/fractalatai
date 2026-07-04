//! Zenoh pub/sub sync for distributing taxa enrichment data.
//!
//! Publishes law-level taxa data (aggregated from DuckDB) as Arrow IPC
//! over zenoh key expressions. Subscribes to taxa updates for receiving
//! enrichment from other nodes in the mesh.

use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
use serde::Deserialize;
use std::io::Cursor;
use thiserror::Error;
use tracing::info;

// ── Errors ──

#[derive(Error, Debug)]
pub enum ZenohError {
    #[error("zenoh session error: {0}")]
    Session(zenoh::Error),
    #[error("Arrow IPC encoding error: {0}")]
    ArrowEncode(#[from] arrow::error::ArrowError),
    #[error("Arrow IPC decoding error: {0}")]
    ArrowDecode(arrow::error::ArrowError),
    #[error("no data to publish for '{law_name}'")]
    NoData { law_name: String },
    #[error("JSON decode error: {0}")]
    Json(#[from] serde_json::Error),
}

// ── Sync events ──

/// A data-change event published by sertantai on the events/sync key.
///
/// Payload example:
/// ```json
/// {
///   "table": "lat",
///   "action": "persist",
///   "metadata": { "law_name": "UK_ukpga_1974_37", "count": 350 },
///   "timestamp": "2026-02-27T15:30:00Z"
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct SyncEvent {
    pub table: String,
    pub action: String,
    pub metadata: SyncEventMetadata,
    pub timestamp: String,
}

/// Metadata within a [`SyncEvent`].
#[derive(Debug, Clone, Deserialize)]
pub struct SyncEventMetadata {
    pub law_name: String,
    #[serde(default)]
    pub count: Option<u64>,
    /// Number of LAT rows deleted (present only on `lat_deleted` events).
    #[serde(default)]
    pub lat_deleted: Option<u64>,
    /// Number of amendment annotation rows deleted (present only on `lat_deleted` events).
    #[serde(default)]
    pub annotations_deleted: Option<u64>,
}

impl SyncEvent {
    /// Deserialize a SyncEvent from a zenoh sample payload.
    pub fn from_payload(bytes: &[u8]) -> Result<Self, ZenohError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}

// ── Key expressions ──

/// Zenoh key expression builders for the fractalaw namespace.
///
/// Key expressions use hermetic `@` tenant prefixes:
/// `fractalaw/@{tenant}/taxa/enrichment/{law_name}`
pub mod keys {
    /// Root prefix for all fractalaw key expressions.
    pub const PREFIX: &str = "fractalaw";

    /// Key expression for a specific law's taxa enrichment data.
    ///
    /// Example: `fractalaw/@acme/taxa/enrichment/UK_ukpga_1974_37`
    pub fn taxa_enrichment(tenant: &str, law_name: &str) -> String {
        format!("{PREFIX}/@{tenant}/taxa/enrichment/{law_name}")
    }

    /// Wildcard key expression for all taxa enrichment under a tenant.
    ///
    /// Example: `fractalaw/@acme/taxa/enrichment/*`
    pub fn taxa_enrichment_wildcard(tenant: &str) -> String {
        format!("{PREFIX}/@{tenant}/taxa/enrichment/*")
    }

    /// Extract the law name from a taxa enrichment key expression.
    ///
    /// Given `fractalaw/@acme/taxa/enrichment/UK_ukpga_1974_37`,
    /// returns `Some("UK_ukpga_1974_37")`.
    pub fn law_name_from_key(key_expr: &str) -> Option<&str> {
        key_expr.rsplit('/').next()
    }

    /// Key expression for a specific law's legislation text (LAT) data
    /// served by sertantai.
    ///
    /// Example: `fractalaw/@acme/data/legislation/lat/UK_uksi_2004_1309`
    pub fn lat(tenant: &str, law_name: &str) -> String {
        format!("{PREFIX}/@{tenant}/data/legislation/lat/{law_name}")
    }

    /// Wildcard key expression for all LAT data under a tenant.
    ///
    /// Example: `fractalaw/@acme/data/legislation/lat/*`
    pub fn lat_wildcard(tenant: &str) -> String {
        format!("{PREFIX}/@{tenant}/data/legislation/lat/*")
    }

    /// Key expression for a specific law's legislation record (LRT) data
    /// served by sertantai.
    ///
    /// Example: `fractalaw/@acme/data/legislation/lrt/UK_ukpga_1974_37`
    pub fn lrt(tenant: &str, law_name: &str) -> String {
        format!("{PREFIX}/@{tenant}/data/legislation/lrt/{law_name}")
    }

    /// Key expression for a specific law's provision-level taxa data.
    ///
    /// Example: `fractalaw/@acme/taxa/provisions/UK_ukpga_1974_37`
    pub fn taxa_provisions(tenant: &str, law_name: &str) -> String {
        format!("{PREFIX}/@{tenant}/taxa/provisions/{law_name}")
    }

    /// Wildcard key expression for all provision-level taxa under a tenant.
    ///
    /// Example: `fractalaw/@acme/taxa/provisions/*`
    pub fn taxa_provisions_wildcard(tenant: &str) -> String {
        format!("{PREFIX}/@{tenant}/taxa/provisions/*")
    }

    /// Key expression for sertantai sync events (data-change notifications).
    ///
    /// Example: `fractalaw/@acme/events/sync`
    pub fn events_sync(tenant: &str) -> String {
        format!("{PREFIX}/@{tenant}/events/sync")
    }

    /// Key expression for the actor dictionary queryable.
    ///
    /// Example: `fractalaw/@acme/dictionary/actors`
    pub fn dictionary_actors(tenant: &str) -> String {
        format!("{PREFIX}/@{tenant}/dictionary/actors")
    }

    /// Key expression for ingestion acknowledgement.
    ///
    /// Example: `fractalaw/@acme/ack/UK_ukpga_1974_37`
    pub fn ack(tenant: &str, law_name: &str) -> String {
        format!("{PREFIX}/@{tenant}/ack/{law_name}")
    }

    /// Key expression for pipeline status queryable.
    ///
    /// Example: `fractalaw/@acme/status`
    pub fn status(tenant: &str) -> String {
        format!("{PREFIX}/@{tenant}/status")
    }

    /// Key expression for per-law status events.
    ///
    /// Example: `fractalaw/@acme/status/UK_ukpga_1974_37`
    pub fn status_event(tenant: &str, law_name: &str) -> String {
        format!("{PREFIX}/@{tenant}/status/{law_name}")
    }

    /// Key expression for triage queryable.
    ///
    /// Fractalaw serves this — sertantai queries to get making/not-making classification.
    /// Example: `fractalaw/@acme/triage`
    pub fn triage(tenant: &str) -> String {
        format!("{PREFIX}/@{tenant}/triage")
    }

    /// Key expression for sertantai customer laws queryable.
    ///
    /// Sertantai serves this — fractalaw queries it to get law names for a customer.
    /// Example: `fractalaw/@dev/sertantai/customers/c075d56b-8420-4408-b695-ccfbc1ba15ec/laws`
    pub fn customer_laws(tenant: &str, customer_id: &str) -> String {
        format!("{PREFIX}/@{tenant}/sertantai/customers/{customer_id}/laws")
    }
}

// ── Arrow IPC helpers ──

/// Encode Arrow RecordBatches into IPC streaming format bytes.
pub fn encode_arrow_ipc(batches: &[RecordBatch]) -> Result<Vec<u8>, ZenohError> {
    if batches.is_empty() {
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
pub fn decode_arrow_ipc(data: &[u8]) -> Result<Vec<RecordBatch>, ZenohError> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    let reader = StreamReader::try_new(Cursor::new(data), None).map_err(ZenohError::ArrowDecode)?;
    reader
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(ZenohError::ArrowDecode)
}

// ── ZenohSync ──

/// Zenoh sync engine for publishing and subscribing to taxa enrichment data.
///
/// Holds a zenoh session and tenant namespace. All operations use
/// the hermetic key expression scheme: `fractalaw/@{tenant}/...`
pub struct ZenohSync {
    session: zenoh::Session,
    tenant: String,
}

impl ZenohSync {
    /// Open a new zenoh session with default peer-mode config.
    ///
    /// `tenant` is the namespace prefix (e.g., `"acme"`). The `@` prefix
    /// is added automatically in key expressions.
    pub async fn new(tenant: &str) -> Result<Self, ZenohError> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(ZenohError::Session)?;
        info!(tenant = %tenant, "opened zenoh session");
        Ok(Self {
            session,
            tenant: tenant.to_string(),
        })
    }

    /// Open a zenoh session with a custom config.
    ///
    /// Use for testing (in-process peers) or production (explicit endpoints).
    pub async fn with_config(tenant: &str, config: zenoh::Config) -> Result<Self, ZenohError> {
        let session = zenoh::open(config).await.map_err(ZenohError::Session)?;
        info!(tenant = %tenant, "opened zenoh session with custom config");
        Ok(Self {
            session,
            tenant: tenant.to_string(),
        })
    }

    /// Get the tenant namespace.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    /// Wait until at least one Zenoh peer connects, or timeout expires.
    ///
    /// Returns the number of connected peers, or 0 if timeout elapsed.
    /// Useful for short-lived sessions (e.g., `sync publish`) where the
    /// remote peer may not have connected yet via scouting or configured endpoint.
    pub async fn wait_for_peers(&self, timeout: std::time::Duration) -> usize {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let count = self.session.info().peers_zid().await.count();
            if count > 0 {
                return count;
            }
            if tokio::time::Instant::now() >= deadline {
                return 0;
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    }

    /// Borrow the underlying zenoh session.
    pub fn session(&self) -> &zenoh::Session {
        &self.session
    }

    /// Publish taxa enrichment data for a specific law.
    ///
    /// `batches` should contain the taxa columns from DuckDB's `legislation`
    /// table (duty_holder, rights_holder, etc.) as Arrow RecordBatches.
    /// The payload is serialized as Arrow IPC streaming format.
    pub async fn publish_taxa(
        &self,
        law_name: &str,
        batches: &[RecordBatch],
    ) -> Result<(), ZenohError> {
        if batches.is_empty() || batches.iter().all(|b| b.num_rows() == 0) {
            return Err(ZenohError::NoData {
                law_name: law_name.to_string(),
            });
        }

        let ipc_bytes = encode_arrow_ipc(batches)?;
        let key = keys::taxa_enrichment(&self.tenant, law_name);

        info!(
            key = %key,
            bytes = ipc_bytes.len(),
            rows = batches.iter().map(|b| b.num_rows()).sum::<usize>(),
            "publishing taxa enrichment"
        );

        self.session
            .put(&key, ipc_bytes)
            .await
            .map_err(ZenohError::Session)?;

        Ok(())
    }

    /// Publish provision-level taxa/fitness data for a specific law.
    ///
    /// `batches` should contain per-provision DRRP and fitness columns
    /// from LanceDB's `legislation_text` table (section_id, drrp_types,
    /// governed_actors, fitness_*, etc.) as Arrow RecordBatches.
    /// The payload is serialized as Arrow IPC streaming format.
    pub async fn publish_provision_taxa(
        &self,
        law_name: &str,
        batches: &[RecordBatch],
    ) -> Result<(), ZenohError> {
        if batches.is_empty() || batches.iter().all(|b| b.num_rows() == 0) {
            return Err(ZenohError::NoData {
                law_name: law_name.to_string(),
            });
        }

        let ipc_bytes = encode_arrow_ipc(batches)?;
        let key = keys::taxa_provisions(&self.tenant, law_name);

        info!(
            key = %key,
            bytes = ipc_bytes.len(),
            rows = batches.iter().map(|b| b.num_rows()).sum::<usize>(),
            "publishing provision-level taxa"
        );

        self.session
            .put(&key, ipc_bytes)
            .await
            .map_err(ZenohError::Session)?;

        Ok(())
    }

    /// Publish the actor dictionary YAML to zenoh.
    ///
    /// Puts the raw YAML content at `fractalaw/@{tenant}/dictionary/actors`.
    /// Sertantai can subscribe or get this to stay in sync with canonical labels.
    pub async fn publish_dictionary(&self, yaml_content: &[u8]) -> Result<(), ZenohError> {
        let key = keys::dictionary_actors(&self.tenant);
        info!(key = %key, bytes = yaml_content.len(), "publishing actor dictionary");
        self.session
            .put(&key, yaml_content.to_vec())
            .await
            .map_err(ZenohError::Session)?;
        Ok(())
    }

    /// Publish an ingestion acknowledgement for a law.
    ///
    /// Puts a JSON payload at `fractalaw/@{tenant}/ack/{law_name}`.
    /// Sertantai can subscribe to show "Enrichment Pending" in the UI.
    pub async fn publish_ack(&self, law_name: &str, provisions: usize) -> Result<(), ZenohError> {
        let key = keys::ack(&self.tenant, law_name);
        let payload = serde_json::json!({
            "law_name": law_name,
            "state": "ingested",
            "provisions": provisions,
        });
        info!(key = %key, provisions, "ack ingestion");
        self.session
            .put(&key, payload.to_string().into_bytes())
            .await
            .map_err(ZenohError::Session)?;
        Ok(())
    }

    /// Declare a queryable that serves the actor dictionary YAML on demand.
    ///
    /// Returns a handle that keeps the queryable alive. Drop it to stop serving.
    /// Sertantai queries `fractalaw/@{tenant}/dictionary/actors` and receives
    /// the raw YAML bytes in the reply.
    pub async fn serve_dictionary(
        &self,
        yaml_content: Vec<u8>,
    ) -> Result<tokio::task::JoinHandle<()>, ZenohError> {
        let key = keys::dictionary_actors(&self.tenant);
        let queryable = self
            .session
            .declare_queryable(&key)
            .await
            .map_err(ZenohError::Session)?;
        info!(key = %key, "serving actor dictionary via queryable");

        let handle = tokio::spawn(async move {
            while let Ok(query) = queryable.recv_async().await {
                let reply_key = query.key_expr().as_str().to_string();
                if let Err(e) = query.reply(&reply_key, yaml_content.clone()).await {
                    tracing::warn!(error = %e, "failed to reply to dictionary query");
                }
            }
        });

        Ok(handle)
    }

    /// Publish a pipeline status event for a law.
    ///
    /// Published to `fractalaw/@{tenant}/status/{law_name}` as JSON.
    pub async fn publish_status_event(
        &self,
        law_name: &str,
        stage: &str,
    ) -> Result<(), ZenohError> {
        let key = keys::status_event(&self.tenant, law_name);
        let payload = serde_json::json!({
            "law_name": law_name,
            "stage": stage,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        self.session
            .put(&key, payload.to_string().into_bytes())
            .await
            .map_err(ZenohError::Session)?;
        Ok(())
    }

    /// Query sertantai for a customer's law names.
    ///
    /// Sends a zenoh `get()` to the sertantai queryable at
    /// `fractalaw/@{tenant}/sertantai/customers/{customer_id}/laws`.
    /// Returns a list of law names (e.g. `["UK_ukpga_1974_37", ...]`).
    ///
    /// Returns an empty Vec if no peer responds within the timeout.
    pub async fn query_customer_laws(
        &self,
        customer_id: &str,
        timeout: std::time::Duration,
    ) -> Result<Vec<String>, ZenohError> {
        let key = keys::customer_laws(&self.tenant, customer_id);
        let replies = self
            .session
            .get(&key)
            .timeout(timeout)
            .await
            .map_err(ZenohError::Session)?;

        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.into_result() {
                let bytes = sample.payload().to_bytes();
                if let Ok(names) = serde_json::from_slice::<Vec<String>>(&bytes) {
                    return Ok(names);
                }
                // Try as newline-separated text
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    let names: Vec<String> = text
                        .lines()
                        .filter(|l| !l.is_empty() && l.contains('_'))
                        .map(|l| l.trim().to_string())
                        .collect();
                    if !names.is_empty() {
                        return Ok(names);
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    /// Query sertantai for legislation text (LAT) for a specific law.
    ///
    /// Sends a zenoh `get()` query to the sertantai queryable at
    /// `fractalaw/@{tenant}/data/legislation/lat/{law_name}`.
    /// Returns decoded Arrow RecordBatches containing all provisions.
    ///
    /// Returns an empty Vec if no peer responds within the timeout.
    pub async fn query_lat(
        &self,
        law_name: &str,
        timeout: std::time::Duration,
    ) -> Result<Vec<RecordBatch>, ZenohError> {
        let key = keys::lat(&self.tenant, law_name);
        info!(key = %key, "querying LAT from sertantai");

        let replies = self
            .session
            .get(&key)
            .timeout(timeout)
            .await
            .map_err(ZenohError::Session)?;

        let mut all_batches = Vec::new();
        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                let bytes = sample.payload().to_bytes();
                if bytes.is_empty() {
                    continue;
                }
                info!(
                    law_name = %law_name,
                    bytes_len = bytes.len(),
                    first_bytes = ?&bytes[..bytes.len().min(64)],
                    "raw LAT reply payload"
                );
                let batches = decode_arrow_ipc(&bytes)?;
                all_batches.extend(batches);
            }
        }

        info!(
            law_name = %law_name,
            batches = all_batches.len(),
            rows = all_batches.iter().map(|b| b.num_rows()).sum::<usize>(),
            "received LAT data"
        );

        Ok(all_batches)
    }

    /// Query sertantai for a single law's legislation record (LRT) via zenoh.
    ///
    /// Response is Arrow IPC streaming format, same as LAT.
    /// Returns an empty Vec if no peer responds within the timeout.
    pub async fn query_lrt(
        &self,
        law_name: &str,
        timeout: std::time::Duration,
    ) -> Result<Vec<RecordBatch>, ZenohError> {
        let key = keys::lrt(&self.tenant, law_name);
        info!(key = %key, "querying LRT from sertantai");

        let replies = self
            .session
            .get(&key)
            .timeout(timeout)
            .await
            .map_err(ZenohError::Session)?;

        let mut all_batches = Vec::new();
        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                let bytes = sample.payload().to_bytes();
                if bytes.is_empty() {
                    continue;
                }
                let batches = decode_arrow_ipc(&bytes)?;
                all_batches.extend(batches);
            }
        }

        info!(
            law_name = %law_name,
            batches = all_batches.len(),
            rows = all_batches.iter().map(|b| b.num_rows()).sum::<usize>(),
            "received LRT data"
        );

        Ok(all_batches)
    }

    /// Subscribe to taxa enrichment updates for the tenant.
    ///
    /// Returns a Subscriber. Receive samples via `subscriber.recv_async().await`.
    /// Each sample's payload is Arrow IPC bytes, decodable with [`decode_arrow_ipc`].
    pub async fn subscribe_taxa(
        &self,
    ) -> Result<
        zenoh::pubsub::Subscriber<zenoh::handlers::FifoChannelHandler<zenoh::sample::Sample>>,
        ZenohError,
    > {
        let key = keys::taxa_enrichment_wildcard(&self.tenant);
        info!(key = %key, "subscribing to taxa enrichment");
        self.session
            .declare_subscriber(&key)
            .await
            .map_err(ZenohError::Session)
    }

    /// Subscribe to sync events (data-change notifications) from sertantai.
    ///
    /// Returns a Subscriber. Receive samples via `subscriber.recv_async().await`.
    /// Each sample's payload is a JSON [`SyncEvent`], decodable with
    /// [`SyncEvent::from_payload`].
    pub async fn subscribe_events(
        &self,
    ) -> Result<
        zenoh::pubsub::Subscriber<zenoh::handlers::FifoChannelHandler<zenoh::sample::Sample>>,
        ZenohError,
    > {
        let key = keys::events_sync(&self.tenant);
        info!(key = %key, "subscribing to sync events");
        self.session
            .declare_subscriber(&key)
            .await
            .map_err(ZenohError::Session)
    }
}

/// Test helpers shared across sync crate tests.
#[cfg(test)]
pub(crate) mod test_helpers {
    use arrow::array::{Int32Array, ListBuilder, StringArray, StringBuilder};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    /// Minimal LAT batch for testing: section_id, law_name, text, sort_key, position.
    pub fn test_lat_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("law_name", DataType::Utf8, false),
            Field::new("section_id", DataType::Utf8, false),
            Field::new("sort_key", DataType::Utf8, false),
            Field::new("position", DataType::Int32, false),
            Field::new("text", DataType::Utf8, false),
        ]));

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["UK_uksi_2004_1309"])),
                Arc::new(StringArray::from(vec!["UK_uksi_2004_1309:s.1"])),
                Arc::new(StringArray::from(vec!["001.000"])),
                Arc::new(Int32Array::from(vec![1])),
                Arc::new(StringArray::from(vec!["Citation and commencement"])),
            ],
        )
        .unwrap()
    }

    /// Minimal taxa schema for testing: name + 4 List<Utf8> holder columns.
    pub fn taxa_test_schema() -> Schema {
        let list_utf8 = DataType::List(Arc::new(Field::new_list_field(DataType::Utf8, true)));
        Schema::new(vec![
            Field::new("name", DataType::Utf8, false),
            Field::new("duty_holder", list_utf8.clone(), true),
            Field::new("rights_holder", list_utf8.clone(), true),
            Field::new("responsibility_holder", list_utf8.clone(), true),
            Field::new("power_holder", list_utf8, true),
        ])
    }

    /// Build a test RecordBatch with one row of taxa data.
    pub fn test_taxa_batch() -> RecordBatch {
        let schema = Arc::new(taxa_test_schema());

        let name = StringArray::from(vec!["UK_ukpga_1974_37"]);

        let mut duty_b = ListBuilder::new(StringBuilder::new());
        duty_b.values().append_value("employer");
        duty_b.values().append_value("self-employed person");
        duty_b.append(true);

        let mut rights_b = ListBuilder::new(StringBuilder::new());
        rights_b.values().append_value("employee");
        rights_b.append(true);

        let mut resp_b = ListBuilder::new(StringBuilder::new());
        resp_b.values().append_value("Secretary of State");
        resp_b.append(true);

        let mut power_b = ListBuilder::new(StringBuilder::new());
        power_b.append(true); // empty list

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(name),
                Arc::new(duty_b.finish()),
                Arc::new(rights_b.finish()),
                Arc::new(resp_b.finish()),
                Arc::new(power_b.finish()),
            ],
        )
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::test_helpers::*;
    use super::*;
    use arrow::array::Array;

    // ── Arrow IPC tests ──

    #[test]
    fn arrow_ipc_roundtrip() {
        let batch = test_taxa_batch();
        let encoded = encode_arrow_ipc(&[batch.clone()]).unwrap();
        assert!(!encoded.is_empty());
        let decoded = decode_arrow_ipc(&encoded).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].num_rows(), batch.num_rows());
        assert_eq!(decoded[0].schema(), batch.schema());
    }

    #[test]
    fn arrow_ipc_empty() {
        let encoded = encode_arrow_ipc(&[]).unwrap();
        assert!(encoded.is_empty());
        let decoded = decode_arrow_ipc(&encoded).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn arrow_ipc_preserves_list_values() {
        let batch = test_taxa_batch();
        let encoded = encode_arrow_ipc(&[batch]).unwrap();
        let decoded = decode_arrow_ipc(&encoded).unwrap();

        // Check duty_holder column has 2 values in the list.
        let duty_col = decoded[0]
            .column_by_name("duty_holder")
            .unwrap()
            .as_any()
            .downcast_ref::<arrow::array::ListArray>()
            .unwrap();
        let values = duty_col.value(0);
        let str_arr = values
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        assert_eq!(str_arr.len(), 2);
        assert_eq!(str_arr.value(0), "employer");
        assert_eq!(str_arr.value(1), "self-employed person");
    }

    // ── Key expression tests ──

    #[test]
    fn key_taxa_enrichment() {
        assert_eq!(
            keys::taxa_enrichment("acme", "UK_ukpga_1974_37"),
            "fractalaw/@acme/taxa/enrichment/UK_ukpga_1974_37"
        );
    }

    #[test]
    fn key_taxa_enrichment_wildcard() {
        assert_eq!(
            keys::taxa_enrichment_wildcard("acme"),
            "fractalaw/@acme/taxa/enrichment/*"
        );
    }

    #[test]
    fn law_name_from_key_extracts() {
        assert_eq!(
            keys::law_name_from_key("fractalaw/@acme/taxa/enrichment/UK_ukpga_1974_37"),
            Some("UK_ukpga_1974_37")
        );
    }

    // ── Pub/sub integration tests ──

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn publish_subscribe_roundtrip() {
        let publisher = ZenohSync::new("test-pub-sub").await.unwrap();
        let subscriber = ZenohSync::new("test-pub-sub").await.unwrap();

        let sub = subscriber.subscribe_taxa().await.unwrap();

        // Small delay to let subscription propagate.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let batch = test_taxa_batch();
        publisher
            .publish_taxa("UK_ukpga_1974_37", &[batch.clone()])
            .await
            .unwrap();

        let sample = tokio::time::timeout(std::time::Duration::from_secs(5), sub.recv_async())
            .await
            .expect("timeout waiting for sample")
            .expect("recv error");

        assert!(sample.key_expr().as_str().contains("UK_ukpga_1974_37"));

        let payload = sample.payload().to_bytes();
        let decoded = decode_arrow_ipc(&payload).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].num_rows(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn publish_no_data_errors() {
        let sync = ZenohSync::new("test-no-data").await.unwrap();
        let result = sync.publish_taxa("some_law", &[]).await;
        assert!(matches!(result, Err(ZenohError::NoData { .. })));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn publish_provision_taxa_roundtrip() {
        let publisher = ZenohSync::new("test-prov-taxa").await.unwrap();
        let subscriber = ZenohSync::new("test-prov-taxa").await.unwrap();

        let key = keys::taxa_provisions_wildcard("test-prov-taxa");
        let sub = subscriber.session().declare_subscriber(&key).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let batch = test_taxa_batch();
        publisher
            .publish_provision_taxa("UK_ukpga_1974_37", &[batch.clone()])
            .await
            .unwrap();

        let sample = tokio::time::timeout(std::time::Duration::from_secs(5), sub.recv_async())
            .await
            .expect("timeout waiting for sample")
            .expect("recv error");

        assert!(
            sample
                .key_expr()
                .as_str()
                .contains("taxa/provisions/UK_ukpga_1974_37")
        );

        let payload = sample.payload().to_bytes();
        let decoded = decode_arrow_ipc(&payload).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].num_rows(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn publish_provision_taxa_no_data_errors() {
        let sync = ZenohSync::new("test-prov-no-data").await.unwrap();
        let result = sync.publish_provision_taxa("some_law", &[]).await;
        assert!(matches!(result, Err(ZenohError::NoData { .. })));
    }

    // ── LAT key expression tests ──

    #[test]
    fn key_lat() {
        assert_eq!(
            keys::lat("acme", "UK_uksi_2004_1309"),
            "fractalaw/@acme/data/legislation/lat/UK_uksi_2004_1309"
        );
    }

    #[test]
    fn key_lat_wildcard() {
        assert_eq!(
            keys::lat_wildcard("acme"),
            "fractalaw/@acme/data/legislation/lat/*"
        );
    }

    // ── LAT query integration test ──

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn query_lat_roundtrip() {
        let sync = ZenohSync::new("test-lat-q").await.unwrap();

        // Simulate sertantai: declare a queryable that responds with Arrow IPC.
        let batch = test_lat_batch();
        let ipc_bytes = encode_arrow_ipc(&[batch.clone()]).unwrap();
        let key = keys::lat("test-lat-q", "*");

        let queryable = sync.session().declare_queryable(&key).await.unwrap();

        let ipc_clone = ipc_bytes.clone();
        let responder = tokio::spawn(async move {
            if let Ok(query) = queryable.recv_async().await {
                let reply_key = query.key_expr().as_str().to_string();
                query.reply(&reply_key, ipc_clone).await.unwrap();
            }
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let result = sync
            .query_lat("UK_uksi_2004_1309", std::time::Duration::from_secs(5))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].num_rows(), batch.num_rows());
        assert_eq!(result[0].schema(), batch.schema());

        responder.await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn query_lat_no_responder_returns_empty() {
        let sync = ZenohSync::new("test-lat-empty").await.unwrap();
        let result = sync
            .query_lat("nonexistent_law", std::time::Duration::from_secs(1))
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    // ── LRT key expression tests ──

    #[test]
    fn key_lrt() {
        assert_eq!(
            keys::lrt("acme", "UK_ukpga_1974_37"),
            "fractalaw/@acme/data/legislation/lrt/UK_ukpga_1974_37"
        );
    }

    #[test]
    fn key_dictionary_actors() {
        assert_eq!(
            keys::dictionary_actors("dev"),
            "fractalaw/@dev/dictionary/actors"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn query_lrt_no_responder_returns_empty() {
        let sync = ZenohSync::new("test-lrt-empty").await.unwrap();
        let result = sync
            .query_lrt("nonexistent_law", std::time::Duration::from_secs(1))
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    // ── Provision taxa key expression tests ──

    #[test]
    fn key_taxa_provisions() {
        assert_eq!(
            keys::taxa_provisions("acme", "UK_ukpga_1974_37"),
            "fractalaw/@acme/taxa/provisions/UK_ukpga_1974_37"
        );
    }

    #[test]
    fn key_taxa_provisions_wildcard() {
        assert_eq!(
            keys::taxa_provisions_wildcard("acme"),
            "fractalaw/@acme/taxa/provisions/*"
        );
    }

    // ── Events key expression tests ──

    #[test]
    fn key_events_sync() {
        assert_eq!(keys::events_sync("dev"), "fractalaw/@dev/events/sync");
    }

    // ── SyncEvent deserialization tests ──

    #[test]
    fn sync_event_deserialize() {
        let json = r#"{
            "table": "lat",
            "action": "persist",
            "metadata": { "law_name": "UK_ukpga_1974_37", "count": 350 },
            "timestamp": "2026-02-27T15:30:00Z"
        }"#;
        let event = SyncEvent::from_payload(json.as_bytes()).unwrap();
        assert_eq!(event.table, "lat");
        assert_eq!(event.action, "persist");
        assert_eq!(event.metadata.law_name, "UK_ukpga_1974_37");
        assert_eq!(event.metadata.count, Some(350));
        assert_eq!(event.timestamp, "2026-02-27T15:30:00Z");
    }

    #[test]
    fn sync_event_deserialize_without_count() {
        let json = r#"{
            "table": "lrt",
            "action": "persist",
            "metadata": { "law_name": "UK_uksi_2004_1309" },
            "timestamp": "2026-02-27T16:00:00Z"
        }"#;
        let event = SyncEvent::from_payload(json.as_bytes()).unwrap();
        assert_eq!(event.table, "lrt");
        assert_eq!(event.metadata.count, None);
    }

    // ── Events subscription integration test ──

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn subscribe_events_roundtrip() {
        let publisher = ZenohSync::new("test-events").await.unwrap();
        let subscriber = ZenohSync::new("test-events").await.unwrap();

        let sub = subscriber.subscribe_events().await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let payload = r#"{"table":"lat","action":"persist","metadata":{"law_name":"UK_ukpga_1974_37","count":350},"timestamp":"2026-02-27T15:30:00Z"}"#;
        let key = keys::events_sync("test-events");
        publisher.session().put(&key, payload).await.unwrap();

        let sample = tokio::time::timeout(std::time::Duration::from_secs(5), sub.recv_async())
            .await
            .expect("timeout waiting for event")
            .expect("recv error");

        let bytes = sample.payload().to_bytes();
        let event = SyncEvent::from_payload(&bytes).unwrap();
        assert_eq!(event.table, "lat");
        assert_eq!(event.metadata.law_name, "UK_ukpga_1974_37");
    }
}
