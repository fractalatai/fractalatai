//! Thing-subject obligation matcher (Rule tier).
//!
//! Detects provisions where the grammatical subject is a THING (not a person)
//! but a modal verb (must/shall) imposes an obligation. These are "rules"
//! about the state of things — e.g., "traffic routes must be suitable",
//! "a cofferdam must be of suitable design".
//!
//! This is Tier 4 in the classification cascade: after Governed v2 and
//! Government v1/v2, before Unknown.

use std::sync::LazyLock;

use regex::Regex;

use super::duty_patterns::{DutyClassification, DutyFamily, DutySubType};

// ── Thing-subject keywords ──────────────────────────────────────────

/// Inanimate/thing subjects that commonly appear as grammatical subjects
/// of obligation-bearing provisions in UK ESH legislation.
const THING_KEYWORDS: &[&str] = &[
    "arrangements",
    "routes",
    "exits",
    "equipment",
    "systems",
    "measures",
    "rooms",
    "facilities",
    "site",
    "steps",
    "structure",
    "vessel",
    "cofferdam",
    "caisson",
    "lighting",
    "precautions",
    "notice",
    "procedures",
    "timber",
    "material",
    "workplace",
    "premises",
    "device",
    "platform",
    "scaffolding",
    "scaffold",
    "plant",
    "machinery",
    "guard",
    "barrier",
    "fence",
    "ventilation",
    "temperature",
    "door",
    "gate",
    "floor",
    "surface",
    "place of work",
    "work equipment",
    "personal protective equipment",
    "excavation",
    "vehicle",
    "means of access",
    "means of egress",
    "welfare facilities",
];

// ── Modal regex ─────────────────────────────────────────────────────

/// Modal verbs indicating obligation (must/shall/is required to).
/// Does NOT include "may" — thing-subject + "may" is not a Rule.
static MODAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:shall|must|is required to|may|entitled)\b").unwrap());

static ENABLING_MODAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:may|entitled)\b").unwrap());

/// Window (in bytes) to look backwards from the modal for a thing keyword.
const THING_WINDOW: usize = 80;

// ── Person-actor keywords (negative guard) ──────────────────────────
// If any of these appear between the thing keyword and the modal,
// the provision likely belongs to Tiers 1-3, not Rule.

const PERSON_KEYWORDS: &[&str] = &[
    "employer",
    "employee",
    "person",
    "contractor",
    "worker",
    "occupier",
    "owner",
    "operator",
    "master",
    "manager",
    "duty holder",
    "responsible person",
    "competent person",
    "self-employed",
    "designer",
    "manufacturer",
    "supplier",
    "installer",
    "principal contractor",
    "client",
    // government actors
    "secretary of state",
    "minister",
    "authority",
    "commission",
    "commissioner",
    "inspector",
    "executive",
    "tribunal",
    "court",
];

// ── Public API ──────────────────────────────────────────────────────

/// Try to match a thing-subject obligation (Rule).
///
/// Algorithm:
/// 1. Find each modal verb (must/shall) in the text
/// 2. Look backwards within `THING_WINDOW` chars for a thing keyword
/// 3. Verify no person keyword appears closer to the modal than the thing
/// 4. Return `DutyClassification { family: Rule, sub_type: ThingObligation }`
pub fn match_rule(text: &str) -> Option<DutyClassification> {
    for modal_match in MODAL_RE.find_iter(text) {
        let modal_start = modal_match.start();

        // Look backwards from modal for thing keywords
        let mut window_start = modal_start.saturating_sub(THING_WINDOW);
        while window_start > 0 && !text.is_char_boundary(window_start) {
            window_start -= 1;
        }
        let before_modal = &text[window_start..modal_start];

        // Find the closest thing keyword (by position, rightmost = closest to modal)
        let mut best_thing: Option<(usize, &str)> = None; // (offset in before_modal, keyword)
        for &kw in THING_KEYWORDS {
            if let Some(pos) = before_modal.to_ascii_lowercase().rfind(kw)
                && best_thing.is_none_or(|(best_pos, _)| pos > best_pos)
            {
                best_thing = Some((pos, kw));
            }
        }

        let Some((thing_pos, _thing_kw)) = best_thing else {
            continue;
        };

        // Negative guard: check if any person keyword appears between the
        // thing keyword and the modal (closer to the modal = real subject)
        let between = &before_modal[thing_pos..].to_ascii_lowercase();
        let person_closer = PERSON_KEYWORDS.iter().any(|pk| between.contains(pk));

        if person_closer {
            continue;
        }

        // Check if the matched modal is enabling (may/entitled) → Liberty
        let sub_type = if ENABLING_MODAL_RE.is_match(modal_match.as_str()) {
            DutySubType::Enabling
        } else {
            DutySubType::ThingObligation
        };

        return Some(DutyClassification {
            family: DutyFamily::Rule,
            sub_type,
            confidence: 0.55,
            span: None,
        });
    }

    None
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traffic_routes_must_be_suitable() {
        let text = "every traffic routes must be suitable for the persons or vehicles using them";
        assert_eq!(match_rule(text).map(|dc| dc.family), Some(DutyFamily::Rule));
    }

    #[test]
    fn cofferdam_must_be_suitable() {
        let text = "a cofferdam must be of suitable design and construction";
        let result = match_rule(text).unwrap();
        assert_eq!(result.family, DutyFamily::Rule);
        assert_eq!(result.sub_type, DutySubType::ThingObligation);
    }

    #[test]
    fn equipment_must_be_provided() {
        let text = "suitable and sufficient fire-fighting equipment must be provided";
        let result = match_rule(text).unwrap();
        assert_eq!(result.family, DutyFamily::Rule);
    }

    #[test]
    fn workplace_shall_be_ventilated() {
        let text = "every workplace shall be ventilated by a sufficient quantity of fresh air";
        let result = match_rule(text).unwrap();
        assert_eq!(result.family, DutyFamily::Rule);
    }

    #[test]
    fn scaffolding_must_be_inspected() {
        let text = "scaffolding must be inspected before being taken into use";
        let result = match_rule(text).unwrap();
        assert_eq!(result.family, DutyFamily::Rule);
    }

    #[test]
    fn person_subject_not_captured_as_rule() {
        // Person-subject should not match — belongs to Governed tier
        let text = "every employer shall ensure the health, safety and welfare of employees";
        assert!(match_rule(text).is_none());
    }

    #[test]
    fn no_modal_no_rule() {
        let text = "the equipment is suitable for its intended purpose";
        assert!(match_rule(text).is_none());
    }

    #[test]
    fn person_closer_than_thing_rejects() {
        // "employer" is between "equipment" and "must" — person takes precedence
        let text = "where equipment is used, the employer must ensure it is maintained";
        assert!(match_rule(text).is_none());
    }

    #[test]
    fn government_actor_rejects() {
        let text = "the equipment used by the authority must be approved";
        assert!(match_rule(text).is_none());
    }

    #[test]
    fn lighting_shall_be_sufficient() {
        let text = "suitable and sufficient lighting shall be provided in every workplace";
        let result = match_rule(text).unwrap();
        assert_eq!(result.family, DutyFamily::Rule);
    }
}
