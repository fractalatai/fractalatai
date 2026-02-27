//! Zenoh pub/sub sync for distributing taxa enrichment data.
//!
//! Publishes law-level taxa data (aggregated from DuckDB) as Arrow IPC
//! over zenoh key expressions. Subscribes to taxa updates for receiving
//! enrichment from other nodes in the mesh.

use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
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
}

/// Test helpers shared across sync crate tests.
#[cfg(test)]
pub(crate) mod test_helpers {
    use arrow::array::{ListBuilder, StringArray, StringBuilder};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

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
    use std::sync::Arc;

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
}
