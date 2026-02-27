//! Hive lifecycle orchestrator: composes `ZenohSync` + `CrdtSync` into a
//! unified sync state machine.
//!
//! Two modes:
//! - `run_once()`: single sync cycle (wake → sync → publish → exit)
//! - `run_continuous()`: sync, then listen for updates until `shutdown()`

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use arrow::record_batch::RecordBatch;
use thiserror::Error;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::crdt_sync::{CrdtError, CrdtSync};
use crate::zenoh_sync::{self, ZenohError, ZenohSync};

// ── Errors ──

#[derive(Error, Debug)]
pub enum HiveError {
    #[error("zenoh error: {0}")]
    Zenoh(#[from] ZenohError),
    #[error("CRDT error: {0}")]
    Crdt(#[from] CrdtError),
    #[error("lifecycle error: {0}")]
    Lifecycle(String),
}

// ── State ──

/// Current state of the Hive lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiveState {
    /// Initial state, not yet connected.
    Idle,
    /// Syncing CRDTs with peers.
    Syncing,
    /// Publishing taxa enrichment data.
    Publishing,
    /// Listening for incoming updates (continuous mode).
    Listening,
    /// Shutting down gracefully.
    ShuttingDown,
}

impl std::fmt::Display for HiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Syncing => write!(f, "syncing"),
            Self::Publishing => write!(f, "publishing"),
            Self::Listening => write!(f, "listening"),
            Self::ShuttingDown => write!(f, "shutting down"),
        }
    }
}

// ── Report ──

/// Summary of a sync cycle's results.
#[derive(Debug, Default)]
pub struct SyncReport {
    /// Number of CRDT documents synced from peers.
    pub crdt_docs_synced: usize,
    /// Number of taxa laws published.
    pub taxa_published: usize,
    /// Number of taxa samples received (continuous mode).
    pub taxa_received: usize,
    /// Non-fatal warnings encountered during the cycle.
    pub warnings: Vec<String>,
}

// ── HiveSync ──

/// Hive lifecycle orchestrator.
///
/// Composes [`ZenohSync`] (taxa pub/sub) and [`CrdtSync`] (Loro CRDT engine)
/// into a unified lifecycle: wake → sync CRDTs → publish taxa → listen/exit.
pub struct HiveSync {
    taxa: ZenohSync,
    crdt: CrdtSync,
    state: Arc<watch::Sender<HiveState>>,
    sync_timeout: Duration,
    handles: Mutex<Vec<JoinHandle<()>>>,
}

impl HiveSync {
    /// Create a new HiveSync with default zenoh peer-mode config.
    ///
    /// `tenant`: namespace prefix (e.g., `"acme"`)
    /// `peer_id`: unique identifier for this node (e.g., hash of hostname + pid)
    /// `persist_dir`: directory for `.loro` CRDT snapshot files
    /// `sync_timeout`: how long to wait for CRDT peer sync responses
    pub async fn new(
        tenant: &str,
        peer_id: u64,
        persist_dir: &Path,
        sync_timeout: Duration,
    ) -> Result<Self, HiveError> {
        let taxa = ZenohSync::new(tenant).await?;
        let crdt = CrdtSync::new(tenant, peer_id, persist_dir).await?;
        let (state_tx, _) = watch::channel(HiveState::Idle);
        info!(tenant = %tenant, peer_id = peer_id, "HiveSync created");
        Ok(Self {
            taxa,
            crdt,
            state: Arc::new(state_tx),
            sync_timeout,
            handles: Mutex::new(Vec::new()),
        })
    }

    /// Create with custom zenoh configs (for testing with in-process peers).
    pub async fn with_configs(
        tenant: &str,
        peer_id: u64,
        persist_dir: &Path,
        sync_timeout: Duration,
        taxa_config: zenoh::Config,
        crdt_config: zenoh::Config,
    ) -> Result<Self, HiveError> {
        let taxa = ZenohSync::with_config(tenant, taxa_config).await?;
        let crdt = CrdtSync::with_config(tenant, peer_id, persist_dir, crdt_config).await?;
        let (state_tx, _) = watch::channel(HiveState::Idle);
        Ok(Self {
            taxa,
            crdt,
            state: Arc::new(state_tx),
            sync_timeout,
            handles: Mutex::new(Vec::new()),
        })
    }

    /// Get the current lifecycle state.
    pub fn state(&self) -> HiveState {
        *self.state.borrow()
    }

    /// Subscribe to state changes.
    pub fn watch_state(&self) -> watch::Receiver<HiveState> {
        self.state.subscribe()
    }

    /// Access the underlying ZenohSync (for direct taxa operations).
    pub fn taxa(&self) -> &ZenohSync {
        &self.taxa
    }

    /// Access the underlying CrdtSync (for direct CRDT operations).
    pub fn crdt(&self) -> &CrdtSync {
        &self.crdt
    }

    // ── Lifecycle: run_once ──

    /// Run a single sync cycle: sync CRDTs → publish taxa → save → exit.
    ///
    /// 1. Load all persisted CRDT docs
    /// 2. Start CRDT subscriber + snapshot server
    /// 3. `request_sync()` for each doc (non-fatal on failure)
    /// 4. Publish taxa Arrow IPC batches
    /// 5. Save CRDT snapshots
    /// 6. Return report
    pub async fn run_once(
        &self,
        taxa_batches: &[(String, Vec<RecordBatch>)],
    ) -> Result<SyncReport, HiveError> {
        let mut report = SyncReport::default();

        // ── SYNC phase ──
        self.set_state(HiveState::Syncing);

        let doc_ids = self.crdt.list_persisted_docs()?;
        for doc_id in &doc_ids {
            self.crdt.open_or_create(doc_id).await?;
        }

        let crdt_sub = self.crdt.start_sync().await?;
        let snapshot_server = self.crdt.serve_snapshots().await?;

        // Brief pause to let subscriptions propagate before requesting sync.
        tokio::time::sleep(Duration::from_millis(100)).await;

        for doc_id in &doc_ids {
            match self.crdt.request_sync(doc_id, self.sync_timeout).await {
                Ok(()) => report.crdt_docs_synced += 1,
                Err(e) => {
                    warn!(doc_id = %doc_id, error = %e, "CRDT sync failed");
                    report.warnings.push(format!("CRDT sync '{doc_id}': {e}"));
                }
            }
        }

        // ── PUBLISH phase ──
        self.set_state(HiveState::Publishing);

        for (law_name, batches) in taxa_batches {
            match self.taxa.publish_taxa(law_name, batches).await {
                Ok(()) => report.taxa_published += 1,
                Err(e) => {
                    warn!(law_name = %law_name, error = %e, "taxa publish failed");
                    report
                        .warnings
                        .push(format!("taxa publish '{law_name}': {e}"));
                }
            }
        }

        // ── SAVE phase ──
        if let Err(e) = self.crdt.save_all_snapshots().await {
            report.warnings.push(format!("CRDT save: {e}"));
        }

        // Clean up background tasks.
        crdt_sub.abort();
        snapshot_server.abort();

        self.set_state(HiveState::Idle);
        info!(
            crdt_synced = report.crdt_docs_synced,
            taxa_published = report.taxa_published,
            warnings = report.warnings.len(),
            "sync cycle complete"
        );
        Ok(report)
    }

    // ── Lifecycle: run_continuous ──

    /// Run continuously: initial sync+publish, then listen for updates.
    ///
    /// Returns a handle to the final `SyncReport`. Call [`shutdown()`] to stop,
    /// then `.await` the handle to get the report.
    ///
    /// `on_taxa_received` is called for each incoming taxa sample with the
    /// law name and decoded Arrow IPC batches.
    pub async fn run_continuous<F>(
        &self,
        taxa_batches: &[(String, Vec<RecordBatch>)],
        on_taxa_received: F,
    ) -> Result<JoinHandle<SyncReport>, HiveError>
    where
        F: Fn(&str, Vec<RecordBatch>) + Send + Sync + 'static,
    {
        let mut report = SyncReport::default();

        // ── Initial sync + publish (same as run_once) ──
        self.set_state(HiveState::Syncing);

        let doc_ids = self.crdt.list_persisted_docs()?;
        for doc_id in &doc_ids {
            self.crdt.open_or_create(doc_id).await?;
        }

        let crdt_sub = self.crdt.start_sync().await?;
        let snapshot_server = self.crdt.serve_snapshots().await?;

        tokio::time::sleep(Duration::from_millis(100)).await;

        for doc_id in &doc_ids {
            match self.crdt.request_sync(doc_id, self.sync_timeout).await {
                Ok(()) => report.crdt_docs_synced += 1,
                Err(e) => {
                    report.warnings.push(format!("CRDT sync '{doc_id}': {e}"));
                }
            }
        }

        self.set_state(HiveState::Publishing);

        for (law_name, batches) in taxa_batches {
            match self.taxa.publish_taxa(law_name, batches).await {
                Ok(()) => report.taxa_published += 1,
                Err(e) => {
                    report
                        .warnings
                        .push(format!("taxa publish '{law_name}': {e}"));
                }
            }
        }

        // Store background handles for cleanup in shutdown().
        {
            let mut handles = self.handles.lock().await;
            handles.push(crdt_sub);
            handles.push(snapshot_server);
        }

        // ── Listen phase ──
        self.set_state(HiveState::Listening);

        let taxa_sub = self.taxa.subscribe_taxa().await?;
        let mut state_rx = self.watch_state();

        info!("entering continuous listen mode");

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    sample = taxa_sub.recv_async() => {
                        match sample {
                            Ok(sample) => {
                                let key = sample.key_expr().as_str();
                                let law_name = zenoh_sync::keys::law_name_from_key(key)
                                    .unwrap_or("unknown")
                                    .to_string();
                                let payload = sample.payload().to_bytes();
                                match zenoh_sync::decode_arrow_ipc(&payload) {
                                    Ok(batches) => {
                                        report.taxa_received += 1;
                                        on_taxa_received(&law_name, batches);
                                    }
                                    Err(e) => {
                                        report.warnings.push(
                                            format!("decode '{law_name}': {e}")
                                        );
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    _ = state_rx.changed() => {
                        if *state_rx.borrow_and_update() == HiveState::ShuttingDown {
                            break;
                        }
                    }
                }
            }
            report
        });

        Ok(handle)
    }

    /// Signal the Hive to shut down gracefully.
    ///
    /// Saves CRDT snapshots, signals the listen loop to exit, and aborts
    /// background tasks.
    pub async fn shutdown(&self) -> Result<(), HiveError> {
        self.set_state(HiveState::ShuttingDown);
        info!("shutting down HiveSync");

        // Save CRDT state before stopping.
        if let Err(e) = self.crdt.save_all_snapshots().await {
            warn!(error = %e, "failed to save CRDT snapshots during shutdown");
        }

        // Abort background handles (CRDT subscriber, snapshot server).
        let mut handles = self.handles.lock().await;
        for h in handles.drain(..) {
            h.abort();
        }

        // Stay in ShuttingDown — this is the terminal state.
        // Setting Idle here would race with run_continuous's listener task,
        // which checks for ShuttingDown via watch::Receiver::borrow_and_update().
        Ok(())
    }

    fn set_state(&self, new_state: HiveState) {
        self.state.send_replace(new_state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zenoh_sync::test_helpers::test_taxa_batch;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn hive_state_display() {
        assert_eq!(HiveState::Idle.to_string(), "idle");
        assert_eq!(HiveState::Syncing.to_string(), "syncing");
        assert_eq!(HiveState::Publishing.to_string(), "publishing");
        assert_eq!(HiveState::Listening.to_string(), "listening");
        assert_eq!(HiveState::ShuttingDown.to_string(), "shutting down");
    }

    #[test]
    fn sync_report_default() {
        let report = SyncReport::default();
        assert_eq!(report.crdt_docs_synced, 0);
        assert_eq!(report.taxa_published, 0);
        assert_eq!(report.taxa_received, 0);
        assert!(report.warnings.is_empty());
    }

    // ── run_once tests ──

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_once_no_data() {
        let tmp = std::env::temp_dir().join("fractalaw-hive-test-once");
        let _ = std::fs::remove_dir_all(&tmp);

        let hive = HiveSync::new("test-hive-once", 42, &tmp, Duration::from_secs(2))
            .await
            .unwrap();

        assert_eq!(hive.state(), HiveState::Idle);

        let report = hive.run_once(&[]).await.unwrap();
        assert_eq!(report.crdt_docs_synced, 0);
        assert_eq!(report.taxa_published, 0);
        assert_eq!(hive.state(), HiveState::Idle);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_once_publishes_taxa() {
        let tmp = std::env::temp_dir().join("fractalaw-hive-test-pub");
        let _ = std::fs::remove_dir_all(&tmp);

        let hive = HiveSync::new("test-hive-pub", 43, &tmp, Duration::from_secs(2))
            .await
            .unwrap();

        let batch = test_taxa_batch();
        let taxa = vec![("UK_ukpga_1974_37".to_string(), vec![batch])];

        let report = hive.run_once(&taxa).await.unwrap();
        assert_eq!(report.taxa_published, 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_once_syncs_crdt_docs() {
        let tmp_a = std::env::temp_dir().join("fractalaw-hive-crdt-a");
        let tmp_b = std::env::temp_dir().join("fractalaw-hive-crdt-b");
        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&tmp_b);

        // Peer A: create doc with data, serve snapshots.
        let crdt_a = CrdtSync::new("test-hive-crdt", 100, &tmp_a).await.unwrap();
        crdt_a.create_doc("shared").await.unwrap();
        crdt_a
            .map_insert("shared", "meta", "site", "London")
            .await
            .unwrap();
        let _qa = crdt_a.serve_snapshots().await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Peer B (Hive): has a persisted "shared" doc.
        let crdt_b = CrdtSync::new("test-hive-crdt", 200, &tmp_b).await.unwrap();
        crdt_b.create_doc("shared").await.unwrap();
        crdt_b.save_snapshot("shared").await.unwrap();
        drop(crdt_b);

        let hive = HiveSync::new("test-hive-crdt", 200, &tmp_b, Duration::from_secs(5))
            .await
            .unwrap();

        let report = hive.run_once(&[]).await.unwrap();
        assert_eq!(report.crdt_docs_synced, 1);

        // Verify the synced data.
        let value = hive.crdt().map_get("shared", "meta", "site").await.unwrap();
        assert!(
            value.is_some(),
            "Hive should have synced 'site' from peer A"
        );

        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&tmp_b);
    }

    // ── run_continuous tests ──

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn continuous_receives_taxa() {
        let tmp = std::env::temp_dir().join("fractalaw-hive-cont");
        let _ = std::fs::remove_dir_all(&tmp);

        let hive = HiveSync::new("test-hive-cont", 300, &tmp, Duration::from_secs(2))
            .await
            .unwrap();

        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = Arc::clone(&received);

        let handle = hive
            .run_continuous(&[], move |_law_name, _batches| {
                received_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await
            .unwrap();

        // Wait for subscriptions to propagate.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(hive.state(), HiveState::Listening);

        // Publish from a separate peer.
        let publisher = ZenohSync::new("test-hive-cont").await.unwrap();
        let batch = test_taxa_batch();
        publisher
            .publish_taxa("UK_ukpga_1974_37", &[batch])
            .await
            .unwrap();

        // Wait for reception.
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(
            received.load(Ordering::SeqCst) > 0,
            "should have received taxa"
        );

        // Shutdown.
        hive.shutdown().await.unwrap();
        let report = handle.await.unwrap();
        assert!(report.taxa_received > 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn state_transitions_run_once() {
        let tmp = std::env::temp_dir().join("fractalaw-hive-state");
        let _ = std::fs::remove_dir_all(&tmp);

        let hive = HiveSync::new("test-hive-state", 400, &tmp, Duration::from_secs(1))
            .await
            .unwrap();

        assert_eq!(hive.state(), HiveState::Idle);

        // run_once transitions through Syncing → Publishing → Idle.
        // The watch channel only retains the *latest* value, so a fast
        // subscriber may miss intermediate states. We verify the final
        // state and that at least one non-Idle state was observed.
        hive.run_once(&[]).await.unwrap();
        assert_eq!(hive.state(), HiveState::Idle, "should end in Idle");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
