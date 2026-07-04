//! Pattern-matching rules for duty type classification.
//!
//! Government patterns (v1 + v2) live here. Governed patterns have moved
//! to `duty_patterns_v2` which uses actor-anchored matching.
//!
//! Also includes shared helper functions: government-actor detection,
//! modal/obligation keyword checks, confidence utilities.

use std::sync::LazyLock;

use regex::Regex;

// ── Types ────────────────────────────────────────────────────────────

/// Top-level duty family: who bears the obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DutyFamily {
    Government,
    Governed,
    Rule,
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
    // Rule (thing-subject)
    ThingObligation,
    // Fallback
    Unclassified,
}

/// Character-level span of the matched DRRP pattern within the text.
///
/// Records the positions of the actor keyword and modal verb that formed the
/// anchored match. Used to extract a focused clause window around the match.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchSpan {
    /// Byte offset where the actor keyword starts.
    pub actor_start: usize,
    /// Byte offset where the modal verb starts.
    pub modal_start: usize,
    /// Byte offset where the modal verb ends.
    pub modal_end: usize,
}

/// Result of a pattern match attempt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DutyClassification {
    pub family: DutyFamily,
    pub sub_type: DutySubType,
    pub confidence: f32,
    /// Where in the text the pattern matched (if position was captured).
    pub span: Option<MatchSpan>,
}

impl DutyClassification {
    pub fn unknown() -> Self {
        Self {
            family: DutyFamily::Unknown,
            sub_type: DutySubType::Unclassified,
            confidence: 0.0,
            span: None,
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

// EU Directive — "Member States shall ensure/require"
static GOV_EU_ENSURE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bmember states?\b.*\bshall\b.*\b(?:ensure|require|establish|provide|take)\b")
        .unwrap()
});
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

// Shared helpers — pub(crate) so making.rs triage can use them
pub(crate) static OBLIGATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bshall\b|\bmust\b|\bis required to\b|\bhas a duty\b").unwrap()
});
pub(crate) static PROHIBITION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bshall not\b|\bmust not\b|\bno person shall\b|\bprohibit").unwrap()
});
pub(crate) static ENABLING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bmay\b|\bpower to\b|\bauthori[sz]e|\benable|\bentitled\b").unwrap());

// ── Actor fragment lists (downcased) ─────────────────────────────────
// Loaded from actor-dictionary.yaml via the actors module.

static GOVERNMENT_ACTORS: LazyLock<Vec<String>> = LazyLock::new(|| {
    super::actors::government_keywords()
});

// ── Shared helper functions ──────────────────────────────────────────

/// True when downcased text mentions a government-side actor.
pub fn has_government_actor(text: &str) -> bool {
    GOVERNMENT_ACTORS.iter().any(|frag| text.contains(frag.as_str()))
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

/// Find the government actor and modal verb byte offsets in downcased text.
///
/// Used to populate `MatchSpan` for government patterns so the position
/// heuristic in `mod.rs` can assign active/counterparty positions.
/// Returns `None` if either component can't be located.
fn find_government_span(text: &str) -> Option<MatchSpan> {
    // Find the first government actor keyword
    let (actor_start, _actor_len) = GOVERNMENT_ACTORS
        .iter()
        .filter_map(|frag| text.find(frag.as_str()).map(|pos| (pos, frag.len())))
        .min_by_key(|&(pos, _)| pos)?;

    // Find the modal verb (obligation or enabling)
    let modal_match = OBLIGATION.find(text).or_else(|| ENABLING.find(text))?;

    Some(MatchSpan {
        actor_start,
        modal_start: modal_match.start(),
        modal_end: modal_match.end(),
    })
}

/// Check whether the first modal verb in the text is enabling (may/power to/entitled)
/// rather than obligatory (shall/must). Used to override government pattern sub-types
/// when the keyword pattern matches but the modal context is enabling.
fn first_modal_is_enabling(text: &str) -> bool {
    let enabling_pos = ENABLING.find(text).map(|m| m.start());
    let obligation_pos = OBLIGATION.find(text).map(|m| m.start());
    match (enabling_pos, obligation_pos) {
        (Some(_), None) => true,
        (Some(e), Some(o)) => e < o,
        _ => false,
    }
}

/// Apply modal context to a government classification: if the text's first modal
/// is enabling, override the sub-type to Enabling (→ Liberty).
fn apply_modal_context(mut dc: DutyClassification, text: &str) -> DutyClassification {
    if dc.sub_type != DutySubType::Enabling && first_modal_is_enabling(text) {
        dc.sub_type = DutySubType::Enabling;
    }
    dc
}

/// Clamp a float to the 0.0..=1.0 range, rounded to 3 decimal places.
pub fn clamp01(v: f32) -> f32 {
    ((v.clamp(0.0, 1.0)) * 1000.0).round() / 1000.0
}

// ── Signal extraction ────────────────────────────────────────────────

/// Extract ALL government v1 signals (not just first match).
pub fn extract_government_v1_signals(text: &str) -> Vec<super::signals::PatternSignal> {
    use super::signals::{PatternSignal, SignalTier};

    let mut signals = Vec::new();

    let try_pattern = |re: &Regex, sub_type: DutySubType, confidence: f32| -> Option<PatternSignal> {
        if re.is_match(text) {
            let dc = apply_modal_context(
                DutyClassification {
                    family: DutyFamily::Government,
                    sub_type,
                    confidence,
                    span: find_government_span(text),
                },
                text,
            );
            Some(PatternSignal {
                tier: SignalTier::GovernmentV1,
                family: dc.family,
                sub_type: dc.sub_type,
                confidence: dc.confidence,
                span: dc.span,
                actor_keyword: None,
                actor_label: None,
            })
        } else {
            None
        }
    };

    if let Some(s) = try_pattern(&GOV_EU_ENSURE, DutySubType::Prescriptive, 0.85) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_REG_MAKING_1, DutySubType::RegulationMaking, 0.90) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_REG_MAKING_2, DutySubType::RegulationMaking, 0.85) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_CODE_APPROVAL, DutySubType::CodeApproval, 0.85) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_ENFORCEMENT_1, DutySubType::Enforcement, 0.85) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_ENFORCEMENT_2, DutySubType::Enforcement, 0.80) {
        signals.push(s);
    }
    // Blunt gate
    if has_government_actor(text) && has_obligation(text) {
        signals.push(PatternSignal {
            tier: SignalTier::GovernmentV1,
            family: DutyFamily::Government,
            sub_type: DutySubType::Prescriptive,
            confidence: 0.60,
            span: find_government_span(text),
            actor_keyword: None,
            actor_label: None,
        });
    }
    if has_government_actor(text) && has_enabling(text) {
        signals.push(PatternSignal {
            tier: SignalTier::GovernmentV1,
            family: DutyFamily::Government,
            sub_type: DutySubType::Enabling,
            confidence: 0.55,
            span: find_government_span(text),
            actor_keyword: None,
            actor_label: None,
        });
    }

    signals
}

/// Extract ALL government v2 signals (not just first match).
pub fn extract_government_v2_signals(text: &str) -> Vec<super::signals::PatternSignal> {
    use super::signals::{PatternSignal, SignalTier};

    let mut signals = Vec::new();

    let try_pattern = |re: &Regex, sub_type: DutySubType, confidence: f32, need_gov: bool| -> Option<PatternSignal> {
        if re.is_match(text) && (!need_gov || has_government_actor(text)) {
            let dc = apply_modal_context(
                DutyClassification {
                    family: DutyFamily::Government,
                    sub_type,
                    confidence,
                    span: find_government_span(text),
                },
                text,
            );
            Some(PatternSignal {
                tier: SignalTier::GovernmentV2,
                family: dc.family,
                sub_type: dc.sub_type,
                confidence: dc.confidence,
                span: dc.span,
                actor_keyword: None,
                actor_label: None,
            })
        } else {
            None
        }
    };

    if let Some(s) = try_pattern(&GOV_DIRECTION, DutySubType::Direction, 0.80, false) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_GUIDANCE, DutySubType::Guidance, 0.75, true) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_CONSULTATION, DutySubType::ConsultationObligation, 0.75, false) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_APPOINTMENT, DutySubType::Appointment, 0.80, false) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_DELEGATION, DutySubType::Delegation, 0.70, true) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_FEES, DutySubType::Fees, 0.65, true) {
        signals.push(s);
    }
    if let Some(s) = try_pattern(&GOV_PARL_REPORTING, DutySubType::ParliamentaryReporting, 0.80, false) {
        signals.push(s);
    }

    signals
}

// ── Pattern matchers ─────────────────────────────────────────────────

/// Try government duty patterns (v1) against downcased text.
///
/// Specific keyword patterns are checked first (enforcement, regulation-making,
/// etc.), then the blunt government-actor + modal gate. All specific patterns
/// pass through `apply_modal_context` so "authority may serve a notice" returns
/// Enabling (→ Liberty), not Enforcement (→ Obligation).
pub fn match_government_v1(text: &str) -> Option<DutyClassification> {
    // EU Directive: "Member States shall ensure/require that..."
    if GOV_EU_ENSURE.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Prescriptive,
            confidence: 0.85,
            span: find_government_span(text),
        }, text));
    }
    if GOV_REG_MAKING_1.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::RegulationMaking,
            confidence: 0.90,
            span: find_government_span(text),
        }, text));
    }
    if GOV_REG_MAKING_2.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::RegulationMaking,
            confidence: 0.85,
            span: find_government_span(text),
        }, text));
    }
    if GOV_CODE_APPROVAL.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::CodeApproval,
            confidence: 0.85,
            span: find_government_span(text),
        }, text));
    }
    if GOV_ENFORCEMENT_1.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Enforcement,
            confidence: 0.85,
            span: find_government_span(text),
        }, text));
    }
    if GOV_ENFORCEMENT_2.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Enforcement,
            confidence: 0.80,
            span: find_government_span(text),
        }, text));
    }
    // Blunt gate — check which modal type dominates
    if has_government_actor(text) {
        let has_obl = has_obligation(text);
        let has_ena = has_enabling(text);
        if has_obl && has_ena {
            // Both modals present — enabling wins if it appears first
            if first_modal_is_enabling(text) {
                return Some(DutyClassification {
                    family: DutyFamily::Government,
                    sub_type: DutySubType::Enabling,
                    confidence: 0.55,
                    span: find_government_span(text),
                });
            } else {
                return Some(DutyClassification {
                    family: DutyFamily::Government,
                    sub_type: DutySubType::Prescriptive,
                    confidence: 0.60,
                    span: find_government_span(text),
                });
            }
        } else if has_obl {
            return Some(DutyClassification {
                family: DutyFamily::Government,
                sub_type: DutySubType::Prescriptive,
                confidence: 0.60,
                span: find_government_span(text),
            });
        } else if has_ena {
            return Some(DutyClassification {
                family: DutyFamily::Government,
                sub_type: DutySubType::Enabling,
                confidence: 0.55,
                span: find_government_span(text),
            });
        }
    }
    None
}

/// Try extended government duty patterns (v2) against downcased text.
///
/// All patterns pass through `apply_modal_context` for enabling/obligation awareness.
pub fn match_government_v2(text: &str) -> Option<DutyClassification> {
    if GOV_DIRECTION.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Direction,
            confidence: 0.80,
            span: find_government_span(text),
        }, text));
    }
    if GOV_GUIDANCE.is_match(text) && has_government_actor(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Guidance,
            confidence: 0.75,
            span: find_government_span(text),
        }, text));
    }
    if GOV_CONSULTATION.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::ConsultationObligation,
            confidence: 0.75,
            span: find_government_span(text),
        }, text));
    }
    if GOV_APPOINTMENT.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Appointment,
            confidence: 0.80,
            span: find_government_span(text),
        }, text));
    }
    if GOV_DELEGATION.is_match(text) && has_government_actor(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Delegation,
            confidence: 0.70,
            span: find_government_span(text),
        }, text));
    }
    if GOV_FEES.is_match(text) && has_government_actor(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::Fees,
            confidence: 0.65,
            span: find_government_span(text),
        }, text));
    }
    if GOV_PARL_REPORTING.is_match(text) {
        return Some(apply_modal_context(DutyClassification {
            family: DutyFamily::Government,
            sub_type: DutySubType::ParliamentaryReporting,
            confidence: 0.80,
            span: find_government_span(text),
        }, text));
    }
    None
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Assert that a government pattern match has the expected family/sub_type/confidence.
    /// Span is tested separately — we only check the classification fields here.
    fn assert_gov_match(
        result: Option<DutyClassification>,
        family: DutyFamily,
        sub_type: DutySubType,
        confidence: f32,
    ) {
        let dc = result.expect("expected a match");
        assert_eq!(dc.family, family);
        assert_eq!(dc.sub_type, sub_type);
        assert!(
            (dc.confidence - confidence).abs() < 0.001,
            "confidence mismatch"
        );
    }

    // ── Government v1 ────────────────────────────────────────────────

    #[test]
    fn gov_v1_regulation_making_sos() {
        let text = "the secretary of state shall have power to make regulations";
        assert_gov_match(
            match_government_v1(text),
            DutyFamily::Government,
            DutySubType::RegulationMaking,
            0.90,
        );
    }

    #[test]
    fn gov_v1_regulation_making_power_to() {
        // "power to" is enabling → overrides to Enabling
        let text = "there is a power to make regulations under this section";
        assert_gov_match(
            match_government_v1(text),
            DutyFamily::Government,
            DutySubType::Enabling,
            0.85,
        );
    }

    #[test]
    fn gov_v1_code_approval() {
        // "may approve" → enabling context
        let text = "the commission may approve a code of practice";
        assert_gov_match(
            match_government_v1(text),
            DutyFamily::Government,
            DutySubType::Enabling,
            0.85,
        );
    }

    #[test]
    fn gov_v1_enforcement_notice_enabling() {
        // "may serve" → enabling context overrides enforcement
        let text = "an inspector may serve an improvement notice or prohibition notice";
        assert_eq!(
            match_government_v1(text).map(|c| c.sub_type),
            Some(DutySubType::Enabling)
        );
    }

    #[test]
    fn gov_v1_enforcement_notice_obligation() {
        // "shall serve" → obligation context preserves enforcement
        let text = "an inspector shall serve an improvement notice or prohibition notice";
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

    #[test]
    fn gov_v1_ofcom_prescriptive() {
        let text = "ofcom must prepare a code of practice";
        let result = match_government_v1(text).unwrap();
        assert_eq!(result.sub_type, DutySubType::Prescriptive);
    }

    #[test]
    fn gov_v1_ofcom_enabling() {
        let text = "ofcom may issue a confirmation decision";
        let result = match_government_v1(text).unwrap();
        assert_eq!(result.sub_type, DutySubType::Enabling);
    }

    #[test]
    fn gov_v1_chief_officer_prescriptive() {
        let text = "the chief officer of police shall by notice require the holder";
        let result = match_government_v1(text).unwrap();
        assert_eq!(result.sub_type, DutySubType::Prescriptive);
    }

    #[test]
    fn gov_v1_sheriff_enabling() {
        let text = "the sheriff may make an order requiring the person to attend";
        let result = match_government_v1(text).unwrap();
        assert_eq!(result.sub_type, DutySubType::Enabling);
    }

    // ── Government v2 ────────────────────────────────────────────────

    #[test]
    fn gov_v2_direction_enabling() {
        // "may give" → enabling context
        let text = "the secretary of state may give directions to the executive";
        assert_gov_match(
            match_government_v2(text),
            DutyFamily::Government,
            DutySubType::Enabling,
            0.80,
        );
    }

    #[test]
    fn gov_v2_direction_obligation() {
        // "shall give" → obligation context preserves Direction
        let text = "the secretary of state shall give directions to the executive";
        assert_gov_match(
            match_government_v2(text),
            DutyFamily::Government,
            DutySubType::Direction,
            0.80,
        );
    }

    #[test]
    fn gov_v2_guidance() {
        // "may issue" → enabling context
        let text = "the executive may issue guidance on the application of these regulations";
        assert_gov_match(
            match_government_v2(text),
            DutyFamily::Government,
            DutySubType::Enabling,
            0.75,
        );
    }

    #[test]
    fn gov_v2_consultation() {
        let text = "the secretary of state shall consult such bodies as appear appropriate";
        assert_gov_match(
            match_government_v2(text),
            DutyFamily::Government,
            DutySubType::ConsultationObligation,
            0.75,
        );
    }

    #[test]
    fn gov_v2_appointment() {
        // "may appoint" → enabling context
        let text = "the executive may appoint any suitably qualified person as an inspector";
        assert_gov_match(
            match_government_v2(text),
            DutyFamily::Government,
            DutySubType::Enabling,
            0.80,
        );
    }

    #[test]
    fn gov_v2_delegation() {
        // "may delegate" → enabling context
        let text = "the authority may delegate any of its functions to a committee";
        assert_gov_match(
            match_government_v2(text),
            DutyFamily::Government,
            DutySubType::Enabling,
            0.70,
        );
    }

    #[test]
    fn gov_v2_fees() {
        // "may charge" → enabling context
        let text = "the authority may charge fees for the performance of functions";
        assert_gov_match(
            match_government_v2(text),
            DutyFamily::Government,
            DutySubType::Enabling,
            0.65,
        );
    }

    #[test]
    fn gov_v2_parliamentary_reporting() {
        let text = "a copy of the report shall be laid before parliament";
        assert_gov_match(
            match_government_v2(text),
            DutyFamily::Government,
            DutySubType::ParliamentaryReporting,
            0.80,
        );
    }

    // ── Helper functions ─────────────────────────────────────────────

    #[test]
    fn government_actor_detection() {
        assert!(has_government_actor("the secretary of state may"));
        assert!(has_government_actor("the inspector shall"));
        assert!(!has_government_actor("every employer shall"));
    }

    #[test]
    fn government_actor_detection_public_family() {
        assert!(has_government_actor(
            "ofcom must prepare a code of practice"
        ));
        assert!(has_government_actor("the chief officer of police shall"));
        assert!(has_government_actor("a constable may seize the dog"));
        assert!(has_government_actor("the sheriff may make an order"));
        assert!(has_government_actor(
            "the procurator fiscal must investigate"
        ));
        assert!(has_government_actor("the department may by order require"));
        assert!(has_government_actor("police force for the area"));
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

    // ── Government span propagation ─────────────────────────────────

    #[test]
    fn gov_v1_regulation_making_has_span() {
        let text = "the secretary of state shall make regulations prescribing requirements";
        let dc = match_government_v1(text).unwrap();
        let span = dc.span.expect("government pattern should populate span");
        assert_eq!(span.actor_start, text.find("secretary").unwrap());
        assert_eq!(span.modal_start, text.find("shall").unwrap());
    }

    #[test]
    fn gov_v1_enforcement_has_span() {
        let text = "an inspector may serve an improvement notice on a person";
        let dc = match_government_v1(text).unwrap();
        let span = dc.span.expect("enforcement pattern should populate span");
        assert_eq!(span.actor_start, text.find("inspector").unwrap());
    }

    #[test]
    fn gov_v1_prescriptive_fallback_has_span() {
        let text = "the authority shall ensure compliance with the requirements";
        let dc = match_government_v1(text).unwrap();
        let span = dc.span.expect("fallback pattern should populate span");
        assert_eq!(span.actor_start, text.find("authority").unwrap());
        assert_eq!(span.modal_start, text.find("shall").unwrap());
    }

    #[test]
    fn gov_v2_direction_has_span() {
        let text = "the secretary of state may give directions to the executive";
        let dc = match_government_v2(text).unwrap();
        let span = dc.span.expect("direction pattern should populate span");
        assert_eq!(span.actor_start, text.find("secretary").unwrap());
    }
}
