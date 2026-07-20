//! JSP term and acronym extraction.
//!
//! Extracts inline definitions — "Full Name (ACRONYM)" patterns — from
//! JSP provision text. Also detects glossary-style definitions where a
//! term is followed by "means" or "is defined as".

use regex::Regex;
use std::sync::LazyLock;

/// An extracted term or acronym definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JspTerm {
    /// The full term as written.
    pub term: String,
    /// The acronym (if extracted from parenthetical pattern).
    pub acronym: Option<String>,
    /// Normalised form for dedup (lowercased, stripped).
    pub normalised: String,
    /// Byte offset in the source text.
    pub offset: usize,
}

// "Full Name (ACRONYM)" — captures the expansion and the abbreviation.
// Two-step: find "(ACRONYM)" then scan backward for the expansion.
static ACRONYM_PAREN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(([A-Z][A-Z0-9a-z-]{1,15})\)").unwrap()
});

// "term" means / "term" is defined as
static DEFINITION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"['"]([^'"]{3,60})['"][\s]+(?:means|is defined as|refers to)"#).unwrap()
});

/// Extract terms and acronyms from JSP provision text.
pub fn extract_terms(text: &str) -> Vec<JspTerm> {
    let mut terms = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Two-step: find "(ACRONYM)" then scan backward for the expansion
    for caps in ACRONYM_PAREN_RE.captures_iter(text) {
        let paren_match = caps.get(0).unwrap();
        let acronym = caps.get(1).unwrap().as_str();

        // Scan backward from the opening paren to find the expansion.
        // Stop at sentence boundaries, other parentheses, or semicolons.
        let before = &text[..paren_match.start()];
        let before_trimmed = before.trim_end();
        if before_trimmed.is_empty() { continue; }

        // Find the nearest boundary before scanning words
        let boundary = before_trimmed.rfind(|c: char| c == '.' || c == ')' || c == ';' || c == ':')
            .map(|i| i + 1)
            .unwrap_or(0);
        let scan_region = before_trimmed[boundary..].trim();
        if scan_region.is_empty() { continue; }

        // Take the last N words (where N ≈ acronym length + a few) as the expansion.
        // Trim from the left to start at a capitalised word.
        let words: Vec<&str> = scan_region.split_whitespace().collect();
        let take = (acronym.len() + 3).min(words.len());
        let candidate_words = &words[words.len() - take..];

        // Find the first capitalised word to anchor the expansion
        let start_idx = candidate_words.iter().position(|w| {
            w.starts_with(|c: char| c.is_ascii_uppercase())
        });
        let expansion = match start_idx {
            Some(idx) => candidate_words[idx..].join(" "),
            None => continue,
        };

        // Skip very short expansions or common false positives
        if expansion.len() < 5 { continue; }
        if expansion.contains("Chapter ") || expansion.contains("Vol ") || expansion.contains("Volume") { continue; }
        if expansion.starts_with("UN ") { continue; }

        let normalised = acronym.to_lowercase();
        if seen.contains(&normalised) { continue; }
        seen.insert(normalised.clone());

        terms.push(JspTerm {
            term: expansion,
            acronym: Some(acronym.to_string()),
            normalised,
            offset: paren_match.start(),
        });
    }

    // Extract "term" means / is defined as
    for caps in DEFINITION_RE.captures_iter(text) {
        let full_match = caps.get(0).unwrap();
        let term = caps.get(1).unwrap().as_str().trim();
        let normalised = term.to_lowercase();

        if seen.contains(&normalised) { continue; }
        seen.insert(normalised.clone());

        terms.push(JspTerm {
            term: term.to_string(),
            acronym: None,
            normalised,
            offset: full_match.start(),
        });
    }

    terms.sort_by_key(|t| t.offset);
    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_acronym_definition() {
        let terms = extract_terms("Extra Low Voltage (ELV) equipment must be tested.");
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].term, "Extra Low Voltage");
        assert_eq!(terms[0].acronym.as_deref(), Some("ELV"));
    }

    #[test]
    fn extracts_li_ion() {
        let terms = extract_terms("Lithium-Ion (Li-ion) batteries present a fire risk.");
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].acronym.as_deref(), Some("Li-ion"));
    }

    #[test]
    fn extracts_puwer() {
        let terms = extract_terms("the Provision and Use of Work Equipment Regulations 1998 (PUWER)");
        assert!(terms.iter().any(|t| t.acronym.as_deref() == Some("PUWER")));
    }

    #[test]
    fn extracts_dsear() {
        let terms = extract_terms("Dangerous Substances and Explosive Atmospheres Regulations 2002 (DSEAR)");
        assert!(terms.iter().any(|t| t.acronym.as_deref() == Some("DSEAR")));
    }

    #[test]
    fn skips_chapter_references() {
        let terms = extract_terms("JSP 375 Vol 1 Chapter 23 (V1.3 Nov 2024)");
        assert!(terms.is_empty());
    }

    #[test]
    fn deduplicates_same_acronym() {
        let terms = extract_terms("Extra Low Voltage (ELV) and Extra Low Voltage (ELV) again.");
        assert_eq!(terms.len(), 1);
    }

    #[test]
    fn extracts_quoted_definition() {
        let terms = extract_terms("'legislation' means the statutory framework applicable to Defence.");
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].term, "legislation");
        assert!(terms[0].acronym.is_none());
    }

    #[test]
    fn no_terms_in_plain_text() {
        let terms = extract_terms("All equipment must be tested regularly.");
        assert!(terms.is_empty());
    }

    #[test]
    fn extracts_multiple_terms() {
        let text = "Extra Low Voltage (ELV) equipment and Electrical Equipment Testing (EET) records.";
        let terms = extract_terms(text);
        assert_eq!(terms.len(), 2);
    }

    #[test]
    fn real_jsp_provision_referred_to_as() {
        // This text uses "referred to as" not "means" — the definition regex won't match.
        // The acronym regex won't fire either — "(in this chapter..." starts lowercase.
        // This is expected: not all inline definitions are extractable by regex.
        let text = "The key legislation (in this chapter referred to as 'legislation') that applies to electrical safety";
        let terms = extract_terms(text);
        // No extractable terms from this pattern
        assert!(terms.is_empty() || terms.iter().any(|t| t.term.contains("legislation")));
    }

    #[test]
    fn real_jsp_provision_means() {
        // "means" pattern does work
        let text = "In this chapter 'legislation' means the statutory framework applicable to Defence.";
        let terms = extract_terms(text);
        assert!(terms.iter().any(|t| t.term == "legislation"));
    }
}
