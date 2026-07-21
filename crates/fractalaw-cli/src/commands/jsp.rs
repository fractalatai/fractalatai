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

    /// Extract individual obligations and RACI assignments from JSP provisions
    ExtractObligations {
        /// Source identifier (e.g., JSP-375-CH23)
        source_id: String,
        /// Show extracted obligations without writing to DuckDB
        #[arg(long)]
        dry_run: bool,
    },

    /// Query RACI assignments for a role across all JSP sources
    Raci {
        /// Role label or fragment (e.g., "Commanding Officer", "Contractor")
        role: String,
    },

    /// Extract mandated artefacts (risk assessments, safety cases, permits, etc.) from obligations
    ExtractArtefacts {
        /// Source identifier (e.g., JSP-375-CH23)
        source_id: String,
        /// Show extracted artefacts without writing to DuckDB
        #[arg(long)]
        dry_run: bool,
    },

    /// Query mandated artefacts by type across all JSP sources
    Artefacts {
        /// Filter by artefact type (e.g., "Risk Assessment", "Permit")
        #[arg(long, name = "type")]
        artefact_type: Option<String>,
    },

    /// Extract terms and acronyms from JSP provisions
    ExtractTerms {
        /// Source identifier (e.g., JSP-375-CH23)
        source_id: String,
        /// Show extracted terms without writing to DuckDB
        #[arg(long)]
        dry_run: bool,
    },

    /// Show term conflicts across JSP sources
    Terms {
        /// Show only conflicts (same acronym, different expansion)
        #[arg(long)]
        conflicts: bool,
    },

    /// Generate controls from JSP mandated artefacts (additive to legislation controls)
    Controls {
        /// Source identifier (e.g., JSP-375-CH23)
        source_id: String,
        /// Show generated controls without writing to DuckDB
        #[arg(long)]
        dry_run: bool,
    },

    /// Traceability gap analysis: legislative obligations with no JSP implementation
    Gaps {
        /// Source identifier — a JSP chapter (e.g., JSP-375-CH23) or parent (e.g., JSP-375)
        source_id: String,
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
        JspAction::ExtractObligations { source_id, dry_run } => cmd_extract_obligations(duck, &source_id, dry_run),
        JspAction::Raci { role } => cmd_raci(duck, &role),
        JspAction::ExtractTerms { source_id, dry_run } => cmd_extract_terms(duck, &source_id, dry_run),
        JspAction::Terms { conflicts } => cmd_terms(duck, conflicts),
        JspAction::ExtractArtefacts { source_id, dry_run } => cmd_extract_artefacts(duck, &source_id, dry_run),
        JspAction::Artefacts { artefact_type } => cmd_artefacts(duck, artefact_type.as_deref()),
        JspAction::Controls { source_id, dry_run } => cmd_controls(duck, &source_id, dry_run),
        JspAction::Gaps { source_id } => cmd_gaps(duck, &source_id),
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

/// Extract obligations and RACI assignments from JSP provisions.
fn cmd_extract_obligations(duck: &DuckStore, source_id: &str, dry_run: bool) -> Result<()> {
    let sql = format!(
        "SELECT section_id, text FROM jsp_provisions WHERE source_id = '{}' AND text IS NOT NULL ORDER BY position",
        source_id.replace('\'', "''")
    );
    let batches = match duck.query_arrow(&sql) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Table 'jsp_provisions' not found. Pull provisions first.");
            return Ok(());
        }
    };

    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total_rows == 0 {
        println!("No provisions found for {source_id}");
        return Ok(());
    }

    println!("Extracting obligations from {total_rows} provisions in {source_id}...");

    let mut all_obligations: Vec<(String, jsp::obligations::Obligation)> = Vec::new();
    let mut raci_counts: std::collections::HashMap<String, std::collections::HashMap<&str, usize>> =
        std::collections::HashMap::new();

    for batch in &batches {
        use arrow::array::{Array, StringArray};

        let sid_col: Option<&StringArray> = batch.column_by_name("section_id")
            .and_then(|c| c.as_any().downcast_ref());
        let text_col: Option<&StringArray> = batch.column_by_name("text")
            .and_then(|c| c.as_any().downcast_ref());

        for row in 0..batch.num_rows() {
            let sid = sid_col.and_then(|c: &StringArray| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");
            let text = text_col.and_then(|c: &StringArray| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");
            if text.is_empty() { continue; }

            let obs = jsp::obligations::extract_obligations(text);
            for ob in obs {
                for r in &ob.raci {
                    raci_counts
                        .entry(r.role_label.clone())
                        .or_default()
                        .entry(r.assignment_type)
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                }
                all_obligations.push((sid.to_string(), ob));
            }
        }
    }

    // Print summary
    println!("\nObligations extracted: {}", all_obligations.len());

    let mandatory = all_obligations.iter().filter(|(_, o)| o.strength == Some("Mandatory")).count();
    let recommended = all_obligations.iter().filter(|(_, o)| o.strength == Some("Recommended")).count();
    let permissive = all_obligations.iter().filter(|(_, o)| o.strength == Some("Permissive")).count();
    println!("  Mandatory: {mandatory}");
    println!("  Recommended: {recommended}");
    println!("  Permissive: {permissive}");

    let with_competence = all_obligations.iter().filter(|(_, o)| !o.competence.is_empty()).count();
    if with_competence > 0 {
        println!("  With competence requirements: {with_competence}");
    }

    if !raci_counts.is_empty() {
        println!("\nRACI assignments:");
        let mut sorted: Vec<_> = raci_counts.iter().collect();
        sorted.sort_by_key(|(label, _)| label.clone());
        for (role, types) in &sorted {
            let r = types.get("R").unwrap_or(&0);
            let a = types.get("A").unwrap_or(&0);
            let c = types.get("C").unwrap_or(&0);
            let i = types.get("I").unwrap_or(&0);
            println!("  {role}: R={r} A={a} C={c} I={i}");
        }
    }

    if dry_run {
        println!("\n(dry run — not writing to DuckDB)");
        return Ok(());
    }

    // Create staging tables
    duck.execute(
        "CREATE TABLE IF NOT EXISTS jsp_obligations (
            obligation_id TEXT PRIMARY KEY,
            section_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            obligation_index INTEGER,
            text TEXT NOT NULL,
            modal_verb TEXT,
            strength TEXT,
            clause_refined TEXT,
            competence_requirements TEXT,
            extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    ).context("failed to create jsp_obligations table")?;

    duck.execute(
        "CREATE TABLE IF NOT EXISTS jsp_raci (
            raci_id TEXT PRIMARY KEY,
            obligation_id TEXT NOT NULL,
            section_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            role_label TEXT NOT NULL,
            assignment_type TEXT NOT NULL,
            assignment_source TEXT NOT NULL,
            extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    ).context("failed to create jsp_raci table")?;

    // Clear existing data for this source
    let safe = source_id.replace('\'', "''");
    duck.execute(&format!("DELETE FROM jsp_obligations WHERE source_id = '{safe}'"))?;
    duck.execute(&format!("DELETE FROM jsp_raci WHERE source_id = '{safe}'"))?;

    let esc = |s: &str| s.replace('\'', "''");
    let mut ob_count = 0usize;
    let mut raci_count = 0usize;

    for (sid, ob) in &all_obligations {
        let ob_id = format!("{sid}:ob.{}", ob.index);
        let competence = ob.competence.join(",");

        duck.execute(&format!(
            "INSERT INTO jsp_obligations (obligation_id, section_id, source_id, obligation_index, text, modal_verb, strength, clause_refined, competence_requirements)
             VALUES ('{}', '{}', '{}', {}, '{}', {}, {}, {}, {})",
            esc(&ob_id), esc(sid), esc(source_id), ob.index,
            esc(&ob.text),
            ob.modal_verb.map(|m| format!("'{m}'")).unwrap_or("NULL".into()),
            ob.strength.map(|s| format!("'{s}'")).unwrap_or("NULL".into()),
            ob.clause_refined.as_deref().map(|c| format!("'{}'", esc(c))).unwrap_or("NULL".into()),
            if competence.is_empty() { "NULL".into() } else { format!("'{}'", esc(&competence)) },
        ))?;
        ob_count += 1;

        for (ri, r) in ob.raci.iter().enumerate() {
            let raci_id = format!("{ob_id}:raci.{ri}");
            duck.execute(&format!(
                "INSERT INTO jsp_raci (raci_id, obligation_id, section_id, source_id, role_label, assignment_type, assignment_source)
                 VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}')",
                esc(&raci_id), esc(&ob_id), esc(sid), esc(source_id),
                esc(&r.role_label), r.assignment_type, r.source,
            ))?;
            raci_count += 1;
        }
    }

    println!("\nStaged {ob_count} obligations and {raci_count} RACI assignments in DuckDB");
    Ok(())
}

/// Query RACI assignments for a role across all JSP sources.
fn cmd_raci(duck: &DuckStore, role: &str) -> Result<()> {
    let safe = role.replace('\'', "''");
    let sql = format!(
        "SELECT r.role_label, r.assignment_type, r.source_id, \
                o.strength, o.text \
         FROM jsp_raci r \
         JOIN jsp_obligations o ON r.obligation_id = o.obligation_id \
         WHERE r.role_label LIKE '%{safe}%' \
         ORDER BY r.role_label, r.source_id, r.assignment_type"
    );

    match duck.query_arrow(&sql) {
        Ok(batches) => {
            let total: usize = batches.iter().map(|b| b.num_rows()).sum();
            if total == 0 {
                println!("No RACI assignments found for '{role}'");
                println!("Run 'fractalaw jsp extract-obligations' first.");
                return Ok(());
            }

            use arrow::array::{Array, StringArray};

            println!("RACI assignments matching '{role}': {total}\n");
            for batch in &batches {
                let role_col: Option<&StringArray> = batch.column_by_name("role_label").and_then(|c| c.as_any().downcast_ref());
                let type_col: Option<&StringArray> = batch.column_by_name("assignment_type").and_then(|c| c.as_any().downcast_ref());
                let src_col: Option<&StringArray> = batch.column_by_name("source_id").and_then(|c| c.as_any().downcast_ref());
                let strength_col: Option<&StringArray> = batch.column_by_name("strength").and_then(|c| c.as_any().downcast_ref());
                let text_col: Option<&StringArray> = batch.column_by_name("text").and_then(|c| c.as_any().downcast_ref());

                for row in 0..batch.num_rows() {
                    let rl = role_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
                    let at = type_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
                    let src = src_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
                    let strength = strength_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
                    let text = text_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");

                    let truncated = if text.len() > 120 { format!("{}...", &text[..120]) } else { text.to_string() };
                    println!("[{at}] {rl} ({src}, {strength})");
                    println!("  {truncated}");
                    println!();
                }
            }
            Ok(())
        }
        Err(_) => {
            println!("Table 'jsp_raci' not found. Run 'fractalaw jsp extract-obligations' first.");
            Ok(())
        }
    }
}

/// Extract terms and acronyms from JSP provisions.
fn cmd_extract_terms(duck: &DuckStore, source_id: &str, dry_run: bool) -> Result<()> {
    let safe = source_id.replace('\'', "''");
    let sql = format!(
        "SELECT section_id, text FROM jsp_provisions WHERE source_id = '{safe}' AND text IS NOT NULL ORDER BY position"
    );
    let batches = match duck.query_arrow(&sql) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Table 'jsp_provisions' not found. Pull provisions first.");
            return Ok(());
        }
    };

    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total == 0 {
        println!("No provisions found for {source_id}");
        return Ok(());
    }

    println!("Extracting terms from {total} provisions in {source_id}...");

    let mut all_terms: Vec<(String, jsp::terms::JspTerm)> = Vec::new();

    for batch in &batches {
        use arrow::array::{Array, StringArray};
        let sid_col: Option<&StringArray> = batch.column_by_name("section_id").and_then(|c| c.as_any().downcast_ref());
        let text_col: Option<&StringArray> = batch.column_by_name("text").and_then(|c| c.as_any().downcast_ref());

        for row in 0..batch.num_rows() {
            let sid = sid_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");
            let text = text_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");
            if text.is_empty() { continue; }

            let terms = jsp::terms::extract_terms(text);
            for t in terms {
                all_terms.push((sid.to_string(), t));
            }
        }
    }

    // Dedup across provisions (same acronym from different provisions)
    let mut seen = std::collections::HashSet::new();
    all_terms.retain(|(_, t)| seen.insert(t.normalised.clone()));

    println!("\nTerms extracted: {}", all_terms.len());
    for (sid, t) in &all_terms {
        let short = sid.rsplit_once(':').map(|(_, s)| s).unwrap_or(sid);
        match &t.acronym {
            Some(acr) => println!("  {acr} = {} (from {short})", t.term),
            None => println!("  \"{}\" (from {short})", t.term),
        }
    }

    if dry_run {
        println!("\n(dry run — not writing to DuckDB)");
        return Ok(());
    }

    duck.execute(
        "CREATE TABLE IF NOT EXISTS jsp_terms (
            term_id TEXT PRIMARY KEY,
            section_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            term TEXT NOT NULL,
            acronym TEXT,
            normalised TEXT NOT NULL,
            extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    ).context("failed to create jsp_terms table")?;

    duck.execute(&format!("DELETE FROM jsp_terms WHERE source_id = '{safe}'"))?;

    let esc = |s: &str| s.replace('\'', "''");
    let mut count = 0usize;
    for (sid, t) in &all_terms {
        if t.normalised.is_empty() { continue; }
        let term_id = format!("{source_id}:{}", t.normalised);
        let acr_sql = t.acronym.as_deref().map(|a| format!("'{}'", esc(a))).unwrap_or("NULL".into());
        duck.execute(&format!(
            "INSERT OR REPLACE INTO jsp_terms (term_id, section_id, source_id, term, acronym, normalised)
             VALUES ('{}', '{}', '{}', '{}', {}, '{}')",
            esc(&term_id), esc(sid), esc(source_id), esc(&t.term), acr_sql, esc(&t.normalised)
        ))?;
        count += 1;
    }

    println!("\nStaged {count} terms in DuckDB (jsp_terms)");
    Ok(())
}

/// Show terms, optionally filtering for conflicts across sources.
fn cmd_terms(duck: &DuckStore, conflicts: bool) -> Result<()> {
    if conflicts {
        let sql = "SELECT t1.acronym, t1.term, t1.source_id, t2.term AS conflicting_term, t2.source_id AS conflicting_source \
                   FROM jsp_terms t1 \
                   JOIN jsp_terms t2 ON t1.normalised = t2.normalised AND t1.source_id < t2.source_id AND t1.term != t2.term \
                   ORDER BY t1.normalised";
        match duck.query_arrow(sql) {
            Ok(batches) => {
                let total: usize = batches.iter().map(|b| b.num_rows()).sum();
                if total == 0 {
                    println!("No term conflicts found across sources.");
                } else {
                    println!("Term conflicts: {total}\n");
                    println!("{}", arrow::util::pretty::pretty_format_batches(&batches)?);
                }
            }
            Err(_) => println!("Table 'jsp_terms' not found. Run 'fractalaw jsp extract-terms' first."),
        }
    } else {
        let sql = "SELECT source_id, count(*) as terms, count(acronym) as acronyms \
                   FROM jsp_terms GROUP BY source_id ORDER BY source_id";
        match duck.query_arrow(sql) {
            Ok(batches) => {
                let total: usize = batches.iter().map(|b| b.num_rows()).sum();
                if total == 0 {
                    println!("No terms in DuckDB. Run 'fractalaw jsp extract-terms' first.");
                } else {
                    println!("{}", arrow::util::pretty::pretty_format_batches(&batches)?);
                }
            }
            Err(_) => println!("Table 'jsp_terms' not found. Run 'fractalaw jsp extract-terms' first."),
        }
    }
    Ok(())
}

/// Extract mandated artefacts from JSP obligations.
fn cmd_extract_artefacts(duck: &DuckStore, source_id: &str, dry_run: bool) -> Result<()> {
    let safe = source_id.replace('\'', "''");
    let sql = format!(
        "SELECT obligation_id, section_id, text FROM jsp_obligations WHERE source_id = '{safe}' ORDER BY section_id, obligation_index"
    );
    let batches = match duck.query_arrow(&sql) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Table 'jsp_obligations' not found. Run 'fractalaw jsp extract-obligations' first.");
            return Ok(());
        }
    };

    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total == 0 {
        println!("No obligations found for {source_id}. Run 'fractalaw jsp extract-obligations' first.");
        return Ok(());
    }

    println!("Scanning {total} obligations in {source_id} for mandated artefacts...");

    let mut all_artefacts: Vec<(String, String, jsp::artefacts::MandatedArtefact)> = Vec::new();
    let mut by_type: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for batch in &batches {
        use arrow::array::{Array, StringArray};

        let oid_col: Option<&StringArray> = batch.column_by_name("obligation_id").and_then(|c| c.as_any().downcast_ref());
        let sid_col: Option<&StringArray> = batch.column_by_name("section_id").and_then(|c| c.as_any().downcast_ref());
        let text_col: Option<&StringArray> = batch.column_by_name("text").and_then(|c| c.as_any().downcast_ref());

        for row in 0..batch.num_rows() {
            let oid = oid_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");
            let sid = sid_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");
            let text = text_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");

            if text.is_empty() { continue; }

            let arts = jsp::artefacts::extract_artefacts(text);
            for art in arts {
                *by_type.entry(art.artefact_type).or_default() += 1;
                all_artefacts.push((oid.to_string(), sid.to_string(), art));
            }
        }
    }

    println!("\nMandated artefacts found: {}", all_artefacts.len());
    if !by_type.is_empty() {
        let mut sorted: Vec<_> = by_type.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (atype, count) in &sorted {
            println!("  {atype}: {count}");
        }
    }

    // Show samples
    if !all_artefacts.is_empty() {
        println!("\nSample artefacts:");
        for (oid, _sid, art) in all_artefacts.iter().take(10) {
            let short_oid = oid.rsplit_once(':').map(|(_, s)| s).unwrap_or(oid);
            println!("  [{:>18}] {} — \"{}\"", art.artefact_type, short_oid, art.matched_text);
        }
        if all_artefacts.len() > 10 {
            println!("  ... and {} more", all_artefacts.len() - 10);
        }
    }

    if dry_run {
        println!("\n(dry run — not writing to DuckDB)");
        return Ok(());
    }

    duck.execute(
        "CREATE TABLE IF NOT EXISTS jsp_mandated_artefacts (
            artefact_id TEXT PRIMARY KEY,
            obligation_id TEXT NOT NULL,
            section_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            artefact_type TEXT NOT NULL,
            matched_text TEXT,
            extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"
    ).context("failed to create jsp_mandated_artefacts table")?;

    duck.execute(&format!("DELETE FROM jsp_mandated_artefacts WHERE source_id = '{safe}'"))?;

    let esc = |s: &str| s.replace('\'', "''");
    let mut count = 0usize;

    for (i, (oid, sid, art)) in all_artefacts.iter().enumerate() {
        let art_id = format!("{oid}:art.{i}");
        duck.execute(&format!(
            "INSERT INTO jsp_mandated_artefacts (artefact_id, obligation_id, section_id, source_id, artefact_type, matched_text)
             VALUES ('{}', '{}', '{}', '{}', '{}', '{}')",
            esc(&art_id), esc(oid), esc(sid), esc(source_id), art.artefact_type, esc(&art.matched_text)
        ))?;
        count += 1;
    }

    println!("\nStaged {count} mandated artefacts in DuckDB (jsp_mandated_artefacts)");
    Ok(())
}

/// Query mandated artefacts by type across all sources.
fn cmd_artefacts(duck: &DuckStore, artefact_type: Option<&str>) -> Result<()> {
    let where_clause = artefact_type
        .map(|t| format!("WHERE a.artefact_type LIKE '%{}%'", t.replace('\'', "''")))
        .unwrap_or_default();

    let sql = format!(
        "SELECT a.artefact_type, a.source_id, a.obligation_id, a.matched_text, o.text \
         FROM jsp_mandated_artefacts a \
         JOIN jsp_obligations o ON a.obligation_id = o.obligation_id \
         {where_clause} \
         ORDER BY a.artefact_type, a.source_id"
    );

    match duck.query_arrow(&sql) {
        Ok(batches) => {
            let total: usize = batches.iter().map(|b| b.num_rows()).sum();
            if total == 0 {
                println!("No mandated artefacts found. Run 'fractalaw jsp extract-artefacts' first.");
                return Ok(());
            }

            use arrow::array::{Array, StringArray};

            println!("Mandated artefacts{}: {total}\n",
                artefact_type.map(|t| format!(" matching '{t}'")).unwrap_or_default());

            for batch in &batches {
                let type_col: Option<&StringArray> = batch.column_by_name("artefact_type").and_then(|c| c.as_any().downcast_ref());
                let src_col: Option<&StringArray> = batch.column_by_name("source_id").and_then(|c| c.as_any().downcast_ref());
                let text_col: Option<&StringArray> = batch.column_by_name("text").and_then(|c| c.as_any().downcast_ref());

                for row in 0..batch.num_rows() {
                    let atype = type_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
                    let src = src_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
                    let text = text_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("");

                    let truncated = if text.len() > 120 { format!("{}...", &text[..120]) } else { text.to_string() };
                    println!("[{atype}] ({src})");
                    println!("  {truncated}");
                    println!();
                }
            }
            Ok(())
        }
        Err(_) => {
            println!("Table 'jsp_mandated_artefacts' not found. Run 'fractalaw jsp extract-artefacts' first.");
            Ok(())
        }
    }
}

/// Generate controls from JSP mandated artefacts.
///
/// Each mandated artefact becomes a control in `suggested_controls` with
/// `source_id` set to the JSP chapter. Controls are additive — they don't
/// touch existing legislation controls.
fn cmd_controls(duck: &DuckStore, source_id: &str, dry_run: bool) -> Result<()> {
    let safe = source_id.replace('\'', "''");

    // Read artefacts with their parent obligation text and RACI
    let sql = format!(
        "SELECT a.artefact_id, a.artefact_type, a.obligation_id, a.section_id, \
                o.text AS obligation_text, o.strength, o.modal_verb, o.competence_requirements, \
                r.role_label, r.assignment_type \
         FROM jsp_mandated_artefacts a \
         JOIN jsp_obligations o ON a.obligation_id = o.obligation_id \
         LEFT JOIN jsp_raci r ON r.obligation_id = a.obligation_id \
         WHERE a.source_id = '{safe}' \
         ORDER BY a.section_id, a.artefact_type"
    );

    let batches = match duck.query_arrow(&sql) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Tables not found. Run extract-obligations and extract-artefacts first.");
            return Ok(());
        }
    };

    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    if total == 0 {
        println!("No mandated artefacts found for {source_id}.");
        return Ok(());
    }

    // Group by artefact_id to collect RACI roles per artefact
    use arrow::array::{Array, StringArray};
    use std::collections::HashMap;

    struct ArtefactData {
        artefact_type: String,
        obligation_text: String,
        strength: String,
        section_id: String,
        competence: String,
        roles: Vec<(String, String)>, // (role_label, assignment_type)
    }

    let mut artefacts: HashMap<String, ArtefactData> = HashMap::new();

    for batch in &batches {
        let aid_col: Option<&StringArray> = batch.column_by_name("artefact_id").and_then(|c| c.as_any().downcast_ref());
        let atype_col: Option<&StringArray> = batch.column_by_name("artefact_type").and_then(|c| c.as_any().downcast_ref());
        let otext_col: Option<&StringArray> = batch.column_by_name("obligation_text").and_then(|c| c.as_any().downcast_ref());
        let strength_col: Option<&StringArray> = batch.column_by_name("strength").and_then(|c| c.as_any().downcast_ref());
        let sid_col: Option<&StringArray> = batch.column_by_name("section_id").and_then(|c| c.as_any().downcast_ref());
        let comp_col: Option<&StringArray> = batch.column_by_name("competence_requirements").and_then(|c| c.as_any().downcast_ref());
        let role_col: Option<&StringArray> = batch.column_by_name("role_label").and_then(|c| c.as_any().downcast_ref());
        let rtype_col: Option<&StringArray> = batch.column_by_name("assignment_type").and_then(|c| c.as_any().downcast_ref());

        for row in 0..batch.num_rows() {
            let get = |col: Option<&StringArray>| col.and_then(|c| if c.is_valid(row) { Some(c.value(row).to_string()) } else { None }).unwrap_or_default();

            let aid = get(aid_col);
            let role = get(role_col);
            let rtype = get(rtype_col);

            let entry = artefacts.entry(aid.clone()).or_insert_with(|| ArtefactData {
                artefact_type: get(atype_col),
                obligation_text: get(otext_col),
                strength: get(strength_col),
                section_id: get(sid_col),
                competence: get(comp_col),
                roles: Vec::new(),
            });

            if !role.is_empty() {
                entry.roles.push((role, rtype));
            }
        }
    }

    println!("Generating controls from {} mandated artefacts in {source_id}...", artefacts.len());

    // Find the law_name(s) this JSP implements (from references)
    let law_name = duck.query_arrow(&format!(
        "SELECT DISTINCT target_id FROM jsp_references \
         WHERE source_id = '{safe}' AND target_type = 'legislation' AND resolved = TRUE \
         LIMIT 1"
    )).ok()
    .and_then(|batches| batches.first().and_then(|b| {
        b.column_by_name("target_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .and_then(|a| if a.len() > 0 && a.is_valid(0) { Some(a.value(0).to_string()) } else { None })
    }));

    // Generate controls
    let mut controls = Vec::new();
    for (aid, data) in &artefacts {
        let responsible: Vec<&str> = data.roles.iter()
            .filter(|(_, t)| t == "R")
            .map(|(r, _)| r.as_str())
            .collect();
        let owner = responsible.first().copied().unwrap_or("(unassigned)");

        // Build indicative-mood control title from artefact type + obligation
        let title = format!(
            "A {} is maintained{}",
            data.artefact_type,
            if owner != "(unassigned)" { format!(" by the {owner}") } else { String::new() }
        );

        let control_json = serde_json::json!({
            "title": title,
            "description": data.obligation_text,
            "what_it_checks": format!("The {} exists, is current, and addresses the specific hazards", data.artefact_type),
            "control_type": match data.artefact_type.as_str() {
                "Risk Assessment" | "Safety Case" | "Hazard Log" => "Preventive",
                "Inspection Report" | "Audit Report" | "Occurrence Report" => "Detective",
                "Emergency Plan" => "Corrective",
                _ => "Directive",
            },
            "nature": "Manual",
            "domain": match data.artefact_type.as_str() {
                "Training Record" => "People",
                "Inspection Report" | "Maintenance Record" => "Physical",
                _ => "Organisational",
            },
            "linked_provisions": [data.section_id],
            "mapping_strength": "Primary",
            "artefact_type": data.artefact_type,
            "responsible_role": owner,
            "competence_requirements": data.competence,
            "source": "jsp",
        });

        controls.push((aid.clone(), data, control_json));
    }

    // Print summary
    println!("\nGenerated {} JSP controls:", controls.len());
    for (_, data, json) in &controls {
        println!("  [{}] {}", data.artefact_type, json["title"].as_str().unwrap_or("?"));
    }

    if dry_run {
        println!("\n(dry run — not writing to DuckDB)");
        return Ok(());
    }

    // Ensure columns exist
    let _ = duck.execute("ALTER TABLE suggested_controls ADD COLUMN source_id TEXT");
    let _ = duck.execute("ALTER TABLE suggested_controls ADD COLUMN related_control_ids TEXT");

    // Clear existing JSP controls for this source
    duck.execute(&format!("DELETE FROM suggested_controls WHERE source_id = '{safe}'"))?;

    let esc = |s: &str| s.replace('\'', "''");
    let mut count = 0usize;
    for (aid, _data, json) in &controls {
        let json_str = serde_json::to_string(json).unwrap_or_default();
        let law = law_name.as_deref().unwrap_or("");

        duck.execute(&format!(
            "INSERT INTO suggested_controls (id, law_name, source_id, control_type, control_json, status, generated_at) \
             VALUES ('{}', '{}', '{}', 'specific', '{}', 'generated', CURRENT_TIMESTAMP)",
            esc(aid), esc(law), esc(source_id), esc(&json_str)
        ))?;
        count += 1;
    }

    println!("\nStaged {count} JSP controls in suggested_controls (source_id = {source_id})");
    Ok(())
}

/// Gap analysis: find legislation referenced by this JSP that has obligations
/// not covered by any JSP provision.
fn cmd_gaps(duck: &DuckStore, source_id: &str) -> Result<()> {
    let safe = source_id.replace('\'', "''");

    // Step 1: Find all legislation law_names referenced by this JSP source
    let laws_sql = format!(
        "SELECT DISTINCT target_id FROM jsp_references \
         WHERE source_id = '{safe}' AND target_type = 'legislation' AND resolved = TRUE"
    );
    let law_batches = match duck.query_arrow(&laws_sql) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Table 'jsp_references' not found. Run 'fractalaw jsp extract-refs' first.");
            return Ok(());
        }
    };

    use arrow::array::{Array, StringArray};
    let mut law_names = Vec::new();
    for batch in &law_batches {
        if let Some(col) = batch.column_by_name("target_id") {
            if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                for i in 0..arr.len() {
                    if arr.is_valid(i) {
                        law_names.push(arr.value(i).to_string());
                    }
                }
            }
        }
    }

    if law_names.is_empty() {
        println!("No legislation references found for {source_id}. Run 'fractalaw jsp extract-refs' first.");
        return Ok(());
    }

    println!("Legislation referenced by {source_id}: {} laws", law_names.len());
    for ln in &law_names {
        println!("  {ln}");
    }

    // Step 2: For each law, find HIGH/MEDIUM obligations not covered by any JSP reference
    // "Covered" means a JSP provision references this specific law
    // "Gap" means the law is referenced at law level but specific provisions aren't traced

    // Get all JSP references to each law (provision-level)
    let covered_sql = format!(
        "SELECT DISTINCT target_id FROM jsp_references \
         WHERE source_id = '{safe}' AND target_type = 'legislation' AND resolved = TRUE \
         AND target_id LIKE '%:%'"  // section_id format has a colon
    );
    let covered_batches = duck.query_arrow(&covered_sql).unwrap_or_default();
    let mut covered_sections = std::collections::HashSet::new();
    for batch in &covered_batches {
        if let Some(col) = batch.column_by_name("target_id") {
            if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
                for i in 0..arr.len() {
                    if arr.is_valid(i) {
                        covered_sections.insert(arr.value(i).to_string());
                    }
                }
            }
        }
    }

    println!("\nLegislative provisions explicitly referenced: {}", covered_sections.len());
    println!("(Gaps are laws referenced at document level but whose individual provisions are not traced)");
    println!("\nNote: provision-level gap analysis requires legislation provisions in DuckDB.");
    println!("This is a law-level summary. Full provision-level gaps need Postgres data.");

    // Step 3: Check legislation table for obligation counts per referenced law
    for ln in &law_names {
        let safe_ln = ln.replace('\'', "''");
        match duck.query_arrow(&format!(
            "SELECT name, title, \
                    COALESCE(json_array_length(duty_holder::JSON), 0) as duty_holders \
             FROM legislation WHERE name = '{safe_ln}'"
        )) {
            Ok(batches) => {
                for batch in &batches {
                    let title_col: Option<&StringArray> = batch.column_by_name("title").and_then(|c| c.as_any().downcast_ref());
                    for row in 0..batch.num_rows() {
                        let title = title_col.and_then(|c| if c.is_valid(row) { Some(c.value(row)) } else { None }).unwrap_or("?");
                        // Count how many JSP provisions reference this law
                        let ref_count = duck.query_arrow(&format!(
                            "SELECT count(*) as n FROM jsp_references WHERE source_id = '{safe}' AND target_id = '{safe_ln}'"
                        )).ok().and_then(|b| b.first().map(|b| b.num_rows())).unwrap_or(0);

                        println!("\n  {ln}: {title}");
                        println!("    JSP references: {ref_count}");
                    }
                }
            }
            Err(_) => {
                println!("\n  {ln}: (not in DuckDB legislation table)");
            }
        }
    }

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
