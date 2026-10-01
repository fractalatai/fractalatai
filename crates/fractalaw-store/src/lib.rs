//! Storage layer: DuckDB (analytical), LanceDB (vector), DataFusion (unified query).

mod error;
pub use error::StoreError;

mod provision_store;
pub use provision_store::{LawDrrpInputs, ProvisionStore};

/// The hub's primary provision store (Postgres + pgvector): the CLIs' default (#71).
pub const HUB_PG_URL: &str = "postgres://fractalaw:fractalaw@localhost:5433/fractalaw";

#[cfg(feature = "duckdb")]
mod duck;
#[cfg(feature = "duckdb")]
pub use duck::DuckStore;

#[cfg(feature = "lancedb")]
mod lance;
#[cfg(feature = "lancedb")]
pub use lance::{LanceStore, read_parquet};

#[cfg(feature = "pg")]
mod pg;
#[cfg(feature = "pg")]
pub use pg::PgStore;
#[cfg(feature = "pg")]
mod pg_lat_sync;
#[cfg(feature = "pg")]
mod pg_provenance;
#[cfg(feature = "pg")]
pub use pg_lat_sync::{LatApplyReport, LatFieldRow, LatSyncState, LegalAmendment, TierCounts, VersionBatch, VersionSpec};
#[cfg(feature = "pg")]
pub use sqlx::PgPool;

#[cfg(all(feature = "duckdb", feature = "datafusion"))]
mod fusion;
#[cfg(all(feature = "duckdb", feature = "datafusion"))]
pub use fusion::FusionStore;
