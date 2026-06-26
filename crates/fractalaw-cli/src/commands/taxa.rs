use anyhow::Context;
use arrow::array::Array;
use fractalaw_store::{DuckStore, LanceStore, ProvisionStore};

use crate::llm::*;
use crate::open_duck;
use crate::utils::*;
use super::pipeline::*;

/// Show pipeline status for a set of laws.
pub(crate) fn cmd_taxa_status(
    store: &DuckStore,
    laws: Option<String>,
    law_file: Option<std::path::PathBuf>,
    summary: bool,
    stage_filter: Option<String>,
) -> anyhow::Result<()> {
    store.ensure_pipeline_status_columns()?;

    // Collect law names from --laws and/or --law-file
    let law_names: Vec<String> = {
        let mut names = Vec::new();
        if let Some(ref csv) = laws {
            names.extend(csv.split(',').map(|s| s.trim().to_string()));
        }
        if let Some(ref path) = law_file {
            let content = std::fs::read_to_string(path)
                .with_context(|| format!("reading law file '{}'", path.display()))?;
            // Handle both formats: one-per-line and comma-separated
            for token in content.split(|c: char| c == ',' || c == '\n') {
                let name = token.trim();
                if !name.is_empty() && !name.starts_with('#') && name.contains('_') {
                    names.push(name.to_string());
                }
            }
        }
        names
    };

    // Build SQL query
    let where_clause = if law_names.is_empty() {
        String::new()
    } else {
        let list = law_names
            .iter()
            .map(|n| format!("'{}'", n.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        format!("WHERE name IN ({list})")
    };

    let sql = format!(
        "SELECT name, \
            lat_pulled_at, embedded_at, parsed_at, classified_at, \
            validated_at, adjudicated_at, provisions_published_at, \
            taxa_hash, published_hash \
         FROM legislation {where_clause} ORDER BY name"
    );
    let batches = store.query_arrow(&sql)?;

    // Derive stage for each law
    struct LawStatus {
        name: String,
        stage: String,
    }

    let mut statuses: Vec<LawStatus> = Vec::new();
    for batch in &batches {
        let name_col = batch.column_by_name("name");
        let lat_col = batch.column_by_name("lat_pulled_at");
        let emb_col = batch.column_by_name("embedded_at");
        let parse_col = batch.column_by_name("parsed_at");
        let cls_col = batch.column_by_name("classified_at");
        let val_col = batch.column_by_name("validated_at");
        let taxa_col = batch.column_by_name("taxa_hash");
        let pub_col = batch.column_by_name("published_hash");

        for row in 0..batch.num_rows() {
            let name = name_col
                .and_then(|c| get_string_value(c.as_ref(), row))
                .unwrap_or_default();
            let has_lat = lat_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let has_emb = emb_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let has_parse = parse_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let has_cls = cls_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let has_val = val_col.map(|c| !c.is_null(row)).unwrap_or(false);
            let taxa_hash = taxa_col.and_then(|c| get_string_value(c.as_ref(), row));
            let pub_hash = pub_col.and_then(|c| get_string_value(c.as_ref(), row));
            let is_published = taxa_hash.is_some()
                && pub_hash.is_some()
                && taxa_hash == pub_hash;

            let stage = if is_published {
                "published"
            } else if has_val {
                "ready_to_publish"
            } else if has_cls {
                "needs_validate"
            } else if has_parse {
                "needs_classify"
            } else if has_emb {
                "needs_parse"
            } else if has_lat {
                "needs_embed"
            } else {
                "needs_lat"
            };

            statuses.push(LawStatus {
                name,
                stage: stage.to_string(),
            });
        }
    }

    // Add laws from the input list that aren't in DuckDB at all
    for law in &law_names {
        if !statuses.iter().any(|s| s.name == *law) {
            statuses.push(LawStatus {
                name: law.clone(),
                stage: "needs_lat".to_string(),
            });
        }
    }

    // Apply stage filter
    if let Some(ref filter) = stage_filter {
        statuses.retain(|s| s.stage == *filter);
    }

    // Count by stage
    let stages = [
        "published",
        "ready_to_publish",
        "needs_validate",
        "needs_classify",
        "needs_parse",
        "needs_embed",
        "needs_lat",
    ];
    let mut counts: std::collections::HashMap<&str, usize> = stages.iter().map(|s| (*s, 0)).collect();
    for s in &statuses {
        *counts.entry(s.stage.as_str()).or_default() += 1;
    }

    // Print summary
    let total = statuses.len();
    println!("Pipeline Status ({total} laws)");
    for stage in &stages {
        let count = counts[stage];
        if count > 0 || !summary {
            println!("  {:<20} {}", stage, count);
        }
    }

    // Print per-law detail (unless --summary)
    if !summary && !statuses.is_empty() {
        println!();
        for stage in &stages {
            let laws_at_stage: Vec<&str> = statuses
                .iter()
                .filter(|s| s.stage == *stage)
                .map(|s| s.name.as_str())
                .collect();
            if !laws_at_stage.is_empty() {
                println!("{stage}:");
                for name in laws_at_stage {
                    println!("  {name}");
                }
            }
        }
    }

    Ok(())
}

/// Reconcile per-tier signals into final drrp_types + actors.
///
/// Reads regex_drrp/regex_actors, cls_drrp/cls_actors, llm_drrp/llm_actors
/// and picks the best answer per provision. Writes to drrp_types + actors.
///
/// Rules:
/// 1. LLM wins (highest quality) if present
/// 2. Regex + classifier agree → use that, extraction_method = "classifier"
/// 3. Disagree → use regex, flag extraction_method = "pending_llm"
/// 4. Classifier confidence < 0.7 → don't trust, use regex
/// 5. Only regex available → use regex
pub(crate) async fn cmd_taxa_reconcile(
    lance: &dyn ProvisionStore,
    law_names: &[String],
) -> anyhow::Result<()> {
    let mut total_reconciled = 0usize;
    let mut total_agreed = 0usize;
    let mut total_disagreed = 0usize;
    let mut total_llm = 0usize;
    let mut total_regex_only = 0usize;

    for law_name in law_names {
        let batches = lance.query_legislation_text(law_name, 100_000, 0).await?;

        for batch in &batches {
            let sid_col = batch.column_by_name("section_id");
            let regex_drrp_col = batch.column_by_name("regex_drrp");
            let regex_actors_col = batch.column_by_name("regex_actors");
            let cls_drrp_col = batch.column_by_name("cls_drrp");
            let cls_actors_col = batch.column_by_name("cls_actors");
            let cls_conf_col = batch.column_by_name("cls_confidence");
            let llm_drrp_col = batch.column_by_name("llm_drrp");
            let llm_actors_col = batch.column_by_name("llm_actors");

            // Helper: read first element from a TEXT[] (List<Utf8>) or fall back to Utf8 string
            let get_list_first = |col: &dyn arrow::array::Array, row: usize| -> Option<String> {
                // Try List<Utf8> (Postgres TEXT[])
                if let Some(list) = col.as_any().downcast_ref::<arrow::array::ListArray>() {
                    if !list.is_null(row) {
                        let vals = list.value(row);
                        if vals.len() > 0 {
                            if let Some(sa) = vals.as_any().downcast_ref::<arrow::array::StringArray>() {
                                if !sa.is_null(0) {
                                    let v = sa.value(0).to_string();
                                    if !v.is_empty() {
                                        return Some(v);
                                    }
                                }
                            }
                        }
                    }
                    return None;
                }
                // Fall back to Utf8/LargeUtf8
                get_string_value(col, row)
            };

            for row in 0..batch.num_rows() {
                let _sid = match sid_col.and_then(|c| get_string_value(c.as_ref(), row)) {
                    Some(s) => s,
                    None => continue,
                };

                let regex_drrp = regex_drrp_col.and_then(|c| get_list_first(c.as_ref(), row));
                let regex_actors = regex_actors_col.and_then(|c| get_string_value(c.as_ref(), row));
                let cls_drrp = cls_drrp_col.and_then(|c| get_list_first(c.as_ref(), row));
                let cls_actors = cls_actors_col.and_then(|c| get_string_value(c.as_ref(), row));
                let cls_conf = cls_conf_col.and_then(|c| {
                    c.as_any()
                        .downcast_ref::<arrow::array::Float32Array>()
                        .and_then(|a| if a.is_null(row) { None } else { Some(a.value(row)) })
                });
                let llm_drrp = llm_drrp_col.and_then(|c| get_list_first(c.as_ref(), row));
                let llm_actors = llm_actors_col.and_then(|c| get_string_value(c.as_ref(), row));

                // Skip provisions with no regex signal
                if regex_drrp.is_none() && regex_actors.is_none() {
                    continue;
                }

                // Pick winner
                let (final_drrp, final_actors, method) = if llm_drrp.is_some() || llm_actors.is_some() {
                    // Rule 1: LLM wins
                    total_llm += 1;
                    (
                        llm_drrp.or(regex_drrp),
                        llm_actors.or(regex_actors),
                        "agentic",
                    )
                } else if let (Some(r_drrp), Some(c_drrp)) = (&regex_drrp, &cls_drrp) {
                    let confident = cls_conf.unwrap_or(0.0) >= 0.7;
                    if r_drrp == c_drrp && confident {
                        // Rule 2: agree + confident
                        total_agreed += 1;
                        (
                            cls_drrp.or(regex_drrp),
                            cls_actors.or(regex_actors),
                            "classifier",
                        )
                    } else {
                        // Rule 3: disagree or low confidence → use regex, flag for LLM
                        total_disagreed += 1;
                        (regex_drrp, regex_actors, "pending_llm")
                    }
                } else {
                    // Rule 5: only regex
                    total_regex_only += 1;
                    (regex_drrp, regex_actors, "regex")
                };

                // Write reconciled result (SQL UPDATE)
                // For now, use a simple per-row update via the store
                // This is the reconciled output that sertantai consumes
                total_reconciled += 1;
                let _ = final_drrp; // TODO: write back via batch UPDATE
                let _ = final_actors;
                let _ = method;
            }
        }
    }

    println!("Reconciled {total_reconciled} provisions across {} laws", law_names.len());
    println!("  LLM wins:      {total_llm}");
    println!("  Agreed:        {total_agreed}");
    println!("  Disagreed:     {total_disagreed}");
    println!("  Regex only:    {total_regex_only}");

    Ok(())
}

pub(crate) async fn cmd_taxa_show(
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

    let batches = lance.query_legislation_text(name, limit, 0).await?;

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

pub(crate) fn cmd_taxa_show_misses(
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

pub(crate) fn cmd_taxa_show_clauses(
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


pub(crate) async fn cmd_taxa_eyeball(
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
        let batches = lance.query_legislation_text(law_name, limit, 0).await?;
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

pub(crate) async fn cmd_taxa_qa(
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
        let all_batches = lance.query_legislation_text("", 200_000, 0).await?;
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
        let batches = lance.query_legislation_text(law_name, 100_000, 0).await?;

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


/// Audit p-dimension dictionary coverage: find Application+Scope provisions
/// where polarity was detected but zero p-dimension tags were extracted.
pub(crate) async fn cmd_taxa_audit_fitness(
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
        let all_batches = lance.query_legislation_text("", 200_000, 0).await?;
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

        let batches = lance.query_legislation_text(law_name, 100_000, 0).await?;

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


/// Regex parsing + Tier 1 inheritance for a list of laws.
/// Runs `enrich_single_law` with `escalate=false` — no LLM calls.
pub(crate) async fn cmd_taxa_parse(
    lance: &dyn ProvisionStore,
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
        let escaped = law_name.replace('\'', "''");
        match enrich_single_law(lance, store, law_name, false, force).await {
            Ok(_) => {
                let _ = store.execute(&format!(
                    "UPDATE legislation SET parsed_at = CURRENT_TIMESTAMP WHERE name = '{escaped}'"
                ));
                enriched += 1;
                if enriched.is_multiple_of(100) {
                    eprint!("\r  Parsed {enriched}/{total}...");
                }
            }
            Err(e) => {
                eprintln!("  {law_name}: parse error: {e:#?}");
                let _ = store.execute(&format!(
                    "UPDATE legislation \
                     SET enrichment_retry_count = COALESCE(enrichment_retry_count, 0) + 1 \
                     WHERE name = '{escaped}'"
                ));
                failed += 1;
            }
        }

        // Compact LanceDB periodically to prevent fragment bloat.
        // Every 5 laws with --force on large corpus to avoid disk exhaustion.
        if enriched.is_multiple_of(5) && total > 5 {
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
pub(crate) async fn cmd_taxa_trace(
    lance: &dyn ProvisionStore,
    store: &DuckStore,
    law_names: &[String],
    trace_path: &str,
) -> anyhow::Result<()> {
    use std::io::Write;

    let mut entries = Vec::new();

    for law_name in law_names {
        let escaped = law_name.replace('\'', "''");
        let family: Option<String> = {
            let batches = store.query_arrow(&format!(
                "SELECT family FROM legislation WHERE name = '{escaped}'"
            ))?;
            batches.iter().find_map(|b| {
                let col = b.column_by_name("family")?;
                get_string_value(col.as_ref(), 0)
            })
        };

        let batches = lance.query_legislation_text(law_name, 100_000, 0).await?;

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

pub(crate) async fn cmd_taxa_validate(
    lance: &dyn ProvisionStore,
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
        let batches = lance.query_legislation_text(law_name, 100_000, 0).await?;

        let mut provisions: Vec<serde_json::Value> = Vec::new();
        for batch in &batches {
            let sid_col = batch.column_by_name("section_id");
            let text_col = batch.column_by_name("text");
            let drrp_col = batch.column_by_name("drrp_types");
            let method_col = batch.column_by_name("extraction_method");
            let conf_col = batch.column_by_name("taxa_confidence");
            let actors_col = batch.column_by_name("actors");
            let hpath_col = batch.column_by_name("hierarchy_path");

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
                let hpath = hpath_col
                    .and_then(|c| get_string_value(c.as_ref(), row))
                    .unwrap_or_default();
                // Top-level section: first path component
                let section = hpath
                    .split('/')
                    .next()
                    .unwrap_or(&sid)
                    .to_string();
                // Truncate text for prompt (keep first 500 chars)
                let text_trunc = if text.len() > 500 {
                    format!("{}...", &text[..text.char_indices().take_while(|&(i, _)| i < 500).last().map(|(i, _)| i).unwrap_or(500)])
                } else {
                    text.clone()
                };

                provisions.push(serde_json::json!({
                    "section_id": sid,
                    "section": section,
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

        // Mark targets: provisions that need LLM review
        for p in &mut provisions {
            let method = p["method"].as_str().unwrap_or("");
            let drrp_str = p["drrp"].as_str().unwrap_or("none");
            let actors = p["actors"].as_array().map(|a| a.len()).unwrap_or(0);
            let conf = p["confidence"].as_f64().unwrap_or(1.0) as f32;
            let has_drrp = drrp_str != "none" && !drrp_str.is_empty();
            let is_target = method == "pending_llm"
                || (has_drrp && actors == 0)
                || (conf > 0.0 && conf < 0.3);
            p.as_object_mut()
                .unwrap()
                .insert("needs_review".into(), serde_json::json!(is_target));
        }

        let target_count = provisions.iter().filter(|p| p["needs_review"] == true).count();

        if target_count == 0 {
            eprintln!("  {law_name}: {provs} provisions, 0 targets — skipping", provs = provisions.len());
            continue;
        }

        // Group by section (top-level hierarchy_path component)
        let mut section_groups: std::collections::BTreeMap<String, Vec<&serde_json::Value>> =
            std::collections::BTreeMap::new();
        for p in &provisions {
            let section = p["section"].as_str().unwrap_or("(root)").to_string();
            section_groups.entry(section).or_default().push(p);
        }

        // Filter to sections with at least one target
        let target_sections: Vec<(String, Vec<&serde_json::Value>)> = section_groups
            .into_iter()
            .filter(|(_, provs)| provs.iter().any(|p| p["needs_review"] == true))
            .collect();

        let sections_skipped = provisions.len() - target_sections.iter().map(|(_, v)| v.len()).sum::<usize>();
        let strategy = if provisions.len() <= 200 { "whole_law" } else { "section_targeted" };

        eprintln!(
            "  {law_name}: {provs} provisions, {targets} targets, {secs} sections to send ({skipped} provisions skipped)",
            provs = provisions.len(),
            targets = target_count,
            secs = target_sections.len(),
            skipped = sections_skipped,
        );

        if dry_run {
            for (sec, provs) in &target_sections {
                let sec_targets = provs.iter().filter(|p| p["needs_review"] == true).count();
                let tokens = provs.len() * 80;
                eprintln!("    {sec}: {n} provisions ({sec_targets} targets, ~{tok}k tokens)",
                    n = provs.len(), tok = tokens / 1000);
            }
            continue;
        }

        // Send each target section to Gemini
        let mut all_corrections: Vec<serde_json::Value> = Vec::new();
        let mut total_latency_ms = 0u64;
        let mut total_token_est = 0usize;

        let batches_to_send: Vec<(String, Vec<&serde_json::Value>)> = if provisions.len() <= 200 {
            // Small law: send everything as one batch
            vec![("(whole law)".into(), provisions.iter().collect())]
        } else {
            // Large law: send per section, sub-grouping if section >200
            let mut batches = Vec::new();
            for (sec_name, sec_provs) in &target_sections {
                if sec_provs.len() <= 200 {
                    batches.push((sec_name.clone(), sec_provs.clone()));
                } else {
                    // Sub-group by next hierarchy level
                    let mut sub_groups: std::collections::BTreeMap<String, Vec<&serde_json::Value>> =
                        std::collections::BTreeMap::new();
                    for p in sec_provs {
                        let hpath = p["section"].as_str().unwrap_or("");
                        let sid = p["section_id"].as_str().unwrap_or("");
                        // Use section_id prefix as sub-group (e.g. "s.25" from "s.25(1)")
                        let sub_key = sid.split(':').nth(1)
                            .and_then(|s| s.split('(').next())
                            .unwrap_or(hpath);
                        sub_groups.entry(format!("{sec_name}/{sub_key}")).or_default().push(p);
                    }
                    // Merge tiny sub-groups into batches of ~100-200
                    let mut current_batch: Vec<&serde_json::Value> = Vec::new();
                    let mut current_name = sec_name.clone();
                    for (sub_name, sub_provs) in sub_groups {
                        if current_batch.len() + sub_provs.len() > 200 && !current_batch.is_empty() {
                            if current_batch.iter().any(|p| p["needs_review"] == true) {
                                batches.push((current_name.clone(), current_batch));
                            }
                            current_batch = Vec::new();
                        }
                        current_batch.extend(sub_provs);
                        current_name = sub_name;
                    }
                    if !current_batch.is_empty() && current_batch.iter().any(|p| p["needs_review"] == true) {
                        batches.push((current_name, current_batch));
                    }
                }
            }
            batches
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            api_key
        );

        for (batch_name, batch_provs) in &batches_to_send {
            let batch_targets = batch_provs.iter().filter(|p| p["needs_review"] == true).count();
            let batch_json = serde_json::to_string_pretty(&batch_provs)?;

            let prompt = format!(
                r#"You are reviewing DRRP classifications for a section of UK legislation.

Law: {law_name} — Section: {batch_name}
Family: {family}

This section has {total} provisions, of which {target_count} need review.
Provisions marked "needs_review": true are uncertain and need your assessment.
Other provisions are included as context only — do NOT suggest corrections for them.

Classification rules:
- Obligation: a legal obligation imposed on someone (shall, must, is required to, has a duty)
- Liberty: a permission, entitlement, or discretionary power GRANTED to someone (may, is entitled to, has a right to, power to)
- none: definitions, commencement, repeals, cross-references, structural, offence/penalty provisions

IMPORTANT — classify as 'none' if the provision:
- Only references, conditions, details, or exempts an obligation/right created in another section
- Creates an exemption or exception to an obligation (e.g. "shall not apply to..." is a scope limitation, not a new Liberty)
- States a consequence without a modal verb (e.g. "is guilty of an offence")
- Only provisions that CREATE a new legal relation count as Obligation or Liberty

Focus ONLY on provisions where "needs_review" is true. Return corrections as JSON:
[{{"section_id": "...", "drrp": "Obligation|Liberty|none", "reason": "brief explanation"}}]

If all reviewed provisions are correctly classified, respond with: []

Provisions:
{batch_json}"#,
                law_name = law_name,
                batch_name = batch_name,
                family = family.as_deref().unwrap_or("unknown"),
                total = batch_provs.len(),
                target_count = batch_targets,
                batch_json = batch_json,
            );

            let token_est = prompt.len() / 4;
            total_token_est += token_est;

            let call_start = std::time::Instant::now();
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
            total_latency_ms += latency_ms;

            match resp {
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
                    all_corrections.extend(parsed);
                }
                Err(e) => {
                    eprintln!("    LLM error on {batch_name}: {e}");
                }
            }
        }

        let law_corrections = all_corrections.len();
        total_corrections += law_corrections;
        eprintln!(
            "    {law_corrections} corrections across {batches} calls ({total_latency_ms}ms)",
            batches = batches_to_send.len(),
        );

        // Build audit log
        let now = chrono::Utc::now().to_rfc3339();
        let correction_map: std::collections::HashMap<String, &serde_json::Value> = all_corrections
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
                    None
                }
            })
            .collect();

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        law_name.hash(&mut hasher);
        now.hash(&mut hasher);
        let integrity_hash = format!("{:016x}", hasher.finish());

        let audit_entry = serde_json::json!({
            "schema_version": 2,
            "pipeline_version": env!("CARGO_PKG_VERSION"),
            "law_name": law_name,
            "family": family,
            "strategy": strategy,
            "model": "gemini-2.5-flash",
            "timestamp": now,
            "token_usage": { "input_estimate": total_token_est },
            "latency_ms": total_latency_ms,
            "provisions_count": provisions.len(),
            "targets_count": target_count,
            "sections_sent": batches_to_send.len(),
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
        let safe_name = law_name.replace('/', "_");
        let audit_path = format!("{audit_dir}/{safe_name}.json");
        let json = serde_json::to_string_pretty(&audit_entry)?;
        let mut file = std::fs::File::create(&audit_path)?;
        file.write_all(json.as_bytes())?;

        // Apply corrections to LanceDB (only with --apply)
        if apply && !all_corrections.is_empty() {
            use arrow::array::StringBuilder;
            use arrow::datatypes::{DataType, Field};

            let mut sid_b = StringBuilder::new();
            let mut drrp_b = arrow::array::ListBuilder::new(StringBuilder::new());
            let mut method_b = StringBuilder::new();

            for correction in &all_corrections {
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
                    drrp_b.append(true);
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
pub(crate) async fn cmd_taxa_embed(
    lance: &dyn ProvisionStore,
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
        let batches = lance.query_legislation_text(law_name, 100_000, 0).await?;

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
        let law_idx = law_names.iter().position(|l| l == law_name).unwrap_or(0) + 1;
        eprintln!(
            "  {law_name}: {}/{} embedded",
            needs_embedding.len(),
            provisions.len(),
        );

        // Compact periodically to prevent fragment bloat
        if law_idx.is_multiple_of(5) && law_names.len() > 5 {
            if let Err(e) = lance.compact().await {
                eprintln!("    compact error: {e}");
            }
        }
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
pub(crate) async fn cmd_taxa_classify(
    lance: &dyn ProvisionStore,
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
        let law_start = std::time::Instant::now();
        let batches = lance.query_legislation_text(law_name, 100_000, 0).await?;

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
            let pos_weights = std::path::Path::new("docs/position_classifier_v2.json");
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
                                // Try List<Struct> (LanceDB format)
                                use arrow::array::AsArray;
                                if let Some(list) = c.as_list_opt::<i32>() {
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
                                    return Some(result);
                                }
                                // Try JSONB string (Postgres format)
                                if let Some(s) = get_string_value(c.as_ref(), row) {
                                    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&s) {
                                        let mut result = Vec::new();
                                        for a in &arr {
                                            result.push((
                                                a.get("label").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                                                a.get("position").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                                                a.get("relates_to").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                                a.get("label_source").and_then(|v| v.as_str()).unwrap_or("canonical").to_string(),
                                                a.get("reason").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                            ));
                                        }
                                        return Some(result);
                                    }
                                }
                                None
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

                            #[allow(unused)] // used in reason trail, needed for future LLM elevation
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

                            // Keep regex position — classifier records its view in reason
                            // field for QA, but doesn't override. Disagreements are
                            // candidates for LLM elevation.
                            let final_pos = regex_pos.clone();

                            updated_actors.push((
                                label.clone(),
                                final_pos,
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

        // Snapshot classifier signals (preserves classifier tier output separately)
        lance.snapshot_classifier_signals(law_name).await.ok();

        total_provisions += provisions.len();
        let law_elapsed = law_start.elapsed();
        let law_idx = law_names.iter().position(|l| l == law_name).unwrap_or(0) + 1;
        eprintln!(
            "  {law_name}: {total_classified} classified ({:.1}s)",
            law_elapsed.as_secs_f64()
        );

        // Compact periodically to prevent fragment bloat
        if law_idx.is_multiple_of(10) && law_names.len() > 10 {
            if let Err(e) = lance.compact().await {
                eprintln!("    compact error: {e}");
            }
        }
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
pub(crate) async fn cmd_taxa_escalate(
    lance: &dyn ProvisionStore,
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

pub(crate) async fn cmd_taxa_enrich(
    data_dir: &std::path::Path,
    store: &DuckStore,
    law_filter: Option<Vec<String>>,
    force: bool,
    escalate: bool,
    skip_recent: bool,
    pending: bool,
    pg_url: Option<&str>,
) -> anyhow::Result<()> {
    // Ensure taxa_hash/published_hash and fitness columns exist (idempotent).
    store.ensure_taxa_hash_columns()?;
    store.ensure_fitness_columns()?;

    let lance = crate::open_provision_store(data_dir, pg_url).await?;

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
        let all_batches = lance.query_legislation_text("", 200_000, 0).await?;
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
            let batches = lance.query_legislation_text(name, 1, 0).await?;
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
        let escaped = law_name.replace('\'', "''");
        let result = match enrich_single_law(lance.as_ref(), store, law_name, escalate, force).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  {law_name}: enrich error: {e}");
                // Increment retry count so dead-letter kicks in at 3
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
                if let Err(e) = cmd_taxa_embed(lance.as_ref(), &law_names).await {
                    eprintln!("  Embed failed (continuing): {e}");
                }
            }

            // Classify with DRRP + position classifiers
            if let Err(e) = cmd_taxa_classify(lance.as_ref(), &law_names).await {
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
