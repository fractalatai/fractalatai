//! Trait abstraction for provision stores (LanceDB, PostgreSQL+pgvector).
//!
//! Hub uses PgStore (write-heavy, no fragment bloat).
//! Edge uses LanceStore (embedded, read-only synced slices).

use arrow::record_batch::RecordBatch;
use async_trait::async_trait;

use crate::StoreError;

/// Provision data store — implemented by LanceStore and PgStore.
#[async_trait]
pub trait ProvisionStore: Send + Sync {
    /// Query all provisions for a law, with limit/offset.
    async fn query_legislation_text(
        &self,
        law_name: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RecordBatch>, StoreError>;

    /// Query provision taxa for zenoh publish.
    async fn query_provision_taxa(
        &self,
        law_name: &str,
    ) -> Result<Vec<RecordBatch>, StoreError>;

    /// Upsert LAT provisions from sertantai.
    async fn upsert_lat(&self, batches: Vec<RecordBatch>) -> Result<usize, StoreError>;

    /// Upsert embeddings (or any column subset keyed on section_id).
    async fn upsert_embeddings(&self, batch: &RecordBatch) -> Result<(), StoreError>;

    /// Upsert taxa classification results.
    async fn update_taxa(&self, batch: RecordBatch) -> Result<(), StoreError>;

    /// Upsert AI-polished results.
    async fn update_polished(&self, batch: RecordBatch) -> Result<(), StoreError>;

    /// Compact / optimise storage (no-op for Postgres).
    async fn compact(&self) -> Result<(), StoreError>;

    /// Ensure required columns exist.
    async fn ensure_gap_c_columns(&self) -> Result<(), StoreError>;

    /// Delete provisions for a law.
    async fn delete_law_lat(&self, law_name: &str) -> Result<usize, StoreError>;

    /// Count total provisions.
    async fn legislation_text_count(&self) -> Result<usize, StoreError>;

    /// Delete annotations for a law (LanceDB-only, no-op for Postgres).
    async fn delete_law_annotations(&self, _law_name: &str) -> Result<usize, StoreError> {
        Ok(0)
    }

    /// Write classifier actor predictions to cls_actors for specific provisions.
    async fn write_cls_actors(&self, _updates: &[(String, String)]) -> Result<(), StoreError> {
        Ok(()) // default no-op
    }

    /// Copy current drrp_types/actors to regex_drrp/regex_actors for a law.
    /// Called after taxa parse to preserve the regex tier signal.
    async fn snapshot_regex_signals(&self, _law_name: &str) -> Result<(), StoreError> {
        Ok(()) // default no-op, PgStore overrides
    }

    /// Copy current drrp_types/actors to cls_drrp/cls_actors for a law.
    /// Called after taxa classify to preserve the classifier tier signal.
    async fn snapshot_classifier_signals(&self, _law_name: &str) -> Result<(), StoreError> {
        Ok(())
    }

    /// Copy current drrp_types/actors to llm_drrp/llm_actors for a law.
    /// Called after taxa validate to preserve the LLM tier signal.
    async fn snapshot_llm_signals(&self, _law_name: &str) -> Result<(), StoreError> {
        Ok(())
    }
}
