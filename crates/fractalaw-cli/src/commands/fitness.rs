//! Fitness applicability extraction — independent of DRRP taxa pipeline.
//!
//! Reads provision text from Postgres, runs polarity detection + dictionary
//! extraction, writes to `fitness_mentions` table. No tier protection,
//! no DRRP coupling.

use anyhow::Context;
use fractalaw_core::taxa::applicability::ApplicabilityNode;
use fractalaw_core::taxa::fitness;
use fractalaw_store::{DuckStore, PgStore};
use regex::Regex;
use std::sync::LazyLock;

/// Look up law → family mapping from DuckDB.
fn load_family_map(duck: &DuckStore) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Ok(batches) = duck.query_arrow("SELECT name, family FROM legislation WHERE family IS NOT NULL") {
        for batch in &batches {
            let name_col = batch.column_by_name("name");
            let fam_col = batch.column_by_name("family");
            if let (Some(n), Some(f)) = (name_col, fam_col) {
                for i in 0..batch.num_rows() {
                    if let (Some(name), Some(fam)) = (
                        crate::utils::get_string_value(n.as_ref(), i),
                        crate::utils::get_string_value(f.as_ref(), i),
                    ) {
                        map.insert(name, fam);
                    }
                }
            }
        }
    }
    map
}

/// Extract fitness mentions for laws (or all laws if none specified).
pub(crate) async fn cmd_fitness_extract(
    pg_url: &str,
    duck: &DuckStore,
    law_names: Option<&[String]>,
    force: bool,
) -> anyhow::Result<()> {
    let store = PgStore::connect(pg_url)
        .await
        .context("connecting to PostgreSQL")?;
    let pool = store.pool();

    // Load law → family mapping for specialist dictionary selection
    let family_map = load_family_map(duck);
    eprintln!("Loaded {} law→family mappings from DuckDB", family_map.len());

    // Ensure the fitness_mentions table exists
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS fitness_mentions (
            id              SERIAL PRIMARY KEY,
            section_id      TEXT NOT NULL,
            span            TEXT,
            polarity        TEXT NOT NULL,
            scope_unit      TEXT,
            entities        TEXT[],
            scope_dimensions TEXT[],
            extraction_method TEXT NOT NULL DEFAULT 'regex',
            confidence      REAL,
            source_detail   TEXT,
            created_at      TIMESTAMPTZ DEFAULT now(),
            updated_at      TIMESTAMPTZ DEFAULT now()
        )",
    )
    .execute(pool)
    .await?;

    // Optionally clear regex-tier COLUMNS for re-extraction.
    // NEVER deletes rows — only NULLs the regex_entities/regex_scope_dimensions columns.
    // Other tiers (slm_entities, ft_entities) are untouched.
    if force {
        if let Some(names) = law_names {
            for name in names {
                sqlx::query(
                    "UPDATE fitness_mentions SET regex_entities = NULL, regex_scope_dimensions = NULL \
                     WHERE section_id LIKE $1"
                )
                .bind(format!("{name}:%"))
                .execute(pool)
                .await?;
            }
            eprintln!("Cleared regex_entities for {} laws (other tiers preserved)", names.len());
        } else {
            sqlx::query(
                "UPDATE fitness_mentions SET regex_entities = NULL, regex_scope_dimensions = NULL"
            )
            .execute(pool)
            .await?;
            eprintln!("Cleared all regex_entities (other tiers preserved)");
        }
    }

    // Query provisions — filter by law names if specified, skip those
    // that already have mentions (unless --force cleared them)
    let rows = if let Some(names) = law_names {
        let name_list = names
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>();
        sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT lt.section_id, lt.text, lt.scope
             FROM legislation_text lt
             WHERE lt.text IS NOT NULL AND lt.text != ''
             AND split_part(lt.section_id, ':', 1) = ANY($1)
             AND NOT EXISTS (
                 SELECT 1 FROM fitness_mentions fm
                 WHERE fm.section_id = lt.section_id
                 AND fm.extraction_method IN ('regex', 'slm', 'manual')
             )",
        )
        .bind(&name_list)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT lt.section_id, lt.text, lt.scope
             FROM legislation_text lt
             WHERE lt.text IS NOT NULL AND lt.text != ''
             AND NOT EXISTS (
                 SELECT 1 FROM fitness_mentions fm
                 WHERE fm.section_id = lt.section_id
                 AND fm.extraction_method IN ('regex', 'slm', 'manual')
             )",
        )
        .fetch_all(pool)
        .await?
    };

    eprintln!("Processing {} provisions...", rows.len());

    let mut inserted = 0u32;
    let mut polarity_count = 0u32;

    for (section_id, text, scope) in &rows {
        // Skip out-of-scope provisions (headings, stubs)
        if scope.as_deref() == Some("out") {
            continue;
        }

        let cleaned = fractalaw_core::taxa::text_cleaner::clean(text);

        // Phase 1: polarity detection (independent of DRRP)
        let polarities = fitness::detect_polarity(&cleaned);
        if polarities.is_empty() {
            continue;
        }

        polarity_count += 1;

        // Phase 2a+2d: dictionary extraction with family-scoped specialists
        let law_name = section_id.split(':').next().unwrap_or("");
        let family = family_map.get(law_name).map(|s| s.as_str());
        let rules = fitness::extract(&cleaned, family);

        if rules.is_empty() {
            // Polarity detected but no dictionary matches.
            // Check for temporal entities (commencement/sunset dates).
            let temporal_entity = extract_date(&cleaned);

            for pol in &polarities {
                if let Some(ref date) = temporal_entity {
                    sqlx::query(
                        "INSERT INTO fitness_mentions
                         (section_id, polarity, regex_entities, regex_scope_dimensions, extraction_method, confidence, source_detail)
                         VALUES ($1, $2, $3, $4, 'regex', 0.9, 'date_extraction')",
                    )
                    .bind(section_id)
                    .bind(pol.as_str())
                    .bind(&[date.clone()] as &[String])
                    .bind(&["temporal".to_string()] as &[String])
                    .execute(pool)
                    .await?;
                } else {
                    sqlx::query(
                        "INSERT INTO fitness_mentions
                         (section_id, polarity, extraction_method, confidence, source_detail)
                         VALUES ($1, $2, 'regex', 0.7, 'polarity_only')",
                    )
                    .bind(section_id)
                    .bind(pol.as_str())
                    .execute(pool)
                    .await?;
                }
                inserted += 1;
            }
        } else {
            // Dictionary matched — store per-rule mentions with entities
            for rule in &rules {
                let entities: Vec<String> =
                    rule.tags.iter().map(|t| t.term.clone()).collect();
                let dims: Vec<String> = {
                    let mut d = std::collections::BTreeSet::new();
                    for tag in &rule.tags {
                        match tag.dimension {
                            fitness::PDimension::Person => {
                                d.insert("personal".to_string());
                            }
                            fitness::PDimension::Process
                            | fitness::PDimension::Plant
                            | fitness::PDimension::Sector => {
                                d.insert("material".to_string());
                            }
                            fitness::PDimension::Place => {
                                d.insert("territorial".to_string());
                            }
                            fitness::PDimension::Property => {
                                d.insert("conditional".to_string());
                            }
                        }
                    }
                    d.into_iter().collect()
                };

                let entities_opt: Option<&[String]> =
                    if entities.is_empty() { None } else { Some(&entities) };
                let dims_opt: Option<&[String]> =
                    if dims.is_empty() { None } else { Some(&dims) };

                sqlx::query(
                    "INSERT INTO fitness_mentions
                     (section_id, polarity, regex_entities, regex_scope_dimensions, extraction_method, confidence, source_detail)
                     VALUES ($1, $2, $3, $4, 'regex', 0.8, 'core_dict')",
                )
                .bind(section_id)
                .bind(rule.polarity.as_str())
                .bind(entities_opt)
                .bind(dims_opt)
                .execute(pool)
                .await?;
                inserted += 1;
            }
        }

        if polarity_count % 1000 == 0 {
            eprint!("\r  {polarity_count} provisions with polarity, {inserted} mentions...");
        }
    }

    eprintln!(
        "\nDone. {} provisions with polarity, {} mentions inserted.",
        polarity_count, inserted
    );

    // ── Date backfill: extract commencement/sunset dates into existing mentions ──
    // These provisions already have polarity-only mentions but no entities.
    // The COMMENCEMENT_RE flagged them; now extract the actual date.
    let date_rows = sqlx::query_as::<_, (i32, String)>(
        "SELECT fm.id, lt.text
         FROM fitness_mentions fm
         JOIN legislation_text lt ON fm.section_id = lt.section_id
         WHERE (fm.regex_entities IS NULL OR fm.regex_entities = '{}')
         AND (fm.slm_entities IS NULL OR fm.slm_entities = '{}')
         AND lt.text IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut dates_extracted = 0u32;
    for (mention_id, text) in &date_rows {
        let cleaned = fractalaw_core::taxa::text_cleaner::clean(text);
        if let Some(date) = extract_date(&cleaned) {
            sqlx::query(
                "UPDATE fitness_mentions
                 SET regex_entities = ARRAY[$1],
                     regex_scope_dimensions = ARRAY['temporal'],
                     source_detail = 'date_extraction',
                     confidence = 0.9
                 WHERE id = $2",
            )
            .bind(&date)
            .bind(mention_id)
            .execute(pool)
            .await?;
            dates_extracted += 1;
        }
    }

    if dates_extracted > 0 {
        eprintln!("Date backfill: {dates_extracted} commencement/sunset dates extracted.");
    }

    Ok(())
}

// ── Temporal entity extraction ──────────────────────────────────────

/// UK legislation date: "30th November 2017", "1st January 2025"
static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(\d{1,2})(?:st|nd|rd|th)?\s+(January|February|March|April|May|June|July|August|September|October|November|December)\s+(\d{4})"
    ).unwrap()
});

/// Month-year only: "April 2011"
static MONTH_YEAR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(January|February|March|April|May|June|July|August|September|October|November|December)\s+(\d{4})"
    ).unwrap()
});

/// Extract a date from commencement/sunset text as an ISO date string.
fn extract_date(text: &str) -> Option<String> {
    // Only try date extraction on commencement/sunset provisions
    let is_temporal = text.contains("come into force")
        || text.contains("comes into force")
        || text.contains("came into force")
        || text.contains("enter into force")
        || text.contains("enters into force")
        || text.contains("ceases to have effect")
        || text.contains("ceased to have effect");

    if !is_temporal {
        return None;
    }

    let months = [
        "january", "february", "march", "april", "may", "june",
        "july", "august", "september", "october", "november", "december",
    ];

    // Try full date first
    if let Some(caps) = DATE_RE.captures(text) {
        let day: u32 = caps[1].parse().ok()?;
        let month_name = caps[2].to_lowercase();
        let month = months.iter().position(|&m| m == month_name)? as u32 + 1;
        let year: u32 = caps[3].parse().ok()?;
        return Some(format!("{year:04}-{month:02}-{day:02}"));
    }

    // Fall back to month-year
    if let Some(caps) = MONTH_YEAR_RE.captures(text) {
        let month_name = caps[1].to_lowercase();
        let month = months.iter().position(|&m| m == month_name)? as u32 + 1;
        let year: u32 = caps[2].parse().ok()?;
        return Some(format!("{year:04}-{month:02}-01"));
    }

    None
}

/// Show fitness mention coverage for laws.
pub(crate) async fn cmd_fitness_status(
    pg_url: &str,
    law_names: Option<&[String]>,
) -> anyhow::Result<()> {
    let store = PgStore::connect(pg_url)
        .await
        .context("connecting to PostgreSQL")?;
    let pool = store.pool();

    let rows = if let Some(names) = law_names {
        let name_list = names.iter().map(|n| n.as_str()).collect::<Vec<_>>();
        sqlx::query_as::<_, (String, i64, i64, i64)>(
            "SELECT split_part(fm.section_id, ':', 1) as law_name,
                    count(*) as mentions,
                    count(*) FILTER (WHERE array_length(fm.entities, 1) > 0) as with_entities,
                    count(DISTINCT fm.polarity) as polarity_types
             FROM fitness_mentions fm
             WHERE split_part(fm.section_id, ':', 1) = ANY($1)
             GROUP BY 1
             ORDER BY 1",
        )
        .bind(&name_list)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, (String, i64, i64, i64)>(
            "SELECT split_part(fm.section_id, ':', 1) as law_name,
                    count(*) as mentions,
                    count(*) FILTER (WHERE array_length(fm.entities, 1) > 0) as with_entities,
                    count(DISTINCT fm.polarity) as polarity_types
             FROM fitness_mentions fm
             GROUP BY 1
             ORDER BY count(*) DESC
             LIMIT 30",
        )
        .fetch_all(pool)
        .await?
    };

    println!(
        "{:<50} {:>8} {:>12} {:>10}",
        "Law", "Mentions", "With entities", "Polarities"
    );
    println!("{}", "-".repeat(82));

    let mut total_mentions = 0i64;
    let mut total_entities = 0i64;
    for (name, mentions, with_ent, pol_types) in &rows {
        total_mentions += mentions;
        total_entities += with_ent;
        println!(
            "{:<50} {:>8} {:>12} {:>10}",
            name, mentions, with_ent, pol_types
        );
    }
    println!("{}", "-".repeat(82));
    println!(
        "{:<50} {:>8} {:>12}",
        "TOTAL", total_mentions, total_entities
    );

    Ok(())
}

// ── Expression tree compiler ────────────────────────────────────────

/// Compile fitness mentions into expression trees per law, write to DuckDB.
pub(crate) async fn cmd_fitness_compile(
    pg_url: &str,
    duck: &DuckStore,
    law_names: Option<&[String]>,
) -> anyhow::Result<()> {
    let store = PgStore::connect(pg_url)
        .await
        .context("connecting to PostgreSQL")?;
    let pool = store.pool();

    // Load entity → scope dimension mapping
    let entity_dims: std::collections::HashMap<String, String> = {
        let rows = sqlx::query_as::<_, (String, Vec<String>)>(
            "SELECT display_name, scope_dimensions FROM fitness_entities",
        )
        .fetch_all(pool)
        .await?;
        rows.into_iter()
            .map(|(name, dims)| (name.to_lowercase(), dims.first().cloned().unwrap_or_else(|| "material".to_string())))
            .collect()
    };

    // Get all non-propagated mentions grouped by law
    let mentions = if let Some(names) = law_names {
        let name_list: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        sqlx::query_as::<_, (String, String, Option<String>, Option<Vec<String>>, Option<Vec<String>>)>(
            "SELECT split_part(section_id, ':', 1) as law_name, polarity, scope_unit, entities, scope_dimensions \
             FROM fitness_mentions \
             WHERE extraction_method != 'propagated' \
             AND entities IS NOT NULL AND entities != '{}' \
             AND split_part(section_id, ':', 1) = ANY($1) \
             ORDER BY split_part(section_id, ':', 1), scope_unit, polarity",
        )
        .bind(&name_list)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, (String, String, Option<String>, Option<Vec<String>>, Option<Vec<String>>)>(
            "SELECT split_part(section_id, ':', 1) as law_name, polarity, scope_unit, entities, scope_dimensions \
             FROM fitness_mentions \
             WHERE extraction_method != 'propagated' \
             AND entities IS NOT NULL AND entities != '{}' \
             ORDER BY split_part(section_id, ':', 1), scope_unit, polarity",
        )
        .fetch_all(pool)
        .await?
    };

    eprintln!("Loaded {} mentions for compilation", mentions.len());

    // Group by law
    let mut by_law: std::collections::BTreeMap<String, Vec<(String, Option<String>, Vec<String>, Vec<String>)>> =
        std::collections::BTreeMap::new();
    for (law, pol, scope, ents, dims) in mentions {
        let ents = ents.unwrap_or_default();
        let dims = dims.unwrap_or_default();
        if !ents.is_empty() {
            by_law.entry(law).or_default().push((pol, scope, ents, dims));
        }
    }

    // Ensure DuckDB column exists
    let _ = duck.execute("ALTER TABLE legislation ADD COLUMN compiled_applicability VARCHAR");

    // Compile each law
    let mut compiled = 0u32;
    for (law_name, mentions) in &by_law {
        let tree = compile_law(mentions, &entity_dims);
        if let Some(tree) = tree {
            let json = tree.to_json().map_err(|e| anyhow::anyhow!("JSON error for {law_name}: {e}"))?;
            duck.execute(&format!(
                "UPDATE legislation SET compiled_applicability = '{}' WHERE name = '{}'",
                json.replace('\'', "''"),
                law_name.replace('\'', "''"),
            ))?;
            compiled += 1;
        }
    }

    eprintln!("Compiled expression trees for {compiled}/{} laws", by_law.len());
    Ok(())
}

/// Compile all mentions for a single law into an expression tree.
fn compile_law(
    mentions: &[(String, Option<String>, Vec<String>, Vec<String>)],
    entity_dims: &std::collections::HashMap<String, String>,
) -> Option<ApplicabilityNode> {
    let mut applies_nodes = Vec::new();
    let mut disapplies_nodes = Vec::new();
    let mut time_nodes = Vec::new();

    for (polarity, _scope, entities, _dims) in mentions {
        // Check for temporal entities (ISO dates)
        let temporal: Vec<&String> = entities.iter().filter(|e| is_iso_date(e)).collect();
        let non_temporal: Vec<&String> = entities.iter().filter(|e| !is_iso_date(e)).collect();

        // Temporal → TimeWindow
        for date in &temporal {
            match polarity.as_str() {
                "AppliesTo" | "ExtendsTo" => {
                    time_nodes.push(ApplicabilityNode::time_window(Some(date), None));
                }
                "DisappliesTo" => {
                    time_nodes.push(ApplicabilityNode::time_window(None, Some(date)));
                }
                _ => {}
            }
        }

        if non_temporal.is_empty() {
            continue;
        }

        // Group entities by scope dimension
        let mut by_dim: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
        for entity in &non_temporal {
            let dim = entity_dims
                .get(&entity.to_lowercase())
                .cloned()
                .unwrap_or_else(|| "material".to_string());
            by_dim.entry(dim).or_default().push(
                entity.to_lowercase().replace(' ', "_"),
            );
        }

        // Each dimension → Match node (same dimension = OR)
        let mut dim_nodes: Vec<ApplicabilityNode> = Vec::new();
        for (dim, codes) in by_dim {
            dim_nodes.push(ApplicabilityNode::match_any(&dim, codes));
        }

        // Multiple dimensions → AND
        let mention_node = ApplicabilityNode::and(dim_nodes);

        match polarity.as_str() {
            "DisappliesTo" => disapplies_nodes.push(mention_node),
            _ => applies_nodes.push(mention_node),
        }
    }

    // Combine: applies AND NOT(disapplies) AND time_windows
    let mut top_nodes = Vec::new();

    if !applies_nodes.is_empty() {
        top_nodes.push(ApplicabilityNode::and(applies_nodes));
    }
    for dis in disapplies_nodes {
        top_nodes.push(dis.negate());
    }
    top_nodes.extend(time_nodes);

    if top_nodes.is_empty() {
        return None;
    }

    Some(ApplicabilityNode::and(top_nodes))
}

fn is_iso_date(s: &str) -> bool {
    s.len() == 10
        && s.as_bytes().get(4) == Some(&b'-')
        && s.as_bytes().get(7) == Some(&b'-')
        && s[..4].chars().all(|c| c.is_ascii_digit())
}
