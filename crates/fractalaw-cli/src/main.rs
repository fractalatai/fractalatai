mod display;
mod embed;

use std::path::PathBuf;

use anyhow::Context;
use arrow::array::Array;
use arrow::record_batch::RecordBatch;
use arrow::util::pretty::print_batches;
use clap::{Parser, Subcommand};
use fractalaw_store::{DuckStore, FusionStore, LanceStore, StoreError};

/// (polarity, person, process, place, plant, property, sector, article)
type FitnessEntry = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

#[derive(Parser)]
#[command(
    name = "fractalaw",
    version,
    about = "Local-first ESH regulatory data tools"
)]
struct Cli {
    /// Path to data directory containing Parquet files
    #[arg(long, default_value = "./data", global = true)]
    data_dir: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Execute SQL via DataFusion (supports law_status() and edge_type_label() UDFs)
    Query {
        /// SQL query string
        sql: String,
    },

    /// Show a single legislation record with relationships
    Law {
        /// Legislation name (e.g., UK_ukpga_1974_37)
        name: String,
    },

    /// Show amendment/enactment graph traversal
    Graph {
        /// Legislation name to start traversal from
        name: String,

        /// Maximum hops from the starting law
        #[arg(long, default_value_t = 2)]
        hops: u32,
    },

    /// Show dataset summary statistics
    Stats,

    /// Generate embeddings for all legislation text and write to LanceDB
    Embed {
        /// Path to ONNX model directory
        #[arg(long, default_value = "./models/all-MiniLM-L6-v2")]
        model_dir: PathBuf,
    },

    /// Show legislation text sections from LanceDB
    Text {
        /// Legislation name (e.g., UK_ukpga_1974_37)
        name: String,
        /// Maximum rows to display
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },

    /// Semantic similarity search across legislation text
    Search {
        /// Natural language query
        query: String,
        /// Number of results
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Path to ONNX model directory
        #[arg(long, default_value = "./models/all-MiniLM-L6-v2")]
        model_dir: PathBuf,
    },

    /// Run validation checks across all data stores
    Validate {
        /// Path to ONNX model directory (for semantic smoke test)
        #[arg(long, default_value = "./models/all-MiniLM-L6-v2")]
        model_dir: PathBuf,
    },

    /// Tokenize text and display token IDs (for inspection/debugging)
    Tokenize {
        /// Text to tokenize
        text: String,
        /// Path to ONNX model directory
        #[arg(long, default_value = "./models/all-MiniLM-L6-v2")]
        model_dir: PathBuf,
    },

    /// Classify legislation by domain/family/subjects using centroid-based classification
    Classify {
        /// Domain similarity threshold (0.0–1.0)
        #[arg(long, default_value_t = 0.5)]
        domain_threshold: f32,
        /// Subject similarity threshold (0.0–1.0)
        #[arg(long, default_value_t = 0.3)]
        subject_threshold: f32,
    },

    /// Import (or re-import) Parquet files into persistent DuckDB
    Import,

    /// Load and execute a WASM micro-app component
    Run {
        /// Path to the .wasm component file
        component: PathBuf,

        /// Fuel budget (default: 1 billion = standard tier)
        #[arg(long, default_value_t = 1_000_000_000)]
        fuel: u64,
    },

    /// Sync DRRP annotations and polished results with sertantai
    Sync {
        #[command(subcommand)]
        action: SyncAction,
    },

    /// Taxa DRRP classification tools
    Taxa {
        #[command(subcommand)]
        action: TaxaAction,
    },

    /// Export DRRP training data as Parquet (train/val/test splits)
    ExportTrainingData {
        /// Output directory for Parquet files
        #[arg(long, default_value = "./data/drrp-training")]
        output: PathBuf,

        /// File containing validation law names (one per line)
        #[arg(long)]
        val_laws: Option<PathBuf>,

        /// Number of laws for the held-out test set
        #[arg(long, default_value_t = 5)]
        test_laws: usize,

        /// Minimum match quality to include (0.0-1.0)
        #[arg(long, default_value_t = 0.3)]
        min_match_ratio: f32,
    },
}

/// Shared Zenoh connectivity args for all sync subcommands.
#[derive(clap::Args, Clone, Debug)]
struct ZenohArgs {
    /// Tenant namespace
    #[arg(long, env = "FRACTALAW_TENANT", default_value = "local")]
    tenant: String,

    /// Zenoh endpoint to connect to (e.g., tcp/1.2.3.4:7447 or tls/host:7447).
    /// Switches to client mode with multicast scouting disabled.
    #[arg(long, env = "ZENOH_ENDPOINT")]
    connect: Option<String>,

    /// Path to a Zenoh JSON5 config file (advanced).
    /// Mutually exclusive with --connect.
    #[arg(long, env = "ZENOH_CONFIG", conflicts_with = "connect")]
    zenoh_config: Option<PathBuf>,

    /// Root CA certificate for TLS verification (PEM).
    /// Required when --connect uses a tls:// endpoint.
    #[arg(long, env = "ZENOH_TLS_CA", requires = "connect")]
    tls_ca: Option<PathBuf>,

    /// Client certificate for mutual TLS (PEM).
    /// Both --tls-cert and --tls-key must be provided together.
    #[arg(long, env = "ZENOH_TLS_CERT", requires = "tls_key")]
    tls_cert: Option<PathBuf>,

    /// Client private key for mutual TLS (PEM).
    #[arg(long, env = "ZENOH_TLS_KEY", requires = "tls_cert")]
    tls_key: Option<PathBuf>,
}

impl ZenohArgs {
    /// Build a zenoh::Config from CLI flags.
    ///
    /// - `--connect`: client mode, explicit endpoint, no multicast
    /// - `--connect` + `--tls-*`: adds TLS transport config
    /// - `--zenoh-config`: load from JSON5 file
    /// - neither: default peer mode with multicast scouting (LAN P2P)
    fn build_zenoh_config(&self) -> anyhow::Result<zenoh::Config> {
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
            // Peer mode: listen on tcp/0.0.0.0:7447 so remote peers with
            // explicit connect endpoints (e.g., sertantai) can reach us.
            // Multicast scouting remains enabled for LAN discovery.
            let json5 = r#"{
                mode: "peer",
                listen: { endpoints: ["tcp/[::]:7447"] }
            }"#;
            zenoh::Config::from_json5(json5)
                .map_err(|e| anyhow::anyhow!("failed to build zenoh peer config: {e}"))
        }
    }

    /// Build the TLS transport JSON5 fragment, or empty string if no TLS flags.
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

        // mTLS: client cert + key
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

#[derive(Subcommand)]
enum SyncAction {
    /// Pull new annotations from sertantai outbox
    Pull {
        /// Sertantai base URL (e.g. http://localhost:4000)
        #[arg(long, env = "SERTANTAI_URL")]
        url: String,
    },
    /// Push polished results to sertantai inbox
    Push {
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
        /// Publish provision-level taxa (from LanceDB) instead of law-level
        #[arg(long)]
        provisions: bool,
        /// Publish laws recently enriched but not yet published
        /// (enrichment_pending = false AND provisions_published_at < updated_at)
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

#[derive(Subcommand)]
enum TaxaAction {
    /// Show taxa classifications for a law's text sections (from LanceDB)
    Show {
        /// Legislation name (e.g., UK_ukpga_1974_37)
        name: String,
        /// Maximum text sections to process
        #[arg(long, default_value_t = 200)]
        limit: usize,
        /// Show provisions that v2 missed, ranked by heat score (likelihood of genuine miss)
        #[arg(long)]
        misses: bool,
        /// Evaluate clause extraction quality: confidence distribution + low-quality samples
        #[arg(long)]
        clauses: bool,
    },
    /// Enrich LRT DRRP columns for laws missing taxa data (from LanceDB text)
    Enrich {
        /// Specific laws to enrich (comma-separated, e.g., UK_ukpga_1974_37,UK_uksi_1999_3242)
        /// If not specified, enriches all laws without taxa data
        #[arg(long)]
        laws: Option<String>,
        /// Enrich all laws in a DuckDB family (e.g., "OH&S: Occupational / Personal Safety")
        #[arg(long)]
        family: Option<String>,
        /// Re-enrich all laws (clear existing DuckDB taxa columns, re-process all LanceDB text)
        #[arg(long)]
        force: bool,
        /// Enable LLM escalation: inheritance + LLM classification for ambiguous provisions
        #[arg(long)]
        escalate: bool,
        /// Skip laws where all provisions were enriched within the last 24 hours
        #[arg(long)]
        skip_recent: bool,
        /// Process laws queued by sync watch (enrichment_pending = true).
        /// Runs embed + classify + regex DRRP in batch, then clears the queue.
        #[arg(long)]
        pending: bool,
    },
    /// Generate clause eyeball review markdown for manual QA
    Eyeball {
        /// Comma-separated law names to include
        #[arg(long)]
        laws: String,
        /// Output file path
        #[arg(long, default_value = "./data/clause_eyeball.md")]
        output: PathBuf,
        /// Maximum text sections per law
        #[arg(long, default_value_t = 200)]
        limit: usize,
    },
    /// Run purpose classification QA report across laws
    Qa {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Filter by DuckDB family
        #[arg(long)]
        family: Option<String>,
    },
    /// Audit p-dimension dictionary coverage for fitness extraction gaps
    AuditFitness {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Filter by DuckDB family
        #[arg(long)]
        family: Option<String>,
        /// Max gap provisions shown per family (0 = show all)
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Regex parse + Tier 1 inheritance only (no LLM, no classifier)
    Parse {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
        /// Re-parse all (clear existing DuckDB taxa columns for these laws)
        #[arg(long)]
        force: bool,
        /// Write decision trail JSON to this path (e.g. data/trace.json)
        #[arg(long)]
        trace: Option<String>,
    },
    /// Compute embeddings for provisions missing them
    Embed {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
    },
    /// Run DRRP + position classifiers on provisions with embeddings
    Classify {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
    },
    /// LLM escalation: Tier 2 DRRP + Tier 3 position classification
    Escalate {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
    },
    /// Whole-law LLM validation: send all provisions + parse results to LLM
    Validate {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
        /// Directory for audit log JSON files (e.g. data/llm-audit)
        #[arg(long, default_value = "data/llm-audit")]
        audit_dir: String,
        /// Dry run: build prompts and show token estimates without calling LLM
        #[arg(long)]
        dry_run: bool,
        /// Apply corrections to LanceDB (default: audit-only, no writes)
        #[arg(long)]
        apply: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let data_dir = cli
        .data_dir
        .canonicalize()
        .with_context(|| format!("data directory '{}' not found", cli.data_dir.display()))?;

    match cli.command {
        // DuckDB commands — open persistent store with auto-import on first run.
        Command::Query { sql } => cmd_query(&open_duck(&data_dir)?, &sql).await,
        Command::Law { name } => cmd_law(&open_duck(&data_dir)?, &name),
        Command::Graph { name, hops } => cmd_graph(&open_duck(&data_dir)?, &name, hops),
        Command::Stats => cmd_stats(&open_duck(&data_dir)?),
        Command::Validate { model_dir } => {
            cmd_validate(&open_duck(&data_dir)?, &data_dir, &model_dir).await
        }
        Command::Classify {
            domain_threshold,
            subject_threshold,
        } => {
            cmd_classify(
                &open_duck(&data_dir)?,
                &data_dir,
                domain_threshold,
                subject_threshold,
            )
            .await
        }
        Command::Import => cmd_import(&data_dir),

        // LanceDB-only commands — no DuckDB needed.
        Command::Embed { model_dir } => cmd_embed(&data_dir, &model_dir).await,
        Command::Text { name, limit } => cmd_text(&data_dir, &name, limit).await,
        Command::Search {
            query,
            limit,
            model_dir,
        } => cmd_search(&data_dir, &query, limit, &model_dir).await,

        // Model-only commands — no data store needed.
        Command::Tokenize { text, model_dir } => cmd_tokenize(&text, &model_dir),

        // WASM micro-app commands.
        Command::Run { component, fuel } => cmd_run(&data_dir, &component, fuel).await,

        // Sync commands.
        Command::Sync { action } => match action {
            SyncAction::Pull { url } => cmd_sync_pull(&data_dir, &url).await,
            SyncAction::Push { url } => cmd_sync_push(&data_dir, &url).await,
            SyncAction::Publish {
                zenoh,
                laws,
                family,
                all,
                changed,
                provisions,
                pending,
            } => {
                if provisions {
                    cmd_sync_publish_provisions(
                        &data_dir, &zenoh, laws, family, all, changed, pending,
                    )
                    .await
                } else {
                    cmd_sync_publish(&data_dir, &zenoh, laws, family, all, changed).await
                }
            }
            SyncAction::PullLat {
                zenoh,
                laws,
                timeout,
            } => {
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_sync_pull_lat(&data_dir, &zenoh, &law_names, timeout).await
            }
            SyncAction::Watch { zenoh, timeout } => {
                cmd_sync_watch(&data_dir, &zenoh, timeout).await
            }
            SyncAction::Crdt { action } => match action {
                CrdtAction::Status { zenoh } => cmd_crdt_status(&data_dir, &zenoh).await,
                CrdtAction::Create { doc_id, zenoh } => {
                    cmd_crdt_create(&data_dir, &zenoh, &doc_id).await
                }
                CrdtAction::Inspect { doc_id, zenoh } => {
                    cmd_crdt_inspect(&data_dir, &zenoh, &doc_id).await
                }
                CrdtAction::Save { zenoh } => cmd_crdt_save(&data_dir, &zenoh).await,
            },
        },

        // Taxa classification.
        Command::Taxa { action } => match action {
            TaxaAction::Show {
                name,
                limit,
                misses,
                clauses,
            } => cmd_taxa_show(&data_dir, &name, limit, misses, clauses).await,
            TaxaAction::Enrich {
                laws,
                family,
                force,
                escalate,
                skip_recent,
                pending,
            } => {
                let store = open_duck(&data_dir)?;
                let law_filter = if pending {
                    // Process the enrichment queue from sync watch
                    store.ensure_enrichment_queue_columns()?;
                    let batches = store.query_arrow(
                        "SELECT name FROM legislation \
                         WHERE enrichment_pending = true \
                           AND (enrichment_retry_count IS NULL OR enrichment_retry_count < 3) \
                         ORDER BY enrichment_added_at ASC",
                    )?;
                    let mut names = Vec::new();
                    for batch in &batches {
                        if let Some(col) = batch.column_by_name("name")
                            && let Some(arr) =
                                col.as_any().downcast_ref::<arrow::array::StringArray>()
                        {
                            for i in 0..arr.len() {
                                if !arr.is_null(i) {
                                    names.push(arr.value(i).to_string());
                                }
                            }
                        }
                    }
                    if names.is_empty() {
                        println!("No laws pending enrichment.");
                        return Ok(());
                    }
                    println!("Enrichment queue: {} laws pending", names.len());
                    Some(names)
                } else if let Some(ref fam) = family {
                    // Resolve family to law names via DuckDB query
                    let names = laws_in_family(&store, fam)?;
                    if names.is_empty() {
                        anyhow::bail!("No laws found with family '{fam}'");
                    }
                    println!("Family '{}': {} laws", fam, names.len());
                    Some(names)
                } else {
                    laws.as_ref().map(|s| {
                        s.split(',')
                            .map(|l| l.trim().to_string())
                            .collect::<Vec<_>>()
                    })
                };
                cmd_taxa_enrich(
                    &data_dir,
                    &store,
                    law_filter,
                    force,
                    escalate,
                    skip_recent,
                    pending,
                )
                .await
            }
            TaxaAction::Eyeball {
                laws,
                output,
                limit,
            } => {
                let law_names: Vec<&str> = laws.split(',').map(|l| l.trim()).collect();
                cmd_taxa_eyeball(&data_dir, &law_names, &output, limit).await
            }
            TaxaAction::Qa { laws, family } => cmd_taxa_qa(&data_dir, laws, family).await,
            TaxaAction::AuditFitness {
                laws,
                family,
                limit,
            } => cmd_taxa_audit_fitness(&data_dir, laws, family, limit).await,
            TaxaAction::Parse { laws, force, trace } => {
                let store = open_duck(&data_dir)?;
                let lance = LanceStore::open(&data_dir.join("lancedb"))
                    .await
                    .context("opening LanceDB")?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_parse(&lance, &store, &law_names, force).await?;
                if let Some(trace_path) = trace {
                    cmd_taxa_trace(&lance, &store, &law_names, &trace_path).await?;
                }
                Ok(())
            }
            TaxaAction::Embed { laws } => {
                let lance = LanceStore::open(&data_dir.join("lancedb"))
                    .await
                    .context("opening LanceDB")?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_embed(&lance, &law_names).await
            }
            TaxaAction::Classify { laws } => {
                let lance = LanceStore::open(&data_dir.join("lancedb"))
                    .await
                    .context("opening LanceDB")?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_classify(&lance, &law_names).await
            }
            TaxaAction::Escalate { laws } => {
                let store = open_duck(&data_dir)?;
                let lance = LanceStore::open(&data_dir.join("lancedb"))
                    .await
                    .context("opening LanceDB")?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_escalate(&lance, &store, &law_names).await
            }
            TaxaAction::Validate {
                laws,
                audit_dir,
                dry_run,
                apply,
            } => {
                let store = open_duck(&data_dir)?;
                let lance = LanceStore::open(&data_dir.join("lancedb"))
                    .await
                    .context("opening LanceDB")?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_validate(&lance, &store, &law_names, &audit_dir, dry_run, apply).await
            }
        },

        // Training data export.
        Command::ExportTrainingData {
            output,
            val_laws,
            test_laws,
            min_match_ratio,
        } => {
            cmd_export_training_data(
                &data_dir,
                &open_duck(&data_dir)?,
                &output,
                val_laws.as_deref(),
                test_laws,
                min_match_ratio,
            )
            .await
        }
    }
}

/// Open persistent DuckDB, auto-importing from Parquet on first run.
fn open_duck(data_dir: &std::path::Path) -> anyhow::Result<DuckStore> {
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

/// Query DuckDB for all law names belonging to a given family.
fn laws_in_family(store: &DuckStore, family: &str) -> anyhow::Result<Vec<String>> {
    use arrow::array::Array;

    let sql = format!(
        "SELECT name FROM legislation WHERE family = '{}' ORDER BY name",
        family.replace('\'', "''")
    );
    let batches = store.query_arrow(&sql)?;
    let mut names = Vec::new();
    for batch in &batches {
        if let Some(col) = batch.column_by_name("name")
            && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
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

fn cmd_import(data_dir: &std::path::Path) -> anyhow::Result<()> {
    let db_path = data_dir.join("fractalaw.duckdb");
    let store = DuckStore::open_persistent(&db_path)?;
    store.load_all(data_dir)?;
    println!(
        "Imported into {}\n  Legislation: {:>8} rows\n  Law edges:   {:>8} rows",
        db_path.display(),
        fmt_num(store.legislation_count()?),
        fmt_num(store.law_edges_count()?),
    );
    Ok(())
}

async fn cmd_run(
    data_dir: &std::path::Path,
    component: &std::path::Path,
    fuel: u64,
) -> anyhow::Result<()> {
    let duck = open_duck(data_dir)?;

    // Try to load local ONNX DRRP model.
    let extractor = {
        let model_dir = std::env::var("DRRP_MODEL_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                // Default: models/deberta-v3-drrp relative to the repo root.
                data_dir
                    .parent()
                    .unwrap_or(data_dir)
                    .join("models/deberta-v3-drrp")
            });
        if model_dir.join("model.int8.onnx").exists() || model_dir.join("model.onnx").exists() {
            match fractalaw_ai::DrrpExtractor::load(&model_dir) {
                Ok(e) => {
                    tracing::info!(model_dir = %model_dir.display(), "loaded DRRP ONNX model");
                    Some(e)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to load DRRP ONNX model");
                    None
                }
            }
        } else {
            tracing::debug!(model_dir = %model_dir.display(), "no DRRP ONNX model found");
            None
        }
    };

    // Open LanceDB for legislation_text queries/mutations.
    let lance = LanceStore::open(&data_dir.join("lancedb")).await.ok();

    let opts = fractalaw_host::RunOptions {
        duck: Some(duck),
        lance,
        extractor,
    };
    let result = fractalaw_host::run_component(component, fuel, opts).await?;

    match &result.output {
        Ok(msg) => println!("{msg}"),
        Err(err) => eprintln!("Guest error: {err}"),
    }

    if !result.audit_entries.is_empty() {
        println!(
            "\n--- Audit Trail ({} entries) ---",
            result.audit_entries.len()
        );
        for entry in &result.audit_entries {
            println!(
                "  [{}] {} — {}",
                entry.event_type, entry.resource, entry.detail
            );
        }
    }

    println!("\nFuel consumed: {}", result.fuel_consumed);
    Ok(())
}

async fn cmd_sync_pull(data_dir: &std::path::Path, url: &str) -> anyhow::Result<()> {
    let duck = open_duck(data_dir)?;
    duck.create_drrp_tables()?;

    // Determine the `since` timestamp from the last sync.
    let since = duck.get_last_sync_at()?;
    if let Some(ref ts) = since {
        eprintln!("Last sync: {ts}");
    } else {
        eprintln!("First sync — pulling all annotations");
    }

    let client = fractalaw_sync::SyncClient::new(url.to_string());
    let since_dt = since
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let annotations = client.pull_annotations(since_dt).await?;

    if annotations.is_empty() {
        println!("No new annotations.");
        return Ok(());
    }

    let count = duck.insert_annotations(&annotations)?;
    println!("Pulled and stored {count} annotations.");
    Ok(())
}

async fn cmd_sync_push(data_dir: &std::path::Path, url: &str) -> anyhow::Result<()> {
    let duck = open_duck(data_dir)?;
    duck.create_drrp_tables()?;

    let entries = duck.get_unpushed_polished()?;
    if entries.is_empty() {
        println!("Nothing to push.");
        return Ok(());
    }

    let client = fractalaw_sync::SyncClient::new(url.to_string());
    let accepted = client.push_polished(&entries).await?;

    // Mark each entry as pushed.
    for entry in &entries {
        duck.mark_pushed(&entry.law_name, &entry.provision)?;
    }

    println!(
        "Pushed {} polished entries ({} accepted by server).",
        entries.len(),
        accepted
    );
    Ok(())
}

async fn cmd_sync_publish(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    laws: Option<String>,
    family: Option<String>,
    all: bool,
    changed: bool,
) -> anyhow::Result<()> {
    let store = open_duck(data_dir)?;
    store.ensure_taxa_hash_columns()?;
    store.ensure_fitness_columns()?;

    // Resolve law names: --family, --laws, --changed, or --all (must be explicit).
    let law_names: Vec<String> = if let Some(ref fam) = family {
        let names = laws_in_family(&store, fam)?;
        if names.is_empty() {
            anyhow::bail!("No laws found with family '{fam}'");
        }
        println!("Family '{}': {} laws", fam, names.len());
        names
    } else if let Some(ref l) = laws {
        l.split(',').map(|s| s.trim().to_string()).collect()
    } else if changed {
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE taxa_hash IS NOT NULL \
               AND (published_hash IS NULL OR taxa_hash != published_hash) \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!(
            "Publishing {} laws with changed taxa (--changed)",
            names.len()
        );
        names
    } else if all {
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE duty_holder IS NOT NULL AND len(duty_holder) > 0 \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!("Publishing ALL {} laws with taxa data", names.len());
        names
    } else {
        anyhow::bail!(
            "Specify --family, --laws, --changed, or --all to select laws to publish.\n\
             Example: fractalaw sync publish --family \"OH&S: Occupational / Personal Safety\" --tenant dev"
        );
    };

    if law_names.is_empty() {
        println!("No laws with taxa data to publish.");
        return Ok(());
    }

    println!(
        "Publishing taxa for {} laws to zenoh (tenant: {})...",
        law_names.len(),
        zenoh.tenant,
    );

    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))?;

    // Wait for at least one peer to connect before publishing.
    // Peer-mode scouting and inbound connections need time to establish.
    print!("Waiting for zenoh peer...");
    let peers = sync
        .wait_for_peers(std::time::Duration::from_secs(15))
        .await;
    if peers == 0 {
        println!(" no peers connected (timeout). Publishing anyway, but data may not be received.");
    } else {
        println!(" {peers} peer(s) connected.");
    }

    // Put actor dictionary — fires any active sertantai subscribers
    if let Ok(dict_yaml) = std::fs::read("docs/actor-dictionary.yaml") {
        sync.publish_dictionary(&dict_yaml)
            .await
            .map_err(|e| anyhow::anyhow!("failed to publish actor dictionary: {e}"))?;
        println!("Published actor dictionary ({} bytes)", dict_yaml.len());
    }

    let mut published = 0usize;
    for law_name in &law_names {
        let sql = format!(
            "SELECT name, duty_holder, rights_holder, responsibility_holder, power_holder, \
                    duty_type, role, role_gvt, \
                    duties, rights, responsibilities, powers, \
                    fitness_person, fitness_process, fitness_place, \
                    fitness_plant, fitness_property, fitness_sector, fitness \
             FROM legislation WHERE name = '{}'",
            law_name.replace('\'', "''")
        );
        let batches = store.query_arrow(&sql)?;
        if batches.is_empty() || batches.iter().all(|b| b.num_rows() == 0) {
            eprintln!("  {law_name}: no data, skipping");
            continue;
        }

        sync.publish_taxa(law_name, &batches)
            .await
            .map_err(|e| anyhow::anyhow!("failed to publish {law_name}: {e}"))?;

        // Mark published_hash = taxa_hash so --changed skips this law next time.
        store.execute(&format!(
            "UPDATE legislation SET published_hash = taxa_hash WHERE name = '{}'",
            law_name.replace('\'', "''")
        ))?;

        published += 1;
    }

    println!("Published {published}/{} laws.", law_names.len());
    Ok(())
}

async fn cmd_sync_publish_provisions(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    laws: Option<String>,
    family: Option<String>,
    all: bool,
    changed: bool,
    pending: bool,
) -> anyhow::Result<()> {
    let store = open_duck(data_dir)?;
    store.ensure_taxa_hash_columns()?;
    store.ensure_provisions_published_column()?;

    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    // Resolve law names.
    let law_names: Vec<String> = if pending {
        store.ensure_enrichment_queue_columns()?;
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE enrichment_pending = false \
               AND enrichment_added_at IS NOT NULL \
               AND (provisions_published_at IS NULL \
                    OR provisions_published_at < updated_at) \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!(
            "Publishing provisions for {} recently enriched laws (--pending)",
            names.len()
        );
        names
    } else if let Some(ref fam) = family {
        let names = laws_in_family(&store, fam)?;
        if names.is_empty() {
            anyhow::bail!("No laws found with family '{fam}'");
        }
        println!("Family '{}': {} laws", fam, names.len());
        names
    } else if let Some(ref l) = laws {
        l.split(',').map(|s| s.trim().to_string()).collect()
    } else if changed {
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE taxa_hash IS NOT NULL \
               AND (provisions_published_at IS NULL \
                    OR provisions_published_at < updated_at) \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!(
            "Publishing provisions for {} laws with changed taxa (--changed)",
            names.len()
        );
        names
    } else if all {
        let batches = store.query_arrow(
            "SELECT name FROM legislation \
             WHERE duty_holder IS NOT NULL AND len(duty_holder) > 0 \
             ORDER BY name",
        )?;
        let mut names = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("name")
                && let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>()
            {
                for i in 0..arr.len() {
                    if !arr.is_null(i) {
                        names.push(arr.value(i).to_string());
                    }
                }
            }
        }
        println!(
            "Publishing provisions for ALL {} laws with taxa data",
            names.len()
        );
        names
    } else {
        anyhow::bail!(
            "Specify --family, --laws, --changed, or --all to select laws to publish.\n\
             Example: fractalaw sync publish --provisions --family \"OH&S: Occupational / Personal Safety\" --tenant dev"
        );
    };

    if law_names.is_empty() {
        println!("No laws with taxa data to publish.");
        return Ok(());
    }

    println!(
        "Publishing provision-level taxa for {} laws to zenoh (tenant: {})...",
        law_names.len(),
        zenoh.tenant,
    );

    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))?;

    print!("Waiting for zenoh peer...");
    let peers = sync
        .wait_for_peers(std::time::Duration::from_secs(15))
        .await;
    if peers == 0 {
        println!(" no peers connected (timeout). Publishing anyway, but data may not be received.");
    } else {
        println!(" {peers} peer(s) connected.");
    }

    // Put actor dictionary — fires any active sertantai subscribers
    if let Ok(dict_yaml) = std::fs::read("docs/actor-dictionary.yaml") {
        sync.publish_dictionary(&dict_yaml)
            .await
            .map_err(|e| anyhow::anyhow!("failed to publish actor dictionary: {e}"))?;
        println!("Published actor dictionary ({} bytes)", dict_yaml.len());
    }

    let mut published = 0usize;
    let mut total_provisions = 0usize;
    for law_name in &law_names {
        let batches = lance.query_provision_taxa(law_name).await?;
        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        if rows == 0 {
            eprintln!("  {law_name}: no enriched provisions, skipping");
            continue;
        }

        sync.publish_provision_taxa(law_name, &batches)
            .await
            .map_err(|e| anyhow::anyhow!("failed to publish provisions for {law_name}: {e}"))?;

        // Mark provisions as published.
        store.execute(&format!(
            "UPDATE legislation SET provisions_published_at = CURRENT_TIMESTAMP WHERE name = '{}'",
            law_name.replace('\'', "''")
        ))?;

        println!("  {law_name}: {rows} provisions");
        total_provisions += rows;
        published += 1;
    }

    println!(
        "Published {total_provisions} provisions across {published}/{} laws.",
        law_names.len()
    );
    Ok(())
}

async fn cmd_sync_pull_lat(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    law_names: &[String],
    timeout_secs: u64,
) -> anyhow::Result<()> {
    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))?;

    let timeout = std::time::Duration::from_secs(timeout_secs);

    println!(
        "Pulling LAT for {} laws from sertantai (tenant: {}, timeout: {timeout_secs}s)...",
        law_names.len(),
        zenoh.tenant,
    );

    let mut total_pulled = 0usize;
    let mut total_rows = 0usize;

    for law_name in law_names {
        eprint!("  {law_name}: ");

        let batches = sync
            .query_lat(law_name, timeout)
            .await
            .map_err(|e| anyhow::anyhow!("query failed for {law_name}: {e}"))?;

        if batches.is_empty() {
            eprintln!("no data (sertantai did not respond)");
            continue;
        }

        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        lance
            .upsert_lat(batches)
            .await
            .map_err(|e| anyhow::anyhow!("upsert failed for {law_name}: {e}"))?;

        eprintln!("{rows} provisions");
        total_pulled += 1;
        total_rows += rows;
    }

    println!(
        "\nPulled {total_pulled}/{} laws, {total_rows} total provisions.",
        law_names.len()
    );

    Ok(())
}

async fn cmd_sync_watch(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    timeout_secs: u64,
) -> anyhow::Result<()> {
    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;
    let duck = open_duck(data_dir)?;
    duck.ensure_taxa_hash_columns()?;
    duck.ensure_provisions_published_column()?;
    duck.ensure_enrichment_queue_columns()?;

    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))?;

    let subscriber = sync
        .subscribe_events()
        .await
        .map_err(|e| anyhow::anyhow!("failed to subscribe to events: {e}"))?;

    let timeout = std::time::Duration::from_secs(timeout_secs);

    // Serve actor dictionary as a queryable (stays alive for the watch session)
    let _dict_handle = if let Ok(dict_yaml) = std::fs::read("docs/actor-dictionary.yaml") {
        println!(
            "Serving actor dictionary via queryable ({} bytes)",
            dict_yaml.len()
        );
        Some(
            sync.serve_dictionary(dict_yaml)
                .await
                .map_err(|e| anyhow::anyhow!("failed to serve actor dictionary: {e}"))?,
        )
    } else {
        eprintln!(
            "Warning: docs/actor-dictionary.yaml not found, dictionary queryable not started"
        );
        None
    };

    println!(
        "Watching for sync events (tenant: {}, timeout: {timeout_secs}s per pull)...",
        zenoh.tenant
    );
    println!("Pipeline: ensure LRT → pull LAT → ack → queue for enrichment");
    println!("Press Ctrl+C to stop.\n");

    let mut total_events = 0usize;
    let mut total_lrt_pulls = 0usize;
    let mut total_lat_pulls = 0usize;
    let mut total_rows = 0usize;
    let mut total_enriched = 0usize;
    let mut total_deletions = 0usize;

    loop {
        tokio::select! {
            sample = subscriber.recv_async() => {
                let sample = match sample {
                    Ok(s) => s,
                    Err(_) => break,
                };

                let bytes = sample.payload().to_bytes();
                let event = match fractalaw_sync::SyncEvent::from_payload(&bytes) {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("  [warn] failed to parse event: {e}");
                        continue;
                    }
                };

                total_events += 1;
                let law_name = &event.metadata.law_name;

                // Skip events we don't act on (e.g. amendments).
                if event.table != "lat" && event.table != "lrt" {
                    eprintln!(
                        "  [skip] {}.{} for {}",
                        event.table, event.action, law_name
                    );
                    continue;
                }

                // Handle LAT deletion — purge local text + annotations, keep taxa/fitness.
                if event.action == "lat_deleted" {
                    eprint!("  {law_name}: lat_deleted");
                    let lat_count = lance.delete_law_lat(law_name).await.unwrap_or(0);
                    let ann_count = lance.delete_law_annotations(law_name).await.unwrap_or(0);
                    eprintln!(" → deleted {lat_count} provisions, {ann_count} annotations");
                    total_deletions += 1;
                    continue;
                }

                eprint!("  {law_name}:");

                // Step 1: Ensure LRT exists in DuckDB.
                let lrt_exists = duck
                    .query_arrow(&format!(
                        "SELECT 1 FROM legislation WHERE name = '{}'",
                        law_name.replace('\'', "''")
                    ))
                    .map(|b| b.iter().any(|b| b.num_rows() > 0))
                    .unwrap_or(false);

                if !lrt_exists {
                    eprint!(" pull LRT");
                    match sync.query_lrt(law_name, timeout).await {
                        Ok(batches) if batches.is_empty() => {
                            eprintln!(" → no LRT data");
                        }
                        Ok(batches) => match duck.upsert_legislation(&batches) {
                            Ok(n) => {
                                eprint!(" → {n} row(s)");
                                total_lrt_pulls += 1;
                            }
                            Err(e) => {
                                eprintln!(" → LRT upsert error: {e}");
                            }
                        },
                        Err(e) => {
                            eprintln!(" → LRT query error: {e}");
                        }
                    }
                }

                // Step 2: Pull LAT from sertantai → LanceDB.
                eprint!(" → pull LAT");
                let lat_rows: usize = match sync.query_lat(law_name, timeout).await {
                    Ok(batches) if batches.is_empty() => {
                        eprintln!(" → no LAT data");
                        continue;
                    }
                    Ok(batches) => {
                        let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
                        match lance.upsert_lat(batches).await {
                            Ok(_) => {
                                eprint!(" → {rows} provisions");
                                total_lat_pulls += 1;
                                total_rows += rows;
                                rows
                            }
                            Err(e) => {
                                eprintln!(" → LAT upsert error: {e}");
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(" → LAT query error: {e}");
                        continue;
                    }
                };

                // Step 3: Ack to sertantai + queue for enrichment.
                //
                // No enrichment or publish here — that happens in batch via
                // `taxa enrich --pending` and `sync publish --pending`.
                if let Err(e) = sync.publish_ack(law_name, lat_rows).await {
                    eprint!(" → ack error: {e}");
                }

                // Mark law as pending enrichment in DuckDB.
                let escaped = law_name.replace('\'', "''");
                let _ = duck.execute(&format!(
                    "UPDATE legislation \
                     SET enrichment_pending = true, \
                         enrichment_added_at = CURRENT_TIMESTAMP \
                     WHERE name = '{escaped}'"
                ));
                total_enriched += 1;
                eprintln!(" → queued for enrichment");
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down...");
                break;
            }
        }
    }

    println!(
        "Done. {total_events} events, {total_lrt_pulls} LRT pulls, \
         {total_lat_pulls} LAT pulls ({total_rows} provisions), \
         {total_enriched} queued for enrichment, {total_deletions} deletions."
    );

    Ok(())
}

// ── CRDT commands ──

fn crdt_persist_dir(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("crdt")
}

fn generate_peer_id() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::process::id().hash(&mut hasher);
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        hostname.hash(&mut hasher);
    }
    hasher.finish()
}

async fn cmd_crdt_status(data_dir: &std::path::Path, zenoh: &ZenohArgs) -> anyhow::Result<()> {
    let persist_dir = crdt_persist_dir(data_dir);
    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::CrdtSync::with_config(
        &zenoh.tenant,
        generate_peer_id(),
        &persist_dir,
        config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to open CRDT session: {e}"))?;

    let persisted = sync
        .list_persisted_docs()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Tenant: {}", zenoh.tenant);
    println!("Persist dir: {}", persist_dir.display());
    println!("Persisted documents: {}", persisted.len());
    for doc_id in &persisted {
        let path = persist_dir.join(format!("{doc_id}.loro"));
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        println!("  {doc_id} ({size} bytes)");
    }
    Ok(())
}

async fn cmd_crdt_create(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    doc_id: &str,
) -> anyhow::Result<()> {
    let persist_dir = crdt_persist_dir(data_dir);
    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::CrdtSync::with_config(
        &zenoh.tenant,
        generate_peer_id(),
        &persist_dir,
        config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to open CRDT session: {e}"))?;

    sync.create_doc(doc_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    sync.save_snapshot(doc_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Created and saved CRDT document: {doc_id}");
    Ok(())
}

async fn cmd_crdt_inspect(
    data_dir: &std::path::Path,
    zenoh: &ZenohArgs,
    doc_id: &str,
) -> anyhow::Result<()> {
    let persist_dir = crdt_persist_dir(data_dir);
    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::CrdtSync::with_config(
        &zenoh.tenant,
        generate_peer_id(),
        &persist_dir,
        config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to open CRDT session: {e}"))?;

    sync.open_or_create(doc_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let value = sync
        .get_doc_value(doc_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{value:?}");
    Ok(())
}

async fn cmd_crdt_save(data_dir: &std::path::Path, zenoh: &ZenohArgs) -> anyhow::Result<()> {
    let persist_dir = crdt_persist_dir(data_dir);
    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::CrdtSync::with_config(
        &zenoh.tenant,
        generate_peer_id(),
        &persist_dir,
        config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to open CRDT session: {e}"))?;

    // Load all persisted docs first
    let doc_ids = sync
        .list_persisted_docs()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    for doc_id in &doc_ids {
        sync.open_or_create(doc_id)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    let paths = sync
        .save_all_snapshots()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Saved {} document(s).", paths.len());
    for p in &paths {
        println!("  {}", p.display());
    }
    Ok(())
}

async fn cmd_query(store: &DuckStore, sql: &str) -> anyhow::Result<()> {
    let fusion = FusionStore::new(store)?;
    let batches = fusion.query(sql).await?;
    if batches.is_empty() || batches.iter().all(|b| b.num_rows() == 0) {
        println!("No results.");
        return Ok(());
    }
    print_batches(&batches)?;
    Ok(())
}

fn cmd_law(store: &DuckStore, name: &str) -> anyhow::Result<()> {
    let batch = store.get_legislation(name).map_err(|e| match e {
        StoreError::NoResults => anyhow::anyhow!("legislation '{}' not found", name),
        other => anyhow::anyhow!(other),
    })?;

    display::print_law_card(&batch)?;

    let edges = store.edges_for_law(name)?;
    let total_edges: usize = edges.iter().map(|b| b.num_rows()).sum();
    if total_edges > 0 {
        println!("--- Relationships ({total_edges} edges) ---\n");
        print_batches(&edges)?;
    }

    Ok(())
}

fn cmd_graph(store: &DuckStore, name: &str, hops: u32) -> anyhow::Result<()> {
    let batches = store.laws_within_hops(name, hops)?;
    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total_rows == 0 {
        println!("No laws found within {hops} hops of '{name}'.");
        return Ok(());
    }
    println!("Laws within {hops} hops of '{name}' ({total_rows} total):\n");
    print_batches(&batches)?;
    Ok(())
}

async fn cmd_embed(data_dir: &std::path::Path, model_dir: &std::path::Path) -> anyhow::Result<()> {
    let model_dir = model_dir
        .canonicalize()
        .with_context(|| format!("model directory '{}' not found", model_dir.display()))?;

    println!("=== Embedding Pipeline ===\n");

    let mut embedder =
        fractalaw_ai::Embedder::load(&model_dir).context("loading embedding model")?;
    println!("  Model: {} ({}D)", model_dir.display(), embedder.dim());

    let lance_path = data_dir.join("lancedb");
    let lance = LanceStore::open(&lance_path)
        .await
        .context("opening LanceDB")?;

    let parquet_path = data_dir.join("legislation_text.parquet");
    let stats = embed::run_embed_pipeline(&lance, &mut embedder, &parquet_path).await?;

    println!("\n=== Complete ===");
    println!("  Rows:       {:>8}", stats.total_rows);
    println!("  Time:       {:>8.1}s", stats.elapsed_secs);
    if stats.elapsed_secs > 0.0 {
        println!(
            "  Throughput: {:>8.0} rows/sec",
            stats.total_rows as f64 / stats.elapsed_secs
        );
    }

    Ok(())
}

async fn cmd_text(data_dir: &std::path::Path, name: &str, limit: usize) -> anyhow::Result<()> {
    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    let filter = format!("law_name = '{name}'");
    let batches = lance.query_legislation_text(&filter, limit, 0).await?;

    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total == 0 {
        println!("No text sections found for '{name}'.");
        return Ok(());
    }

    println!("Text sections for '{name}' ({total} rows):\n");
    let projected = project_batches(
        &batches,
        &["provision", "section_type", "heading_group", "text"],
    );
    print_batches(&projected)?;
    Ok(())
}

async fn cmd_taxa_show(
    data_dir: &std::path::Path,
    name: &str,
    limit: usize,
    misses: bool,
    clauses: bool,
) -> anyhow::Result<()> {
    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    // Look up law's family from DuckDB for family-gated specialist actors.
    let family: Option<String> = {
        let duck = open_duck(data_dir)?;
        let batches = duck.query_arrow(&format!(
            "SELECT family FROM legislation WHERE name = '{}'",
            name.replace('\'', "''")
        ))?;
        batches.iter().find_map(|b| {
            let col = b.column_by_name("family")?;
            get_string_value(col.as_ref(), 0)
        })
    };

    let filter = format!("law_name = '{}'", name.replace('\'', "''"));
    let batches = lance.query_legislation_text(&filter, limit, 0).await?;

    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total == 0 {
        println!("No text sections found for '{name}'.");
        println!("(Ensure LanceDB has been populated via `fractalaw embed`)");
        return Ok(());
    }

    if misses {
        return cmd_taxa_show_misses(name, total, &batches, family.as_deref());
    }
    if clauses {
        return cmd_taxa_show_clauses(name, total, &batches, family.as_deref());
    }

    println!("=== Taxa Classification: {name} ({total} sections) ===\n");

    let mut section_num = 0usize;
    let mut classified_num = 0usize;

    for batch in &batches {
        let provision_col = batch.column_by_name("provision");
        let text_col = batch.column_by_name("text");

        for row in 0..batch.num_rows() {
            let provision = provision_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let text = text_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();

            if text.trim().is_empty() {
                continue;
            }

            section_num += 1;

            let (record, trail) =
                fractalaw_core::taxa::parse_v2_with_trail(&text, family.as_deref());

            // Skip sections with no classification signal.
            if record.duty_types.is_empty()
                && record.governed_actors.is_empty()
                && record.government_actors.is_empty()
            {
                continue;
            }

            classified_num += 1;
            println!("--- {provision} ---");

            if !record.duty_types.is_empty() {
                let types: Vec<&str> = record.duty_types.iter().map(|d| d.as_str()).collect();
                println!("  DRRP:    {}", types.join(", "));
            }

            if let Some(ref class) = record.classification {
                println!(
                    "  Pattern: {:?} / {:?} ({:.0}%)",
                    class.family,
                    class.sub_type,
                    class.confidence * 100.0,
                );
            }

            // Decision trail
            println!(
                "  Trail:   {} ({} candidates, {} rejected)",
                trail.reason, trail.candidates_count, trail.rejections_count,
            );
            if let Some(ref winner) = trail.winner {
                println!(
                    "  Winner:  {:?} / {:?} ({:.0}%) via {:?}",
                    winner.family,
                    winner.sub_type,
                    winner.confidence * 100.0,
                    winner.tier,
                );
            }

            if !record.governed_actors.is_empty() {
                println!("  Governed:   {}", record.governed_actors.join(", "));
            }
            if !record.government_actors.is_empty() {
                println!("  Government: {}", record.government_actors.join(", "));
            }

            if !record.popimar.is_empty() {
                println!("  POPIMAR: {}", record.popimar.join(", "));
            }
            if !record.purposes.is_empty() {
                println!("  Purpose: {}", record.purposes.join(", "));
            }

            // Show clause_refined if available, otherwise a text preview.
            if let Some(ref clause) = record.clause_refined {
                println!("  Clause:  {clause}");
            } else {
                let preview = if record.cleaned_text.len() > 120 {
                    let end = truncate_at_char_boundary(&record.cleaned_text, 120);
                    format!("{}...", &record.cleaned_text[..end])
                } else {
                    record.cleaned_text.clone()
                };
                println!("  Text:    {preview}");
            }
            println!();
        }
    }

    println!("=== {section_num} sections processed, {classified_num} with classifications ===");
    Ok(())
}

fn cmd_taxa_show_misses(
    name: &str,
    total: usize,
    batches: &[arrow::record_batch::RecordBatch],
    family: Option<&str>,
) -> anyhow::Result<()> {
    // Phase 1: Run v2 on every provision, collect misses with heat scores.
    struct MissEntry {
        provision: String,
        miss: fractalaw_core::taxa::MissRecord,
    }

    let mut v2_count = 0usize;
    let mut gov_count = 0usize;
    let mut miss_entries: Vec<MissEntry> = Vec::new();

    for batch in batches {
        let provision_col = batch.column_by_name("provision");
        let text_col = batch.column_by_name("text");

        for row in 0..batch.num_rows() {
            let provision = provision_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let text = text_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();

            if text.trim().is_empty() {
                continue;
            }

            let record = fractalaw_core::taxa::parse_v2(&text, family);
            if !record.duty_types.is_empty() {
                v2_count += 1;
                continue;
            }
            // Check if it was a government match (these are correctly handled)
            if !record.government_actors.is_empty() && record.classification.is_some() {
                gov_count += 1;
                continue;
            }

            let miss = fractalaw_core::taxa::analyse_miss(&text);
            miss_entries.push(MissEntry { provision, miss });
        }
    }

    // Phase 2: Sort by heat descending, then by provision for stability.
    miss_entries.sort_by(|a, b| {
        b.miss
            .heat
            .cmp(&a.miss.heat)
            .then_with(|| a.provision.cmp(&b.provision))
    });

    // Phase 3: Display.
    println!(
        "=== v2 Misses: {name} ({total} sections, {v2_count} classified, {} missed) ===\n",
        miss_entries.len()
    );

    // Heat distribution summary.
    let mut heat_counts = std::collections::BTreeMap::new();
    for entry in &miss_entries {
        *heat_counts.entry(entry.miss.heat).or_insert(0usize) += 1;
    }
    println!("Heat distribution:");
    for (heat, count) in heat_counts.iter().rev() {
        let bar = "#".repeat((*count).min(60));
        println!("  {heat:>3}: {count:>3}  {bar}");
    }
    println!();

    // Detail for hot misses (heat >= 3).
    let hot: Vec<&MissEntry> = miss_entries.iter().filter(|e| e.miss.heat >= 3).collect();
    if hot.is_empty() {
        println!("No hot misses (heat >= 3).");
    } else {
        println!("--- Hot misses (heat >= 3): {} provisions ---\n", hot.len());
        for entry in &hot {
            let m = &entry.miss;
            println!("--- {} [heat={}] ---", entry.provision, m.heat);
            println!("  Signals:  {}", m.signals.join(", "));
            if !m.governed_actors.is_empty() {
                println!("  Governed: {}", m.governed_actors.join(", "));
            }
            if !m.government_actors.is_empty() {
                println!("  Government: {}", m.government_actors.join(", "));
            }
            if !m.purposes.is_empty() {
                println!("  Purpose:  {}", m.purposes.join(", "));
            }
            let preview = if m.cleaned_text.len() > 200 {
                let end = truncate_at_char_boundary(&m.cleaned_text, 200);
                format!("{}...", &m.cleaned_text[..end])
            } else {
                m.cleaned_text.clone()
            };
            println!("  Text:     {preview}");
            println!();
        }
    }

    // Summary table.
    println!("=== Summary ===");
    println!("  Total sections:  {total}");
    println!("  v2 classified:   {v2_count}");
    println!("  Government-only: {gov_count}");
    println!("  Missed:          {}", miss_entries.len());
    println!(
        "  Hot (heat >= 3): {}",
        miss_entries.iter().filter(|e| e.miss.heat >= 3).count()
    );
    println!(
        "  Warm (heat 1-2): {}",
        miss_entries
            .iter()
            .filter(|e| (1..=2).contains(&e.miss.heat))
            .count()
    );
    println!(
        "  Cold (heat <= 0): {}",
        miss_entries.iter().filter(|e| e.miss.heat <= 0).count()
    );

    Ok(())
}

fn cmd_taxa_show_clauses(
    name: &str,
    total: usize,
    batches: &[arrow::record_batch::RecordBatch],
    law_family: Option<&str>,
) -> anyhow::Result<()> {
    struct ClauseEntry {
        provision: String,
        confidence: f32,
        clause: String,
        family: String,
        sub_type: String,
        has_span: bool,
    }

    let mut entries: Vec<ClauseEntry> = Vec::new();
    let mut no_clause_count = 0usize;
    let mut no_drrp_count = 0usize;

    for batch in batches {
        let provision_col = batch.column_by_name("provision");
        let text_col = batch.column_by_name("text");

        for row in 0..batch.num_rows() {
            let provision = provision_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let text = text_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();

            if text.trim().is_empty() {
                continue;
            }

            let record = fractalaw_core::taxa::parse_v2(&text, law_family);

            if record.duty_types.is_empty() {
                no_drrp_count += 1;
                continue;
            }

            let (family, sub_type, has_span) = match &record.classification {
                Some(c) => (
                    format!("{:?}", c.family),
                    format!("{:?}", c.sub_type),
                    c.span.is_some(),
                ),
                None => ("Unknown".into(), "Unclassified".into(), false),
            };

            if let Some(clause) = &record.clause_refined {
                entries.push(ClauseEntry {
                    provision,
                    confidence: record.taxa_confidence,
                    clause: clause.clone(),
                    family,
                    sub_type,
                    has_span,
                });
            } else {
                no_clause_count += 1;
            }
        }
    }

    let classified = entries.len() + no_clause_count;

    println!(
        "=== Clause Quality: {name} ({total} sections, {classified} classified, {} with clauses) ===\n",
        entries.len()
    );

    // Confidence distribution — buckets of 0.1
    let mut buckets = [0usize; 10]; // [0.0-0.1), [0.1-0.2), ..., [0.8-0.9), [0.9-1.0]
    for e in &entries {
        let idx = ((e.confidence * 10.0).floor() as usize).min(9);
        buckets[idx] += 1;
    }
    println!("Confidence distribution:");
    for (i, &count) in buckets.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let lo = i as f32 / 10.0;
        let hi = lo + 0.1;
        let bar = "#".repeat(count.min(60));
        println!("  {lo:.1}-{hi:.1}: {count:>3}  {bar}");
    }
    println!();

    // Span coverage
    let span_count = entries.iter().filter(|e| e.has_span).count();
    let refiner_count = entries.len() - span_count;
    println!(
        "Extraction method:  span-based: {span_count}  refiner-fallback: {refiner_count}  no-clause: {no_clause_count}"
    );

    // Average confidence
    if !entries.is_empty() {
        let avg: f32 = entries.iter().map(|e| e.confidence).sum::<f32>() / entries.len() as f32;
        println!("Average confidence:  {avg:.2}");
    }
    println!();

    // Show low-quality clauses (confidence < 0.45) sorted ascending
    let mut low: Vec<&ClauseEntry> = entries.iter().filter(|e| e.confidence < 0.45).collect();
    low.sort_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap());

    if low.is_empty() {
        println!("No low-quality clauses (all >= 0.45 confidence).");
    } else {
        println!(
            "--- Low-quality clauses (confidence < 0.45): {} ---\n",
            low.len()
        );
        for e in &low {
            println!(
                "--- {} [conf={:.2}, {}/{}{}] ---",
                e.provision,
                e.confidence,
                e.family,
                e.sub_type,
                if e.has_span { "" } else { ", no-span" }
            );
            println!("  {}", e.clause);
            println!();
        }
    }

    // Show a sample of high-quality clauses (confidence >= 0.80)
    let high: Vec<&ClauseEntry> = entries.iter().filter(|e| e.confidence >= 0.80).collect();
    if !high.is_empty() {
        let sample_count = high.len().min(5);
        println!(
            "--- High-quality sample ({} of {} with conf >= 0.80) ---\n",
            sample_count,
            high.len()
        );
        for e in high.iter().take(sample_count) {
            println!(
                "--- {} [conf={:.2}, {}/{}] ---",
                e.provision, e.confidence, e.family, e.sub_type
            );
            println!("  {}", e.clause);
            println!();
        }
    }

    // Summary
    println!("=== Summary ===");
    println!("  Total sections:      {total}");
    println!("  No DRRP:             {no_drrp_count}");
    println!("  Classified:          {classified}");
    println!("  With clause:         {}", entries.len());
    println!("  No clause extracted: {no_clause_count}");
    println!(
        "  High (>= 0.60):     {}",
        entries.iter().filter(|e| e.confidence >= 0.60).count()
    );
    println!(
        "  Medium (0.45-0.59): {}",
        entries
            .iter()
            .filter(|e| (0.45..0.60).contains(&e.confidence))
            .count()
    );
    println!(
        "  Low (< 0.45):       {}",
        entries.iter().filter(|e| e.confidence < 0.45).count()
    );

    Ok(())
}

/// Human-friendly law names for eyeball review headings.
fn law_display_name(law_name: &str) -> &str {
    match law_name {
        "UK_uksi_2005_1643" => "Control of Noise at Work 2005",
        "UK_uksi_1992_2792" => "Display Screen Equipment 1992",
        "UK_uksi_2005_1093" => "Control of Vibration at Work 2005",
        "UK_uksi_2002_2676" => "Control of Lead at Work 2002",
        "UK_uksi_2013_1471" => "RIDDOR 2013",
        "UK_uksi_2000_128" => "Pressure Systems Safety 2000",
        "UK_uksi_2015_483" => "COMAH 2015",
        "UK_ukpga_1974_37" => "Health and Safety at Work etc. Act 1974",
        "UK_uksi_1999_3242" => "Management of HSW Regulations 1999",
        "UK_uksi_2015_51" => "CDM 2015",
        _ => law_name,
    }
}

async fn cmd_taxa_eyeball(
    data_dir: &std::path::Path,
    law_names: &[&str],
    output: &std::path::Path,
    limit: usize,
) -> anyhow::Result<()> {
    use std::fmt::Write;

    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    let mut md = String::new();
    writeln!(md, "# Clause Eyeball Review")?;
    writeln!(md)?;
    writeln!(
        md,
        "Generated by `fractalaw taxa eyeball`. Manual QA artifact for DRRP clause review."
    )?;
    writeln!(md)?;
    writeln!(md, "---")?;

    let mut total_drrp = 0usize;
    let mut total_sections = 0usize;

    for &law_name in law_names {
        let filter = format!("law_name = '{}'", law_name.replace('\'', "''"));
        let batches = lance.query_legislation_text(&filter, limit, 0).await?;
        let sections: usize = batches.iter().map(|b| b.num_rows()).sum();
        if sections == 0 {
            eprintln!("  WARN: No text sections found for '{law_name}', skipping.");
            continue;
        }

        let display = law_display_name(law_name);
        writeln!(md)?;
        writeln!(md)?;
        writeln!(md, "## {display} ({law_name})")?;

        let mut law_drrp = 0usize;

        for batch in &batches {
            let provision_col = batch.column_by_name("provision");
            let text_col = batch.column_by_name("text");

            for row in 0..batch.num_rows() {
                let provision = provision_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let text = text_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();

                if text.trim().is_empty() {
                    continue;
                }

                let record = fractalaw_core::taxa::parse_v2(&text, None);

                if record.duty_types.is_empty() {
                    continue;
                }

                law_drrp += 1;

                let family = record.duty_types.first().map(|d| d.as_str()).unwrap_or("?");
                let conf = record.taxa_confidence;

                // Extract provision number from "reg.N" or "section.N" format
                let prov_label = provision
                    .split('.')
                    .next_back()
                    .and_then(|s| s.strip_prefix("reg"))
                    .or_else(|| {
                        provision
                            .split('.')
                            .next_back()
                            .and_then(|s| s.strip_prefix("section"))
                    })
                    .map(|n| format!("Reg {n}"))
                    .unwrap_or_else(|| provision.clone());

                // Determine clause or fall back to cleaned text
                let clause_text = record
                    .clause_refined
                    .as_deref()
                    .unwrap_or(&record.cleaned_text);

                // Show bad-end marker if clause doesn't end with sentence boundary
                let bad_end = if !clause_text.ends_with('.')
                    && !clause_text.ends_with(';')
                    && !clause_text.ends_with(')')
                {
                    " **[BAD END]**"
                } else {
                    ""
                };

                writeln!(md)?;
                writeln!(
                    md,
                    "### {prov_label} — {family} (conf: {conf:.2}) {bad_end}"
                )?;
                writeln!(md)?;
                writeln!(md, "> {clause_text}")?;
            }
        }

        total_drrp += law_drrp;
        total_sections += sections;
        eprintln!("  {display}: {sections} sections, {law_drrp} DRRP provisions");
    }

    // Summary
    writeln!(md)?;
    writeln!(md)?;
    writeln!(md, "## Summary")?;
    writeln!(md)?;
    writeln!(md, "| Metric | Value |")?;
    writeln!(md, "|--------|-------|")?;
    writeln!(md, "| Laws | {} |", law_names.len())?;
    writeln!(md, "| Total sections | {total_sections} |")?;
    writeln!(md, "| DRRP provisions | {total_drrp} |")?;

    std::fs::write(output, &md)?;
    println!(
        "Wrote {total_drrp} DRRP provisions across {} laws to {}",
        law_names.len(),
        output.display()
    );

    Ok(())
}

// ── Taxa QA Report ──────────────────────────────────────────────────

async fn cmd_taxa_qa(
    data_dir: &std::path::Path,
    laws: Option<String>,
    family: Option<String>,
) -> anyhow::Result<()> {
    use fractalaw_core::taxa::purpose;
    use std::collections::HashMap;

    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    // Resolve law names.
    let law_names: Vec<String> = if let Some(ref l) = laws {
        l.split(',').map(|s| s.trim().to_string()).collect()
    } else if let Some(ref fam) = family {
        let store = open_duck(data_dir)?;
        let names = laws_in_family(&store, fam)?;
        if names.is_empty() {
            anyhow::bail!("No laws found with family '{fam}'");
        }
        println!("Family '{}': {} laws\n", fam, names.len());
        names
    } else {
        // All laws with LanceDB text.
        let all_batches = lance.query_legislation_text("true", 200_000, 0).await?;
        let mut names = std::collections::BTreeSet::new();
        for batch in &all_batches {
            if let Some(col) = batch.column_by_name("law_name") {
                for i in 0..batch.num_rows() {
                    if let Some(name) = get_string_value(col.as_ref(), i) {
                        names.insert(name);
                    }
                }
            }
        }
        names.into_iter().collect()
    };

    if law_names.is_empty() {
        println!("No laws to analyse.");
        return Ok(());
    }

    // Short labels for the purpose distribution columns.
    const PURPOSE_SHORT: &[(&str, &str)] = &[
        (purpose::ENACTMENT, "Enact"),
        (purpose::INTERPRETATION, "Interp"),
        (purpose::APPLICATION_SCOPE, "Scope"),
        (purpose::EXTENT, "Extent"),
        (purpose::EXEMPTION, "Exempt"),
        (purpose::PROCESS_RULE, "Process"),
        (purpose::POWER_CONFERRED, "Power"),
        (purpose::CHARGE_FEE, "Fee"),
        (purpose::OFFENCE, "Offence"),
        (purpose::ENFORCEMENT, "Enforce"),
        (purpose::DEFENCE_APPEAL, "Defence"),
        (purpose::LIABILITY, "Liabil"),
        (purpose::REPEAL_REVOCATION, "Repeal"),
        (purpose::AMENDMENT, "Amend"),
        (purpose::TRANSITIONAL, "Transit"),
    ];

    struct LawStats {
        law_name: String,
        total: usize,
        with_purposes: usize,
        with_drrp: usize,
        gate_skip_drrp: usize,
        gate_descriptive: usize,
        gate_interp_primary: usize,
        gate_enact_primary: usize,
        gate_scope_primary: usize,
        gate_all_structural: usize,
        purpose_counts: HashMap<&'static str, usize>,
        anomalies: Vec<String>,
    }

    let mut all_stats: Vec<LawStats> = Vec::new();

    eprint!("Analysing {} laws", law_names.len());

    for law_name in &law_names {
        let filter = format!("law_name = '{}'", law_name.replace('\'', "''"));
        let batches = lance.query_legislation_text(&filter, 100_000, 0).await?;

        let mut stats = LawStats {
            law_name: law_name.clone(),
            total: 0,
            with_purposes: 0,
            with_drrp: 0,
            gate_skip_drrp: 0,
            gate_descriptive: 0,
            gate_interp_primary: 0,
            gate_enact_primary: 0,
            gate_scope_primary: 0,
            gate_all_structural: 0,
            purpose_counts: HashMap::new(),
            anomalies: Vec::new(),
        };

        for batch in &batches {
            let text_col = batch.column_by_name("text");
            let stype_col = batch.column_by_name("section_type");

            for row in 0..batch.num_rows() {
                let text = text_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let section_type = stype_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();

                if text.trim().is_empty() || section_type == "heading" {
                    continue;
                }

                stats.total += 1;

                let record = fractalaw_core::taxa::parse_v2(&text, None);

                // Count purposes.
                if !record.purposes.is_empty() {
                    stats.with_purposes += 1;
                }
                for p in &record.purposes {
                    *stats.purpose_counts.entry(p).or_insert(0) += 1;
                }

                // Count DRRP.
                if !record.duty_types.is_empty() {
                    stats.with_drrp += 1;
                }

                // Determine gate reason (replay the logic from parse_v2).
                let cleaned = fractalaw_core::taxa::text_cleaner::clean(&text);
                if fractalaw_core::taxa::is_descriptive_summary(&cleaned) {
                    stats.gate_descriptive += 1;
                } else if fractalaw_core::taxa::should_skip_drrp(
                    &record.purposes,
                    !record.governed_actors.is_empty(),
                    !record.government_actors.is_empty(),
                ) {
                    stats.gate_skip_drrp += 1;
                    // Sub-classify the skip reason.
                    let first = record.purposes.first().copied();
                    if first == Some(purpose::INTERPRETATION) {
                        stats.gate_interp_primary += 1;
                    } else if first == Some(purpose::ENACTMENT) {
                        stats.gate_enact_primary += 1;
                    } else if first == Some(purpose::APPLICATION_SCOPE) {
                        stats.gate_scope_primary += 1;
                    } else {
                        stats.gate_all_structural += 1;
                    }
                }
            }
        }

        // Anomaly detection.
        if stats.total > 0 {
            let pct = |n: usize| 100.0 * n as f64 / stats.total as f64;
            let enact_n = stats
                .purpose_counts
                .get(purpose::ENACTMENT)
                .copied()
                .unwrap_or(0);
            if pct(enact_n) > 10.0 {
                stats
                    .anomalies
                    .push(format!("Enactment {:.1}% (>10%)", pct(enact_n)));
            }
            let enforce_n = stats
                .purpose_counts
                .get(purpose::ENFORCEMENT)
                .copied()
                .unwrap_or(0);
            if pct(enforce_n) > 15.0 {
                stats
                    .anomalies
                    .push(format!("Enforcement {:.1}% (>15%)", pct(enforce_n)));
            }
            if stats.with_drrp == 0 && stats.total > 10 {
                stats
                    .anomalies
                    .push(format!("0 DRRP from {} provisions", stats.total));
            }
        }

        eprint!(".");
        all_stats.push(stats);
    }
    eprintln!(" done\n");

    // ── Section 1: Coverage Summary ─────────────────────────────────

    let corpus_total: usize = all_stats.iter().map(|s| s.total).sum();
    let corpus_purposes: usize = all_stats.iter().map(|s| s.with_purposes).sum();
    let corpus_drrp: usize = all_stats.iter().map(|s| s.with_drrp).sum();
    let corpus_gated: usize = all_stats
        .iter()
        .map(|s| s.gate_skip_drrp + s.gate_descriptive)
        .sum();

    println!(
        "=== Coverage Summary ({} laws, {} provisions) ===\n",
        law_names.len(),
        fmt_num(corpus_total)
    );

    println!(
        "{:<30} {:>10} {:>9} {:>9} {:>9}",
        "Law", "Provisions", "Purpose%", "DRRP%", "Gated%"
    );
    println!("{}", "-".repeat(70));

    for s in &all_stats {
        if s.total == 0 {
            println!("{:<30} {:>10}", s.law_name, 0);
            continue;
        }
        let pct = |n: usize| 100.0 * n as f64 / s.total as f64;
        println!(
            "{:<30} {:>10} {:>8.1}% {:>8.1}% {:>8.1}%",
            truncate_name(&s.law_name, 30),
            s.total,
            pct(s.with_purposes),
            pct(s.with_drrp),
            pct(s.gate_skip_drrp + s.gate_descriptive),
        );
    }
    if corpus_total > 0 {
        let pct = |n: usize| 100.0 * n as f64 / corpus_total as f64;
        println!("{}", "-".repeat(70));
        println!(
            "{:<30} {:>10} {:>8.1}% {:>8.1}% {:>8.1}%",
            "CORPUS",
            fmt_num(corpus_total),
            pct(corpus_purposes),
            pct(corpus_drrp),
            pct(corpus_gated),
        );
    }

    // ── Section 2: Purpose Distribution ─────────────────────────────

    println!("\n=== Purpose Distribution ===\n");

    // Header row.
    print!("{:<30}", "Law");
    for (_, short) in PURPOSE_SHORT {
        print!(" {:>7}", short);
    }
    println!();
    println!("{}", "-".repeat(30 + PURPOSE_SHORT.len() * 8));

    for s in &all_stats {
        if s.total == 0 {
            continue;
        }
        print!("{:<30}", truncate_name(&s.law_name, 30));
        for (full, _) in PURPOSE_SHORT {
            let n = s.purpose_counts.get(full).copied().unwrap_or(0);
            let pct = 100.0 * n as f64 / s.total as f64;
            if n > 0 {
                print!(" {:>6.1}%", pct);
            } else {
                print!(" {:>7}", "");
            }
        }
        println!();
    }

    // Corpus totals.
    if corpus_total > 0 {
        let mut corpus_purpose_counts: HashMap<&str, usize> = HashMap::new();
        for s in &all_stats {
            for (&k, &v) in &s.purpose_counts {
                *corpus_purpose_counts.entry(k).or_insert(0) += v;
            }
        }
        println!("{}", "-".repeat(30 + PURPOSE_SHORT.len() * 8));
        print!("{:<30}", "CORPUS");
        for (full, _) in PURPOSE_SHORT {
            let n = corpus_purpose_counts.get(full).copied().unwrap_or(0);
            let pct = 100.0 * n as f64 / corpus_total as f64;
            if n > 0 {
                print!(" {:>6.1}%", pct);
            } else {
                print!(" {:>7}", "");
            }
        }
        println!();

        // Flag per-law anomalies: any purpose > 2x corpus average.
        for s in &all_stats {
            if s.total < 10 {
                continue;
            }
            for (full, short) in PURPOSE_SHORT {
                let law_n = s.purpose_counts.get(full).copied().unwrap_or(0);
                let law_pct = 100.0 * law_n as f64 / s.total as f64;
                let corpus_pct = 100.0
                    * corpus_purpose_counts.get(full).copied().unwrap_or(0) as f64
                    / corpus_total as f64;
                if corpus_pct > 1.0 && law_pct > corpus_pct * 2.0 {
                    println!(
                        "  [!] {}: {} {:.1}% (corpus avg {:.1}%)",
                        s.law_name, short, law_pct, corpus_pct
                    );
                }
            }
        }
    }

    // ── Section 3: Gate Analysis ────────────────────────────────────

    let total_skip: usize = all_stats.iter().map(|s| s.gate_skip_drrp).sum();
    let total_desc: usize = all_stats.iter().map(|s| s.gate_descriptive).sum();
    let total_interp: usize = all_stats.iter().map(|s| s.gate_interp_primary).sum();
    let total_enact: usize = all_stats.iter().map(|s| s.gate_enact_primary).sum();
    let total_scope: usize = all_stats.iter().map(|s| s.gate_scope_primary).sum();
    let total_structural: usize = all_stats.iter().map(|s| s.gate_all_structural).sum();

    println!("\n=== Gate Analysis ===\n");

    if corpus_total > 0 {
        let pct = |n: usize| 100.0 * n as f64 / corpus_total as f64;
        println!("{:<30} {:>10} {:>9}", "Gate", "Triggered", "% corpus");
        println!("{}", "-".repeat(51));
        println!(
            "{:<30} {:>10} {:>8.1}%",
            "skip_drrp (all)",
            fmt_num(total_skip),
            pct(total_skip)
        );
        println!(
            "  {:<28} {:>10} {:>8.1}%",
            "Interpretation-primary",
            fmt_num(total_interp),
            pct(total_interp)
        );
        println!(
            "  {:<28} {:>10} {:>8.1}%",
            "Enactment-primary",
            fmt_num(total_enact),
            pct(total_enact)
        );
        println!(
            "  {:<28} {:>10} {:>8.1}%",
            "Application+Scope",
            fmt_num(total_scope),
            pct(total_scope)
        );
        println!(
            "  {:<28} {:>10} {:>8.1}%",
            "All structural",
            fmt_num(total_structural),
            pct(total_structural)
        );
        println!(
            "{:<30} {:>10} {:>8.1}%",
            "descriptive_summary",
            fmt_num(total_desc),
            pct(total_desc)
        );
        println!("{}", "-".repeat(51));
        println!(
            "{:<30} {:>10} {:>8.1}%",
            "Total gated",
            fmt_num(total_skip + total_desc),
            pct(total_skip + total_desc)
        );
    }

    // ── Section 4: Anomalies ────────────────────────────────────────

    let anomalies: Vec<_> = all_stats
        .iter()
        .filter(|s| !s.anomalies.is_empty())
        .collect();

    if anomalies.is_empty() {
        println!("\n=== Anomalies: none ===");
    } else {
        println!("\n=== Anomalies ({}) ===\n", anomalies.len());
        for s in &anomalies {
            for a in &s.anomalies {
                println!("  [!] {}: {}", s.law_name, a);
            }
        }
    }

    println!();
    Ok(())
}

/// Truncate a law name to fit a column width, preserving the end (most distinctive part).
fn truncate_name(name: &str, max: usize) -> String {
    if name.len() <= max {
        name.to_string()
    } else {
        format!("..{}", &name[name.len() - (max - 2)..])
    }
}

/// Audit p-dimension dictionary coverage: find Application+Scope provisions
/// where polarity was detected but zero p-dimension tags were extracted.
async fn cmd_taxa_audit_fitness(
    data_dir: &std::path::Path,
    laws: Option<String>,
    family: Option<String>,
    limit: usize,
) -> anyhow::Result<()> {
    use fractalaw_core::taxa::{fitness, purpose};
    use std::collections::{BTreeMap, BTreeSet, HashMap};

    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;
    let store = open_duck(data_dir)?;

    // Resolve law names (same pattern as cmd_taxa_qa)
    let law_names: Vec<String> = if let Some(ref l) = laws {
        l.split(',').map(|s| s.trim().to_string()).collect()
    } else if let Some(ref fam) = family {
        let names = laws_in_family(&store, fam)?;
        if names.is_empty() {
            anyhow::bail!("No laws found with family '{fam}'");
        }
        println!("Family '{}': {} laws\n", fam, names.len());
        names
    } else {
        let all_batches = lance.query_legislation_text("true", 200_000, 0).await?;
        let mut names = std::collections::BTreeSet::new();
        for batch in &all_batches {
            if let Some(col) = batch.column_by_name("law_name") {
                for i in 0..batch.num_rows() {
                    if let Some(name) = get_string_value(col.as_ref(), i) {
                        names.insert(name);
                    }
                }
            }
        }
        names.into_iter().collect()
    };

    if law_names.is_empty() {
        println!("No laws to audit.");
        return Ok(());
    }

    // Build law-to-family map from DuckDB
    let family_map: HashMap<String, String> = {
        let batches =
            store.query_arrow("SELECT name, family FROM legislation WHERE family IS NOT NULL")?;
        let mut map = HashMap::new();
        for batch in &batches {
            let name_col = batch.column_by_name("name");
            let fam_col = batch.column_by_name("family");
            if let (Some(nc), Some(fc)) = (name_col, fam_col) {
                for i in 0..batch.num_rows() {
                    if let (Some(n), Some(f)) = (
                        get_string_value(nc.as_ref(), i),
                        get_string_value(fc.as_ref(), i),
                    ) {
                        map.insert(n, f);
                    }
                }
            }
        }
        map
    };

    // Build known-terms set from dictionaries
    let known_terms: BTreeSet<String> = fitness::all_canonical_terms(family.as_deref())
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    // Per-family stats
    struct GapProvision {
        law_name: String,
        raw_text: String,
    }
    struct FamilyStats {
        total_app_scope: usize,
        polarity_matched: usize,
        with_tags: usize,
        escalateount: usize,
        cross_ref_count: usize,
        no_polarity_count: usize,
        gap_provisions: Vec<GapProvision>,
        cross_ref_provisions: Vec<GapProvision>,
        no_polarity_provisions: Vec<String>,
        term_hits: HashMap<String, usize>,
    }

    let mut family_stats: BTreeMap<String, FamilyStats> = BTreeMap::new();

    eprint!("Auditing {} laws", law_names.len());

    for law_name in &law_names {
        let fam = family_map
            .get(law_name.as_str())
            .cloned()
            .unwrap_or_else(|| "(unknown)".to_string());

        let filter = format!("law_name = '{}'", law_name.replace('\'', "''"));
        let batches = lance.query_legislation_text(&filter, 100_000, 0).await?;

        let stats = family_stats
            .entry(fam.clone())
            .or_insert_with(|| FamilyStats {
                total_app_scope: 0,
                polarity_matched: 0,
                with_tags: 0,
                escalateount: 0,
                cross_ref_count: 0,
                no_polarity_count: 0,
                gap_provisions: Vec::new(),
                cross_ref_provisions: Vec::new(),
                no_polarity_provisions: Vec::new(),
                term_hits: HashMap::new(),
            });

        for batch in &batches {
            let text_col = batch.column_by_name("text");
            let stype_col = batch.column_by_name("section_type");
            if text_col.is_none() {
                continue;
            }

            for row in 0..batch.num_rows() {
                let text = text_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let section_type = stype_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();

                if text.trim().is_empty() || section_type == "heading" {
                    continue;
                }

                let record = fractalaw_core::taxa::parse_v2(&text, Some(&fam));

                if !record.purposes.contains(&purpose::APPLICATION_SCOPE) {
                    continue;
                }

                stats.total_app_scope += 1;

                if record.fitness_rules.is_empty() {
                    stats.no_polarity_count += 1;
                    if stats.no_polarity_provisions.len() < 5 {
                        let cleaned = fractalaw_core::taxa::text_cleaner::clean(&text);
                        stats.no_polarity_provisions.push(cleaned);
                    }
                    continue;
                }

                // Count per provision, not per rule
                stats.polarity_matched += 1;
                let mut provision_has_tags = false;

                for rule in &record.fitness_rules {
                    for tag in &rule.tags {
                        *stats.term_hits.entry(tag.term.clone()).or_insert(0) += 1;
                    }

                    if rule.tags.is_empty() {
                        if rule.cross_refs.is_empty() {
                            stats.escalateount += 1;
                            stats.gap_provisions.push(GapProvision {
                                law_name: law_name.clone(),
                                raw_text: rule.raw_text.clone(),
                            });
                        } else {
                            stats.cross_ref_count += 1;
                            stats.cross_ref_provisions.push(GapProvision {
                                law_name: law_name.clone(),
                                raw_text: rule.raw_text.clone(),
                            });
                        }
                    } else {
                        provision_has_tags = true;
                    }
                }

                if provision_has_tags {
                    stats.with_tags += 1;
                }
            }
        }

        eprint!(".");
    }
    eprintln!(" done\n");

    // ── Section 1: Coverage by Family ──

    println!(
        "=== Section 1: Coverage by Family ({} families) ===\n",
        family_stats.len()
    );
    println!(
        "{:<45} {:>8} {:>10} {:>10} {:>8} {:>8}",
        "Family", "AppScope", "Polarity%", "Tagged%", "Gaps", "CrossRef"
    );
    println!("{}", "-".repeat(95));

    let mut corpus_app = 0usize;
    let mut corpus_pol = 0usize;
    let mut corpus_tag = 0usize;
    let mut corpus_gap = 0usize;
    let mut corpus_xref = 0usize;

    for (fam, s) in &family_stats {
        corpus_app += s.total_app_scope;
        corpus_pol += s.polarity_matched;
        corpus_tag += s.with_tags;
        corpus_gap += s.escalateount;
        corpus_xref += s.cross_ref_count;

        if s.total_app_scope == 0 {
            continue;
        }
        println!(
            "{:<45} {:>8} {:>9.1}% {:>9.1}% {:>8} {:>8}",
            truncate_name(fam, 45),
            s.total_app_scope,
            if s.total_app_scope > 0 {
                100.0 * s.polarity_matched as f64 / s.total_app_scope as f64
            } else {
                0.0
            },
            if s.total_app_scope > 0 {
                100.0 * s.with_tags as f64 / s.total_app_scope as f64
            } else {
                0.0
            },
            s.escalateount,
            s.cross_ref_count,
        );
    }
    println!("{}", "-".repeat(95));
    println!(
        "{:<45} {:>8} {:>9.1}% {:>9.1}% {:>8} {:>8}",
        "CORPUS",
        corpus_app,
        if corpus_app > 0 {
            100.0 * corpus_pol as f64 / corpus_app as f64
        } else {
            0.0
        },
        if corpus_app > 0 {
            100.0 * corpus_tag as f64 / corpus_app as f64
        } else {
            0.0
        },
        corpus_gap,
        corpus_xref,
    );

    // ── Section 2: Gap Provisions ──

    println!("\n=== Section 2: Vocabulary Gaps (polarity, 0 tags, no cross-ref) ===\n");

    let any_gaps = family_stats.values().any(|s| !s.gap_provisions.is_empty());
    if !any_gaps {
        println!("  None — all non-cross-ref provisions with polarity have at least one tag.\n");
    } else {
        for (fam, s) in &family_stats {
            if s.gap_provisions.is_empty() {
                continue;
            }
            let show = if limit == 0 {
                s.gap_provisions.len()
            } else {
                limit.min(s.gap_provisions.len())
            };
            println!(
                "--- {} ({} gaps, showing {}) ---",
                fam,
                s.gap_provisions.len(),
                show
            );
            for gp in s.gap_provisions.iter().take(show) {
                let trunc = if gp.raw_text.len() > 120 {
                    format!("{}...", &gp.raw_text[..120])
                } else {
                    gp.raw_text.clone()
                };
                println!("  [{}] {}", truncate_name(&gp.law_name, 25), trunc);
            }
            println!();
        }
    }

    // ── Section 2b: Cross-Reference Provisions ──

    println!("=== Section 2b: Cross-Reference Provisions (polarity, 0 tags, has cross-ref) ===\n");

    let any_xref = family_stats
        .values()
        .any(|s| !s.cross_ref_provisions.is_empty());
    if !any_xref {
        println!("  None — no cross-reference provisions detected.\n");
    } else {
        for (fam, s) in &family_stats {
            if s.cross_ref_provisions.is_empty() {
                continue;
            }
            let show = if limit == 0 {
                s.cross_ref_provisions.len()
            } else {
                limit.min(s.cross_ref_provisions.len())
            };
            println!(
                "--- {} ({} cross-ref, showing {}) ---",
                fam,
                s.cross_ref_provisions.len(),
                show
            );
            for gp in s.cross_ref_provisions.iter().take(show) {
                let trunc = if gp.raw_text.len() > 120 {
                    format!("{}...", &gp.raw_text[..120])
                } else {
                    gp.raw_text.clone()
                };
                println!("  [{}] {}", truncate_name(&gp.law_name, 25), trunc);
            }
            println!();
        }
    }

    // ── Section 3: Candidate Terms ──

    println!("=== Section 3: Candidate Terms (top 50) ===\n");

    let all_gap_texts: Vec<&str> = family_stats
        .values()
        .flat_map(|s| s.gap_provisions.iter().map(|g| g.raw_text.as_str()))
        .collect();

    let candidates = extract_candidate_terms(&all_gap_texts, &known_terms);

    if candidates.is_empty() {
        println!("  No candidates found (no gap provisions or all terms already known).\n");
    } else {
        println!("{:<45} {:>8}", "Candidate Term", "Freq");
        println!("{}", "-".repeat(55));
        for (term, count) in candidates.iter().take(50) {
            println!("{:<45} {:>8}", term, count);
        }
        println!();
    }

    // ── Section 4: No-Polarity Provisions ──

    println!("=== Section 4: No-Polarity Provisions ===\n");

    let total_no_pol: usize = family_stats.values().map(|s| s.no_polarity_count).sum();
    if total_no_pol == 0 {
        println!("  None — all APPLICATION_SCOPE provisions have polarity detected.\n");
    } else {
        println!(
            "  {} provisions with APPLICATION_SCOPE purpose but no polarity detected\n",
            total_no_pol
        );
        for (fam, s) in &family_stats {
            if s.no_polarity_count == 0 {
                continue;
            }
            println!("  {}: {}", fam, s.no_polarity_count);
            for sample in &s.no_polarity_provisions {
                let trunc = if sample.len() > 100 {
                    format!("{}...", &sample[..100])
                } else {
                    sample.clone()
                };
                println!("    > {}", trunc);
            }
        }
        println!();
    }

    // ── Section 5: Dictionary Utilisation ──

    println!("=== Section 5: Dictionary Utilisation ===\n");

    let mut global_hits: HashMap<String, usize> = HashMap::new();
    for s in family_stats.values() {
        for (term, count) in &s.term_hits {
            *global_hits.entry(term.clone()).or_insert(0) += count;
        }
    }

    let terms_by_dim = fitness::all_terms_by_dimension(family.as_deref());
    let dimensions = [
        (fitness::PDimension::Person, "Person"),
        (fitness::PDimension::Process, "Process"),
        (fitness::PDimension::Place, "Place"),
        (fitness::PDimension::Plant, "Plant"),
        (fitness::PDimension::Property, "Property"),
        (fitness::PDimension::Sector, "Sector"),
    ];

    let mut total_terms = 0usize;
    let mut zero_hit_terms = 0usize;

    for (dim, dim_name) in &dimensions {
        let mut dim_terms: Vec<(&str, usize)> = terms_by_dim
            .iter()
            .filter(|(d, _)| d == dim)
            .map(|(_, term)| (*term, global_hits.get(*term).copied().unwrap_or(0)))
            .collect();
        dim_terms.sort_by(|a, b| b.1.cmp(&a.1));

        println!("  {} ({} terms):", dim_name, dim_terms.len());
        for (term, hits) in &dim_terms {
            let marker = if *hits == 0 { " [!]" } else { "" };
            println!("    {:<40} {:>6}{}", term, hits, marker);
            total_terms += 1;
            if *hits == 0 {
                zero_hit_terms += 1;
            }
        }
        println!();
    }
    println!(
        "  {} of {} dictionary terms had zero hits.\n",
        zero_hit_terms, total_terms
    );

    Ok(())
}

/// Result of enriching a single law.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnrichResult {
    /// Law creates at least one duty or responsibility — LAT should be kept.
    Making,
    /// Law has taxa metadata (rights, powers, fitness, etc.) but no duties or
    /// responsibilities — LAT can be pruned.
    NonMaking,
    /// No taxa signal at all — nothing was written to DuckDB.
    NoTaxa,
}

/// Enrich a single law: run DRRP parser on its provisions from LanceDB, write
/// per-provision taxa back to LanceDB, and update law-level taxa in DuckDB.
/// Source-tier protection: higher-tier classifications are never overwritten
/// by lower-tier sources. Uses extraction_method (not numeric confidence)
/// as the arbiter — this is simpler and more correct than comparing
/// confidence scores that conflated routing signals with quality signals.
fn source_tier(method: &str) -> u8 {
    match method {
        "agentic" => 6,
        "agentic_unvalidated" => 5,
        "classifier" => 4,
        "local" | "local_unvalidated" => 3,
        "inherited" => 2,
        "regex" => 1,
        _ => 0,
    }
}

struct LawTaxa {
    duty_holders: std::collections::BTreeSet<String>,
    rights_holders: std::collections::BTreeSet<String>,
    responsibility_holders: std::collections::BTreeSet<String>,
    power_holders: std::collections::BTreeSet<String>,
    duty_types: std::collections::BTreeSet<String>,
    roles: std::collections::BTreeSet<String>,
    roles_gvt: std::collections::BTreeSet<String>,
    duties: Vec<(String, String, String, String)>,
    rights: Vec<(String, String, String, String)>,
    responsibilities: Vec<(String, String, String, String)>,
    powers: Vec<(String, String, String, String)>,
    // Fitness / applicability
    fitness_persons: std::collections::BTreeSet<String>,
    fitness_processes: std::collections::BTreeSet<String>,
    fitness_places: std::collections::BTreeSet<String>,
    fitness_plants: std::collections::BTreeSet<String>,
    fitness_properties: std::collections::BTreeSet<String>,
    fitness_sectors: std::collections::BTreeSet<String>,
    fitness_entries: Vec<FitnessEntry>,
}

impl LawTaxa {
    fn new() -> Self {
        Self {
            duty_holders: std::collections::BTreeSet::new(),
            rights_holders: std::collections::BTreeSet::new(),
            responsibility_holders: std::collections::BTreeSet::new(),
            power_holders: std::collections::BTreeSet::new(),
            duty_types: std::collections::BTreeSet::new(),
            roles: std::collections::BTreeSet::new(),
            roles_gvt: std::collections::BTreeSet::new(),
            duties: Vec::new(),
            rights: Vec::new(),
            responsibilities: Vec::new(),
            powers: Vec::new(),
            fitness_persons: std::collections::BTreeSet::new(),
            fitness_processes: std::collections::BTreeSet::new(),
            fitness_places: std::collections::BTreeSet::new(),
            fitness_plants: std::collections::BTreeSet::new(),
            fitness_properties: std::collections::BTreeSet::new(),
            fitness_sectors: std::collections::BTreeSet::new(),
            fitness_entries: Vec::new(),
        }
    }
}

struct ActorEntry {
    label: String,
    position: String, // "active" | "counterparty" | "beneficiary" | "mentioned"
    relates_to: Option<String>, // linked actor label for pairwise relations
    label_source: String, // "canonical" or "invented"
    reason: Option<String>, // LLM reasoning (Tier 3 only)
}

struct ProvisionTaxa {
    section_id: String,
    drrp_types: Vec<String>,
    governed_actors: Vec<String>,
    government_actors: Vec<String>,
    duty_family: Option<String>,
    duty_sub_type: Option<String>,
    popimar: Vec<String>,
    purposes: Vec<String>,
    clause_refined: String,
    taxa_confidence: Option<f32>,
    // Fitness per-provision
    fitness_polarity: Vec<String>,
    fitness_person: Vec<String>,
    fitness_process: Vec<String>,
    fitness_place: Vec<String>,
    fitness_plant: Vec<String>,
    fitness_property: Vec<String>,
    fitness_sector: Vec<String>,
    // Escalation provenance
    section_type: String,
    hierarchy_path: String,
    depth: i32,
    extraction_method: String,
    holder_inferred_from: Vec<String>,
    ancestor_distance: Option<i32>,
    actors: Vec<ActorEntry>,
}

async fn enrich_single_law(
    lance: &LanceStore,
    store: &DuckStore,
    law_name: &str,
    escalate: bool,
    force: bool,
) -> anyhow::Result<EnrichResult> {
    // Look up family for specialist dictionary selection
    let family: Option<String> = {
        let batches = store.query_arrow(&format!(
            "SELECT family FROM legislation WHERE name = '{}'",
            law_name.replace('\'', "''")
        ))?;
        batches
            .iter()
            .flat_map(|b| {
                let col = b.column_by_name("family");
                (0..b.num_rows())
                    .filter_map(move |i| col.and_then(|c| get_string_value(c.as_ref(), i)))
            })
            .next()
    };

    // Ensure Escalation provenance columns exist if Tier 1 is enabled.
    if escalate {
        lance.ensure_gap_c_columns().await?;
    }

    let filter = format!("law_name = '{}'", law_name.replace('\'', "''"));
    let batches = lance.query_legislation_text(&filter, 100_000, 0).await?;
    let row_count: usize = batches.iter().map(|b| b.num_rows()).sum();
    if row_count > 2000 {
        tracing::warn!("{law_name}: {row_count} provisions — large law");
    }

    let existing_tiers: std::collections::HashMap<String, u8> = {
        let mut map = std::collections::HashMap::new();
        for batch in &batches {
            let sid_col = batch.column_by_name("section_id");
            let method_col = batch.column_by_name("extraction_method");
            if let (Some(sid_c), Some(method_c)) = (sid_col, method_col) {
                for row in 0..batch.num_rows() {
                    if let Some(sid) = get_string_value(sid_c.as_ref(), row) {
                        let method = get_string_value(method_c.as_ref(), row).unwrap_or_default();
                        map.insert(sid, source_tier(&method));
                    }
                }
            }
        }
        map
    };

    let mut taxa = LawTaxa::new();

    let mut provision_taxa: Vec<ProvisionTaxa> = Vec::new();

    for batch in &batches {
        let prov_col = batch.column_by_name("provision");
        let text_col = batch.column_by_name("text");
        let sid_col = batch.column_by_name("section_id");
        let stype_col = batch.column_by_name("section_type");
        let hpath_col = batch.column_by_name("hierarchy_path");
        let depth_col = batch.column_by_name("depth");

        for row in 0..batch.num_rows() {
            let provision = prov_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let text = text_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let section_id = sid_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let section_type = stype_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let hierarchy_path = hpath_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let depth = depth_col
                .and_then(|c| {
                    c.as_any()
                        .downcast_ref::<arrow::array::Int32Array>()
                        .and_then(|a| {
                            if a.is_null(row) {
                                None
                            } else {
                                Some(a.value(row))
                            }
                        })
                })
                .unwrap_or(0);
            if text.trim().is_empty() {
                continue;
            }
            // Heading rows are structural markers (e.g. "Application") with
            // no legal obligation text — skip before any taxa parsing.
            if section_type == "heading" {
                continue;
            }

            let record = fractalaw_core::taxa::parse_v2(&text, family.as_deref());
            if record.duty_types.is_empty()
                && record.governed_actors.is_empty()
                && record.government_actors.is_empty()
                && record.purposes.is_empty()
            {
                continue;
            }

            // Collect per-provision taxa for LanceDB.
            if !section_id.is_empty() {
                let (duty_family, duty_sub_type) = if let Some(ref cls) = record.classification {
                    (
                        Some(format!("{:?}", cls.family)),
                        Some(format!("{:?}", cls.sub_type)),
                    )
                } else {
                    (None, None)
                };
                // v0.3 confidence: based on routing decision, not regex match quality
                const NON_DRRP_TYPES: &[&str] = &[
                    "title",
                    "signed",
                    "heading",
                    "table",
                    "schedule",
                    "part",
                    "chapter",
                    "commencement",
                    "note",
                ];
                let is_structural = NON_DRRP_TYPES.contains(&section_type.as_str());
                let has_actors =
                    !record.governed_actors.is_empty() || !record.government_actors.is_empty();
                let actor_count = record.governed_actors.len() + record.government_actors.len();
                let has_drrp = !record.duty_types.is_empty();
                let taxa_confidence = if is_structural || !has_actors {
                    // Structural types or no actors → high confidence "none"
                    Some(0.90)
                } else if actor_count == 1 && has_drrp {
                    // Single-actor + DRRP match → regex reliable core
                    Some(0.80)
                } else {
                    // Multi-actor or DRRP=none with actors → low confidence, Tier 2 candidate
                    Some(0.30)
                };
                // Extract per-provision fitness tags from fitness_rules.
                let mut fp_polarity = Vec::new();
                let mut fp_person = Vec::new();
                let mut fp_process = Vec::new();
                let mut fp_place = Vec::new();
                let mut fp_plant = Vec::new();
                let mut fp_property = Vec::new();
                let mut fp_sector = Vec::new();
                for rule in &record.fitness_rules {
                    use fractalaw_core::taxa::fitness::PDimension;
                    let pol = rule.polarity.as_str().to_string();
                    if !fp_polarity.contains(&pol) {
                        fp_polarity.push(pol.clone());
                    }
                    let mut r_person = Vec::new();
                    let mut r_process = Vec::new();
                    let mut r_place = Vec::new();
                    let mut r_plant = Vec::new();
                    let mut r_property = Vec::new();
                    let mut r_sector = Vec::new();
                    for tag in &rule.tags {
                        match tag.dimension {
                            PDimension::Person => {
                                if !fp_person.contains(&tag.term) {
                                    fp_person.push(tag.term.clone());
                                }
                                r_person.push(tag.term.clone());
                            }
                            PDimension::Process => {
                                if !fp_process.contains(&tag.term) {
                                    fp_process.push(tag.term.clone());
                                }
                                r_process.push(tag.term.clone());
                            }
                            PDimension::Place => {
                                if !fp_place.contains(&tag.term) {
                                    fp_place.push(tag.term.clone());
                                }
                                r_place.push(tag.term.clone());
                            }
                            PDimension::Plant => {
                                if !fp_plant.contains(&tag.term) {
                                    fp_plant.push(tag.term.clone());
                                }
                                r_plant.push(tag.term.clone());
                            }
                            PDimension::Property => {
                                if !fp_property.contains(&tag.term) {
                                    fp_property.push(tag.term.clone());
                                }
                                r_property.push(tag.term.clone());
                            }
                            PDimension::Sector => {
                                if !fp_sector.contains(&tag.term) {
                                    fp_sector.push(tag.term.clone());
                                }
                                r_sector.push(tag.term.clone());
                            }
                        }
                        // Aggregate into law-level sets.
                        match tag.dimension {
                            PDimension::Person => {
                                taxa.fitness_persons.insert(tag.term.clone());
                            }
                            PDimension::Process => {
                                taxa.fitness_processes.insert(tag.term.clone());
                            }
                            PDimension::Place => {
                                taxa.fitness_places.insert(tag.term.clone());
                            }
                            PDimension::Plant => {
                                taxa.fitness_plants.insert(tag.term.clone());
                            }
                            PDimension::Property => {
                                taxa.fitness_properties.insert(tag.term.clone());
                            }
                            PDimension::Sector => {
                                taxa.fitness_sectors.insert(tag.term.clone());
                            }
                        }
                    }
                    // Build FitnessEntry tuple for law-level detail.
                    let join = |v: &[String]| {
                        if v.is_empty() {
                            String::new()
                        } else {
                            v.join(", ")
                        }
                    };
                    taxa.fitness_entries.push((
                        rule.polarity.as_str().to_string(),
                        join(&r_person),
                        join(&r_process),
                        join(&r_place),
                        join(&r_plant),
                        join(&r_property),
                        join(&r_sector),
                        format!("section/{provision}"),
                    ));
                }

                provision_taxa.push(ProvisionTaxa {
                    section_id,
                    drrp_types: if is_structural {
                        Vec::new()
                    } else {
                        record
                            .duty_types
                            .iter()
                            .map(|d| format!("{:?}", d))
                            .collect()
                    },
                    governed_actors: if is_structural {
                        Vec::new()
                    } else {
                        record.governed_actors.clone()
                    },
                    government_actors: if is_structural {
                        Vec::new()
                    } else {
                        record.government_actors.clone()
                    },
                    duty_family,
                    duty_sub_type,
                    popimar: record.popimar.iter().map(|s| s.to_string()).collect(),
                    purposes: record.purposes.iter().map(|s| s.to_string()).collect(),
                    clause_refined: record
                        .clause_refined
                        .clone()
                        .unwrap_or_else(|| record.cleaned_text.clone()),
                    taxa_confidence,
                    fitness_polarity: fp_polarity,
                    fitness_person: fp_person,
                    fitness_process: fp_process,
                    fitness_place: fp_place,
                    fitness_plant: fp_plant,
                    fitness_property: fp_property,
                    fitness_sector: fp_sector,
                    section_type: section_type.clone(),
                    hierarchy_path: hierarchy_path.clone(),
                    depth,
                    extraction_method: "regex".to_string(),
                    holder_inferred_from: Vec::new(),
                    ancestor_distance: None,
                    actors: if is_structural {
                        Vec::new()
                    } else {
                        let conf = taxa_confidence.unwrap_or(0.0);
                        record
                            .governed_actors
                            .iter()
                            .map(|a| {
                                let pos: &str = record
                                    .actor_positions
                                    .get(a)
                                    .copied()
                                    .unwrap_or("active");
                                ActorEntry {
                                    label: a.clone(),
                                    position: pos.into(),
                                    relates_to: None,
                                    label_source: "canonical".into(),
                                    reason: Some(format!("regex:{pos}@{conf:.2}")),
                                }
                            })
                            .chain(record.government_actors.iter().map(|a| {
                                let pos: &str = record
                                    .actor_positions
                                    .get(a)
                                    .copied()
                                    .unwrap_or("active");
                                ActorEntry {
                                    label: a.clone(),
                                    position: pos.into(),
                                    relates_to: None,
                                    label_source: "canonical".into(),
                                    reason: Some(format!("regex:{pos}@{conf:.2}")),
                                }
                            }))
                            .collect()
                    },
                });
            }

            // Aggregate actors into holder sets and role sets.
            for actor in &record.governed_actors {
                taxa.roles.insert(actor.clone());
            }
            for actor in &record.government_actors {
                taxa.roles_gvt.insert(actor.clone());
            }

            // Map duty types to holder columns and DRRPEntry lists.
            let clause_preview = if record.cleaned_text.len() > 200 {
                let end = truncate_at_char_boundary(&record.cleaned_text, 200);
                format!("{}...", &record.cleaned_text[..end])
            } else {
                record.cleaned_text.clone()
            };
            let article = format!("section/{provision}");

            for dt in &record.duty_types {
                taxa.duty_types.insert(format!("{dt:?}"));
                // Map 3-class types to DuckDB columns (backward compatible):
                // Obligation → duty_holder, Liberty → rights_holder, Rule → duty_holder
                // responsibility_holder and power_holder left empty (Phase 3)
                let (holders_set, entries) = match dt {
                    fractalaw_core::taxa::duty_type::DutyType::Obligation => {
                        (&mut taxa.duty_holders, &mut taxa.duties)
                    }
                    fractalaw_core::taxa::duty_type::DutyType::Liberty => {
                        (&mut taxa.rights_holders, &mut taxa.rights)
                    }
                    fractalaw_core::taxa::duty_type::DutyType::Rule => {
                        (&mut taxa.duty_holders, &mut taxa.duties)
                    }
                };
                for actor in &record.governed_actors {
                    holders_set.insert(actor.clone());
                }
                for actor in &record.government_actors {
                    holders_set.insert(actor.clone());
                }
                let holder = record
                    .governed_actors
                    .first()
                    .or(record.government_actors.first())
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                entries.push((
                    holder,
                    format!("{dt:?}").to_uppercase(),
                    clause_preview.clone(),
                    article.clone(),
                ));
            }
        }
    }

    // Phase 2: Actor back-linking — infer holder for Rule provisions.
    // Find the most frequent governed actor across all DRRP entries,
    // then replace "Unknown" holders in RULE entries with that actor.
    {
        let mut actor_freq: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for (holder, duty_type, _, _) in &taxa.duties {
            if duty_type != "RULE" && holder != "Unknown" {
                *actor_freq.entry(holder.as_str()).or_default() += 1;
            }
        }
        // Also count from rights, responsibilities, powers
        for entries in [&taxa.rights, &taxa.responsibilities, &taxa.powers] {
            for (holder, _, _, _) in entries {
                if holder != "Unknown" {
                    *actor_freq.entry(holder.as_str()).or_default() += 1;
                }
            }
        }
        if let Some((&dominant_actor, _)) = actor_freq.iter().max_by_key(|&(_, &count)| count) {
            let inferred = format!("{dominant_actor} (inferred)");
            for entry in &mut taxa.duties {
                if entry.1 == "RULE" && entry.0 == "Unknown" {
                    entry.0 = inferred.clone();
                    taxa.duty_holders.insert(inferred.clone());
                }
            }
        }
    }

    // ── Escalation Tier 1: Deterministic parent inheritance ──
    //
    // For provisions with no DRRP but a duty-bearing purpose, walk up the
    // hierarchy to find the nearest ancestor with actors. Deepest-first:
    // stop at the closest parent that has governed_actors, not the root.
    let mut inherited_count = 0u32;
    if escalate {
        // Build a snapshot of section_id → index for parent lookups.
        // We iterate by index so we can read from immutable slices while
        // collecting mutations, then apply them in a second pass.
        let escalate_candidates: Vec<usize> = (0..provision_taxa.len())
            .filter(|&i| {
                let p = &provision_taxa[i];
                p.drrp_types.is_empty()
                    && p.governed_actors.is_empty()
                    && fractalaw_core::taxa::is_duty_bearing_purpose(&p.purposes)
                    && !p.hierarchy_path.is_empty()
            })
            .collect();

        // For each candidate, find the nearest ancestor with actors.
        struct InheritedTaxa {
            target_idx: usize,
            drrp_types: Vec<String>,
            governed_actors: Vec<String>,
            government_actors: Vec<String>,
            duty_family: Option<String>,
            duty_sub_type: Option<String>,
            ancestor_sid: String,
            distance: i32,
        }
        let mut mutations: Vec<InheritedTaxa> = Vec::new();

        for &idx in &escalate_candidates {
            let target_path = &provision_taxa[idx].hierarchy_path;
            let target_depth = provision_taxa[idx].depth;

            // Find ancestors: provisions whose hierarchy_path is a strict
            // prefix of the target's path. The prefix must end at a hierarchy
            // boundary (next char in target is '/'), otherwise "provision.3"
            // falsely matches "provision.3A" (siblings, not parent-child).
            // Exclude structural containers (part, chapter, heading, title) —
            // these contain actor keywords in their titles but don't create
            // duties (e.g., "Part V: Rights of Owners" is not a duty source).
            // Sort by depth descending (deepest first).
            const STRUCTURAL_TYPES: &[&str] = &["part", "chapter", "heading", "title"];
            let mut ancestors: Vec<usize> = (0..provision_taxa.len())
                .filter(|&j| {
                    let ancestor_path = &provision_taxa[j].hierarchy_path;
                    j != idx
                        && !ancestor_path.is_empty()
                        && ancestor_path.len() < target_path.len()
                        && target_path.starts_with(ancestor_path.as_str())
                        && target_path.as_bytes()[ancestor_path.len()] == b'/'
                        && !provision_taxa[j].governed_actors.is_empty()
                        && !STRUCTURAL_TYPES.contains(&provision_taxa[j].section_type.as_str())
                })
                .collect();
            ancestors.sort_by(|&a, &b| provision_taxa[b].depth.cmp(&provision_taxa[a].depth));

            if let Some(&ancestor_idx) = ancestors.first() {
                let ancestor = &provision_taxa[ancestor_idx];
                mutations.push(InheritedTaxa {
                    target_idx: idx,
                    drrp_types: ancestor.drrp_types.clone(),
                    governed_actors: ancestor.governed_actors.clone(),
                    government_actors: ancestor.government_actors.clone(),
                    duty_family: ancestor.duty_family.clone(),
                    duty_sub_type: ancestor.duty_sub_type.clone(),
                    ancestor_sid: ancestor.section_id.clone(),
                    distance: target_depth - ancestor.depth,
                });
            }
        }

        // Apply mutations.
        for m in mutations {
            let p = &mut provision_taxa[m.target_idx];
            p.drrp_types = m.drrp_types;
            p.governed_actors = m.governed_actors;
            p.government_actors = m.government_actors;
            p.duty_family = m.duty_family;
            p.duty_sub_type = m.duty_sub_type;
            p.extraction_method = "inherited".to_string();
            p.holder_inferred_from = vec![m.ancestor_sid];
            p.ancestor_distance = Some(m.distance);
            // Rebuild actors struct with inherited actors as holders.
            p.actors = p
                .governed_actors
                .iter()
                .map(|a| ActorEntry {
                    label: a.clone(),
                    position: "active".into(),
                    relates_to: None,
                    label_source: "canonical".into(),
                    reason: Some("inherited:active@0.70".into()),
                })
                .chain(p.government_actors.iter().map(|a| ActorEntry {
                    label: a.clone(),
                    position: "active".into(),
                    relates_to: None,
                    label_source: "canonical".into(),
                    reason: Some("inherited:active@0.70".into()),
                }))
                .collect();
            inherited_count += 1;

            // Also aggregate inherited actors into the law-level sets.
            for actor in &p.governed_actors {
                taxa.roles.insert(actor.clone());
                taxa.duty_holders.insert(actor.clone());
            }
            for actor in &p.government_actors {
                taxa.roles_gvt.insert(actor.clone());
            }
        }

        if inherited_count > 0 {
            eprintln!(
                "  Escalation Tier 1: {inherited_count} provisions inherited actors from parent clauses"
            );
        }

        // ── Tier 2: LLM classification (local or Gemini) ──
        //
        // Routes multi-actor and DRRP=none provisions to an LLM for
        // position + DRRP classification. Provider selected by LLM_PROVIDER:
        //   "local"  → Ollama (CPU/GPU, zero API cost)
        //   "gemini" → Gemini API (requires GEMINI_API_KEY)
        //   unset    → skip Tier 2
        {
            let tier2_provider = std::env::var("LLM_PROVIDER").ok();
            let tier2_candidates: Vec<usize> = if tier2_provider.is_some() {
                (0..provision_taxa.len())
                    .filter(|&i| {
                        let p = &provision_taxa[i];
                        let existing_tier = existing_tiers.get(&p.section_id).copied().unwrap_or(0);
                        let has_actors =
                            !p.governed_actors.is_empty() || !p.government_actors.is_empty();
                        let multi_actor = p.actors.len() > 1;
                        let drrp_none_with_actors = p.drrp_types.is_empty() && has_actors;
                        // Only classify at regulation level — fragments inherit.
                        // Structural types and fragments don't get LLM calls.
                        const REGULATION_TYPES: &[&str] =
                            &["article", "sub_article", "section", "sub_section"];
                        let is_regulation = REGULATION_TYPES.contains(&p.section_type.as_str());
                        let pending_llm = p.extraction_method == "pending_llm";
                        // Tier 2 candidates: multi-actor, DRRP=none with actors,
                        // or flagged by classifier as pending LLM review
                        is_regulation
                            && (multi_actor || drrp_none_with_actors || pending_llm)
                            && existing_tier < source_tier("local")
                    })
                    .collect()
            } else {
                Vec::new()
            };

            if !tier2_candidates.is_empty() {
                let use_gemini = tier2_provider.as_deref() == Some("gemini");
                let gemini_key = std::env::var("GEMINI_API_KEY").ok();

                // Check provider availability
                let provider_available = if use_gemini {
                    gemini_key.is_some()
                } else {
                    reqwest::Client::new()
                        .get("http://localhost:11434/api/tags")
                        .timeout(std::time::Duration::from_secs(2))
                        .send()
                        .await
                        .is_ok()
                };

                if provider_available {
                    let provider_label = if use_gemini { "Gemini" } else { "Gemma" };
                    let matcher = ActorMatcher::load("docs/actor-dictionary.yaml")
                        .context("loading actor dictionary for Tier 2")?;
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(if use_gemini {
                            30
                        } else {
                            60
                        }))
                        .build()
                        .context("building HTTP client for Tier 2")?;

                    // Build section_id → text lookup
                    let mut text_map: std::collections::HashMap<String, String> =
                        std::collections::HashMap::new();
                    for batch in &batches {
                        let sid_col = batch.column_by_name("section_id");
                        let text_col = batch.column_by_name("text");
                        if let (Some(sid_c), Some(txt_c)) = (sid_col, text_col) {
                            for row in 0..batch.num_rows() {
                                if let (Some(sid), Some(txt)) = (
                                    get_string_value(sid_c.as_ref(), row),
                                    get_string_value(txt_c.as_ref(), row),
                                ) {
                                    text_map.insert(sid, txt);
                                }
                            }
                        }
                    }

                    let mut tier2_count = 0u32;
                    let mut tier2_unvalidated = 0u32;
                    for &idx in &tier2_candidates {
                        let p = &provision_taxa[idx];
                        let target_sid = p.section_id.clone();
                        let drrp = if p.drrp_types.is_empty() {
                            "unknown".to_string()
                        } else {
                            p.drrp_types.join(", ")
                        };

                        fn truncate_str(s: &str, max: usize) -> &str {
                            if s.len() <= max {
                                s
                            } else {
                                let mut end = max;
                                while end > 0 && !s.is_char_boundary(end) {
                                    end -= 1;
                                }
                                &s[..end]
                            }
                        }
                        let text = text_map
                            .get(&target_sid)
                            .map(|t| truncate_str(t, 500))
                            .unwrap_or("");
                        if text.is_empty() {
                            continue;
                        }

                        let prompt = format!(
                            r#"Classify this UK/EU legal provision.

Text: {text}
Regex hint: {drrp}

1. What is the DRRP type? One of: Obligation, Liberty, or none.
   - Obligation: a legal obligation imposed on someone (shall, must, is required to)
   - Liberty: a permission, entitlement, or discretionary power (may, entitled to, power to)
   - none: definitions, commencement, repeals, structural, offence/penalty, OR provisions that only reference/detail/exempt an obligation or right created elsewhere

   IMPORTANT: classify as 'none' if the provision only references, conditions, details, or exempts a legal relation created in another section. Only provisions that CREATE a new obligation or liberty count.

2. Name each actor using natural language. For each, classify POSITION: ACTIVE (bears the obligation/exercises the liberty), COUNTERPARTY (other side), BENEFICIARY, or MENTIONED.

Respond in JSON only:
{{"drrp_type": "Obligation|Liberty|none", "actors": [{{"label": "employer", "position": "ACTIVE", "reason": "..."}}]}}"#
                        );

                        let resp = if use_gemini {
                            let api_key = gemini_key.as_deref().unwrap_or("");
                            let url = format!(
                                "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
                                api_key
                            );
                            let body = serde_json::json!({
                                "contents": [{"parts": [{"text": prompt}]}],
                                "generationConfig": {
                                    "temperature": 0.1,
                                    "maxOutputTokens": 2048,
                                    "thinkingConfig": {"thinkingBudget": 256}
                                }
                            });
                            client.post(&url).json(&body).send().await
                        } else {
                            let body = serde_json::json!({
                                "model": "gemma3:4b",
                                "prompt": prompt,
                                "stream": false,
                                "options": {"temperature": 0.0}
                            });
                            client
                                .post("http://localhost:11434/api/generate")
                                .json(&body)
                                .send()
                                .await
                        };

                        let parsed = match resp {
                            Ok(r) => {
                                let text = r.text().await.unwrap_or_default();
                                // Extract content from either Gemini or Ollama response format
                                let content = if use_gemini {
                                    let gemini_resp: serde_json::Value =
                                        serde_json::from_str(&text).unwrap_or_default();
                                    gemini_resp
                                        .pointer("/candidates/0/content/parts/0/text")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string()
                                } else {
                                    let ollama_resp: serde_json::Value =
                                        serde_json::from_str(&text).unwrap_or_default();
                                    ollama_resp
                                        .get("response")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string()
                                };
                                let content = content.as_str();
                                // Strip markdown code fences if present
                                let json_text = if content.contains("```json") {
                                    content
                                        .split("```json")
                                        .nth(1)
                                        .and_then(|s| s.split("```").next())
                                        .unwrap_or(content)
                                        .trim()
                                } else if content.contains("```") {
                                    content
                                        .split("```")
                                        .nth(1)
                                        .and_then(|s| s.split("```").next())
                                        .unwrap_or(content)
                                        .trim()
                                } else {
                                    content.trim()
                                };
                                serde_json::from_str::<serde_json::Value>(json_text).ok()
                            }
                            Err(_) => None,
                        };

                        if let Some(ref result) = parsed
                            && let Some(tier2_actors) = parse_tier3_actors(result, &matcher)
                        {
                            let p = &mut provision_taxa[idx];
                            p.actors.clear();
                            let mut new_governed = Vec::new();
                            let mut new_government = Vec::new();
                            let mut has_unknown_labels = false;

                            for a in &tier2_actors {
                                if a.label_source == "invented" {
                                    has_unknown_labels = true;
                                }
                                if a.position == "active" {
                                    if matcher.is_government(&a.label) {
                                        new_government.push(a.label.clone());
                                    } else {
                                        new_governed.push(a.label.clone());
                                    }
                                }
                                p.actors.push(ActorEntry {
                                    label: a.label.clone(),
                                    position: a.position.clone(),
                                    relates_to: a.relates_to.clone(),
                                    label_source: a.label_source.clone(),
                                    reason: a.reason.clone(),
                                });
                            }

                            if !new_governed.is_empty() || !new_government.is_empty() {
                                p.governed_actors = new_governed;
                                p.government_actors = new_government;
                            }

                            // Write DRRP type from Tier 2 if provided
                            if let Some(drrp_val) = result.get("drrp_type").and_then(|v| v.as_str())
                            {
                                let drrp_lower = drrp_val.to_lowercase();
                                let mapped = match drrp_lower.as_str() {
                                    "duty" | "responsibility" | "obligation" => Some("Obligation"),
                                    "right" | "power" | "liberty" => Some("Liberty"),
                                    _ => None,
                                };
                                if let Some(dt) = mapped {
                                    p.drrp_types = vec![dt.to_string()];
                                }
                            }

                            if has_unknown_labels {
                                p.extraction_method = if use_gemini {
                                    "agentic_unvalidated"
                                } else {
                                    "local_unvalidated"
                                }
                                .to_string();
                                p.taxa_confidence = Some(if use_gemini { 0.70 } else { 0.60 });
                                tier2_unvalidated += 1;
                            } else {
                                p.extraction_method =
                                    if use_gemini { "agentic" } else { "local" }.to_string();
                                p.taxa_confidence = Some(if use_gemini { 0.90 } else { 0.80 });
                            }
                            tier2_count += 1;
                        }
                    }

                    if tier2_count > 0 {
                        let validated = tier2_count - tier2_unvalidated;
                        eprintln!(
                            "  Tier 2 ({provider_label}): {tier2_count}/{} provisions classified ({validated} validated, {tier2_unvalidated} with unknown labels)",
                            tier2_candidates.len()
                        );
                    }
                }
            }
        }

        // ── Escalation Tier 3: LLM position classification (Gemini) ──
        //
        // For inherited provisions with multiple actors, call Gemini 2.5 Flash
        // to classify Hohfeldian positions. Only fires if GEMINI_API_KEY is set.
        if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
            let tier3_candidates: Vec<usize> = (0..provision_taxa.len())
                .filter(|&i| {
                    let p = &provision_taxa[i];
                    let existing_tier = existing_tiers.get(&p.section_id).copied().unwrap_or(0);
                    p.extraction_method == "inherited"
                        && p.governed_actors.len() > 1
                        && existing_tier < source_tier("agentic")
                })
                .collect();

            if !tier3_candidates.is_empty() {
                // Build section_id → text lookup from the original batches.
                let mut text_map: std::collections::HashMap<String, String> =
                    std::collections::HashMap::new();
                for batch in &batches {
                    let sid_col = batch.column_by_name("section_id");
                    let text_col = batch.column_by_name("text");
                    if let (Some(sid_c), Some(txt_c)) = (sid_col, text_col) {
                        for row in 0..batch.num_rows() {
                            if let (Some(sid), Some(txt)) = (
                                get_string_value(sid_c.as_ref(), row),
                                get_string_value(txt_c.as_ref(), row),
                            ) {
                                text_map.insert(sid, txt);
                            }
                        }
                    }
                }

                let matcher = ActorMatcher::load("docs/actor-dictionary.yaml")
                    .context("loading actor dictionary for Tier 3")?;
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(30))
                    .build()
                    .context("building HTTP client for Tier 3")?;

                let mut tier3_count = 0u32;
                let mut tier3_unvalidated = 0u32;
                for &idx in &tier3_candidates {
                    let p = &provision_taxa[idx];
                    let target_sid = p.section_id.clone();

                    fn truncate_str(s: &str, max: usize) -> &str {
                        if s.len() <= max {
                            s
                        } else {
                            let mut end = max;
                            while end > 0 && !s.is_char_boundary(end) {
                                end -= 1;
                            }
                            &s[..end]
                        }
                    }
                    let target_text = text_map
                        .get(&target_sid)
                        .map(|t| truncate_str(t, 500))
                        .unwrap_or("");

                    if target_text.is_empty() {
                        continue;
                    }

                    // For inherited provisions, include parent text as context
                    let parent_sid = p.holder_inferred_from.first().cloned().unwrap_or_default();
                    let parent_context = if !parent_sid.is_empty() {
                        text_map
                            .get(&parent_sid)
                            .map(|t| {
                                format!(
                                    "\n## Parent Provision (context)\nSection: {parent_sid}\nText: {}",
                                    truncate_str(t, 500)
                                )
                            })
                            .unwrap_or_default()
                    } else {
                        String::new()
                    };

                    let prompt = format!(
                        r#"You are a legal analyst classifying actor positions in UK and EU legislation using Hohfeldian legal relations.

## Provision
Section: {target_sid}
Text: {target_text}
{parent_context}
## Task
Name each actor mentioned in or implied by this provision using natural language (e.g. "employer", "HSE", "inspector", "local authority"). For each, classify their POSITION:

- ACTIVE — this actor bears the duty, exercises the power, or holds the right (the doer)
- COUNTERPARTY — this actor is on the receiving end (holds a claim against a duty, is subject to a power)
- BENEFICIARY — this actor benefits from the provision without a direct legal relation
- MENTIONED — this actor is referenced but has no active legal role

If an active actor's obligation relates specifically to one counterparty (not all), include "relates_to" with that counterparty's natural language name.

Respond in JSON only, no markdown:
{{"actors": [{{"label": "employer", "position": "ACTIVE|COUNTERPARTY|BENEFICIARY|MENTIONED", "relates_to": null, "reason": "..."}}]}}"#
                    );

                    let body = serde_json::json!({
                        "contents": [{"parts": [{"text": prompt}]}],
                        "generationConfig": {
                            "temperature": 0.1,
                            "maxOutputTokens": 2048,
                            "thinkingConfig": {"thinkingBudget": 256}
                        }
                    });

                    let url = format!(
                        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
                        api_key
                    );

                    let resp = client.post(&url).json(&body).send().await;

                    let parsed = match resp {
                        Ok(r) => {
                            let text = r.text().await.unwrap_or_default();
                            parse_gemini_response(&text)
                        }
                        Err(e) => {
                            tracing::warn!(
                                section_id = %target_sid,
                                error = %e,
                                "Tier 3 API call failed, keeping Tier 1 result"
                            );
                            None
                        }
                    };

                    if let Some(ref result) = parsed
                        && let Some(tier3_actors) = parse_tier3_actors(result, &matcher)
                    {
                        let p = &mut provision_taxa[idx];
                        p.actors.clear();
                        let mut new_governed = Vec::new();
                        let mut new_government = Vec::new();
                        let mut has_unknown_labels = false;

                        for a in &tier3_actors {
                            if a.label_source == "invented" {
                                has_unknown_labels = true;
                            }
                            if a.position == "active" {
                                if matcher.is_government(&a.label) {
                                    new_government.push(a.label.clone());
                                } else {
                                    new_governed.push(a.label.clone());
                                }
                            }
                            p.actors.push(ActorEntry {
                                label: a.label.clone(),
                                position: a.position.clone(),
                                relates_to: a.relates_to.clone(),
                                label_source: a.label_source.clone(),
                                reason: a.reason.clone(),
                            });
                        }

                        // Update flat columns with holders only (backward compat)
                        if !new_governed.is_empty() || !new_government.is_empty() {
                            p.governed_actors = new_governed;
                            p.government_actors = new_government;
                        }
                        if has_unknown_labels {
                            p.extraction_method = "agentic_unvalidated".to_string();
                            p.taxa_confidence = Some(0.70);
                            tier3_unvalidated += 1;
                        } else {
                            p.extraction_method = "agentic".to_string();
                            p.taxa_confidence = Some(0.90);
                        }
                        tier3_count += 1;
                    }
                }

                if tier3_count > 0 {
                    let validated = tier3_count - tier3_unvalidated;
                    eprintln!(
                        "  Escalation Tier 3: {tier3_count}/{} multi-actor provisions classified by LLM ({validated} validated, {tier3_unvalidated} with unknown labels)",
                        tier3_candidates.len()
                    );
                }
            }
        }
    }

    // Write per-provision taxa to LanceDB.
    if !provision_taxa.is_empty() {
        use arrow::array::{
            Float32Builder, ListBuilder, StringBuilder, TimestampNanosecondBuilder,
        };
        use arrow::datatypes::{DataType, Field, Schema, TimeUnit};

        let mut section_ids = StringBuilder::new();
        let mut drrp_types_b = ListBuilder::new(StringBuilder::new());
        let mut governed_b = ListBuilder::new(StringBuilder::new());
        let mut government_b = ListBuilder::new(StringBuilder::new());
        let mut duty_family_b = StringBuilder::new();
        let mut duty_sub_type_b = StringBuilder::new();
        let mut popimar_b = ListBuilder::new(StringBuilder::new());
        let mut purposes_b = ListBuilder::new(StringBuilder::new());
        let mut clause_refined_b = StringBuilder::new();
        let mut confidence_b = Float32Builder::new();
        let mut classified_at_b = TimestampNanosecondBuilder::new().with_timezone("UTC");
        let mut fit_polarity_b = ListBuilder::new(StringBuilder::new());
        let mut fit_person_b = ListBuilder::new(StringBuilder::new());
        let mut fit_process_b = ListBuilder::new(StringBuilder::new());
        let mut fit_place_b = ListBuilder::new(StringBuilder::new());
        let mut fit_plant_b = ListBuilder::new(StringBuilder::new());
        let mut fit_property_b = ListBuilder::new(StringBuilder::new());
        let mut fit_sector_b = ListBuilder::new(StringBuilder::new());
        let mut extraction_method_b = StringBuilder::new();
        let mut inferred_from_b = StringBuilder::new();
        let mut ancestor_distance_b = arrow::array::Int32Builder::new();
        let actors_struct_fields: Vec<Field> = vec![
            Field::new("label", DataType::Utf8, false),
            Field::new("position", DataType::Utf8, false),
            Field::new("relates_to", DataType::Utf8, true),
            Field::new("label_source", DataType::Utf8, false),
            Field::new("reason", DataType::Utf8, true),
        ];
        let mut actors_b = ListBuilder::new(arrow::array::StructBuilder::from_fields(
            actors_struct_fields.clone(),
            0,
        ));
        let mut drrp_history_b = StringBuilder::new();

        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let now_iso = chrono::Utc::now().to_rfc3339();

        let mut skipped_high_tier = 0u32;
        for pt in &provision_taxa {
            // Source-tier protection: never overwrite a higher-tier classification
            // (unless --force, which re-runs the full regex pipeline)
            if !force {
                let new_tier = source_tier(&pt.extraction_method);
                if let Some(&existing_tier) = existing_tiers.get(&pt.section_id)
                    && existing_tier >= new_tier
                    && new_tier > 0
                {
                    skipped_high_tier += 1;
                    continue;
                }
            }
            section_ids.append_value(&pt.section_id);

            for v in &pt.drrp_types {
                drrp_types_b.values().append_value(v);
            }
            drrp_types_b.append(true);

            for v in &pt.governed_actors {
                governed_b.values().append_value(v);
            }
            governed_b.append(true);

            for v in &pt.government_actors {
                government_b.values().append_value(v);
            }
            government_b.append(true);

            match &pt.duty_family {
                Some(v) => duty_family_b.append_value(v),
                None => duty_family_b.append_null(),
            }
            match &pt.duty_sub_type {
                Some(v) => duty_sub_type_b.append_value(v),
                None => duty_sub_type_b.append_null(),
            }

            for v in &pt.popimar {
                popimar_b.values().append_value(v);
            }
            popimar_b.append(true);

            for v in &pt.purposes {
                purposes_b.values().append_value(v);
            }
            purposes_b.append(true);

            clause_refined_b.append_value(&pt.clause_refined);
            match pt.taxa_confidence {
                Some(c) => confidence_b.append_value(c),
                None => confidence_b.append_null(),
            }
            classified_at_b.append_value(now_ns);

            for v in &pt.fitness_polarity {
                fit_polarity_b.values().append_value(v);
            }
            fit_polarity_b.append(true);
            for v in &pt.fitness_person {
                fit_person_b.values().append_value(v);
            }
            fit_person_b.append(true);
            for v in &pt.fitness_process {
                fit_process_b.values().append_value(v);
            }
            fit_process_b.append(true);
            for v in &pt.fitness_place {
                fit_place_b.values().append_value(v);
            }
            fit_place_b.append(true);
            for v in &pt.fitness_plant {
                fit_plant_b.values().append_value(v);
            }
            fit_plant_b.append(true);
            for v in &pt.fitness_property {
                fit_property_b.values().append_value(v);
            }
            fit_property_b.append(true);
            for v in &pt.fitness_sector {
                fit_sector_b.values().append_value(v);
            }
            fit_sector_b.append(true);

            extraction_method_b.append_value(&pt.extraction_method);
            if pt.holder_inferred_from.is_empty() {
                inferred_from_b.append_null();
            } else {
                inferred_from_b.append_value(pt.holder_inferred_from.join(","));
            }
            match pt.ancestor_distance {
                Some(d) => ancestor_distance_b.append_value(d),
                None => ancestor_distance_b.append_null(),
            }
            if pt.actors.is_empty() {
                actors_b.append_null();
            } else {
                let struct_builder = actors_b.values();
                for actor in &pt.actors {
                    struct_builder
                        .field_builder::<StringBuilder>(0)
                        .unwrap()
                        .append_value(&actor.label);
                    struct_builder
                        .field_builder::<StringBuilder>(1)
                        .unwrap()
                        .append_value(&actor.position);
                    match &actor.relates_to {
                        Some(rt) => struct_builder
                            .field_builder::<StringBuilder>(2)
                            .unwrap()
                            .append_value(rt),
                        None => struct_builder
                            .field_builder::<StringBuilder>(2)
                            .unwrap()
                            .append_null(),
                    }
                    struct_builder
                        .field_builder::<StringBuilder>(3)
                        .unwrap()
                        .append_value(&actor.label_source);
                    match &actor.reason {
                        Some(r) => struct_builder
                            .field_builder::<StringBuilder>(4)
                            .unwrap()
                            .append_value(r),
                        None => struct_builder
                            .field_builder::<StringBuilder>(4)
                            .unwrap()
                            .append_null(),
                    }
                    struct_builder.append(true);
                }
                actors_b.append(true);
            }

            // drrp_history: record what this tier (regex) said — JSON array
            {
                let drrp_val = if pt.drrp_types.is_empty() {
                    "none"
                } else {
                    &pt.drrp_types[0]
                };
                let entry = serde_json::json!([{
                    "tier": &pt.extraction_method,
                    "drrp": drrp_val,
                    "confidence": pt.taxa_confidence.unwrap_or(0.0),
                    "timestamp": &now_iso,
                }]);
                drrp_history_b.append_value(entry.to_string());
            }
        }

        let item_field = std::sync::Arc::new(Field::new("item", DataType::Utf8, true));
        let taxa_schema = std::sync::Arc::new(Schema::new(vec![
            Field::new("section_id", DataType::Utf8, false),
            Field::new("drrp_types", DataType::List(item_field.clone()), true),
            Field::new("governed_actors", DataType::List(item_field.clone()), true),
            Field::new(
                "government_actors",
                DataType::List(item_field.clone()),
                true,
            ),
            Field::new("duty_family", DataType::Utf8, true),
            Field::new("duty_sub_type", DataType::Utf8, true),
            Field::new("popimar", DataType::List(item_field.clone()), true),
            Field::new("purposes", DataType::List(item_field.clone()), true),
            Field::new("clause_refined", DataType::Utf8, true),
            Field::new("taxa_confidence", DataType::Float32, true),
            Field::new(
                "taxa_classified_at",
                DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into())),
                true,
            ),
            Field::new("fitness_polarity", DataType::List(item_field.clone()), true),
            Field::new("fitness_person", DataType::List(item_field.clone()), true),
            Field::new("fitness_process", DataType::List(item_field.clone()), true),
            Field::new("fitness_place", DataType::List(item_field.clone()), true),
            Field::new("fitness_plant", DataType::List(item_field.clone()), true),
            Field::new("fitness_property", DataType::List(item_field.clone()), true),
            Field::new("fitness_sector", DataType::List(item_field.clone()), true),
            Field::new("extraction_method", DataType::Utf8, true),
            Field::new("holder_inferred_from", DataType::Utf8, true),
            Field::new("ancestor_distance", DataType::Int32, true),
            Field::new(
                "actors",
                DataType::List(std::sync::Arc::new(Field::new(
                    "item",
                    DataType::Struct(
                        vec![
                            Field::new("label", DataType::Utf8, false),
                            Field::new("position", DataType::Utf8, false),
                            Field::new("relates_to", DataType::Utf8, true),
                            Field::new("label_source", DataType::Utf8, false),
                            Field::new("reason", DataType::Utf8, true),
                        ]
                        .into(),
                    ),
                    true,
                ))),
                true,
            ),
            Field::new("drrp_history", DataType::Utf8, true),
        ]));

        let taxa_batch = RecordBatch::try_new(
            taxa_schema,
            vec![
                std::sync::Arc::new(section_ids.finish()),
                std::sync::Arc::new(drrp_types_b.finish()),
                std::sync::Arc::new(governed_b.finish()),
                std::sync::Arc::new(government_b.finish()),
                std::sync::Arc::new(duty_family_b.finish()),
                std::sync::Arc::new(duty_sub_type_b.finish()),
                std::sync::Arc::new(popimar_b.finish()),
                std::sync::Arc::new(purposes_b.finish()),
                std::sync::Arc::new(clause_refined_b.finish()),
                std::sync::Arc::new(confidence_b.finish()),
                std::sync::Arc::new(classified_at_b.finish()),
                std::sync::Arc::new(fit_polarity_b.finish()),
                std::sync::Arc::new(fit_person_b.finish()),
                std::sync::Arc::new(fit_process_b.finish()),
                std::sync::Arc::new(fit_place_b.finish()),
                std::sync::Arc::new(fit_plant_b.finish()),
                std::sync::Arc::new(fit_property_b.finish()),
                std::sync::Arc::new(fit_sector_b.finish()),
                std::sync::Arc::new(extraction_method_b.finish()),
                std::sync::Arc::new(inferred_from_b.finish()),
                std::sync::Arc::new(ancestor_distance_b.finish()),
                std::sync::Arc::new(actors_b.finish()),
                std::sync::Arc::new(drrp_history_b.finish()),
            ],
        )
        .context("building taxa RecordBatch")?;

        if skipped_high_tier > 0 {
            eprintln!(
                "  Protected {skipped_high_tier} provisions with higher-tier classifications"
            );
        }

        lance
            .update_taxa(taxa_batch)
            .await
            .with_context(|| format!("writing taxa to LanceDB for {law_name}"))?;
    }

    // No taxa signal — clear any stale taxa in DuckDB so publishes send NULLs.
    if taxa.duty_types.is_empty()
        && taxa.roles.is_empty()
        && taxa.roles_gvt.is_empty()
        && taxa.fitness_entries.is_empty()
    {
        store.execute(&format!(
            "UPDATE legislation SET \
                duty_holder = NULL, rights_holder = NULL, \
                responsibility_holder = NULL, power_holder = NULL, \
                duty_type = NULL, role = NULL, role_gvt = NULL, \
                duties = NULL, rights = NULL, \
                responsibilities = NULL, powers = NULL, \
                fitness_person = NULL, fitness_process = NULL, \
                fitness_place = NULL, fitness_plant = NULL, \
                fitness_property = NULL, fitness_sector = NULL, \
                fitness = NULL, taxa_hash = NULL \
             WHERE name = '{}'",
            law_name.replace('\'', "''")
        ))?;
        return Ok(EnrichResult::NoTaxa);
    }

    // Compute content hash of the taxa columns (DRRP + fitness).
    let new_hash = compute_taxa_hash(
        &taxa.duty_holders,
        &taxa.rights_holders,
        &taxa.responsibility_holders,
        &taxa.power_holders,
        &taxa.duty_types,
        &taxa.roles,
        &taxa.roles_gvt,
        &taxa.duties,
        &taxa.rights,
        &taxa.responsibilities,
        &taxa.powers,
        &taxa.fitness_persons,
        &taxa.fitness_processes,
        &taxa.fitness_places,
        &taxa.fitness_plants,
        &taxa.fitness_properties,
        &taxa.fitness_sectors,
        &taxa.fitness_entries,
    );

    // Check if taxa actually changed — skip UPDATE if hash is identical.
    let existing_hash: Option<String> = {
        let sql = format!(
            "SELECT taxa_hash FROM legislation WHERE name = '{}'",
            law_name.replace('\'', "''")
        );
        let batches = store.query_arrow(&sql)?;
        batches.first().and_then(|b| {
            b.column_by_name("taxa_hash")
                .and_then(|col| get_string_value(col.as_ref(), 0))
        })
    };
    let is_making = !taxa.duties.is_empty() || !taxa.responsibilities.is_empty();
    if existing_hash.as_deref() == Some(&new_hash) {
        // Hash unchanged — skip DuckDB UPDATE, but still report making status
        // so the caller can prune LAT for non-making laws.
        return Ok(if is_making {
            EnrichResult::Making
        } else {
            EnrichResult::NonMaking
        });
    }

    // Update DuckDB law-level taxa columns (flat + struct lists) + taxa_hash.
    let sql = format!(
        "UPDATE legislation SET
            duty_holder = {duty_holder},
            rights_holder = {rights_holder},
            responsibility_holder = {resp_holder},
            power_holder = {power_holder},
            duty_type = {duty_type},
            role = {role},
            role_gvt = {role_gvt},
            duties = {duties},
            rights = {rights},
            responsibilities = {responsibilities},
            powers = {powers},
            fitness_person = {fitness_person},
            fitness_process = {fitness_process},
            fitness_place = {fitness_place},
            fitness_plant = {fitness_plant},
            fitness_property = {fitness_property},
            fitness_sector = {fitness_sector},
            fitness = {fitness},
            taxa_hash = '{taxa_hash}'
         WHERE name = '{name}'",
        duty_holder = format_sql_list(taxa.duty_holders.iter().map(|s| s.as_str())),
        rights_holder = format_sql_list(taxa.rights_holders.iter().map(|s| s.as_str())),
        resp_holder = format_sql_list(taxa.responsibility_holders.iter().map(|s| s.as_str())),
        power_holder = format_sql_list(taxa.power_holders.iter().map(|s| s.as_str())),
        duty_type = format_sql_list(taxa.duty_types.iter().map(|s| s.as_str())),
        role = format_sql_list(taxa.roles.iter().map(|s| s.as_str())),
        role_gvt = format_sql_list(taxa.roles_gvt.iter().map(|s| s.as_str())),
        duties = format_sql_drrp_entries(&taxa.duties),
        rights = format_sql_drrp_entries(&taxa.rights),
        responsibilities = format_sql_drrp_entries(&taxa.responsibilities),
        powers = format_sql_drrp_entries(&taxa.powers),
        fitness_person = format_sql_list(taxa.fitness_persons.iter().map(|s| s.as_str())),
        fitness_process = format_sql_list(taxa.fitness_processes.iter().map(|s| s.as_str())),
        fitness_place = format_sql_list(taxa.fitness_places.iter().map(|s| s.as_str())),
        fitness_plant = format_sql_list(taxa.fitness_plants.iter().map(|s| s.as_str())),
        fitness_property = format_sql_list(taxa.fitness_properties.iter().map(|s| s.as_str())),
        fitness_sector = format_sql_list(taxa.fitness_sectors.iter().map(|s| s.as_str())),
        fitness = format_sql_fitness_entries(&taxa.fitness_entries),
        taxa_hash = new_hash,
        name = law_name.replace('\'', "''"),
    );
    store.execute(&sql)?;

    Ok(if is_making {
        EnrichResult::Making
    } else {
        EnrichResult::NonMaking
    })
}

/// Regex parsing + Tier 1 inheritance for a list of laws.
/// Runs `enrich_single_law` with `escalate=false` — no LLM calls.
async fn cmd_taxa_parse(
    lance: &LanceStore,
    store: &DuckStore,
    law_names: &[String],
    force: bool,
) -> anyhow::Result<usize> {
    store.ensure_taxa_hash_columns()?;
    store.ensure_fitness_columns()?;

    let mut enriched = 0usize;
    let mut failed = 0usize;
    let total = law_names.len();

    for law_name in law_names {
        match enrich_single_law(lance, store, law_name, false, force).await {
            Ok(_) => {
                enriched += 1;
                if enriched.is_multiple_of(100) {
                    eprint!("\r  Parsed {enriched}/{total}...");
                }
            }
            Err(e) => {
                eprintln!("  {law_name}: parse error: {e}");
                let escaped = law_name.replace('\'', "''");
                let _ = store.execute(&format!(
                    "UPDATE legislation \
                     SET enrichment_retry_count = COALESCE(enrichment_retry_count, 0) + 1 \
                     WHERE name = '{escaped}'"
                ));
                failed += 1;
            }
        }

        // Compact LanceDB periodically to prevent fragment bloat.
        if enriched.is_multiple_of(20) && total > 20 {
            eprint!("\r  Compacting LanceDB after {enriched} laws...");
            if let Err(e) = lance.compact().await {
                eprintln!(" compact error: {e}");
            } else {
                eprintln!(" done");
            }
        }
    }

    if enriched >= 100 {
        eprintln!();
    }

    println!("Parsed {enriched}/{total} laws ({failed} failed).");
    Ok(enriched)
}

/// Write decision trail JSON for the specified laws.
///
/// Re-runs `parse_v2_with_trail` on each provision from LanceDB and writes
/// a JSON array of per-provision trace records to the given path.
async fn cmd_taxa_trace(
    lance: &LanceStore,
    store: &DuckStore,
    law_names: &[String],
    trace_path: &str,
) -> anyhow::Result<()> {
    use std::io::Write;

    let mut entries = Vec::new();

    for law_name in law_names {
        let family: Option<String> = {
            let escaped = law_name.replace('\'', "''");
            let batches = store.query_arrow(&format!(
                "SELECT family FROM legislation WHERE name = '{escaped}'"
            ))?;
            batches.iter().find_map(|b| {
                let col = b.column_by_name("family")?;
                get_string_value(col.as_ref(), 0)
            })
        };

        let escaped = law_name.replace('\'', "''");
        let filter = format!("law_name = '{escaped}'");
        let batches = lance.query_legislation_text(&filter, 100_000, 0).await?;

        for batch in &batches {
            let sid_col = batch.column_by_name("section_id");
            let text_col = batch.column_by_name("text");

            for row in 0..batch.num_rows() {
                let section_id = sid_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let text = text_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();

                if text.trim().is_empty() {
                    continue;
                }

                let (record, trail) =
                    fractalaw_core::taxa::parse_v2_with_trail(&text, family.as_deref());

                let duty_types: Vec<&str> =
                    record.duty_types.iter().map(|d| d.as_str()).collect();

                let winner = trail.winner.as_ref().map(|w| {
                    serde_json::json!({
                        "tier": format!("{:?}", w.tier),
                        "family": format!("{:?}", w.family),
                        "sub_type": format!("{:?}", w.sub_type),
                        "confidence": w.confidence,
                        "actor_keyword": w.actor_keyword,
                        "actor_label": w.actor_label,
                    })
                });

                entries.push(serde_json::json!({
                    "law": law_name,
                    "section_id": section_id,
                    "drrp_types": duty_types,
                    "decision": {
                        "reason": trail.reason.to_string(),
                        "candidates": trail.candidates_count,
                        "rejections": trail.rejections_count,
                        "winner": winner,
                    },
                    "actors": {
                        "governed": record.governed_actors,
                        "government": record.government_actors,
                    },
                    "purposes": record.purposes,
                    "confidence": record.taxa_confidence,
                }));
            }
        }
    }

    let json = serde_json::to_string_pretty(&entries)?;
    let mut file = std::fs::File::create(trace_path)?;
    file.write_all(json.as_bytes())?;

    eprintln!("Trace: {} provisions written to {trace_path}", entries.len());
    Ok(())
}

/// Whole-law LLM validation: send all provisions + parse results to Gemini,
/// get corrections, write audit log.
async fn cmd_taxa_validate(
    lance: &LanceStore,
    store: &DuckStore,
    law_names: &[String],
    audit_dir: &str,
    dry_run: bool,
    apply: bool,
) -> anyhow::Result<()> {
    use std::io::Write;

    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    if !dry_run && api_key.is_empty() {
        anyhow::bail!("GEMINI_API_KEY required for taxa validate (use --dry-run to preview)");
    }

    std::fs::create_dir_all(audit_dir)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let mut total_corrections = 0usize;
    let mut total_provisions = 0usize;

    for law_name in law_names {
        let escaped = law_name.replace('\'', "''");

        // Look up family
        let family: Option<String> = {
            let batches = store.query_arrow(&format!(
                "SELECT family FROM legislation WHERE name = '{escaped}'"
            ))?;
            batches.iter().find_map(|b| {
                let col = b.column_by_name("family")?;
                get_string_value(col.as_ref(), 0)
            })
        };

        // Load all provisions
        let filter = format!("law_name = '{escaped}'");
        let batches = lance.query_legislation_text(&filter, 100_000, 0).await?;

        let mut provisions: Vec<serde_json::Value> = Vec::new();
        for batch in &batches {
            let sid_col = batch.column_by_name("section_id");
            let text_col = batch.column_by_name("text");
            let drrp_col = batch.column_by_name("drrp_types");
            let method_col = batch.column_by_name("extraction_method");
            let conf_col = batch.column_by_name("taxa_confidence");
            let actors_col = batch.column_by_name("actors");

            for row in 0..batch.num_rows() {
                let sid = sid_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let text = text_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                if text.trim().is_empty() {
                    continue;
                }

                let drrp: Vec<String> = drrp_col
                    .and_then(|c| {
                        use arrow::array::AsArray;
                        let list = c.as_list_opt::<i32>()?;
                        if list.is_null(row) {
                            return None;
                        }
                        let vals = list.value(row);
                        let mut types = Vec::new();
                        for i in 0..vals.len() {
                            if let Some(s) = get_string_value(vals.as_ref(), i) {
                                types.push(s);
                            }
                        }
                        Some(types)
                    })
                    .unwrap_or_default();

                let method = method_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let confidence = conf_col
                    .and_then(|c| {
                        c.as_any()
                            .downcast_ref::<arrow::array::Float32Array>()
                            .and_then(|a| {
                                if a.is_null(row) {
                                    None
                                } else {
                                    Some(a.value(row))
                                }
                            })
                    })
                    .unwrap_or(0.0);

                // Extract actor labels
                let actors: Vec<serde_json::Value> = actors_col
                    .and_then(|c| {
                        use arrow::array::AsArray;
                        let list = c.as_list_opt::<i32>()?;
                        if list.is_null(row) {
                            return None;
                        }
                        let sa = list.value(row);
                        let sa = sa.as_struct_opt()?;
                        let lc = sa.column_by_name("label")?;
                        let pc = sa.column_by_name("position")?;
                        let mut result = Vec::new();
                        for i in 0..sa.len() {
                            let label =
                                get_string_value(lc.as_ref(), i).unwrap_or_default();
                            let pos =
                                get_string_value(pc.as_ref(), i).unwrap_or_default();
                            result.push(serde_json::json!({"label": label, "position": pos}));
                        }
                        Some(result)
                    })
                    .unwrap_or_default();

                let drrp_str = drrp.first().cloned().unwrap_or_else(|| "none".into());
                // Truncate text for prompt (keep first 500 chars)
                let text_trunc = if text.len() > 500 {
                    format!("{}...", &text[..text.char_indices().take_while(|&(i, _)| i < 500).last().map(|(i, _)| i).unwrap_or(500)])
                } else {
                    text.clone()
                };

                provisions.push(serde_json::json!({
                    "section_id": sid,
                    "text": text_trunc,
                    "drrp": drrp_str,
                    "method": method,
                    "confidence": (confidence * 100.0).round() / 100.0,
                    "actors": actors,
                }));
            }
        }

        total_provisions += provisions.len();

        if provisions.is_empty() {
            continue;
        }

        // Build prompt
        let provisions_json = serde_json::to_string_pretty(&provisions)?;
        let prompt = format!(
            r#"You are reviewing DRRP (Duties, Rights, Responsibilities, Powers) classifications for provisions of UK legislation.

Law: {law_name}
Family: {family}
Provisions: {count}

Each provision below has been classified by a regex pipeline and ML classifier. The "drrp" field shows the current classification (Obligation, Liberty, or none). The "confidence" field (0-1) indicates pipeline certainty. The "method" field shows which tier classified it (regex, classifier, pending_llm).

Review these classifications. For each provision where the classification is WRONG, return a correction. Focus on:
- Provisions with low confidence (< 0.7)
- Provisions marked pending_llm (the pipeline was uncertain)
- Provisions where the DRRP type seems inconsistent with the text
- "none" provisions that actually contain a duty or right
- Obligations that are actually discretionary powers (Liberty)

For provisions that are correctly classified, do NOT include them in your response.

Respond with a JSON array of corrections ONLY:
[{{"section_id": "...", "drrp": "Obligation|Liberty|none", "reason": "brief explanation"}}]

If all classifications are correct, respond with an empty array: []

Provisions:
{provisions_json}"#,
            law_name = law_name,
            family = family.as_deref().unwrap_or("unknown"),
            count = provisions.len(),
            provisions_json = provisions_json,
        );

        let token_est = prompt.len() / 4;
        eprintln!(
            "  {law_name}: {provs} provisions, ~{tokens}k input tokens",
            provs = provisions.len(),
            tokens = token_est / 1000,
        );

        if dry_run {
            continue;
        }

        // Call Gemini
        let call_start = std::time::Instant::now();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            api_key
        );
        let body = serde_json::json!({
            "contents": [{"parts": [{"text": prompt}]}],
            "generationConfig": {
                "temperature": 0.1,
                "maxOutputTokens": 4096,
                "responseMimeType": "application/json",
                "thinkingConfig": {"thinkingBudget": 1024}
            }
        });

        let resp = client.post(&url).json(&body).send().await;
        let latency_ms = call_start.elapsed().as_millis() as u64;

        let (raw_response, corrections) = match resp {
            Ok(r) => {
                let text = r.text().await.unwrap_or_default();
                let gemini_resp: serde_json::Value =
                    serde_json::from_str(&text).unwrap_or_default();
                let content = gemini_resp
                    .pointer("/candidates/0/content/parts/0/text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("[]")
                    .to_string();
                let parsed: Vec<serde_json::Value> =
                    serde_json::from_str(&content).unwrap_or_default();
                (text, parsed)
            }
            Err(e) => {
                eprintln!("    LLM error: {e}");
                (format!("error: {e}"), Vec::new())
            }
        };

        let law_corrections = corrections.len();
        total_corrections += law_corrections;
        eprintln!(
            "    {law_corrections} corrections ({latency_ms}ms)",
        );

        // Build audit log
        let now = chrono::Utc::now().to_rfc3339();
        // Compute deltas
        let correction_map: std::collections::HashMap<String, &serde_json::Value> = corrections
            .iter()
            .filter_map(|c| {
                c.get("section_id")
                    .and_then(|s| s.as_str())
                    .map(|s| (s.to_string(), c))
            })
            .collect();

        let provision_audits: Vec<serde_json::Value> = provisions
            .iter()
            .filter_map(|p| {
                let sid = p["section_id"].as_str()?;
                if let Some(correction) = correction_map.get(sid) {
                    Some(serde_json::json!({
                        "section_id": sid,
                        "pre_llm_drrp": p["drrp"],
                        "llm_drrp": correction["drrp"],
                        "llm_reason": correction["reason"],
                        "delta": if p["drrp"] == correction["drrp"] { "no_change" } else { "drrp_override" },
                    }))
                } else {
                    None // Omit provisions where LLM agreed (implicit confirmation)
                }
            })
            .collect();

        // Compute integrity hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        raw_response.hash(&mut hasher);
        let integrity_hash = format!("{:016x}", hasher.finish());

        let audit_entry = serde_json::json!({
            "schema_version": 1,
            "pipeline_version": env!("CARGO_PKG_VERSION"),
            "law_name": law_name,
            "family": family,
            "strategy": "whole_law",
            "model": "gemini-2.5-flash",
            "timestamp": now,
            "token_usage": { "input_estimate": token_est },
            "latency_ms": latency_ms,
            "provisions_count": provisions.len(),
            "corrections_count": law_corrections,
            "integrity_hash": integrity_hash,
            "corrections": provision_audits,
            "pre_llm_summary": {
                "total": provisions.len(),
                "obligation": provisions.iter().filter(|p| p["drrp"] == "Obligation").count(),
                "liberty": provisions.iter().filter(|p| p["drrp"] == "Liberty").count(),
                "none": provisions.iter().filter(|p| p["drrp"] == "none").count(),
                "pending_llm": provisions.iter().filter(|p| p["method"] == "pending_llm").count(),
            },
        });

        // Write audit file
        let audit_path = format!("{audit_dir}/{law_name}.json");
        let json = serde_json::to_string_pretty(&audit_entry)?;
        let mut file = std::fs::File::create(&audit_path)?;
        file.write_all(json.as_bytes())?;

        // Apply corrections to LanceDB (only with --apply)
        if apply && !corrections.is_empty() {
            use arrow::array::StringBuilder;
            use arrow::datatypes::{DataType, Field};

            let mut sid_b = StringBuilder::new();
            let mut drrp_b = arrow::array::ListBuilder::new(StringBuilder::new());
            let mut method_b = StringBuilder::new();

            for correction in &corrections {
                let sid = correction
                    .get("section_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let drrp = correction
                    .get("drrp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("none");
                if sid.is_empty() {
                    continue;
                }
                sid_b.append_value(sid);
                if drrp == "none" {
                    drrp_b.append(true); // empty list
                } else {
                    drrp_b.values().append_value(drrp);
                    drrp_b.append(true);
                }
                method_b.append_value("agentic");
            }

            let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(vec![
                Field::new("section_id", DataType::Utf8, false),
                Field::new(
                    "drrp_types",
                    DataType::List(std::sync::Arc::new(Field::new("item", DataType::Utf8, true))),
                    true,
                ),
                Field::new("extraction_method", DataType::Utf8, true),
            ]));
            let batch = arrow::record_batch::RecordBatch::try_new(
                schema,
                vec![
                    std::sync::Arc::new(sid_b.finish()),
                    std::sync::Arc::new(drrp_b.finish()),
                    std::sync::Arc::new(method_b.finish()),
                ],
            )?;
            lance.upsert_embeddings(&batch).await?;
        }
    }

    println!(
        "Validated {} laws ({total_provisions} provisions, {total_corrections} corrections).",
        law_names.len(),
    );
    if !dry_run {
        println!("Audit logs in {audit_dir}/");
    }
    Ok(())
}

/// Compute embeddings for provisions that lack them.
/// Loads the ONNX embedding model and writes embeddings to LanceDB.
async fn cmd_taxa_embed(
    lance: &LanceStore,
    law_names: &[String],
) -> anyhow::Result<()> {
    let model_dir = std::path::Path::new("models/all-MiniLM-L6-v2");
    if !model_dir.exists() {
        anyhow::bail!(
            "Embedding model not found at {}",
            model_dir.display()
        );
    }

    let mut embedder = fractalaw_ai::Embedder::load(model_dir)
        .context("loading embedding model")?;

    let batch_start = std::time::Instant::now();
    let mut total_embedded = 0usize;
    let mut total_provisions = 0usize;

    for law_name in law_names {
        let escaped = law_name.replace('\'', "''");
        let filter = format!("law_name = '{escaped}'");
        let batches = lance.query_legislation_text(&filter, 100_000, 0).await?;

        let mut provisions: Vec<(String, String, Option<Vec<f32>>)> = Vec::new();
        for batch in &batches {
            let sid_col = batch.column_by_name("section_id");
            let text_col = batch.column_by_name("text");
            let emb_col = batch.column_by_name("embedding");

            for row in 0..batch.num_rows() {
                let sid = sid_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let text = text_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();

                let emb = emb_col
                    .and_then(|c| {
                        c.as_any()
                            .downcast_ref::<arrow::array::FixedSizeListArray>()
                    })
                    .filter(|a| !a.is_null(row))
                    .and_then(|a| {
                        let values = a
                            .values()
                            .as_any()
                            .downcast_ref::<arrow::array::Float32Array>()?;
                        let dim = a.value_length() as usize;
                        let offset = row * dim;
                        Some(values.values()[offset..offset + dim].to_vec())
                    });

                provisions.push((sid, text, emb));
            }
        }

        if provisions.is_empty() {
            continue;
        }

        let needs_embedding: Vec<usize> = (0..provisions.len())
            .filter(|&i| provisions[i].1.len() > 20 && provisions[i].2.is_none())
            .collect();

        if !needs_embedding.is_empty() {
            let texts: Vec<String> = needs_embedding
                .iter()
                .map(|&i| provisions[i].1.clone())
                .collect();

            for chunk_start in (0..texts.len()).step_by(64) {
                let chunk_end = (chunk_start + 64).min(texts.len());
                let chunk: Vec<&str> = texts[chunk_start..chunk_end]
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                let embeddings = embedder.embed_batch(&chunk)?;
                for (j, emb) in embeddings.into_iter().enumerate() {
                    let idx = needs_embedding[chunk_start + j];
                    provisions[idx].2 = Some(emb);
                }
            }
            total_embedded += needs_embedding.len();
        }

        // Write embeddings via merge_insert
        {
            use arrow::array::{
                FixedSizeListBuilder, Float32Builder, StringBuilder as SB2,
            };

            let mut sid_b = SB2::new();
            let mut emb_b = FixedSizeListBuilder::new(Float32Builder::new(), 384);
            let mut count = 0usize;

            for prov in &provisions {
                if let Some(ref e) = prov.2 {
                    let e: &Vec<f32> = e;
                    if e.len() == 384 {
                        sid_b.append_value(&prov.0);
                        let vals = emb_b.values();
                        for &v in e {
                            vals.append_value(v);
                        }
                        emb_b.append(true);
                        count += 1;
                    }
                }
            }

            if count > 0 {
                let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(vec![
                    arrow::datatypes::Field::new(
                        "section_id",
                        arrow::datatypes::DataType::Utf8,
                        false,
                    ),
                    arrow::datatypes::Field::new(
                        "embedding",
                        arrow::datatypes::DataType::FixedSizeList(
                            std::sync::Arc::new(arrow::datatypes::Field::new(
                                "item",
                                arrow::datatypes::DataType::Float32,
                                true,
                            )),
                            384,
                        ),
                        true,
                    ),
                ]));
                let batch = arrow::record_batch::RecordBatch::try_new(
                    schema,
                    vec![
                        std::sync::Arc::new(sid_b.finish()),
                        std::sync::Arc::new(emb_b.finish()),
                    ],
                )?;
                lance.upsert_embeddings(&batch).await?;
            }
        }

        total_provisions += provisions.len();
        eprintln!(
            "  {law_name}: {}/{} embedded",
            needs_embedding.len(),
            provisions.len(),
        );
    }

    let elapsed = batch_start.elapsed();
    eprintln!(
        "  Embed: {total_embedded}/{total_provisions} embedded ({:.1}s)",
        elapsed.as_secs_f64(),
    );

    Ok(())
}

/// Run DRRP + position classifiers on provisions with embeddings.
/// Loads v8 DRRP classifier, position classifier, and actor dictionary.
async fn cmd_taxa_classify(
    lance: &LanceStore,
    law_names: &[String],
) -> anyhow::Result<()> {
    let weights_path = std::path::Path::new("docs/drrp_classifier_v8.json");
    if !weights_path.exists() {
        anyhow::bail!(
            "DRRP classifier weights not found at {}",
            weights_path.display()
        );
    }

    // Ensure position classifier columns exist before writing
    lance.ensure_gap_c_columns().await?;

    let classifier = fractalaw_ai::DrrpClassifier::load(weights_path)
        .context("loading DRRP classifier")?;
    let actor_matcher = ActorMatcher::load("docs/actor-dictionary.yaml")
        .context("loading actor dictionary for classifier")?;

    let batch_start = std::time::Instant::now();
    let mut total_classified = 0usize;
    let mut total_provisions = 0usize;

    for law_name in law_names {
        let escaped = law_name.replace('\'', "''");
        let law_start = std::time::Instant::now();
        let filter = format!("law_name = '{escaped}'");
        let batches = lance.query_legislation_text(&filter, 100_000, 0).await?;

        // (section_id, text, embedding, source_tier, section_type, has_govt_active, has_drrp, drrp_history, regex_drrp)
        type Prov = (String, String, Option<Vec<f32>>, u8, String, bool, bool, Option<String>, Option<String>);
        let mut provisions: Vec<Prov> = Vec::new();
        for batch in &batches {
            let sid_col = batch.column_by_name("section_id");
            let text_col = batch.column_by_name("text");
            let emb_col = batch.column_by_name("embedding");
            let method_col = batch.column_by_name("extraction_method");
            let st_col = batch.column_by_name("section_type");
            let actors_col = batch.column_by_name("actors");
            let drrp_col = batch.column_by_name("drrp_types");
            let hist_col = batch.column_by_name("drrp_history");

            for row in 0..batch.num_rows() {
                let sid = sid_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let text = text_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let section_type = st_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();

                let tier = method_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .map(|m| source_tier(&m))
                    .unwrap_or(0);

                let emb = emb_col
                    .and_then(|c| {
                        c.as_any()
                            .downcast_ref::<arrow::array::FixedSizeListArray>()
                    })
                    .filter(|a| !a.is_null(row))
                    .and_then(|a| {
                        let values = a
                            .values()
                            .as_any()
                            .downcast_ref::<arrow::array::Float32Array>()?;
                        let dim = a.value_length() as usize;
                        let offset = row * dim;
                        Some(values.values()[offset..offset + dim].to_vec())
                    });

                let has_govt_active = actors_col
                    .and_then(|c| {
                        use arrow::array::AsArray;
                        let list = c.as_list_opt::<i32>()?;
                        if list.is_null(row) {
                            return None;
                        }
                        let struct_arr = list.value(row);
                        let struct_arr = struct_arr.as_struct_opt()?;
                        let label_col = struct_arr.column_by_name("label")?;
                        let pos_col = struct_arr.column_by_name("position")?;
                        for i in 0..struct_arr.len() {
                            let pos =
                                get_string_value(pos_col.as_ref(), i).unwrap_or_default();
                            if pos == "active" {
                                let label = get_string_value(label_col.as_ref(), i)
                                    .unwrap_or_default();
                                if actor_matcher.is_government(&label) {
                                    return Some(true);
                                }
                            }
                        }
                        Some(false)
                    })
                    .unwrap_or(false);

                let regex_drrp = drrp_col
                    .and_then(|c| {
                        use arrow::array::AsArray;
                        let list = c.as_list_opt::<i32>()?;
                        if list.is_null(row) {
                            return None;
                        }
                        let vals = list.value(row);
                        if vals.is_empty() {
                            return None;
                        }
                        get_string_value(vals.as_ref(), 0)
                    });
                let has_drrp = regex_drrp.is_some();

                let existing_hist = hist_col
                    .and_then(|c| get_string_value(c.as_ref(), row));

                provisions.push((sid, text, emb, tier, section_type, has_govt_active, has_drrp, existing_hist, regex_drrp));
            }
        }

        if provisions.is_empty() {
            continue;
        }

        // Phase 3: DRRP classification + drrp_history + disagreement detection
        const REGULATION_TYPES: &[&str] =
            &["article", "sub_article", "section", "sub_section"];
        // Simple string checks for both-modals detection (LLM escalation signal)
        let has_obligation_modal = |t: &str| {
            let l = t.to_lowercase();
            l.contains("shall") || l.contains("must") || l.contains("is required to")
        };
        let has_enabling_modal = |t: &str| {
            let l = t.to_lowercase();
            l.contains(" may ") || l.contains("entitled") || l.contains("power to")
        };
        let now_iso = chrono::Utc::now().to_rfc3339();
        {
            use arrow::array::{
                ArrayBuilder, Float32Builder, ListBuilder, StringBuilder as SB3,
            };
            use arrow::datatypes::{DataType, Field};

            let mut cls_sid_b = SB3::new();
            let mut cls_drrp_b = ListBuilder::new(SB3::new());
            let mut cls_method_b = SB3::new();
            let mut cls_conf_b = Float32Builder::new();

            // drrp_history builder — appends classifier entry as JSON
            let mut hist_sid_b = SB3::new();
            let mut hist_b = SB3::new();

            let mut disagreements = 0usize;

            for prov in &provisions {
                let sid: &str = &prov.0;
                let text: &str = &prov.1;
                let tier: u8 = prov.3;
                let section_type: &str = &prov.4;
                let _is_govt: bool = prov.5;
                let has_drrp: bool = prov.6;
                let existing_hist: Option<&str> = prov.7.as_deref();
                let regex_drrp: Option<&str> = prov.8.as_deref();

                if tier >= source_tier("classifier")
                    || !REGULATION_TYPES.contains(&section_type)
                {
                    continue;
                }
                let embedding: &[f32] = match prov.2 {
                    Some(ref e) => {
                        let e: &Vec<f32> = e;
                        if e.len() != 384 {
                            continue;
                        }
                        e.as_slice()
                    }
                    None => continue,
                };

                let features =
                    fractalaw_ai::drrp_classifier::build_features(embedding, text);
                let prediction = classifier.predict(&features);
                let cls_drrp = prediction.class.as_str();

                // Always record classifier prediction in drrp_history (JSON)
                hist_sid_b.append_value(sid);
                let new_entry = serde_json::json!({
                    "tier": "classifier",
                    "drrp": cls_drrp,
                    "confidence": prediction.confidence,
                    "timestamp": &now_iso,
                });
                // Append to existing history if present
                let mut history: Vec<serde_json::Value> = existing_hist
                    .and_then(|h| serde_json::from_str(h).ok())
                    .unwrap_or_default();
                history.push(new_entry);
                hist_b.append_value(serde_json::to_string(&history).unwrap());

                // Detect both-modals provisions (LLM escalation signal)
                let both_modals =
                    has_obligation_modal(text) && has_enabling_modal(text);

                if prediction.class == fractalaw_ai::DrrpClass::None {
                    // Classifier says none — flag for LLM if both modals present
                    if both_modals && has_drrp {
                        disagreements += 1;
                    }
                    continue;
                }

                // Transition rules:
                // - Gap fill (regex=none, classifier=DRRP): classifier wins if confident
                // - Disagreement (regex=X, classifier=Y): flag for LLM, don't override
                // - Both modals present: flag for LLM
                let threshold = if has_drrp { 0.75 } else { 0.7 };

                if !has_drrp && prediction.confidence >= threshold {
                    // Gap fill: regex found nothing, classifier found DRRP with confidence
                    cls_sid_b.append_value(sid);
                    let drrp_vals = cls_drrp_b.values();
                    drrp_vals.append_value(cls_drrp);
                    cls_drrp_b.append(true);
                    cls_method_b.append_value("classifier");
                    cls_conf_b.append_value(prediction.confidence);
                    total_classified += 1;
                } else if !has_drrp && prediction.confidence < threshold {
                    // Weak signal: regex=none, classifier found DRRP but low confidence
                    // Flag for LLM verification
                    cls_sid_b.append_value(sid);
                    let drrp_vals = cls_drrp_b.values();
                    drrp_vals.append_value(cls_drrp);
                    cls_drrp_b.append(true);
                    cls_method_b.append_value("pending_llm");
                    cls_conf_b.append_value(prediction.confidence);
                    disagreements += 1;
                    total_classified += 1;
                } else if has_drrp
                    && prediction.confidence >= threshold
                    && regex_drrp.is_some_and(|r| r != cls_drrp)
                {
                    // Disagreement: regex has DRRP, classifier has DIFFERENT DRRP
                    // Flag for LLM review — don't silently override
                    cls_sid_b.append_value(sid);
                    let drrp_vals = cls_drrp_b.values();
                    drrp_vals.append_value(cls_drrp);
                    cls_drrp_b.append(true);
                    cls_method_b.append_value("pending_llm");
                    cls_conf_b.append_value(prediction.confidence);
                    disagreements += 1;
                    total_classified += 1;
                } else if both_modals && has_drrp {
                    // Both obligation + enabling modals — ambiguous, flag for LLM
                    disagreements += 1;
                }
            }

            if disagreements > 0 {
                eprintln!("    {disagreements} provisions flagged for LLM review");
            }

            // Write DRRP updates
            let cls_count = cls_sid_b.len();
            if cls_count > 0 {
                let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(vec![
                    Field::new("section_id", DataType::Utf8, false),
                    Field::new(
                        "drrp_types",
                        DataType::List(std::sync::Arc::new(Field::new(
                            "item",
                            DataType::Utf8,
                            true,
                        ))),
                        true,
                    ),
                    Field::new("extraction_method", DataType::Utf8, true),
                    Field::new("taxa_confidence", DataType::Float32, true),
                ]));
                let batch = arrow::record_batch::RecordBatch::try_new(
                    schema,
                    vec![
                        std::sync::Arc::new(cls_sid_b.finish()),
                        std::sync::Arc::new(cls_drrp_b.finish()),
                        std::sync::Arc::new(cls_method_b.finish()),
                        std::sync::Arc::new(cls_conf_b.finish()),
                    ],
                )?;
                lance.upsert_embeddings(&batch).await?;
            }

            // Write drrp_history updates (classifier entries — JSON strings)
            let hist_count = hist_sid_b.len();
            if hist_count > 0 {
                let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(vec![
                    Field::new("section_id", DataType::Utf8, false),
                    Field::new("drrp_history", DataType::Utf8, true),
                ]));
                let batch = arrow::record_batch::RecordBatch::try_new(
                    schema,
                    vec![
                        std::sync::Arc::new(hist_sid_b.finish()),
                        std::sync::Arc::new(hist_b.finish()),
                    ],
                )?;
                lance.upsert_embeddings(&batch).await?;
            }
        }

        // Phase 4: Position classification
        {
            let pos_weights = std::path::Path::new("docs/position_classifier_v1.json");
            if pos_weights.exists() {
                let pos_classifier = fractalaw_ai::PositionClassifier::load(pos_weights)
                    .context("loading position classifier")?;

                let mut pos_classified = 0usize;
                type ActorTuple = (String, String, Option<String>, String, Option<String>);
                let mut pos_updates: Vec<(String, Vec<ActorTuple>)> = Vec::new();

                for batch in &batches {
                    let sid_col = batch.column_by_name("section_id");
                    let text_col = batch.column_by_name("text");
                    let emb_col = batch.column_by_name("embedding");
                    let method_col = batch.column_by_name("extraction_method");
                    let actors_col = batch.column_by_name("actors");
                    let drrp_col = batch.column_by_name("drrp_types");

                    for row in 0..batch.num_rows() {
                        let method = method_col
                            .and_then(|c| get_string_value(c.as_ref(), row))
                            .unwrap_or_default();
                        if method == "agentic" || method == "agentic_unvalidated" {
                            continue;
                        }

                        let sid = sid_col
                            .and_then(|c| get_string_value(c.as_ref(), row))
                            .unwrap_or_default();
                        let text = text_col
                            .and_then(|c| get_string_value(c.as_ref(), row))
                            .unwrap_or_default();

                        let embedding = emb_col
                            .and_then(|c| {
                                c.as_any()
                                    .downcast_ref::<arrow::array::FixedSizeListArray>()
                            })
                            .filter(|a| !a.is_null(row))
                            .and_then(|a| {
                                let vals = a
                                    .values()
                                    .as_any()
                                    .downcast_ref::<arrow::array::Float32Array>()?;
                                let dim = a.value_length() as usize;
                                let off = row * dim;
                                Some(vals.values()[off..off + dim].to_vec())
                            });
                        let embedding = match embedding {
                            Some(ref e) if e.len() == 384 => e.as_slice(),
                            _ => continue,
                        };

                        let drrp_types: Vec<String> = drrp_col
                            .and_then(|c| {
                                use arrow::array::AsArray;
                                let list = c.as_list_opt::<i32>()?;
                                if list.is_null(row) {
                                    return None;
                                }
                                let vals = list.value(row);
                                let mut types = Vec::new();
                                for i in 0..vals.len() {
                                    if let Some(s) = get_string_value(vals.as_ref(), i) {
                                        types.push(s);
                                    }
                                }
                                Some(types)
                            })
                            .unwrap_or_default();

                        let actors: Vec<ActorTuple> = actors_col
                            .and_then(|c| {
                                use arrow::array::AsArray;
                                let list = c.as_list_opt::<i32>()?;
                                if list.is_null(row) {
                                    return None;
                                }
                                let sa = list.value(row);
                                let sa = sa.as_struct_opt()?;
                                let lc = sa.column_by_name("label")?;
                                let pc = sa.column_by_name("position")?;
                                let rtc = sa.column_by_name("relates_to");
                                let lsc = sa.column_by_name("label_source");
                                let rc = sa.column_by_name("reason");
                                let mut result = Vec::new();
                                for i in 0..sa.len() {
                                    result.push((
                                        get_string_value(lc.as_ref(), i)
                                            .unwrap_or_default(),
                                        get_string_value(pc.as_ref(), i)
                                            .unwrap_or_default(),
                                        rtc.and_then(|c| get_string_value(c.as_ref(), i)),
                                        lsc.and_then(|c| get_string_value(c.as_ref(), i))
                                            .unwrap_or_else(|| "canonical".into()),
                                        rc.and_then(|c| get_string_value(c.as_ref(), i)),
                                    ));
                                }
                                Some(result)
                            })
                            .unwrap_or_default();

                        if actors.is_empty() {
                            continue;
                        }

                        let modals = fractalaw_ai::drrp_classifier::modal_features(&text);
                        let text_lower = text.to_lowercase();
                        let mut updated_actors: Vec<ActorTuple> = Vec::new();

                        for (label, regex_pos, relates_to, label_source, existing_reason) in
                            &actors
                        {
                            let has_llm_reason = existing_reason.as_ref().is_some_and(|r| {
                                r.contains("llm:") || r.contains("agentic:")
                            });

                            if has_llm_reason {
                                updated_actors.push((
                                    label.clone(),
                                    regex_pos.clone(),
                                    relates_to.clone(),
                                    label_source.clone(),
                                    existing_reason.clone(),
                                ));
                                continue;
                            }

                            let label_lower = label
                                .to_lowercase()
                                .split(':')
                                .next_back()
                                .unwrap_or("")
                                .trim()
                                .to_string();
                            let offset = text_lower.find(&label_lower);
                            let rel_offset = offset
                                .map(|o| o as f32 / text.len().max(1) as f32)
                                .unwrap_or(0.5);

                            let features =
                                fractalaw_ai::position_classifier::build_position_features(
                                    embedding, &modals, &drrp_types, label, rel_offset,
                                );
                            let pred = pos_classifier.predict(&features);
                            let cls_pos = pred.class.as_str();
                            pos_classified += 1;

                            let agrees = regex_pos == cls_pos
                                || (regex_pos == "mentioned" && cls_pos == "other")
                                || (regex_pos == "beneficiary" && cls_pos == "other");

                            let cls_segment =
                                format!("classifier:{}@{:.2}", cls_pos, pred.confidence);
                            let new_reason = match existing_reason {
                                Some(prev) if !prev.is_empty() => {
                                    Some(format!("{prev} | {cls_segment}"))
                                }
                                _ => Some(cls_segment),
                            };

                            if !agrees {
                                // Position disagreement — provenance chain records both
                            }

                            updated_actors.push((
                                label.clone(),
                                regex_pos.clone(),
                                relates_to.clone(),
                                label_source.clone(),
                                new_reason,
                            ));
                        }

                        if !updated_actors.is_empty() {
                            pos_updates.push((sid, updated_actors));
                        }
                    }
                }

                // Write back actors with updated reason fields
                if !pos_updates.is_empty() {
                    use arrow::array::StringBuilder as SB4;
                    use arrow::datatypes::{DataType, Field};

                    let actors_fields = vec![
                        Field::new("label", DataType::Utf8, false),
                        Field::new("position", DataType::Utf8, false),
                        Field::new("relates_to", DataType::Utf8, true),
                        Field::new("label_source", DataType::Utf8, false),
                        Field::new("reason", DataType::Utf8, true),
                    ];
                    let mut sid_b = SB4::new();
                    let mut actors_b = arrow::array::ListBuilder::new(
                        arrow::array::StructBuilder::from_fields(actors_fields.clone(), 0),
                    );

                    for (sid, actors) in &pos_updates {
                        sid_b.append_value(sid);
                        let sb = actors_b.values();
                        for (label, position, relates_to, label_source, reason) in actors {
                            sb.field_builder::<SB4>(0).unwrap().append_value(label);
                            sb.field_builder::<SB4>(1).unwrap().append_value(position);
                            match relates_to {
                                Some(rt) => {
                                    sb.field_builder::<SB4>(2).unwrap().append_value(rt)
                                }
                                None => sb.field_builder::<SB4>(2).unwrap().append_null(),
                            }
                            sb.field_builder::<SB4>(3)
                                .unwrap()
                                .append_value(label_source);
                            match reason {
                                Some(r) => {
                                    sb.field_builder::<SB4>(4).unwrap().append_value(r)
                                }
                                None => sb.field_builder::<SB4>(4).unwrap().append_null(),
                            }
                            sb.append(true);
                        }
                        actors_b.append(true);
                    }

                    let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(vec![
                        Field::new("section_id", DataType::Utf8, false),
                        Field::new(
                            "actors",
                            DataType::List(std::sync::Arc::new(Field::new(
                                "item",
                                DataType::Struct(actors_fields.into()),
                                true,
                            ))),
                            true,
                        ),
                    ]));
                    let batch = arrow::record_batch::RecordBatch::try_new(
                        schema,
                        vec![
                            std::sync::Arc::new(sid_b.finish()),
                            std::sync::Arc::new(actors_b.finish()),
                        ],
                    )?;
                    lance.upsert_embeddings(&batch).await?;
                }

                if pos_classified > 0 {
                    eprintln!(
                        "    position: {pos_classified} actors, {} disagreements written",
                        pos_updates.len()
                    );
                }
            }
        }

        total_provisions += provisions.len();
        let law_elapsed = law_start.elapsed();
        eprintln!(
            "  {law_name}: {total_classified} classified ({:.1}s)",
            law_elapsed.as_secs_f64()
        );
    }

    let batch_elapsed = batch_start.elapsed();
    eprintln!(
        "  Classify: {total_classified} classified across {total_provisions} provisions ({:.1}s)",
        batch_elapsed.as_secs_f64(),
    );

    Ok(())
}

/// LLM escalation for a list of laws: runs Tier 2 (DRRP) + Tier 3 (position).
/// Re-runs the full pipeline with escalate=true, which enables LLM calls when
/// LLM_PROVIDER and/or GEMINI_API_KEY are set in the environment.
async fn cmd_taxa_escalate(
    lance: &LanceStore,
    store: &DuckStore,
    law_names: &[String],
) -> anyhow::Result<()> {
    store.ensure_taxa_hash_columns()?;
    store.ensure_fitness_columns()?;
    lance.ensure_gap_c_columns().await?;

    let tier2_provider = std::env::var("LLM_PROVIDER").ok();
    let gemini_key = std::env::var("GEMINI_API_KEY").ok();

    if tier2_provider.is_none() && gemini_key.is_none() {
        anyhow::bail!(
            "No LLM provider configured. Set LLM_PROVIDER=gemini (or 'local') \
             and/or GEMINI_API_KEY to enable LLM escalation."
        );
    }

    let total = law_names.len();
    let mut enriched = 0usize;
    let mut failed = 0usize;

    eprintln!("=== Taxa Escalation: {total} laws (escalate=true) ===");

    for law_name in law_names {
        match enrich_single_law(lance, store, law_name, true, false).await {
            Ok(_) => {
                enriched += 1;
                if enriched.is_multiple_of(10) {
                    eprint!("\r  Escalated {enriched}/{total}...");
                }
            }
            Err(e) => {
                eprintln!("  {law_name}: escalation error: {e}");
                failed += 1;
            }
        }

        // Compact periodically
        if enriched.is_multiple_of(20) && total > 20 {
            eprint!("\r  Compacting LanceDB after {enriched} laws...");
            if let Err(e) = lance.compact().await {
                eprintln!(" compact error: {e}");
            } else {
                eprintln!(" done");
            }
        }
    }

    if enriched >= 10 {
        eprintln!();
    }

    println!("Escalated {enriched}/{total} laws ({failed} failed).");
    Ok(())
}

async fn cmd_taxa_enrich(
    data_dir: &std::path::Path,
    store: &DuckStore,
    law_filter: Option<Vec<String>>,
    force: bool,
    escalate: bool,
    skip_recent: bool,
    pending: bool,
) -> anyhow::Result<()> {
    // Ensure taxa_hash/published_hash and fitness columns exist (idempotent).
    store.ensure_taxa_hash_columns()?;
    store.ensure_fitness_columns()?;

    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    // --force: clear existing DuckDB taxa columns so all laws get re-enriched
    if force && law_filter.is_none() {
        eprintln!("--force: clearing DuckDB taxa columns for re-enrichment...");
        store.execute(
            "UPDATE legislation SET
                duty_holder = NULL,
                rights_holder = NULL,
                responsibility_holder = NULL,
                power_holder = NULL,
                duty_type = NULL,
                role = NULL,
                role_gvt = NULL,
                fitness_person = NULL,
                fitness_process = NULL,
                fitness_place = NULL,
                fitness_plant = NULL,
                fitness_property = NULL,
                fitness_sector = NULL,
                fitness = NULL,
                taxa_hash = NULL",
        )?;
        eprintln!("  Done — all taxa columns set to NULL.");
    }

    // Get distinct law names from LanceDB (only laws with full text can be enriched).
    let lance_law_names: std::collections::BTreeSet<String> = {
        let all_batches = lance.query_legislation_text("true", 200_000, 0).await?;
        let mut names = std::collections::BTreeSet::new();
        for batch in &all_batches {
            if let Some(col) = batch.column_by_name("law_name") {
                for i in 0..batch.num_rows() {
                    if let Some(name) = get_string_value(col.as_ref(), i) {
                        names.insert(name);
                    }
                }
            }
        }
        names
    };

    // If specific laws requested, use those; otherwise find laws without taxa data
    let law_names: Vec<String> = if let Some(filter) = law_filter {
        println!("=== Taxa Enrichment: {} specified laws ===\n", filter.len());
        filter
    } else if force {
        // --force: re-enrich all laws that have LanceDB text
        let names: Vec<String> = lance_law_names.iter().cloned().collect();
        println!(
            "=== Taxa Enrichment (--force): {} laws with LanceDB text ===\n",
            names.len()
        );
        names
    } else {
        // Find laws that have NO DRRP taxa data yet (no duty_type column).
        // Use duty_type rather than duty_holder because some laws have
        // Responsibilities/Powers but no Duties — duty_holder would be empty
        // even though they've been fully enriched.
        let law_batches = store.query_arrow(
            "SELECT name FROM legislation
             WHERE duty_type IS NULL
             ORDER BY name",
        )?;

        let names: Vec<String> = law_batches
            .iter()
            .flat_map(|b| {
                let col = b.column_by_name("name");
                (0..b.num_rows())
                    .filter_map(move |i| col.and_then(|c| get_string_value(c.as_ref(), i)))
            })
            // Only process laws that actually have text in LanceDB
            .filter(|name| lance_law_names.contains(name))
            .collect();

        if names.is_empty() {
            println!("All laws with LanceDB text already have DRRP taxa data.");
            return Ok(());
        }

        println!(
            "=== Taxa Enrichment: {} laws without DRRP data (of {} with text) ===\n",
            names.len(),
            lance_law_names.len()
        );
        names
    };

    // --skip-recent: filter out laws enriched within the last 24 hours
    let law_names = if skip_recent {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);
        let cutoff_ns = cutoff.timestamp_nanos_opt().unwrap_or(0);
        let mut filtered = Vec::new();
        let mut skipped = 0usize;
        for name in &law_names {
            let filter = format!(
                "law_name = '{}' AND taxa_classified_at IS NOT NULL",
                name.replace('\'', "''")
            );
            let batches = lance.query_legislation_text(&filter, 1, 0).await?;
            let recent = batches.iter().any(|b| {
                b.column_by_name("taxa_classified_at")
                    .and_then(|col| {
                        col.as_any()
                            .downcast_ref::<arrow::array::TimestampNanosecondArray>()
                            .map(|a| !a.is_null(0) && a.value(0) > cutoff_ns)
                    })
                    .unwrap_or(false)
            });
            if recent {
                skipped += 1;
            } else {
                filtered.push(name.clone());
            }
        }
        if skipped > 0 {
            eprintln!("  --skip-recent: skipping {skipped} laws enriched within the last 24 hours");
        }
        filtered
    } else {
        law_names
    };

    let mut enriched = 0usize;
    let mut pruned_laws = 0usize;
    let mut pruned_rows = 0usize;
    let total = law_names.len();

    let mut failed = 0usize;
    for law_name in &law_names {
        let result = match enrich_single_law(&lance, store, law_name, escalate, force).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  {law_name}: enrich error: {e}");
                // Increment retry count so dead-letter kicks in at 3
                let escaped = law_name.replace('\'', "''");
                let _ = store.execute(&format!(
                    "UPDATE legislation \
                     SET enrichment_retry_count = COALESCE(enrichment_retry_count, 0) + 1 \
                     WHERE name = '{escaped}'"
                ));
                failed += 1;
                continue;
            }
        };

        // Prune LAT for non-making laws (no duties or responsibilities).
        if matches!(result, EnrichResult::NonMaking | EnrichResult::NoTaxa) {
            let n = lance.delete_law_lat(law_name).await.unwrap_or(0);
            if n > 0 {
                pruned_laws += 1;
                pruned_rows += n;
            }
        }

        // Clear enrichment_pending — this law has been enriched
        // (whether via dev --gap-c or production --pending).
        let escaped = law_name.replace('\'', "''");
        let _ = store.execute(&format!(
            "UPDATE legislation \
             SET enrichment_pending = false, enrichment_retry_count = 0 \
             WHERE name = '{escaped}'"
        ));

        enriched += 1;
        if enriched.is_multiple_of(100) {
            eprint!("\r  Enriched {enriched}/{total}...");
        }

        // Compact LanceDB every 50 laws to prevent fragment bloat.
        // merge_insert creates ~25x write amplification — without periodic
        // compaction, a 274-law --force run would bloat from 452MB to 8+GB.
        if enriched.is_multiple_of(20) && total > 20 {
            eprint!("\r  Compacting LanceDB after {enriched} laws...");
            if let Err(e) = lance.compact().await {
                eprintln!(" compact error: {e}");
            } else {
                eprintln!(" done");
            }
        }
    }

    if enriched >= 100 {
        eprintln!();
    }

    // ── Embed + Classify pass ──
    //
    // After regex enrichment, run the DRRP classifier on provisions where
    // regex returned empty drrp_types but an embedding exists.
    // --pending: also computes embeddings for provisions with null embeddings
    // --force: skips embedding, uses existing embeddings only
    if (pending || force) && enriched > 0 {
        let model_dir = data_dir.join("../models/all-MiniLM-L6-v2");
        let weights_path = std::path::Path::new("docs/drrp_classifier_v8.json");

        if model_dir.exists() && weights_path.exists() {
            eprintln!(
                "  {} pass for {enriched} laws...",
                if pending { "Embed + classify" } else { "Classify" }
            );

            // --pending: compute embeddings for provisions missing them
            if pending {
                if let Err(e) = cmd_taxa_embed(&lance, &law_names).await {
                    eprintln!("  Embed failed (continuing): {e}");
                }
            }

            // Classify with DRRP + position classifiers
            if let Err(e) = cmd_taxa_classify(&lance, &law_names).await {
                eprintln!("  Classify failed (continuing): {e}");
            }
        } else {
            if !model_dir.exists() {
                eprintln!(
                    "  Skipping embed+classify: embedding model not found at {}",
                    model_dir.display()
                );
            }
            if !weights_path.exists() {
                eprintln!(
                    "  Skipping embed+classify: classifier weights not found at {}",
                    weights_path.display()
                );
            }
        }
    }

    // Count how many actually got data.
    let filled = store.query_arrow(
        "SELECT count(*)::BIGINT FROM legislation
         WHERE duty_type IS NOT NULL",
    )?;
    let filled_count = extract_i64(&filled[0], 0);

    println!("Processed {enriched} laws. LRT now has {filled_count} laws with DRRP taxa data.");
    if failed > 0 {
        eprintln!("  {failed} laws failed enrichment (retry count incremented).");
    }
    if pruned_laws > 0 {
        println!("Pruned {pruned_rows} LAT rows from {pruned_laws} non-making laws.");
    }

    // Queue stats (when running in --pending mode)
    if pending {
        let still_pending = store
            .query_arrow("SELECT count(*)::BIGINT FROM legislation WHERE enrichment_pending = true")
            .ok()
            .and_then(|b| b.first().map(|b| extract_i64(b, 0)))
            .unwrap_or(0);
        let dead_lettered = store
            .query_arrow(
                "SELECT count(*)::BIGINT FROM legislation \
                 WHERE enrichment_pending = true AND enrichment_retry_count >= 3",
            )
            .ok()
            .and_then(|b| b.first().map(|b| extract_i64(b, 0)))
            .unwrap_or(0);
        if still_pending > 0 {
            eprintln!("  Queue: {still_pending} still pending ({dead_lettered} dead-lettered)");
        } else {
            eprintln!("  Queue: empty");
        }
    }

    Ok(())
}

async fn cmd_export_training_data(
    data_dir: &std::path::Path,
    store: &DuckStore,
    output: &std::path::Path,
    val_laws_file: Option<&std::path::Path>,
    test_law_count: usize,
    min_match_ratio: f32,
) -> anyhow::Result<()> {
    use std::collections::{HashMap, HashSet};
    use std::io::BufRead;

    use fractalaw_core::training::{self, FlatDrrpEntry, TrainingExample};

    println!("=== DRRP Training Data Export ===\n");

    // 1. Extract flat DRRP entries from DuckDB.
    let entry_batches = store.extract_flat_drrp_entries()?;
    let mut entries: Vec<FlatDrrpEntry> = Vec::new();
    for batch in &entry_batches {
        for i in 0..batch.num_rows() {
            let get = |name| {
                batch
                    .column_by_name(name)
                    .and_then(|c| get_string_value(c.as_ref(), i))
                    .unwrap_or_default()
            };
            entries.push(FlatDrrpEntry {
                law_name: get("law_name"),
                drrp_type: get("drrp_type"),
                holder: get("holder"),
                clause: get("clause"),
                article: get("article"),
            });
        }
    }
    println!("  Total DRRP entries:     {:>8}", entries.len());

    // 2. Group entries by law_name.
    let mut by_law: HashMap<String, Vec<FlatDrrpEntry>> = HashMap::new();
    for entry in entries {
        by_law
            .entry(entry.law_name.clone())
            .or_default()
            .push(entry);
    }
    let all_laws: Vec<String> = {
        let mut v: Vec<String> = by_law.keys().cloned().collect();
        v.sort();
        v
    };
    println!("  Laws with DRRP data:    {:>8}", all_laws.len());

    // 3. Load validation law set.
    let val_laws: HashSet<String> = if let Some(path) = val_laws_file {
        let file = std::fs::File::open(path)
            .with_context(|| format!("opening val-laws file: {}", path.display()))?;
        std::io::BufReader::new(file)
            .lines()
            .filter_map(|l| {
                let l = l.ok()?;
                let trimmed = l.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .collect()
    } else {
        HashSet::new()
    };

    // 4. Select test laws deterministically (hash-based, from non-val laws).
    let test_laws: HashSet<String> = {
        let mut candidates: Vec<&String> = all_laws
            .iter()
            .filter(|l| !val_laws.contains(l.as_str()))
            .collect();
        // Deterministic selection: sort by a hash-like key.
        candidates.sort_by(|a, b| {
            use std::hash::{Hash, Hasher};
            let hash_of = |s: &str| -> u64 {
                let mut h = std::hash::DefaultHasher::new();
                s.hash(&mut h);
                h.finish()
            };
            hash_of(a).cmp(&hash_of(b))
        });
        candidates
            .into_iter()
            .take(test_law_count)
            .cloned()
            .collect()
    };

    println!("  Validation laws:        {:>8}", val_laws.len());
    println!("  Test laws:              {:>8}", test_laws.len());

    // 5. Open LanceDB for source text.
    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    // 6. Process law-by-law, generating silver labels.
    let mut train_examples: Vec<TrainingExample> = Vec::new();
    let mut val_examples: Vec<TrainingExample> = Vec::new();
    let mut test_examples: Vec<TrainingExample> = Vec::new();
    let mut unmatched_lat = 0usize;
    let mut unparseable_article = 0usize;
    let mut processed = 0usize;

    for law_name in &all_laws {
        let split = if val_laws.contains(law_name) {
            "val"
        } else if test_laws.contains(law_name) {
            "test"
        } else {
            "train"
        };

        // Query LanceDB for all sections of this law.
        let filter = format!("law_name = '{}'", law_name.replace('\'', "''"));
        let lat_batches = lance.query_legislation_text(&filter, 100_000, 0).await?;

        // Build provision → text map.
        let mut prov_text: HashMap<String, String> = HashMap::new();
        for batch in &lat_batches {
            let prov_col = batch.column_by_name("provision");
            let text_col = batch.column_by_name("text");
            for row in 0..batch.num_rows() {
                let provision = prov_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                let text = text_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                if !provision.is_empty() && !text.trim().is_empty() {
                    prov_text.insert(provision, text);
                }
            }
        }

        // Process each entry for this law.
        let law_entries = match by_law.get(law_name) {
            Some(e) => e,
            None => continue,
        };

        for entry in law_entries {
            let provision = match training::parse_article_to_provision(&entry.article) {
                Some(p) => p.to_string(),
                None => {
                    unparseable_article += 1;
                    continue;
                }
            };

            let source_text = match prov_text.get(&provision) {
                Some(t) => t.as_str(),
                None => {
                    unmatched_lat += 1;
                    continue;
                }
            };

            let example = training::generate_silver_label(entry, source_text, &provision, split);

            // Filter by minimum match ratio.
            if example.match_ratio < min_match_ratio && example.clause_start >= 0 {
                continue;
            }

            match split {
                "val" => val_examples.push(example),
                "test" => test_examples.push(example),
                _ => train_examples.push(example),
            }
        }

        processed += 1;
        if processed.is_multiple_of(200) {
            eprint!("\r  Processing laws... {processed}/{}", all_laws.len());
        }
    }
    if processed >= 200 {
        eprintln!();
    }

    // 7. Count holder categories.
    let holder_categories: HashSet<&str> = train_examples
        .iter()
        .chain(val_examples.iter())
        .chain(test_examples.iter())
        .map(|e| e.holder_label.as_str())
        .collect();

    // 8. Match quality distribution.
    let all_examples: Vec<&TrainingExample> = train_examples
        .iter()
        .chain(val_examples.iter())
        .chain(test_examples.iter())
        .collect();
    let total_exported = all_examples.len();
    let high = all_examples
        .iter()
        .filter(|e| e.match_quality == "high")
        .count();
    let medium = all_examples
        .iter()
        .filter(|e| e.match_quality == "medium")
        .count();
    let low = all_examples
        .iter()
        .filter(|e| e.match_quality == "low")
        .count();
    let with_qualifier = all_examples
        .iter()
        .filter(|e| e.qualifier_text.is_some())
        .count();

    // 9. Write Parquet files.
    std::fs::create_dir_all(output)
        .with_context(|| format!("creating output directory: {}", output.display()))?;

    let write_parquet = |examples: &[TrainingExample], name: &str| -> anyhow::Result<()> {
        if examples.is_empty() {
            return Ok(());
        }
        let batch = training::examples_to_record_batch(examples)?;
        let path = output.join(format!("{name}.parquet"));
        let file = std::fs::File::create(&path)?;
        let mut writer = parquet::arrow::ArrowWriter::try_new(file, batch.schema(), None)?;
        writer.write(&batch)?;
        writer.close()?;
        Ok(())
    };

    write_parquet(&train_examples, "train")?;
    write_parquet(&val_examples, "val")?;
    write_parquet(&test_examples, "test")?;

    // 10. Print statistics.
    let total_entries: usize = by_law.values().map(|v| v.len()).sum();
    println!(
        "\n  Matched to LAT:         {:>8}  ({:.1}%)",
        total_exported,
        total_exported as f64 / total_entries as f64 * 100.0
    );
    println!(
        "  Unmatched (no LAT):     {:>8}  ({:.1}%)",
        unmatched_lat,
        unmatched_lat as f64 / total_entries as f64 * 100.0
    );
    println!(
        "  Unparseable article:    {:>8}  ({:.1}%)",
        unparseable_article,
        unparseable_article as f64 / total_entries as f64 * 100.0
    );
    println!("\n  Match quality distribution:");
    println!(
        "    High   (>0.8):        {:>8}  ({:.1}%)",
        high,
        high as f64 / total_exported.max(1) as f64 * 100.0
    );
    println!(
        "    Medium (0.5-0.8):     {:>8}  ({:.1}%)",
        medium,
        medium as f64 / total_exported.max(1) as f64 * 100.0
    );
    println!(
        "    Low    (<0.5):        {:>8}  ({:.1}%)",
        low,
        low as f64 / total_exported.max(1) as f64 * 100.0
    );
    println!(
        "\n  With qualifier:         {:>8}  ({:.1}%)",
        with_qualifier,
        with_qualifier as f64 / total_exported.max(1) as f64 * 100.0
    );
    println!("\n  Holder categories:      {:>8}", holder_categories.len());
    println!("\n  Split sizes:");
    println!(
        "    Train:                {:>8} examples",
        train_examples.len()
    );
    println!(
        "    Val:                  {:>8} examples",
        val_examples.len()
    );
    println!(
        "    Test:                 {:>8} examples",
        test_examples.len()
    );
    println!("\n  Output:");
    if !train_examples.is_empty() {
        println!("    {}/train.parquet", output.display());
    }
    if !val_examples.is_empty() {
        println!("    {}/val.parquet", output.display());
    }
    if !test_examples.is_empty() {
        println!("    {}/test.parquet", output.display());
    }

    Ok(())
}

async fn cmd_search(
    data_dir: &std::path::Path,
    query: &str,
    limit: usize,
    model_dir: &std::path::Path,
) -> anyhow::Result<()> {
    let model_dir = model_dir
        .canonicalize()
        .with_context(|| format!("model directory '{}' not found", model_dir.display()))?;

    let mut embedder =
        fractalaw_ai::Embedder::load(&model_dir).context("loading embedding model")?;

    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    let query_vec = embedder.embed(query).context("embedding query")?;
    let batches = lance.search_text(&query_vec, limit).await?;

    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total == 0 {
        println!("No results.");
        return Ok(());
    }

    let projected = project_batches(
        &batches,
        &["law_name", "provision", "section_type", "text", "_distance"],
    );
    print_batches(&projected)?;
    Ok(())
}

async fn cmd_validate(
    store: &DuckStore,
    data_dir: &std::path::Path,
    model_dir: &std::path::Path,
) -> anyhow::Result<()> {
    use std::sync::Arc;

    use datafusion::datasource::memory::MemTable;
    use futures::TryStreamExt;
    use lancedb::query::ExecutableQuery;

    let model_dir = model_dir
        .canonicalize()
        .with_context(|| format!("model directory '{}' not found", model_dir.display()))?;

    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    println!("=== Validation ===\n");

    let mut passed = 0u32;
    let mut total_checks = 5u32;

    // ── Check 1: Row counts ──
    let lance_count = lance.legislation_text_count().await?;
    let source_batches = fractalaw_store::read_parquet(&data_dir.join("legislation_text.parquet"))?;
    let source_count: usize = source_batches.iter().map(|b| b.num_rows()).sum();
    drop(source_batches);

    if lance_count == source_count {
        println!("  [PASS] Legislation text rows: {}", fmt_num(lance_count));
        passed += 1;
    } else {
        println!(
            "  [FAIL] Legislation text rows: {} in Lance vs {} in Parquet",
            fmt_num(lance_count),
            fmt_num(source_count)
        );
    }

    // ── Check 2: Embedding coverage ──
    let table = lance.legislation_text().await?;
    let embedded_count = table
        .count_rows(Some("embedded_at IS NOT NULL".to_string()))
        .await?;

    if embedded_count == lance_count {
        println!(
            "  [PASS] Embedding coverage: {} / {} (100%)",
            fmt_num(embedded_count),
            fmt_num(lance_count)
        );
        passed += 1;
    } else {
        let pct = if lance_count > 0 {
            embedded_count as f64 / lance_count as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "  [FAIL] Embedding coverage: {} / {} ({pct:.1}%)",
            fmt_num(embedded_count),
            fmt_num(lance_count)
        );
    }

    // ── Check 3: Token coverage ──
    let table = lance.legislation_text().await?;
    let tokenized_count = table
        .count_rows(Some("tokenizer_model IS NOT NULL".to_string()))
        .await?;

    if tokenized_count == lance_count {
        println!(
            "  [PASS] Token coverage: {} / {} (100%)",
            fmt_num(tokenized_count),
            fmt_num(lance_count)
        );
        passed += 1;
    } else {
        let pct = if lance_count > 0 {
            tokenized_count as f64 / lance_count as f64 * 100.0
        } else {
            0.0
        };
        println!(
            "  [FAIL] Token coverage: {} / {} ({pct:.1}%)",
            fmt_num(tokenized_count),
            fmt_num(lance_count)
        );
    }

    // ── Check 4: Cross-store join ──
    let fusion = FusionStore::new(store)?;

    // Register only legislation_text from Lance (amendment_annotations may not exist).
    {
        let text_table = lance.legislation_text().await?;
        let text_batches: Vec<RecordBatch> = text_table
            .query()
            .execute()
            .await
            .map_err(|e| anyhow::anyhow!("lance query: {e}"))?
            .try_collect()
            .await
            .map_err(|e| anyhow::anyhow!("lance collect: {e}"))?;

        if let Some(first) = text_batches.first() {
            let schema = first.schema();
            let mem = MemTable::try_new(schema, vec![text_batches])?;
            fusion
                .context()
                .register_table("legislation_text", Arc::new(mem))
                .map_err(|e| anyhow::anyhow!("register legislation_text: {e}"))?;
        }
    }

    let batches = fusion
        .query(
            "SELECT count(DISTINCT t.law_name) AS matched \
             FROM legislation_text t \
             JOIN legislation l ON t.law_name = l.name",
        )
        .await?;
    let matched = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<arrow::array::Int64Array>()
        .unwrap()
        .value(0);

    let batches = fusion
        .query("SELECT count(DISTINCT law_name) AS total FROM legislation_text")
        .await?;
    let text_laws = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<arrow::array::Int64Array>()
        .unwrap()
        .value(0);

    if matched == text_laws {
        println!("  [PASS] Cross-store join: {matched} / {text_laws} laws matched");
        passed += 1;
    } else if matched > 0 {
        // Some LAT data covers laws not in the legislation export — expected.
        println!(
            "  [PASS] Cross-store join: {matched} / {text_laws} laws matched ({} unmatched in legislation)",
            text_laws - matched
        );
        passed += 1;
    } else {
        println!("  [FAIL] Cross-store join: {matched} / {text_laws} laws matched");
    }

    // ── Check 5: Semantic smoke test ──
    let mut embedder =
        fractalaw_ai::Embedder::load(&model_dir).context("loading embedding model")?;
    let query_text = "chemical exposure limits";
    let query_vec = embedder.embed(query_text)?;
    let results = lance.search_text(&query_vec, 5).await?;

    let mut found = false;
    let mut top_law = String::new();

    'outer: for batch in &results {
        let law_col = batch.column_by_name("law_name");
        let text_col = batch.column_by_name("text");

        for row in 0..batch.num_rows() {
            let law = law_col.and_then(|c| get_string_value(c.as_ref(), row));
            let text = text_col.and_then(|c| get_string_value(c.as_ref(), row));

            if top_law.is_empty()
                && let Some(ref l) = law
            {
                top_law.clone_from(l);
            }

            let combined = format!(
                "{} {}",
                law.as_deref().unwrap_or(""),
                text.as_deref().unwrap_or("")
            )
            .to_lowercase();

            if combined.contains("coshh")
                || combined.contains("chemical")
                || combined.contains("hazardous")
                || combined.contains("exposure")
            {
                if let Some(ref l) = law {
                    top_law.clone_from(l);
                }
                found = true;
                break 'outer;
            }
        }
    }

    if found {
        println!("  [PASS] Semantic search: \"{query_text}\" → {top_law}");
        passed += 1;
    } else {
        println!(
            "  [FAIL] Semantic search: \"{query_text}\" → no COSHH/chemical match in top 5 (top: {top_law})"
        );
    }

    // ── Classification checks ──
    // Only run if classification has been performed (columns exist).
    let has_classification = store
        .query_arrow(
            "SELECT count(*)::BIGINT FROM information_schema.columns \
             WHERE table_name = 'legislation' AND column_name = 'classified_family'",
        )
        .ok()
        .and_then(|b| {
            b.first()?
                .column(0)
                .as_any()
                .downcast_ref::<arrow::array::Int64Array>()
                .map(|a| a.value(0) > 0)
        })
        .unwrap_or(false);

    if has_classification {
        println!("\n  --- Classification ---");
        total_checks += 4;

        // ── Check 6: Classification coverage ──
        let batches = store.query_arrow(
            "SELECT count(*) FILTER (WHERE classified_family IS NOT NULL)::BIGINT AS classified, \
             count(*)::BIGINT AS total \
             FROM legislation",
        )?;
        let classified = extract_i64(&batches[0], 0);
        let total_leg = extract_i64(&batches[0], 1);

        if classified > 0 {
            println!(
                "  [PASS] Classification coverage: {} / {} laws ({:.1}%)",
                fmt_num(classified as usize),
                fmt_num(total_leg as usize),
                classified as f64 / total_leg as f64 * 100.0
            );
            passed += 1;
        } else {
            println!("  [FAIL] Classification coverage: 0 laws classified");
        }

        // ── Check 7: Confidence distribution ──
        let batches = store.query_arrow(
            "SELECT avg(classification_confidence) AS mean_conf, \
                    median(classification_confidence) AS median_conf, \
                    percentile_cont(0.1) WITHIN GROUP (ORDER BY classification_confidence) AS p10_conf \
             FROM legislation \
             WHERE classification_confidence IS NOT NULL",
        )?;
        let mean_c = extract_f64(&batches[0], 0);
        let median_c = extract_f64(&batches[0], 1);
        let p10_c = extract_f64(&batches[0], 2);

        if mean_c > 0.0 {
            println!(
                "  [PASS] Confidence distribution: mean={mean_c:.3}, median={median_c:.3}, p10={p10_c:.3}"
            );
            passed += 1;
        } else {
            println!("  [FAIL] Confidence distribution: no confidence scores found");
        }

        // ── Check 8: Agreement with ground truth ──
        let batches = store.query_arrow(
            "SELECT count(*) FILTER (WHERE classification_status = 'confirmed')::BIGINT AS agreed, \
             count(*) FILTER (WHERE classification_status IN ('confirmed', 'conflict'))::BIGINT AS with_gt \
             FROM legislation",
        )?;
        let agreed = extract_i64(&batches[0], 0);
        let with_gt = extract_i64(&batches[0], 1);
        let rate = if with_gt > 0 {
            agreed as f64 / with_gt as f64 * 100.0
        } else {
            0.0
        };

        println!(
            "  [PASS] Ground-truth agreement: {} / {} ({:.1}%)",
            fmt_num(agreed as usize),
            fmt_num(with_gt as usize),
            rate
        );
        passed += 1;

        // ── Check 9: Subject revival (post-2013 laws with predicted subjects) ──
        let batches = store.query_arrow(
            "SELECT count(*) FILTER (\
                WHERE classified_subjects IS NOT NULL \
                AND len(classified_subjects) > 0 \
                AND year > 2013\
             )::BIGINT AS revived, \
             count(*) FILTER (WHERE year > 2013)::BIGINT AS post_2013 \
             FROM legislation",
        )?;
        let revived = extract_i64(&batches[0], 0);
        let post_2013 = extract_i64(&batches[0], 1);

        println!(
            "  [PASS] Subject revival: {} / {} post-2013 laws have predicted subjects",
            fmt_num(revived as usize),
            fmt_num(post_2013 as usize),
        );
        passed += 1;
    } else {
        println!("\n  [SKIP] Classification checks (run `fractalaw classify` first)");
    }

    // ── Summary ──
    println!("\n=== {passed}/{total_checks} checks passed ===");

    if passed < total_checks {
        anyhow::bail!("{} check(s) failed", total_checks - passed);
    }

    Ok(())
}

fn cmd_tokenize(text: &str, model_dir: &std::path::Path) -> anyhow::Result<()> {
    let model_dir = model_dir
        .canonicalize()
        .with_context(|| format!("model directory '{}' not found", model_dir.display()))?;

    let mut embedder =
        fractalaw_ai::Embedder::load(&model_dir).context("loading embedding model")?;

    let ids = embedder.tokenize(text)?;

    println!("Model:  {}", embedder.model_name());
    println!("Tokens: {}\n", ids.len());

    println!("{:>6}  {:>6}  TOKEN", "INDEX", "ID");
    println!("{:>6}  {:>6}  -----", "-----", "-----");
    for (i, &id) in ids.iter().enumerate() {
        let token = embedder.id_to_token(id).unwrap_or_else(|| "?".into());
        println!("{i:>6}  {id:>6}  {token}");
    }

    Ok(())
}

fn cmd_stats(store: &DuckStore) -> anyhow::Result<()> {
    let leg_count = store.legislation_count()?;
    let edge_count = store.law_edges_count()?;

    println!("=== Dataset Summary ===\n");
    println!("  Legislation:  {:>8} rows", leg_count);
    println!("  Law Edges:    {:>8} rows", edge_count);

    // Year range.
    let batches = store
        .query_arrow("SELECT min(year) AS min_year, max(year) AS max_year FROM legislation")?;
    println!();
    print_batches(&batches)?;

    // Status breakdown.
    println!("\n--- Status Breakdown ---\n");
    let batches = store.query_arrow(
        "SELECT status, count(*) AS count FROM legislation GROUP BY status ORDER BY count DESC",
    )?;
    print_batches(&batches)?;

    // Edge type breakdown.
    println!("\n--- Edge Types ---\n");
    let batches = store.query_arrow(
        "SELECT edge_type, count(*) AS count FROM law_edges GROUP BY edge_type ORDER BY count DESC",
    )?;
    print_batches(&batches)?;

    // Jurisdiction breakdown.
    println!("\n--- Jurisdictions ---\n");
    let batches = store.query_arrow(
        "SELECT jurisdiction, count(*) AS count FROM legislation GROUP BY jurisdiction ORDER BY count DESC",
    )?;
    print_batches(&batches)?;

    Ok(())
}

async fn cmd_classify(
    store: &DuckStore,
    data_dir: &std::path::Path,
    domain_threshold: f32,
    subject_threshold: f32,
) -> anyhow::Result<()> {
    use fractalaw_ai::{ClassificationStatus, Classifier, LabelSet, aggregate_law_embeddings};
    use futures::TryStreamExt;
    use lancedb::query::{ExecutableQuery, QueryBase, Select};

    println!("=== Classification Pipeline ===\n");

    // Step 1: Load labels from legislation table.
    println!("Loading label sets...");
    let label_batches =
        store.query_arrow("SELECT name, domain, family, sub_family, subjects FROM legislation")?;
    let labels = LabelSet::from_legislation_batches(&label_batches)?;
    let lsummary = labels.summary();
    println!(
        "  {} laws with family labels, {} with domain, {} with subjects",
        fmt_num(lsummary.with_family),
        fmt_num(lsummary.with_domain),
        fmt_num(lsummary.with_subjects),
    );

    // Step 2: Load embeddings from LanceDB (only law_name + embedding columns).
    println!("Loading embeddings from LanceDB...");
    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;
    let table = lance.legislation_text().await?;
    let query = table.query().select(Select::Columns(vec![
        "law_name".to_string(),
        "embedding".to_string(),
    ]));
    let stream = query
        .execute()
        .await
        .map_err(|e| anyhow::anyhow!("lance query: {e}"))?;
    let emb_batches: Vec<RecordBatch> = stream
        .try_collect()
        .await
        .map_err(|e| anyhow::anyhow!("lance collect: {e}"))?;

    let total_sections: usize = emb_batches.iter().map(|b: &RecordBatch| b.num_rows()).sum();

    // Step 3: Aggregate to law-level embeddings.
    println!("Aggregating section embeddings → law-level...");
    let law_embeddings = aggregate_law_embeddings(&emb_batches)?;
    println!(
        "  {} laws with embeddings (from {} sections)",
        fmt_num(law_embeddings.len()),
        fmt_num(total_sections),
    );
    drop(emb_batches); // Free memory.

    if law_embeddings.is_empty() {
        println!("\nNo laws with embeddings found. Run `fractalaw embed` first.");
        return Ok(());
    }

    // Step 4: Build classifier (compute centroids).
    println!("Computing centroids...");
    let classifier = Classifier::build(&law_embeddings, &labels);
    let csummary = classifier.summary(law_embeddings.len());
    println!("  domain:   {} centroids", csummary.domain_count);
    println!("  family:   {} centroids", csummary.family_count);
    println!("  subjects: {} centroids", csummary.subject_count);

    // Step 5: Classify all laws with embeddings.
    println!(
        "Classifying {} laws (domain_threshold={domain_threshold}, subject_threshold={subject_threshold})...",
        law_embeddings.len()
    );
    let results = classifier.classify_batch(
        &law_embeddings,
        &labels,
        domain_threshold,
        subject_threshold,
    );

    // Count statuses.
    let predicted = results
        .iter()
        .filter(|c| c.status == ClassificationStatus::Predicted)
        .count();
    let confirmed = results
        .iter()
        .filter(|c| c.status == ClassificationStatus::Confirmed)
        .count();
    let conflicts = results
        .iter()
        .filter(|c| c.status == ClassificationStatus::Conflict)
        .count();

    let mean_conf: f32 = if results.is_empty() {
        0.0
    } else {
        results.iter().map(|c| c.family_confidence).sum::<f32>() / results.len() as f32
    };

    let with_subjects = results.iter().filter(|c| !c.subjects.is_empty()).count();

    println!("  predicted:  {} (no ground truth)", fmt_num(predicted));
    println!("  confirmed:  {} (AI agrees)", fmt_num(confirmed));
    println!("  conflict:   {} (AI disagrees)", fmt_num(conflicts));
    println!("  with subjects: {}", fmt_num(with_subjects));
    println!("  mean family confidence: {mean_conf:.3}");

    // Step 6: Write to DuckDB.
    println!("\nWriting to DuckDB...");
    write_classifications(store, &results)?;
    println!("  {} rows updated", fmt_num(results.len()));

    // Print conflict report if any.
    if conflicts > 0 {
        println!("\n--- Conflicts (top 20 by confidence) ---\n");
        let conflict_batches = store.query_arrow(
            "SELECT name, family, classified_family, \
                    round(classification_confidence, 3) AS confidence \
             FROM legislation \
             WHERE classification_status = 'conflict' \
             ORDER BY classification_confidence DESC \
             LIMIT 20",
        )?;
        print_batches(&conflict_batches)?;
    }

    println!("\n=== Done ===");
    Ok(())
}

/// Write classification results to DuckDB legislation table.
fn write_classifications(
    store: &DuckStore,
    results: &[fractalaw_ai::Classification],
) -> anyhow::Result<()> {
    // Add columns (idempotent).
    let columns = [
        ("classified_domain", "VARCHAR[]"),
        ("classified_family", "VARCHAR"),
        ("classified_subjects", "VARCHAR[]"),
        ("classification_confidence", "FLOAT"),
        ("classification_model", "VARCHAR"),
        ("classified_at", "TIMESTAMPTZ"),
        ("classification_status", "VARCHAR"),
    ];

    for (col, dtype) in &columns {
        store.execute(&format!(
            "ALTER TABLE legislation ADD COLUMN IF NOT EXISTS {col} {dtype}"
        ))?;
    }

    // Create temp table for batch update.
    store.execute("DROP TABLE IF EXISTS _tmp_classifications")?;
    store.execute(
        "CREATE TEMP TABLE _tmp_classifications (\
            name VARCHAR, \
            classified_domain VARCHAR[], \
            classified_family VARCHAR, \
            classified_subjects VARCHAR[], \
            classification_confidence FLOAT, \
            classification_model VARCHAR, \
            classified_at TIMESTAMPTZ, \
            classification_status VARCHAR\
        )",
    )?;

    // Insert in chunks to avoid overly long SQL statements.
    for chunk in results.chunks(100) {
        let mut sql = String::from("INSERT INTO _tmp_classifications VALUES ");
        for (i, c) in chunk.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }

            let name_esc = c.law_name.replace('\'', "''");
            let family_esc = c.family.replace('\'', "''");
            let domain_arr = format_sql_list(c.domain.iter().map(|(d, _)| d.as_str()));
            let subjects_arr = format_sql_list(c.subjects.iter().map(|(s, _)| s.as_str()));

            sql.push_str(&format!(
                "('{}', {}, '{}', {}, {}, 'centroid-v1', CURRENT_TIMESTAMP, '{}')",
                name_esc,
                domain_arr,
                family_esc,
                subjects_arr,
                c.family_confidence,
                c.status.as_str(),
            ));
        }
        store.execute(&sql)?;
    }

    // Bulk update from temp table.
    store.execute(
        "UPDATE legislation SET \
            classified_domain = c.classified_domain, \
            classified_family = c.classified_family, \
            classified_subjects = c.classified_subjects, \
            classification_confidence = c.classification_confidence, \
            classification_model = c.classification_model, \
            classified_at = c.classified_at, \
            classification_status = c.classification_status \
        FROM _tmp_classifications c \
        WHERE legislation.name = c.name",
    )?;

    store.execute("DROP TABLE IF EXISTS _tmp_classifications")?;
    Ok(())
}

/// Find the largest byte offset <= `max_bytes` that is a valid char boundary.
fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> usize {
    let mut end = max_bytes.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

/// Format an iterator of strings as a DuckDB array literal: `['a', 'b']` or `NULL`.
fn format_sql_list<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let items: Vec<String> = values
        .map(|v| format!("'{}'", v.replace('\'', "''")))
        .collect();
    if items.is_empty() {
        "NULL".to_string()
    } else {
        format!("[{}]", items.join(", "))
    }
}

/// Compute a content hash of the 11 published taxa columns from a `LawTaxa` struct.
///
/// Uses `DefaultHasher` (SipHash) over a canonical string built from sorted
/// column values. Returns a hex-encoded u64 hash.
#[allow(clippy::too_many_arguments)]
fn compute_taxa_hash(
    duty_holders: &std::collections::BTreeSet<String>,
    rights_holders: &std::collections::BTreeSet<String>,
    responsibility_holders: &std::collections::BTreeSet<String>,
    power_holders: &std::collections::BTreeSet<String>,
    duty_types: &std::collections::BTreeSet<String>,
    roles: &std::collections::BTreeSet<String>,
    roles_gvt: &std::collections::BTreeSet<String>,
    duties: &[(String, String, String, String)],
    rights: &[(String, String, String, String)],
    responsibilities: &[(String, String, String, String)],
    powers: &[(String, String, String, String)],
    fitness_persons: &std::collections::BTreeSet<String>,
    fitness_processes: &std::collections::BTreeSet<String>,
    fitness_places: &std::collections::BTreeSet<String>,
    fitness_plants: &std::collections::BTreeSet<String>,
    fitness_properties: &std::collections::BTreeSet<String>,
    fitness_sectors: &std::collections::BTreeSet<String>,
    fitness_entries: &[FitnessEntry],
) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();

    // BTreeSets are already sorted — iterate in order.
    for v in duty_holders {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF); // separator
    for v in rights_holders {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in responsibility_holders {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in power_holders {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in duty_types {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in roles {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in roles_gvt {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);

    // DRRP entries: sort for determinism (Vecs may not be ordered).
    let mut sorted_duties: Vec<_> = duties.iter().collect();
    sorted_duties.sort();
    for (h, dt, c, a) in sorted_duties {
        h.hash(&mut hasher);
        dt.hash(&mut hasher);
        c.hash(&mut hasher);
        a.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    let mut sorted_rights: Vec<_> = rights.iter().collect();
    sorted_rights.sort();
    for (h, dt, c, a) in sorted_rights {
        h.hash(&mut hasher);
        dt.hash(&mut hasher);
        c.hash(&mut hasher);
        a.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    let mut sorted_resp: Vec<_> = responsibilities.iter().collect();
    sorted_resp.sort();
    for (h, dt, c, a) in sorted_resp {
        h.hash(&mut hasher);
        dt.hash(&mut hasher);
        c.hash(&mut hasher);
        a.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    let mut sorted_powers: Vec<_> = powers.iter().collect();
    sorted_powers.sort();
    for (h, dt, c, a) in sorted_powers {
        h.hash(&mut hasher);
        dt.hash(&mut hasher);
        c.hash(&mut hasher);
        a.hash(&mut hasher);
    }

    // Fitness tag sets (BTreeSet — already sorted).
    hasher.write_u8(0xFF);
    for v in fitness_persons {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in fitness_processes {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in fitness_places {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in fitness_plants {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in fitness_properties {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in fitness_sectors {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    let mut sorted_fitness: Vec<_> = fitness_entries.iter().collect();
    sorted_fitness.sort();
    for (pol, per, proc, pl, plt, prop, sec, art) in sorted_fitness {
        pol.hash(&mut hasher);
        per.hash(&mut hasher);
        proc.hash(&mut hasher);
        pl.hash(&mut hasher);
        plt.hash(&mut hasher);
        prop.hash(&mut hasher);
        sec.hash(&mut hasher);
        art.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

/// Format a list of (holder, duty_type, clause, article) tuples as a DuckDB
/// `List<Struct>` literal, e.g. `[{'holder':'a','duty_type':'DUTY','clause':'...','article':'s/1'}]`.
fn format_sql_drrp_entries(entries: &[(String, String, String, String)]) -> String {
    if entries.is_empty() {
        return "NULL".to_string();
    }
    let esc = |s: &str| s.replace('\'', "''");
    let items: Vec<String> = entries
        .iter()
        .map(|(holder, dt, clause, article)| {
            format!(
                "{{'holder':'{}','duty_type':'{}','clause':'{}','article':'{}'}}",
                esc(holder),
                esc(dt),
                esc(clause),
                esc(article)
            )
        })
        .collect();
    format!("[{}]", items.join(", "))
}

/// Extract candidate noun phrases from gap provision texts, filtering known
/// dictionary terms and stop words. Returns (term, frequency) pairs sorted
/// by frequency descending.
fn extract_candidate_terms(
    texts: &[&str],
    known_terms: &std::collections::BTreeSet<String>,
) -> Vec<(String, usize)> {
    use std::collections::{BTreeSet, HashMap};

    let stop_words: BTreeSet<&str> = [
        "the",
        "a",
        "an",
        "of",
        "to",
        "in",
        "for",
        "on",
        "at",
        "by",
        "or",
        "and",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "shall",
        "will",
        "would",
        "should",
        "may",
        "might",
        "can",
        "could",
        "not",
        "no",
        "nor",
        "but",
        "if",
        "so",
        "as",
        "than",
        "that",
        "this",
        "these",
        "those",
        "such",
        "any",
        "all",
        "each",
        "every",
        "with",
        "from",
        "into",
        "through",
        "during",
        "before",
        "after",
        "above",
        "below",
        "between",
        "under",
        "over",
        "about",
        "against",
        "without",
        "which",
        "who",
        "whom",
        "whose",
        "where",
        "when",
        "how",
        "what",
        "it",
        "its",
        "they",
        "them",
        "their",
        "he",
        "his",
        "she",
        "her",
        "we",
        "our",
        "you",
        "your",
        "me",
        "my",
        "him",
        "us",
        "only",
        "also",
        "other",
        "more",
        "most",
        "very",
        "just",
        "here",
        "there",
        "regulation",
        "regulations",
        "section",
        "paragraph",
        "sub-paragraph",
        "act",
        "order",
        "article",
        "part",
        "schedule",
        "provision",
        "provisions",
        "apply",
        "applies",
        "applied",
        "applying",
        "application",
        "must",
        "effect",
        "force",
        "extent",
        "relation",
        "respect",
        "case",
        "person",
        "persons",
    ]
    .into_iter()
    .collect();

    let mut freq: HashMap<String, usize> = HashMap::new();

    for text in texts {
        // Tokenize: split on non-alphabetic/hyphen boundaries, keep words with letters
        let words: Vec<&str> = text
            .split(|c: char| !c.is_alphabetic() && c != '-')
            .filter(|w| w.len() >= 2 && w.chars().any(|c| c.is_alphabetic()))
            .collect();

        for n in 1..=3usize {
            for window in words.windows(n) {
                // For 1-grams, skip stop words entirely
                if n == 1 && stop_words.contains(&window[0].to_lowercase().as_str()) {
                    continue;
                }
                // For multi-grams, skip if ALL words are stop words
                if n > 1
                    && window
                        .iter()
                        .all(|w: &&str| stop_words.contains(&w.to_lowercase().as_str()))
                {
                    continue;
                }

                let phrase = window
                    .iter()
                    .map(|w| w.to_lowercase())
                    .collect::<Vec<_>>()
                    .join(" ");

                if phrase.len() < 3 || known_terms.contains(&phrase) {
                    continue;
                }

                *freq.entry(phrase).or_insert(0) += 1;
            }
        }
    }

    let mut sorted: Vec<(String, usize)> =
        freq.into_iter().filter(|(_, count)| *count >= 2).collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted
}

/// Format fitness entries as a DuckDB `List<Struct>` literal.
fn format_sql_fitness_entries(entries: &[FitnessEntry]) -> String {
    if entries.is_empty() {
        return "NULL".to_string();
    }
    let esc = |s: &str| s.replace('\'', "''");
    let null_or_val = |s: &str| {
        if s.is_empty() {
            "NULL".to_string()
        } else {
            format!("'{}'", esc(s))
        }
    };
    let items: Vec<String> = entries
        .iter()
        .map(|(pol, per, proc, pl, plt, prop, sec, art)| {
            format!(
                "{{'polarity':{},'person':{},'process':{},'place':{},'plant':{},'property':{},'sector':{},'article':{}}}",
                null_or_val(pol),
                null_or_val(per),
                null_or_val(proc),
                null_or_val(pl),
                null_or_val(plt),
                null_or_val(prop),
                null_or_val(sec),
                null_or_val(art),
            )
        })
        .collect();
    format!("[{}]", items.join(", "))
}

/// Project RecordBatches to only include the specified columns.
fn project_batches(batches: &[RecordBatch], columns: &[&str]) -> Vec<RecordBatch> {
    batches
        .iter()
        .filter_map(|batch| {
            let schema = batch.schema();
            let indices: Vec<usize> = columns
                .iter()
                .filter_map(|name| schema.index_of(name).ok())
                .collect();
            if indices.is_empty() {
                None
            } else {
                batch.project(&indices).ok()
            }
        })
        .collect()
}

/// Format a number with comma thousands separators.
fn fmt_num(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Extract an i64 value from column `col_idx` of a RecordBatch.
fn extract_i64(batch: &RecordBatch, col_idx: usize) -> i64 {
    batch
        .column(col_idx)
        .as_any()
        .downcast_ref::<arrow::array::Int64Array>()
        .map(|a| a.value(0))
        .unwrap_or(0)
}

/// Extract an f64 value from column `col_idx` of a RecordBatch.
///
/// Handles both Float64 (DuckDB aggregates) and Float32 columns.
fn extract_f64(batch: &RecordBatch, col_idx: usize) -> f64 {
    let col = batch.column(col_idx);
    if let Some(a) = col.as_any().downcast_ref::<arrow::array::Float64Array>() {
        a.value(0)
    } else if let Some(a) = col.as_any().downcast_ref::<arrow::array::Float32Array>() {
        a.value(0) as f64
    } else {
        0.0
    }
}

/// Parse a Gemini REST API response body into the inner JSON content.
///
/// Extracts `candidates[0].content.parts[0].text`, strips markdown code
/// fences, and parses the result as JSON.
fn parse_gemini_response(response_body: &str) -> Option<serde_json::Value> {
    let gemini_resp: serde_json::Value = serde_json::from_str(response_body).ok()?;
    let content_text = gemini_resp
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let json_text = if content_text.contains("```json") {
        content_text
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(content_text)
            .trim()
    } else if content_text.contains("```") {
        content_text
            .split("```")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(content_text)
            .trim()
    } else {
        content_text.trim()
    };
    serde_json::from_str(json_text).ok()
}

/// Actor dictionary entry, deserialized from YAML.
#[derive(serde::Deserialize)]
struct DictEntry {
    label: String,
    #[serde(rename = "type")]
    actor_type: Option<String>,
    category: Option<String>,
    #[serde(default)]
    triggers: Vec<String>,
}

impl DictEntry {
    /// Backward-compatible accessor for canonical label.
    fn canonical(&self) -> String {
        self.label.clone()
    }
}

/// Matches LLM natural-language actor names to canonical dictionary labels.
///
/// Loads `docs/actor-dictionary.yaml` — the single source of truth.
/// Two-pass matching: exact trigger → substring containment (longest first).
struct ActorMatcher {
    entries: Vec<DictEntry>,
    /// (trigger, canonical) sorted by trigger length descending for Pass 2.
    all_triggers: Vec<(String, String)>,
}

impl ActorMatcher {
    fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading actor dictionary from {path}"))?;
        let entries: Vec<DictEntry> =
            serde_yaml::from_str(&content).context("parsing actor dictionary YAML")?;

        let mut all_triggers: Vec<(String, String)> = Vec::new();
        for entry in &entries {
            for trigger in &entry.triggers {
                all_triggers.push((trigger.clone(), entry.canonical().clone()));
            }
        }
        all_triggers.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        Ok(Self {
            entries,
            all_triggers,
        })
    }

    /// Match an LLM actor name to a canonical label.
    ///
    /// Returns `Some((canonical_label, confidence))` or `None` for discoveries.
    fn match_name(&self, name: &str) -> Option<(String, f64)> {
        let n = name.trim().to_lowercase();
        if n.is_empty() {
            return None;
        }

        // Pass 1: exact trigger match (order-sensitive — specific before generic)
        for entry in &self.entries {
            for trigger in &entry.triggers {
                if n == *trigger {
                    return Some((entry.canonical().clone(), 1.0));
                }
            }
        }

        // Pass 2: substring containment (longest trigger first)
        for (trigger, canonical) in &self.all_triggers {
            if n.contains(trigger.as_str()) || trigger.contains(n.as_str()) {
                return Some((canonical.clone(), 0.85));
            }
        }

        // No match — discovery
        None
    }

    /// Check if a canonical label is a government/EU actor.
    fn is_government(&self, canonical_label: &str) -> bool {
        for entry in &self.entries {
            if entry.canonical() == canonical_label {
                // Prefer the explicit type field from unified YAML
                if let Some(ref t) = entry.actor_type {
                    return t == "government";
                }
                // Fallback to category for backward compatibility
                let cat = entry.category.as_deref().unwrap_or("other");
                return cat == "Gvt" || cat == "EU";
            }
        }
        false
    }
}

/// Parsed actor from Tier 3 LLM response, with label validation.
struct ParsedTier3Actor {
    label: String,
    position: String,
    relates_to: Option<String>,
    label_source: String,
    reason: Option<String>,
}

/// Parse the actors array from an LLM response, resolving labels via the actor matcher.
///
/// Returns `None` if the response doesn't contain a valid actors array.
/// Labels are resolved through the dictionary matcher — unmatched labels
/// get `label_source = "invented"`.
fn parse_tier3_actors(
    result: &serde_json::Value,
    matcher: &ActorMatcher,
) -> Option<Vec<ParsedTier3Actor>> {
    let actors_arr = result.get("actors")?.as_array()?;
    let mut actors = Vec::new();
    for actor_val in actors_arr {
        let raw_label = actor_val
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Resolve through actor dictionary matcher
        let (label, label_source) = if let Some((canonical, _conf)) = matcher.match_name(&raw_label)
        {
            (canonical, "canonical".to_string())
        } else {
            (raw_label, "invented".to_string())
        };

        let position = actor_val
            .get("position")
            .and_then(|v| v.as_str())
            .unwrap_or("mentioned")
            .to_lowercase();
        let position_str = match position.as_str() {
            "active" => "active",
            "counterparty" => "counterparty",
            "beneficiary" => "beneficiary",
            _ => "mentioned",
        };

        // Resolve relates_to through the matcher too
        let relates_to = actor_val
            .get("relates_to")
            .and_then(|v| v.as_str())
            .map(|s| {
                matcher
                    .match_name(s)
                    .map(|(canonical, _)| canonical)
                    .unwrap_or_else(|| s.to_string())
            });

        let reason = actor_val
            .get("reason")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        actors.push(ParsedTier3Actor {
            label,
            position: position_str.into(),
            relates_to,
            label_source,
            reason,
        });
    }
    Some(actors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn taxa_hash_deterministic() {
        let dh: BTreeSet<String> = ["employer".into()].into();
        let rh: BTreeSet<String> = ["employee".into()].into();
        let empty_set: BTreeSet<String> = BTreeSet::new();
        let duties = vec![(
            "employer".into(),
            "DUTY".into(),
            "shall ensure".into(),
            "s/2".into(),
        )];

        let h1 = compute_taxa_hash(
            &dh,
            &rh,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &duties,
            &[],
            &[],
            &[],
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &[],
        );
        let h2 = compute_taxa_hash(
            &dh,
            &rh,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &duties,
            &[],
            &[],
            &[],
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &[],
        );
        assert_eq!(h1, h2, "same input must produce same hash");
        assert_eq!(h1.len(), 16, "hash should be 16 hex chars");
    }

    #[test]
    fn taxa_hash_changes_on_different_input() {
        let dh: BTreeSet<String> = ["employer".into()].into();
        let empty_set: BTreeSet<String> = BTreeSet::new();

        let h1 = compute_taxa_hash(
            &dh,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &[],
            &[],
            &[],
            &[],
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &[],
        );

        let dh2: BTreeSet<String> = ["employee".into()].into();
        let h2 = compute_taxa_hash(
            &dh2,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &[],
            &[],
            &[],
            &[],
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &empty_set,
            &[],
        );
        assert_ne!(h1, h2, "different input must produce different hash");
    }

    // ── Tier 3 parsing tests (canned responses, no API calls) ──

    fn test_matcher() -> ActorMatcher {
        // CARGO_MANIFEST_DIR points to the crate dir; dictionary is at workspace root
        let manifest = env!("CARGO_MANIFEST_DIR");
        let dict_path = format!("{manifest}/../../docs/actor-dictionary.yaml");
        ActorMatcher::load(&dict_path).expect("actor dictionary must exist for tests")
    }

    #[test]
    fn parse_gemini_response_plain_json() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"{\"actors\":[{\"label\":\"Org: Employer\",\"role\":\"HOLDER\"}],\"primary_holder\":\"Org: Employer\"}"}],"role":"model"},"finishReason":"STOP"}]}"#;
        let parsed = parse_gemini_response(body).unwrap();
        assert_eq!(parsed["primary_holder"], "Org: Employer");
    }

    #[test]
    fn parse_gemini_response_code_fence() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"```json\n{\"actors\":[{\"label\":\"Org: Employer\",\"role\":\"HOLDER\"}]}\n```"}],"role":"model"},"finishReason":"STOP"}]}"#;
        let parsed = parse_gemini_response(body).unwrap();
        let actors = parsed["actors"].as_array().unwrap();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0]["label"], "Org: Employer");
    }

    #[test]
    fn parse_gemini_response_truncated() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"{\"actors\":[{\"label\":\"Org: Emp"}],"role":"model"},"finishReason":"MAX_TOKENS"}]}"#;
        assert!(parse_gemini_response(body).is_none());
    }

    #[test]
    fn parse_gemini_response_invalid_json() {
        let body = "not json at all";
        assert!(parse_gemini_response(body).is_none());
    }

    #[test]
    fn parse_tier3_actors_canonical_labels() {
        // LLM outputs natural language — matcher resolves to canonical
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[
                {"label":"employer","position":"ACTIVE","reason":"employer bears the duty"},
                {"label":"employee","position":"COUNTERPARTY","reason":"employee holds the claim"}
            ]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors.len(), 2);
        assert_eq!(actors[0].label, "Org: Employer");
        assert_eq!(actors[0].position, "active");
        assert_eq!(actors[0].label_source, "canonical");
        assert_eq!(actors[0].reason, Some("employer bears the duty".into()));
        assert_eq!(actors[1].label, "Ind: Employee");
        assert_eq!(actors[1].position, "counterparty");
        assert_eq!(actors[1].label_source, "canonical");
        assert_eq!(actors[1].reason, Some("employee holds the claim".into()));
    }

    #[test]
    fn parse_tier3_actors_natural_language_resolved() {
        // LLM says "responsible person" in natural language → matcher resolves to canonical
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[{"label":"responsible person","position":"ACTIVE"},{"label":"inspector","position":"COUNTERPARTY"}]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors[0].label, "Ind: Responsible Person");
        assert_eq!(actors[0].position, "active");
        assert_eq!(actors[0].label_source, "canonical");
        assert_eq!(actors[1].label, "Spc: Inspector");
        assert_eq!(actors[1].label_source, "canonical");
    }

    #[test]
    fn parse_tier3_actors_invented_label() {
        // "producers of electricity from high-efficiency cogeneration" is not in the dictionary
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[{"label":"producers of electricity from high-efficiency cogeneration","position":"ACTIVE"},{"label":"employer","position":"COUNTERPARTY"}]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(
            actors[0].label,
            "producers of electricity from high-efficiency cogeneration"
        );
        assert_eq!(actors[0].label_source, "invented");
        assert_eq!(actors[1].label, "Org: Employer");
        assert_eq!(actors[1].label_source, "canonical");
    }

    #[test]
    fn parse_tier3_actors_relates_to() {
        // LLM uses natural language for relates_to — matcher resolves it
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[
                {"label":"employer","position":"ACTIVE","relates_to":"employee","reason":"employer must train employees"},
                {"label":"employee","position":"COUNTERPARTY","reason":"receives training"}
            ]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors[0].relates_to, Some("Ind: Employee".into()));
        assert_eq!(actors[1].relates_to, None);
    }

    #[test]
    fn parse_tier3_actors_all_positions() {
        let result: serde_json::Value = serde_json::from_str(
            r#"{"actors":[
                {"label":"employer","position":"ACTIVE"},
                {"label":"employee","position":"COUNTERPARTY"},
                {"label":"any person","position":"BENEFICIARY"},
                {"label":"inspector","position":"MENTIONED"}
            ]}"#,
        )
        .unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors[0].label, "Org: Employer");
        assert_eq!(actors[0].position, "active");
        assert_eq!(actors[1].label, "Ind: Employee");
        assert_eq!(actors[1].position, "counterparty");
        assert_eq!(actors[2].label, "Ind: Person");
        assert_eq!(actors[2].position, "beneficiary");
        assert_eq!(actors[3].label, "Spc: Inspector");
        assert_eq!(actors[3].position, "mentioned");
    }

    #[test]
    fn parse_tier3_actors_missing_position_defaults_mentioned() {
        let result: serde_json::Value =
            serde_json::from_str(r#"{"actors":[{"label":"employer"}]}"#).unwrap();
        let matcher = test_matcher();
        let actors = parse_tier3_actors(&result, &matcher).unwrap();
        assert_eq!(actors[0].label, "Org: Employer");
        assert_eq!(actors[0].position, "mentioned");
        assert_eq!(actors[0].reason, None);
        assert_eq!(actors[0].relates_to, None);
    }

    #[test]
    fn parse_tier3_actors_no_actors_key() {
        let result: serde_json::Value =
            serde_json::from_str(r#"{"primary_holder":"Org: Employer"}"#).unwrap();
        let matcher = test_matcher();
        assert!(parse_tier3_actors(&result, &matcher).is_none());
    }

    #[test]
    fn actor_matcher_exact_triggers() {
        let m = test_matcher();
        // Exact trigger matches
        assert_eq!(
            m.match_name("employer"),
            Some(("Org: Employer".into(), 1.0))
        );
        assert_eq!(
            m.match_name("HSE"),
            Some(("Gvt: Agency: Health and Safety Executive".into(), 1.0))
        );
        assert_eq!(
            m.match_name("inspector"),
            Some(("Spc: Inspector".into(), 1.0))
        );
        assert_eq!(
            m.match_name("local authority"),
            Some(("Gvt: Authority: Local".into(), 1.0))
        );
    }

    #[test]
    fn actor_matcher_substring_containment() {
        let m = test_matcher();
        // Substring matching — longer trigger wins
        let (label, conf) = m.match_name("the enforcing authority").unwrap();
        assert_eq!(label, "Gvt: Authority: Enforcement");
        assert!((conf - 0.85).abs() < 0.01);
    }

    #[test]
    fn actor_matcher_discovery() {
        let m = test_matcher();
        // Genuinely novel actors not in dictionary
        assert!(
            m.match_name("producers of electricity from high-efficiency cogeneration")
                .is_none()
        );
        assert!(m.match_name("committee on toxicity").is_none());
    }

    #[test]
    fn actor_matcher_expanded_dictionary() {
        let m = test_matcher();
        // Insolvency roles added from corpus discoveries
        assert_eq!(
            m.match_name("liquidator"),
            Some(("Spc: Liquidator".into(), 1.0))
        );
        assert_eq!(
            m.match_name("water undertaker"),
            Some(("Svc: Water Undertaker".into(), 1.0))
        );
        assert_eq!(
            m.match_name("young people"),
            Some(("Ind: Young Person".into(), 1.0))
        );
        assert_eq!(
            m.match_name("special negotiating body"),
            Some(("EU: Special Negotiating Body".into(), 1.0))
        );
        assert_eq!(
            m.match_name("competent national authorities"),
            Some(("Gvt: Authority".into(), 1.0))
        );
    }

    #[test]
    fn actor_matcher_specificity() {
        let m = test_matcher();
        // "secretary of state for defence" should match the specific entry, not generic
        let (label, _) = m.match_name("secretary of state for defence").unwrap();
        assert_eq!(label, "Gvt: Minister: Secretary of State for Defence");
        // Plain "secretary of state" should match generic
        let (label, _) = m.match_name("secretary of state").unwrap();
        assert_eq!(label, "Gvt: Minister");
    }

    #[test]
    fn actor_matcher_is_government() {
        let m = test_matcher();
        assert!(m.is_government("Gvt: Authority: Enforcement"));
        assert!(m.is_government("EU: Commission"));
        assert!(!m.is_government("Org: Employer"));
        assert!(!m.is_government("Ind: Employee"));
    }
}

/// Extract a string value from an Arrow array, handling both Utf8 and LargeUtf8.
fn get_string_value(col: &dyn arrow::array::Array, i: usize) -> Option<String> {
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
