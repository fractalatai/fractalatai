//! Fitness rule extraction for law-level applicability.
//!
//! Parses Application+Scope provisions into structured `FitnessRule`s with:
//! - **Polarity**: applies-to / disapplies-to / extends-to
//! - **P-dimensions**: Person, Process, Place, Plant, Property, Sector
//!
//! Designed to run at enrichment time on provisions tagged with
//! `purpose::APPLICATION_SCOPE`, but also callable standalone on any text.

use std::sync::LazyLock;

use regex::Regex;

// ── Enums ────────────────────────────────────────────────────────────

/// Whether the rule includes or excludes applicability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RulePolarity {
    /// "These Regulations shall apply to..."
    AppliesTo,
    /// "These Regulations shall not apply to..."
    DisappliesTo,
    /// "These Regulations extend to..." (geographic scope)
    ExtendsTo,
}

impl RulePolarity {
    pub fn as_str(&self) -> &'static str {
        match self {
            RulePolarity::AppliesTo => "AppliesTo",
            RulePolarity::DisappliesTo => "DisappliesTo",
            RulePolarity::ExtendsTo => "ExtendsTo",
        }
    }
}

/// The 6 p-dimensions of applicability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PDimension {
    /// Who the law applies to (employer, worker, operator, etc.)
    Person,
    /// What activities trigger applicability (construction work, diving, etc.)
    Process,
    /// Where the law applies (Great Britain, offshore, mine, etc.)
    Place,
    /// What equipment/substances trigger it (asbestos, pressure systems, etc.)
    Plant,
    /// Qualifying conditions (at work, 5 or more employees, etc.)
    Property,
    /// Industry/sector (construction, mining, nuclear, etc.)
    Sector,
}

impl PDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            PDimension::Person => "Person",
            PDimension::Process => "Process",
            PDimension::Place => "Place",
            PDimension::Plant => "Plant",
            PDimension::Property => "Property",
            PDimension::Sector => "Sector",
        }
    }
}

// ── Struct ───────────────────────────────────────────────────────────

/// A single tagged match: which p-dimension, and the matched term.
#[derive(Debug, Clone, PartialEq)]
pub struct PDimensionTag {
    pub dimension: PDimension,
    pub term: String,
}

/// An applicability rule extracted from an Application+Scope provision.
#[derive(Debug, Clone, PartialEq)]
pub struct FitnessRule {
    /// Polarity: does this rule include or exclude?
    pub polarity: RulePolarity,
    /// P-dimension tags extracted from the text.
    pub tags: Vec<PDimensionTag>,
    /// The raw text this rule was extracted from.
    pub raw_text: String,
}

// ── Polarity detection ──────────────────────────────────────────────

/// Negative applicability — must match BEFORE positive to avoid
/// "shall apply" matching inside "shall not apply".
static DISAPPLIES_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:shall\s+not\s+apply|do(?:es)?\s+not\s+apply|shall\s+not\s+extend|do(?:es)?\s+not\s+extend|shall\s+have\s+no\s+effect|cease[sd]?\s+to\s+have\s+effect|shall\s+not\s+have\s+effect)"
    ).unwrap()
});

/// Geographic extension.
static EXTENDS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:extend[s]?\s+(?:only\s+)?to|shall\s+extend\s+(?:only\s+)?to|extend[s]?\s+outside)",
    )
    .unwrap()
});

/// Positive applicability.
static APPLIES_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:shall\s+apply|appl(?:y|ies)\s+(?:to|in\s+relation\s+to|where|in)|shall\s+have\s+effect|under\s+a\s+like\s+duty|shall\s+bind\s+the\s+Crown)"
    ).unwrap()
});

fn detect_polarity(text: &str) -> Option<RulePolarity> {
    // Order matters: check negative before positive
    if DISAPPLIES_RE.is_match(text) {
        return Some(RulePolarity::DisappliesTo);
    }
    if EXTENDS_RE.is_match(text) {
        return Some(RulePolarity::ExtendsTo);
    }
    if APPLIES_RE.is_match(text) {
        return Some(RulePolarity::AppliesTo);
    }
    None
}

// ── P-dimension dictionaries ────────────────────────────────────────
//
// Each dictionary is a list of (pattern, canonical_term) pairs.
// Patterns are case-insensitive word-boundary matches.

struct DictEntry {
    re: Regex,
    term: &'static str,
}

fn dict(entries: &[(&str, &'static str)]) -> Vec<DictEntry> {
    entries
        .iter()
        .map(|(pat, term)| DictEntry {
            re: Regex::new(&format!(r"(?i)\b{}\b", pat)).unwrap(),
            term,
        })
        .collect()
}

static PERSON_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        // Compound terms first (longer matches before shorter)
        ("self[- ]employed\\s+person", "self-employed person"),
        ("competent\\s+person", "competent person"),
        ("responsible\\s+person", "responsible person"),
        ("appointed\\s+person", "appointed person"),
        ("duty\\s+holder", "duty holder"),
        ("agency\\s+worker", "agency worker"),
        ("person\\s+at\\s+work", "person at work"),
        ("persons?\\s+(?:at\\s+)?work", "person at work"),
        ("sub[- ]?contractor", "sub-contractor"),
        // Single-word terms
        ("employer(?:s)?", "employer"),
        ("employee(?:s)?", "employee"),
        ("worker(?:s)?", "worker"),
        ("contractor(?:s)?", "contractor"),
        ("operator(?:s)?", "operator"),
        ("manufacturer(?:s)?", "manufacturer"),
        ("supplier(?:s)?", "supplier"),
        ("importer(?:s)?", "importer"),
        ("occupier(?:s)?", "occupier"),
        ("owner(?:s)?", "owner"),
        ("installer(?:s)?", "installer"),
        ("designer(?:s)?", "designer"),
        (
            "master(?:s)?\\s+(?:of|or)\\s+(?:the\\s+|a\\s+)?(?:sea[- ]going\\s+)?ship",
            "master of ship",
        ),
        ("master\\s+or\\s+crew", "master of ship"),
        ("crew", "crew"),
    ])
});

static PROCESS_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("construction\\s+work", "construction work"),
        ("diving\\s+(?:operation|project|work)", "diving operations"),
        ("mining\\s+operation", "mining operations"),
        ("quarrying", "quarrying"),
        (
            "handling\\s+(?:of\\s+)?dangerous\\s+substances?",
            "handling dangerous substances",
        ),
        ("loading\\s+(?:or|and)\\s+unloading", "loading/unloading"),
        ("manual\\s+handling", "manual handling"),
        ("work\\s+at\\s+height", "work at height"),
        (
            "work\\s+with\\s+display\\s+screen",
            "work with display screens",
        ),
        (
            "transport(?:ation)?\\s+(?:by\\s+)?(?:road|rail|sea|air|waterway)",
            "transport",
        ),
        ("health\\s+surveillance", "health surveillance"),
        ("risk\\s+assessment", "risk assessment"),
        ("asbestos\\s+work", "asbestos work"),
        ("work\\s+with\\s+lead", "lead work"),
        (
            "work\\s+with\\s+(?:ionising\\s+)?radiation",
            "radiation work",
        ),
        ("petroleum\\s+(?:operation|activit)", "petroleum operations"),
        ("electrical\\s+work", "electrical work"),
        ("gas\\s+(?:fitting|supply|work|installation)", "gas work"),
        ("work\\s+(?:with|involving)\\s+explosive", "explosives work"),
        ("noise\\s+(?:exposure|assessment)", "noise exposure"),
        ("vibration\\s+(?:exposure|assessment)", "vibration exposure"),
        ("pressure\\s+system", "pressure systems work"),
    ])
});

static PLACE_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        // Compound/specific before general
        ("outside\\s+Great\\s+Britain", "outside Great Britain"),
        ("Great\\s+Britain", "Great Britain"),
        ("Northern\\s+Ireland", "Northern Ireland"),
        ("United\\s+Kingdom", "United Kingdom"),
        ("offshore\\s+installation", "offshore installation"),
        ("territorial\\s+(?:sea|waters?)", "territorial sea"),
        ("continental\\s+shelf", "continental shelf"),
        ("construction\\s+site", "construction site"),
        ("England(?:\\s+and\\s+Wales)?", "England"),
        ("Wales", "Wales"),
        ("Scotland", "Scotland"),
        ("offshore", "offshore"),
        ("mine(?:s)?", "mine"),
        ("quarr(?:y|ies)", "quarry"),
        ("factor(?:y|ies)", "factory"),
        ("premises", "premises"),
        ("workplace(?:s)?", "workplace"),
        ("ship(?:s)?", "ship"),
        ("aircraft", "aircraft"),
    ])
});

static PLANT_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("dangerous\\s+substance(?:s)?", "dangerous substances"),
        ("display\\s+screen\\s+equipment", "display screen equipment"),
        (
            "personal\\s+protective\\s+equipment",
            "personal protective equipment",
        ),
        ("work\\s+equipment", "work equipment"),
        ("pressure\\s+system(?:s)?", "pressure systems"),
        ("dangerous\\s+goods", "dangerous goods"),
        ("gas\\s+fitting(?:s)?", "gas fittings"),
        ("ionising\\s+radiation", "ionising radiation"),
        ("biological\\s+agent(?:s)?", "biological agents"),
        ("asbestos", "asbestos"),
        ("lead", "lead"),
        ("explosive(?:s)?", "explosives"),
        ("petroleum", "petroleum"),
        ("chemical(?:s)?", "chemicals"),
    ])
});

static PROPERTY_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("at\\s+work", "at work"),
        (
            "(?:five|5)\\s+or\\s+more\\s+employee",
            "5 or more employees",
        ),
        (
            "sporadic\\s+and\\s+low\\s+intensity",
            "sporadic and low intensity",
        ),
        (
            "carried\\s+out\\s+solely\\s+by\\s+(?:the\\s+)?crew",
            "carried out solely by crew",
        ),
        (
            "normal\\s+ship[- ]?board\\s+activiti?es",
            "normal shipboard activities",
        ),
        (
            "not\\s+(?:liable|likely)\\s+to\\s+expose",
            "not liable to expose persons",
        ),
        ("not\\s+in\\s+prolonged\\s+use", "not in prolonged use"),
        ("on\\s+board\\s+(?:a\\s+)?transport", "on board transport"),
        (
            "in\\s+the\\s+public\\s+service\\s+of\\s+the\\s+Crown",
            "Crown service",
        ),
    ])
});

static SECTOR_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        (
            "construction\\s+(?:industry|sector|project)",
            "construction",
        ),
        ("mining\\s+(?:industry|sector|operation)", "mining"),
        (
            "offshore\\s+(?:oil|gas|petroleum|industry)",
            "offshore oil & gas",
        ),
        (
            "nuclear\\s+(?:industry|sector|installation|site)",
            "nuclear",
        ),
        ("chemical\\s+(?:industry|sector)", "chemicals"),
        ("petroleum\\s+(?:industry|sector)", "petroleum"),
        ("gas\\s+(?:industry|sector|distribution)", "gas supply"),
        ("maritime", "maritime"),
        ("shipping", "maritime"),
        ("agricultur", "agriculture"),
        ("waste\\s+(?:management|disposal)", "waste management"),
        ("water\\s+(?:industry|undertaker|supply)", "water industry"),
    ])
});

// ── Public API ───────────────────────────────────────────────────────

/// Extract fitness rules from an Application+Scope provision text.
///
/// Returns a vec because some provisions contain both applies-to and
/// disapplies-to clauses (e.g. "shall not apply to X, but shall apply to Y").
///
/// Returns an empty vec if no polarity pattern is detected.
pub fn extract(text: &str) -> Vec<FitnessRule> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }

    // Check for compound provisions: "shall not apply... but shall apply..."
    // Split on " but " when both polarities are present.
    if DISAPPLIES_RE.is_match(text)
        && APPLIES_RE.is_match(text)
        && let Some(rules) = try_split_compound(text)
    {
        return rules;
    }

    // Single-polarity provision
    let polarity = match detect_polarity(text) {
        Some(p) => p,
        None => return vec![],
    };

    let tags = extract_tags(text);
    vec![FitnessRule {
        polarity,
        tags,
        raw_text: text.to_string(),
    }]
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Try to split a compound provision into separate applies/disapplies rules.
fn try_split_compound(text: &str) -> Option<Vec<FitnessRule>> {
    // Look for "but" or "except that" or "save that" as split points
    let split_re = LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:but\s+(?:they\s+)?shall|except\s+that|save\s+that)").unwrap()
    });

    let m = split_re.find(text)?;
    let part_a = &text[..m.start()];
    let part_b = &text[m.start()..];

    let pol_a = detect_polarity(part_a)?;
    let pol_b = detect_polarity(part_b)?;

    // Only split if we get different polarities
    if pol_a == pol_b {
        return None;
    }

    let tags_a = extract_tags(part_a);
    let tags_b = extract_tags(part_b);

    Some(vec![
        FitnessRule {
            polarity: pol_a,
            tags: tags_a,
            raw_text: part_a.trim().to_string(),
        },
        FitnessRule {
            polarity: pol_b,
            tags: tags_b,
            raw_text: part_b.trim().to_string(),
        },
    ])
}

/// Extract all p-dimension tags from text.
fn extract_tags(text: &str) -> Vec<PDimensionTag> {
    let mut tags = Vec::new();

    let dicts: &[(PDimension, &[DictEntry])] = &[
        (PDimension::Person, &PERSON_DICT),
        (PDimension::Process, &PROCESS_DICT),
        (PDimension::Place, &PLACE_DICT),
        (PDimension::Plant, &PLANT_DICT),
        (PDimension::Property, &PROPERTY_DICT),
        (PDimension::Sector, &SECTOR_DICT),
    ];

    for (dim, dict) in dicts {
        for entry in *dict {
            if entry.re.is_match(text) {
                // Avoid duplicates for the same term in the same dimension
                let already = tags
                    .iter()
                    .any(|t: &PDimensionTag| t.dimension == *dim && t.term == entry.term);
                if !already {
                    tags.push(PDimensionTag {
                        dimension: *dim,
                        term: entry.term.to_string(),
                    });
                }
            }
        }
    }

    tags
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: check that a rule has a tag with the given dimension and term.
    fn has_tag(rule: &FitnessRule, dim: PDimension, term: &str) -> bool {
        rule.tags
            .iter()
            .any(|t| t.dimension == dim && t.term == term)
    }

    // ── Polarity detection ──────────────────────────────────────────

    #[test]
    fn positive_applies_to() {
        let text =
            "These Regulations shall apply to every employer and every self-employed person.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::AppliesTo);
        assert!(has_tag(&rules[0], PDimension::Person, "employer"));
        assert!(has_tag(
            &rules[0],
            PDimension::Person,
            "self-employed person"
        ));
    }

    #[test]
    fn negative_disapplies() {
        let text = "These Regulations shall not apply to or in relation to the master or crew of a sea-going ship.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
        assert!(has_tag(&rules[0], PDimension::Person, "master of ship"));
        assert!(has_tag(&rules[0], PDimension::Person, "crew"));
    }

    #[test]
    fn does_not_apply() {
        let text =
            "This regulation does not apply to construction work carried out by the armed forces.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
        assert!(has_tag(&rules[0], PDimension::Process, "construction work"));
    }

    #[test]
    fn extends_to() {
        let text = "These Regulations extend to Northern Ireland.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::ExtendsTo);
        assert!(has_tag(&rules[0], PDimension::Place, "Northern Ireland"));
    }

    // ── Geographic patterns ─────────────────────────────────────────

    #[test]
    fn geographic_applies_to_england() {
        let text = "These Regulations apply to England only.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::AppliesTo);
        assert!(has_tag(&rules[0], PDimension::Place, "England"));
    }

    #[test]
    fn geographic_great_britain() {
        let text = "These Regulations shall apply in Great Britain.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Place, "Great Britain"));
    }

    #[test]
    fn extends_outside_gb() {
        let text = "These Regulations extend to outside Great Britain as sections 1 to 59 and 80 to 82 of the 1974 Act apply.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::ExtendsTo);
        assert!(has_tag(
            &rules[0],
            PDimension::Place,
            "outside Great Britain"
        ));
    }

    // ── Person patterns ─────────────────────────────────────────────

    #[test]
    fn self_employed_extension() {
        let text = "These Regulations shall apply to a self-employed person as they apply to an employer and an employee.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::AppliesTo);
        assert!(has_tag(
            &rules[0],
            PDimension::Person,
            "self-employed person"
        ));
        assert!(has_tag(&rules[0], PDimension::Person, "employer"));
        assert!(has_tag(&rules[0], PDimension::Person, "employee"));
    }

    #[test]
    fn like_duty() {
        let text = "The employer shall, so far as is reasonably practicable, be under a like duty in respect of any other person at work.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::AppliesTo);
        assert!(has_tag(&rules[0], PDimension::Person, "employer"));
        assert!(has_tag(&rules[0], PDimension::Person, "person at work"));
    }

    #[test]
    fn crown_application() {
        let text = "The provisions of these Regulations shall apply to persons in the public service of the Crown as they apply to other persons.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Property, "Crown service"));
    }

    // ── Process/activity patterns ───────────────────────────────────

    #[test]
    fn activity_scoped() {
        let text = "These Regulations apply to construction work.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Process, "construction work"));
    }

    #[test]
    fn diving_operations() {
        let text = "These Regulations shall apply to and in relation to any diving project.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Process, "diving operations"));
    }

    // ── Place patterns ──────────────────────────────────────────────

    #[test]
    fn quarry_scope() {
        let text = "These Regulations shall apply to all quarries where persons work.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Place, "quarry"));
        assert!(has_tag(&rules[0], PDimension::Person, "person at work"));
    }

    #[test]
    fn mine_exclusion() {
        let text = "These Regulations shall not apply to any place below ground in a mine.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
        assert!(has_tag(&rules[0], PDimension::Place, "mine"));
    }

    // ── Compound provisions ─────────────────────────────────────────

    #[test]
    fn compound_disapplies_then_applies() {
        let text = "These Regulations shall not apply in relation to such premises, but they shall apply in relation to premises used for domestic purposes.";
        let rules = extract(text);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
        assert_eq!(rules[1].polarity, RulePolarity::AppliesTo);
        assert!(has_tag(&rules[0], PDimension::Place, "premises"));
        assert!(has_tag(&rules[1], PDimension::Place, "premises"));
    }

    // ── Plant patterns ──────────────────────────────────────────────

    #[test]
    fn gas_fittings() {
        let text = "These Regulations shall apply to gas fittings used in connection with gas which has been conveyed through a distribution main.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Plant, "gas fittings"));
    }

    #[test]
    fn dangerous_substances() {
        let text =
            "These Regulations apply where a dangerous substance is present at the workplace.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(
            &rules[0],
            PDimension::Plant,
            "dangerous substances"
        ));
        assert!(has_tag(&rules[0], PDimension::Place, "workplace"));
    }

    // ── Property patterns ───────────────────────────────────────────

    #[test]
    fn ship_board_activities() {
        let text = "These Regulations shall not apply to normal ship-board activities of a ship's crew under the direction of the master.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
        assert!(has_tag(
            &rules[0],
            PDimension::Property,
            "normal shipboard activities"
        ));
        assert!(has_tag(&rules[0], PDimension::Person, "crew"));
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn empty_returns_empty() {
        assert!(extract("").is_empty());
        assert!(extract("   ").is_empty());
    }

    #[test]
    fn no_polarity_returns_empty() {
        let text = "Citation, commencement and interpretation.";
        assert!(extract(text).is_empty());
    }

    #[test]
    fn ceases_to_have_effect() {
        let text = "This regulation ceases to have effect on 1st April 2025.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
    }

    #[test]
    fn bind_the_crown() {
        let text = "This Act shall bind the Crown.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::AppliesTo);
    }

    #[test]
    fn multiple_places() {
        let text = "These Regulations apply in relation to England and Wales.";
        let rules = extract(text);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Place, "England"));
        assert!(has_tag(&rules[0], PDimension::Place, "Wales"));
    }
}
