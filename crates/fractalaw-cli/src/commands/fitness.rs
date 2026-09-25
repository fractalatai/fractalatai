//! Fitness applicability extraction — independent of DRRP taxa pipeline.
//!
//! Reads provision text from Postgres, runs polarity detection + dictionary
//! extraction, writes to `fitness_mentions` table. No tier protection,
//! no DRRP coupling.

use anyhow::Context;
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
/// Fill the reconciled `entities` / `scope_dimensions` columns for mentions
/// that have tier output but were never reconciled (e.g. extraction batches run
/// after the July reconcile). Entities: first non-empty of ft > regex > slm.
/// Scope dimensions: union across tiers. Rows already reconciled and the tier
/// columns themselves are never modified.
pub(crate) async fn cmd_fitness_reconcile(
    pg_url: &str,
    law_names: Option<&[String]>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let store = PgStore::connect(pg_url)
        .await
        .context("connecting to PostgreSQL")?;
    let pool = store.pool();

    let law_filter = if law_names.is_some() {
        "AND split_part(section_id, ':', 1) = ANY($1)"
    } else {
        ""
    };
    let pending_where = format!(
        "extraction_method != 'propagated' \
         AND (entities IS NULL OR entities = '{{}}') \
         AND (cardinality(ft_entities) > 0 OR cardinality(regex_entities) > 0 OR cardinality(slm_entities) > 0) {law_filter}"
    );

    let count_sql = format!(
        "SELECT count(*), count(DISTINCT split_part(section_id, ':', 1)) FROM fitness_mentions WHERE {pending_where}"
    );
    let mut q = sqlx::query_as::<_, (i64, i64)>(&count_sql);
    if let Some(names) = law_names {
        q = q.bind(names.to_vec());
    }
    let (rows, laws) = q.fetch_one(pool).await?;
    println!("{rows} unreconciled mentions across {laws} laws");
    if dry_run || rows == 0 {
        return Ok(());
    }

    let update_sql = format!(
        "UPDATE fitness_mentions SET \
           entities = COALESCE(NULLIF(ft_entities, '{{}}'), NULLIF(regex_entities, '{{}}'), NULLIF(slm_entities, '{{}}')), \
           scope_dimensions = ARRAY(SELECT DISTINCT d FROM unnest( \
               COALESCE(ft_scope_dimensions, '{{}}') || COALESCE(regex_scope_dimensions, '{{}}') \
               || COALESCE(slm_scope_dimensions, '{{}}')) AS d ORDER BY d), \
           updated_at = now() \
         WHERE {pending_where}"
    );
    let mut q = sqlx::query(&update_sql);
    if let Some(names) = law_names {
        q = q.bind(names.to_vec());
    }
    let result = q.execute(pool).await?;
    println!("Reconciled {} mentions", result.rows_affected());
    Ok(())
}

/// Per-law title and application, for compile and `fitness application`.
pub(crate) struct LawMeta {
    pub title: String,
    pub application: Option<fractalaw_core::taxa::application::Application>,
}

fn law_context(
    meta: &std::collections::HashMap<String, LawMeta>,
    law_name: &str,
) -> fractalaw_core::taxa::applicability_compile::LawContext {
    let m = meta.get(law_name);
    fractalaw_core::taxa::applicability_compile::LawContext {
        title: m.map(|m| m.title.clone()).unwrap_or_default(),
        application: m.and_then(|m| m.application.as_ref()).map(|a| a.regions.clone()),
    }
}

/// Load title (DuckDB, else the law's citation provision) and derive application
/// (text clauses, title, type code, extent) for every law in DuckDB, or `law_names`.
pub(crate) async fn load_law_meta(
    pool: &sqlx::PgPool,
    duck: &DuckStore,
    law_names: Option<&[String]>,
) -> anyhow::Result<std::collections::HashMap<String, LawMeta>> {
    use arrow::array::{Array, StringArray};
    use fractalaw_core::taxa::applicability_compile::title_from_citation;
    use fractalaw_core::taxa::application::{ApplicationInput, derive_application};
    use std::collections::HashMap;

    // LRT: name, type_code, title, extent_code, extent_source (v2.4; NULL = legacy/unverified)
    duck.execute("ALTER TABLE legislation ADD COLUMN IF NOT EXISTS extent_source VARCHAR")?;
    type LrtRow = (String, String, Option<String>, Option<String>, Option<String>);
    let mut lrt: Vec<LrtRow> = Vec::new();
    for batch in duck.query_arrow("SELECT name, type_code, title, extent_code, extent_source FROM legislation")? {
        let col = |i: usize| batch.column(i).as_any().downcast_ref::<StringArray>().cloned();
        let (Some(n), Some(t), Some(ti), Some(e), Some(es)) = (col(0), col(1), col(2), col(3), col(4)) else {
            continue;
        };
        for i in 0..batch.num_rows() {
            if n.is_null(i) {
                continue;
            }
            let opt = |a: &StringArray| (!a.is_null(i)).then(|| a.value(i).to_string());
            lrt.push((n.value(i).to_string(), opt(&t).unwrap_or_default(), opt(&ti), opt(&e), opt(&es)));
        }
    }
    if let Some(names) = law_names {
        lrt.retain(|(n, ..)| names.iter().any(|x| x == n));
    }

    let law_filter = if law_names.is_some() { "AND law_name = ANY($1)" } else { "" };
    let bind = |sql: String| {
        let names = law_names.map(|n| n.to_vec());
        async move {
            let mut q = sqlx::query_as::<_, (String, String, Option<String>)>(&sql);
            if let Some(names) = names {
                q = q.bind(names);
            }
            q.fetch_all(pool).await
        }
    };

    // Provisions that could state application or extent
    let mut clauses: HashMap<String, Vec<(String, String)>> = HashMap::new();
    // Stems that state application/extent, with their sub-provisions joined
    // ("These Regulations apply— (a) in Great Britain; and (b) ...")
    let stem_filter = law_filter.replace("law_name", "s.law_name");
    for (law, sid, text) in bind(format!(
        "SELECT s.law_name, s.section_id, \
                s.text || ' ' || coalesce((SELECT string_agg(c.text, ' ' ORDER BY c.sort_key) \
                    FROM legislation_text c WHERE c.law_name = s.law_name \
                    AND c.section_id LIKE s.section_id || '(%'), '') \
         FROM legislation_text s \
         WHERE s.text ~* '(these\\s+regulations|this\\s+(act|order|measure|scheme|instrument)|these\\s+(rules|byelaws))' \
         AND s.text ~* '(appl(y|ies)|extends?)' {stem_filter}"
    ))
    .await?
    {
        if let Some(t) = text {
            clauses.entry(law).or_default().push((sid, t));
        }
    }

    // Citation provisions (title fallback)
    let mut cited: HashMap<String, String> = HashMap::new();
    for (law, _sid, text) in bind(format!(
        "SELECT law_name, section_id, text FROM legislation_text WHERE text ~* 'may\\s+be\\s+cited\\s+as' {law_filter}"
    ))
    .await?
    {
        if let Some(title) = text.as_deref().and_then(title_from_citation) {
            cited.entry(law).or_insert(title);
        }
    }

    // LAT provision extents
    let mut lat_extents: HashMap<String, Vec<String>> = HashMap::new();
    for (law, code, _) in bind(format!(
        "SELECT DISTINCT law_name, extent_code, NULL::text FROM legislation_text \
         WHERE extent_code IS NOT NULL AND extent_code <> '' {law_filter}"
    ))
    .await?
    {
        lat_extents.entry(law).or_default().push(code);
    }

    let empty_p: Vec<(String, String)> = Vec::new();
    let empty_e: Vec<String> = Vec::new();
    let mut meta = HashMap::new();
    for (name, type_code, title, extent, extent_source) in lrt {
        let title = title.filter(|t| !t.is_empty()).or_else(|| cited.get(&name).cloned()).unwrap_or_default();
        let application = derive_application(&ApplicationInput {
            type_code: &type_code,
            title: &title,
            provisions: clauses.get(&name).unwrap_or(&empty_p),
            lat_extents: lat_extents.get(&name).unwrap_or(&empty_e),
            lrt_extent: extent.as_deref(),
            lrt_extent_source: extent_source.as_deref(),
        });
        meta.insert(name, LawMeta { title, application });
    }
    Ok(meta)
}

/// Derive law application for every law (or `law_names`) and store it in DuckDB
/// (`application_regions`, `application_source`, `application_evidence`), or
/// write JSONL to `out`. Not published until ZENOH-SPEC v2.4 (sertantai-legal #163).
pub(crate) async fn cmd_fitness_application(
    pg_url: &str,
    duck: &DuckStore,
    law_names: Option<&[String]>,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    let store = PgStore::connect(pg_url)
        .await
        .context("connecting to PostgreSQL")?;
    let meta = load_law_meta(store.pool(), duck, law_names).await?;

    let mut by_source: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut names: Vec<&String> = meta.keys().collect();
    names.sort();

    if let Some(path) = out {
        use std::io::Write;
        let mut w = std::io::BufWriter::new(
            std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?,
        );
        for name in &names {
            let a = meta[*name].application.as_ref();
            *by_source.entry(a.map(|a| a.source.as_str()).unwrap_or("none")).or_default() += 1;
            writeln!(
                w,
                "{}",
                serde_json::json!({
                    "name": name,
                    "application_regions": a.map(|a| &a.regions),
                    "application_source": a.map(|a| a.source.as_str()),
                    "application_evidence": a.map(|a| &a.evidence),
                })
            )?;
        }
    } else {
        duck.execute("ALTER TABLE legislation ADD COLUMN IF NOT EXISTS application_regions VARCHAR[]")?;
        duck.execute("ALTER TABLE legislation ADD COLUMN IF NOT EXISTS application_source VARCHAR")?;
        duck.execute("ALTER TABLE legislation ADD COLUMN IF NOT EXISTS application_evidence VARCHAR")?;
        let q = |s: &str| format!("'{}'", s.replace('\'', "''"));
        for name in &names {
            let a = meta[*name].application.as_ref();
            *by_source.entry(a.map(|a| a.source.as_str()).unwrap_or("none")).or_default() += 1;
            let (regions, source, evidence) = match a {
                Some(a) => (
                    format!("[{}]", a.regions.iter().map(|r| q(r)).collect::<Vec<_>>().join(", ")),
                    q(a.source.as_str()),
                    q(&a.evidence),
                ),
                None => ("NULL".into(), "NULL".into(), "NULL".into()),
            };
            duck.execute(&format!(
                "UPDATE legislation SET application_regions = {regions}, application_source = {source}, \
                 application_evidence = {evidence} WHERE name = {}",
                q(name)
            ))?;
        }
    }

    println!("Application derived for {} laws:", names.len());
    for (source, n) in by_source {
        println!("  {source:16} {n}");
    }
    Ok(())
}

pub(crate) async fn cmd_fitness_compile(
    pg_url: &str,
    duck: &DuckStore,
    law_names: Option<&[String]>,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    use fractalaw_core::taxa::applicability::ApplicabilityNode;
    use fractalaw_core::taxa::applicability_compile::{MentionInput, compile_law, repair_tree};

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

    // Non-propagated mentions (with section_id so we can attach provision text)
    let law_filter = "AND split_part(section_id, ':', 1) = ANY($1)";
    let sql = format!(
        "SELECT split_part(section_id, ':', 1) AS law_name, section_id, polarity, entities \
         FROM fitness_mentions \
         WHERE extraction_method != 'propagated' \
         AND entities IS NOT NULL AND entities != '{{}}' {} \
         ORDER BY 1, section_id, polarity",
        if law_names.is_some() { law_filter } else { "" }
    );
    let mut q = sqlx::query_as::<_, (String, String, String, Option<Vec<String>>)>(&sql);
    if let Some(names) = law_names {
        q = q.bind(names.to_vec());
    }
    let mentions = q.fetch_all(pool).await?;
    eprintln!("Loaded {} mentions for compilation", mentions.len());

    // Group by law
    let mut by_law: std::collections::BTreeMap<String, Vec<(String, String, Vec<String>)>> =
        std::collections::BTreeMap::new();
    for (law, section_id, pol, ents) in mentions {
        let ents = ents.unwrap_or_default();
        if !ents.is_empty() {
            by_law.entry(law).or_default().push((section_id, pol, ents));
        }
    }

    let _ = duck.execute("ALTER TABLE legislation ADD COLUMN compiled_applicability VARCHAR");

    // Law title + application (grounding context and root gate)
    let meta = load_law_meta(pool, duck, law_names).await?;

    let mut out_file = match out {
        Some(path) => Some(std::io::BufWriter::new(
            std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?,
        )),
        None => None,
    };

    let mut compiled = 0u32;
    for (law_name, law_mentions) in &by_law {
        // Provision text for grounding: each mention sees its provision + sub-provisions
        let rows = sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT section_id, text FROM legislation_text WHERE law_name = $1",
        )
        .bind(law_name)
        .fetch_all(pool)
        .await?;
        let text_for = |section_id: &str| -> String {
            let child_prefix = format!("{section_id}(");
            rows.iter()
                .filter(|(sid, _)| sid == section_id || sid.starts_with(&child_prefix))
                .filter_map(|(_, t)| t.as_deref())
                .collect::<Vec<_>>()
                .join(" ")
        };

        let inputs: Vec<MentionInput> = law_mentions
            .iter()
            .map(|(sid, pol, ents)| MentionInput {
                polarity: pol.clone(),
                entities: ents.clone(),
                text: text_for(sid),
            })
            .collect();

        let ctx = law_context(&meta, law_name);
        let tree = compile_law(&inputs, &entity_dims, &ctx);
        let json = match &tree {
            Some(t) => Some(t.to_json().map_err(|e| anyhow::anyhow!("JSON error for {law_name}: {e}"))?),
            None => None,
        };

        if let Some(w) = out_file.as_mut() {
            use std::io::Write;
            let line = serde_json::json!({ "name": law_name, "tree": tree });
            writeln!(w, "{line}")?;
        } else {
            let value = match &json {
                Some(j) => format!("'{}'", j.replace('\'', "''")),
                None => "NULL".to_string(),
            };
            duck.execute(&format!(
                "UPDATE legislation SET compiled_applicability = {value} WHERE name = '{}'",
                law_name.replace('\'', "''"),
            ))?;
        }
        if json.is_some() {
            compiled += 1;
        }
    }

    // Laws with an existing tree but no mentions left to compile from: repair in place
    let mut repaired = 0u32;
    let existing = duck.query_arrow("SELECT name, compiled_applicability FROM legislation WHERE compiled_applicability IS NOT NULL")?;
    for batch in &existing {
        use arrow::array::{Array, StringArray};
        let (Some(names), Some(trees)) = (
            batch.column(0).as_any().downcast_ref::<StringArray>(),
            batch.column(1).as_any().downcast_ref::<StringArray>(),
        ) else {
            continue;
        };
        for i in 0..batch.num_rows() {
            let law_name = names.value(i);
            if by_law.contains_key(law_name) || law_names.is_some_and(|ns| !ns.iter().any(|n| n == law_name)) {
                continue;
            }
            let Ok(old) = ApplicabilityNode::from_json(trees.value(i)) else { continue };
            let ctx = law_context(&meta, law_name);
            let tree = repair_tree(old, &ctx);
            if let Some(w) = out_file.as_mut() {
                use std::io::Write;
                writeln!(w, "{}", serde_json::json!({ "name": law_name, "tree": tree, "repaired": true }))?;
            } else {
                let value = match &tree {
                    Some(t) => format!("'{}'", t.to_json()?.replace('\'', "''")),
                    None => "NULL".to_string(),
                };
                duck.execute(&format!(
                    "UPDATE legislation SET compiled_applicability = {value} WHERE name = '{}'",
                    law_name.replace('\'', "''"),
                ))?;
            }
            repaired += 1;
        }
    }
    eprintln!("Repaired {repaired} existing trees with no mentions to compile from");

    match out {
        Some(path) => eprintln!(
            "Compiled expression trees for {compiled}/{} laws → {} (DuckDB untouched)",
            by_law.len(),
            path.display()
        ),
        None => eprintln!("Compiled expression trees for {compiled}/{} laws", by_law.len()),
    }
    Ok(())
}
