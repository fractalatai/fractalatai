//! JSP-specific modal verb patterns and obligation strength classification.
//!
//! JSPs use different modal conventions from legislation:
//! - "will" is mandatory (not future tense)
//! - "is to" is mandatory (not used in legislation)
//! - "shall" and "must" are mandatory (same as legislation)
//! - "should" is a recommendation (not used in legislation)
//! - "may" is permissive (same as legislation)

use regex::Regex;
use std::sync::LazyLock;

use crate::taxa::actors::ExtractedActors;

/// A modal verb match found in JSP text.
#[derive(Debug, Clone)]
pub struct ModalMatch {
    /// The modal verb as a static string.
    pub modal_verb: &'static str,
    /// Obligation strength classification.
    pub strength: &'static str,
    /// DRRP duty type label.
    pub duty_type: &'static str,
    /// Byte offset of the start of the match context in lowercase text.
    pub span_start: usize,
    /// Byte offset of the end of the modal verb in lowercase text.
    pub span_end: usize,
}

/// Ordered modal patterns — checked in priority order.
/// Each tuple: (regex pattern, modal_verb label, strength, duty_type)
static MODAL_PATTERNS: LazyLock<Vec<(Regex, &'static str, &'static str, &'static str)>> =
    LazyLock::new(|| {
        vec![
            // Negative mandatory — check before positive to avoid "shall not" → "shall"
            (
                Regex::new(r"(?:shall\s+not|must\s+not|may\s+not|will\s+not)\b").unwrap(),
                "shall not",
                "Mandatory",
                "Obligation",
            ),
            // Mandatory modals
            (
                Regex::new(r"\bshall\b").unwrap(),
                "shall",
                "Mandatory",
                "Obligation",
            ),
            (
                Regex::new(r"\bmust\b").unwrap(),
                "must",
                "Mandatory",
                "Obligation",
            ),
            // JSP-specific: "will" as mandatory (NOT future tense)
            (
                Regex::new(r"\bwill\b").unwrap(),
                "will",
                "Mandatory",
                "Obligation",
            ),
            // JSP-specific: "is to" / "are to" as mandatory
            (
                Regex::new(r"\b(?:is|are)\s+to\b").unwrap(),
                "is to",
                "Mandatory",
                "Obligation",
            ),
            // "is required to" / "is responsible for"
            (
                Regex::new(r"\b(?:is|are)\s+(?:required|responsible)\b").unwrap(),
                "is required to",
                "Mandatory",
                "Obligation",
            ),
            // Recommendation
            (
                Regex::new(r"\bshould\b").unwrap(),
                "should",
                "Recommended",
                "Recommendation",
            ),
            // Permissive
            (
                Regex::new(r"\bmay\b").unwrap(),
                "may",
                "Permissive",
                "Permission",
            ),
        ]
    });

/// Find the highest-priority modal verb in JSP text.
///
/// Scans the lowercase text for modal verbs in priority order.
/// Returns the first match found. If a governed actor is present
/// near the modal, the match is higher confidence.
pub fn find_modal(lower_text: &str, _extracted: &ExtractedActors) -> Option<ModalMatch> {
    for (re, modal_verb, strength, duty_type) in MODAL_PATTERNS.iter() {
        if let Some(m) = re.find(lower_text) {
            return Some(ModalMatch {
                modal_verb,
                strength,
                duty_type,
                span_start: m.start(),
                span_end: m.end(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxa::actors::ExtractedActors;

    fn empty_actors() -> ExtractedActors {
        ExtractedActors::default()
    }

    #[test]
    fn shall_is_mandatory() {
        let m = find_modal("the co shall ensure safety", &empty_actors()).unwrap();
        assert_eq!(m.modal_verb, "shall");
        assert_eq!(m.strength, "Mandatory");
    }

    #[test]
    fn will_is_mandatory_in_jsp() {
        let m = find_modal("the sdh will ensure compliance", &empty_actors()).unwrap();
        assert_eq!(m.modal_verb, "will");
        assert_eq!(m.strength, "Mandatory");
    }

    #[test]
    fn is_to_is_mandatory() {
        let m = find_modal("the odh is to review the safety case", &empty_actors()).unwrap();
        assert_eq!(m.modal_verb, "is to");
        assert_eq!(m.strength, "Mandatory");
    }

    #[test]
    fn should_is_recommended() {
        let m = find_modal("units should consider alternatives", &empty_actors()).unwrap();
        assert_eq!(m.modal_verb, "should");
        assert_eq!(m.strength, "Recommended");
    }

    #[test]
    fn may_is_permissive() {
        let m = find_modal("the dsa may direct inspections", &empty_actors()).unwrap();
        assert_eq!(m.modal_verb, "may");
        assert_eq!(m.strength, "Permissive");
    }

    #[test]
    fn shall_not_before_shall() {
        let m = find_modal("personnel shall not enter without authorisation", &empty_actors()).unwrap();
        assert_eq!(m.modal_verb, "shall not");
        assert_eq!(m.strength, "Mandatory");
    }

    #[test]
    fn no_modal_returns_none() {
        let m = find_modal("this chapter provides guidance on safety management", &empty_actors());
        assert!(m.is_none());
    }

    #[test]
    fn is_required_to() {
        let m = find_modal("the co is required to maintain records", &empty_actors()).unwrap();
        assert_eq!(m.modal_verb, "is required to");
        assert_eq!(m.strength, "Mandatory");
    }
}
