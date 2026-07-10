//! Fitness applicability extraction — independent of DRRP taxa pipeline.
//!
//! Reads provision text from Postgres, runs polarity detection + dictionary
//! extraction, writes to `fitness_mentions` table. No tier protection,
//! no DRRP coupling.

use anyhow::Context;
use fractalaw_core::taxa::fitness;
use fractalaw_store::PgStore;

/// Extract fitness mentions for laws (or all laws if none specified).
pub(crate) async fn cmd_fitness_extract(
    pg_url: &str,
    law_names: Option<&[String]>,
    force: bool,
) -> anyhow::Result<()> {
    let store = PgStore::connect(pg_url)
        .await
        .context("connecting to PostgreSQL")?;
    let pool = store.pool();

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

    // Optionally clear existing mentions for these laws
    if force {
        if let Some(names) = law_names {
            for name in names {
                sqlx::query("DELETE FROM fitness_mentions WHERE section_id LIKE $1")
                    .bind(format!("{name}:%"))
                    .execute(pool)
                    .await?;
            }
            eprintln!("Cleared fitness_mentions for {} laws", names.len());
        } else {
            sqlx::query("DELETE FROM fitness_mentions")
                .execute(pool)
                .await?;
            eprintln!("Cleared all fitness_mentions");
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

        // Phase 2a: dictionary extraction (core dictionaries only for now)
        let rules = fitness::extract(&cleaned, None);

        if rules.is_empty() {
            // Polarity detected but no dictionary matches — store
            // polarity-only mention (gap for NER to fill later)
            for pol in &polarities {
                sqlx::query(
                    "INSERT INTO fitness_mentions
                     (section_id, polarity, extraction_method, confidence, source_detail)
                     VALUES ($1, $2, 'regex', 0.7, 'polarity_only')",
                )
                .bind(section_id)
                .bind(pol.as_str())
                .execute(pool)
                .await?;
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
                     (section_id, polarity, entities, scope_dimensions, extraction_method, confidence, source_detail)
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
    Ok(())
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
