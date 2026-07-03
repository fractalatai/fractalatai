mod commands;
mod display;
mod embed;
mod llm;
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

    /// Use PostgreSQL+pgvector instead of LanceDB
    #[arg(long, global = true, env = "FRACTALAW_PG")]
    pg: Option<String>,

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
        #[arg(long, default_value = "./docs/clause_eyeball.md")]
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

/// Open the provision store (PgStore if --pg is set, otherwise LanceStore).
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
        let store = LanceStore::open(&data_dir.join("lancedb"))
            .await
            .context("opening LanceDB")?;
        Ok(Box::new(store))
    }
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
                cmd_taxa_eyeball(&data_dir, &law_names, &output, limit).await
            }
            TaxaAction::Qa { laws, family } => cmd_taxa_qa(&data_dir, laws, family).await,
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
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_infer(lance.as_ref(), &law_names).await
            }
            TaxaAction::Reconcile { laws } => {
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_reconcile(lance.as_ref(), &law_names).await
            }
            TaxaAction::Backfill { laws } => {
                let store = open_duck(&data_dir)?;
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                let mut total = 0usize;
                let mut sig_total = 0usize;
                let mut parts_total = 0usize;
                for law_name in &law_names {
                    let updated = lance.backfill_from_actors(law_name).await?;
                    let sig = lance.backfill_significance(law_name).await?;

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
                println!(
                    "Backfilled {total} provisions, {sig_total} significance, {parts_total} Part breakdowns across {} laws",
                    law_names.len()
                );
                Ok(())
            }
            TaxaAction::Slm { laws } => {
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_slm(lance.as_ref(), &law_names).await
            }
            TaxaAction::AuditFitness {
                laws,
                family,
                limit,
            } => cmd_taxa_audit_fitness(&data_dir, laws, family, limit).await,
            TaxaAction::Parse { laws, force, trace } => {
                let store = open_duck(&data_dir)?;
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_parse(lance.as_ref(), &store, &law_names, force).await?;
                if let Some(trace_path) = trace {
                    cmd_taxa_trace(lance.as_ref(), &store, &law_names, &trace_path).await?;
                }
                Ok(())
            }
            TaxaAction::Embed { laws } => {
                let store = open_duck(&data_dir)?;
                store.ensure_pipeline_status_columns()?;
                let lance = open_provision_store(&data_dir, pg_url.as_deref()).await?;
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                let result = cmd_taxa_embed(lance.as_ref(), &law_names).await;
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
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                let result = cmd_taxa_classify(lance.as_ref(), &law_names).await;
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
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                cmd_taxa_escalate(lance.as_ref(), &store, &law_names).await
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
                let law_names: Vec<String> =
                    laws.split(',').map(|s| s.trim().to_string()).collect();
                let result = cmd_taxa_validate(lance.as_ref(), &store, &law_names, &audit_dir, dry_run, apply).await;
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
}

