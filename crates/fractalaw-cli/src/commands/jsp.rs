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

    /// Extract cross-references (legislation, JSPs, standards) from JSP provisions
    ExtractRefs {
        /// Source identifier (e.g., JSP-375-CH23)
        source_id: String,
        /// Show extracted references without writing to DuckDB
        #[arg(long)]
        dry_run: bool,
    },

    /// Trace: find which JSP provisions reference a given legislation provision or law
    Trace {
        /// Target identifier — a law_name (e.g., UK_uksi_1989_635) or section_id
        target_id: String,
    },

    /// Show enrichment stats for JSP sources in DuckDB
    Stats,
}

/// Dispatch a JSP subcommand.
pub fn cmd_jsp(action: JspAction, duck: &DuckStore) -> Result<()> {
    match action {
        JspAction::Enrich { source_id, dry_run } => cmd_enrich(duck, &source_id, dry_run),
        JspAction::ExtractRefs { source_id, dry_run } => cmd_extract_refs(duck, &source_id, dry_run),
        JspAction::Trace { target_id } => cmd_trace(duck, &target_id),
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

/// Extract cross-references from JSP provisions and stage in DuckDB.
fn cmd_extract_refs(duck: &DuckStore, source_id: &str, dry_run: bool) -> Result<()> {
    let sql = format!(
        "SELECT section_id, text FROM jsp_provisions WHERE source_id = '{}' AND text IS NOT NULL ORDER BY position",
        source_id.replace('\'', "''")
    );
    let batches = match duck.query_arrow(&sql) {
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

    println!("Extracting references from {total_rows} provisions in {source_id}...");

    let mut all_refs: Vec<(String, jsp::references::JspReference)> = Vec::new();
    let mut by_type: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

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

            if text.is_empty() { continue; }

            let refs = jsp::references::extract_references(text);
            for r in refs {
                *by_type.entry(r.target_type).or_default() += 1;
                all_refs.push((sid.to_string(), r));
            }
        }
    }

    // Print summary
    println!("\nReferences found: {}", all_refs.len());
    for (ref_type, count) in &by_type {
        println!("  {ref_type}: {count}");
    }

    // Show sample references
    if !all_refs.is_empty() {
        println!("\nSample references:");
        for (sid, r) in all_refs.iter().take(20) {
            let short_sid = sid.rsplit_once(':').map(|(_, s)| s).unwrap_or(sid);
            println!("  [{:>11}] {short_sid} → {}", r.target_type, r.citation);
        }
        if all_refs.len() > 20 {
            println!("  ... and {} more", all_refs.len() - 20);
        }
    }

    // Resolve references against local data
    println!("\nResolving references...");
    let mut resolved_count = 0usize;

    // Build legislation lookup: title → name, keyed by year
    let leg_lookup: Vec<(String, String, u32)> = match duck.query_arrow(
        "SELECT name, title, year FROM legislation WHERE title IS NOT NULL"
    ) {
        Ok(batches) => {
            let mut entries = Vec::new();
            for batch in &batches {
                use arrow::array::{Array, StringArray, Int64Array};
                let name_col = batch.column_by_name("name").and_then(|c| c.as_any().downcast_ref::<StringArray>());
                let title_col = batch.column_by_name("title").and_then(|c| c.as_any().downcast_ref::<StringArray>());
                let year_col = batch.column_by_name("year").and_then(|c| c.as_any().downcast_ref::<Int64Array>());
                for row in 0..batch.num_rows() {
                    let name = name_col.and_then(|c| if c.is_valid(row) { Some(c.value(row).to_string()) } else { None });
                    let title = title_col.and_then(|c| if c.is_valid(row) { Some(c.value(row).to_string()) } else { None });
                    let year = year_col.and_then(|c| if c.is_valid(row) { Some(c.value(row) as u32) } else { None });
                    if let (Some(n), Some(t), Some(y)) = (name, title, year) {
                        entries.push((n, t, y));
                    }
                }
            }
            entries
        }
        Err(_) => Vec::new(),
    };

    // Resolve each reference
    for (_sid, r) in all_refs.iter_mut() {
        match r.target_type {
            "legislation" => {
                if let Some((keywords, year)) = jsp::resolve::normalise_legislation_citation(&r.citation) {
                    // Find best match by year + keyword overlap
                    let mut best: Option<(&str, f32)> = None;
                    for (name, title, leg_year) in &leg_lookup {
                        if *leg_year != year { continue; }
                        let score = jsp::resolve::match_score(title, &keywords);
                        if score >= 0.6 {
                            if best.is_none() || score > best.unwrap().1 {
                                best = Some((name.as_str(), score));
                            }
                        }
                    }
                    if let Some((name, _score)) = best {
                        r.resolved_id = Some(name.to_string());
                        resolved_count += 1;
                    }
                }
            }
            "jsp" => {
                if let Some(source_id) = jsp::resolve::normalise_jsp_citation(&r.citation) {
                    r.resolved_id = Some(source_id);
                    resolved_count += 1;
                }
            }
            _ => {
                // Standards and guidance — no resolution yet
            }
        }
    }

    println!("Resolved {resolved_count}/{} references", all_refs.len());

    // Print resolved legislation
    let leg_resolved: Vec<_> = all_refs.iter()
        .filter(|(_, r)| r.target_type == "legislation" && r.resolved_id.is_some())
        .collect();
    if !leg_resolved.is_empty() {
        println!("\nLegislation resolved:");
        for (_, r) in &leg_resolved {
            println!("  {} → {}", r.citation.chars().take(60).collect::<String>(), r.resolved_id.as_deref().unwrap());
        }
    }

    let jsp_resolved: Vec<_> = all_refs.iter()
        .filter(|(_, r)| r.target_type == "jsp" && r.resolved_id.is_some())
        .collect();
    if !jsp_resolved.is_empty() {
        println!("\nJSP cross-references resolved:");
        let mut seen = std::collections::HashSet::new();
        for (_, r) in &jsp_resolved {
            let id = r.resolved_id.as_deref().unwrap();
            if seen.insert(id) {
                println!("  {} → {}", r.citation, id);
            }
        }
    }

    if dry_run {
        println!("\n(dry run — not writing to DuckDB)");
        return Ok(());
    }

    // Create staging table
    duck.execute(
        "CREATE TABLE IF NOT EXISTS jsp_references (
            reference_id TEXT PRIMARY KEY,
            source_section_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            target_type TEXT NOT NULL,
            citation TEXT NOT NULL,
            target_id TEXT,
            resolved BOOLEAN DEFAULT FALSE,
            extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    ).context("failed to create jsp_references table")?;

    // Clear existing references for this source
    duck.execute(&format!(
        "DELETE FROM jsp_references WHERE source_id = '{}'",
        source_id.replace('\'', "''")
    )).context("failed to clear existing references")?;

    let mut inserted = 0usize;
    for (i, (sid, r)) in all_refs.iter().enumerate() {
        let ref_id = format!("{sid}:ref.{i}");
        let esc = |s: &str| s.replace('\'', "''");

        let target_id_sql = r.resolved_id.as_deref()
            .map(|id| format!("'{}'", esc(id)))
            .unwrap_or_else(|| "NULL".to_string());
        let resolved_sql = if r.resolved_id.is_some() { "TRUE" } else { "FALSE" };

        duck.execute(&format!(
            "INSERT INTO jsp_references (reference_id, source_section_id, source_id, target_type, citation, target_id, resolved)
             VALUES ('{}', '{}', '{}', '{}', '{}', {}, {})",
            esc(&ref_id), esc(sid), esc(source_id), r.target_type, esc(&r.citation), target_id_sql, resolved_sql
        )).context("failed to insert reference")?;
        inserted += 1;
    }

    println!("\nStaged {inserted} references in DuckDB (jsp_references)");
    Ok(())
}

/// Trace: find JSP provisions that reference a given law or section.
fn cmd_trace(duck: &DuckStore, target_id: &str) -> Result<()> {
    let safe = target_id.replace('\'', "''");

    // Match by exact target_id or by prefix (law_name matches all sections)
    let sql = format!(
        "SELECT r.source_section_id, r.source_id, r.target_type, r.citation, r.target_id, \
                p.text \
         FROM jsp_references r \
         LEFT JOIN jsp_provisions p ON r.source_section_id = p.section_id \
         WHERE r.target_id = '{safe}' OR r.target_id LIKE '{safe}:%' \
         ORDER BY r.source_id, r.source_section_id"
    );

    match duck.query_arrow(&sql) {
        Ok(batches) => {
            let total: usize = batches.iter().map(|b| b.num_rows()).sum();
            if total == 0 {
                // Try citation text search as fallback
                let citation_sql = format!(
                    "SELECT r.source_section_id, r.source_id, r.target_type, r.citation, r.target_id, \
                            p.text \
                     FROM jsp_references r \
                     LEFT JOIN jsp_provisions p ON r.source_section_id = p.section_id \
                     WHERE r.citation LIKE '%{safe}%' \
                     ORDER BY r.source_id, r.source_section_id"
                );
                match duck.query_arrow(&citation_sql) {
                    Ok(b2) => {
                        let total2: usize = b2.iter().map(|b| b.num_rows()).sum();
                        if total2 == 0 {
                            println!("No JSP references found for '{target_id}'");
                            println!("Run 'fractalaw jsp extract-refs' first to populate the reference table.");
                            return Ok(());
                        }
                        print_trace_results(&b2, target_id);
                    }
                    Err(_) => {
                        println!("Table 'jsp_references' not found. Run 'fractalaw jsp extract-refs' first.");
                    }
                }
                return Ok(());
            }
            print_trace_results(&batches, target_id);
            Ok(())
        }
        Err(_) => {
            println!("Table 'jsp_references' not found. Run 'fractalaw jsp extract-refs' first.");
            Ok(())
        }
    }
}

/// Print trace results in a readable format.
fn print_trace_results(batches: &[arrow::record_batch::RecordBatch], target_id: &str) {
    use arrow::array::{Array, StringArray};

    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    println!("JSP provisions referencing '{target_id}': {total}\n");

    for batch in batches {
        let sid_col: Option<&StringArray> = batch.column_by_name("source_section_id")
            .and_then(|c| c.as_any().downcast_ref());
        let src_col: Option<&StringArray> = batch.column_by_name("source_id")
            .and_then(|c| c.as_any().downcast_ref());
        let citation_col: Option<&StringArray> = batch.column_by_name("citation")
            .and_then(|c| c.as_any().downcast_ref());
        let text_col: Option<&StringArray> = batch.column_by_name("text")
            .and_then(|c| c.as_any().downcast_ref());

        for row in 0..batch.num_rows() {
            let sid = sid_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
            let src = src_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
            let citation = citation_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
            let text = text_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");

            // Shorten the section_id for readability
            let short_sid = sid.rsplit_once(':').map(|(_, s)| s).unwrap_or(sid);

            println!("[{src}] {short_sid}");
            println!("  Citation: {citation}");
            if !text.is_empty() {
                let truncated = if text.len() > 150 { format!("{}...", &text[..150]) } else { text.to_string() };
                println!("  Text: {truncated}");
            }
            println!();
        }
    }
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
