mod sync;

use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use fractalaw_store::DuckStore;

#[derive(Parser)]
#[command(
    name = "fractalaw-sync",
    version,
    about = "Zenoh sync tools — publish, watch, pull-lat"
)]
struct Cli {
    /// Path to data directory containing DuckDB and runtime data
    #[arg(long, default_value = "./data", global = true)]
    data_dir: PathBuf,

    /// Use PostgreSQL+pgvector instead of LanceDB for provision store
    #[arg(long, global = true, env = "FRACTALAW_PG")]
    pg: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Push polished results to sertantai inbox
    Push {
        /// Sertantai base URL (e.g. http://localhost:4000)
        #[arg(long, env = "SERTANTAI_URL")]
        url: String,
    },
    /// Pull new annotations from sertantai outbox
    Pull {
        /// Sertantai base URL (e.g. http://localhost:4000)
        #[arg(long, env = "SERTANTAI_URL")]
        url: String,
    },
    /// Publish taxa enrichment to zenoh mesh
    Publish {
        #[command(flatten)]
        zenoh: ZenohArgs,
        /// Specific laws to publish (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Publish all laws in a DuckDB family
        #[arg(long)]
        family: Option<String>,
        /// Publish ALL laws with taxa data (must be explicit)
        #[arg(long)]
        all: bool,
        /// Only publish laws whose taxa changed since last publish
        #[arg(long)]
        changed: bool,
        /// Publish provision-level taxa (from Postgres) instead of law-level
        #[arg(long)]
        provisions: bool,
        /// Publish laws recently enriched but not yet published
        #[arg(long)]
        pending: bool,
    },
    /// Pull legislation text (LAT) from sertantai via zenoh
    PullLat {
        #[command(flatten)]
        zenoh: ZenohArgs,
        /// Law names to pull (comma-separated)
        #[arg(long)]
        laws: String,
        /// Query timeout in seconds
        #[arg(long, default_value_t = 30)]
        timeout: u64,
    },
    /// Watch for sync events and run the full round-trip pipeline (long-running)
    Watch {
        #[command(flatten)]
        zenoh: ZenohArgs,
        /// Query timeout in seconds (per-law pull)
        #[arg(long, default_value_t = 30)]
        timeout: u64,
    },
    /// CRDT document management and sync
    Crdt {
        #[command(subcommand)]
        action: CrdtAction,
    },
}

#[derive(Subcommand)]
enum CrdtAction {
    /// Show status of persisted CRDT documents
    Status {
        #[command(flatten)]
        zenoh: ZenohArgs,
    },
    /// Create a new empty CRDT document
    Create {
        /// Document ID
        doc_id: String,
        #[command(flatten)]
        zenoh: ZenohArgs,
    },
    /// Inspect a CRDT document's current state
    Inspect {
        /// Document ID
        doc_id: String,
        #[command(flatten)]
        zenoh: ZenohArgs,
    },
    /// Save all loaded CRDT documents to disk
    Save {
        #[command(flatten)]
        zenoh: ZenohArgs,
    },
}

/// Shared Zenoh connectivity args for all sync subcommands.
#[derive(clap::Args, Clone, Debug)]
pub(crate) struct ZenohArgs {
    /// Tenant namespace
    #[arg(long, env = "FRACTALAW_TENANT", default_value = "local")]
    pub(crate) tenant: String,

    /// Zenoh endpoint to connect to (e.g., tcp/1.2.3.4:7447 or tls/host:7447).
    #[arg(long, env = "ZENOH_ENDPOINT")]
    connect: Option<String>,

    /// Path to a Zenoh JSON5 config file (advanced).
    #[arg(long, env = "ZENOH_CONFIG", conflicts_with = "connect")]
    zenoh_config: Option<PathBuf>,

    /// Root CA certificate for TLS verification (PEM).
    #[arg(long, env = "ZENOH_TLS_CA", requires = "connect")]
    tls_ca: Option<PathBuf>,

    /// Client certificate for mutual TLS (PEM).
    #[arg(long, env = "ZENOH_TLS_CERT", requires = "tls_key")]
    tls_cert: Option<PathBuf>,

    /// Client private key for mutual TLS (PEM).
    #[arg(long, env = "ZENOH_TLS_KEY", requires = "tls_cert")]
    tls_key: Option<PathBuf>,
}

impl ZenohArgs {
    pub(crate) fn build_zenoh_config(&self) -> anyhow::Result<zenoh::Config> {
        if let Some(ref endpoint) = self.connect {
            let tls_block = self.build_tls_json5(endpoint)?;
            let json5 = format!(
                r#"{{
                    mode: "client",
                    connect: {{ endpoints: ["{endpoint}"] }},
                    scouting: {{ multicast: {{ enabled: false }} }}{tls_block}
                }}"#
            );
            zenoh::Config::from_json5(&json5).map_err(|e| {
                anyhow::anyhow!("failed to build zenoh client config for '{endpoint}': {e}")
            })
        } else if let Some(ref path) = self.zenoh_config {
            zenoh::Config::from_file(path).map_err(|e| {
                anyhow::anyhow!("failed to load zenoh config from '{}': {e}", path.display())
            })
        } else {
            let json5 = r#"{
                mode: "peer",
                listen: { endpoints: ["tcp/[::]:7447"] }
            }"#;
            zenoh::Config::from_json5(json5)
                .map_err(|e| anyhow::anyhow!("failed to build zenoh peer config: {e}"))
        }
    }

    fn build_tls_json5(&self, endpoint: &str) -> anyhow::Result<String> {
        let is_tls = endpoint.starts_with("tls/") || endpoint.starts_with("quic/");

        if self.tls_ca.is_none() && self.tls_cert.is_none() {
            if is_tls {
                anyhow::bail!(
                    "TLS endpoint '{endpoint}' requires --tls-ca (and optionally --tls-cert + --tls-key for mTLS)"
                );
            }
            return Ok(String::new());
        }

        let ca = self
            .tls_ca
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("--tls-ca is required for TLS endpoints"))?;

        if !ca.exists() {
            anyhow::bail!("TLS CA certificate not found: {}", ca.display());
        }

        let mut tls_fields = format!(
            r#"root_ca_certificate: "{}""#,
            ca.display().to_string().replace('\\', "\\\\")
        );

        if let (Some(cert), Some(key)) = (&self.tls_cert, &self.tls_key) {
            if !cert.exists() {
                anyhow::bail!("TLS client certificate not found: {}", cert.display());
            }
            if !key.exists() {
                anyhow::bail!("TLS client key not found: {}", key.display());
            }
            tls_fields.push_str(&format!(
                r#",
                        enable_mtls: true,
                        connect_certificate: "{}",
                        connect_private_key: "{}""#,
                cert.display().to_string().replace('\\', "\\\\"),
                key.display().to_string().replace('\\', "\\\\"),
            ));
        }

        Ok(format!(
            r#",
                    transport: {{ link: {{ tls: {{ {tls_fields} }} }} }}"#
        ))
    }
}

/// Open persistent DuckDB, auto-importing from Parquet on first run.
pub(crate) fn open_duck(data_dir: &std::path::Path) -> anyhow::Result<DuckStore> {
    let db_path = data_dir.join("fractalaw.duckdb");
    let store = DuckStore::open_persistent(&db_path)?;
    if !store.has_tables() {
        eprintln!(
            "First run — importing Parquet into {}...",
            db_path.display()
        );
        store.load_all(data_dir)?;
    }
    Ok(store)
}

/// Open the provision store (PgStore if --pg is set, otherwise LanceDB not available).
async fn open_provision_store(
    pg_url: Option<&str>,
) -> anyhow::Result<Box<dyn fractalaw_store::ProvisionStore>> {
    if let Some(url) = pg_url {
        let store = fractalaw_store::PgStore::connect(url)
            .await
            .context("connecting to PostgreSQL")?;
        Ok(Box::new(store))
    } else {
        anyhow::bail!(
            "fractalaw-sync requires --pg for provision store access.\n\
             LanceDB is not available in the sync binary."
        );
    }
}

/// Extract a string value from an Arrow array, handling both Utf8 and LargeUtf8.
pub(crate) fn get_string_value(col: &dyn arrow::array::Array, i: usize) -> Option<String> {
    use arrow::array::{Array, LargeStringArray, StringArray};
    if let Some(arr) = col.as_any().downcast_ref::<StringArray>()
        && !arr.is_null(i)
    {
        return Some(arr.value(i).to_string());
    } else if let Some(arr) = col.as_any().downcast_ref::<LargeStringArray>()
        && !arr.is_null(i)
    {
        return Some(arr.value(i).to_string());
    }
    None
}

/// Query DuckDB for all law names belonging to a given family.
pub(crate) fn laws_in_family(store: &DuckStore, family: &str) -> anyhow::Result<Vec<String>> {
    use arrow::array::{Array, StringArray};
    let sql = format!(
        "SELECT name FROM legislation WHERE family = '{}' ORDER BY name",
        family.replace('\'', "''")
    );
    let batches = store.query_arrow(&sql)?;
    let mut names = Vec::new();
    for batch in &batches {
        if let Some(col) = batch.column_by_name("name")
            && let Some(arr) = col.as_any().downcast_ref::<StringArray>()
        {
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    names.push(arr.value(i).to_string());
                }
            }
        }
    }
    Ok(names)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let pg_url = cli.pg.clone();

    let data_dir = cli
        .data_dir
        .canonicalize()
        .with_context(|| format!("data directory '{}' not found", cli.data_dir.display()))?;

    match cli.command {
        Command::Pull { url } => sync::cmd_sync_pull(&data_dir, &url).await,
        Command::Push { url } => sync::cmd_sync_push(&data_dir, &url).await,
        Command::Publish {
            zenoh,
            laws,
            family,
            all,
            changed,
            provisions,
            pending,
        } => {
            if provisions {
                sync::cmd_sync_publish_provisions(
                    &data_dir, &zenoh, laws, family, all, changed, pending,
                    pg_url.as_deref(),
                )
                .await
            } else {
                sync::cmd_sync_publish(&data_dir, &zenoh, laws, family, all, changed).await
            }
        }
        Command::PullLat {
            zenoh,
            laws,
            timeout,
        } => {
            let law_names: Vec<String> =
                laws.split(',').map(|s| s.trim().to_string()).collect();
            sync::cmd_sync_pull_lat(&data_dir, &zenoh, &law_names, timeout, pg_url.as_deref()).await
        }
        Command::Watch { zenoh, timeout } => {
            sync::cmd_sync_watch(&data_dir, &zenoh, timeout, pg_url.as_deref()).await
        }
        Command::Crdt { action } => match action {
            CrdtAction::Status { zenoh } => sync::cmd_crdt_status(&data_dir, &zenoh).await,
            CrdtAction::Create { doc_id, zenoh } => {
                sync::cmd_crdt_create(&data_dir, &zenoh, &doc_id).await
            }
            CrdtAction::Inspect { doc_id, zenoh } => {
                sync::cmd_crdt_inspect(&data_dir, &zenoh, &doc_id).await
            }
            CrdtAction::Save { zenoh } => sync::cmd_crdt_save(&data_dir, &zenoh).await,
        },
    }
}
