//! Top-level duty type classifier.
//!
//! Orchestrates the three pattern tiers (government v1 → v2 → governed → unknown)
//! and maps the result to three categories:
//! - **Obligation** — duty/responsibility on any actor (government or governed)
//! - **Liberty** — permission/power granted to any actor
//! - **Rule** — thing-subject obligation (no person-actor)
//!
//! The Duty/Responsibility and Right/Power distinctions are derivable from
//! the actor label (governed vs government) at display time.
//!
//! Ported from `Taxa.DutyType` + `Taxa.DutyTypeLib`.

use super::actors::ActorMatch;
use super::duty_patterns::{self, DutyClassification, DutyFamily, DutySubType};
use super::duty_patterns_offence;
use super::duty_patterns_rule;
use super::duty_patterns_v2;

/// The three duty-type labels (Obligation / Liberty / Rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DutyType {
    Obligation,
    Liberty,
    Rule,
}

impl DutyType {
    /// Display label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Obligation => "Obligation",
            Self::Liberty => "Liberty",
            Self::Rule => "Rule",
        }
    }

    /// Sort priority (lower = higher priority).
    pub fn priority(self) -> u8 {
        match self {
            Self::Obligation => 1,
            Self::Liberty => 2,
            Self::Rule => 3,
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
/// 1. Governed v2: actor-anchored patterns (actor keyword before modal within window)
/// 2. Government v1 (strong patterns — embedded actor keywords)
/// 3. Government v2 (extended patterns — embedded actor keywords)
/// 4. Offence-as-duty (offence-creating language as implicit prohibition)
/// 5. Rule (thing-subject + modal — no person-actor)
/// 6. Falls back to `Unknown` / empty
///
/// The family determines DRRP mapping:
/// - `Government` + obligation → Obligation
/// - `Government` + enabling   → Liberty
/// - `Governed`   + obligation → Obligation
/// - `Governed`   + enabling   → Liberty
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
    // Tier 4: Offence-as-duty (offence-creating language as implicit prohibition)
    if let Some(dc) = duty_patterns_offence::match_offence_as_duty(text) {
        return to_result(dc);
    }
    // Tier 5: Rule (thing-subject + modal — no person-actor)
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
///
/// Both Government and Governed map to the same types now — the
/// Duty/Responsibility and Right/Power distinctions are derivable
/// from the actor label at display time.
fn map_to_duty_type(dc: &DutyClassification) -> Vec<DutyType> {
    match dc.family {
        DutyFamily::Government | DutyFamily::Governed => {
            match dc.sub_type {
                DutySubType::Enabling => vec![DutyType::Liberty],
                DutySubType::Prohibitive => vec![DutyType::Obligation],
                _ => {
                    if duty_patterns::has_enabling(dc.sub_type.as_str_lower()) {
                        vec![DutyType::Liberty]
                    } else {
                        vec![DutyType::Obligation]
                    }
                }
            }
        }
        DutyFamily::Rule => match dc.sub_type {
            DutySubType::Enabling => vec![DutyType::Liberty],
            _ => vec![DutyType::Obligation], // thing-subject obligations — implied duty-holder resolved by classifier/LLM
        },
        DutyFamily::Unknown => Vec::new(),
    }
}

impl DutySubType {
    /// Lowercase label for keyword checks.
    pub(crate) fn as_str_lower(self) -> &'static str {
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

/// Sort duty types by priority (Obligation → Liberty → Rule).
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
    fn classify_employer_obligation() {
        let text = "every employer shall ensure the health, safety and welfare of employees";
        let actors = vec![actor("Org: Employer", "employer")];
        let result = classify(text, &actors, &[]);
        assert_eq!(result.duty_types, vec![DutyType::Obligation]);
    }

    #[test]
    fn classify_government_obligation() {
        let text = "the secretary of state shall have power to make regulations";
        let result = classify(text, &[], &[]);
        assert_eq!(result.duty_types, vec![DutyType::Obligation]);
    }

    #[test]
    fn classify_government_liberty() {
        let text = "the commission may authorise any person to carry out";
        let result = classify(text, &[], &[]);
        assert_eq!(result.duty_types, vec![DutyType::Liberty]);
    }

    #[test]
    fn classify_governed_liberty() {
        let text = "the employee may request a review of the assessment";
        let actors = vec![actor("Ind: Employee", "employee")];
        let result = classify(text, &actors, &[]);
        assert_eq!(result.duty_types, vec![DutyType::Liberty]);
    }

    #[test]
    fn classify_unknown_text() {
        let text = "the quick brown fox jumped over the lazy dog";
        let result = classify(text, &[], &[]);
        assert!(result.duty_types.is_empty());
        assert!(result.classification.is_none());
    }

    #[test]
    fn classify_prohibition_is_obligation() {
        let text = "no person shall carry out work at height unless properly trained";
        let actors = vec![actor("Ind: Person", "person")];
        let result = classify(text, &actors, &[]);
        assert_eq!(result.duty_types, vec![DutyType::Obligation]);
    }

    #[test]
    fn classify_gov_direction_is_liberty() {
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
            DutyType::Liberty,
            DutyType::Obligation,
            DutyType::Obligation,
            DutyType::Rule,
        ];
        sort_duty_types(&mut types);
        assert_eq!(
            types,
            vec![DutyType::Obligation, DutyType::Liberty, DutyType::Rule]
        );
    }

    #[test]
    fn duty_type_as_str() {
        assert_eq!(DutyType::Obligation.as_str(), "Obligation");
        assert_eq!(DutyType::Liberty.as_str(), "Liberty");
        assert_eq!(DutyType::Rule.as_str(), "Rule");
    }

    // ── Offence-as-duty tier ─────────────────────────────────────────

    #[test]
    fn classify_offence_for_person_as_obligation() {
        let text = "it is an offence for a person to fail to comply with a condition subject to which a firearm certificate is held by him.";
        let result = classify(text, &[], &[]);
        assert_eq!(result.duty_types, vec![DutyType::Obligation]);
    }

    #[test]
    fn classify_commits_offence_as_obligation() {
        let text = "a person commits an offence if the person passes any relevant substance from trade premises into a public sewer.";
        let result = classify(text, &[], &[]);
        assert_eq!(result.duty_types, vec![DutyType::Obligation]);
    }

    #[test]
    fn classify_unlawful_for_as_obligation() {
        let text =
            "it shall be unlawful for any person to keep a dog unless he holds a dog licence.";
        let result = classify(text, &[], &[]);
        assert_eq!(result.duty_types, vec![DutyType::Obligation]);
    }

    #[test]
    fn classify_penalty_not_duty() {
        let text = "a person guilty of an offence under this section is liable on summary conviction to a fine not exceeding level 5.";
        let result = classify(text, &[], &[]);
        assert!(
            result.duty_types.is_empty(),
            "penalty provision should not classify, got: {:?}",
            result.duty_types
        );
    }

    #[test]
    fn governed_v2_takes_precedence_over_offence() {
        // If governed v2 matches (actor-anchored), it should win over offence tier
        let text = "every employer shall ensure that no person commits an offence if the workplace is unsafe.";
        let actors = vec![actor("Org: Employer", "employer")];
        let result = classify(text, &actors, &[]);
        assert_eq!(result.duty_types, vec![DutyType::Obligation]);
        // Should match governed v2 (employer shall ensure), not offence tier
        assert_eq!(result.classification.unwrap().family, DutyFamily::Governed);
    }

    #[test]
    fn classify_thing_subject_as_obligation() {
        // Thing-subject obligations (no person-actor) now map to Obligation
        // — implied duty-holder resolved by classifier/LLM tiers
        let text = "every traffic routes must be suitable for the persons using them";
        let result = classify(text, &[], &[]);
        assert_eq!(result.duty_types, vec![DutyType::Obligation]);
    }

    #[test]
    fn person_takes_precedence_over_thing() {
        // "employer" is a governed actor → Governed tier wins over Rule
        let text = "the employer shall ensure that equipment is suitable";
        let actors = vec![actor("Org: Employer", "employer")];
        let result = classify(text, &actors, &[]);
        assert_eq!(result.duty_types, vec![DutyType::Obligation]);
    }

    #[test]
    fn enforcing_authority_may_is_liberty() {
        // "enforcing authority may serve" = enabling/Liberty, not Obligation
        let text = "the enforcing authority may serve on the responsible person a notice \
                     if the authority is of the opinion that the premises constitute a serious risk";
        let actors = vec![
            actor("Gvt: Authority: Enforcement", "enforcing authority"),
            actor("Ind: Responsible Person", "responsible person"),
        ];
        let result = classify(text, &actors, &[]);
        assert_eq!(
            result.duty_types,
            vec![DutyType::Liberty],
            "got {:?} from {:?}",
            result.duty_types,
            result.classification,
        );
    }

    #[test]
    fn enforcing_authority_may_via_parse_v2() {
        // Integration: parse_v2 extracts its own actors — verify Liberty
        let text = "29.—(1) The enforcing authority may serve on the responsible person \
                     a notice (in this Order referred to as \"an alterations notice\") \
                     if the authority is of the opinion that the premises—\n\
                     (a) constitute a serious risk to relevant persons (whether due to \
                     the features of the premises, their use, any hazard present, or any \
                     other circumstances); or\n\
                     (b) may constitute such a risk if a change is made to them or the \
                     use to which they are put.";
        let record = crate::taxa::parse_v2(text, None);
        eprintln!("duty_types: {:?}", record.duty_types);
        eprintln!("governed: {:?}", record.governed_actors);
        eprintln!("government: {:?}", record.government_actors);
        eprintln!("classification: {:?}", record.classification);
        eprintln!("purposes: {:?}", record.purposes);
        assert!(
            record.duty_types.contains(&DutyType::Liberty),
            "expected Liberty, got {:?} from {:?}",
            record.duty_types,
            record.classification,
        );
    }
}
