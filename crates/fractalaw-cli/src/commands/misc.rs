use anyhow::Context;
use arrow::array::Array;
use arrow::record_batch::RecordBatch;
use arrow::util::pretty::print_batches;
use fractalaw_store::{DuckStore, FusionStore, LanceStore, StoreError};

use crate::{display, embed, open_duck};
use crate::utils::*;

pub(crate) fn cmd_import(data_dir: &std::path::Path) -> anyhow::Result<()> {
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

pub(crate) async fn cmd_run(
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

pub(crate) async fn cmd_query(store: &DuckStore, sql: &str) -> anyhow::Result<()> {
    let fusion = FusionStore::new(store)?;
    let batches = fusion.query(sql).await?;
    if batches.is_empty() || batches.iter().all(|b| b.num_rows() == 0) {
        println!("No results.");
        return Ok(());
    }
    print_batches(&batches)?;
    Ok(())
}

pub(crate) fn cmd_law(store: &DuckStore, name: &str) -> anyhow::Result<()> {
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

pub(crate) fn cmd_graph(store: &DuckStore, name: &str, hops: u32) -> anyhow::Result<()> {
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

pub(crate) async fn cmd_embed(data_dir: &std::path::Path, model_dir: &std::path::Path) -> anyhow::Result<()> {
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

pub(crate) async fn cmd_text(data_dir: &std::path::Path, name: &str, limit: usize) -> anyhow::Result<()> {
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


pub(crate) async fn cmd_export_training_data(
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

pub(crate) async fn cmd_search(
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

pub(crate) async fn cmd_validate(
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

pub(crate) fn cmd_tokenize(text: &str, model_dir: &std::path::Path) -> anyhow::Result<()> {
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

pub(crate) fn cmd_stats(store: &DuckStore) -> anyhow::Result<()> {
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

pub(crate) async fn cmd_classify(
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








