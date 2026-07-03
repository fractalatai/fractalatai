//! Correlative actor inference rules.
//!
//! When regex finds Actor A with a specific position, infer Actor B
//! with a correlative position (Hohfeldian correlatives).

use std::collections::HashMap;

static RULES_YAML: &str = include_str!("../../data/correlative-rules.yaml");

/// A correlative inference rule.
#[derive(Debug, serde::Deserialize)]
pub struct CorrelativeRule {
    pub trigger_actor: String,
    pub trigger_position: String,
    #[serde(default)]
    pub trigger_drrp: Option<String>,
    pub inferred_actor: String,
    pub inferred_category: String,
    pub inferred_position: String,
    #[serde(default)]
    pub inferred_drrp: Option<String>,
}

/// An inferred actor ready for upsert.
#[derive(Debug)]
pub struct InferredActor {
    pub section_id: String,
    pub actor_label: String,
    pub actor_category: String,
    pub drrp: Option<String>,
    pub position: String,
}

/// Load correlative rules from the embedded YAML.
pub fn load_rules() -> Vec<CorrelativeRule> {
    serde_yaml::from_str(RULES_YAML).expect("failed to parse correlative-rules.yaml")
}

/// Apply correlative rules to a set of actors grouped by section_id.
///
/// For each section, checks if any existing actor matches a rule trigger.
/// If so, and if the inferred actor doesn't already exist in that section,
/// produces an `InferredActor`.
///
/// Only regex-tier actors are eligible triggers (no cascading).
pub fn apply_rules(
    rules: &[CorrelativeRule],
    actors_by_section: &HashMap<String, Vec<(String, String, Option<String>, Option<String>)>>,
    // section_id → Vec<(actor_label, actor_category, regex_drrp, regex_position)>
) -> Vec<InferredActor> {
    let mut inferred = Vec::new();

    for (section_id, actors) in actors_by_section {
        // Collect existing labels in this section for duplicate check
        let existing_labels: std::collections::HashSet<&str> =
            actors.iter().map(|(label, _, _, _)| label.as_str()).collect();

        for rule in rules {
            // Check if any actor in this section matches the trigger
            let triggered = actors.iter().any(|(label, _cat, drrp, pos)| {
                label == &rule.trigger_actor
                    && pos.as_deref() == Some(rule.trigger_position.as_str())
                    && match &rule.trigger_drrp {
                        Some(td) => drrp.as_deref() == Some(td.as_str()),
                        None => true,
                    }
            });

            if !triggered {
                continue;
            }

            // Don't infer if the actor already exists in this section
            if existing_labels.contains(rule.inferred_actor.as_str()) {
                continue;
            }

            // Inherit DRRP from trigger if not specified in rule
            let drrp = rule.inferred_drrp.clone().or_else(|| {
                actors
                    .iter()
                    .find(|(label, _, _, _)| label == &rule.trigger_actor)
                    .and_then(|(_, _, drrp, _)| drrp.clone())
            });

            inferred.push(InferredActor {
                section_id: section_id.clone(),
                actor_label: rule.inferred_actor.clone(),
                actor_category: rule.inferred_category.clone(),
                drrp,
                position: rule.inferred_position.clone(),
            });
        }
    }

    inferred
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_rules_parses() {
        let rules = load_rules();
        assert!(rules.len() >= 3);
        assert_eq!(rules[0].trigger_actor, "Ind: Employee");
        assert_eq!(rules[0].inferred_actor, "Org: Employer");
    }

    #[test]
    fn apply_employee_employer_rule() {
        let rules = load_rules();
        let mut sections = HashMap::new();
        sections.insert(
            "test:s.1".to_string(),
            vec![(
                "Ind: Employee".to_string(),
                "Ind".to_string(),
                Some("Obligation".to_string()),
                Some("active".to_string()),
            )],
        );

        let inferred = apply_rules(&rules, &sections);
        assert_eq!(inferred.len(), 1);
        assert_eq!(inferred[0].actor_label, "Org: Employer");
        assert_eq!(inferred[0].position, "counterparty");
        assert_eq!(inferred[0].drrp, Some("Obligation".to_string()));
    }

    #[test]
    fn no_duplicate_inference() {
        let rules = load_rules();
        let mut sections = HashMap::new();
        sections.insert(
            "test:s.1".to_string(),
            vec![
                (
                    "Ind: Employee".to_string(),
                    "Ind".to_string(),
                    Some("Obligation".to_string()),
                    Some("active".to_string()),
                ),
                (
                    "Org: Employer".to_string(),
                    "Org".to_string(),
                    Some("Obligation".to_string()),
                    Some("counterparty".to_string()),
                ),
            ],
        );

        let inferred = apply_rules(&rules, &sections);
        assert_eq!(inferred.len(), 0, "should not infer actor that already exists");
    }
}
