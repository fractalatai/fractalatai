use arrow::array::{Array, Float32Array, Float64Array, Int64Array, LargeStringArray, StringArray};
use arrow::record_batch::RecordBatch;
use fractalaw_store::DuckStore;

// Legacy FitnessEntry type alias removed.

/// Find the largest byte offset <= `max_bytes` that is a valid char boundary.
pub(crate) fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> usize {
    let mut end = max_bytes.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

/// Truncate a law name to fit a column width, preserving the end (most distinctive part).
pub(crate) fn truncate_name(name: &str, max: usize) -> String {
    if name.len() <= max {
        name.to_string()
    } else {
        format!("..{}", &name[name.len() - (max - 2)..])
    }
}

/// Format a number with comma thousands separators.
pub(crate) fn fmt_num(n: usize) -> String {
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
pub(crate) fn extract_i64(batch: &RecordBatch, col_idx: usize) -> i64 {
    batch
        .column(col_idx)
        .as_any()
        .downcast_ref::<Int64Array>()
        .map(|a| a.value(0))
        .unwrap_or(0)
}

/// Extract an f64 value from column `col_idx` of a RecordBatch.
///
/// Handles both Float64 (DuckDB aggregates) and Float32 columns.
pub(crate) fn extract_f64(batch: &RecordBatch, col_idx: usize) -> f64 {
    let col = batch.column(col_idx);
    if let Some(a) = col.as_any().downcast_ref::<Float64Array>() {
        a.value(0)
    } else if let Some(a) = col.as_any().downcast_ref::<Float32Array>() {
        a.value(0) as f64
    } else {
        0.0
    }
}

/// Extract a string value from an Arrow array, handling both Utf8 and LargeUtf8.
pub(crate) fn get_string_value(col: &dyn Array, i: usize) -> Option<String> {
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

/// Project RecordBatches to only include the specified columns.
pub(crate) fn project_batches(batches: &[RecordBatch], columns: &[&str]) -> Vec<RecordBatch> {
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

/// Format an iterator of strings as a DuckDB array literal: `['a', 'b']` or `NULL`.
pub(crate) fn format_sql_list<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let items: Vec<String> = values
        .map(|v| format!("'{}'", v.replace('\'', "''")))
        .collect();
    if items.is_empty() {
        "NULL".to_string()
    } else {
        format!("[{}]", items.join(", "))
    }
}

/// Format a list of (holder, duty_type, clause, article) tuples as a DuckDB
/// `List<Struct>` literal, e.g. `[{'holder':'a','duty_type':'DUTY','clause':'...','article':'s/1'}]`.
pub(crate) fn format_sql_drrp_entries(entries: &[(String, String, String, String)]) -> String {
    if entries.is_empty() {
        return "NULL".to_string();
    }
    let esc = |s: &str| s.replace('\'', "''");
    let items: Vec<String> = entries
        .iter()
        .map(|(holder, dt, clause, article)| {
            format!(
                "{{'holder':'{}','duty_type':'{}','clause':'{}','article':'{}'}}",
                esc(holder),
                esc(dt),
                esc(clause),
                esc(article)
            )
        })
        .collect();
    format!("[{}]", items.join(", "))
}

// Legacy format_sql_fitness_entries() removed.

/// Compute a content hash of the DRRP taxa columns from a `LawTaxa` struct.
///
/// Uses `DefaultHasher` (SipHash) over a canonical string built from sorted
/// column values. Returns a hex-encoded u64 hash.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_taxa_hash(
    duty_holders: &std::collections::BTreeSet<String>,
    rights_holders: &std::collections::BTreeSet<String>,
    responsibility_holders: &std::collections::BTreeSet<String>,
    power_holders: &std::collections::BTreeSet<String>,
    duty_types: &std::collections::BTreeSet<String>,
    roles: &std::collections::BTreeSet<String>,
    roles_gvt: &std::collections::BTreeSet<String>,
    duties: &[(String, String, String, String)],
    rights: &[(String, String, String, String)],
    responsibilities: &[(String, String, String, String)],
    powers: &[(String, String, String, String)],
) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();

    // BTreeSets are already sorted — iterate in order.
    for v in duty_holders {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF); // separator
    for v in rights_holders {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in responsibility_holders {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in power_holders {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in duty_types {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in roles {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    for v in roles_gvt {
        v.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);

    // DRRP entries: sort for determinism (Vecs may not be ordered).
    let mut sorted_duties: Vec<_> = duties.iter().collect();
    sorted_duties.sort();
    for (h, dt, c, a) in sorted_duties {
        h.hash(&mut hasher);
        dt.hash(&mut hasher);
        c.hash(&mut hasher);
        a.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    let mut sorted_rights: Vec<_> = rights.iter().collect();
    sorted_rights.sort();
    for (h, dt, c, a) in sorted_rights {
        h.hash(&mut hasher);
        dt.hash(&mut hasher);
        c.hash(&mut hasher);
        a.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    let mut sorted_resp: Vec<_> = responsibilities.iter().collect();
    sorted_resp.sort();
    for (h, dt, c, a) in sorted_resp {
        h.hash(&mut hasher);
        dt.hash(&mut hasher);
        c.hash(&mut hasher);
        a.hash(&mut hasher);
    }
    hasher.write_u8(0xFF);
    let mut sorted_powers: Vec<_> = powers.iter().collect();
    sorted_powers.sort();
    for (h, dt, c, a) in sorted_powers {
        h.hash(&mut hasher);
        dt.hash(&mut hasher);
        c.hash(&mut hasher);
        a.hash(&mut hasher);
    }

    // Legacy fitness hash inputs removed.

    format!("{:016x}", hasher.finish())
}

/// Extract candidate noun phrases from gap provision texts, filtering known
/// dictionary terms and stop words. Returns (term, frequency) pairs sorted
/// by frequency descending.
pub(crate) fn extract_candidate_terms(
    texts: &[&str],
    known_terms: &std::collections::BTreeSet<String>,
) -> Vec<(String, usize)> {
    use std::collections::{BTreeSet, HashMap};

    let stop_words: BTreeSet<&str> = [
        "the",
        "a",
        "an",
        "of",
        "to",
        "in",
        "for",
        "on",
        "at",
        "by",
        "or",
        "and",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "shall",
        "will",
        "would",
        "should",
        "may",
        "might",
        "can",
        "could",
        "not",
        "no",
        "nor",
        "but",
        "if",
        "so",
        "as",
        "than",
        "that",
        "this",
        "these",
        "those",
        "such",
        "any",
        "all",
        "each",
        "every",
        "with",
        "from",
        "into",
        "through",
        "during",
        "before",
        "after",
        "above",
        "below",
        "between",
        "under",
        "over",
        "about",
        "against",
        "without",
        "which",
        "who",
        "whom",
        "whose",
        "where",
        "when",
        "how",
        "what",
        "it",
        "its",
        "they",
        "them",
        "their",
        "he",
        "his",
        "she",
        "her",
        "we",
        "our",
        "you",
        "your",
        "me",
        "my",
        "him",
        "us",
        "only",
        "also",
        "other",
        "more",
        "most",
        "very",
        "just",
        "here",
        "there",
        "regulation",
        "regulations",
        "section",
        "paragraph",
        "sub-paragraph",
        "act",
        "order",
        "article",
        "part",
        "schedule",
        "provision",
        "provisions",
        "apply",
        "applies",
        "applied",
        "applying",
        "application",
        "must",
        "effect",
        "force",
        "extent",
        "relation",
        "respect",
        "case",
        "person",
        "persons",
    ]
    .into_iter()
    .collect();

    let mut freq: HashMap<String, usize> = HashMap::new();

    for text in texts {
        // Tokenize: split on non-alphabetic/hyphen boundaries, keep words with letters
        let words: Vec<&str> = text
            .split(|c: char| !c.is_alphabetic() && c != '-')
            .filter(|w| w.len() >= 2 && w.chars().any(|c| c.is_alphabetic()))
            .collect();

        for n in 1..=3usize {
            for window in words.windows(n) {
                // For 1-grams, skip stop words entirely
                if n == 1 && stop_words.contains(&window[0].to_lowercase().as_str()) {
                    continue;
                }
                // For multi-grams, skip if ALL words are stop words
                if n > 1
                    && window
                        .iter()
                        .all(|w: &&str| stop_words.contains(&w.to_lowercase().as_str()))
                {
                    continue;
                }

                let phrase = window
                    .iter()
                    .map(|w| w.to_lowercase())
                    .collect::<Vec<_>>()
                    .join(" ");

                if phrase.len() < 3 || known_terms.contains(&phrase) {
                    continue;
                }

                *freq.entry(phrase).or_insert(0) += 1;
            }
        }
    }

    let mut sorted: Vec<(String, usize)> =
        freq.into_iter().filter(|(_, count)| *count >= 2).collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted
}

/// Write classification results to DuckDB legislation table.
pub(crate) fn write_classifications(
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

/// Human-friendly law names for eyeball review headings.
pub(crate) fn law_display_name(law_name: &str) -> &str {
    match law_name {
        "UK_uksi_2005_1643" => "Control of Noise at Work 2005",
        "UK_uksi_1992_2792" => "Display Screen Equipment 1992",
        "UK_uksi_2005_1093" => "Control of Vibration at Work 2005",
        "UK_uksi_2002_2676" => "Control of Lead at Work 2002",
        "UK_uksi_2013_1471" => "RIDDOR 2013",
        "UK_uksi_2000_128" => "Pressure Systems Safety 2000",
        "UK_uksi_2015_483" => "COMAH 2015",
        "UK_ukpga_1974_37" => "Health and Safety at Work etc. Act 1974",
        "UK_uksi_1999_3242" => "Management of HSW Regulations 1999",
        "UK_uksi_2015_51" => "CDM 2015",
        _ => law_name,
    }
}

/// Query DuckDB for all law names belonging to a given family.
pub(crate) fn laws_in_family(store: &DuckStore, family: &str) -> anyhow::Result<Vec<String>> {
    let sql = format!(
        "SELECT name FROM legislation WHERE family = '{}' ORDER BY name",
        family.replace('\'', "''")
    );
    let batches = store.query_arrow(&sql)?;
    let mut names = Vec::new();
    for batch in &batches {
        if let Some(col) = batch.column_by_name("name")
            && let Some(arr) = col.as_any().downcast_ref::<StringArray>()
        {
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    names.push(arr.value(i).to_string());
                }
            }
        }
    }
    Ok(names)
}
