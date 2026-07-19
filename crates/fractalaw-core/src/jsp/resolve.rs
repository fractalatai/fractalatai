//! Resolution of extracted JSP references to canonical identifiers.
//!
//! Resolves legislation citations to fractalaw `law_name` identifiers
//! and JSP cross-references to sertantai `source_id` format.

use regex::Regex;
use std::sync::LazyLock;

/// A resolved reference — the extracted citation mapped to a canonical identifier.
#[derive(Debug, Clone)]
pub struct ResolvedReference {
    /// The original citation text.
    pub citation: String,
    /// Target type: legislation / jsp / standard / guidance.
    pub target_type: String,
    /// Resolved identifier (law_name for legislation, source_id for JSPs).
    /// None if unresolved.
    pub target_id: Option<String>,
    /// Whether the reference was resolved against a known entity.
    pub resolved: bool,
}

// ── JSP citation normalisation ──────────────────────────────────────

static JSP_CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"JSP\s+(\d{3})\s*(?:,?\s*Vol(?:ume)?\s*(\d+)\s*)?(?:,?\s*Ch(?:apter)?\s*(\d+))?"
    ).unwrap()
});

/// Normalise a JSP citation to the sertantai source_id format.
///
/// "JSP 375 Volume 1, Chapter 23" → "JSP-375-CH23"
/// "JSP 375 Vol 1 Chapter 8" → "JSP-375-CH08"
/// "JSP 850" → "JSP-850" (bare JSP, no chapter)
pub fn normalise_jsp_citation(citation: &str) -> Option<String> {
    let caps = JSP_CITATION_RE.captures(citation)?;
    let jsp_num = caps.get(1)?.as_str();
    // Volume is captured but not used in source_id format (chapters are unique within a JSP)
    let chapter = caps.get(3).map(|m| m.as_str());

    match chapter {
        Some(ch) => {
            let ch_num: u32 = ch.parse().ok()?;
            Some(format!("JSP-{jsp_num}-CH{ch_num:02}"))
        }
        None => Some(format!("JSP-{jsp_num}")),
    }
}

// ── Legislation citation normalisation ──────────────────────────────

static YEAR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(\d{4})\b").unwrap()
});

/// Extract normalised title keywords and year from a legislation citation.
///
/// "the Electricity at Work Regulations 1989" → (["electricity", "work", "regulations"], 1989)
/// "Health and Safety at Work etc. Act 1974" → (["health", "safety", "work", "act"], 1974)
pub fn normalise_legislation_citation(citation: &str) -> Option<(Vec<String>, u32)> {
    let year = YEAR_RE.captures(citation)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())?;

    // Extract significant words (skip stopwords and short words)
    let stopwords = ["the", "of", "and", "at", "in", "to", "for", "etc", "etc."];
    let keywords: Vec<String> = citation
        .split(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')')
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| w.len() > 2 && !stopwords.contains(&w.as_str()) && w.parse::<u32>().is_err())
        .collect();

    if keywords.is_empty() {
        None
    } else {
        Some((keywords, year))
    }
}

/// Score how well a legislation title matches extracted keywords.
///
/// Returns 0.0 to 1.0 — fraction of keywords found in the title.
pub fn match_score(title: &str, keywords: &[String]) -> f32 {
    if keywords.is_empty() {
        return 0.0;
    }
    let lower_title = title.to_lowercase();
    let matched = keywords.iter().filter(|kw| lower_title.contains(kw.as_str())).count();
    matched as f32 / keywords.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_jsp_with_volume_chapter() {
        assert_eq!(
            normalise_jsp_citation("JSP 375 Volume 1, Chapter 23"),
            Some("JSP-375-CH23".to_string())
        );
    }

    #[test]
    fn normalise_jsp_vol_shorthand() {
        assert_eq!(
            normalise_jsp_citation("JSP 375 Vol 1 Chapter 8"),
            Some("JSP-375-CH08".to_string())
        );
    }

    #[test]
    fn normalise_jsp_bare() {
        assert_eq!(
            normalise_jsp_citation("JSP 850"),
            Some("JSP-850".to_string())
        );
    }

    #[test]
    fn normalise_jsp_volume_3_chapter_3() {
        assert_eq!(
            normalise_jsp_citation("JSP 375 Volume 3, Chapter 3"),
            Some("JSP-375-CH03".to_string())
        );
    }

    #[test]
    fn normalise_legislation_electricity() {
        let (kw, year) = normalise_legislation_citation(
            "the Electricity at Work Regulations 1989"
        ).unwrap();
        assert_eq!(year, 1989);
        assert!(kw.contains(&"electricity".to_string()));
        assert!(kw.contains(&"work".to_string()));
        assert!(kw.contains(&"regulations".to_string()));
    }

    #[test]
    fn normalise_legislation_hswa() {
        let (kw, year) = normalise_legislation_citation(
            "Health and Safety at Work etc. Act 1974"
        ).unwrap();
        assert_eq!(year, 1974);
        assert!(kw.contains(&"health".to_string()));
        assert!(kw.contains(&"safety".to_string()));
        assert!(kw.contains(&"act".to_string()));
    }

    #[test]
    fn match_score_exact() {
        let keywords = vec!["electricity".into(), "work".into(), "regulations".into()];
        let score = match_score("Electricity at Work Regulations", &keywords);
        assert_eq!(score, 1.0);
    }

    #[test]
    fn match_score_partial() {
        let keywords = vec!["electricity".into(), "work".into(), "regulations".into()];
        let score = match_score("Electricity Supply Regulations", &keywords);
        assert!(score > 0.5 && score < 1.0);
    }

    #[test]
    fn match_score_no_match() {
        let keywords = vec!["electricity".into(), "work".into(), "regulations".into()];
        let score = match_score("Health and Safety at Work Act", &keywords);
        assert!(score < 0.5);
    }
}
