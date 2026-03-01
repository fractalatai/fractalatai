//! Clause structure decomposition for DRRP-classified provisions.
//!
//! Decomposes a `clause_refined` string into structured components:
//! - **applicability**: conditional preamble (free text)
//! - **modal**: obligation strength (8-value enum)
//! - **qualifiers**: standard legal modifiers (enum tags)
//! - **action**: the core obligation (free text)
//!
//! Designed to run at enrichment time (with `MatchSpan` for precision) or
//! standalone on stored clause text (regex fallback for modal detection).

use std::sync::LazyLock;

use regex::Regex;

use super::duty_patterns::MatchSpan;

// ── Enums ────────────────────────────────────────────────────────────

/// Modal verb indicating the nature of the obligation.
/// 8 values — captures obligation strength only, not the action verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modal {
    Shall,
    Must,
    May,
    ShallNot,
    MustNot,
    MayNot,
    IsRequiredTo,
    HasDutyTo,
}

impl Modal {
    pub fn as_str(&self) -> &'static str {
        match self {
            Modal::Shall => "Shall",
            Modal::Must => "Must",
            Modal::May => "May",
            Modal::ShallNot => "ShallNot",
            Modal::MustNot => "MustNot",
            Modal::MayNot => "MayNot",
            Modal::IsRequiredTo => "IsRequiredTo",
            Modal::HasDutyTo => "HasDutyTo",
        }
    }
}

/// Standard legal qualifier/modifier on the obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Qualifier {
    // Standard of care
    Sfairp,
    Sfaip,
    SuitableAndSufficient,
    AdequateAndAppropriate,
    // Timing
    Immediately,
    AsSoonAsReasonablyPracticable,
    AsSoonAsPracticable,
    WithoutDelay,
    // Conditionality
    WhereNecessary,
    WhereAppropriate,
    // Form
    InWriting,
    // Scope
    InSoFarAs,
}

impl Qualifier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Qualifier::Sfairp => "Sfairp",
            Qualifier::Sfaip => "Sfaip",
            Qualifier::SuitableAndSufficient => "SuitableAndSufficient",
            Qualifier::AdequateAndAppropriate => "AdequateAndAppropriate",
            Qualifier::Immediately => "Immediately",
            Qualifier::AsSoonAsReasonablyPracticable => "AsSoonAsReasonablyPracticable",
            Qualifier::AsSoonAsPracticable => "AsSoonAsPracticable",
            Qualifier::WithoutDelay => "WithoutDelay",
            Qualifier::WhereNecessary => "WhereNecessary",
            Qualifier::WhereAppropriate => "WhereAppropriate",
            Qualifier::InWriting => "InWriting",
            Qualifier::InSoFarAs => "InSoFarAs",
        }
    }
}

// ── Struct ───────────────────────────────────────────────────────────

/// Decomposed structure of a legal clause.
/// Actor data comes from existing `governed_actors`/`government_actors` columns.
#[derive(Debug, Clone, PartialEq)]
pub struct ClauseStructure {
    /// Conditional preamble, if any ("Where X applies...", "Subject to Y...").
    pub applicability: Option<String>,
    /// The modal verb — 8 fixed values.
    pub modal: Modal,
    /// Standard legal qualifiers modifying the obligation.
    pub qualifiers: Vec<Qualifier>,
    /// The core obligation/action (free text).
    pub action: String,
}

// ── Regex patterns ───────────────────────────────────────────────────

/// Modal verb pattern — ordered so negated forms match before positive.
static MODAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(shall\s+not|must\s+not|may\s+not|shall|must|may|(?:is|are)\s+required\s+to|has\s+a\s+duty\s+to)\b",
    )
    .unwrap()
});

/// Reverse-duty pattern: "It shall be the duty of [actor] to [action]".
static REVERSE_DUTY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[Ii]t\s+shall\s+be\s+the\s+duty\s+of\s+(.+?)\s+to\s+(.+)").unwrap()
});

/// Conditional preamble — starts with Where/When/If/Subject to, ends at the
/// last comma before the modal verb. The text after the comma is typically
/// an actor phrase like "the employer", "an operator", "each person".
static CONDITIONAL_START_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:Where|When|If|Subject to)\b").unwrap());

/// Qualifier patterns — ordered most-specific first to avoid substring matches.
/// Each tuple: (regex, Qualifier variant).
static QUALIFIER_PATTERNS: LazyLock<Vec<(Regex, Qualifier)>> = LazyLock::new(|| {
    vec![
        // Standard of care (SFAIRP before SFAIP — longer match first)
        (
            Regex::new(r"(?i)so far as is reasonably practicable").unwrap(),
            Qualifier::Sfairp,
        ),
        (
            Regex::new(r"(?i)so far as is practicable").unwrap(),
            Qualifier::Sfaip,
        ),
        (
            Regex::new(r"(?i)suitable and sufficient").unwrap(),
            Qualifier::SuitableAndSufficient,
        ),
        (
            Regex::new(r"(?i)adequate and appropriate").unwrap(),
            Qualifier::AdequateAndAppropriate,
        ),
        // Timing (ASARP before ASAP — longer match first)
        (
            Regex::new(r"(?i)as soon as (?:is )?reasonably practicable").unwrap(),
            Qualifier::AsSoonAsReasonablyPracticable,
        ),
        (
            Regex::new(r"(?i)as soon as (?:is )?practicable").unwrap(),
            Qualifier::AsSoonAsPracticable,
        ),
        (
            Regex::new(r"(?i)\bimmediately\b").unwrap(),
            Qualifier::Immediately,
        ),
        (
            Regex::new(r"(?i)without (?:undue |unreasonable )?delay").unwrap(),
            Qualifier::WithoutDelay,
        ),
        // Conditionality
        (
            Regex::new(r"(?i)\bwhere necessary\b").unwrap(),
            Qualifier::WhereNecessary,
        ),
        (
            Regex::new(r"(?i)\bwhere appropriate\b").unwrap(),
            Qualifier::WhereAppropriate,
        ),
        // Form
        (
            Regex::new(r"(?i)\bin writing\b").unwrap(),
            Qualifier::InWriting,
        ),
        // Scope (after SFAIRP/SFAIP to avoid partial matches)
        (
            Regex::new(r"(?i)(?:in )?so far as(?: is necessary)?").unwrap(),
            Qualifier::InSoFarAs,
        ),
    ]
});

// ── Public API ───────────────────────────────────────────────────────

/// Decompose a clause into structured components.
///
/// `span` is optional — when available (enrichment time), gives precise byte
/// offsets for the modal verb. Without it (standalone mode), falls back to
/// regex modal detection.
///
/// Returns `None` if no modal verb can be found in the clause.
pub fn decompose(clause: &str, span: Option<MatchSpan>) -> Option<ClauseStructure> {
    let clause = clause.trim();
    if clause.is_empty() {
        return None;
    }

    // Try reverse-duty pattern first (P4) — "It shall be the duty of X to Y"
    if let Some(cs) = try_reverse_duty(clause) {
        return Some(cs);
    }

    // Find the modal verb position — use span if available, else regex
    let (modal, modal_start, modal_end) = find_modal(clause, span)?;

    // Extract applicability: conditional preamble before the actor/modal
    let applicability = extract_applicability(clause, modal_start);

    // Extract action: everything after the modal
    let action_raw = clause[modal_end..].trim();
    let action = clean_action(action_raw);

    if action.is_empty() {
        return None;
    }

    // Extract qualifiers from the full clause text
    let qualifiers = extract_qualifiers(clause);

    Some(ClauseStructure {
        applicability,
        modal,
        qualifiers,
        action,
    })
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Try to parse "It shall be the duty of [actor] to [action]".
fn try_reverse_duty(clause: &str) -> Option<ClauseStructure> {
    let caps = REVERSE_DUTY_RE.captures(clause)?;
    let action_raw = caps.get(2)?.as_str().trim();
    let action = clean_action(action_raw);

    if action.is_empty() {
        return None;
    }

    let qualifiers = extract_qualifiers(clause);

    Some(ClauseStructure {
        applicability: None,
        modal: Modal::HasDutyTo,
        qualifiers,
        action,
    })
}

/// Find the modal verb in the clause.
/// Prefers span data (precise) over regex (fallback).
fn find_modal(clause: &str, span: Option<MatchSpan>) -> Option<(Modal, usize, usize)> {
    if let Some(sp) = span {
        // Use span positions to extract the modal text
        let modal_text = &clause[sp.modal_start..sp.modal_end];
        let modal = classify_modal(modal_text)?;
        return Some((modal, sp.modal_start, sp.modal_end));
    }

    // Regex fallback for standalone mode
    let m = MODAL_RE.find(clause)?;
    let modal = classify_modal(m.as_str())?;
    Some((modal, m.start(), m.end()))
}

/// Classify a modal verb string into the enum.
fn classify_modal(text: &str) -> Option<Modal> {
    let lower = text.to_lowercase();
    let normalised: String = lower.split_whitespace().collect::<Vec<_>>().join(" ");

    match normalised.as_str() {
        "shall not" => Some(Modal::ShallNot),
        "must not" => Some(Modal::MustNot),
        "may not" => Some(Modal::MayNot),
        "shall" => Some(Modal::Shall),
        "must" => Some(Modal::Must),
        "may" => Some(Modal::May),
        "is required to" | "are required to" => Some(Modal::IsRequiredTo),
        "has a duty to" => Some(Modal::HasDutyTo),
        _ => None,
    }
}

/// Extract the conditional preamble (applicability) from before the modal.
///
/// Looks for "Where/When/If/Subject to" at clause start, then splits at the
/// last comma before the modal verb to isolate the conditional from the
/// actor phrase (e.g. "the employer").
fn extract_applicability(clause: &str, modal_start: usize) -> Option<String> {
    // Must start with a conditional keyword
    if !CONDITIONAL_START_RE.is_match(clause) {
        return None;
    }

    // Find the last comma before the modal
    let before_modal = &clause[..modal_start];
    let comma_pos = before_modal.rfind(',')?;

    let preamble = before_modal[..comma_pos].trim();
    if preamble.is_empty() {
        return None;
    }

    Some(preamble.to_string())
}

/// Extract all matching qualifiers from the clause text.
fn extract_qualifiers(clause: &str) -> Vec<Qualifier> {
    let mut found = Vec::new();
    for (re, qual) in QUALIFIER_PATTERNS.iter() {
        if re.is_match(clause) && !found.contains(qual) {
            found.push(*qual);
        }
    }
    found
}

/// Clean the action text — trim leading punctuation/whitespace artifacts.
fn clean_action(raw: &str) -> String {
    let trimmed = raw
        .trim_start_matches(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .trim_end();
    trimmed.to_string()
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_shall() {
        let clause =
            "Every employer shall ensure that personal protective equipment is compatible.";
        let cs = decompose(clause, None).unwrap();
        assert_eq!(cs.modal, Modal::Shall);
        assert!(cs.applicability.is_none());
        assert!(cs.qualifiers.is_empty());
        assert!(cs.action.starts_with("ensure that"));
    }

    #[test]
    fn direct_must_not() {
        let clause = "A person must not damage, interfere with, or obstruct an eel pass.";
        let cs = decompose(clause, None).unwrap();
        assert_eq!(cs.modal, Modal::MustNot);
        assert!(cs.applicability.is_none());
        assert!(cs.action.starts_with("damage"));
    }

    #[test]
    fn qualified_sfairp() {
        let clause = "Each employer shall ensure, so far as is reasonably practicable, the safety of employees.";
        let cs = decompose(clause, None).unwrap();
        assert_eq!(cs.modal, Modal::Shall);
        assert!(cs.qualifiers.contains(&Qualifier::Sfairp));
        assert!(cs.action.starts_with("ensure"));
    }

    #[test]
    fn conditional_lead() {
        let clause = "Where a dangerous substance is present at the workplace, the employer shall make a suitable and sufficient assessment of the risks.";
        let cs = decompose(clause, None).unwrap();
        assert_eq!(cs.modal, Modal::Shall);
        assert!(cs.applicability.is_some());
        assert!(
            cs.applicability
                .as_ref()
                .unwrap()
                .starts_with("Where a dangerous substance")
        );
        assert!(cs.qualifiers.contains(&Qualifier::SuitableAndSufficient));
        assert!(cs.action.starts_with("make a"));
    }

    #[test]
    fn reverse_duty() {
        let clause = "It shall be the duty of each licensing authority to establish and maintain a register.";
        let cs = decompose(clause, None).unwrap();
        assert_eq!(cs.modal, Modal::HasDutyTo);
        assert!(cs.action.starts_with("establish and maintain"));
    }

    #[test]
    fn with_span() {
        let clause = "The operator shall immediately notify the Agency of any obstruction.";
        // "The operator shall..."
        //  0123456789012345678
        //  ^actor=4   ^modal=13..18
        let span = MatchSpan {
            actor_start: 4,
            modal_start: 13,
            modal_end: 18,
        };
        let cs = decompose(clause, Some(span)).unwrap();
        assert_eq!(cs.modal, Modal::Shall);
        assert!(cs.qualifiers.contains(&Qualifier::Immediately));
    }

    #[test]
    fn multiple_qualifiers() {
        let clause = "The employer shall immediately provide, in writing, a suitable and sufficient assessment.";
        let cs = decompose(clause, None).unwrap();
        assert_eq!(cs.modal, Modal::Shall);
        assert!(cs.qualifiers.contains(&Qualifier::Immediately));
        assert!(cs.qualifiers.contains(&Qualifier::InWriting));
        assert!(cs.qualifiers.contains(&Qualifier::SuitableAndSufficient));
    }

    #[test]
    fn is_required_to() {
        let clause = "The operator is required to maintain records of all inspections.";
        let cs = decompose(clause, None).unwrap();
        assert_eq!(cs.modal, Modal::IsRequiredTo);
        assert!(cs.action.starts_with("maintain records"));
    }

    #[test]
    fn may_permissive() {
        let clause = "An operator may apply to the regulator for the variation of conditions.";
        let cs = decompose(clause, None).unwrap();
        assert_eq!(cs.modal, Modal::May);
        assert!(cs.action.starts_with("apply to"));
    }

    #[test]
    fn empty_clause_returns_none() {
        assert!(decompose("", None).is_none());
        assert!(decompose("   ", None).is_none());
    }

    #[test]
    fn no_modal_returns_none() {
        assert!(decompose("Citation and commencement.", None).is_none());
    }

    #[test]
    fn conditional_with_qualifier() {
        let clause = "Where the employer employs five or more employees, the employer shall record, in writing, the significant findings of the assessment.";
        let cs = decompose(clause, None).unwrap();
        assert!(cs.applicability.is_some());
        assert!(cs.qualifiers.contains(&Qualifier::InWriting));
        assert_eq!(cs.modal, Modal::Shall);
    }
}
