//! Storage layer: DuckDB (analytical), LanceDB (vector), DataFusion (unified query).

mod error;
pub use error::StoreError;

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

#[cfg(all(feature = "duckdb", feature = "datafusion"))]
mod fusion;
#[cfg(all(feature = "duckdb", feature = "datafusion"))]
pub use fusion::FusionStore;
