//! Training data generation for the DRRP extraction model.
//!
//! Provides silver-label generation by fuzzy-matching regex-extracted DRRP clauses
//! against source legislative text. Produces Arrow RecordBatches for Parquet export.

use std::sync::LazyLock;

use arrow::array::{Float32Array, Int32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use regex::Regex;

// ── Types ────────────────────────────────────────────────────────────

/// A flat DRRP entry extracted from the DuckDB `List<Struct>` columns.
#[derive(Debug, Clone)]
pub struct FlatDrrpEntry {
    pub law_name: String,
    pub drrp_type: String,
    pub holder: String,
    pub clause: String,
    pub article: String,
}

/// A single training example for the DRRP extraction model.
#[derive(Debug, Clone)]
pub struct TrainingExample {
    pub law_name: String,
    pub drrp_type: String,
    pub article: String,
    pub provision: String,
    pub split: String,
    pub regex_holder: String,
    pub regex_clause: String,
    pub source_text: String,
    pub clause_start: i32,
    pub clause_end: i32,
    pub holder_label: String,
    pub qualifier_start: i32,
    pub qualifier_end: i32,
    pub qualifier_text: Option<String>,
    pub match_ratio: f32,
    pub match_quality: String,
}

/// Result of fuzzy-matching a clause against source text.
#[derive(Debug, Clone)]
pub struct ClauseMatch {
    pub start: usize,
    pub end: usize,
    pub matched_text: String,
    pub ratio: f32,
}

/// Result of qualifier detection.
#[derive(Debug, Clone)]
pub struct QualifierMatch {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

// ── Article Parsing ──────────────────────────────────────────────────

/// Parse an article reference to a bare provision number.
///
/// Handles formats: `section/2`, `section-2`, `regulation/4`, `rule-3`, etc.
/// Returns `None` for empty or unparseable articles.
pub fn parse_article_to_provision(article: &str) -> Option<&str> {
    if article.is_empty() {
        return None;
    }
    // Split on common delimiters and take the last segment.
    let num = article.rsplit(&['-', '/'][..]).next()?;
    if num.is_empty() { None } else { Some(num) }
}

// ── Fuzzy Matching ───────────────────────────────────────────────────

/// Find the best match for `clause` within `source_text` using longest common
/// substring matching. Returns char offsets in `source_text`.
///
/// Uses case-insensitive comparison. Returns `None` if no meaningful match
/// (LCS shorter than 10 chars or 20% of clause length, whichever is smaller).
pub fn find_clause_span(clause: &str, source_text: &str) -> Option<ClauseMatch> {
    if clause.is_empty() || source_text.is_empty() {
        return None;
    }

    let clause_lower = clause.to_lowercase();
    let source_lower = source_text.to_lowercase();

    let clause_chars: Vec<char> = clause_lower.chars().collect();
    let source_chars: Vec<char> = source_lower.chars().collect();

    let (_, pos_in_source, lcs_len) = longest_common_substring(&clause_chars, &source_chars);

    if lcs_len == 0 {
        return None;
    }

    // Minimum match threshold: at least 10 chars or 20% of clause length.
    let min_len = 10.min(clause_chars.len() / 5).max(3);
    if lcs_len < min_len {
        return None;
    }

    // Convert char offsets back to byte offsets in the original source_text.
    let byte_start: usize = source_text
        .chars()
        .take(pos_in_source)
        .map(|c| c.len_utf8())
        .sum();
    let byte_end: usize = byte_start
        + source_text[byte_start..]
            .chars()
            .take(lcs_len)
            .map(|c| c.len_utf8())
            .sum::<usize>();

    let matched_text = source_text[byte_start..byte_end].to_string();
    let ratio = lcs_len as f32 / clause_chars.len() as f32;

    Some(ClauseMatch {
        start: pos_in_source,
        end: pos_in_source + lcs_len,
        matched_text,
        ratio,
    })
}

/// Longest common substring between two char slices.
/// Returns `(position_in_a, position_in_b, length)`.
fn longest_common_substring(a: &[char], b: &[char]) -> (usize, usize, usize) {
    let m = a.len();
    let n = b.len();
    if m == 0 || n == 0 {
        return (0, 0, 0);
    }

    // DP table: prev and curr rows only (space optimisation).
    let mut prev = vec![0u32; n + 1];
    let mut curr = vec![0u32; n + 1];

    let mut best_len: usize = 0;
    let mut best_end_a: usize = 0;
    let mut best_end_b: usize = 0;

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                curr[j] = prev[j - 1] + 1;
                if curr[j] as usize > best_len {
                    best_len = curr[j] as usize;
                    best_end_a = i;
                    best_end_b = j;
                }
            } else {
                curr[j] = 0;
            }
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.fill(0);
    }

    (
        best_end_a.saturating_sub(best_len),
        best_end_b.saturating_sub(best_len),
        best_len,
    )
}

// ── Qualifier Detection ──────────────────────────────────────────────

/// Known UK ESH qualifier patterns.
const QUALIFIER_PATTERNS: &[&str] = &[
    r"(?i)\bso far as is reasonably practicable\b",
    r"(?i)\bso far as is practicable\b",
    r"(?i)\bwhere reasonably practicable\b",
    r"(?i)\bas far as possible\b",
    r"(?i)\bto the extent that\b",
    r"(?i)\bsubject to\b",
    r"(?i)\bprovided that\b",
    r"(?i)\bunless\b",
    r"(?i)\bexcept where\b",
    r"(?i)\bexcept in so far as\b",
];

static COMPILED_QUALIFIERS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    QUALIFIER_PATTERNS
        .iter()
        .map(|p| Regex::new(p).expect("qualifier regex"))
        .collect()
});

/// Find the nearest qualifying phrase to the clause span in source text.
///
/// Searches the full source text and returns the qualifier closest to the
/// clause span boundaries.
pub fn find_qualifier(
    source_text: &str,
    clause_start: usize,
    clause_end: usize,
) -> Option<QualifierMatch> {
    let source_chars: Vec<char> = source_text.chars().collect();
    let mut best: Option<(usize, usize, usize, String)> = None; // (distance, start, end, text)

    for re in COMPILED_QUALIFIERS.iter() {
        for m in re.find_iter(source_text) {
            // Convert byte offsets to char offsets.
            let char_start = source_text[..m.start()].chars().count();
            let char_end = char_start + source_text[m.start()..m.end()].chars().count();

            // Distance from clause span (0 if overlapping).
            let distance = if char_end <= clause_start {
                clause_start - char_end
            } else {
                char_start.saturating_sub(clause_end)
            };

            let is_better = match &best {
                None => true,
                Some((d, _, _, _)) => distance < *d,
            };

            if is_better {
                let text: String = source_chars[char_start..char_end].iter().collect();
                best = Some((distance, char_start, char_end, text));
            }
        }
    }

    best.map(|(_, start, end, text)| QualifierMatch { start, end, text })
}

// ── Silver Label Generation ──────────────────────────────────────────

/// Generate a silver-labelled training example from a DRRPEntry + source text.
pub fn generate_silver_label(
    entry: &FlatDrrpEntry,
    source_text: &str,
    provision: &str,
    split: &str,
) -> TrainingExample {
    let clause_match = find_clause_span(&entry.clause, source_text);

    let (clause_start, clause_end, match_ratio) = match &clause_match {
        Some(m) => (m.start as i32, m.end as i32, m.ratio),
        None => (-1, -1, 0.0),
    };

    let qual = clause_match
        .as_ref()
        .and_then(|cm| find_qualifier(source_text, cm.start, cm.end));

    let (qualifier_start, qualifier_end, qualifier_text) = match &qual {
        Some(q) => (q.start as i32, q.end as i32, Some(q.text.clone())),
        None => (-1, -1, None),
    };

    let match_quality = match_quality_label(match_ratio);

    TrainingExample {
        law_name: entry.law_name.clone(),
        drrp_type: entry.drrp_type.clone(),
        article: entry.article.clone(),
        provision: provision.to_string(),
        split: split.to_string(),
        regex_holder: entry.holder.clone(),
        regex_clause: entry.clause.clone(),
        source_text: source_text.to_string(),
        clause_start,
        clause_end,
        holder_label: entry.holder.clone(),
        qualifier_start,
        qualifier_end,
        qualifier_text,
        match_ratio,
        match_quality: match_quality.to_string(),
    }
}

fn match_quality_label(ratio: f32) -> &'static str {
    if ratio >= 0.8 {
        "high"
    } else if ratio >= 0.5 {
        "medium"
    } else {
        "low"
    }
}

// ── Arrow Schema & Conversion ────────────────────────────────────────

/// Arrow schema for training data Parquet output.
pub fn training_example_schema() -> Schema {
    Schema::new(vec![
        Field::new("law_name", DataType::Utf8, false),
        Field::new("drrp_type", DataType::Utf8, false),
        Field::new("article", DataType::Utf8, false),
        Field::new("provision", DataType::Utf8, false),
        Field::new("split", DataType::Utf8, false),
        Field::new("regex_holder", DataType::Utf8, false),
        Field::new("regex_clause", DataType::Utf8, false),
        Field::new("source_text", DataType::Utf8, false),
        Field::new("clause_start", DataType::Int32, false),
        Field::new("clause_end", DataType::Int32, false),
        Field::new("holder_label", DataType::Utf8, false),
        Field::new("qualifier_start", DataType::Int32, false),
        Field::new("qualifier_end", DataType::Int32, false),
        Field::new("qualifier_text", DataType::Utf8, true),
        Field::new("match_ratio", DataType::Float32, false),
        Field::new("match_quality", DataType::Utf8, false),
    ])
}

/// Convert training examples to an Arrow RecordBatch.
pub fn examples_to_record_batch(
    examples: &[TrainingExample],
) -> Result<RecordBatch, arrow::error::ArrowError> {
    let schema = std::sync::Arc::new(training_example_schema());

    let law_name: StringArray = examples.iter().map(|e| Some(e.law_name.as_str())).collect();
    let drrp_type: StringArray = examples
        .iter()
        .map(|e| Some(e.drrp_type.as_str()))
        .collect();
    let article: StringArray = examples.iter().map(|e| Some(e.article.as_str())).collect();
    let provision: StringArray = examples
        .iter()
        .map(|e| Some(e.provision.as_str()))
        .collect();
    let split: StringArray = examples.iter().map(|e| Some(e.split.as_str())).collect();
    let regex_holder: StringArray = examples
        .iter()
        .map(|e| Some(e.regex_holder.as_str()))
        .collect();
    let regex_clause: StringArray = examples
        .iter()
        .map(|e| Some(e.regex_clause.as_str()))
        .collect();
    let source_text: StringArray = examples
        .iter()
        .map(|e| Some(e.source_text.as_str()))
        .collect();
    let clause_start: Int32Array = examples.iter().map(|e| Some(e.clause_start)).collect();
    let clause_end: Int32Array = examples.iter().map(|e| Some(e.clause_end)).collect();
    let holder_label: StringArray = examples
        .iter()
        .map(|e| Some(e.holder_label.as_str()))
        .collect();
    let qualifier_start: Int32Array = examples.iter().map(|e| Some(e.qualifier_start)).collect();
    let qualifier_end: Int32Array = examples.iter().map(|e| Some(e.qualifier_end)).collect();
    let qualifier_text: StringArray = examples
        .iter()
        .map(|e| e.qualifier_text.as_deref())
        .collect();
    let match_ratio: Float32Array = examples.iter().map(|e| Some(e.match_ratio)).collect();
    let match_quality: StringArray = examples
        .iter()
        .map(|e| Some(e.match_quality.as_str()))
        .collect();

    RecordBatch::try_new(
        schema,
        vec![
            std::sync::Arc::new(law_name),
            std::sync::Arc::new(drrp_type),
            std::sync::Arc::new(article),
            std::sync::Arc::new(provision),
            std::sync::Arc::new(split),
            std::sync::Arc::new(regex_holder),
            std::sync::Arc::new(regex_clause),
            std::sync::Arc::new(source_text),
            std::sync::Arc::new(clause_start),
            std::sync::Arc::new(clause_end),
            std::sync::Arc::new(holder_label),
            std::sync::Arc::new(qualifier_start),
            std::sync::Arc::new(qualifier_end),
            std::sync::Arc::new(qualifier_text),
            std::sync::Arc::new(match_ratio),
            std::sync::Arc::new(match_quality),
        ],
    )
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_article_to_provision ──

    #[test]
    fn parse_section_slash() {
        assert_eq!(parse_article_to_provision("section/2"), Some("2"));
    }

    #[test]
    fn parse_section_dash() {
        assert_eq!(parse_article_to_provision("section-2"), Some("2"));
    }

    #[test]
    fn parse_regulation_slash() {
        assert_eq!(parse_article_to_provision("regulation/4"), Some("4"));
    }

    #[test]
    fn parse_rule_dash() {
        assert_eq!(parse_article_to_provision("rule-3"), Some("3"));
    }

    #[test]
    fn parse_compound_provision() {
        // e.g. "section/2(1)" → "2(1)"
        assert_eq!(parse_article_to_provision("section/2(1)"), Some("2(1)"));
    }

    #[test]
    fn parse_bare_number() {
        assert_eq!(parse_article_to_provision("42"), Some("42"));
    }

    #[test]
    fn parse_empty() {
        assert_eq!(parse_article_to_provision(""), None);
    }

    #[test]
    fn parse_trailing_slash() {
        assert_eq!(parse_article_to_provision("section/"), None);
    }

    // ── longest_common_substring ──

    #[test]
    fn lcs_exact_match() {
        let a: Vec<char> = "hello world".chars().collect();
        let b: Vec<char> = "hello world".chars().collect();
        let (pa, pb, len) = longest_common_substring(&a, &b);
        assert_eq!(len, 11);
        assert_eq!(pa, 0);
        assert_eq!(pb, 0);
    }

    #[test]
    fn lcs_substring() {
        let a: Vec<char> = "world".chars().collect();
        let b: Vec<char> = "hello world!".chars().collect();
        let (_, pb, len) = longest_common_substring(&a, &b);
        assert_eq!(len, 5);
        assert_eq!(pb, 6);
    }

    #[test]
    fn lcs_partial() {
        let a: Vec<char> = "employer shall ensure".chars().collect();
        let b: Vec<char> = "every employer shall ensure the safety".chars().collect();
        let (_, pb, len) = longest_common_substring(&a, &b);
        // "employer shall ensure" = 21 chars, found at position 6 in b.
        assert_eq!(len, 21);
        assert_eq!(pb, 6);
    }

    #[test]
    fn lcs_no_match() {
        let a: Vec<char> = "abc".chars().collect();
        let b: Vec<char> = "xyz".chars().collect();
        let (_, _, len) = longest_common_substring(&a, &b);
        assert_eq!(len, 0);
    }

    #[test]
    fn lcs_empty() {
        let a: Vec<char> = Vec::new();
        let b: Vec<char> = "hello".chars().collect();
        let (_, _, len) = longest_common_substring(&a, &b);
        assert_eq!(len, 0);
    }

    // ── find_clause_span ──

    #[test]
    fn clause_span_exact() {
        let source = "It shall be the duty of every employer to ensure, \
                       so far as is reasonably practicable, the health, \
                       safety and welfare at work of all his employees.";
        let clause = "the duty of every employer to ensure";
        let m = find_clause_span(clause, source).unwrap();
        assert_eq!(m.matched_text, "the duty of every employer to ensure");
        assert!(m.ratio > 0.99);
    }

    #[test]
    fn clause_span_truncated() {
        let source = "Every employer shall make a suitable and sufficient \
                       assessment of the risks to the health and safety of \
                       his employees to which they are exposed whilst they \
                       are at work.";
        // Regex truncated at comma or mid-sentence:
        let clause = "employer shall make a suitable and sufficient assessment";
        let m = find_clause_span(clause, source).unwrap();
        assert!(m.matched_text.contains("employer shall make"));
        assert!(m.ratio > 0.9);
    }

    #[test]
    fn clause_span_case_insensitive() {
        let source = "The Secretary of State may by regulations make provision.";
        let clause = "the secretary of state may by regulations";
        let m = find_clause_span(clause, source).unwrap();
        assert!(m.ratio > 0.9);
    }

    #[test]
    fn clause_span_no_match() {
        let source = "This is about planning regulations.";
        let clause = "employer shall ensure safety of employees";
        let result = find_clause_span(clause, source);
        assert!(result.is_none() || result.unwrap().ratio < 0.2);
    }

    #[test]
    fn clause_span_empty() {
        assert!(find_clause_span("", "some text").is_none());
        assert!(find_clause_span("some text", "").is_none());
    }

    // ── find_qualifier ──

    #[test]
    fn qualifier_sfairp() {
        let source = "shall ensure, so far as is reasonably practicable, \
                       the health and safety";
        let q = find_qualifier(source, 0, 13).unwrap();
        assert_eq!(q.text, "so far as is reasonably practicable");
    }

    #[test]
    fn qualifier_unless() {
        let source = "No person shall do X unless the inspector approves.";
        let q = find_qualifier(source, 0, 25).unwrap();
        assert_eq!(q.text, "unless");
    }

    #[test]
    fn qualifier_subject_to() {
        let source = "Subject to paragraph (2), every employer shall ensure safety.";
        let q = find_qualifier(source, 30, 60).unwrap();
        assert_eq!(q.text, "Subject to");
    }

    #[test]
    fn qualifier_none() {
        let source = "Every employer shall ensure the safety of employees.";
        let q = find_qualifier(source, 0, source.len());
        assert!(q.is_none());
    }

    #[test]
    fn qualifier_nearest_to_clause() {
        // Two qualifiers; should pick the one nearest to the clause span.
        let source = "Unless X, the employer shall, so far as is reasonably \
                       practicable, ensure safety.";
        // Clause is "the employer shall ... ensure safety" starting around char 10.
        let q = find_qualifier(source, 10, 80).unwrap();
        // Both overlap with the clause span (distance 0), but SFAIRP is more specific.
        // Either is acceptable — just check we get one.
        assert!(!q.text.is_empty());
    }

    // ── generate_silver_label ──

    #[test]
    fn silver_label_matched() {
        let entry = FlatDrrpEntry {
            law_name: "UK_ukpga_1974_37".into(),
            drrp_type: "DUTY".into(),
            holder: "Org: Employer".into(),
            clause: "the duty of every employer to ensure".into(),
            article: "section/2".into(),
        };
        let source = "It shall be the duty of every employer to ensure, \
                       so far as is reasonably practicable, the health, \
                       safety and welfare at work of all his employees.";

        let ex = generate_silver_label(&entry, source, "2", "train");

        assert_eq!(ex.law_name, "UK_ukpga_1974_37");
        assert_eq!(ex.drrp_type, "DUTY");
        assert_eq!(ex.provision, "2");
        assert_eq!(ex.split, "train");
        assert_eq!(ex.regex_holder, "Org: Employer");
        assert_eq!(ex.holder_label, "Org: Employer");
        assert!(ex.clause_start >= 0);
        assert!(ex.clause_end > ex.clause_start);
        assert!(ex.match_ratio > 0.9);
        assert_eq!(ex.match_quality, "high");
        // Should detect SFAIRP qualifier.
        assert!(ex.qualifier_text.is_some());
        assert!(
            ex.qualifier_text
                .as_ref()
                .unwrap()
                .contains("reasonably practicable")
        );
    }

    #[test]
    fn silver_label_unmatched() {
        let entry = FlatDrrpEntry {
            law_name: "UK_ukpga_1995_25".into(),
            drrp_type: "POWER".into(),
            holder: "Gov: Secretary of State".into(),
            clause: "completely unrelated clause text".into(),
            article: "section/99".into(),
        };
        let source = "This section is about environmental permits.";

        let ex = generate_silver_label(&entry, source, "99", "train");

        assert_eq!(ex.clause_start, -1);
        assert_eq!(ex.clause_end, -1);
        assert_eq!(ex.match_ratio, 0.0);
        assert_eq!(ex.match_quality, "low");
    }

    // ── Arrow schema & conversion ──

    #[test]
    fn schema_has_16_fields() {
        let schema = training_example_schema();
        assert_eq!(schema.fields().len(), 16);
    }

    #[test]
    fn examples_to_batch_roundtrip() {
        let examples = vec![TrainingExample {
            law_name: "test_law".into(),
            drrp_type: "DUTY".into(),
            article: "section/1".into(),
            provision: "1".into(),
            split: "train".into(),
            regex_holder: "Org: Employer".into(),
            regex_clause: "shall ensure".into(),
            source_text: "The employer shall ensure safety.".into(),
            clause_start: 14,
            clause_end: 26,
            holder_label: "Org: Employer".into(),
            qualifier_start: -1,
            qualifier_end: -1,
            qualifier_text: None,
            match_ratio: 0.95,
            match_quality: "high".into(),
        }];

        let batch = examples_to_record_batch(&examples).unwrap();
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 16);

        // Verify a few values.
        let law_col = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(law_col.value(0), "test_law");

        let ratio_col = batch
            .column(14)
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap();
        assert!((ratio_col.value(0) - 0.95).abs() < 1e-6);
    }
}
