mod commands;
mod display;
mod embed;
mod llm;
mod provenance;
mod utils;

use std::path::PathBuf;

use anyhow::Context;
use arrow::array::Array;
use clap::{Parser, Subcommand};
use fractalaw_store::{DuckStore, LanceStore};
use commands::misc::*;
use commands::taxa::*;
use utils::*;

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

    /// Hub provision store, PostgreSQL+pgvector (the default, #71)
    #[arg(long, global = true, env = "FRACTALAW_PG", default_value = fractalaw_store::HUB_PG_URL)]
    pg: String,

    /// Use the LanceDB provision store (data/lancedb) instead of the hub
    /// Postgres. For edge/offline use only: the hub's LanceDB is not kept current.
    #[arg(long, global = true)]
    lance: bool,

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

    /// Taxa DRRP classification tools
    Taxa {
        #[command(subcommand)]
        action: TaxaAction,
    },

    /// Fitness applicability extraction (independent of DRRP taxa)
    Fitness {
        #[command(subcommand)]
        action: FitnessAction,
    },

    /// JSP (Joint Service Publication) enrichment pipeline
    Jsp {
        #[command(subcommand)]
        action: commands::jsp::JspAction,
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
    /// Show pipeline status for laws (which stage each law is at)
    Status {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Read law names from a file (one per line or CSV)
        #[arg(long)]
        law_file: Option<PathBuf>,
        /// Show summary counts only
        #[arg(long)]
        summary: bool,
        /// Filter to a specific stage (e.g. needs_embed, ready_to_publish)
        #[arg(long)]
        stage: Option<String>,
    },
    /// Infer correlative actors from regex signals (Hohfeldian correlatives)
    Infer {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
    },
    /// Reconcile per-tier signals (regex/classifier/LLM) into final drrp_types + actors
    Reconcile {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
    },
    /// Backfill legislation_text.actors/drrp_types from reconciled provision_actors
    Backfill {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
        /// Write nothing: print each law's current vs new law-level verdict (TSV)
        #[arg(long)]
        dry_run: bool,
    },
    /// Classify pending_llm actors via local SLM (Ollama gemma3-position)
    Slm {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
    },
    /// Whole-law LLM validation: send all provisions + parse results to LLM
    Validate {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: String,
        /// Directory for audit log JSON files (e.g. data/audit)
        #[arg(long, default_value = "data/audit")]
        audit_dir: String,
        /// Dry run: build prompts and show token estimates without calling LLM
        #[arg(long)]
        dry_run: bool,
        /// Apply corrections to LanceDB (default: audit-only, no writes)
        #[arg(long)]
        apply: bool,
    },
}

#[derive(Subcommand)]
enum FitnessAction {
    /// Extract applicability mentions from provision text → fitness_mentions table.
    /// Runs polarity detection + dictionary extraction independently of DRRP taxa.
    Extract {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Read law names from a file (one per line or CSV)
        #[arg(long)]
        law_file: Option<PathBuf>,
        /// Re-extract all (clear existing fitness_mentions for these laws)
        #[arg(long)]
        force: bool,
    },
    /// Show fitness mention counts and coverage per law
    Status {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Read law names from a file (one per line or CSV)
        #[arg(long)]
        law_file: Option<PathBuf>,
    },
    /// Reconcile tier entities into `entities` (ft > regex > slm) for rows not yet reconciled
    Reconcile {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Read law names from a file (one per line or CSV)
        #[arg(long)]
        law_file: Option<PathBuf>,
        /// Report what would be reconciled without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Derive law application (nations where it applies) → DuckDB application_* columns
    Application {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Read law names from a file (one per line or CSV)
        #[arg(long)]
        law_file: Option<PathBuf>,
        /// Write JSONL to this file instead of DuckDB
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compile fitness mentions into expression trees per law → DuckDB
    Compile {
        /// Specific laws (comma-separated)
        #[arg(long)]
        laws: Option<String>,
        /// Read law names from a file (one per line or CSV)
        #[arg(long)]
        law_file: Option<PathBuf>,
        /// Write trees as JSONL ({"name","tree"}) to this file instead of DuckDB
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

/// Commands not yet ported to the hub Postgres (#71) run only with an
/// explicit `--lance`, so they never read the stale LanceDB by default.
fn require_lance(pg_url: Option<&str>, command: &str) -> anyhow::Result<()> {
    if pg_url.is_some() {
        anyhow::bail!(
            "`{command}` still reads LanceDB only (not yet ported to the hub Postgres, fractalatai #71). \
             Re-run with --lance to use data/lancedb, which is not kept current."
        );
    }
    Ok(())
}

/// Open the provision store: the hub Postgres, or LanceDB with `--lance`.
async fn open_provision_store(
    data_dir: &std::path::Path,
    pg_url: Option<&str>,
) -> anyhow::Result<Box<dyn fractalaw_store::ProvisionStore>> {
    if let Some(url) = pg_url {
        let store = fractalaw_store::PgStore::connect(url)
            .await
            .context("connecting to PostgreSQL")?;
        Ok(Box::new(store))
    } else {
        tracing::warn!(path = %data_dir.join("lancedb").display(), "provision store: LanceDB (--lance), not the hub Postgres");
        let store = LanceStore::open(&data_dir.join("lancedb"))
            .await
            .context("opening LanceDB")?;
        Ok(Box::new(store))
    }
}

/// Law-level DRRP outcome for one law (#55, #68): the verdict or the reason
/// there's none, and the roll-up to write where there is one. Shared by
/// `taxa backfill` and its `--dry-run`.
fn law_drrp_for(
    inputs: &fractalaw_store::LawDrrpInputs,
) -> (&'static str, Option<fractalaw_core::taxa::law_drrp::LawDrrp>) {
    use fractalaw_core::taxa::law_drrp::LawDrrp;
    if inputs.signals.iter().any(|(_, _, _, method, _)| method.is_none()) {
        return ("unreconciled", None);
    }
    if inputs.signals.is_empty() {
        return if inputs.duty_text_provisions == 0 && inputs.substantive_provisions > 0 {
            ("no_obligations_no_duty_text", Some(LawDrrp::default()))
        } else {
            ("no_actors", None)
        };
    }
    if !inputs.provisions.iter().any(|(_, _, scope)| scope.is_some()) {
        return ("not_run", None);
    }
    let texts: std::collections::HashMap<&str, &str> = inputs
        .provisions
        .iter()
        .filter_map(|(sid, text, _)| text.as_deref().map(|t| (sid.as_str(), t)))
        .collect();
    // Amendment-scope provisions belong to the amended instrument (#57)
    let amendment: std::collections::HashSet<&str> = inputs
        .provisions
        .iter()
        .filter(|(_, _, scope)| scope.as_deref() == Some("amendment"))
        .map(|(sid, _, _)| sid.as_str())
        .collect();
    let law = rollup(inputs, &amendment, &std::collections::HashSet::new(), &texts);
    (law.verdict().map_or("holder_unknown", |v| v.as_str()), Some(law))
}

/// Layer-1b correlatives per provision actor (#72), as (section_id, label,
/// JSON array) rows for `write_actor_correlatives`. Every reconciled actor
/// gets a row, `[]` when it holds none.
fn provision_correlatives(inputs: &fractalaw_store::LawDrrpInputs) -> Vec<(String, String, String)> {
    use fractalaw_core::taxa::correlatives::{PositionedActor, derive_correlatives};
    let mut sections: std::collections::HashMap<String, Vec<PositionedActor>> = std::collections::HashMap::new();
    for (sid, label, drrp, _, position) in &inputs.signals {
        sections.entry(sid.clone()).or_default().push(PositionedActor {
            label: label.clone(),
            drrp: drrp.clone().unwrap_or_default(),
            position: position.clone().unwrap_or_default(),
            access_inferred: inputs.access_inferred.contains(&(sid.clone(), label.clone())),
        });
    }
    derive_correlatives(&sections)
        .into_iter()
        .map(|((sid, label), cs)| (sid, label, serde_json::to_string(&cs).expect("correlatives serialise")))
        .collect()
}

/// Roll up a law's signals, skipping amendment-scope and `exclude`d sections.
fn rollup(
    inputs: &fractalaw_store::LawDrrpInputs,
    amendment: &std::collections::HashSet<&str>,
    exclude: &std::collections::HashSet<&str>,
    texts: &std::collections::HashMap<&str, &str>,
) -> fractalaw_core::taxa::law_drrp::LawDrrp {
    use fractalaw_core::taxa::law_drrp::{ActorSignal, aggregate};
    let signals: Vec<ActorSignal> = inputs
        .signals
        .iter()
        .filter(|(sid, ..)| !amendment.contains(sid.as_str()) && !exclude.contains(sid.as_str()))
        .map(|(sid, label, drrp, _, position)| ActorSignal {
            section_id: sid.clone(),
            actor_label: label.clone(),
            drrp: drrp.clone().unwrap_or_default(),
            position: position.clone().unwrap_or_default(),
            access_inferred: inputs.access_inferred.contains(&(sid.clone(), label.clone())),
        })
        .collect();
    let unknown: Vec<String> =
        inputs.holder_unknown.iter().filter(|s| !exclude.contains(s.as_str())).cloned().collect();
    aggregate(&signals, &unknown, |sid| texts.get(sid).map(|t| t.to_string()))
}

/// The current (as-amended) view for a law whose as-made DRRP is written
/// (#73 R1a): the roll-up over live provisions only, and its verdict label.
/// `revoked` when legal's `live` says so or no substantive provision is live.
/// None where the as-made view isn't written either (unreconciled, no actors).
fn current_for(
    inputs: &fractalaw_store::LawDrrpInputs,
    outcome: &str,
    live: Option<&str>,
) -> Option<(&'static str, fractalaw_core::taxa::law_drrp::LawDrrp)> {
    use fractalaw_core::taxa::law_drrp::{LawDrrp, current_verdict, live_is_revoked};
    if matches!(outcome, "unreconciled" | "no_actors" | "not_run") {
        return None;
    }
    let revoked = live_is_revoked(live) || (inputs.substantive_total > 0 && inputs.substantive_provisions == 0);
    if outcome == "no_obligations_no_duty_text" {
        let law = LawDrrp::default();
        return Some((current_verdict(revoked, &law), law));
    }
    let texts: std::collections::HashMap<&str, &str> = inputs
        .provisions
        .iter()
        .filter_map(|(sid, text, _)| text.as_deref().map(|t| (sid.as_str(), t)))
        .collect();
    let amendment: std::collections::HashSet<&str> = inputs
        .provisions
        .iter()
        .filter(|(_, _, scope)| scope.as_deref() == Some("amendment"))
        .map(|(sid, _, _)| sid.as_str())
        .collect();
    let exclude: std::collections::HashSet<&str> = inputs.non_live.iter().map(String::as_str).collect();
    let law = rollup(inputs, &amendment, &exclude, &texts);
    let label = current_verdict(revoked, &law);
    Some((label, law.current_payload(label)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    // Postgres unless --lance: never fall back to LanceDB silently (#71)
    let pg_url = (!cli.lance).then(|| cli.pg.clone());

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
            require_lance(pg_url.as_deref(), "validate")?;
            cmd_validate(&open_duck(&data_dir)?, &data_dir, &model_dir).await
        }
        Command::Classify {
            domain_threshold,
            subject_threshold,
        } => {
            require_lance(pg_url.as_deref(), "classify")?;
            cmd_classify(
                &open_duck(&data_dir)?,
                &data_dir,
                domain_threshold,
                subject_threshold,
            )
            .await
        }
        Command::Import => cmd_import(&data_dir),

        // Provision-store commands — no DuckDB needed.
        Command::Embed { model_dir } => {
            require_lance(pg_url.as_deref(), "embed")?;
            cmd_embed(&data_dir, &model_dir).await
        }
        Command::Text { name, limit } => {
            let store = open_provision_store(&data_dir, pg_url.as_deref()).await?;
            cmd_text(store.as_ref(), &name, limit).await
        }
        Command::Search {
            query,
            limit,
            model_dir,
        } => {
            let store = open_provision_store(&data_dir, pg_url.as_deref()).await?;
            cmd_search(store.as_ref(), &query, limit, &model_dir).await
        }

        // Model-only commands — no data store needed.
        Command::Tokenize { text, model_dir } => cmd_tokenize(&text, &model_dir),

        // WASM micro-app commands.
        Command::Run { component, fuel } => {
            require_lance(pg_url.as_deref(), "run")?;
            cmd_run(&data_dir, &component, fuel).await
        }

        // Taxa classification.
        Command::Taxa { action } => match action {
            TaxaAction::Show {
                name,
                limit,
                misses,
                clauses,
            } => cmd_taxa_show(open_provision_store(&data_dir, pg_url.as_deref()).await?.as_ref(), &data_dir, &name, limit, misses, clauses).await,
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
                    pg_url.as_deref(),
                )
                .await
            }
            TaxaAction::Eyeball {
                laws,
                output,
                limit,
            } => {
                let law_names: Vec<&str> = laws.split(',').map(|l| l.trim()).collect();
                cmd_taxa_eyeball(open_provision_store(&data_dir, pg_url.as_deref()).await?.as_ref(), &law_names, &output, limit).await
            }
            TaxaAction::Qa { laws, family } => cmd_taxa_qa(open_provision_store(&data_dir, pg_url.as_deref()).await?.as_ref(), &data_dir, laws, family).await,
            TaxaAction::Status {
                laws,
                law_file,
                summary,
                stage,
            } => {
                let store = open_duck(&data_dir)?;
                cmd_taxa_status(&store, laws, law_file, summary, stage)
            }
            TaxaAction::Infer { laws } => {
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names = provenance::without_enabling_extent(
                    pg_url.as_deref(),
                    laws.split(',').map(|s| s.trim().to_string()).collect(),
                )
                .await?;
                let r = cmd_taxa_infer(lance.as_ref(), &law_names).await;
                if r.is_ok() { provenance::record(pg_url.as_deref(), Some(&law_names), provenance::taxa_infer()).await; }
                r
            }
            TaxaAction::Reconcile { laws } => {
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names = provenance::without_enabling_extent(
                    pg_url.as_deref(),
                    laws.split(',').map(|s| s.trim().to_string()).collect(),
                )
                .await?;
                let r = cmd_taxa_reconcile(lance.as_ref(), &law_names).await;
                if r.is_ok() { provenance::record(pg_url.as_deref(), Some(&law_names), provenance::taxa_reconcile()).await; }
                r
            }
            TaxaAction::Backfill { laws, dry_run } => {
                let store = open_duck(&data_dir)?;
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names = provenance::without_enabling_extent(
                    pg_url.as_deref(),
                    laws.split(',').map(|s| s.trim().to_string()).collect(),
                )
                .await?;
                let mut total = 0usize;
                let mut sig_total = 0usize;
                let mut parts_total = 0usize;
                let mut law_sig_total = 0usize;
                let drrp_types = commands::pipeline::drrp_column_types(&store)?;
                let mut verdicts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
                let mut drrp_rolled_up: Vec<String> = Vec::new();
                let mut no_duty_text: Vec<String> = Vec::new();
                let mut transitions: std::collections::BTreeMap<(String, String), usize> = std::collections::BTreeMap::new();
                // (as-made verdict, current verdict) counts (#73 R1a)
                let mut current_views: std::collections::BTreeMap<(String, String), usize> = std::collections::BTreeMap::new();
                store.ensure_current_view_columns()?;
                store.ensure_correlative_columns()?;
                if dry_run {
                    println!("law\tcurrent\tnew\tduties\tresponsibilities\trights\tpowers\tholder_unknown\tnon_active_excluded\tcurrent_view\tclaim_holders\tliability_holders\tprotected_holders");
                }
                for law_name in &law_names {
                    if dry_run {
                        // Same inputs and branches as the real run, nothing written
                        let current = commands::pipeline::read_law_verdict(&store, law_name)?.unwrap_or("-");
                        let inputs = lance.query_law_drrp_inputs(law_name).await?;
                        let (new, law) = law_drrp_for(&inputs);
                        let live = commands::pipeline::read_law_live(&store, law_name)?;
                        let cur = current_for(&inputs, new, live.as_deref()).map_or("-", |(v, _)| v);
                        *current_views.entry((new.to_string(), cur.to_string())).or_default() += 1;
                        let corr = law.as_ref().map_or_else(|| "\t\t".to_string(), |l| {
                            let j = |s: &std::collections::BTreeSet<String>| s.iter().cloned().collect::<Vec<_>>().join("; ");
                            format!("{}\t{}\t{}", j(&l.claim_holders), j(&l.liability_holders), j(&l.protected_holders))
                        });
                        let row = law.map(|l| {
                            format!(
                                "{}\t{}\t{}\t{}\t{}\t{}",
                                l.duties.len(), l.responsibilities.len(), l.rights.len(), l.powers.len(),
                                l.holder_unknown, l.excluded_non_active
                            )
                        });
                        println!("{law_name}\t{current}\t{new}\t{}\t{cur}\t{corr}", row.unwrap_or_else(|| "\t\t\t\t\t".into()));
                        *transitions.entry((current.to_string(), new.to_string())).or_default() += 1;
                        continue;
                    }
                    let updated = lance.backfill_from_actors(law_name).await?;
                    let inputs = lance.query_law_drrp_inputs(law_name).await?;
                    // Layer-1b correlatives into the provision actors JSON (#72)
                    lance.write_actor_correlatives(law_name, &provision_correlatives(&inputs)).await?;
                    let sig = lance.backfill_significance(law_name).await?;

                    // Law-level significance (Approach L + K profile) → DuckDB (#55)
                    let (high, medium, low, _) = lance.query_significance_profile(law_name).await?;
                    let law_sig = fractalaw_core::taxa::law_significance::law_significance(high, medium, low);
                    let set = match &law_sig {
                        Some(s) => {
                            law_sig_total += 1;
                            format!(
                                "significance_rating = '{}', significance_score = {}, \
                                 significance_high_count = {}, significance_medium_count = {}, \
                                 significance_low_count = {}, significance_total_obligations = {}",
                                s.rating, s.score, s.high, s.medium, s.low, s.total
                            )
                        }
                        None => "significance_rating = NULL, significance_score = NULL, \
                                 significance_high_count = NULL, significance_medium_count = NULL, \
                                 significance_low_count = NULL, significance_total_obligations = NULL"
                            .to_string(),
                    };
                    store.execute(&format!(
                        "UPDATE legislation SET {set} WHERE name = '{}'",
                        law_name.replace('\'', "''")
                    ))?;

                    // Law-level DRRP from reconciled provision_actors → DuckDB (#55).
                    // Only where DRRP ran (parsed provisions); otherwise leave it as is.
                    let (outcome, law) = law_drrp_for(&inputs);
                    match (outcome, law) {
                        ("unreconciled", _) => {
                            // Reconcile hasn't run: no verdict rather than a false "no obligations"
                            eprintln!("  {law_name}: unreconciled provision_actors, law-level DRRP left unchanged (run taxa reconcile)");
                        }
                        ("no_obligations_no_duty_text", Some(law)) => {
                            // No actors and no duty text in any substantive provision: parse ran and
                            // found nothing to impose — evidence of no obligations, not an actor gap.
                            commands::pipeline::write_law_drrp(&store, law_name, &law, &drrp_types)?;
                            eprintln!(
                                "  {law_name}: no duty text in {} substantive provisions → no_obligations",
                                inputs.substantive_provisions
                            );
                            no_duty_text.push(law_name.clone());
                        }
                        ("no_actors", _) => {
                            // No actor rows but duty text present (or nothing parsed): parse found no
                            // duty-bearer it could name (actor model gap), so there is no evidence
                            // either way. Leave law-level DRRP unchanged.
                            eprintln!("  {law_name}: no provision_actors, law-level DRRP left unchanged");
                        }
                        ("holder_unknown", Some(law)) => {
                            // No Duty/Responsibility, but Obligations whose holder we can't name:
                            // no verdict, not empowering or no_obligations (#68). Raw duty_type
                            // with empty lists, so legal overwrites the stale verdict.
                            commands::pipeline::write_law_drrp(&store, law_name, &law.holder_unknown_payload(), &drrp_types)?;
                            eprintln!("  {law_name}: {} holder-unknown Obligation provisions → no verdict", law.holder_unknown);
                            drrp_rolled_up.push(law_name.clone());
                        }
                        (_, Some(law)) => {
                            commands::pipeline::write_law_drrp(&store, law_name, &law, &drrp_types)?;
                            drrp_rolled_up.push(law_name.clone());
                        }
                        _ => {}
                    }
                    *verdicts.entry(outcome).or_default() += 1;
                    // Current (as-amended) view beside the as-made one (#73 R1a)
                    let live = commands::pipeline::read_law_live(&store, law_name)?;
                    if let Some((label, cur)) = current_for(&inputs, outcome, live.as_deref()) {
                        commands::pipeline::write_law_current(&store, law_name, label, &cur)?;
                        *current_views.entry((outcome.to_string(), label.to_string())).or_default() += 1;
                    }

                    // Part-level significance breakdown for large Acts
                    if let Some(parts_json) = lance.query_significance_parts(law_name).await? {
                        store.execute(&format!(
                            "UPDATE legislation SET significance_parts = '{}' WHERE name = '{}'",
                            parts_json.replace('\'', "''"),
                            law_name.replace('\'', "''")
                        ))?;
                        parts_total += 1;
                        eprintln!("  {law_name}: {updated} backfilled, {sig} significance, parts breakdown computed");
                    } else {
                        eprintln!("  {law_name}: {updated} backfilled, {sig} significance");
                    }

                    total += updated;
                    sig_total += sig;
                }
                if dry_run {
                    eprintln!("Verdict transitions (current → new) across {} laws:", law_names.len());
                    for ((from, to), n) in &transitions {
                        eprintln!("  {from} → {to}: {n}");
                    }
                    eprintln!("As made → current view (#73 R1a):");
                    for ((made, cur), n) in &current_views {
                        eprintln!("  {made} → {cur}: {n}");
                    }
                    return Ok(());
                }
                println!(
                    "Backfilled {total} provisions, {sig_total} significance, {law_sig_total} law-level ratings, {parts_total} Part breakdowns across {} laws",
                    law_names.len()
                );
                println!("Law-level DRRP verdicts: {verdicts:?}");
                println!("As made → current view (#73 R1a): {current_views:?}");
                // Provenance (#63): DRRP roll-up only where it wrote a verdict; significance for all
                let [drrp_stage, sig_stage]: [_; 2] = provenance::taxa_backfill().try_into().expect("two stages");
                // The verdict's basis stays auditable: no-duty-text verdicts are recorded as such
                let mut no_duty_stage = drrp_stage.clone();
                no_duty_stage.model = "fractalaw-law-drrp:no_duty_text".to_string();
                provenance::record(pg_url.as_deref(), Some(&drrp_rolled_up), vec![drrp_stage]).await;
                if !no_duty_text.is_empty() {
                    provenance::record(pg_url.as_deref(), Some(&no_duty_text), vec![no_duty_stage]).await;
                }
                provenance::record(pg_url.as_deref(), Some(&law_names), vec![sig_stage]).await;
                Ok(())
            }
            TaxaAction::Slm { laws } => {
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names = provenance::without_enabling_extent(
                    pg_url.as_deref(),
                    laws.split(',').map(|s| s.trim().to_string()).collect(),
                )
                .await?;
                let r = cmd_taxa_slm(lance.as_ref(), &law_names).await;
                if r.is_ok() { provenance::record(pg_url.as_deref(), Some(&law_names), provenance::taxa_slm()).await; }
                r
            }
            TaxaAction::AuditFitness {
                laws,
                family,
                limit,
            } => cmd_taxa_audit_fitness(open_provision_store(&data_dir, pg_url.as_deref()).await?.as_ref(), &data_dir, laws, family, limit).await,
            TaxaAction::Parse { laws, force, trace } => {
                let store = open_duck(&data_dir)?;
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names = provenance::without_enabling_extent(
                    pg_url.as_deref(),
                    laws.split(',').map(|s| s.trim().to_string()).collect(),
                )
                .await?;
                cmd_taxa_parse(lance.as_ref(), &store, &law_names, force).await?;
                provenance::record(pg_url.as_deref(), Some(&law_names), provenance::taxa_parse()).await;
                if let Some(trace_path) = trace {
                    cmd_taxa_trace(lance.as_ref(), &store, &law_names, &trace_path).await?;
                }
                Ok(())
            }
            TaxaAction::Embed { laws } => {
                let store = open_duck(&data_dir)?;
                store.ensure_pipeline_status_columns()?;
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names = provenance::without_enabling_extent(
                    pg_url.as_deref(),
                    laws.split(',').map(|s| s.trim().to_string()).collect(),
                )
                .await?;
                let result = cmd_taxa_embed(lance.as_ref(), &law_names).await;
                if result.is_ok() { provenance::record(pg_url.as_deref(), Some(&law_names), provenance::taxa_embed()).await; }
                for name in &law_names {
                    let escaped = name.replace('\'', "''");
                    let _ = store.execute(&format!(
                        "UPDATE legislation SET embedded_at = CURRENT_TIMESTAMP WHERE name = '{escaped}'"
                    ));
                }
                result
            }
            TaxaAction::Classify { laws } => {
                let store = open_duck(&data_dir)?;
                store.ensure_pipeline_status_columns()?;
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names = provenance::without_enabling_extent(
                    pg_url.as_deref(),
                    laws.split(',').map(|s| s.trim().to_string()).collect(),
                )
                .await?;
                let result = cmd_taxa_classify(lance.as_ref(), &law_names).await;
                if result.is_ok() { provenance::record(pg_url.as_deref(), Some(&law_names), provenance::taxa_classify()).await; }
                for name in &law_names {
                    let escaped = name.replace('\'', "''");
                    let _ = store.execute(&format!(
                        "UPDATE legislation SET classified_at = CURRENT_TIMESTAMP WHERE name = '{escaped}'"
                    ));
                }
                result
            }
            TaxaAction::Escalate { laws } => {
                let store = open_duck(&data_dir)?;
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names = provenance::without_enabling_extent(
                    pg_url.as_deref(),
                    laws.split(',').map(|s| s.trim().to_string()).collect(),
                )
                .await?;
                let r = cmd_taxa_escalate(lance.as_ref(), &store, &law_names).await;
                if r.is_ok() { provenance::record(pg_url.as_deref(), Some(&law_names), provenance::taxa_escalate()).await; }
                r
            }
            TaxaAction::Validate {
                laws,
                audit_dir,
                dry_run,
                apply,
            } => {
                let store = open_duck(&data_dir)?;
                store.ensure_pipeline_status_columns()?;
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names = provenance::without_enabling_extent(
                    pg_url.as_deref(),
                    laws.split(',').map(|s| s.trim().to_string()).collect(),
                )
                .await?;
                let result = cmd_taxa_validate(lance.as_ref(), &store, &law_names, &audit_dir, dry_run, apply).await;
                if result.is_ok() && apply && !dry_run {
                    provenance::record(pg_url.as_deref(), Some(&law_names), provenance::taxa_validate()).await;
                }
                if !dry_run {
                    for name in &law_names {
                        let escaped = name.replace('\'', "''");
                        let _ = store.execute(&format!(
                            "UPDATE legislation SET validated_at = CURRENT_TIMESTAMP WHERE name = '{escaped}'"
                        ));
                    }
                }
                result
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
                open_provision_store(&data_dir, pg_url.as_deref()).await?.as_ref(),
                &open_duck(&data_dir)?,
                &output,
                val_laws.as_deref(),
                test_laws,
                min_match_ratio,
            )
            .await
        }

        // Fitness applicability extraction (independent of DRRP taxa).
        Command::Fitness { action } => match action {
            FitnessAction::Extract {
                laws,
                law_file,
                force,
            } => {
                let pg_url = pg_url
                    .as_deref()
                    .unwrap_or(fractalaw_store::HUB_PG_URL);
                let law_names = resolve_law_names(laws.as_deref(), law_file.as_deref())?;
                let law_names = provenance::fitness_scope(Some(pg_url), law_names).await?;
                let duck = open_duck(&data_dir)?;
                let r = commands::fitness::cmd_fitness_extract(pg_url, &duck, law_names.as_deref(), force).await;
                if r.is_ok() { provenance::record(Some(pg_url), law_names.as_deref(), provenance::fitness_extract()).await; }
                r
            }
            FitnessAction::Status { laws, law_file } => {
                let pg_url = pg_url
                    .as_deref()
                    .unwrap_or(fractalaw_store::HUB_PG_URL);
                let law_names = resolve_law_names(laws.as_deref(), law_file.as_deref())?;
                commands::fitness::cmd_fitness_status(pg_url, law_names.as_deref()).await
            }
            FitnessAction::Reconcile { laws, law_file, dry_run } => {
                let pg_url = pg_url
                    .as_deref()
                    .unwrap_or(fractalaw_store::HUB_PG_URL);
                let law_names = resolve_law_names(laws.as_deref(), law_file.as_deref())?;
                let law_names = provenance::fitness_scope(Some(pg_url), law_names).await?;
                let r = commands::fitness::cmd_fitness_reconcile(pg_url, law_names.as_deref(), dry_run).await;
                if r.is_ok() && !dry_run { provenance::record(Some(pg_url), law_names.as_deref(), provenance::fitness_reconcile()).await; }
                r
            }
            FitnessAction::Application { laws, law_file, out } => {
                let pg_url = pg_url
                    .as_deref()
                    .unwrap_or(fractalaw_store::HUB_PG_URL);
                let law_names = resolve_law_names(laws.as_deref(), law_file.as_deref())?;
                let law_names = provenance::fitness_scope(Some(pg_url), law_names).await?;
                let duck = open_duck(&data_dir)?;
                let r = commands::fitness::cmd_fitness_application(pg_url, &duck, law_names.as_deref(), out.as_deref()).await;
                // --out writes JSONL for review, not DuckDB: nothing enriched
                if r.is_ok() && out.is_none() { provenance::record(Some(pg_url), law_names.as_deref(), provenance::fitness_application()).await; }
                r
            }
            FitnessAction::Compile { laws, law_file, out } => {
                let pg_url = pg_url
                    .as_deref()
                    .unwrap_or(fractalaw_store::HUB_PG_URL);
                let law_names = resolve_law_names(laws.as_deref(), law_file.as_deref())?;
                let law_names = provenance::fitness_scope(Some(pg_url), law_names).await?;
                let duck = open_duck(&data_dir)?;
                let r = commands::fitness::cmd_fitness_compile(pg_url, &duck, law_names.as_deref(), out.as_deref()).await;
                if r.is_ok() && out.is_none() { provenance::record(Some(pg_url), law_names.as_deref(), provenance::fitness_compile()).await; }
                r
            }
        },

        // JSP enrichment pipeline (separate from legislation taxa).
        Command::Jsp { action } => commands::jsp::cmd_jsp(action, &open_duck(&data_dir)?),
    }
}

/// Resolve law names from --laws or --law-file arguments.
fn resolve_law_names(
    laws: Option<&str>,
    law_file: Option<&std::path::Path>,
) -> anyhow::Result<Option<Vec<String>>> {
    if let Some(l) = laws {
        Ok(Some(
            l.split(',').map(|s| s.trim().to_string()).collect(),
        ))
    } else if let Some(path) = law_file {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading law file: {}", path.display()))?;
        let names: Vec<String> = content
            .split(',')
            .flat_map(|s| s.split('\n'))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(Some(names))
    } else {
        Ok(None)
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
        );
        assert_ne!(h1, h2, "different input must produce different hash");
    }
}

