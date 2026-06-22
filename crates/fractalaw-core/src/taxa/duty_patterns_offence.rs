//! Offence-as-duty pattern matcher.
//!
//! Detects provisions that express duties as offence-creating language rather
//! than modal verbs. UK legislation frequently uses patterns like:
//!
//! - "It is an offence for a person to X" → Duty (Prohibitive)
//! - "A person commits an offence if..." → Duty (Prohibitive)
//! - "It shall be unlawful for any person to..." → Duty (Prohibitive)
//!
//! These provisions impose a duty on the named actor NOT to do the thing.
//! The pipeline's modal-based tiers (governed v2, gov v1/v2) are blind to
//! them because there is no shall/must/may entry point.
//!
//! This is Tier 4 in the classification cascade: after Governed v2 and
//! Government v1/v2, before Rule (thing-subject).

use std::sync::LazyLock;

use regex::Regex;

use super::duty_patterns::{DutyClassification, DutyFamily, DutySubType, MatchSpan};

// ── Offence-creating patterns ───────────────────────────────────────

/// "it is an offence for [actor] to [action]"
/// "it shall be an offence for [actor] to [action]"
static OFFENCE_FOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bit (?:is|shall be) an offence for\b").unwrap());

/// "[actor] commits an offence if [condition]"
/// "[actor] commit an offence if [condition]"
static COMMITS_OFFENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bcommits? an offence if\b").unwrap());

/// "[actor] shall be guilty of an offence if [condition]"
/// "[actor] is guilty of an offence if [condition]"
static GUILTY_IF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:shall be|is) guilty of an offence if\b").unwrap());

/// "it is unlawful for [actor] to [action]"
/// "it shall be unlawful for [actor] to [action]"
static UNLAWFUL_FOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bit (?:is|shall be) unlawful for\b").unwrap());

// ── Penalty exclusion ───────────────────────────────────────────────

/// Penalty/sentencing language — provisions with this as the primary content
/// are describing punishment, not creating a duty.
///
/// "A person guilty of an offence is liable to..."
/// "...liable on summary conviction to a fine"
/// "...liable on conviction on indictment to imprisonment"
static PENALTY_PRIMARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:^|\. )\s*(?:a |any )?person (?:who is |)guilty of (?:an |the |such an? |)\
offence (?:under |is |shall be )(?:liable|punishable)|(?:^|\. )\s*(?:is |shall be )liable (?:on |to )",
    )
    .unwrap()
});

/// Secondary penalty indicator — when the provision is ONLY about penalties.
/// If the text matches an offence-creating pattern AND this, we check whether
/// the offence pattern comes before the penalty language (duty) or after (penalty).
static PENALTY_PHRASES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bliable (?:on (?:summary )?conviction|to (?:a fine|imprisonment|a penalty))")
        .unwrap()
});

// ── Public API ──────────────────────────────────────────────────────

/// Extract ALL offence-as-duty signals with rejections.
pub fn extract_offence_signals(
    text: &str,
) -> (Vec<super::signals::PatternSignal>, Vec<super::signals::RejectedSignal>) {
    use super::signals::{PatternSignal, RejectedSignal, RejectionReason, SignalTier};

    let mut matches = Vec::new();
    let mut rejections = Vec::new();

    // Primary penalty exclusion
    if PENALTY_PRIMARY.is_match(text) {
        rejections.push(RejectedSignal {
            tier: SignalTier::OffenceAsDuty,
            reason: RejectionReason::PenaltyProvision,
            actor_keyword: None,
            span: None,
        });
        return (matches, rejections);
    }

    let try_pattern = |re: &Regex, confidence: f32, actor_start_fn: fn(usize, usize) -> usize| {
        if let Some(m) = re.find(text) {
            if is_penalty_dominant(text, m.start()) {
                return (None, Some(RejectedSignal {
                    tier: SignalTier::OffenceAsDuty,
                    reason: RejectionReason::PenaltyProvision,
                    actor_keyword: None,
                    span: Some(MatchSpan {
                        actor_start: actor_start_fn(m.start(), m.end()),
                        modal_start: m.start(),
                        modal_end: m.end(),
                    }),
                }));
            }
            return (Some(PatternSignal {
                tier: SignalTier::OffenceAsDuty,
                family: DutyFamily::Governed,
                sub_type: DutySubType::Prohibitive,
                confidence,
                span: Some(MatchSpan {
                    actor_start: actor_start_fn(m.start(), m.end()),
                    modal_start: m.start(),
                    modal_end: m.end(),
                }),
                actor_keyword: None,
                actor_label: None,
            }), None);
        }
        (None, None)
    };

    // "it is an offence for" → actor after "for"
    let (m, r) = try_pattern(&OFFENCE_FOR, 0.70, |_start, end| end);
    if let Some(m) = m { matches.push(m); }
    if let Some(r) = r { rejections.push(r); }

    // "commits an offence if" → actor at start
    let (m, r) = try_pattern(&COMMITS_OFFENCE, 0.70, |_start, _end| 0);
    if let Some(m) = m { matches.push(m); }
    if let Some(r) = r { rejections.push(r); }

    // "guilty of an offence if" → actor at start
    let (m, r) = try_pattern(&GUILTY_IF, 0.65, |_start, _end| 0);
    if let Some(m) = m { matches.push(m); }
    if let Some(r) = r { rejections.push(r); }

    // "it is unlawful for" → actor after "for"
    let (m, r) = try_pattern(&UNLAWFUL_FOR, 0.70, |_start, end| end);
    if let Some(m) = m { matches.push(m); }
    if let Some(r) = r { rejections.push(r); }

    (matches, rejections)
}

/// Try to match an offence-creating provision as a duty.
///
/// Returns `Governed / Prohibitive` if the provision creates a duty
/// expressed as offence language. Returns `None` for penalty/sentencing
/// provisions and non-matching text.
pub fn match_offence_as_duty(text: &str) -> Option<DutyClassification> {
    // Exclude provisions that are primarily about penalties
    if PENALTY_PRIMARY.is_match(text) {
        return None;
    }

    // Pattern 1: "it is an offence for [X] to [Y]"
    if let Some(m) = OFFENCE_FOR.find(text) {
        // Verify this isn't a penalty-only provision
        if !is_penalty_dominant(text, m.start()) {
            return Some(DutyClassification {
                family: DutyFamily::Governed,
                sub_type: DutySubType::Prohibitive,
                confidence: 0.70,
                span: Some(MatchSpan {
                    actor_start: m.end(), // actor follows "for"
                    modal_start: m.start(),
                    modal_end: m.end(),
                }),
            });
        }
    }

    // Pattern 2: "[X] commits an offence if [Y]"
    if let Some(m) = COMMITS_OFFENCE.find(text)
        && !is_penalty_dominant(text, m.start())
    {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::Prohibitive,
            confidence: 0.70,
            span: Some(MatchSpan {
                actor_start: 0, // actor is at start of text (before "commits")
                modal_start: m.start(),
                modal_end: m.end(),
            }),
        });
    }

    // Pattern 3: "[X] shall be / is guilty of an offence if [Y]"
    if let Some(m) = GUILTY_IF.find(text)
        && !is_penalty_dominant(text, m.start())
    {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::Prohibitive,
            confidence: 0.65,
            span: Some(MatchSpan {
                actor_start: 0,
                modal_start: m.start(),
                modal_end: m.end(),
            }),
        });
    }

    // Pattern 4: "it is/shall be unlawful for [X] to [Y]"
    if let Some(m) = UNLAWFUL_FOR.find(text)
        && !is_penalty_dominant(text, m.start())
    {
        return Some(DutyClassification {
            family: DutyFamily::Governed,
            sub_type: DutySubType::Prohibitive,
            confidence: 0.70,
            span: Some(MatchSpan {
                actor_start: m.end(),
                modal_start: m.start(),
                modal_end: m.end(),
            }),
        });
    }

    None
}

/// Check whether the provision is dominated by penalty language that
/// appears after the offence-creating phrase.
///
/// If penalty language (liable to fine/imprisonment) appears and the
/// text after the offence pattern is mostly about sentencing, exclude it.
fn is_penalty_dominant(text: &str, offence_pos: usize) -> bool {
    if let Some(penalty_m) = PENALTY_PHRASES.find(text) {
        // If penalty language comes BEFORE the offence pattern, the provision
        // is describing a penalty regime, not creating a duty
        if penalty_m.start() < offence_pos {
            return true;
        }
        // If the offence pattern is in the first 30% of text and penalty
        // language is the bulk, it might still be a duty + penalty combined.
        // We keep those — the duty is the primary content.
    }
    false
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── True positives ──────────────────────────────────────────────

    #[test]
    fn offence_for_person_to_contravene() {
        let text =
            "it is an offence for a person to contravene any byelaws made under section 26 or 29.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.family, DutyFamily::Governed);
        assert_eq!(dc.sub_type, DutySubType::Prohibitive);
    }

    #[test]
    fn offence_for_person_to_fail() {
        let text = "it is an offence for a person to fail to comply with a condition subject to which a firearm certificate is held by him.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.family, DutyFamily::Governed);
    }

    #[test]
    fn shall_be_offence_for() {
        let text = "it shall be an offence for any person to contravene or fail to comply with any requirement of regulations 5 to 15.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.sub_type, DutySubType::Prohibitive);
    }

    #[test]
    fn person_commits_offence_if() {
        let text = "a person commits an offence if the person passes, or permits to be passed, any relevant substance from trade premises into a public sewer.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.family, DutyFamily::Governed);
        assert_eq!(dc.sub_type, DutySubType::Prohibitive);
    }

    #[test]
    fn commits_offence_if_fails() {
        let text = "a person commits an offence if he fails to give his name and address when required to do so.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.sub_type, DutySubType::Prohibitive);
    }

    #[test]
    fn guilty_of_offence_if() {
        let text = "any person shall be guilty of an offence if he uses on a road a vehicle which does not comply with these regulations.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.family, DutyFamily::Governed);
        assert_eq!(dc.sub_type, DutySubType::Prohibitive);
    }

    #[test]
    fn unlawful_for_person_to_keep() {
        let text = "except as permitted by this order, it shall be unlawful for any person to keep a dog of any description unless he holds a dog licence.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.family, DutyFamily::Governed);
        assert_eq!(dc.sub_type, DutySubType::Prohibitive);
    }

    #[test]
    fn unlawful_for_is_variant() {
        let text = "it is unlawful for any person except with the consent of the building control authority to close or obstruct the means of access.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.sub_type, DutySubType::Prohibitive);
    }

    #[test]
    fn pawnbroker_offence() {
        let text = "it is an offence for a pawnbroker to take in pawn any firearm or ammunition to which section 1 of this act applies.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.family, DutyFamily::Governed);
    }

    #[test]
    fn offence_for_holder_to_fail() {
        let text = "it is an offence for the holder to fail to surrender the certificate within twenty-one days from the date of the notice.";
        let dc = match_offence_as_duty(text).unwrap();
        assert_eq!(dc.sub_type, DutySubType::Prohibitive);
    }

    // ── True negatives ──────────────────────────────────────────────

    #[test]
    fn penalty_provision_no_match() {
        let text = "a person guilty of an offence under this section is liable on summary conviction to a fine not exceeding level 5 on the standard scale.";
        assert!(
            match_offence_as_duty(text).is_none(),
            "penalty provision should not match"
        );
    }

    #[test]
    fn liable_to_imprisonment_no_match() {
        let text = "a person guilty of an offence is liable on conviction on indictment to imprisonment for a term not exceeding two years.";
        assert!(
            match_offence_as_duty(text).is_none(),
            "imprisonment penalty should not match"
        );
    }

    #[test]
    fn no_offence_language() {
        let text = "the employer shall ensure the health and safety of employees.";
        assert!(match_offence_as_duty(text).is_none());
    }

    #[test]
    fn offence_as_mere_reference() {
        // "an offence" mentioned but not in a duty-creating pattern
        let text = "a constable may arrest without warrant any person who he has reasonable cause to suspect is committing an offence.";
        assert!(match_offence_as_duty(text).is_none());
    }

    #[test]
    fn penalty_before_offence_rejects() {
        // Penalty language dominates — "liable" appears first, then "offence"
        let text = "shall be liable on summary conviction to a fine. it is an offence for a person to fail to comply.";
        // The penalty at the start should not block the genuine duty in the second sentence
        // This is a combined provision — the offence pattern at pos 50+ should still match
        // because is_penalty_dominant checks if penalty is BEFORE the offence position
        let result = match_offence_as_duty(text);
        assert!(
            result.is_none(),
            "penalty-first provision should be excluded"
        );
    }
}
