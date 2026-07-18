//! JSP (Joint Service Publication) classification pipeline.
//!
//! Separate from the legislative [`taxa`] pipeline. JSPs assign
//! organisational responsibilities (SRO, ODH, CO, AP) — not
//! Hohfeldian duty/right positions. Modal verb conventions differ:
//! "will" and "is to" are mandatory in JSP context.
//!
//! Shares infrastructure (text cleaning, regex caching) but has its
//! own actor dictionary, its own modal patterns, and its own output type.
//!
//! ## Pipeline stages
//!
//! 1. [`taxa::text_cleaner`] — HTML stripping, whitespace normalisation (shared)
//! 2. [`actors`] — JSP actor extraction from `data/jsp-actor-dictionary.yaml`
//! 3. [`patterns`] — JSP-specific modal verb patterns ("will"/"is to" as mandatory)
//! 4. Clause extraction — "who must do what" snippet
//!
//! ## Usage
//!
//! ```
//! use fractalaw_core::jsp;
//!
//! let text = "The Commanding Officer shall maintain a Safety Case.";
//! let record = jsp::parse(text);
//! assert!(!record.governed_actors.is_empty());
//! ```

pub mod actors;
pub mod patterns;

use crate::taxa::text_cleaner;

/// A classified JSP provision — the output of the JSP pipeline.
///
/// Deliberately separate from [`taxa::TaxaRecord`]. JSP provisions
/// use responsibility-assignment, not Hohfeldian DRRP.
#[derive(Debug, Clone, Default)]
pub struct JspRecord {
    /// Cleaned text that was analysed.
    pub cleaned_text: String,

    /// Governed actors found (duty holders, COs, contractors, etc.).
    pub governed_actors: Vec<String>,

    /// Government/oversight actors found (DSA, ministerial roles, etc.).
    pub government_actors: Vec<String>,

    /// Obligation strength: Mandatory / Recommended / Permissive / None.
    pub strength: Option<&'static str>,

    /// The modal verb that triggered classification.
    pub modal_verb: Option<&'static str>,

    /// DRRP duty types, reusing the legislative taxonomy where applicable.
    /// Obligation (shall/must/will/is to), Permission (may), Recommendation (should).
    pub duty_types: Vec<&'static str>,

    /// Focused clause extract — "who must do what" snippet.
    pub clause_refined: Option<String>,

    /// Confidence score for the extraction (0.0..=1.0).
    pub confidence: f32,
}

/// Run the JSP classification pipeline on provision text.
pub fn parse(raw_text: &str) -> JspRecord {
    if raw_text.trim().is_empty() {
        return JspRecord::default();
    }

    let cleaned = text_cleaner::clean(raw_text);
    let extracted = actors::extract_actors(&cleaned);

    let lower = cleaned.to_lowercase();
    let modal_match = patterns::find_modal(&lower, &extracted);

    let (strength, modal_verb, duty_types) = match &modal_match {
        Some(m) => (Some(m.strength), Some(m.modal_verb), vec![m.duty_type]),
        None => (None, None, vec![]),
    };

    let clause_refined = modal_match
        .as_ref()
        .and_then(|m| extract_clause(&cleaned, m));

    let confidence = if clause_refined.is_some() { 0.7 } else if !duty_types.is_empty() { 0.4 } else { 0.0 };

    JspRecord {
        cleaned_text: cleaned,
        governed_actors: extracted.governed_labels(),
        government_actors: extracted.government_labels(),
        strength,
        modal_verb,
        duty_types,
        clause_refined,
        confidence,
    }
}

/// Extract a focused clause from the cleaned text using a modal match.
fn extract_clause(cleaned_text: &str, modal: &patterns::ModalMatch) -> Option<String> {
    let text_len = cleaned_text.len();
    if modal.span_start >= text_len || modal.span_end > text_len {
        return None;
    }

    // Scan backward from the modal match for sentence start
    let before = &cleaned_text[..modal.span_start];
    let sent_start = before
        .rfind(|c: char| c == '.' || c == ';')
        .map(|i| i + 1)
        .unwrap_or(0);

    // Scan forward from modal end for sentence end (period, semicolon, or text end)
    let after = &cleaned_text[modal.span_end..];
    let sent_end = after
        .find(|c: char| c == '.' || c == ';')
        .map(|i| modal.span_end + i + 1)
        .unwrap_or(text_len);

    let clause = cleaned_text[sent_start..sent_end].trim();
    if clause.len() > 10 {
        Some(clause.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandatory_shall() {
        let r = parse("The Commanding Officer shall ensure all personnel receive safety briefings.");
        assert_eq!(r.strength, Some("Mandatory"));
        assert_eq!(r.modal_verb, Some("shall"));
        assert!(r.governed_actors.contains(&"MoD: Commanding Officer".to_string()));
        assert!(r.clause_refined.is_some());
    }

    #[test]
    fn mandatory_will() {
        let r = parse("The Senior Duty Holder will ensure that safety cases are maintained.");
        assert_eq!(r.strength, Some("Mandatory"));
        assert_eq!(r.modal_verb, Some("will"));
        assert!(r.governed_actors.contains(&"MoD: Senior Duty Holder".to_string()));
    }

    #[test]
    fn mandatory_is_to() {
        let r = parse("The Operating Duty Holder is to review the safety case annually.");
        assert_eq!(r.strength, Some("Mandatory"));
        assert_eq!(r.modal_verb, Some("is to"));
    }

    #[test]
    fn recommendation_should() {
        let r = parse("Units should consider establishing a safety committee.");
        assert_eq!(r.strength, Some("Recommended"));
        assert_eq!(r.modal_verb, Some("should"));
    }

    #[test]
    fn permission_may() {
        let r = parse("The DSA may direct additional safety measures.");
        assert_eq!(r.strength, Some("Permissive"));
        assert_eq!(r.modal_verb, Some("may"));
    }

    #[test]
    fn no_obligation() {
        let r = parse("This chapter provides guidance on safety management.");
        assert!(r.strength.is_none());
        assert!(r.duty_types.is_empty());
    }

    #[test]
    fn empty_text() {
        let r = parse("");
        assert!(r.governed_actors.is_empty());
        assert!(r.duty_types.is_empty());
    }
}
