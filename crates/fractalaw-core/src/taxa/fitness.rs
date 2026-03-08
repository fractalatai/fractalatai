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
    /// Cross-references to other provisions detected in the text.
    pub cross_refs: Vec<String>,
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

/// Detect references to other provisions (regulation N, paragraph N, schedule N, etc.).
static CROSS_REF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:regulations?|paragraphs?|sub-paragraphs?|sections?|articles?|schedules?|parts?)\s+[\d(]+[\d().a-z]*",
    )
    .unwrap()
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

fn detect_cross_refs(text: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    CROSS_REF_RE
        .find_iter(text)
        .filter_map(|m| {
            let s = m.as_str().to_string();
            seen.insert(s.to_lowercase()).then_some(s)
        })
        .collect()
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
        (
            "in\\s+the\\s+public\\s+service\\s+of\\s+the\\s+Crown",
            "Crown service",
        ),
        ("(?:bind|apply\\s+to)\\s+the\\s+Crown", "Crown application"),
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
        (
            "carriage\\s+(?:of\\s+)?(?:dangerous\\s+)?(?:goods|class\\s+\\d)",
            "carriage of goods",
        ),
        ("national\\s+carriage", "carriage of goods"),
        (
            "carriage\\s+by\\s+(?:road|rail|sea|air|vehicles?)",
            "carriage of goods",
        ),
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
        ("establishment(?:s)?", "establishment"),
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
        ("\\bPPE\\b", "personal protective equipment"),
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

// ── OH&S Specialist Dictionaries ─────────────────────────────────────
//
// Terms specific to Occupational Health & Safety law families.
// Applied only when the law's family starts with "OH&S".

static OHS_PERSON_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("young\\s+person(?:s)?", "young person"),
        ("new\\s+or\\s+expectant\\s+moth", "new or expectant mother"),
        ("principal\\s+contractor", "principal contractor"),
        ("principal\\s+designer", "principal designer"),
        ("domestic\\s+client(?:s)?", "domestic client"),
        ("client(?:s)?", "client"),
    ])
});

static OHS_PROCESS_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("lifting\\s+operation(?:s)?", "lifting operations"),
        ("confined\\s+space(?:s)?", "confined spaces"),
        ("provision\\s+and\\s+use", "provision and use"),
        ("working\\s+at\\s+height", "work at height"),
        ("work(?:ing)?\\s+near\\s+voltage", "work near voltage"),
    ])
});

static OHS_PLANT_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("pressure\\s+equipment", "pressure equipment"),
        ("lifting\\s+equipment", "lifting equipment"),
        ("machiner(?:y|ies)", "machinery"),
        ("lift(?:s)?(?:\\s+and\\s+escalator)?", "lifts"),
        ("electrical\\s+equipment", "electrical equipment"),
        ("scaffold(?:s|ing)?", "scaffolding"),
        ("safety\\s+sign(?:s)?", "safety signs"),
        ("first[- ]?aid", "first-aid"),
        (
            "door[,\\s]+gate[,\\s]+(?:or\\s+)?hatch",
            "door, gate or hatch",
        ),
    ])
});

static OHS_PROCESS_EXT_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("fumigat(?:ions?|ants?|ing)", "fumigation"),
        ("relevant\\s+project", "construction project"),
    ])
});

// ── FIRE Specialist Dictionaries ─────────────────────────────────────
//
// Terms specific to Fire Safety and Dangerous/Explosive Substances.
// Applied when the law's family starts with "FIRE".

static FIRE_PERSON_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        ("fire\\s+(?:and\\s+rescue\\s+)?authority", "fire authority"),
        ("enforcing\\s+authority", "enforcing authority"),
        ("licensing\\s+authority", "licensing authority"),
        ("Chief\\s+Inspector", "chief inspector"),
    ])
});

static FIRE_PROCESS_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        (
            "transport(?:ing|ation)?\\s+(?:of\\s+)?(?:fireworks|explosives|ammunition|dangerous\\s+goods)",
            "transporting dangerous goods",
        ),
        (
            "transfer\\s+(?:of\\s+)?(?:any\\s+)?(?:component|explosives?|ammunition)",
            "transfer of explosives",
        ),
        (
            "manufacture\\s+(?:or\\s+)?(?:compression|storage|keeping)",
            "manufacture of explosives",
        ),
        (
            "filling\\s+(?:of\\s+)?(?:any\\s+)?cylinder",
            "filling of cylinders",
        ),
        (
            "storage\\s+of\\s+(?:explosives?|fireworks?|ammunition)",
            "storage of explosives",
        ),
    ])
});

static FIRE_PLACE_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        // Cross-family candidate: also relevant to MARITIME, PORT etc.
        ("(?:nuclear\\s+)?harbour\\s+area", "harbour area"),
        ("harbour(?:s)?", "harbour"),
        ("licensed\\s+premises", "licensed premises"),
        ("petroleum\\s+store", "petroleum store"),
        ("magazine(?:s)?", "magazine"),
    ])
});

static FIRE_PLANT_DICT: LazyLock<Vec<DictEntry>> = LazyLock::new(|| {
    dict(&[
        // Compound terms first
        ("small\\s+arms\\s+ammunition", "small arms ammunition"),
        ("pyrotechnic\\s+article(?:s)?", "pyrotechnic articles"),
        ("ammonium\\s+nitrate(?:\\s+blasting)?", "ammonium nitrate"),
        ("fire\\s+alarm(?:s)?", "fire alarm"),
        ("smoke\\s+detector(?:s)?", "smoke detector"),
        ("sprinkler(?:s)?(?:\\s+system)?", "sprinkler system"),
        ("fire\\s+extinguisher(?:s)?", "fire extinguisher"),
        ("firework(?:s)?", "fireworks"),
        ("ammunition", "ammunition"),
        ("detonator(?:s)?", "detonator"),
    ])
});

/// Return specialist dictionaries for a given law family.
///
/// Currently OH&S and FIRE families have specialists. Returns empty vec
/// for unknown families — the core dictionaries still run.
fn specialist_dicts_for(family: &str) -> Vec<(PDimension, &'static [DictEntry])> {
    if family.starts_with("OH&S") {
        vec![
            (PDimension::Person, &OHS_PERSON_DICT),
            (PDimension::Process, &OHS_PROCESS_DICT),
            (PDimension::Process, &OHS_PROCESS_EXT_DICT),
            (PDimension::Plant, &OHS_PLANT_DICT),
        ]
    } else if family.starts_with("FIRE") {
        vec![
            (PDimension::Person, &FIRE_PERSON_DICT),
            (PDimension::Process, &FIRE_PROCESS_DICT),
            (PDimension::Place, &FIRE_PLACE_DICT),
            (PDimension::Plant, &FIRE_PLANT_DICT),
        ]
    } else {
        vec![]
    }
}

// ── Public API ───────────────────────────────────────────────────────

/// Extract fitness rules from an Application+Scope provision text.
///
/// Returns a vec because some provisions contain both applies-to and
/// disapplies-to clauses (e.g. "shall not apply to X, but shall apply to Y").
///
/// Returns an empty vec if no polarity pattern is detected.
pub fn extract(text: &str, family: Option<&str>) -> Vec<FitnessRule> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }

    // Check for compound provisions: "shall not apply... but shall apply..."
    // Split on " but " when both polarities are present.
    if DISAPPLIES_RE.is_match(text)
        && APPLIES_RE.is_match(text)
        && let Some(rules) = try_split_compound(text, family)
    {
        return rules;
    }

    // Single-polarity provision
    let polarity = match detect_polarity(text) {
        Some(p) => p,
        None => return vec![],
    };

    let tags = extract_tags(text, family);
    let cross_refs = detect_cross_refs(text);
    vec![FitnessRule {
        polarity,
        tags,
        raw_text: text.to_string(),
        cross_refs,
    }]
}

/// Return all canonical terms from core + optional specialist dictionaries.
///
/// Used by audit tooling to filter known terms from candidate suggestions.
/// When `family` is `Some`, includes specialist terms for that family.
pub fn all_canonical_terms(family: Option<&str>) -> Vec<&'static str> {
    let dicts: &[&[DictEntry]] = &[
        &PERSON_DICT,
        &PROCESS_DICT,
        &PLACE_DICT,
        &PLANT_DICT,
        &PROPERTY_DICT,
        &SECTOR_DICT,
    ];
    let mut terms = Vec::new();
    for dict in dicts {
        for entry in dict.iter() {
            if !terms.contains(&entry.term) {
                terms.push(entry.term);
            }
        }
    }
    if let Some(fam) = family {
        for (_dim, dict) in specialist_dicts_for(fam) {
            for entry in dict {
                if !terms.contains(&entry.term) {
                    terms.push(entry.term);
                }
            }
        }
    }
    terms
}

/// Return all canonical terms grouped by their p-dimension.
///
/// Used by audit tooling to report dictionary utilisation per dimension.
/// When `family` is `Some`, includes specialist terms for that family.
pub fn all_terms_by_dimension(family: Option<&str>) -> Vec<(PDimension, &'static str)> {
    let dicts: &[(PDimension, &[DictEntry])] = &[
        (PDimension::Person, &PERSON_DICT),
        (PDimension::Process, &PROCESS_DICT),
        (PDimension::Place, &PLACE_DICT),
        (PDimension::Plant, &PLANT_DICT),
        (PDimension::Property, &PROPERTY_DICT),
        (PDimension::Sector, &SECTOR_DICT),
    ];
    let mut result = Vec::new();
    for (dim, dict) in dicts {
        for entry in dict.iter() {
            if !result.iter().any(|(d, t)| d == dim && *t == entry.term) {
                result.push((*dim, entry.term));
            }
        }
    }
    if let Some(fam) = family {
        for (dim, dict) in specialist_dicts_for(fam) {
            for entry in dict {
                if !result.iter().any(|(d, t)| *d == dim && *t == entry.term) {
                    result.push((dim, entry.term));
                }
            }
        }
    }
    result
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Try to split a compound provision into separate applies/disapplies rules.
fn try_split_compound(text: &str, family: Option<&str>) -> Option<Vec<FitnessRule>> {
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

    let tags_a = extract_tags(part_a, family);
    let tags_b = extract_tags(part_b, family);

    Some(vec![
        FitnessRule {
            polarity: pol_a,
            tags: tags_a,
            raw_text: part_a.trim().to_string(),
            cross_refs: detect_cross_refs(part_a),
        },
        FitnessRule {
            polarity: pol_b,
            tags: tags_b,
            raw_text: part_b.trim().to_string(),
            cross_refs: detect_cross_refs(part_b),
        },
    ])
}

/// Extract all p-dimension tags from text.
///
/// Runs core dictionaries always.  When `family` is `Some`, also runs any
/// specialist dictionaries that match the family prefix.
fn extract_tags(text: &str, family: Option<&str>) -> Vec<PDimensionTag> {
    let mut tags = Vec::new();

    // Core dictionaries (always applied)
    let core_dicts: &[(PDimension, &[DictEntry])] = &[
        (PDimension::Person, &PERSON_DICT),
        (PDimension::Process, &PROCESS_DICT),
        (PDimension::Place, &PLACE_DICT),
        (PDimension::Plant, &PLANT_DICT),
        (PDimension::Property, &PROPERTY_DICT),
        (PDimension::Sector, &SECTOR_DICT),
    ];

    for (dim, dict) in core_dicts {
        for entry in *dict {
            if entry.re.is_match(text) {
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

    // Specialist dictionaries (applied when family matches)
    if let Some(fam) = family {
        for (dim, dict) in specialist_dicts_for(fam) {
            for entry in dict {
                if entry.re.is_match(text) {
                    let already = tags
                        .iter()
                        .any(|t: &PDimensionTag| t.dimension == dim && t.term == entry.term);
                    if !already {
                        tags.push(PDimensionTag {
                            dimension: dim,
                            term: entry.term.to_string(),
                        });
                    }
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
        let rules = extract(text, None);
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
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
        assert!(has_tag(&rules[0], PDimension::Person, "master of ship"));
        assert!(has_tag(&rules[0], PDimension::Person, "crew"));
    }

    #[test]
    fn does_not_apply() {
        let text =
            "This regulation does not apply to construction work carried out by the armed forces.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
        assert!(has_tag(&rules[0], PDimension::Process, "construction work"));
    }

    #[test]
    fn extends_to() {
        let text = "These Regulations extend to Northern Ireland.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::ExtendsTo);
        assert!(has_tag(&rules[0], PDimension::Place, "Northern Ireland"));
    }

    // ── Geographic patterns ─────────────────────────────────────────

    #[test]
    fn geographic_applies_to_england() {
        let text = "These Regulations apply to England only.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::AppliesTo);
        assert!(has_tag(&rules[0], PDimension::Place, "England"));
    }

    #[test]
    fn geographic_great_britain() {
        let text = "These Regulations shall apply in Great Britain.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Place, "Great Britain"));
    }

    #[test]
    fn extends_outside_gb() {
        let text = "These Regulations extend to outside Great Britain as sections 1 to 59 and 80 to 82 of the 1974 Act apply.";
        let rules = extract(text, None);
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
        let rules = extract(text, None);
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
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::AppliesTo);
        assert!(has_tag(&rules[0], PDimension::Person, "employer"));
        assert!(has_tag(&rules[0], PDimension::Person, "person at work"));
    }

    #[test]
    fn crown_application() {
        let text = "The provisions of these Regulations shall apply to persons in the public service of the Crown as they apply to other persons.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Person, "Crown service"));
    }

    // ── Process/activity patterns ───────────────────────────────────

    #[test]
    fn activity_scoped() {
        let text = "These Regulations apply to construction work.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Process, "construction work"));
    }

    #[test]
    fn diving_operations() {
        let text = "These Regulations shall apply to and in relation to any diving project.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Process, "diving operations"));
    }

    // ── Place patterns ──────────────────────────────────────────────

    #[test]
    fn quarry_scope() {
        let text = "These Regulations shall apply to all quarries where persons work.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Place, "quarry"));
        assert!(has_tag(&rules[0], PDimension::Person, "person at work"));
    }

    #[test]
    fn mine_exclusion() {
        let text = "These Regulations shall not apply to any place below ground in a mine.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
        assert!(has_tag(&rules[0], PDimension::Place, "mine"));
    }

    // ── Compound provisions ─────────────────────────────────────────

    #[test]
    fn compound_disapplies_then_applies() {
        let text = "These Regulations shall not apply in relation to such premises, but they shall apply in relation to premises used for domestic purposes.";
        let rules = extract(text, None);
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
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Plant, "gas fittings"));
    }

    #[test]
    fn dangerous_substances() {
        let text =
            "These Regulations apply where a dangerous substance is present at the workplace.";
        let rules = extract(text, None);
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
        let rules = extract(text, None);
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
        assert!(extract("", None).is_empty());
        assert!(extract("   ", None).is_empty());
    }

    #[test]
    fn no_polarity_returns_empty() {
        let text = "Citation, commencement and interpretation.";
        assert!(extract(text, None).is_empty());
    }

    #[test]
    fn ceases_to_have_effect() {
        let text = "This regulation ceases to have effect on 1st April 2025.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::DisappliesTo);
    }

    #[test]
    fn bind_the_crown() {
        let text = "This Act shall bind the Crown.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].polarity, RulePolarity::AppliesTo);
    }

    #[test]
    fn multiple_places() {
        let text = "These Regulations apply in relation to England and Wales.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(has_tag(&rules[0], PDimension::Place, "England"));
        assert!(has_tag(&rules[0], PDimension::Place, "Wales"));
    }

    // ── OH&S specialist dictionary tests ────────────────────────────

    #[test]
    fn ohs_lifting_operations_with_family() {
        let text = "These Regulations shall apply to lifting operations.";
        // Without family: no Process tag (not in core dict)
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(!has_tag(
            &rules[0],
            PDimension::Process,
            "lifting operations"
        ));
        // With OH&S family: Process tag found
        let rules = extract(text, Some("OH&S: Occupational / Personal Safety"));
        assert_eq!(rules.len(), 1);
        assert!(has_tag(
            &rules[0],
            PDimension::Process,
            "lifting operations"
        ));
    }

    #[test]
    fn ohs_pressure_equipment_with_family() {
        let text = "These Regulations apply to pressure equipment used at work.";
        let rules = extract(text, None);
        assert!(!has_tag(&rules[0], PDimension::Plant, "pressure equipment"));
        let rules = extract(text, Some("OH&S: Mines & Quarries"));
        assert!(has_tag(&rules[0], PDimension::Plant, "pressure equipment"));
    }

    #[test]
    fn ohs_young_person_with_family() {
        let text =
            "These Regulations shall not apply to a young person employed in a family undertaking.";
        let rules = extract(text, None);
        assert!(!has_tag(&rules[0], PDimension::Person, "young person"));
        let rules = extract(text, Some("OH&S: Occupational / Personal Safety"));
        assert!(has_tag(&rules[0], PDimension::Person, "young person"));
    }

    #[test]
    fn ohs_confined_spaces_with_family() {
        let text = "These Regulations apply where work in confined spaces is carried out.";
        let rules = extract(text, None);
        assert!(!has_tag(&rules[0], PDimension::Process, "confined spaces"));
        let rules = extract(text, Some("OH&S: Occupational / Personal Safety"));
        assert!(has_tag(&rules[0], PDimension::Process, "confined spaces"));
    }

    #[test]
    fn non_ohs_family_no_specialist() {
        let text = "These Regulations apply to lifting operations.";
        // Non-OH&S family should not get specialist tags
        let rules = extract(text, Some("AGRICULTURE"));
        assert!(!has_tag(
            &rules[0],
            PDimension::Process,
            "lifting operations"
        ));
    }

    // ── FIRE specialist dictionary tests ─────────────────────────────

    #[test]
    fn fire_fireworks_with_family() {
        let text = "This regulation does not apply to a person who is transporting fireworks on behalf of another person.";
        // Without family: no Plant tag for fireworks (not in core dict)
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(!has_tag(&rules[0], PDimension::Plant, "fireworks"));
        // With FIRE family: Plant tag found
        let rules = extract(text, Some("FIRE: Dangerous and Explosive Substances"));
        assert!(has_tag(&rules[0], PDimension::Plant, "fireworks"));
        assert!(has_tag(
            &rules[0],
            PDimension::Process,
            "transporting dangerous goods"
        ));
    }

    #[test]
    fn fire_small_arms_ammunition_with_family() {
        let text = "This regulation shall not apply to the transfer of any component of small arms ammunition by a person for his own sporting use.";
        let rules = extract(text, Some("FIRE: Dangerous and Explosive Substances"));
        assert!(has_tag(
            &rules[0],
            PDimension::Plant,
            "small arms ammunition"
        ));
        assert!(has_tag(
            &rules[0],
            PDimension::Process,
            "transfer of explosives"
        ));
    }

    #[test]
    fn fire_pyrotechnic_articles_with_family() {
        let text = "This regulation does not apply to pyrotechnic articles for vehicles.";
        let rules = extract(text, None);
        assert!(!has_tag(
            &rules[0],
            PDimension::Plant,
            "pyrotechnic articles"
        ));
        let rules = extract(text, Some("FIRE"));
        assert!(has_tag(
            &rules[0],
            PDimension::Plant,
            "pyrotechnic articles"
        ));
    }

    #[test]
    fn fire_harbour_area_with_family() {
        let text = "This regulation applies where the harbour area in respect of which the licence was issued becomes a nuclear harbour area.";
        let rules = extract(text, Some("FIRE: Dangerous and Explosive Substances"));
        assert!(has_tag(&rules[0], PDimension::Place, "harbour area"));
    }

    #[test]
    fn fire_authority_with_family() {
        let text = "These Regulations shall apply to every fire and rescue authority in England.";
        let rules = extract(text, Some("FIRE"));
        assert!(has_tag(&rules[0], PDimension::Person, "fire authority"));
        assert!(has_tag(&rules[0], PDimension::Place, "England"));
    }

    #[test]
    fn fire_specialist_not_on_ohs() {
        let text = "This regulation does not apply to fireworks.";
        let rules = extract(text, Some("OH&S: Occupational / Personal Safety"));
        assert!(!has_tag(&rules[0], PDimension::Plant, "fireworks"));
    }

    #[test]
    fn ohs_machinery_with_family() {
        let text = "These Regulations shall apply to machinery used at work.";
        let rules = extract(text, Some("OH&S: Occupational / Personal Safety"));
        assert!(has_tag(&rules[0], PDimension::Plant, "machinery"));
    }

    // ── Cross-reference detection ────────────────────────────────────

    #[test]
    fn cross_ref_regulation_detected() {
        let text = "Regulation 11(11) shall not apply in relation to any visiting force.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].cross_refs, vec!["Regulation 11(11)"]);
    }

    #[test]
    fn cross_ref_paragraph_detected() {
        let text = "paragraph (2) shall not apply to fumigations using the fumigant specified in Column 1.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].cross_refs, vec!["paragraph (2)"]);
    }

    #[test]
    fn cross_ref_schedule_detected() {
        let text = "Schedule 5 shall have effect.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].cross_refs, vec!["Schedule 5"]);
    }

    #[test]
    fn cross_ref_multiple_deduplicated() {
        let text = "Regulation 16A and regulation 17A do not apply in circumstances where regulation 16 and regulation 17 apply to the same premises.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].cross_refs.len(), 4);
        assert!(rules[0].cross_refs.contains(&"Regulation 16A".to_string()));
        assert!(rules[0].cross_refs.contains(&"regulation 17A".to_string()));
        assert!(rules[0].cross_refs.contains(&"regulation 16".to_string()));
        assert!(rules[0].cross_refs.contains(&"regulation 17".to_string()));
    }

    #[test]
    fn cross_ref_plural_regulations() {
        let text = "This regulation shall not apply to regulations 21 and 28(2), which expressly say on whom the duties are imposed.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        // "regulations 21" matches; "28(2)" has no prefix so is not detected
        assert!(rules[0].cross_refs.contains(&"regulations 21".to_string()));
    }

    #[test]
    fn no_cross_ref_when_none_present() {
        let text = "These Regulations shall apply to every employer and self-employed person.";
        let rules = extract(text, None);
        assert_eq!(rules.len(), 1);
        assert!(rules[0].cross_refs.is_empty());
    }
}
