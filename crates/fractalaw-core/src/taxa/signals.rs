//! Signal extraction layer for the DRRP classification pipeline.
//!
//! Separates signal detection (finding actor-modal pairs, keyword matches,
//! pattern hits) from decision logic (choosing which classification to accept).
//! Every tier runs and contributes signals to a [`SignalSet`], which the
//! [`decision`](super::decision) module then evaluates.

use super::actors::ActorMatch;
use super::duty_patterns::{DutyFamily, DutySubType, MatchSpan};

/// Which regex tier produced a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalTier {
    GovernedV2,
    GovernmentV1,
    GovernmentV2,
    OffenceAsDuty,
    Rule,
}

/// A single positive pattern hit from the regex cascade.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternSignal {
    pub tier: SignalTier,
    pub family: DutyFamily,
    pub sub_type: DutySubType,
    pub confidence: f32,
    pub span: Option<MatchSpan>,
    /// Which actor keyword produced this match (if actor-anchored).
    pub actor_keyword: Option<String>,
    /// Which actor label (e.g. "Org: Employer") this signal relates to.
    pub actor_label: Option<String>,
}

/// Why a potential match was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RejectionReason {
    SubordinateClause,
    EpistemicMay,
    DefinitionalConstruction,
    PenaltyProvision,
    LegalFiction,
    DescriptiveSummary,
    PurposeGated,
}

/// A candidate match that was rejected, with the reason.
#[derive(Debug, Clone, PartialEq)]
pub struct RejectedSignal {
    pub tier: SignalTier,
    pub reason: RejectionReason,
    pub actor_keyword: Option<String>,
    pub span: Option<MatchSpan>,
}

/// All signals extracted from a single provision, before decision logic.
#[derive(Debug, Clone, Default)]
pub struct SignalSet {
    /// All positive pattern hits across all tiers.
    pub matches: Vec<PatternSignal>,
    /// Candidates that were rejected, with reasons.
    pub rejected: Vec<RejectedSignal>,
    /// Extracted governed actors.
    pub governed_actors: Vec<ActorMatch>,
    /// Extracted government actors.
    pub government_actors: Vec<ActorMatch>,
    /// Purpose classifications.
    pub purposes: Vec<&'static str>,
    /// Pre-filter flags.
    pub is_legal_fiction: bool,
    pub is_descriptive_summary: bool,
    pub purpose_gated: bool,
}

/// Extract all signals from lowercased text using all 5 regex tiers.
///
/// This is the signal detection layer — it runs every tier and collects
/// all matches and rejections. The decision layer ([`super::decision::decide`])
/// then picks the winner.
pub fn extract_all(
    lower: &str,
    governed_actors: &[ActorMatch],
    government_actors: &[ActorMatch],
    purposes: &[&'static str],
    is_legal_fiction: bool,
    is_descriptive_summary: bool,
    purpose_gated: bool,
) -> SignalSet {
    let mut signals = SignalSet {
        governed_actors: governed_actors.to_vec(),
        government_actors: government_actors.to_vec(),
        purposes: purposes.to_vec(),
        is_legal_fiction,
        is_descriptive_summary,
        purpose_gated,
        ..Default::default()
    };

    // Don't extract DRRP signals if gated out
    if purpose_gated || is_descriptive_summary {
        return signals;
    }

    // Tier 1: Governed V2 (actor-anchored) — all signals
    {
        let (matches, rejections) =
            super::duty_patterns_v2::extract_governed_v2_signals(lower, governed_actors);
        signals.matches.extend(matches);
        signals.rejected.extend(rejections);
    }

    // Tier 2: Government V1 — all signals
    signals
        .matches
        .extend(super::duty_patterns::extract_government_v1_signals(lower));

    // Tier 3: Government V2 — all signals
    signals
        .matches
        .extend(super::duty_patterns::extract_government_v2_signals(lower));

    // Tier 4: Offence-as-duty — all signals
    {
        let (matches, rejections) =
            super::duty_patterns_offence::extract_offence_signals(lower);
        signals.matches.extend(matches);
        signals.rejected.extend(rejections);
    }

    // Tier 5: Rule (thing-subject) — all signals
    {
        let (matches, rejections) =
            super::duty_patterns_rule::extract_rule_signals(lower);
        signals.matches.extend(matches);
        signals.rejected.extend(rejections);
    }

    // Legal fiction is a post-match rejection
    if is_legal_fiction && !signals.matches.is_empty() {
        for sig in signals.matches.drain(..) {
            signals.rejected.push(RejectedSignal {
                tier: sig.tier,
                reason: RejectionReason::LegalFiction,
                actor_keyword: sig.actor_keyword,
                span: sig.span,
            });
        }
    }

    signals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_produces_empty_signals() {
        let signals = extract_all("", &[], &[], &[], false, false, false);
        assert!(signals.matches.is_empty());
        assert!(signals.rejected.is_empty());
    }

    #[test]
    fn purpose_gated_skips_extraction() {
        let signals = extract_all(
            "the employer shall ensure safety",
            &[],
            &[],
            &["Interpretation+Definition"],
            false,
            false,
            true, // purpose_gated
        );
        assert!(signals.matches.is_empty());
        assert!(signals.purpose_gated);
    }

    #[test]
    fn descriptive_summary_skips_extraction() {
        let signals = extract_all(
            "the employer shall ensure safety",
            &[],
            &[],
            &[],
            false,
            true, // descriptive_summary
            false,
        );
        assert!(signals.matches.is_empty());
        assert!(signals.is_descriptive_summary);
    }

    #[test]
    fn legal_fiction_moves_matches_to_rejected() {
        let actors = vec![ActorMatch {
            label: "Org: Employer".into(),
            keyword: "employer".into(),
            offset: 4,
        }];
        let signals = extract_all(
            "the employer shall ensure health and safety",
            &actors,
            &[],
            &[],
            true, // is_legal_fiction
            false,
            false,
        );
        assert!(signals.matches.is_empty());
        assert!(!signals.rejected.is_empty());
        assert_eq!(signals.rejected[0].reason, RejectionReason::LegalFiction);
    }
}
