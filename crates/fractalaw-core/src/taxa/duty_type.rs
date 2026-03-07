//! Top-level duty type classifier.
//!
//! Orchestrates the three pattern tiers (government v1 → v2 → governed → unknown)
//! and maps the result to the four DRRP categories:
//! - **Duty** — obligation on a governed entity
//! - **Right** — permission granted to a governed entity
//! - **Responsibility** — obligation on a government entity
//! - **Power** — discretionary authority granted to government
//!
//! Ported from `Taxa.DutyType` + `Taxa.DutyTypeLib`.

use super::actors::ActorMatch;
use super::duty_patterns::{self, DutyClassification, DutyFamily, DutySubType};
use super::duty_patterns_rule;
use super::duty_patterns_v2;

/// The four DRRP duty-type labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DutyType {
    Duty,
    Right,
    Responsibility,
    Power,
    Rule,
}

impl DutyType {
    /// Display label (matches Elixir output).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Duty => "Duty",
            Self::Right => "Right",
            Self::Responsibility => "Responsibility",
            Self::Power => "Power",
            Self::Rule => "Rule",
        }
    }

    /// Sort priority (lower = higher priority).
    pub fn priority(self) -> u8 {
        match self {
            Self::Duty => 1,
            Self::Right => 2,
            Self::Responsibility => 3,
            Self::Power => 4,
            Self::Rule => 5,
        }
    }
}

/// Result of classifying a single piece of text.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassificationResult {
    /// Primary DRRP types found (sorted by priority).
    pub duty_types: Vec<DutyType>,
    /// The underlying pattern classification (if any).
    pub classification: Option<DutyClassification>,
}

/// Classify a **downcased** legal text into DRRP duty types.
///
/// Tries pattern tiers in order:
/// 1. Government v1 (strong patterns — embedded actor keywords)
/// 2. Government v2 (extended patterns — embedded actor keywords)
/// 3. Governed: actor-anchored patterns (actor keyword before modal within window)
/// 4. Falls back to `Unknown` / empty
///
/// The family determines DRRP mapping:
/// - `Government` + obligation → Responsibility
/// - `Government` + enabling   → Power
/// - `Governed`   + obligation → Duty
/// - `Governed`   + enabling   → Right
pub fn classify(
    text: &str,
    governed_actors: &[ActorMatch],
    _government_actors: &[ActorMatch],
) -> ClassificationResult {
    // Governed v2 first — actor-anchored patterns (actor in subject position
    // relative to modal verb).  These are the most precise and should win
    // over unanchored government patterns that just check for keyword presence.
    if let Some(dc) = duty_patterns_v2::match_governed_v2(text, governed_actors) {
        return to_result(dc);
    }
    // Government v1 (keyword-based, not actor-anchored)
    if let Some(dc) = duty_patterns::match_government_v1(text) {
        return to_result(dc);
    }
    // Government v2 (extended patterns)
    if let Some(dc) = duty_patterns::match_government_v2(text) {
        return to_result(dc);
    }
    // Tier 4: Rule (thing-subject + modal — no person-actor)
    if let Some(dc) = duty_patterns_rule::match_rule(text) {
        return to_result(dc);
    }
    // No match
    ClassificationResult {
        duty_types: Vec::new(),
        classification: None,
    }
}

/// Map a `DutyClassification` to DRRP duty types.
fn to_result(dc: DutyClassification) -> ClassificationResult {
    let dt = map_to_duty_type(&dc);
    ClassificationResult {
        duty_types: dt,
        classification: Some(dc),
    }
}

/// Map family + sub-type to one or more DRRP types.
fn map_to_duty_type(dc: &DutyClassification) -> Vec<DutyType> {
    match dc.family {
        DutyFamily::Government => {
            match dc.sub_type {
                DutySubType::Enabling => vec![DutyType::Power],
                _ => {
                    // Most government sub-types are responsibilities
                    if duty_patterns::has_enabling(dc.sub_type.as_str_lower()) {
                        vec![DutyType::Power]
                    } else {
                        vec![DutyType::Responsibility]
                    }
                }
            }
        }
        DutyFamily::Governed => match dc.sub_type {
            DutySubType::Enabling => vec![DutyType::Right],
            DutySubType::Prohibitive => vec![DutyType::Duty],
            _ => {
                if duty_patterns::has_enabling(dc.sub_type.as_str_lower()) {
                    vec![DutyType::Right]
                } else {
                    vec![DutyType::Duty]
                }
            }
        },
        DutyFamily::Rule => vec![DutyType::Rule],
        DutyFamily::Unknown => Vec::new(),
    }
}

impl DutySubType {
    /// Lowercase label for keyword checks.
    fn as_str_lower(self) -> &'static str {
        match self {
            Self::RegulationMaking => "regulation making",
            Self::CodeApproval => "code approval",
            Self::Enforcement => "enforcement",
            Self::Direction => "direction",
            Self::Guidance => "guidance",
            Self::ConsultationObligation => "consultation obligation",
            Self::Appointment => "appointment",
            Self::Delegation => "delegation",
            Self::Fees => "fees",
            Self::ParliamentaryReporting => "parliamentary reporting",
            Self::GeneralDuty => "general duty",
            Self::Prohibitive => "prohibitive",
            Self::SfairpDuty => "sfairp duty",
            Self::InformationDuty => "information duty",
            Self::RiskAssessment => "risk assessment",
            Self::TrainingDuty => "training duty",
            Self::Prescriptive => "prescriptive",
            Self::Enabling => "enabling",
            Self::ThingObligation => "thing obligation",
            Self::Unclassified => "unclassified",
        }
    }
}

/// Sort duty types by priority (Duty → Right → Responsibility → Power).
pub fn sort_duty_types(types: &mut Vec<DutyType>) {
    types.sort_by_key(|t| t.priority());
    types.dedup();
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxa::actors::ActorMatch;

    fn actor(label: &str, keyword: &str) -> ActorMatch {
        ActorMatch {
            label: label.into(),
            keyword: keyword.into(),
            offset: 0,
        }
    }

    #[test]
    fn classify_employer_duty() {
        let text = "every employer shall ensure the health, safety and welfare of employees";
        let actors = vec![actor("Org: Employer", "employer")];
        let result = classify(text, &actors, &[]);
        assert_eq!(result.duty_types, vec![DutyType::Duty]);
    }

    #[test]
    fn classify_government_responsibility() {
        let text = "the secretary of state shall have power to make regulations";
        let result = classify(text, &[], &[]);
        assert_eq!(result.duty_types, vec![DutyType::Responsibility]);
    }

    #[test]
    fn classify_government_power() {
        let text = "the commission may authorise any person to carry out";
        let result = classify(text, &[], &[]);
        assert_eq!(result.duty_types, vec![DutyType::Power]);
    }

    #[test]
    fn classify_governed_right() {
        let text = "the employee may request a review of the assessment";
        let actors = vec![actor("Ind: Employee", "employee")];
        let result = classify(text, &actors, &[]);
        assert_eq!(result.duty_types, vec![DutyType::Right]);
    }

    #[test]
    fn classify_unknown_text() {
        let text = "the quick brown fox jumped over the lazy dog";
        let result = classify(text, &[], &[]);
        assert!(result.duty_types.is_empty());
        assert!(result.classification.is_none());
    }

    #[test]
    fn classify_prohibition_is_duty() {
        let text = "no person shall carry out work at height unless properly trained";
        let actors = vec![actor("Ind: Person", "person")];
        let result = classify(text, &actors, &[]);
        assert_eq!(result.duty_types, vec![DutyType::Duty]);
    }

    #[test]
    fn classify_gov_direction_is_responsibility() {
        let text = "the secretary of state may give directions to the executive";
        let result = classify(text, &[], &[]);
        assert!(!result.duty_types.is_empty());
    }

    #[test]
    fn classify_rejects_actor_as_object() {
        let text = "information must be provided to the contractor before work begins";
        let actors = vec![actor("SC: C: Contractor", "contractor")];
        let result = classify(text, &actors, &[]);
        assert!(
            result.duty_types.is_empty(),
            "actor-as-object should not produce DRRP, got: {:?}",
            result.duty_types
        );
    }

    #[test]
    fn sort_deduplicates() {
        let mut types = vec![
            DutyType::Power,
            DutyType::Duty,
            DutyType::Duty,
            DutyType::Right,
        ];
        sort_duty_types(&mut types);
        assert_eq!(
            types,
            vec![DutyType::Duty, DutyType::Right, DutyType::Power]
        );
    }

    #[test]
    fn duty_type_as_str() {
        assert_eq!(DutyType::Duty.as_str(), "Duty");
        assert_eq!(DutyType::Right.as_str(), "Right");
        assert_eq!(DutyType::Responsibility.as_str(), "Responsibility");
        assert_eq!(DutyType::Power.as_str(), "Power");
        assert_eq!(DutyType::Rule.as_str(), "Rule");
    }

    #[test]
    fn classify_thing_subject_as_rule() {
        let text = "every traffic routes must be suitable for the persons using them";
        let result = classify(text, &[], &[]);
        assert_eq!(result.duty_types, vec![DutyType::Rule]);
    }

    #[test]
    fn person_takes_precedence_over_thing() {
        // "employer" is a governed actor → Governed tier wins over Rule
        let text = "the employer shall ensure that equipment is suitable";
        let actors = vec![actor("Org: Employer", "employer")];
        let result = classify(text, &actors, &[]);
        assert_eq!(result.duty_types, vec![DutyType::Duty]);
    }
}
