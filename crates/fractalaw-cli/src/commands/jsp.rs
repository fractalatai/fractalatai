//! CLI commands for JSP (Joint Service Publication) enrichment.
//!
//! Runs the JSP-specific DRRP parser on provision text and stages
//! results in DuckDB. Publishing to sertantai via Zenoh is handled
//! by fractalaw-sync-cli (publish-secondary), not here.

use anyhow::{Context, Result};
use clap::Subcommand;
use fractalaw_core::jsp;
use fractalaw_store::DuckStore;

#[derive(Subcommand)]
pub enum JspAction {
    /// Run JSP DRRP enrichment on provisions from DuckDB staging
    Enrich {
        /// Source identifier (e.g., JSP-375-CH23)
        source_id: String,
        /// Show parse results without writing to DuckDB
        #[arg(long)]
        dry_run: bool,
    },

    /// Show enrichment stats for JSP sources in DuckDB
    Stats,
}

/// Dispatch a JSP subcommand.
pub fn cmd_jsp(action: JspAction, duck: &DuckStore) -> Result<()> {
    match action {
        JspAction::Enrich { source_id, dry_run } => cmd_enrich(duck, &source_id, dry_run),
        JspAction::Stats => cmd_stats(duck),
    }
}

/// Enrich JSP provisions with DRRP classification.
///
/// Reads provision text from the `jsp_provisions` DuckDB staging table
/// (populated by `fractalaw-sync pull-secondary`), runs the JSP parser,
/// and writes enrichment results to `jsp_enrichment` DuckDB table.
fn cmd_enrich(duck: &DuckStore, source_id: &str, dry_run: bool) -> Result<()> {
    // Read provisions from DuckDB staging
    let sql = format!(
        "SELECT section_id, text FROM jsp_provisions WHERE source_id = '{}' AND text IS NOT NULL ORDER BY position",
        source_id.replace('\'', "''")
    );
    let batches = duck.query_arrow(&sql);

    let batches = match batches {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Table 'jsp_provisions' not found. Pull provisions first:");
            eprintln!("  fractalaw-sync pull-secondary --source-id {source_id} --tenant dev --connect tcp/localhost:7447");
            return Ok(());
        }
    };

    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total_rows == 0 {
        println!("No provisions found for {source_id}");
        return Ok(());
    }

    println!("Enriching {total_rows} provisions from {source_id}...");

    let mut mandatory = 0usize;
    let mut recommended = 0usize;
    let mut permissive = 0usize;
    let mut no_obligation = 0usize;
    let mut actors_found = std::collections::HashMap::<String, usize>::new();

    // Collect results for DuckDB insert
    let mut results: Vec<(String, jsp::JspRecord)> = Vec::new();

    for batch in &batches {
        use arrow::array::{Array, StringArray};

        let sid_col: Option<&StringArray> = batch
            .column_by_name("section_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let text_col: Option<&StringArray> = batch
            .column_by_name("text")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

        for row in 0..batch.num_rows() {
            let sid = sid_col
                .and_then(|c: &StringArray| if c.is_valid(row) { Some(c.value(row)) } else { None })
                .unwrap_or("");
            let text = text_col
                .and_then(|c: &StringArray| if c.is_valid(row) { Some(c.value(row)) } else { None })
                .unwrap_or("");

            let record = jsp::parse(text);

            match record.strength {
                Some("Mandatory") => mandatory += 1,
                Some("Recommended") => recommended += 1,
                Some("Permissive") => permissive += 1,
                _ => no_obligation += 1,
            }

            for actor in &record.governed_actors {
                *actors_found.entry(actor.clone()).or_default() += 1;
            }
            for actor in &record.government_actors {
                *actors_found.entry(actor.clone()).or_default() += 1;
            }

            results.push((sid.to_string(), record));
        }
    }

    // Print stats
    println!("\nResults for {source_id}:");
    println!("  Mandatory:      {mandatory}");
    println!("  Recommended:    {recommended}");
    println!("  Permissive:     {permissive}");
    println!("  No obligation:  {no_obligation}");

    if !actors_found.is_empty() {
        println!("\nActors found:");
        let mut sorted: Vec<_> = actors_found.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        for (actor, count) in &sorted {
            println!("  {count:>4}  {actor}");
        }
    }

    if dry_run {
        println!("\n(dry run — not writing to DuckDB)");
        return Ok(());
    }

    // Create staging table and insert results
    duck.execute(
        "CREATE TABLE IF NOT EXISTS jsp_enrichment (
            section_id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL,
            drrp_types TEXT,
            governed_actors TEXT,
            government_actors TEXT,
            obligation_strength TEXT,
            modal_verb TEXT,
            clause_refined TEXT,
            enriched_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    ).context("failed to create jsp_enrichment table")?;

    // Clear existing results for this source
    duck.execute(&format!(
        "DELETE FROM jsp_enrichment WHERE source_id = '{}'",
        source_id.replace('\'', "''")
    )).context("failed to clear existing enrichment")?;

    let mut inserted = 0usize;
    for (sid, record) in &results {
        let drrp = record.duty_types.join(",");
        let governed = record.governed_actors.join(",");
        let government = record.government_actors.join(",");
        let strength = record.strength.unwrap_or("");
        let modal = record.modal_verb.unwrap_or("");
        let clause = record.clause_refined.as_deref().unwrap_or("").replace('\'', "''");

        duck.execute(&format!(
            "INSERT INTO jsp_enrichment (section_id, source_id, drrp_types, governed_actors, government_actors, obligation_strength, modal_verb, clause_refined)
             VALUES ('{}', '{}', NULLIF('{}',''), NULLIF('{}',''), NULLIF('{}',''), NULLIF('{}',''), NULLIF('{}',''), NULLIF('{}',''))",
            sid.replace('\'', "''"),
            source_id.replace('\'', "''"),
            drrp, governed, government, strength, modal, clause
        )).context("failed to insert enrichment")?;
        inserted += 1;
    }

    println!("\nStaged {inserted} enrichment rows in DuckDB (jsp_enrichment)");
    println!("Publish with: fractalaw-sync publish-secondary --source-id {source_id} --tenant dev --connect tcp/localhost:7447");
    Ok(())
}

/// Show enrichment stats for JSP sources in DuckDB.
fn cmd_stats(duck: &DuckStore) -> Result<()> {
    let sql = "SELECT source_id, count(*) as provisions, \
               count(drrp_types) as enriched, \
               count(CASE WHEN obligation_strength = 'Mandatory' THEN 1 END) as mandatory, \
               count(CASE WHEN obligation_strength = 'Recommended' THEN 1 END) as recommended, \
               count(CASE WHEN obligation_strength = 'Permissive' THEN 1 END) as permissive \
               FROM jsp_enrichment GROUP BY source_id ORDER BY source_id";

    match duck.query_arrow(sql) {
        Ok(batches) => {
            let total: usize = batches.iter().map(|b| b.num_rows()).sum();
            if total == 0 {
                println!("No JSP enrichment data in DuckDB. Run 'fractalaw jsp enrich' first.");
                return Ok(());
            }
            println!("{}", arrow::util::pretty::pretty_format_batches(&batches)?);
            Ok(())
        }
        Err(_) => {
            println!("Table 'jsp_enrichment' not found. Run 'fractalaw jsp enrich' first.");
            Ok(())
        }
    }
}
