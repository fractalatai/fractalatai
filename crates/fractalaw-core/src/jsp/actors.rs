//! JSP actor extraction — MoD organisational roles.
//!
//! Separate actor dictionary from the legislative pipeline.
//! Loaded from `data/jsp-actor-dictionary.yaml` at compile time.
//!
//! Reuses the [`ActorMatch`] and [`ExtractedActors`] types from
//! [`crate::taxa::actors`] for compatibility, but uses a completely
//! separate compiled dictionary.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::taxa::actors::{ActorMatch, ExtractedActors};

// ── YAML types (same shape as legislative dictionary) ──────────────

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct ActorDef {
    label: String,
    #[serde(rename = "type")]
    actor_type: String,
    #[serde(default)]
    regex_patterns: Vec<String>,
    #[serde(default)]
    triggers: Vec<String>,
    #[serde(default)]
    category: String,
}

struct CompiledDictionary {
    government: Vec<(String, Regex)>,
    governed: Vec<(String, Regex)>,
    all_labels: HashSet<String>,
}

/// The JSP actor dictionary YAML, embedded at compile time.
static JSP_ACTOR_YAML: &str = include_str!("../../data/jsp-actor-dictionary.yaml");

/// The compiled JSP actor dictionary, built on first access.
static DICTIONARY: LazyLock<CompiledDictionary> = LazyLock::new(|| {
    let defs: Vec<ActorDef> =
        serde_yaml::from_str(JSP_ACTOR_YAML).expect("jsp-actor-dictionary.yaml parse error");

    let mut government = Vec::new();
    let mut governed = Vec::new();
    let mut all_labels = HashSet::new();

    for def in &defs {
        all_labels.insert(def.label.clone());

        if def.regex_patterns.is_empty() {
            continue;
        }

        let compiled: Vec<(String, Regex)> = def
            .regex_patterns
            .iter()
            .map(|pat| {
                let full = format!(r"(?:[\s[:punct:]]|^){pat}(?:[\s[:punct:]]|$)");
                let re = Regex::new(&full)
                    .unwrap_or_else(|e| panic!("bad regex for '{}': {e}", def.label));
                (def.label.clone(), re)
            })
            .collect();

        if def.actor_type == "government" {
            government.extend(compiled);
        } else {
            governed.extend(compiled);
        }
    }

    CompiledDictionary {
        government,
        governed,
        all_labels,
    }
});

// ── Public API ──────────────────────────────────────────────────────

/// All valid JSP actor labels.
pub fn all_actor_labels() -> HashSet<String> {
    DICTIONARY.all_labels.clone()
}

/// Extract JSP actors (governed + government) from text.
pub fn extract_actors(text: &str) -> ExtractedActors {
    // Pad text with spaces for boundary matching
    let padded = format!(" {text} ");
    ExtractedActors {
        governed: run_patterns(&padded, &DICTIONARY.governed),
        government: run_patterns(&padded, &DICTIONARY.government),
    }
}

/// Run a set of compiled patterns against text, returning matches.
fn run_patterns(text: &str, patterns: &[(String, Regex)]) -> Vec<ActorMatch> {
    let mut matches = Vec::new();
    let mut seen_labels = HashSet::new();

    for (label, re) in patterns {
        if seen_labels.contains(label) {
            continue;
        }
        if let Some(m) = re.find(text) {
            let keyword = m.as_str().trim().to_lowercase();
            // Adjust offset for the padding space
            let offset = if m.start() > 0 { m.start() - 1 } else { 0 };
            matches.push(ActorMatch {
                label: label.clone(),
                keyword,
                offset,
            });
            seen_labels.insert(label.clone());
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_commanding_officer() {
        let result = extract_actors("The Commanding Officer shall ensure safety.");
        let labels = result.governed_labels();
        assert!(labels.contains(&"MoD: Commanding Officer".to_string()));
    }

    #[test]
    fn extracts_dsa_as_government() {
        let result = extract_actors("The Defence Safety Authority may direct inspections.");
        let labels = result.government_labels();
        assert!(labels.contains(&"MoD: Defence Safety Authority".to_string()));
    }

    #[test]
    fn extracts_multiple_actors() {
        let result = extract_actors(
            "The Senior Duty Holder will delegate to the Operating Duty Holder who will ensure the Contractor complies."
        );
        let governed = result.governed_labels();
        assert!(governed.contains(&"MoD: Senior Duty Holder".to_string()));
        assert!(governed.contains(&"MoD: Operating Duty Holder".to_string()));
        assert!(governed.contains(&"MoD: Contractor".to_string()));
    }

    #[test]
    fn specific_before_generic_duty_holder() {
        // "Senior Duty Holder" should match before generic "Duty Holder"
        let result = extract_actors("The Senior Duty Holder is accountable.");
        let labels = result.governed_labels();
        assert!(labels.contains(&"MoD: Senior Duty Holder".to_string()));
    }

    #[test]
    fn no_actors_in_neutral_text() {
        let result = extract_actors("This chapter provides guidance on safety management.");
        assert!(result.governed.is_empty());
        assert!(result.government.is_empty());
    }
}
