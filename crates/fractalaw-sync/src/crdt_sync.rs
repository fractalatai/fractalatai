//! Loro CRDT sync engine over Zenoh.
//!
//! Manages a collection of named Loro documents, auto-publishes incremental
//! updates over zenoh pub/sub, subscribes to remote updates, serves snapshots
//! for late-joiners, and persists document state to disk as `.loro` files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use loro::{ExportMode, LoroDoc, LoroValue, Subscription, ValueOrContainer, VersionVector};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::zenoh_sync::keys::PREFIX;

// ── Errors ──

#[derive(Error, Debug)]
pub enum CrdtError {
    #[error("zenoh session error: {0}")]
    Session(zenoh::Error),
    #[error("Loro export error: {0}")]
    Export(#[from] loro::LoroEncodeError),
    #[error("Loro import error: {0}")]
    Import(#[from] loro::LoroError),
    #[error("document '{doc_id}' not found")]
    DocNotFound { doc_id: String },
    #[error("document '{doc_id}' already exists")]
    DocAlreadyExists { doc_id: String },
    #[error("snapshot persistence error: {0}")]
    Io(#[from] std::io::Error),
    #[error("version vector decode error")]
    VersionVectorDecode,
    #[error("sync timeout after {0:?}")]
    Timeout(std::time::Duration),
}

// ── Key expressions ──

/// Zenoh key expression builders for the CRDT namespace.
///
/// Key expressions use hermetic `@` tenant prefixes:
/// `fractalaw/@{tenant}/crdt/{doc_id}/{suffix}`
pub mod crdt_keys {
    use super::PREFIX;

    /// Key expression for incremental CRDT updates.
    ///
    /// Example: `fractalaw/@acme/crdt/site-meta/updates`
    pub fn updates(tenant: &str, doc_id: &str) -> String {
        format!("{PREFIX}/@{tenant}/crdt/{doc_id}/updates")
    }

    /// Key expression for full snapshot queries.
    ///
    /// Example: `fractalaw/@acme/crdt/site-meta/snapshot`
    pub fn snapshot(tenant: &str, doc_id: &str) -> String {
        format!("{PREFIX}/@{tenant}/crdt/{doc_id}/snapshot")
    }

    /// Wildcard for all CRDT updates under a tenant.
    ///
    /// Example: `fractalaw/@acme/crdt/*/updates`
    pub fn updates_wildcard(tenant: &str) -> String {
        format!("{PREFIX}/@{tenant}/crdt/*/updates")
    }

    /// Extract the doc_id from a CRDT key expression.
    ///
    /// Given `fractalaw/@acme/crdt/site-meta/updates`,
    /// returns `Some("site-meta")`.
    pub fn doc_id_from_key(key_expr: &str) -> Option<&str> {
        let parts: Vec<&str> = key_expr.split('/').collect();
        parts
            .iter()
            .position(|&p| p == "crdt")
            .and_then(|i| parts.get(i + 1).copied())
    }
}

// ── Managed document ──

/// A Loro document with its auto-publish subscription handle.
struct ManagedDoc {
    doc: LoroDoc,
    /// Held to keep the `subscribe_local_update` subscription alive.
    /// Dropping this unsubscribes.
    _local_update_sub: Option<Subscription>,
}

// ── CrdtSync ──

/// CRDT sync engine: manages named Loro documents with Zenoh transport.
///
/// Each document is identified by a string `doc_id` that maps to zenoh
/// key expressions under `fractalaw/@{tenant}/crdt/{doc_id}/...`.
///
/// Documents live in-memory as `LoroDoc` instances. Snapshots persist to
/// `{persist_dir}/{doc_id}.loro` on explicit save.
pub struct CrdtSync {
    session: zenoh::Session,
    tenant: String,
    peer_id: u64,
    docs: Arc<RwLock<HashMap<String, ManagedDoc>>>,
    persist_dir: PathBuf,
}

impl CrdtSync {
    /// Open a new CRDT sync engine with default zenoh peer-mode config.
    ///
    /// `peer_id` should be unique per node (e.g., hash of machine-id + pid).
    /// `persist_dir` is where `.loro` snapshot files are stored.
    pub async fn new(tenant: &str, peer_id: u64, persist_dir: &Path) -> Result<Self, CrdtError> {
        let session = zenoh::open(zenoh::Config::default())
            .await
            .map_err(CrdtError::Session)?;
        info!(tenant = %tenant, peer_id = peer_id, "opened CRDT sync session");
        Ok(Self {
            session,
            tenant: tenant.to_string(),
            peer_id,
            docs: Arc::new(RwLock::new(HashMap::new())),
            persist_dir: persist_dir.to_path_buf(),
        })
    }

    /// Open with a custom zenoh config (for testing with in-process peers).
    pub async fn with_config(
        tenant: &str,
        peer_id: u64,
        persist_dir: &Path,
        config: zenoh::Config,
    ) -> Result<Self, CrdtError> {
        let session = zenoh::open(config).await.map_err(CrdtError::Session)?;
        info!(tenant = %tenant, peer_id = peer_id, "opened CRDT sync session with custom config");
        Ok(Self {
            session,
            tenant: tenant.to_string(),
            peer_id,
            docs: Arc::new(RwLock::new(HashMap::new())),
            persist_dir: persist_dir.to_path_buf(),
        })
    }

    /// Get the tenant namespace.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    // ── Document lifecycle ──

    /// Create a new empty Loro document with the given ID.
    ///
    /// The document is registered in-memory and a local update subscription
    /// is wired to auto-publish incremental updates to zenoh.
    pub async fn create_doc(&self, doc_id: &str) -> Result<(), CrdtError> {
        let mut docs = self.docs.write().await;
        if docs.contains_key(doc_id) {
            return Err(CrdtError::DocAlreadyExists {
                doc_id: doc_id.to_string(),
            });
        }

        let doc = LoroDoc::new();
        doc.set_peer_id(self.peer_id)?;

        let sub = self.wire_local_update_publisher(&doc, doc_id);

        docs.insert(
            doc_id.to_string(),
            ManagedDoc {
                doc,
                _local_update_sub: Some(sub),
            },
        );
        info!(doc_id = %doc_id, "created CRDT document");
        Ok(())
    }

    /// Open a document from a persisted snapshot, or create if not found.
    ///
    /// Looks for `{persist_dir}/{doc_id}.loro` and loads it if present.
    pub async fn open_or_create(&self, doc_id: &str) -> Result<(), CrdtError> {
        let snapshot_path = self.snapshot_path(doc_id);
        if snapshot_path.exists() {
            let bytes = std::fs::read(&snapshot_path)?;
            let doc = LoroDoc::from_snapshot(&bytes)?;
            doc.set_peer_id(self.peer_id)?;
            let sub = self.wire_local_update_publisher(&doc, doc_id);
            let mut docs = self.docs.write().await;
            docs.insert(
                doc_id.to_string(),
                ManagedDoc {
                    doc,
                    _local_update_sub: Some(sub),
                },
            );
            info!(doc_id = %doc_id, path = %snapshot_path.display(), "loaded CRDT document from snapshot");
        } else {
            self.create_doc(doc_id).await?;
        }
        Ok(())
    }

    /// List all currently loaded document IDs.
    pub async fn list_docs(&self) -> Vec<String> {
        self.docs.read().await.keys().cloned().collect()
    }

    /// Close a document, optionally persisting a final snapshot.
    pub async fn close_doc(&self, doc_id: &str, persist: bool) -> Result<(), CrdtError> {
        if persist {
            self.save_snapshot(doc_id).await?;
        }
        let mut docs = self.docs.write().await;
        docs.remove(doc_id);
        info!(doc_id = %doc_id, persisted = persist, "closed CRDT document");
        Ok(())
    }

    // ── Local mutations ──

    /// Insert or update a key-value pair in a named map container.
    ///
    /// `container` is the Loro map container name (e.g., "metadata", "status").
    /// Commits after the insert, triggering auto-publish.
    pub async fn map_insert(
        &self,
        doc_id: &str,
        container: &str,
        key: &str,
        value: impl Into<LoroValue>,
    ) -> Result<(), CrdtError> {
        let docs = self.docs.read().await;
        let managed = docs.get(doc_id).ok_or_else(|| CrdtError::DocNotFound {
            doc_id: doc_id.to_string(),
        })?;
        let map = managed.doc.get_map(container);
        map.insert(key, value)?;
        managed.doc.commit();
        Ok(())
    }

    /// Get a value from a named map container.
    pub async fn map_get(
        &self,
        doc_id: &str,
        container: &str,
        key: &str,
    ) -> Result<Option<ValueOrContainer>, CrdtError> {
        let docs = self.docs.read().await;
        let managed = docs.get(doc_id).ok_or_else(|| CrdtError::DocNotFound {
            doc_id: doc_id.to_string(),
        })?;
        let map = managed.doc.get_map(container);
        Ok(map.get(key))
    }

    /// Append a value to a named list container.
    ///
    /// Commits after the push, triggering auto-publish.
    pub async fn list_push(
        &self,
        doc_id: &str,
        container: &str,
        value: impl Into<LoroValue>,
    ) -> Result<(), CrdtError> {
        let docs = self.docs.read().await;
        let managed = docs.get(doc_id).ok_or_else(|| CrdtError::DocNotFound {
            doc_id: doc_id.to_string(),
        })?;
        let list = managed.doc.get_list(container);
        list.push(value)?;
        managed.doc.commit();
        Ok(())
    }

    /// Get the full deep value of a document (all containers resolved).
    pub async fn get_doc_value(&self, doc_id: &str) -> Result<LoroValue, CrdtError> {
        let docs = self.docs.read().await;
        let managed = docs.get(doc_id).ok_or_else(|| CrdtError::DocNotFound {
            doc_id: doc_id.to_string(),
        })?;
        Ok(managed.doc.get_deep_value())
    }

    /// Get the version vector of a document's oplog.
    pub async fn doc_version_vector(&self, doc_id: &str) -> Result<VersionVector, CrdtError> {
        let docs = self.docs.read().await;
        let managed = docs.get(doc_id).ok_or_else(|| CrdtError::DocNotFound {
            doc_id: doc_id.to_string(),
        })?;
        Ok(managed.doc.oplog_vv())
    }

    // ── Auto-publish wiring ──

    /// Wire a Loro doc's local update subscription to publish to zenoh.
    ///
    /// The `subscribe_local_update` callback is synchronous, so we bridge
    /// to async with `tokio::spawn` for the zenoh put.
    fn wire_local_update_publisher(&self, doc: &LoroDoc, doc_id: &str) -> Subscription {
        let session = self.session.clone();
        let key = crdt_keys::updates(&self.tenant, doc_id);
        let doc_id_owned = doc_id.to_string();

        doc.subscribe_local_update(Box::new(move |bytes: &Vec<u8>| {
            let session = session.clone();
            let key = key.clone();
            let bytes = bytes.clone();
            let doc_id = doc_id_owned.clone();

            tokio::spawn(async move {
                debug!(
                    doc_id = %doc_id,
                    key = %key,
                    bytes = bytes.len(),
                    "publishing CRDT incremental update"
                );
                if let Err(e) = session.put(&key, bytes).await {
                    error!(
                        doc_id = %doc_id,
                        error = %e,
                        "failed to publish CRDT update to zenoh"
                    );
                }
            });

            true // keep subscription alive
        }))
    }

    // ── Remote sync ──

    /// Start subscribing to CRDT updates from all peers for the tenant.
    ///
    /// Spawns a background task that receives incremental updates from zenoh
    /// and imports them into local Loro documents. Unknown doc_ids are skipped.
    pub async fn start_sync(&self) -> Result<tokio::task::JoinHandle<()>, CrdtError> {
        let key = crdt_keys::updates_wildcard(&self.tenant);
        let subscriber = self
            .session
            .declare_subscriber(&key)
            .await
            .map_err(CrdtError::Session)?;

        info!(key = %key, "subscribed to CRDT updates");

        let docs = Arc::clone(&self.docs);
        let handle = tokio::spawn(async move {
            while let Ok(sample) = subscriber.recv_async().await {
                let key_expr = sample.key_expr().as_str();
                let doc_id = match crdt_keys::doc_id_from_key(key_expr) {
                    Some(id) => id.to_string(),
                    None => {
                        warn!(key = %key_expr, "could not extract doc_id from CRDT key");
                        continue;
                    }
                };

                let payload = sample.payload().to_bytes();
                let docs = docs.read().await;

                if let Some(managed) = docs.get(&doc_id) {
                    match managed.doc.import(&payload) {
                        Ok(status) => {
                            debug!(
                                doc_id = %doc_id,
                                bytes = payload.len(),
                                has_pending = status.pending.is_some(),
                                "imported CRDT update from peer"
                            );
                            if status.pending.is_some() {
                                warn!(
                                    doc_id = %doc_id,
                                    "imported update has missing causal dependencies"
                                );
                            }
                        }
                        Err(e) => {
                            warn!(
                                doc_id = %doc_id,
                                error = %e,
                                "failed to import CRDT update"
                            );
                        }
                    }
                } else {
                    debug!(
                        doc_id = %doc_id,
                        "received CRDT update for unknown doc, ignoring"
                    );
                }
            }
        });

        Ok(handle)
    }

    // ── Late-joiner: queryable ──

    /// Declare a zenoh queryable that serves document snapshots on request.
    ///
    /// Responds with incremental updates (if the querier sends their VV as payload)
    /// or a full snapshot (if no payload).
    pub async fn serve_snapshots(&self) -> Result<tokio::task::JoinHandle<()>, CrdtError> {
        let key = format!("{PREFIX}/@{}/crdt/*/snapshot", self.tenant);
        let queryable = self
            .session
            .declare_queryable(&key)
            .await
            .map_err(CrdtError::Session)?;

        info!(key = %key, "serving CRDT snapshots via queryable");

        let docs = Arc::clone(&self.docs);
        let tenant = self.tenant.clone();

        let handle = tokio::spawn(async move {
            while let Ok(query) = queryable.recv_async().await {
                let key_expr = query.key_expr().as_str();
                let doc_id = match crdt_keys::doc_id_from_key(key_expr) {
                    Some(id) => id.to_string(),
                    None => continue,
                };

                let docs = docs.read().await;
                if let Some(managed) = docs.get(&doc_id) {
                    let export_bytes = if let Some(payload) = query.payload() {
                        let peer_vv_bytes = payload.to_bytes();
                        match VersionVector::decode(&peer_vv_bytes) {
                            Ok(peer_vv) => managed.doc.export(ExportMode::updates_owned(peer_vv)),
                            Err(_) => managed.doc.export(ExportMode::Snapshot),
                        }
                    } else {
                        managed.doc.export(ExportMode::Snapshot)
                    };

                    match export_bytes {
                        Ok(bytes) => {
                            let reply_key = crdt_keys::snapshot(&tenant, &doc_id);
                            if let Err(e) = query.reply(&reply_key, bytes).await {
                                warn!(doc_id = %doc_id, error = %e, "failed to reply to snapshot query");
                            }
                        }
                        Err(e) => {
                            warn!(doc_id = %doc_id, error = %e, "failed to export snapshot");
                        }
                    }
                }
            }
        });

        Ok(handle)
    }

    // ── Late-joiner: query ──

    /// Request a snapshot or incremental update from peers for a document.
    ///
    /// Sends this node's VV as the query payload so peers can respond with
    /// only the missing updates.
    pub async fn request_sync(
        &self,
        doc_id: &str,
        timeout: std::time::Duration,
    ) -> Result<(), CrdtError> {
        let key = crdt_keys::snapshot(&self.tenant, doc_id);

        let vv_bytes = {
            let docs = self.docs.read().await;
            match docs.get(doc_id) {
                Some(managed) => managed.doc.oplog_vv().encode(),
                None => VersionVector::default().encode(),
            }
        };

        debug!(doc_id = %doc_id, "requesting CRDT sync from peers");

        let replies = self
            .session
            .get(&key)
            .payload(vv_bytes)
            .timeout(timeout)
            .await
            .map_err(CrdtError::Session)?;

        let mut received = false;
        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                let bytes = sample.payload().to_bytes();

                // Ensure the doc exists locally.
                {
                    let docs = self.docs.read().await;
                    if !docs.contains_key(doc_id) {
                        drop(docs);
                        self.create_doc(doc_id).await?;
                    }
                }

                let docs = self.docs.read().await;
                if let Some(managed) = docs.get(doc_id) {
                    managed.doc.import(&bytes)?;
                    received = true;
                    info!(
                        doc_id = %doc_id,
                        bytes = bytes.len(),
                        "imported sync response from peer"
                    );
                }
            }
        }

        if !received {
            debug!(doc_id = %doc_id, "no peers responded to sync request");
        }

        Ok(())
    }

    // ── Persistence ──

    /// Save a document's current state as a snapshot file.
    ///
    /// Writes to `{persist_dir}/{doc_id}.loro`. Uses atomic write (temp + rename).
    pub async fn save_snapshot(&self, doc_id: &str) -> Result<PathBuf, CrdtError> {
        let docs = self.docs.read().await;
        let managed = docs.get(doc_id).ok_or_else(|| CrdtError::DocNotFound {
            doc_id: doc_id.to_string(),
        })?;

        let bytes = managed.doc.export(ExportMode::Snapshot)?;
        let path = self.snapshot_path(doc_id);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Atomic write: temp file + rename
        let tmp_path = path.with_extension("loro.tmp");
        std::fs::write(&tmp_path, &bytes)?;
        std::fs::rename(&tmp_path, &path)?;

        info!(
            doc_id = %doc_id,
            path = %path.display(),
            bytes = bytes.len(),
            "saved CRDT snapshot"
        );
        Ok(path)
    }

    /// Save all loaded documents' snapshots.
    pub async fn save_all_snapshots(&self) -> Result<Vec<PathBuf>, CrdtError> {
        let doc_ids: Vec<String> = self.docs.read().await.keys().cloned().collect();
        let mut paths = Vec::new();
        for doc_id in &doc_ids {
            paths.push(self.save_snapshot(doc_id).await?);
        }
        Ok(paths)
    }

    /// List all persisted snapshot files in the persistence directory.
    pub fn list_persisted_docs(&self) -> Result<Vec<String>, CrdtError> {
        let mut doc_ids = Vec::new();
        if self.persist_dir.exists() {
            for entry in std::fs::read_dir(&self.persist_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "loro")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    doc_ids.push(stem.to_string());
                }
            }
        }
        doc_ids.sort();
        Ok(doc_ids)
    }

    fn snapshot_path(&self, doc_id: &str) -> PathBuf {
        self.persist_dir.join(format!("{doc_id}.loro"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ── Key expression tests ──

    #[test]
    fn crdt_keys_updates() {
        assert_eq!(
            crdt_keys::updates("acme", "site-meta"),
            "fractalaw/@acme/crdt/site-meta/updates"
        );
    }

    #[test]
    fn crdt_keys_snapshot() {
        assert_eq!(
            crdt_keys::snapshot("acme", "risk-london"),
            "fractalaw/@acme/crdt/risk-london/snapshot"
        );
    }

    #[test]
    fn crdt_keys_updates_wildcard() {
        assert_eq!(
            crdt_keys::updates_wildcard("acme"),
            "fractalaw/@acme/crdt/*/updates"
        );
    }

    #[test]
    fn crdt_keys_doc_id_extraction() {
        assert_eq!(
            crdt_keys::doc_id_from_key("fractalaw/@acme/crdt/site-meta/updates"),
            Some("site-meta")
        );
        assert_eq!(
            crdt_keys::doc_id_from_key("fractalaw/@acme/crdt/risk-london/snapshot"),
            Some("risk-london")
        );
    }

    // ── Version vector roundtrip ──

    #[test]
    fn version_vector_encode_decode() {
        let doc = LoroDoc::new();
        doc.set_peer_id(42).unwrap();
        let map = doc.get_map("test");
        map.insert("key", "value").unwrap();
        doc.commit();

        let vv = doc.oplog_vv();
        let encoded = vv.encode();
        let decoded = VersionVector::decode(&encoded).unwrap();
        assert_eq!(vv, decoded);
    }

    // ── Loro basic operations ──

    #[test]
    fn loro_map_insert_get() {
        let doc = LoroDoc::new();
        let map = doc.get_map("metadata");
        map.insert("name", "London HQ").unwrap();
        map.insert("active", true).unwrap();
        doc.commit();

        let name = map.get("name").unwrap().get_deep_value();
        assert_eq!(name, LoroValue::String("London HQ".into()));

        let active = map.get("active").unwrap().get_deep_value();
        assert_eq!(active, LoroValue::Bool(true));
    }

    #[test]
    fn loro_list_push_get() {
        let doc = LoroDoc::new();
        let list = doc.get_list("actions");
        list.push("action-1").unwrap();
        list.push("action-2").unwrap();
        doc.commit();

        assert_eq!(list.len(), 2);
        let v0 = list.get(0).unwrap().get_deep_value();
        assert_eq!(v0, LoroValue::String("action-1".into()));
    }

    // ── Snapshot persistence roundtrip ──

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn snapshot_persistence_roundtrip() {
        let tmp = std::env::temp_dir().join("fractalaw-crdt-test-persist");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let sync = CrdtSync::new("test-persist", 100, &tmp).await.unwrap();

        // Create doc and add data
        sync.create_doc("my-doc").await.unwrap();
        sync.map_insert("my-doc", "meta", "name", "Test Site")
            .await
            .unwrap();
        sync.map_insert("my-doc", "meta", "active", true)
            .await
            .unwrap();

        // Save snapshot
        let path = sync.save_snapshot("my-doc").await.unwrap();
        assert!(path.exists());

        // List persisted docs
        let persisted = sync.list_persisted_docs().unwrap();
        assert_eq!(persisted, vec!["my-doc"]);

        // Close and reopen
        sync.close_doc("my-doc", false).await.unwrap();
        assert!(sync.list_docs().await.is_empty());

        sync.open_or_create("my-doc").await.unwrap();
        let value = sync.get_doc_value("my-doc").await.unwrap();
        // Deep value should contain "meta" map with "name" and "active"
        if let LoroValue::Map(map) = &value {
            let meta = map.get("meta").unwrap();
            if let LoroValue::Map(meta_map) = meta {
                assert_eq!(
                    meta_map.get("name").unwrap(),
                    &LoroValue::String("Test Site".into())
                );
                assert_eq!(meta_map.get("active").unwrap(), &LoroValue::Bool(true));
            } else {
                panic!("expected Map for 'meta', got: {meta:?}");
            }
        } else {
            panic!("expected Map at root, got: {value:?}");
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ── Two-peer sync ──

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn two_peer_map_sync() {
        let tmp_a = std::env::temp_dir().join("fractalaw-crdt-test-a");
        let tmp_b = std::env::temp_dir().join("fractalaw-crdt-test-b");
        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&tmp_b);

        let peer_a = CrdtSync::new("test-crdt-sync", 1, &tmp_a).await.unwrap();
        let peer_b = CrdtSync::new("test-crdt-sync", 2, &tmp_b).await.unwrap();

        peer_a.create_doc("shared").await.unwrap();
        peer_b.create_doc("shared").await.unwrap();

        // Start sync on both sides
        let _handle_a = peer_a.start_sync().await.unwrap();
        let _handle_b = peer_b.start_sync().await.unwrap();

        // Let subscriptions propagate
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Peer A writes
        peer_a
            .map_insert("shared", "meta", "name", "London HQ")
            .await
            .unwrap();

        // Wait for propagation
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Peer B should see the value
        let value = peer_b.map_get("shared", "meta", "name").await.unwrap();
        assert!(value.is_some(), "Peer B should see Peer A's write");
        let deep = value.unwrap().get_deep_value();
        assert_eq!(deep, LoroValue::String("London HQ".into()));

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&tmp_b);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn two_peer_concurrent_edits() {
        let tmp_a = std::env::temp_dir().join("fractalaw-crdt-test-conc-a");
        let tmp_b = std::env::temp_dir().join("fractalaw-crdt-test-conc-b");
        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&tmp_b);

        let peer_a = CrdtSync::new("test-crdt-conc", 10, &tmp_a).await.unwrap();
        let peer_b = CrdtSync::new("test-crdt-conc", 20, &tmp_b).await.unwrap();

        peer_a.create_doc("shared").await.unwrap();
        peer_b.create_doc("shared").await.unwrap();

        let _ha = peer_a.start_sync().await.unwrap();
        let _hb = peer_b.start_sync().await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Both peers edit different keys concurrently
        peer_a
            .map_insert("shared", "meta", "from_a", "hello from A")
            .await
            .unwrap();
        peer_b
            .map_insert("shared", "meta", "from_b", "hello from B")
            .await
            .unwrap();

        // Wait for convergence
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Both peers should have both keys
        let a_sees_b = peer_a.map_get("shared", "meta", "from_b").await.unwrap();
        let b_sees_a = peer_b.map_get("shared", "meta", "from_a").await.unwrap();
        assert!(a_sees_b.is_some(), "Peer A should see Peer B's write");
        assert!(b_sees_a.is_some(), "Peer B should see Peer A's write");

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&tmp_b);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn late_joiner_snapshot() {
        let tmp_a = std::env::temp_dir().join("fractalaw-crdt-test-late-a");
        let tmp_b = std::env::temp_dir().join("fractalaw-crdt-test-late-b");
        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&tmp_b);

        let peer_a = CrdtSync::new("test-crdt-late", 100, &tmp_a).await.unwrap();

        // Peer A creates doc and makes several edits
        peer_a.create_doc("shared").await.unwrap();
        peer_a
            .map_insert("shared", "meta", "name", "London HQ")
            .await
            .unwrap();
        peer_a
            .map_insert("shared", "meta", "status", "active")
            .await
            .unwrap();
        peer_a
            .list_push("shared", "actions", "audit-2026-01")
            .await
            .unwrap();

        // Peer A serves snapshots
        let _qa = peer_a.serve_snapshots().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Peer B joins late, requests sync
        let peer_b = CrdtSync::new("test-crdt-late", 200, &tmp_b).await.unwrap();
        peer_b.create_doc("shared").await.unwrap();
        peer_b
            .request_sync("shared", Duration::from_secs(5))
            .await
            .unwrap();

        // Peer B should have all of Peer A's data
        let name = peer_b.map_get("shared", "meta", "name").await.unwrap();
        assert!(name.is_some(), "Peer B should have name from snapshot");
        assert_eq!(
            name.unwrap().get_deep_value(),
            LoroValue::String("London HQ".into())
        );

        let status = peer_b.map_get("shared", "meta", "status").await.unwrap();
        assert!(status.is_some(), "Peer B should have status from snapshot");

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&tmp_b);
    }
}
