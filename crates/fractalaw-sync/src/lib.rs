//! Sync layer: Arrow Flight RPC transport, Zenoh pub/sub, Loro CRDTs for conflict resolution.

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "http")]
pub use http::{SyncClient, SyncError};

#[cfg(feature = "zenoh")]
pub mod zenoh_sync;

#[cfg(feature = "zenoh")]
pub use zenoh_sync::{SyncEvent, ZenohError, ZenohSync};

#[cfg(feature = "zenoh")]
pub mod crdt_sync;

#[cfg(feature = "zenoh")]
pub use crdt_sync::{CrdtError, CrdtSync};

#[cfg(feature = "zenoh")]
pub mod hive;

#[cfg(feature = "zenoh")]
pub use hive::{HiveError, HiveState, HiveSync, SyncReport};
