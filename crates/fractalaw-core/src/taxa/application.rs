//! Law application: the nations where a law operates.
//!
//! Distinct from *extent* (the legal systems a law forms part of). A Welsh SI
//! extends to England and Wales but applies in Wales; an "(England)" SI extends
//! to England and Wales but applies in England. The screener needs application.
//! See sertantai-legal #162 (extent) and #163 (ZENOH-SPEC v2.4 application fields).
//!
//! Derivation, first match wins:
//! 1. `text_clause`: "These Regulations apply (only) (in relation) to England";
//!    a law-level "do not apply to Scotland" subtracts from what steps 2-4 give
//! 2. `title`: nations named in a title parenthetical, "(England)", "(Wales) Act"
//! 3. `type_code`: devolved legislation types
//! 4. `extent_fallback`: extent clause in the text, else LAT provision extents,
//!    else the LRT extent

use std::sync::LazyLock;

use regex::Regex;

pub const ENGLAND: &str = "england";
pub const WALES: &str = "wales";
pub const SCOTLAND: &str = "scotland";
pub const NORTHERN_IRELAND: &str = "northern_ireland";

/// Canonical nation order.
pub const NATIONS: [&str; 4] = [ENGLAND, WALES, SCOTLAND, NORTHERN_IRELAND];

/// Territorial codes that express jurisdiction. In compiled trees they are
/// replaced by a single root application gate.
pub const JURISDICTION_CODES: &[&str] = &[
    ENGLAND,
    WALES,
    SCOTLAND,
    NORTHERN_IRELAND,
    "united_kingdom",
    "great_britain",
    "england_and_wales",
    "uk",
    "gb",
    "ni",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationSource {
    TextClause,
    Title,
    TypeCode,
    ExtentFallback,
}

impl ApplicationSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TextClause => "text_clause",
            Self::Title => "title",
            Self::TypeCode => "type_code",
            Self::ExtentFallback => "extent_fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Application {
    /// Subset of [`NATIONS`], canonical order, non-empty
    pub regions: Vec<String>,
    pub source: ApplicationSource,
    pub evidence: String,
}

/// Inputs for one law.
#[derive(Debug, Default)]
pub struct ApplicationInput<'a> {
    pub type_code: &'a str,
    pub title: &'a str,
    /// (section_id, text) of the law's provisions (all, or at least those
    /// mentioning apply/extend together with a nation)
    pub provisions: &'a [(String, String)],
    /// Distinct non-empty provision-level extent codes from LAT ("E+W", "S" ...)
    pub lat_extents: &'a [String],
    /// Law-level extent from the LRT ("UK", "E+W", "GB" ...)
    pub lrt_extent: Option<&'a str>,
}

const SELF_REF: &str =
    r"\b(?:these\s+regulations|this\s+(?:act|order|measure|scheme|instrument)|these\s+(?:rules|byelaws))\b";
const NATION_ALT: &str =
    r"(?:the\s+)?(?:united\s+kingdom|great\s+britain|england\s+and\s+wales|northern\s+ireland|england|wales|scotland)";

static APPLY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i){SELF_REF}[^.;]{{0,40}}?\b(?:shall\s+|do\s+|does\s+)?(not\s+)?(?:only\s+)?appl(?:y|ies)[\s—–-]+(?:\([a-z]\)\s*)?(?:only\s+)?(?:in\s+relation\s+to|to|in)\s+({NATION_ALT}(?:\s*(?:,|and|or)\s*{NATION_ALT})*)"
    ))
    .unwrap()
});

static EXTEND_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i){SELF_REF}[^.;]{{0,40}}?\b(?:shall\s+|do\s+|does\s+)?(not\s+)?extends?[\s—–-]+(?:only\s+)?to\s+({NATION_ALT}(?:\s*(?:,|and|or)\s*{NATION_ALT})*)"
    ))
    .unwrap()
});

static TITLE_PAREN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(([^()]*)\)").unwrap());

static NATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)united\s+kingdom|great\s+britain|northern\s+ireland|england|wales|scotland").unwrap()
});

/// Nations named in a phrase ("England and Wales" → [england, wales]).
pub fn nations_in(phrase: &str) -> Vec<String> {
    let mut found = [false; 4];
    for m in NATION_RE.find_iter(phrase) {
        match m.as_str().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ").as_str() {
            "united kingdom" => found = [true; 4],
            "great britain" => {
                found[0] = true;
                found[1] = true;
                found[2] = true;
            }
            "england" => found[0] = true,
            "wales" => found[1] = true,
            "scotland" => found[2] = true,
            "northern ireland" => found[3] = true,
            _ => {}
        }
    }
    NATIONS
        .iter()
        .zip(found)
        .filter(|(_, f)| *f)
        .map(|(n, _)| n.to_string())
        .collect()
}

/// Nations for an extent code ("E+W+S+NI", "E+W", "S", "NI", "UK", "GB" ...).
pub fn nations_for_extent(code: &str) -> Vec<String> {
    let c = code.to_uppercase().replace(['.', ' '], "");
    match c.as_str() {
        "UK" => return NATIONS.iter().map(|s| s.to_string()).collect(),
        "GB" => return vec![ENGLAND.into(), WALES.into(), SCOTLAND.into()],
        _ => {}
    }
    let parts: Vec<&str> = c.split('+').collect();
    let mut found = [false; 4];
    for p in parts {
        match p {
            "E" => found[0] = true,
            "W" => found[1] = true,
            "S" => found[2] = true,
            "NI" | "N" => found[3] = true,
            _ if p.contains("NORTHERNIRELAND") || p.contains("NOTHERNIRELAND") => found[3] = true,
            _ => {}
        }
    }
    NATIONS
        .iter()
        .zip(found)
        .filter(|(_, f)| *f)
        .map(|(n, _)| n.to_string())
        .collect()
}

static PARTIAL_SUBJECT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:\bof|\bto|\bonly\s+the\s+following\s+\w+|\bexcept|\bto\s+the\s+extent(?:\s+that)?|\bwhere|\bif|\bwhen|\bso\s+far\s+as|\binsofar\s+as)\s*$").unwrap());

/// "Only the following provisions of this Act extend to ..." / "section 3 of these
/// Regulations applies to ..." — the subject is part of the law, not the law.
fn is_partial_subject(text: &str, m: &regex::Match) -> bool {
    let start = text[..m.start()].char_indices().rev().nth(40).map(|(i, _)| i).unwrap_or(0);
    PARTIAL_SUBJECT_RE.is_match(&text[start..m.start()])
}

static DESCRIPTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bas\s+(?:it|they|that\s+\w+|those\s+\w+)\b|\bso\s+far\s+as\b|\bin\s*so\s+far\b|\binsofar\b").unwrap()
});

/// "these Regulations as they apply in relation to England" describes, it doesn't declare.
fn is_descriptive(matched: &str) -> bool {
    let verb = matched.to_lowercase().find("appl").or_else(|| matched.to_lowercase().find("extend"));
    DESCRIPTIVE_RE.is_match(&matched[..verb.unwrap_or(matched.len())])
}

static NOTHING_IN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bnothing\s+in\s*$").unwrap());

/// "Nothing in these Regulations applies to Wales" negates.
fn is_nothing_in(text: &str, m: &regex::Match) -> bool {
    let start = text[..m.start()].char_indices().rev().nth(20).map(|(i, _)| i).unwrap_or(0);
    NOTHING_IN_RE.is_match(&text[start..m.start()])
}

fn union(into: &mut Vec<String>, more: Vec<String>) {
    for n in more {
        if !into.contains(&n) {
            into.push(n);
        }
    }
    into.sort_by_key(|n| NATIONS.iter().position(|x| x == n));
}

fn snippet(section_id: &str, text: &str, m: regex::Match) -> String {
    let local = section_id.split_once(':').map(|(_, s)| s).unwrap_or(section_id);
    let start = text[..m.start()].rfind(['.', ';']).map(|i| i + 1).unwrap_or(0);
    let clause: String = text[start..m.end()].split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{local}: \"{clause}\"")
}

fn type_code_regions(type_code: &str) -> Option<Vec<String>> {
    let nation = match type_code {
        "wsi" | "anaw" | "asc" | "mwa" | "wlc" => WALES,
        "ssi" | "asp" | "ssa" | "aosp" => SCOTLAND,
        "nisr" | "nia" | "apni" | "nisi" | "nisro" | "aip" => NORTHERN_IRELAND,
        _ => return None,
    };
    Some(vec![nation.to_string()])
}

/// Steps 2-4 (everything except text application clauses).
fn derive_without_clause(input: &ApplicationInput) -> Option<Application> {
    // 2. title parentheticals
    let mut title_regions = Vec::new();
    for cap in TITLE_PAREN_RE.captures_iter(input.title) {
        union(&mut title_regions, nations_in(&cap[1]));
    }
    if !title_regions.is_empty() {
        return Some(Application {
            regions: title_regions,
            source: ApplicationSource::Title,
            evidence: input.title.to_string(),
        });
    }

    // 3. type code
    if let Some(regions) = type_code_regions(input.type_code) {
        return Some(Application {
            regions,
            source: ApplicationSource::TypeCode,
            evidence: input.type_code.to_string(),
        });
    }

    // 4. extent: text extent clause > LAT provision extents > LRT.
    //    "does not extend to Northern Ireland" subtracts from whichever base applies.
    let mut pos = Vec::new();
    let mut neg = Vec::new();
    let mut evidence = Vec::new();
    for (sid, text) in input.provisions {
        for cap in EXTEND_RE.captures_iter(text) {
            let m = cap.get(0).unwrap();
            let regions = nations_in(&cap[2]);
            if regions.is_empty() || is_partial_subject(text, &m) || is_descriptive(m.as_str()) {
                continue;
            }
            evidence.push(snippet(sid, text, m));
            if cap.get(1).is_some() || is_nothing_in(text, &m) {
                union(&mut neg, regions);
            } else {
                union(&mut pos, regions);
            }
        }
    }
    let base: Option<(Vec<String>, String)> = if !pos.is_empty() {
        None
    } else {
        let mut lat = Vec::new();
        for code in input.lat_extents {
            union(&mut lat, nations_for_extent(code));
        }
        if !lat.is_empty() {
            Some((lat, format!("LAT extent {}", input.lat_extents.join(","))))
        } else {
            input
                .lrt_extent
                .map(|e| (nations_for_extent(e), format!("LRT extent {e}")))
                .filter(|(r, _)| !r.is_empty())
        }
    };
    let (mut regions, base_evidence) = match base {
        Some((r, ev)) => (r, Some(ev)),
        None => (pos, None),
    };
    regions.retain(|n| !neg.contains(n));
    if regions.is_empty() {
        return None;
    }
    evidence.extend(base_evidence);
    Some(Application {
        regions,
        source: ApplicationSource::ExtentFallback,
        evidence: evidence.join(" | "),
    })
}

/// Derive a law's application. `None` when no source says anything.
pub fn derive_application(input: &ApplicationInput) -> Option<Application> {
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    let mut evidence_pos = Vec::new();
    let mut evidence_neg = Vec::new();

    for (sid, text) in input.provisions {
        for cap in APPLY_RE.captures_iter(text) {
            let m = cap.get(0).unwrap();
            let regions = nations_in(&cap[2]);
            if regions.is_empty() || is_partial_subject(text, &m) || is_descriptive(m.as_str()) {
                continue;
            }
            let ev = snippet(sid, text, m);
            if cap.get(1).is_some() || is_nothing_in(text, &m) {
                union(&mut negative, regions);
                evidence_neg.push(ev);
            } else {
                union(&mut positive, regions);
                evidence_pos.push(ev);
            }
        }
    }

    if !positive.is_empty() {
        positive.retain(|n| !negative.contains(n));
        if !positive.is_empty() {
            let mut evidence = evidence_pos;
            evidence.extend(evidence_neg);
            return Some(Application {
                regions: positive,
                source: ApplicationSource::TextClause,
                evidence: evidence.join(" | "),
            });
        }
    }

    let base = derive_without_clause(input);
    if negative.is_empty() {
        return base;
    }
    // Subtractive clause: "These Regulations do not apply to Scotland"
    let base_regions = base
        .as_ref()
        .map(|b| b.regions.clone())
        .unwrap_or_else(|| NATIONS.iter().map(|s| s.to_string()).collect());
    let regions: Vec<String> = base_regions.into_iter().filter(|n| !negative.contains(n)).collect();
    if regions.is_empty() {
        return base;
    }
    let mut evidence = evidence_neg;
    if let Some(b) = &base {
        evidence.push(format!("base {}: {}", b.source.as_str(), b.evidence));
    }
    Some(Application {
        regions,
        source: ApplicationSource::TextClause,
        evidence: evidence.join(" | "),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    fn regions(a: &Option<Application>) -> Vec<&str> {
        a.as_ref().map(|a| a.regions.iter().map(|s| s.as_str()).collect()).unwrap_or_default()
    }

    #[test]
    fn nations_parsing() {
        assert_eq!(nations_in("England and Wales"), vec!["england", "wales"]);
        assert_eq!(nations_in("Great Britain"), vec!["england", "wales", "scotland"]);
        assert_eq!(nations_in("the United Kingdom"), NATIONS.to_vec());
        assert_eq!(nations_for_extent("E+W+S+N.I."), NATIONS.to_vec());
        assert_eq!(nations_for_extent("E+W"), vec!["england", "wales"]);
        assert_eq!(nations_for_extent("GB"), vec!["england", "wales", "scotland"]);
    }

    #[test]
    fn application_clause_beats_extent() {
        // UK_uksi_2006_3368: extends E+W, applies to England only
        let p = provs(&[
            ("UK_uksi_2006_3368:reg.1(2)", "These Regulations apply in relation to England only."),
            ("UK_uksi_2006_3368:reg.1(9)", "These Regulations extend to England and Wales."),
        ]);
        let a = derive_application(&ApplicationInput {
            type_code: "uksi",
            title: "Smoke-free (Premises and Enforcement) Regulations",
            provisions: &p,
            lat_extents: &["E+W".into()],
            lrt_extent: Some("UK"),
        });
        assert_eq!(regions(&a), vec!["england"]);
        let a = a.unwrap();
        assert_eq!(a.source, ApplicationSource::TextClause);
        assert_eq!(a.evidence, "reg.1(2): \"These Regulations apply in relation to England\"");
    }

    #[test]
    fn title_parenthetical() {
        // UK_uksi_2025_140: text only says "extend to England and Wales"
        let p = provs(&[("UK_uksi_2025_140:reg.1(3)", "These Regulations extend to England and Wales.")]);
        let a = derive_application(&ApplicationInput {
            type_code: "uksi",
            title: "Separation of Waste (England) Regulations",
            provisions: &p,
            lat_extents: &["E+W".into()],
            lrt_extent: Some("UK"),
        });
        assert_eq!(regions(&a), vec!["england"]);
        assert_eq!(a.unwrap().source, ApplicationSource::Title);
        let a = derive_application(&ApplicationInput {
            type_code: "uksi",
            title: "Keeping and Introduction of Fish (England and River Esk Catchment Area) Regulations",
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["england"]);
    }

    #[test]
    fn devolved_type_code_ignores_placeholder_extent() {
        // Unrevised NISR: legislation.gov.uk placeholder E+W+S+N.I.
        let a = derive_application(&ApplicationInput {
            type_code: "nisr",
            title: "Dangerous Substances in Harbour Areas Regulations",
            lrt_extent: Some("UK"),
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["northern_ireland"]);
        assert_eq!(a.unwrap().source, ApplicationSource::TypeCode);
        let a = derive_application(&ApplicationInput { type_code: "wsi", lrt_extent: Some("E+W"), ..Default::default() });
        assert_eq!(regions(&a), vec!["wales"]);
    }

    #[test]
    fn extent_fallback_order() {
        let p = provs(&[("UK_uksi_1:reg.1(3)", "These Regulations extend to Great Britain.")]);
        let a = derive_application(&ApplicationInput {
            type_code: "uksi",
            provisions: &p,
            lat_extents: &["E+W+S+NI".into()],
            lrt_extent: Some("UK"),
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["england", "wales", "scotland"]);
        let a = derive_application(&ApplicationInput {
            type_code: "uksi",
            lat_extents: &["E+W".into()],
            lrt_extent: Some("UK"),
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["england", "wales"]);
        let a = derive_application(&ApplicationInput { type_code: "ukpga", lrt_extent: Some("GB"), ..Default::default() });
        assert_eq!(a.unwrap().evidence, "LRT extent GB");
        assert!(derive_application(&ApplicationInput { type_code: "ukpga", ..Default::default() }).is_none());
    }

    #[test]
    fn negative_extent_subtracts() {
        // UK_ukpga_1974_37 s.84(1)
        let p = provs(&[("UK_ukpga_1974_37:s.84(1)", "This Act, except— does not extend to Northern Ireland.")]);
        let a = derive_application(&ApplicationInput {
            type_code: "ukpga",
            provisions: &p,
            lat_extents: &["E+W+S+NI".into(), "E+W".into()],
            lrt_extent: Some("UK"),
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["england", "wales", "scotland"]);
    }

    #[test]
    fn application_clause_across_sub_paragraphs() {
        // UK_uksi_2015_51 reg.3 with its sub-paragraphs joined
        let p = provs(&[(
            "UK_uksi_2015_51:reg.3",
            "These Regulations apply— (a) in Great Britain; and (b) to premises and activities outside Great Britain",
        )]);
        let a = derive_application(&ApplicationInput {
            type_code: "uksi",
            provisions: &p,
            lat_extents: &["E+W+S+NI".into()],
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["england", "wales", "scotland"]);
        assert_eq!(a.unwrap().source, ApplicationSource::TextClause);
    }

    #[test]
    fn partial_extent_clause_ignored() {
        // UK_ukpga_1990_43 s.164(4)
        let p = provs(&[(
            "UK_ukpga_1990_43:s.164(4)",
            "(4) Only the following provisions of this Act (together with this section) extend to Northern Ireland—",
        )]);
        let a = derive_application(&ApplicationInput {
            type_code: "ukpga",
            provisions: &p,
            lat_extents: &["E+W+S".into(), "E+W".into(), "S".into()],
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["england", "wales", "scotland"]);
        let p = provs(&[("UK_x:reg.9", "Regulation 5 of these Regulations applies to Scotland only.")]);
        let a = derive_application(&ApplicationInput { type_code: "uksi", provisions: &p, lrt_extent: Some("GB"), ..Default::default() });
        assert_eq!(a.unwrap().source, ApplicationSource::ExtentFallback);
    }

    #[test]
    fn descriptive_and_nothing_in_clauses() {
        // UK_uksi_2009_3344 reg.26(1)(a), UK_uksi_2016_1092 reg.38D(4)
        let p = provs(&[
            ("UK_a:reg.26(1)(a)", "for the purposes of enforcing these Regulations as they apply in relation to England"),
            ("UK_a:reg.38D(4)", "has the meaning given to it in regulation 2(1) of these Regulations as it applies in Northern Ireland"),
        ]);
        let a = derive_application(&ApplicationInput { type_code: "uksi", provisions: &p, lrt_extent: Some("E+W"), ..Default::default() });
        assert_eq!(a.unwrap().source, ApplicationSource::ExtentFallback);
        // UK_uksi_2005_894 reg.1(3)
        let p = provs(&[("UK_uksi_2005_894:reg.1(3)", "Nothing in these Regulations applies to Wales.")]);
        let a = derive_application(&ApplicationInput { type_code: "uksi", provisions: &p, lat_extents: &["E+W".into()], ..Default::default() });
        assert_eq!(regions(&a), vec!["england"]);
    }

    #[test]
    fn conditional_clauses_ignored() {
        // UK_uksi_2015_627 reg.35(1)(a), UK_uksi_2016_1092 reg.38D(4)
        let p = provs(&[
            ("UK_a:reg.35(1)(a)", "(and to the extent that these Regulations apply in relation to Wales and Scotland);"),
            ("UK_a:reg.38D(4)", "paragraph 2(2)(c) of Schedule 2 to these Regulations, as that Schedule applies in Northern Ireland."),
        ]);
        let a = derive_application(&ApplicationInput { type_code: "uksi", provisions: &p, lrt_extent: Some("E+W"), ..Default::default() });
        assert_eq!(a.unwrap().source, ApplicationSource::ExtentFallback);
    }

    #[test]
    fn subtractive_clause() {
        let p = provs(&[("UK_ukpga_1:s.99", "This Act does not apply to Northern Ireland.")]);
        let a = derive_application(&ApplicationInput {
            type_code: "ukpga",
            provisions: &p,
            lrt_extent: Some("UK"),
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["england", "wales", "scotland"]);
        assert_eq!(a.unwrap().source, ApplicationSource::TextClause);
    }

    #[test]
    fn partial_application_is_not_law_level() {
        // UK_uksi_2015_1360 reg.1(2)
        let p = provs(&[("UK_uksi_2015_1360:reg.1(2)", "Regulations 7 to 9 and 11 apply to England only.")]);
        let a = derive_application(&ApplicationInput {
            type_code: "uksi",
            provisions: &p,
            lat_extents: &["E+W".into()],
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["england", "wales"]);
        assert_eq!(a.unwrap().source, ApplicationSource::ExtentFallback);
    }

    #[test]
    fn scottish_act_title_and_cross_references() {
        // "(Scotland)" in the title; a cross-reference to another Act must not count
        let p = provs(&[("UK_asp_2021_4:s.1", "The Scottish Ministers may by regulations make provision corresponding to EU law.")]);
        let a = derive_application(&ApplicationInput {
            type_code: "asp",
            title: "UK Withdrawal from the European Union (Continuity) (Scotland) Act",
            provisions: &p,
            lrt_extent: Some("S"),
            ..Default::default()
        });
        assert_eq!(regions(&a), vec!["scotland"]);
    }
}
