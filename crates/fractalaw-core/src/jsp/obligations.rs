//! JSP obligation extraction — split provisions into individual obligations
//! and assign RACI roles from narrative text.
//!
//! A single JSP provision may contain multiple obligations, especially in
//! lettered lists ("a. X must... b. Y must..."). This module splits them
//! and assigns RACI types based on actor + modal verb patterns.

use regex::Regex;
use std::sync::LazyLock;

use super::actors;
use super::patterns;

/// A single obligation extracted from a JSP provision.
#[derive(Debug, Clone)]
pub struct Obligation {
    /// Sequential index within the provision (0-based).
    pub index: usize,
    /// The obligation text (a sentence or list item).
    pub text: String,
    /// The modal verb that creates the obligation.
    pub modal_verb: Option<&'static str>,
    /// Obligation strength: Mandatory / Recommended / Permissive.
    pub strength: Option<&'static str>,
    /// "Who must do what" clause extract.
    pub clause_refined: Option<String>,
    /// RACI assignments for this obligation.
    pub raci: Vec<RaciAssignment>,
    /// Competence requirements mentioned (e.g., "competent person", "trained").
    pub competence: Vec<String>,
}

/// A RACI role assignment for an obligation.
#[derive(Debug, Clone)]
pub struct RaciAssignment {
    /// The actor label (from JSP actor dictionary).
    pub role_label: String,
    /// R (Responsible) / A (Accountable) / C (Consulted) / I (Informed).
    pub assignment_type: &'static str,
    /// How the assignment was determined.
    pub source: &'static str,
}

// ── List item splitting ─────────────────────────────────────────────

static LIST_ITEM_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Matches lettered items: "a. ", "b. ", "(a) ", "(1) "
    // Also numbered sub-items: "(1) ", "(2) "
    Regex::new(r"(?:^|\.\s+)(?:([a-z])\.\s|\(([a-z0-9]+)\)\s)").unwrap()
});

/// Split a provision into individual obligation segments.
///
/// If the provision contains lettered/numbered list items, each item
/// becomes a separate segment. Otherwise the whole provision is one segment.
pub fn split_provision(text: &str) -> Vec<String> {
    let matches: Vec<_> = LIST_ITEM_RE.find_iter(text).collect();

    if matches.len() < 2 {
        // No list structure — return the whole provision
        return vec![text.to_string()];
    }

    let mut segments = Vec::new();

    // Text before the first list item (the preamble)
    let first_start = matches[0].start();
    if first_start > 0 {
        let preamble = text[..first_start].trim();
        if !preamble.is_empty() {
            segments.push(preamble.to_string());
        }
    }

    // Each list item runs from its match to the next match (or end of text)
    for i in 0..matches.len() {
        let start = matches[i].start();
        let end = if i + 1 < matches.len() {
            matches[i + 1].start()
        } else {
            text.len()
        };
        let segment = text[start..end].trim();
        if !segment.is_empty() {
            segments.push(segment.to_string());
        }
    }

    segments
}

// ── Competence extraction ───────────────────────────────────────────

static COMPETENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:competent\s+person|competence|competent|qualified|trained|training\s+course|certification|certified)").unwrap()
});

fn extract_competence(text: &str) -> Vec<String> {
    COMPETENCE_RE
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

// ── RACI inference from narrative ───────────────────────────────────

static ACCOUNTABLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:is\s+accountable\s+for|accountable\s+for|has\s+overall\s+responsibility)").unwrap()
});

static CONSULTED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:in\s+consultation\s+with|consult(?:ed)?\s+with|seek(?:ing)?\s+advice\s+from)").unwrap()
});

static INFORMED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:shall\s+be\s+(?:informed|notified|advised)|must\s+be\s+(?:informed|notified)|report(?:ed|ing)?\s+to)").unwrap()
});

/// Infer RACI assignments from obligation text and extracted actors.
fn infer_raci(text: &str, modal: Option<&patterns::ModalMatch>) -> Vec<RaciAssignment> {
    let extracted = actors::extract_actors(text);
    let mut assignments = Vec::new();

    let lower = text.to_lowercase();

    // Check for explicit accountability markers
    if ACCOUNTABLE_RE.is_match(&lower) {
        for actor in &extracted.governed {
            assignments.push(RaciAssignment {
                role_label: actor.label.clone(),
                assignment_type: "A",
                source: "narrative",
            });
        }
        return assignments;
    }

    // Check for consulted markers
    if CONSULTED_RE.is_match(&lower) {
        for actor in &extracted.governed {
            assignments.push(RaciAssignment {
                role_label: actor.label.clone(),
                assignment_type: "C",
                source: "narrative",
            });
        }
        return assignments;
    }

    // Check for informed markers — applies to both governed and government actors
    if INFORMED_RE.is_match(&lower) {
        for actor in extracted.governed.iter().chain(extracted.government.iter()) {
            assignments.push(RaciAssignment {
                role_label: actor.label.clone(),
                assignment_type: "I",
                source: "narrative",
            });
        }
        return assignments;
    }

    // Default: if there's a mandatory modal and a governed actor, they're Responsible
    if let Some(m) = modal {
        if m.strength == "Mandatory" || m.strength == "Recommended" {
            // The actor nearest to the modal verb is Responsible
            if let Some(first) = extracted.governed.first() {
                assignments.push(RaciAssignment {
                    role_label: first.label.clone(),
                    assignment_type: "R",
                    source: "narrative",
                });
            }
            // Additional governed actors are also Responsible (they share the duty)
            for actor in extracted.governed.iter().skip(1) {
                assignments.push(RaciAssignment {
                    role_label: actor.label.clone(),
                    assignment_type: "R",
                    source: "narrative",
                });
            }
        }
    }

    // Government actors mentioned in context are Informed
    for actor in &extracted.government {
        assignments.push(RaciAssignment {
            role_label: actor.label.clone(),
            assignment_type: "I",
            source: "inferred",
        });
    }

    assignments
}

// ── Main extraction ─────────────────────────────────────────────────

/// Extract obligations from a JSP provision.
///
/// Splits the provision into segments (list items), then extracts
/// modal verb, strength, actors, RACI assignments, and competence
/// requirements from each segment.
pub fn extract_obligations(text: &str) -> Vec<Obligation> {
    let segments = split_provision(text);
    let mut obligations = Vec::new();

    for (index, segment) in segments.iter().enumerate() {
        let extracted = actors::extract_actors(segment);
        let lower = segment.to_lowercase();
        let modal = patterns::find_modal(&lower, &extracted);

        let clause_refined = modal.as_ref().and_then(|m| {
            super::extract_clause(segment, m)
        });

        let raci = infer_raci(segment, modal.as_ref());
        let competence = extract_competence(segment);

        let strength = modal.as_ref().map(|m| m.strength);
        let modal_verb = modal.as_ref().map(|m| m.modal_verb);

        // Only include segments that have an obligation or actor
        if modal.is_some() || !raci.is_empty() {
            obligations.push(Obligation {
                index,
                text: segment.clone(),
                modal_verb,
                strength,
                clause_refined,
                raci,
                competence,
            });
        }
    }

    obligations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lettered_list() {
        let text = "50. Key issues: a. X must do thing. b. Y must do other thing. c. Z must comply.";
        let segments = split_provision(text);
        assert!(segments.len() >= 3, "expected at least 3 segments, got {}", segments.len());
    }

    #[test]
    fn single_obligation_no_split() {
        let text = "The Commanding Officer shall ensure safety.";
        let segments = split_provision(text);
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn extracts_single_obligation() {
        let obs = extract_obligations("The Commanding Officer must ensure all equipment is tested.");
        assert_eq!(obs.len(), 1);
        assert_eq!(obs[0].strength, Some("Mandatory"));
        assert_eq!(obs[0].modal_verb, Some("must"));
        assert!(obs[0].raci.iter().any(|r| r.role_label == "MoD: Commanding Officer" && r.assignment_type == "R"));
    }

    #[test]
    fn extracts_multiple_obligations_from_list() {
        let text = "50. Key issues: a. Operators must understand risks. b. Maintainers must comply with safety info. c. Users must complete training.";
        let obs = extract_obligations(text);
        assert!(obs.len() >= 2, "expected at least 2 obligations, got {}", obs.len());
        for ob in &obs {
            assert_eq!(ob.strength, Some("Mandatory"));
        }
    }

    #[test]
    fn detects_competence_requirement() {
        let obs = extract_obligations("Testing must only be performed by a competent person.");
        assert_eq!(obs.len(), 1);
        assert!(!obs[0].competence.is_empty());
    }

    #[test]
    fn accountable_maps_to_raci_a() {
        let obs = extract_obligations("The Senior Duty Holder is accountable for safety.");
        assert_eq!(obs.len(), 1);
        assert!(obs[0].raci.iter().any(|r| r.assignment_type == "A"));
    }

    #[test]
    fn informed_maps_to_raci_i() {
        let obs = extract_obligations("The DSA shall be informed of all incidents.");
        assert_eq!(obs.len(), 1);
        assert!(obs[0].raci.iter().any(|r| r.assignment_type == "I"));
    }

    #[test]
    fn recommendation_extracts() {
        let obs = extract_obligations("Units should consider establishing safety committees.");
        assert_eq!(obs.len(), 1);
        assert_eq!(obs[0].strength, Some("Recommended"));
    }

    #[test]
    fn no_obligation_in_descriptive_text() {
        let obs = extract_obligations("This chapter provides guidance on electrical safety.");
        assert!(obs.is_empty());
    }

    #[test]
    fn real_jsp_provision_para_50() {
        let text = "50. The key safety issues to consider with the use of EVs and PTs are as follows: \
            a. Individuals using or managing the use of EVs and PTs must understand their associated risks and to make sure those risks are reduced to ALARP. \
            b. Operators and maintainers of EVs and PTs must comply with any safety information provided by the Defence organisation or the vehicle manufacturer. \
            c. Operators and maintainers of EVs and PTs must have completed (and remain current in) any training courses deemed necessary by their Defence organisation.";
        let obs = extract_obligations(text);
        assert!(obs.len() >= 3, "expected at least 3 obligations from para.50, got {}", obs.len());
    }
}
