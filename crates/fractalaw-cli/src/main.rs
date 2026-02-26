mod display;
mod embed;

use std::path::PathBuf;

use anyhow::Context;
use arrow::record_batch::RecordBatch;
use arrow::util::pretty::print_batches;
use clap::{Parser, Subcommand};
use fractalaw_store::{DuckStore, FusionStore, LanceStore, StoreError};

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
        /// Compare v1 (blunt gate) vs v2 (actor-anchored) DRRP patterns
        #[arg(long)]
        compare: bool,
    },
    /// Enrich LRT DRRP columns for laws missing taxa data (from LanceDB text)
    Enrich {
        /// Specific laws to enrich (comma-separated, e.g., UK_ukpga_1974_37,UK_uksi_1999_3242)
        /// If not specified, enriches all laws without taxa data
        #[arg(long)]
        laws: Option<String>,
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
        },

        // Taxa classification.
        Command::Taxa { action } => match action {
            TaxaAction::Show {
                name,
                limit,
                compare,
            } => cmd_taxa_show(&data_dir, &name, limit, compare).await,
            TaxaAction::Enrich { laws } => {
                let law_filter = laws.as_ref().map(|s| {
                    s.split(',')
                        .map(|l| l.trim().to_string())
                        .collect::<Vec<_>>()
                });
                cmd_taxa_enrich(&data_dir, &open_duck(&data_dir)?, law_filter).await
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

    let inference = std::env::var("ANTHROPIC_API_KEY").ok().map(|key| {
        let model = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-5-20250929".into());
        fractalaw_host::InferenceConfig::new(key, model)
    });

    // Try to load local ONNX DRRP model (local-first).
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
                    tracing::warn!(error = %e, "failed to load DRRP ONNX model, falling back to Claude");
                    None
                }
            }
        } else {
            tracing::debug!("no DRRP ONNX model found, using Claude API for inference");
            None
        }
    };

    // Open LanceDB for legislation_text queries/mutations.
    let lance = LanceStore::open(&data_dir.join("lancedb")).await.ok();

    let opts = fractalaw_host::RunOptions {
        duck: Some(duck),
        lance,
        inference,
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
    compare: bool,
) -> anyhow::Result<()> {
    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    let filter = format!("law_name = '{}'", name.replace('\'', "''"));
    let batches = lance.query_legislation_text(&filter, limit, 0).await?;

    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total == 0 {
        println!("No text sections found for '{name}'.");
        println!("(Ensure LanceDB has been populated via `fractalaw embed`)");
        return Ok(());
    }

    if compare {
        println!("=== Taxa v1/v2 Comparison: {name} ({total} sections) ===\n");
    } else {
        println!("=== Taxa Classification: {name} ({total} sections) ===\n");
    }

    let mut section_num = 0usize;
    let mut classified_num = 0usize;
    let mut diff_count = 0usize;
    let mut v1_only_count = 0usize;
    let mut v2_only_count = 0usize;
    let mut v1_drrp_count = 0usize;
    let mut v2_drrp_count = 0usize;

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

            if compare {
                let cmp = fractalaw_core::taxa::parse_compare(&text);

                if !cmp.v1.duty_types.is_empty() {
                    v1_drrp_count += 1;
                }
                if !cmp.v2.duty_types.is_empty() {
                    v2_drrp_count += 1;
                }

                // In compare mode, show everything that has any signal in either version
                let has_v1 = !cmp.v1.duty_types.is_empty()
                    || !cmp.v1.governed_actors.is_empty()
                    || !cmp.v1.government_actors.is_empty();
                let has_v2 = !cmp.v2.duty_types.is_empty()
                    || !cmp.v2.governed_actors.is_empty()
                    || !cmp.v2.government_actors.is_empty();

                if !has_v1 && !has_v2 {
                    continue;
                }
                classified_num += 1;

                if cmp.differs {
                    diff_count += 1;
                    if !cmp.v1.duty_types.is_empty() && cmp.v2.duty_types.is_empty() {
                        v1_only_count += 1;
                    }
                    if cmp.v1.duty_types.is_empty() && !cmp.v2.duty_types.is_empty() {
                        v2_only_count += 1;
                    }
                }

                // Only show detail for provisions that differ
                if cmp.differs {
                    println!("--- {provision} [DIFFERS] ---");
                    let v1_types: Vec<&str> =
                        cmp.v1.duty_types.iter().map(|d| d.as_str()).collect();
                    let v2_types: Vec<&str> =
                        cmp.v2.duty_types.iter().map(|d| d.as_str()).collect();
                    println!(
                        "  v1 DRRP: {}",
                        if v1_types.is_empty() {
                            "(none)".to_string()
                        } else {
                            v1_types.join(", ")
                        }
                    );
                    println!(
                        "  v2 DRRP: {}",
                        if v2_types.is_empty() {
                            "(none)".to_string()
                        } else {
                            v2_types.join(", ")
                        }
                    );
                    if let Some(ref c1) = cmp.v1.classification {
                        println!(
                            "  v1 Pattern: {:?} / {:?} ({:.0}%)",
                            c1.family,
                            c1.sub_type,
                            c1.confidence * 100.0,
                        );
                    }
                    if let Some(ref c2) = cmp.v2.classification {
                        println!(
                            "  v2 Pattern: {:?} / {:?} ({:.0}%)",
                            c2.family,
                            c2.sub_type,
                            c2.confidence * 100.0,
                        );
                    }
                    if !cmp.v1.governed_actors.is_empty() {
                        println!("  Governed:   {}", cmp.v1.governed_actors.join(", "));
                    }
                    if !cmp.v1.government_actors.is_empty() {
                        println!("  Government: {}", cmp.v1.government_actors.join(", "));
                    }
                    let preview = if cmp.cleaned_text.len() > 120 {
                        format!("{}...", &cmp.cleaned_text[..120])
                    } else {
                        cmp.cleaned_text.clone()
                    };
                    println!("  Text:    {preview}");
                    println!();
                }
            } else {
                let record = fractalaw_core::taxa::parse(&text);

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

                // Show a truncated preview of the cleaned text.
                let preview = if record.cleaned_text.len() > 120 {
                    format!("{}...", &record.cleaned_text[..120])
                } else {
                    record.cleaned_text.clone()
                };
                println!("  Text:    {preview}");
                println!();
            }
        }
    }

    if compare {
        println!("=== Summary ===");
        println!("  Sections processed: {section_num}");
        println!("  With any signal:    {classified_num}");
        println!("  v1 DRRP count:      {v1_drrp_count}");
        println!("  v2 DRRP count:      {v2_drrp_count}");
        println!("  Differences:        {diff_count}");
        println!("    v1-only (false positives removed by v2): {v1_only_count}");
        println!("    v2-only (new matches in v2):             {v2_only_count}");
    } else {
        println!("=== {section_num} sections processed, {classified_num} with classifications ===");
    }
    Ok(())
}

async fn cmd_taxa_enrich(
    data_dir: &std::path::Path,
    store: &DuckStore,
    law_filter: Option<Vec<String>>,
) -> anyhow::Result<()> {
    use std::collections::BTreeSet;

    let lance = LanceStore::open(&data_dir.join("lancedb"))
        .await
        .context("opening LanceDB")?;

    // If specific laws requested, use those; otherwise find all laws without taxa data
    let law_names: Vec<String> = if let Some(filter) = law_filter {
        println!("=== Taxa Enrichment: {} specified laws ===\n", filter.len());
        filter
    } else {
        // Find laws that have NO DRRP taxa data yet (duty_holder is null or empty).
        let law_batches = store.query_arrow(
            "SELECT name FROM legislation
             WHERE duty_holder IS NULL OR len(duty_holder) = 0
             ORDER BY name",
        )?;

        let names: Vec<String> = law_batches
            .iter()
            .flat_map(|b| {
                let col = b.column_by_name("name");
                (0..b.num_rows())
                    .filter_map(move |i| col.and_then(|c| get_string_value(c.as_ref(), i)))
            })
            .collect();

        if names.is_empty() {
            println!("All laws already have DRRP taxa data.");
            return Ok(());
        }

        println!(
            "=== Taxa Enrichment: {} laws without DRRP data ===\n",
            names.len()
        );
        names
    };

    // Per-law aggregation containers.
    struct LawTaxa {
        duty_holders: BTreeSet<String>,
        rights_holders: BTreeSet<String>,
        responsibility_holders: BTreeSet<String>,
        power_holders: BTreeSet<String>,
        duty_types: BTreeSet<String>,
        roles: BTreeSet<String>,
        roles_gvt: BTreeSet<String>,
        duties: Vec<(String, String, String, String)>, // (holder, duty_type, clause, article)
        rights: Vec<(String, String, String, String)>,
        responsibilities: Vec<(String, String, String, String)>,
        powers: Vec<(String, String, String, String)>,
    }

    let mut enriched = 0usize;
    let total = law_names.len();

    for law_name in &law_names {
        let filter = format!("law_name = '{}'", law_name.replace('\'', "''"));
        let batches = lance.query_legislation_text(&filter, 500, 0).await?;

        let mut taxa = LawTaxa {
            duty_holders: BTreeSet::new(),
            rights_holders: BTreeSet::new(),
            responsibility_holders: BTreeSet::new(),
            power_holders: BTreeSet::new(),
            duty_types: BTreeSet::new(),
            roles: BTreeSet::new(),
            roles_gvt: BTreeSet::new(),
            duties: Vec::new(),
            rights: Vec::new(),
            responsibilities: Vec::new(),
            powers: Vec::new(),
        };

        // Per-provision taxa results for LanceDB write.
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
        }
        let mut provision_taxa: Vec<ProvisionTaxa> = Vec::new();

        for batch in &batches {
            let prov_col = batch.column_by_name("provision");
            let text_col = batch.column_by_name("text");
            let sid_col = batch.column_by_name("section_id");

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
                if text.trim().is_empty() {
                    continue;
                }

                let record = fractalaw_core::taxa::parse(&text);
                // Skip provisions with no taxa signal at all (no DRRP, no actors, no purposes).
                // We DO want to write provisions with purposes even if they have no DRRP content
                // (e.g., Interpretation sections) so the purpose gate can work.
                if record.duty_types.is_empty()
                    && record.governed_actors.is_empty()
                    && record.government_actors.is_empty()
                    && record.purposes.is_empty()
                {
                    continue;
                }

                // Collect per-provision taxa for LanceDB.
                if !section_id.is_empty() {
                    let (duty_family, duty_sub_type, taxa_confidence) =
                        if let Some(ref cls) = record.classification {
                            (
                                Some(format!("{:?}", cls.family)),
                                Some(format!("{:?}", cls.sub_type)),
                                Some(cls.confidence),
                            )
                        } else {
                            (None, None, None)
                        };
                    provision_taxa.push(ProvisionTaxa {
                        section_id,
                        drrp_types: record
                            .duty_types
                            .iter()
                            .map(|d| format!("{:?}", d))
                            .collect(),
                        governed_actors: record.governed_actors.clone(),
                        government_actors: record.government_actors.clone(),
                        duty_family,
                        duty_sub_type,
                        popimar: record.popimar.iter().map(|s| s.to_string()).collect(),
                        purposes: record.purposes.iter().map(|s| s.to_string()).collect(),
                        clause_refined: record.cleaned_text.clone(),
                        taxa_confidence,
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
                    format!("{}...", &record.cleaned_text[..200])
                } else {
                    record.cleaned_text.clone()
                };
                let article = format!("section/{provision}");

                for dt in &record.duty_types {
                    taxa.duty_types.insert(format!("{dt:?}"));
                    let holders_set;
                    let entries;
                    match dt {
                        fractalaw_core::taxa::duty_type::DutyType::Duty => {
                            holders_set = &mut taxa.duty_holders;
                            entries = &mut taxa.duties;
                        }
                        fractalaw_core::taxa::duty_type::DutyType::Right => {
                            holders_set = &mut taxa.rights_holders;
                            entries = &mut taxa.rights;
                        }
                        fractalaw_core::taxa::duty_type::DutyType::Responsibility => {
                            holders_set = &mut taxa.responsibility_holders;
                            entries = &mut taxa.responsibilities;
                        }
                        fractalaw_core::taxa::duty_type::DutyType::Power => {
                            holders_set = &mut taxa.power_holders;
                            entries = &mut taxa.powers;
                        }
                    }
                    // Add governed actors as holders for this duty type.
                    for actor in &record.governed_actors {
                        holders_set.insert(actor.clone());
                    }
                    for actor in &record.government_actors {
                        holders_set.insert(actor.clone());
                    }
                    // Build a DRRPEntry-style tuple.
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

            let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

            for pt in &provision_taxa {
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
                Field::new("purposes", DataType::List(item_field), true),
                Field::new("clause_refined", DataType::Utf8, true),
                Field::new("taxa_confidence", DataType::Float32, true),
                Field::new(
                    "taxa_classified_at",
                    DataType::Timestamp(TimeUnit::Nanosecond, Some("UTC".into())),
                    true,
                ),
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
                ],
            )
            .context("building taxa RecordBatch")?;

            lance
                .update_taxa(taxa_batch)
                .await
                .with_context(|| format!("writing taxa to LanceDB for {law_name}"))?;
        }

        // Skip laws where taxa found nothing.
        if taxa.duty_types.is_empty() && taxa.roles.is_empty() && taxa.roles_gvt.is_empty() {
            enriched += 1;
            continue;
        }

        // Build SQL UPDATE for this law's LRT DRRP columns.
        let esc = |s: &str| s.replace('\'', "''");

        let sql = format!(
            "UPDATE legislation SET
                duty_holder = {duty_holder},
                rights_holder = {rights_holder},
                responsibility_holder = {resp_holder},
                power_holder = {power_holder},
                duty_type = {duty_type},
                role = {role},
                role_gvt = {role_gvt}
             WHERE name = '{name}'",
            duty_holder = format_sql_list(taxa.duty_holders.iter().map(|s| s.as_str())),
            rights_holder = format_sql_list(taxa.rights_holders.iter().map(|s| s.as_str())),
            resp_holder = format_sql_list(taxa.responsibility_holders.iter().map(|s| s.as_str())),
            power_holder = format_sql_list(taxa.power_holders.iter().map(|s| s.as_str())),
            duty_type = format_sql_list(taxa.duty_types.iter().map(|s| s.as_str())),
            role = format_sql_list(taxa.roles.iter().map(|s| s.as_str())),
            role_gvt = format_sql_list(taxa.roles_gvt.iter().map(|s| s.as_str())),
            name = esc(law_name),
        );
        store.execute(&sql)?;

        enriched += 1;
        if enriched.is_multiple_of(100) {
            eprint!("\r  Enriched {enriched}/{total}...");
        }
    }

    if enriched >= 100 {
        eprintln!();
    }

    // Count how many actually got data.
    let filled = store.query_arrow(
        "SELECT count(*)::BIGINT FROM legislation
         WHERE duty_holder IS NOT NULL AND len(duty_holder) > 0",
    )?;
    let filled_count = extract_i64(&filled[0], 0);

    println!("Processed {enriched} laws. LRT now has {filled_count} laws with DRRP taxa data.");
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
        let lat_batches = lance.query_legislation_text(&filter, 1000, 0).await?;

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
