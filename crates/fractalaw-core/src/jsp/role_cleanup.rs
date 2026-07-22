//! JSP RACI role cleanup — blacklist, fuzzy map, and normalisation.
//!
//! Loaded from YAML at compile time:
//! - `data/jsp-role-blacklist.yaml` — non-role hallucinations to drop
//! - `data/jsp-role-fuzzy-map.yaml` — non-canonical labels mapped to canonical
//!
//! Applied as a post-filter on SLM RACI output.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

/// Blacklisted role labels — not organisational roles.
static BLACKLIST_YAML: &str = include_str!("../../data/jsp-role-blacklist.yaml");
static BLACKLIST: LazyLock<HashSet<String>> = LazyLock::new(|| {
    serde_yaml::from_str::<Vec<String>>(BLACKLIST_YAML)
        .expect("jsp-role-blacklist.yaml parse error")
        .into_iter()
        .collect()
});

/// Fuzzy map: non-canonical label → canonical label.
static FUZZY_MAP_YAML: &str = include_str!("../../data/jsp-role-fuzzy-map.yaml");
static FUZZY_MAP: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_yaml::from_str::<HashMap<String, String>>(FUZZY_MAP_YAML)
        .expect("jsp-role-fuzzy-map.yaml parse error")
});

/// Check if a role label is blacklisted (not a real role).
pub fn is_blacklisted(role: &str) -> bool {
    BLACKLIST.contains(role)
}

/// Normalise a role label: if it's in the fuzzy map, return the canonical label.
/// If blacklisted, return None. Otherwise return as-is.
pub fn normalise_role(role: &str) -> Option<&str> {
    if BLACKLIST.contains(role) {
        return None;
    }
    if let Some(canonical) = FUZZY_MAP.get(role) {
        Some(canonical.as_str())
    } else {
        Some(role)
    }
}

/// Clean a list of RACI assignments: drop blacklisted, normalise fuzzy.
///
/// Input: vec of (role_label, assignment_type) pairs.
/// Output: cleaned vec with blacklisted removed and fuzzy-matched normalised.
pub fn clean_raci(assignments: &[(String, String)]) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for (role, atype) in assignments {
        match normalise_role(role) {
            None => continue, // blacklisted
            Some(normalised) => {
                let key = (normalised.to_string(), atype.clone());
                if seen.insert(key.clone()) {
                    result.push(key);
                }
            }
        }
    }

    result
}

/// Number of blacklisted labels.
pub fn blacklist_count() -> usize {
    BLACKLIST.len()
}

/// Number of fuzzy map entries.
pub fn fuzzy_map_count() -> usize {
    FUZZY_MAP.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blacklist_loads() {
        assert!(blacklist_count() > 0);
        assert!(is_blacklisted("MoD: Risk Assessment"));
        assert!(is_blacklisted("MoD: Legislation"));
        assert!(!is_blacklisted("MoD: Commanding Officer"));
    }

    #[test]
    fn fuzzy_map_loads() {
        assert!(fuzzy_map_count() > 0);
    }

    #[test]
    fn normalise_blacklisted_returns_none() {
        assert_eq!(normalise_role("MoD: Risk Assessment"), None);
    }

    #[test]
    fn normalise_fuzzy_returns_canonical() {
        assert_eq!(normalise_role("MoD: Employee"), Some("MoD: User"));
        assert_eq!(normalise_role("MoD: Employer"), Some("MoD: Defence Organisation"));
        assert_eq!(normalise_role("MoD: Accountable Officer"), Some("MoD: Accountable Person"));
    }

    #[test]
    fn normalise_canonical_returns_as_is() {
        assert_eq!(normalise_role("MoD: Commanding Officer"), Some("MoD: Commanding Officer"));
        assert_eq!(normalise_role("MoD: Accountable Person"), Some("MoD: Accountable Person"));
    }

    #[test]
    fn clean_raci_drops_blacklisted() {
        let input = vec![
            ("MoD: Commanding Officer".into(), "R".into()),
            ("MoD: Risk Assessment".into(), "R".into()),
            ("MoD: Legislation".into(), "I".into()),
        ];
        let cleaned = clean_raci(&input);
        assert_eq!(cleaned.len(), 1);
        assert_eq!(cleaned[0].0, "MoD: Commanding Officer");
    }

    #[test]
    fn clean_raci_normalises_fuzzy() {
        let input = vec![
            ("MoD: Employee".into(), "R".into()),
            ("MoD: Employer".into(), "R".into()),
        ];
        let cleaned = clean_raci(&input);
        assert_eq!(cleaned.len(), 2);
        assert_eq!(cleaned[0].0, "MoD: User");
        assert_eq!(cleaned[1].0, "MoD: Defence Organisation");
    }

    #[test]
    fn clean_raci_deduplicates_after_normalise() {
        let input = vec![
            ("MoD: Employee".into(), "R".into()),
            ("MoD: Worker".into(), "R".into()),  // both map to User
        ];
        let cleaned = clean_raci(&input);
        assert_eq!(cleaned.len(), 1);
        assert_eq!(cleaned[0].0, "MoD: User");
    }
}
