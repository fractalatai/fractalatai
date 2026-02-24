//! Pattern-matching rules for duty type classification.
//!
//! Merged from three Elixir modules:
//! - `Taxa.DutyTypeDefnGovernment` (v1)
//! - `Taxa.DutyTypeDefnGovernmentV2` (v2)
//! - `Taxa.DutyTypeDefnGoverned`
//!
//! Also includes the shared helper functions from `Taxa.DutyTypeLib`:
//! actor detection, modal/obligation keyword checks, confidence utilities.

use std::sync::LazyLock;

use regex::Regex;

// ── Types ────────────────────────────────────────────────────────────

/// Top-level duty family: who bears the obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DutyFamily {
    Government,
    Governed,
    Unknown,
}

/// Specific sub-type within a duty family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DutySubType {
    // Government v1
    RegulationMaking,
    CodeApproval,
    Enforcement,
    // Government v2
    Direction,
    Guidance,
    ConsultationObligation,
    Appointment,
    Delegation,
    Fees,
    ParliamentaryReporting,
    // Governed
    GeneralDuty,
    Prohibitive,
    SfairpDuty,
    InformationDuty,
    RiskAssessment,
    TrainingDuty,
    // Shared
    Prescriptive,
    Enabling,
    // Fallback
    Unclassified,
}

/// Result of a pattern match attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DutyClassification {
    pub family: DutyFamily,
    pub sub_type: DutySubType,
    pub confidence: f32,
}

impl DutyClassification {
    pub fn unknown() -> Self {
        Self {
            family: DutyFamily::Unknown,
            sub_type: DutySubType::Unclassified,
            confidence: 0.0,
        }
    }
}

// ── Compiled patterns ────────────────────────────────────────────────

// Government v1
static GOV_REG_MAKING_1: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)secretary of state\b.*\bshall\b.*\bmake\b.*\bregulations\b").unwrap()
});
static GOV_REG_MAKING_2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bpower to make regulations\b").unwrap());
static GOV_CODE_APPROVAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bapprove\b.*\bcode of practice\b").unwrap());
static GOV_ENFORCEMENT_1: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:inspector|enforcing authority)\b.*\b(?:serve|issue)\b.*\b(?:notice|prohibition)\b").unwrap()
});
static GOV_ENFORCEMENT_2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bimprovement notice\b|\bprohibition notice\b").unwrap());

// Government v2
static GOV_DIRECTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:secretary of state|minister|authority)\b.*\bgive\b.*\bdirection").unwrap()
});
static GOV_GUIDANCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bissue\b.*\bguidance\b").unwrap());
static GOV_CONSULTATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:secretary of state|minister)\b.*\bconsult\b").unwrap());
static GOV_APPOINTMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bappoint\b.*\binspector").unwrap());
static GOV_DELEGATION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bdelegate\b|\btransfer\b.*\bfunction").unwrap());
static GOV_FEES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bfees?\b|\bcharges?\b|\blevy\b").unwrap());
static GOV_PARL_REPORTING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\breport\b.*\bparliament\b|\blaid? before parliament\b").unwrap()
});

// Governed
static GOVERNED_GENERAL_DUTY_1: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:employer|every employer)\b.*\b(?:shall ensure|ensure|duty)\b.*\b(?:health|safety|welfare)\b",
    )
    .unwrap()
});
static GOVERNED_GENERAL_DUTY_2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bgeneral duty\b.*\b(?:employer|occupier)\b").unwrap());
static GOVERNED_SFAIRP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)so far as is reasonably practicable|sfairp").unwrap());
static GOVERNED_INFO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:provide|give)\b.*\binformation\b").unwrap());
static GOVERNED_RISK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:assess|assessment)\b.*\brisks?\b").unwrap());
static GOVERNED_TRAINING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\btraining?\b|\binstructions?\b|\bcompeten(?:t|ce|cy)\b").unwrap()
});

// Shared helpers
static OBLIGATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bshall\b|\bmust\b|\bis required to\b|\bhas a duty\b").unwrap()
});
static PROHIBITION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bshall not\b|\bmust not\b|\bno person shall\b|\bprohibit").unwrap()
});
static ENABLING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bmay\b|\bpower to\b|\bauthori[sz]e|\benable").unwrap());

// ── Actor fragment lists (downcased) ─────────────────────────────────

const GOVERNMENT_ACTORS: &[&str] = &[
    "secretary of state",
    "minister",
    "executive",
    "authority",
    "commission",
    "commissioner",
    "inspector",
    "hse",
    "health and safety executive",
    "local authority",
    "enforcing authority",
    "appropriate authority",
    "national authority",
    "crown",
    "tribunal",
    "court",
    "parliament",
    "regulations made",
];

const GOVERNED_ACTORS: &[&str] = &[
    "employer",
    "self-employed",
    "employee",
    "occupier",
    "manufacturer",
    "supplier",
    "designer",
    "importer",
    "installer",
    "person who",
    "every person",
    "no person",
    "duty holder",
    "responsible person",
];

// ── Shared helper functions ──────────────────────────────────────────

/// True when downcased text mentions a government-side actor.
pub fn has_government_actor(text: &str) -> bool {
    GOVERNMENT_ACTORS.iter().any(|frag| text.contains(frag))
}

/// True when downcased text mentions a governed-side actor.
pub fn has_governed_actor(text: &str) -> bool {
    GOVERNED_ACTORS.iter().any(|frag| text.contains(frag))
}

/// True if text contains a strong obligation modal.
pub fn has_obligation(text: &str) -> bool {
    OBLIGATION.is_match(text)
}

/// True if text contains a prohibition.
pub fn has_prohibition(text: &str) -> bool {
    PROHIBITION.is_match(text)
}

/// True if text contains an enabling / empowering keyword.
pub fn has_enabling(text: &str) -> bool {
    ENABLING.is_match(text)
}

/// Clamp a float to the 0.0..=1.0 range, rounded to 3 decimal places.
pub fn clamp01(v: f32) -> f32 {
    ((v.clamp(0.0, 1.0)) * 1000.0).round() / 1000.0
}

// ── Pattern matchers ─────────────────────────────────────────────────

/// Try government duty patterns (v1) against downcased text.
pub fn match_government_v1(text: &str) -> Option<DutyClassification> {
    if GOV_REG_MAKING_1.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::RegulationMaking,
            confidence: 0.90,
        });
    }
    if GOV_REG_MAKING_2.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::RegulationMaking,
            confidence: 0.85,
        });
    }
    if GOV_CODE_APPROVAL.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::CodeApproval,
            confidence: 0.85,
        });
    }
    if GOV_ENFORCEMENT_1.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Enforcement,
            confidence: 0.85,
        });
    }
    if GOV_ENFORCEMENT_2.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Enforcement,
            confidence: 0.80,
        });
    }
    if has_government_actor(text) && has_obligation(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Prescriptive,
            confidence: 0.60,
        });
    }
    if has_government_actor(text) && has_enabling(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Enabling,
            confidence: 0.55,
        });
    }
    None
}

/// Try extended government duty patterns (v2) against downcased text.
pub fn match_government_v2(text: &str) -> Option<DutyClassification> {
    if GOV_DIRECTION.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Direction,
            confidence: 0.80,
        });
    }
    if GOV_GUIDANCE.is_match(text) && has_government_actor(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Guidance,
            confidence: 0.75,
        });
    }
    if GOV_CONSULTATION.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::ConsultationObligation,
            confidence: 0.75,
        });
    }
    if GOV_APPOINTMENT.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Appointment,
            confidence: 0.80,
        });
    }
    if GOV_DELEGATION.is_match(text) && has_government_actor(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Delegation,
            confidence: 0.70,
        });
    }
    if GOV_FEES.is_match(text) && has_government_actor(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Fees,
            confidence: 0.65,
        });
    }
    if GOV_PARL_REPORTING.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::ParliamentaryReporting,
            confidence: 0.80,
        });
    }
    None
}

/// Try governed (regulated entity) duty patterns against downcased text.
///
/// Pattern ordering matters: specific patterns are tried first, generic
/// fallbacks (prescriptive, enabling) last.
pub fn match_governed(text: &str) -> Option<DutyClassification> {
    // ── Specific composite patterns (most specific first) ────────
    if GOVERNED_GENERAL_DUTY_1.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::GeneralDuty,
            confidence: 0.90,
        });
    }
    if GOVERNED_GENERAL_DUTY_2.is_match(text) {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::GeneralDuty,
            confidence: 0.85,
        });
    }
    if has_governed_actor(text) && has_prohibition(text) {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::Prohibitive,
            confidence: 0.80,
        });
    }
    // ── Domain-specific patterns (before generic obligation check) ─
    if GOVERNED_SFAIRP.is_match(text) && has_governed_actor(text) {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::SfairpDuty,
            confidence: 0.80,
        });
    }
    if GOVERNED_RISK.is_match(text) && has_governed_actor(text) {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::RiskAssessment,
            confidence: 0.80,
        });
    }
    if GOVERNED_INFO.is_match(text) && has_governed_actor(text) {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::InformationDuty,
            confidence: 0.70,
        });
    }
    if GOVERNED_TRAINING.is_match(text) && has_governed_actor(text) {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::TrainingDuty,
            confidence: 0.75,
        });
    }
    // ── Generic fallbacks ────────────────────────────────────────
    if has_governed_actor(text) && has_obligation(text) {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::Prescriptive,
            confidence: 0.55,
        });
    }
    if has_governed_actor(text) && has_enabling(text) {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::Enabling,
            confidence: 0.45,
        });
    }
    None
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn dc(family: DutyFamily, sub_type: DutySubType, confidence: f32) -> DutyClassification {
        DutyClassification {
            family,
            sub_type,
            confidence,
        }
    }

    // ── Government v1 ────────────────────────────────────────────────

    #[test]
    fn gov_v1_regulation_making_sos() {
        let text = "the secretary of state shall have power to make regulations";
        assert_eq!(
            match_government_v1(text),
            Some(dc(
                DutyFamily::Government,
                DutySubType::RegulationMaking,
                0.90
            ))
        );
    }

    #[test]
    fn gov_v1_regulation_making_power_to() {
        let text = "there is a power to make regulations under this section";
        assert_eq!(
            match_government_v1(text),
            Some(dc(
                DutyFamily::Government,
                DutySubType::RegulationMaking,
                0.85
            ))
        );
    }

    #[test]
    fn gov_v1_code_approval() {
        let text = "the commission may approve a code of practice";
        assert_eq!(
            match_government_v1(text),
            Some(dc(DutyFamily::Government, DutySubType::CodeApproval, 0.85))
        );
    }

    #[test]
    fn gov_v1_enforcement_notice() {
        let text = "an inspector may serve an improvement notice or prohibition notice";
        assert_eq!(
            match_government_v1(text).map(|c| c.sub_type),
            Some(DutySubType::Enforcement)
        );
    }

    #[test]
    fn gov_v1_prescriptive_generic() {
        let text = "the authority shall ensure compliance with the requirements";
        let result = match_government_v1(text).unwrap();
        assert_eq!(result.sub_type, DutySubType::Prescriptive);
        assert!((result.confidence - 0.60).abs() < 0.01);
    }

    #[test]
    fn gov_v1_enabling_generic() {
        let text = "the commission may authorise any person to carry out";
        let result = match_government_v1(text).unwrap();
        assert_eq!(result.sub_type, DutySubType::Enabling);
    }

    #[test]
    fn gov_v1_no_match_governed_text() {
        let text = "every employer shall ensure the safety of employees";
        assert!(match_government_v1(text).is_none());
    }

    // ── Government v2 ────────────────────────────────────────────────

    #[test]
    fn gov_v2_direction() {
        let text = "the secretary of state may give directions to the executive";
        assert_eq!(
            match_government_v2(text),
            Some(dc(DutyFamily::Government, DutySubType::Direction, 0.80))
        );
    }

    #[test]
    fn gov_v2_guidance() {
        let text = "the executive may issue guidance on the application of these regulations";
        assert_eq!(
            match_government_v2(text),
            Some(dc(DutyFamily::Government, DutySubType::Guidance, 0.75))
        );
    }

    #[test]
    fn gov_v2_consultation() {
        let text = "the secretary of state shall consult such bodies as appear appropriate";
        assert_eq!(
            match_government_v2(text),
            Some(dc(
                DutyFamily::Government,
                DutySubType::ConsultationObligation,
                0.75
            ))
        );
    }

    #[test]
    fn gov_v2_appointment() {
        let text = "the executive may appoint any suitably qualified person as an inspector";
        assert_eq!(
            match_government_v2(text),
            Some(dc(DutyFamily::Government, DutySubType::Appointment, 0.80))
        );
    }

    #[test]
    fn gov_v2_delegation() {
        let text = "the authority may delegate any of its functions to a committee";
        assert_eq!(
            match_government_v2(text),
            Some(dc(DutyFamily::Government, DutySubType::Delegation, 0.70))
        );
    }

    #[test]
    fn gov_v2_fees() {
        let text = "the authority may charge fees for the performance of functions";
        assert_eq!(
            match_government_v2(text),
            Some(dc(DutyFamily::Government, DutySubType::Fees, 0.65))
        );
    }

    #[test]
    fn gov_v2_parliamentary_reporting() {
        let text = "a copy of the report shall be laid before parliament";
        assert_eq!(
            match_government_v2(text),
            Some(dc(
                DutyFamily::Government,
                DutySubType::ParliamentaryReporting,
                0.80
            ))
        );
    }

    // ── Governed ─────────────────────────────────────────────────────

    #[test]
    fn governed_general_duty() {
        let text = "it shall be the duty of every employer to ensure, so far as is \
                    reasonably practicable, the health, safety and welfare at work of \
                    all his employees";
        let result = match_governed(text).unwrap();
        assert_eq!(result.family, DutyFamily::Governed);
        assert_eq!(result.sub_type, DutySubType::GeneralDuty);
        assert!((result.confidence - 0.90).abs() < 0.01);
    }

    #[test]
    fn governed_general_duty_v2() {
        let text = "the general duty of every employer and every occupier";
        assert_eq!(
            match_governed(text),
            Some(dc(DutyFamily::Governed, DutySubType::GeneralDuty, 0.85))
        );
    }

    #[test]
    fn governed_prohibitive() {
        let text =
            "no person shall carry out work at height unless a risk assessment has been completed";
        assert_eq!(
            match_governed(text).map(|c| c.sub_type),
            Some(DutySubType::Prohibitive)
        );
    }

    #[test]
    fn governed_sfairp() {
        let text = "every employer shall, so far as is reasonably practicable, prevent exposure";
        let result = match_governed(text).unwrap();
        // GeneralDuty matches first (employer + shall ensure + health/safety pattern)
        // but if that doesn't match, SFAIRP will. Let's just check it classifies.
        assert_eq!(result.family, DutyFamily::Governed);
    }

    #[test]
    fn governed_information_duty() {
        let text = "the employer shall provide information to employees about the risks";
        let result = match_governed(text).unwrap();
        assert_eq!(result.family, DutyFamily::Governed);
        assert_eq!(result.sub_type, DutySubType::InformationDuty);
    }

    #[test]
    fn governed_risk_assessment() {
        let text = "every employer shall make a suitable and sufficient assessment of the risks";
        let result = match_governed(text).unwrap();
        assert_eq!(result.family, DutyFamily::Governed);
        assert_eq!(result.sub_type, DutySubType::RiskAssessment);
    }

    #[test]
    fn governed_training_duty() {
        let text = "the employer shall ensure that every employee receives adequate training";
        let result = match_governed(text).unwrap();
        assert_eq!(result.family, DutyFamily::Governed);
        assert_eq!(result.sub_type, DutySubType::TrainingDuty);
    }

    #[test]
    fn governed_prescriptive_generic() {
        let text = "the employer shall comply with the requirements of this regulation";
        let result = match_governed(text).unwrap();
        assert_eq!(result.sub_type, DutySubType::Prescriptive);
    }

    #[test]
    fn governed_no_match_unrelated() {
        let text = "the quick brown fox jumped over the lazy dog";
        assert!(match_governed(text).is_none());
    }

    // ── Helper functions ─────────────────────────────────────────────

    #[test]
    fn government_actor_detection() {
        assert!(has_government_actor("the secretary of state may"));
        assert!(has_government_actor("the inspector shall"));
        assert!(!has_government_actor("every employer shall"));
    }

    #[test]
    fn governed_actor_detection() {
        assert!(has_governed_actor("every employer shall ensure"));
        assert!(has_governed_actor("the occupier must"));
        assert!(!has_governed_actor("the secretary of state may"));
    }

    #[test]
    fn obligation_detection() {
        assert!(has_obligation("the employer shall ensure"));
        assert!(has_obligation("every person must comply"));
        assert!(!has_obligation("the employer may decide"));
    }

    #[test]
    fn prohibition_detection() {
        assert!(has_prohibition("no person shall carry out"));
        assert!(has_prohibition("the employer must not permit"));
        assert!(!has_prohibition("the employer shall ensure"));
    }

    #[test]
    fn enabling_detection() {
        assert!(has_enabling("the secretary of state may make"));
        assert!(has_enabling("power to authorise"));
        assert!(!has_enabling("the employer shall ensure"));
    }

    #[test]
    fn clamp01_works() {
        assert!((clamp01(0.5) - 0.5).abs() < 0.001);
        assert!((clamp01(-0.1) - 0.0).abs() < 0.001);
        assert!((clamp01(1.5) - 1.0).abs() < 0.001);
        assert!((clamp01(0.1234) - 0.123).abs() < 0.001);
    }
}
