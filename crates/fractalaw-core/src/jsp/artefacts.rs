//! JSP mandated artefact extraction — things that JSPs require to exist.
//!
//! Detects mentions of mandated artefacts (risk assessments, safety cases,
//! permits, inspection records, training records, etc.) in obligation text
//! and classifies them by type.
//!
//! This is regex-based detection and type classification. Detailed property
//! extraction (owner, approver, review frequency, acceptance criterion) is
//! deferred to LLM enrichment in Phase 5.

use regex::Regex;
use std::sync::LazyLock;

/// A mandated artefact detected in an obligation.
#[derive(Debug, Clone)]
pub struct MandatedArtefact {
    /// Artefact type from the taxonomy.
    pub artefact_type: &'static str,
    /// The matched text fragment.
    pub matched_text: String,
    /// Byte offset in the source text.
    pub offset: usize,
}

/// Artefact type patterns — ordered most specific first to avoid
/// generic patterns consuming specific matches.
static ARTEFACT_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // Specific compound types first
        (Regex::new(r"(?i)\brisk\s+assessment").unwrap(), "Risk Assessment"),
        (Regex::new(r"(?i)\bfire\s+risk\s+assessment").unwrap(), "Risk Assessment"),
        (Regex::new(r"(?i)\bDSEAR\s+assessment").unwrap(), "Risk Assessment"),
        (Regex::new(r"(?i)\bsafety\s+case").unwrap(), "Safety Case"),
        (Regex::new(r"(?i)\bhazard\s+log").unwrap(), "Hazard Log"),
        (Regex::new(r"(?i)\bpermit\s+to\s+work").unwrap(), "Permit"),
        (Regex::new(r"(?i)\bemergency\s+plan").unwrap(), "Emergency Plan"),
        (Regex::new(r"(?i)\bemergency\s+(?:arrangements?|procedures?)").unwrap(), "Emergency Plan"),
        (Regex::new(r"(?i)\bmethod\s+statement").unwrap(), "Method Statement"),
        (Regex::new(r"(?i)\bsafe\s+system\s+of\s+work").unwrap(), "Method Statement"),
        // Training / competence
        (Regex::new(r"(?i)\btraining\s+(?:course|record|programme|requirement)").unwrap(), "Training Record"),
        (Regex::new(r"(?i)\bcompetence\s+(?:record|assessment|certificate)").unwrap(), "Training Record"),
        // Inspection / testing
        (Regex::new(r"(?i)\binspection\s+(?:record|report|regime|programme)").unwrap(), "Inspection Report"),
        (Regex::new(r"(?i)\btest(?:ing)?\s+(?:record|report|regime|certificate)").unwrap(), "Inspection Report"),
        (Regex::new(r"(?i)\binspection\s+and\s+test").unwrap(), "Inspection Report"),
        (Regex::new(r"(?i)\bperiodic\s+inspection").unwrap(), "Inspection Report"),
        // Audit
        (Regex::new(r"(?i)\baudit\s+(?:programme|report|record|trail)").unwrap(), "Audit Report"),
        (Regex::new(r"(?i)\bsafety\s+audit").unwrap(), "Audit Report"),
        // Procedures / policies
        (Regex::new(r"(?i)\bsafety\s+procedure").unwrap(), "Procedure"),
        (Regex::new(r"(?i)\breporting\s+procedure").unwrap(), "Procedure"),
        (Regex::new(r"(?i)\bwritten\s+procedure").unwrap(), "Procedure"),
        (Regex::new(r"(?i)\bsafety\s+management\s+(?:system|plan)").unwrap(), "Procedure"),
        // Records
        (Regex::new(r"(?i)\bmaintenance\s+record").unwrap(), "Maintenance Record"),
        (Regex::new(r"(?i)\boccurrence\s+report").unwrap(), "Occurrence Report"),
        (Regex::new(r"(?i)\bincident\s+report").unwrap(), "Occurrence Report"),
        (Regex::new(r"(?i)\bsafety\s+occurrence").unwrap(), "Occurrence Report"),
    ]
});

/// Extract mandated artefacts from obligation text.
///
/// Returns deduplicated artefacts sorted by offset.
/// Each artefact type is returned at most once per text.
pub fn extract_artefacts(text: &str) -> Vec<MandatedArtefact> {
    let mut artefacts = Vec::new();
    let mut seen_types = std::collections::HashSet::new();

    for (re, artefact_type) in ARTEFACT_PATTERNS.iter() {
        if seen_types.contains(artefact_type) {
            continue;
        }
        if let Some(m) = re.find(text) {
            artefacts.push(MandatedArtefact {
                artefact_type,
                matched_text: m.as_str().to_string(),
                offset: m.start(),
            });
            seen_types.insert(artefact_type);
        }
    }

    artefacts.sort_by_key(|a| a.offset);
    artefacts
}

/// All known artefact types in the taxonomy.
pub fn artefact_types() -> &'static [&'static str] {
    &[
        "Risk Assessment",
        "Safety Case",
        "Hazard Log",
        "Permit",
        "Emergency Plan",
        "Method Statement",
        "Training Record",
        "Inspection Report",
        "Audit Report",
        "Procedure",
        "Maintenance Record",
        "Occurrence Report",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_risk_assessment() {
        let arts = extract_artefacts("A risk assessment must be conducted for all electrical hazards.");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].artefact_type, "Risk Assessment");
    }

    #[test]
    fn detects_safety_case() {
        let arts = extract_artefacts("The CO shall maintain a safety case demonstrating ALARP.");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].artefact_type, "Safety Case");
    }

    #[test]
    fn detects_permit_to_work() {
        let arts = extract_artefacts("A permit to work must be obtained before entry.");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].artefact_type, "Permit");
    }

    #[test]
    fn detects_inspection_and_test() {
        let arts = extract_artefacts("Inspection and test must be used to determine serviceability.");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].artefact_type, "Inspection Report");
    }

    #[test]
    fn detects_training_record() {
        let arts = extract_artefacts("Personnel must have completed the required training course.");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].artefact_type, "Training Record");
    }

    #[test]
    fn detects_emergency_arrangements() {
        let arts = extract_artefacts("Emergency arrangements must be in place before work begins.");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].artefact_type, "Emergency Plan");
    }

    #[test]
    fn detects_safety_occurrence() {
        let arts = extract_artefacts("All safety occurrences must be reported and investigated.");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].artefact_type, "Occurrence Report");
    }

    #[test]
    fn detects_multiple_artefacts() {
        let text = "The risk assessment must consider hazards and a permit to work must be obtained.";
        let arts = extract_artefacts(text);
        assert_eq!(arts.len(), 2);
        let types: Vec<_> = arts.iter().map(|a| a.artefact_type).collect();
        assert!(types.contains(&"Risk Assessment"));
        assert!(types.contains(&"Permit"));
    }

    #[test]
    fn deduplicates_same_type() {
        let text = "The risk assessment must be suitable. The risk assessment must also cover fire.";
        let arts = extract_artefacts(text);
        assert_eq!(arts.len(), 1); // only one Risk Assessment per text
    }

    #[test]
    fn no_artefacts_in_plain_text() {
        let arts = extract_artefacts("All equipment must be safely maintained.");
        assert!(arts.is_empty());
    }

    #[test]
    fn detects_dsear_assessment() {
        let arts = extract_artefacts("A DSEAR assessment must be conducted.");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].artefact_type, "Risk Assessment");
    }

    #[test]
    fn detects_safe_system_of_work() {
        let arts = extract_artefacts("A safe system of work must be established.");
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].artefact_type, "Method Statement");
    }
}
