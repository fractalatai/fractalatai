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
    /// Query a customer's applicable law names from sertantai
    CustomerLaws {
        #[command(flatten)]
        zenoh: ZenohArgs,
        /// Customer name (e.g. "QQ") — discovers UUID automatically
        #[arg(long)]
        name: Option<String>,
        /// Customer UUID (skip discovery, query directly)
        #[arg(long, conflicts_with = "name")]
        customer: Option<String>,
        /// List available customers (discovery only)
        #[arg(long, conflicts_with_all = ["name", "customer"])]
        list: bool,
        /// Query timeout in seconds
        #[arg(long, default_value_t = 15)]
        timeout: u64,
        /// Write law names to a CSV file (comma-separated, one line)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Triage laws: scan provisions to substantiate making/not-making classification
    Triage {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Triage all laws in a DuckDB family
        #[arg(long)]
        family: Option<String>,
        /// Triage all laws with LAT data
        #[arg(long)]
        all: bool,
        /// Show detailed per-law signals (not just summary)
        #[arg(long)]
        verbose: bool,
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
        Command::CustomerLaws {
            zenoh,
            name,
            customer,
            list,
            timeout,
            output,
        } => {
            cmd_customer_laws(&zenoh, name.as_deref(), customer.as_deref(), list, timeout, output.as_deref()).await
        }
        Command::Triage {
            laws,
            family,
            all,
            verbose,
        } => {
            cmd_triage(&data_dir, laws, family, all, verbose, pg_url.as_deref()).await
        }
    }
}

async fn cmd_customer_laws(
    zenoh: &ZenohArgs,
    name: Option<&str>,
    customer_id: Option<&str>,
    list: bool,
    timeout_secs: u64,
    output: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    let config = zenoh.build_zenoh_config()?;
    let sync = fractalaw_sync::ZenohSync::with_config(&zenoh.tenant, config)
        .await
        .map_err(|e| anyhow::anyhow!("failed to open zenoh session: {e}"))?;

    print!("Waiting for zenoh peer...");
    let peers = sync
        .wait_for_peers(std::time::Duration::from_secs(10))
        .await;
    if peers == 0 {
        println!(" no peers connected (timeout).");
        anyhow::bail!("No zenoh peers found — is sertantai running?");
    }
    println!(" {peers} peer(s) connected.");

    let timeout = std::time::Duration::from_secs(timeout_secs);

    // Resolve customer UUID: either given directly or discovered by name
    let resolved_id = if let Some(id) = customer_id {
        id.to_string()
    } else {
        // Discovery step: query /sertantai/customers
        let customers = sync
            .query_customers(timeout)
            .await
            .map_err(|e| anyhow::anyhow!("customer discovery failed: {e}"))?;

        if customers.is_empty() {
            anyhow::bail!("No customers returned from sertantai. Is the queryable running?");
        }

        if list || name.is_none() {
            println!("\nAvailable customers:");
            for c in &customers {
                let cname = c["name"].as_str().unwrap_or("?");
                let cid = c["id"].as_str().unwrap_or("?");
                let count = c["law_count"].as_u64().unwrap_or(0);
                println!("  {cname:20} {count:>4} laws  {cid}");
            }
            if list {
                return Ok(());
            }
            anyhow::bail!("Specify --name <CUSTOMER_NAME> or --customer <UUID>");
        }

        let target_name = name.unwrap();
        let found = customers.iter().find(|c| {
            c["name"]
                .as_str()
                .is_some_and(|n| n.eq_ignore_ascii_case(target_name))
        });

        match found {
            Some(c) => {
                let id = c["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("customer has no id field"))?;
                let count = c["law_count"].as_u64().unwrap_or(0);
                println!("Found customer '{}': {} laws ({})", target_name, count, id);
                id.to_string()
            }
            None => {
                println!("Customer '{}' not found. Available:", target_name);
                for c in &customers {
                    let cname = c["name"].as_str().unwrap_or("?");
                    println!("  {cname}");
                }
                anyhow::bail!("customer not found");
            }
        }
    };

    // Fetch laws for the resolved customer
    println!("Querying laws for customer {resolved_id}...");
    let names = sync
        .query_customer_laws(&resolved_id, timeout)
        .await
        .map_err(|e| anyhow::anyhow!("query failed: {e}"))?;

    if names.is_empty() {
        println!("No laws returned.");
        return Ok(());
    }

    println!("{} laws in customer register.", names.len());

    if let Some(path) = output {
        let csv = names.join(",");
        std::fs::write(path, format!("{csv}\n"))?;
        println!("Written to {}", path.display());
    } else {
        println!("{}", names.join(","));
    }

    Ok(())
}

async fn cmd_triage(
    data_dir: &std::path::Path,
    laws: Option<String>,
    family: Option<String>,
    all: bool,
    verbose: bool,
    pg_url: Option<&str>,
) -> anyhow::Result<()> {
    use arrow::array::{Array, StringArray};
    use fractalaw_core::taxa::making;

    let store = open_duck(data_dir)?;
    store.ensure_triage_columns()?;
    let lance = open_provision_store(pg_url).await?;

    // Resolve law names
    let law_names: Vec<String> = if let Some(ref fam) = family {
        let names = laws_in_family(&store, fam)?;
        if names.is_empty() {
            anyhow::bail!("No laws found with family '{fam}'");
        }
        println!("Family '{}': {} laws", fam, names.len());
        names
    } else if let Some(ref l) = laws {
        l.split(',').map(|s| s.trim().to_string()).collect()
    } else if all {
        let batches = store.query_arrow(
            "SELECT name FROM legislation WHERE is_making IS NOT NULL ORDER BY name",
        )?;
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
        println!("Triaging ALL {} laws", names.len());
        names
    } else {
        anyhow::bail!(
            "Specify --laws, --family, or --all.\n\
             Example: fractalaw-sync triage --laws UK_ukpga_1974_37 --pg postgres://..."
        );
    };

    if law_names.is_empty() {
        println!("No laws to triage.");
        return Ok(());
    }

    // Get DuckDB metadata for all laws (title, description, is_making)
    let meta_sql = format!(
        "SELECT name, title, description, is_making FROM legislation WHERE name IN ({})",
        law_names
            .iter()
            .map(|n| format!("'{}'", n.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let meta_batches = store.query_arrow(&meta_sql)?;

    // Build metadata lookup
    let mut meta_map: std::collections::HashMap<String, (Option<String>, Option<String>, Option<bool>)> =
        std::collections::HashMap::new();
    for batch in &meta_batches {
        let name_col = batch.column_by_name("name");
        let title_col = batch.column_by_name("title");
        let desc_col = batch.column_by_name("description");
        let making_col = batch.column_by_name("is_making");
        for row in 0..batch.num_rows() {
            let name = name_col.and_then(|c| get_string_value(c.as_ref(), row)).unwrap_or_default();
            let title = title_col.and_then(|c| get_string_value(c.as_ref(), row));
            let desc = desc_col.and_then(|c| get_string_value(c.as_ref(), row));
            let is_making = making_col.map(|c| {
                if c.is_null(row) { None }
                else {
                    c.as_any()
                        .downcast_ref::<arrow::array::BooleanArray>()
                        .map(|a| a.value(row))
                }
            }).flatten();
            meta_map.insert(name, (title, desc, is_making));
        }
    }

    let mut making_count = 0u32;
    let mut not_making_count = 0u32;
    let mut uncertain_count = 0u32;
    let mut disagree_count = 0u32;

    for law_name in &law_names {
        // Get provision texts from Postgres
        let batches = lance
            .query_legislation_text(law_name, 100_000, 0)
            .await
            .map_err(|e| anyhow::anyhow!("query {law_name}: {e}"))?;

        let mut texts: Vec<String> = Vec::new();
        for batch in &batches {
            if let Some(col) = batch.column_by_name("text") {
                for i in 0..batch.num_rows() {
                    if let Some(t) = get_string_value(col.as_ref(), i) {
                        if !t.is_empty() {
                            texts.push(t);
                        }
                    }
                }
            }
        }

        // Look up DuckDB family for specialist actor gating
        let law_family = {
            let fam_batches = store.query_arrow(&format!(
                "SELECT family FROM legislation WHERE name = '{}'",
                law_name.replace('\'', "''")
            ))?;
            fam_batches.first().and_then(|b| {
                b.column_by_name("family")
                    .and_then(|c| get_string_value(c.as_ref(), 0))
            })
        };

        // Run triage
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let counts = making::triage_provisions(&text_refs, law_family.as_deref());

        // Get metadata
        let (title, desc, sertantai_making) = meta_map
            .get(law_name.as_str())
            .cloned()
            .unwrap_or((None, None, None));

        let meta = making::LawMetadata {
            title: title.as_deref(),
            description: desc.as_deref(),
            body_paras: Some(counts.total),
            schedule_paras: None,
        };

        let result = making::detect_with_triage(&meta, &counts);

        // Persist triage result to DuckDB
        {
            let escaped_name = law_name.replace('\'', "''");
            let _ = store.execute(&format!(
                "UPDATE legislation \
                 SET triage_classification = '{}', \
                     triage_confidence = {}, \
                     triage_tier = {}, \
                     triaged_at = CURRENT_TIMESTAMP \
                 WHERE name = '{escaped_name}'",
                result.classification.as_str(),
                result.confidence,
                result.tier,
            ));
        }

        // Compare with sertantai's is_making
        let agrees = match (sertantai_making, &result.classification) {
            (Some(true), making::MakingClassification::Making) => true,
            (Some(false), making::MakingClassification::NotMaking) => true,
            (None, _) => true, // no opinion to disagree with
            _ => false,
        };
        if !agrees {
            disagree_count += 1;
        }

        match result.classification {
            making::MakingClassification::Making => making_count += 1,
            making::MakingClassification::NotMaking => not_making_count += 1,
            making::MakingClassification::Uncertain => uncertain_count += 1,
        }

        if verbose || !agrees {
            let sertantai_str = match sertantai_making {
                Some(true) => "making",
                Some(false) => "not_making",
                None => "null",
            };
            let flag = if agrees { " " } else { "!" };
            println!(
                "{flag} {law_name}: triage={} ({:.0}%) sertantai={} provisions={} obligations={} actors={} amendment={}",
                result.classification.as_str(),
                result.confidence * 100.0,
                sertantai_str,
                counts.total,
                counts.with_obligation,
                counts.with_actor,
                counts.amendment,
            );
            if verbose {
                for s in &result.signals {
                    println!(
                        "    T{} {} {:?} {:.0}% — {}",
                        s.tier,
                        s.name,
                        s.direction,
                        s.confidence * 100.0,
                        s.value.chars().take(80).collect::<String>(),
                    );
                }
            }
        }
    }

    println!(
        "\nTriage: {} making, {} not_making, {} uncertain ({} disagree with sertantai)",
        making_count, not_making_count, uncertain_count, disagree_count,
    );

    Ok(())
}
