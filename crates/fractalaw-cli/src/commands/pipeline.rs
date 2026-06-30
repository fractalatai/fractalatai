use anyhow::Context;
use arrow::array::Array;
use arrow::record_batch::RecordBatch;
use fractalaw_store::{DuckStore, ProvisionStore};

use crate::llm::*;
use crate::utils::*;

/// Result of enriching a single law.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnrichResult {
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
pub(crate) fn source_tier(method: &str) -> u8 {
    match method {
        "adjudicated" => 7,
        "agentic" => 6,
        "agentic_unvalidated" => 5,
        "classifier" => 4,
        "local" | "local_unvalidated" => 3,
        "inherited" => 2,
        "regex" => 1,
        _ => 0,
    }
}

pub(crate) struct LawTaxa {
    pub(crate) duty_holders: std::collections::BTreeSet<String>,
    pub(crate) rights_holders: std::collections::BTreeSet<String>,
    pub(crate) responsibility_holders: std::collections::BTreeSet<String>,
    pub(crate) power_holders: std::collections::BTreeSet<String>,
    pub(crate) duty_types: std::collections::BTreeSet<String>,
    pub(crate) roles: std::collections::BTreeSet<String>,
    pub(crate) roles_gvt: std::collections::BTreeSet<String>,
    pub(crate) duties: Vec<(String, String, String, String)>,
    pub(crate) rights: Vec<(String, String, String, String)>,
    pub(crate) responsibilities: Vec<(String, String, String, String)>,
    pub(crate) powers: Vec<(String, String, String, String)>,
    pub(crate) fitness_persons: std::collections::BTreeSet<String>,
    pub(crate) fitness_processes: std::collections::BTreeSet<String>,
    pub(crate) fitness_places: std::collections::BTreeSet<String>,
    pub(crate) fitness_plants: std::collections::BTreeSet<String>,
    pub(crate) fitness_properties: std::collections::BTreeSet<String>,
    pub(crate) fitness_sectors: std::collections::BTreeSet<String>,
    pub(crate) fitness_entries: Vec<FitnessEntry>,
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

pub(crate) struct ActorEntry {
    pub(crate) label: String,
    pub(crate) position: String,
    pub(crate) relates_to: Option<String>,
    pub(crate) label_source: String,
    pub(crate) reason: Option<String>,
}

pub(crate) struct ProvisionTaxa {
    pub(crate) section_id: String,
    pub(crate) drrp_types: Vec<String>,
    pub(crate) governed_actors: Vec<String>,
    pub(crate) government_actors: Vec<String>,
    pub(crate) duty_family: Option<String>,
    pub(crate) duty_sub_type: Option<String>,
    pub(crate) popimar: Vec<String>,
    pub(crate) purposes: Vec<String>,
    pub(crate) clause_refined: String,
    pub(crate) taxa_confidence: Option<f32>,
    pub(crate) fitness_polarity: Vec<String>,
    pub(crate) fitness_person: Vec<String>,
    pub(crate) fitness_process: Vec<String>,
    pub(crate) fitness_place: Vec<String>,
    pub(crate) fitness_plant: Vec<String>,
    pub(crate) fitness_property: Vec<String>,
    pub(crate) fitness_sector: Vec<String>,
    pub(crate) section_type: String,
    pub(crate) hierarchy_path: String,
    pub(crate) depth: i32,
    pub(crate) extraction_method: String,
    pub(crate) holder_inferred_from: Vec<String>,
    pub(crate) ancestor_distance: Option<i32>,
    pub(crate) actors: Vec<ActorEntry>,
}

pub(crate) async fn enrich_single_law(
    lance: &dyn ProvisionStore,
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

    let batches = lance.query_legislation_text(law_name, 100_000, 0).await?;
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

    // Stage 1: Parse provisions — run regex DRRP extraction
    let mut taxa = LawTaxa::new();
    let mut provision_taxa = parse_provisions(&batches, family.as_deref(), &mut taxa);

    // Stage 2: Backlink actors — infer holders for Rule provisions
    backlink_actors(&mut taxa);

    // Stage 3: Escalation — Tier 1 inheritance + Tier 2/3 LLM classification
    if escalate {
        apply_escalation(
            &batches,
            &existing_tiers,
            &mut provision_taxa,
            &mut taxa,
        )
        .await?;
    }

    // Stage 4: Write per-provision taxa to LanceDB
    write_provision_taxa(lance, law_name, &provision_taxa, &existing_tiers, force)
        .await?;

    // Stage 4b: Write per-actor regex signals to provision_actors
    {
        let mut actor_rows = Vec::new();
        for pt in &provision_taxa {
            let drrp = pt.drrp_types.first().cloned();
            for actor in &pt.actors {
                let category = if actor.label.contains(':') {
                    actor.label.split(':').next().unwrap_or("other").trim().to_string()
                } else {
                    "other".to_string()
                };
                actor_rows.push((
                    pt.section_id.clone(),
                    actor.label.clone(),
                    category,
                    drrp.clone(),
                    actor.position.clone(),
                    "regex".to_string(),
                ));
            }
        }
        if !actor_rows.is_empty() {
            lance.upsert_provision_actors(&actor_rows).await.ok();
        }
    }

    // Stage 5: Write law-level taxa to DuckDB
    write_law_taxa(store, law_name, &taxa)

}


/// Check if a provision is an LLM validation target.
#[allow(dead_code)]
pub(crate) fn is_llm_target(method: &str, drrp: &[String], actors: &[serde_json::Value], confidence: f32) -> bool {
    method == "pending_llm"
        || (drrp.iter().any(|d| !d.is_empty()) && actors.is_empty())  // orphan
        || (confidence > 0.0 && confidence < 0.3)  // very low confidence
}


/// Parse all provisions from LanceDB batches, running DRRP regex extraction.
/// Returns per-provision taxa and aggregates law-level taxa into `taxa`.
fn parse_provisions(
    batches: &[RecordBatch],
    family: Option<&str>,
    taxa: &mut LawTaxa,
) -> Vec<ProvisionTaxa> {
    let mut provision_taxa: Vec<ProvisionTaxa> = Vec::new();

    for batch in batches {
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
            // Base case filter: skip provisions that can't contain legal obligations.
            let scope = fractalaw_core::taxa::provision_scope(
                Some(section_type.as_str()),
                &text,
                &[], // purposes not yet known — Pass 1 only (section_type + text length)
            );
            if scope == fractalaw_core::taxa::ProvisionScope::Out {
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

            // Pass 2: re-evaluate scope with purposes now available
            let purpose_strs: Vec<&str> = record.purposes.iter().map(|s| &**s).collect();
            let scope = fractalaw_core::taxa::provision_scope(
                Some(section_type.as_str()),
                &text,
                &purpose_strs,
            );
            // OUT = no actors at all (headings, titles, stubs)
            // STRUCTURAL = actors kept but defaulted to "mentioned" (handled by should_default_to_mentioned below)
            let is_structural = scope == fractalaw_core::taxa::ProvisionScope::Out;

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

                // Purpose gate: override positions to "mentioned" for structural
                // provisions without duty-bearing modals.
                if fractalaw_core::taxa::should_default_to_mentioned(
                    &record.purposes,
                    &text,
                ) {
                    if let Some(pt) = provision_taxa.last_mut() {
                        for actor in &mut pt.actors {
                            actor.position = "mentioned".to_string();
                            actor.reason = Some(format!(
                                "purpose_gated:mentioned ({})",
                                record.purposes.join(", ")
                            ));
                        }
                    }
                }
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

    provision_taxa
}

/// Infer holders for Rule provisions by finding the most frequent governed actor.
fn backlink_actors(taxa: &mut LawTaxa) {
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

/// Apply escalation tiers: Tier 1 (deterministic parent inheritance),
/// Tier 2 (LLM classification), Tier 3 (LLM position classification).
async fn apply_escalation(
    batches: &[RecordBatch],
    existing_tiers: &std::collections::HashMap<String, u8>,
    provision_taxa: &mut Vec<ProvisionTaxa>,
    taxa: &mut LawTaxa,
) -> anyhow::Result<()> {
    let mut inherited_count = 0u32;

    // Build a snapshot of section_id → index for parent lookups.
    // We iterate by index so we can read from immutable slices while
    // collecting mutations, then apply them in a second pass.
    let escalate_candidates: Vec<usize> = (0..provision_taxa.len())
        .filter(|&i| {
            let p = &provision_taxa[i];
            let no_actors = p.governed_actors.is_empty() && p.government_actors.is_empty();
            let needs_inheritance =
                // Original: no DRRP, no actors, duty-bearing purpose
                (p.drrp_types.is_empty() && no_actors && fractalaw_core::taxa::is_duty_bearing_purpose(&p.purposes))
                // New: has DRRP but no actors (thing-subject orphans)
                || (!p.drrp_types.is_empty() && no_actors);
            needs_inheritance && !p.hierarchy_path.is_empty()
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
        let had_drrp = !p.drrp_types.is_empty();
        if !had_drrp {
            // No DRRP — inherit everything (original behaviour)
            p.drrp_types = m.drrp_types;
            p.duty_family = m.duty_family;
            p.duty_sub_type = m.duty_sub_type;
        }
        // Always inherit actors if missing
        p.governed_actors = m.governed_actors;
        p.government_actors = m.government_actors;
        p.extraction_method = if had_drrp {
            // Keep regex/classifier DRRP, just inherit actors
            p.extraction_method.clone()
        } else {
            "inherited".to_string()
        };
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
                for batch in batches {
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
            for batch in batches {
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

    Ok(())
}

/// Build Arrow RecordBatch from per-provision taxa and write to LanceDB.
async fn write_provision_taxa(
    lance: &dyn ProvisionStore,
    law_name: &str,
    provision_taxa: &[ProvisionTaxa],
    existing_tiers: &std::collections::HashMap<String, u8>,
    force: bool,
) -> anyhow::Result<()> {
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
    for pt in provision_taxa {
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
    Ok(())
}

/// Check taxa hash, write law-level taxa to DuckDB if changed.
fn write_law_taxa(
    store: &DuckStore,
    law_name: &str,
    taxa: &LawTaxa,
) -> anyhow::Result<EnrichResult> {
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

