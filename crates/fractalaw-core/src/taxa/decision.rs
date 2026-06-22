//! Decision engine for the DRRP classification pipeline.
//!
//! Given a [`SignalSet`] of all signals extracted from a provision,
//! picks the best classification. The default strategy replicates the
//! current first-match-wins tier cascade: GovernedV2 > GovV1 > GovV2 >
//! Offence > Rule, then highest confidence within the winning tier.

use super::duty_patterns::{DutyClassification, DutyFamily};
use super::duty_type::{ClassificationResult, DutyType};
use super::signals::{PatternSignal, SignalSet, SignalTier};

/// Trace of how the decision was made.
#[derive(Debug, Clone)]
pub struct DecisionTrail {
    /// The winning signal (if any).
    pub winner: Option<PatternSignal>,
    /// Why this signal won.
    pub reason: &'static str,
    /// Total positive signals considered.
    pub candidates_count: usize,
    /// Total rejected signals.
    pub rejections_count: usize,
}

/// Tier priority order — matches the existing first-match-wins cascade.
const TIER_ORDER: &[SignalTier] = &[
    SignalTier::GovernedV2,
    SignalTier::GovernmentV1,
    SignalTier::GovernmentV2,
    SignalTier::OffenceAsDuty,
    SignalTier::Rule,
];

/// Given a complete SignalSet, pick the best classification.
///
/// Default strategy: tier priority first, then highest confidence within
/// the winning tier. This replicates the current first-match-wins cascade.
pub fn decide(signals: &SignalSet) -> (ClassificationResult, DecisionTrail) {
    let empty = (
        ClassificationResult {
            duty_types: Vec::new(),
            classification: None,
        },
        DecisionTrail {
            winner: None,
            reason: "no_signals",
            candidates_count: signals.matches.len(),
            rejections_count: signals.rejected.len(),
        },
    );

    if signals.purpose_gated {
        return (
            empty.0,
            DecisionTrail {
                reason: "purpose_gated",
                ..empty.1
            },
        );
    }
    if signals.is_descriptive_summary {
        return (
            empty.0,
            DecisionTrail {
                reason: "descriptive_summary",
                ..empty.1
            },
        );
    }
    if signals.is_legal_fiction {
        return (
            empty.0,
            DecisionTrail {
                reason: "legal_fiction",
                ..empty.1
            },
        );
    }

    if signals.matches.is_empty() {
        return empty;
    }

    // Find the best signal: tier priority first, then confidence within tier
    for &tier in TIER_ORDER {
        let best = signals
            .matches
            .iter()
            .filter(|s| s.tier == tier)
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal));

        if let Some(winner) = best {
            let dc = DutyClassification {
                family: winner.family,
                sub_type: winner.sub_type,
                confidence: winner.confidence,
                span: winner.span,
            };
            let duty_types = map_to_duty_types(&dc);
            return (
                ClassificationResult {
                    duty_types,
                    classification: Some(dc),
                },
                DecisionTrail {
                    winner: Some(winner.clone()),
                    reason: "tier_priority_then_confidence",
                    candidates_count: signals.matches.len(),
                    rejections_count: signals.rejected.len(),
                },
            );
        }
    }

    empty
}

/// Map a DutyClassification to DRRP DutyTypes.
///
/// Replicates the logic from `duty_type::map_to_duty_type`.
fn map_to_duty_types(dc: &DutyClassification) -> Vec<DutyType> {
    use super::duty_patterns::DutySubType;

    match dc.family {
        DutyFamily::Government | DutyFamily::Governed => match dc.sub_type {
            DutySubType::Enabling => vec![DutyType::Liberty],
            DutySubType::Prohibitive => vec![DutyType::Obligation],
            _ => {
                if super::duty_patterns::has_enabling(dc.sub_type.as_str_lower()) {
                    vec![DutyType::Liberty]
                } else {
                    vec![DutyType::Obligation]
                }
            }
        },
        DutyFamily::Rule => match dc.sub_type {
            DutySubType::Enabling => vec![DutyType::Liberty],
            _ => vec![DutyType::Obligation],
        },
        DutyFamily::Unknown => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxa::duty_patterns::{DutyFamily, DutySubType};
    use crate::taxa::signals::SignalTier;

    fn make_signal(tier: SignalTier, family: DutyFamily, sub_type: DutySubType, confidence: f32) -> PatternSignal {
        PatternSignal {
            tier,
            family,
            sub_type,
            confidence,
            span: None,
            actor_keyword: None,
            actor_label: None,
        }
    }

    #[test]
    fn empty_signals_returns_no_classification() {
        let signals = SignalSet::default();
        let (result, trail) = decide(&signals);
        assert!(result.duty_types.is_empty());
        assert!(trail.winner.is_none());
        assert_eq!(trail.reason, "no_signals");
    }

    #[test]
    fn purpose_gated_returns_empty() {
        let signals = SignalSet {
            purpose_gated: true,
            matches: vec![make_signal(
                SignalTier::GovernedV2,
                DutyFamily::Governed,
                DutySubType::Prescriptive,
                0.80,
            )],
            ..Default::default()
        };
        let (result, trail) = decide(&signals);
        assert!(result.duty_types.is_empty());
        assert_eq!(trail.reason, "purpose_gated");
    }

    #[test]
    fn tier_priority_wins_over_confidence() {
        let signals = SignalSet {
            matches: vec![
                make_signal(SignalTier::GovernmentV1, DutyFamily::Government, DutySubType::Enforcement, 0.95),
                make_signal(SignalTier::GovernedV2, DutyFamily::Governed, DutySubType::Prescriptive, 0.70),
            ],
            ..Default::default()
        };
        let (_, trail) = decide(&signals);
        // GovernedV2 wins despite lower confidence — tier priority
        assert_eq!(trail.winner.as_ref().unwrap().tier, SignalTier::GovernedV2);
    }

    #[test]
    fn highest_confidence_within_tier() {
        let signals = SignalSet {
            matches: vec![
                make_signal(SignalTier::GovernedV2, DutyFamily::Governed, DutySubType::Prescriptive, 0.60),
                make_signal(SignalTier::GovernedV2, DutyFamily::Governed, DutySubType::GeneralDuty, 0.90),
            ],
            ..Default::default()
        };
        let (_, trail) = decide(&signals);
        assert_eq!(trail.winner.as_ref().unwrap().sub_type, DutySubType::GeneralDuty);
    }

    #[test]
    fn enabling_maps_to_liberty() {
        let signals = SignalSet {
            matches: vec![make_signal(
                SignalTier::GovernedV2,
                DutyFamily::Governed,
                DutySubType::Enabling,
                0.50,
            )],
            ..Default::default()
        };
        let (result, _) = decide(&signals);
        assert_eq!(result.duty_types, vec![DutyType::Liberty]);
    }

    #[test]
    fn legal_fiction_returns_empty() {
        let signals = SignalSet {
            is_legal_fiction: true,
            ..Default::default()
        };
        let (result, trail) = decide(&signals);
        assert!(result.duty_types.is_empty());
        assert_eq!(trail.reason, "legal_fiction");
    }
}
