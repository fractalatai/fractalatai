//! Compile a law's fitness mentions into an [`ApplicabilityNode`] tree.
//!
//! Pure logic, no I/O: the CLI loads mentions and provision text from
//! Postgres and passes them in.
//!
//! Semantics (unchanged from the July compiler):
//! - AppliesTo mentions are disjunctive: `Or(applies...)`
//! - DisappliesTo mentions become a law-level `Not(Or(...))`, except codes that
//!   also appear in AppliesTo (those are provision-level overrides)
//! - dates become a single TimeWindow gate
//!
//! Source fixes for the QQ-03 lint (sertantai-legal #161):
//! - L1: output is [`ApplicabilityNode::normalize`]d
//! - L2: `from` only from law-level commencement, `to` only from a law-level
//!   sunset ("These Regulations cease to have effect ..."), never `to < from`
//! - L4: codes must be grounded in the provision text (or its sub-provisions)
//!   or the law's own title, after stripping citations of other legislation;
//!   `construction` in the statutory-interpretation sense is dropped
//! - Whole-law Not: only law-level disapplications ("These Regulations do not
//!   apply to ...", "Nothing in this Act applies to ...") become the root Not.
//!   Provision-level exceptions ("Regulation 9 does not apply to ...") can't be
//!   scoped in the tree, so they are dropped rather than negating the whole law.
//! - L3 (subject): a law never disapplies the subject its own title names.
//!   "These Regulations shall not apply to activities to which the Control of
//!   Noise at Work Regulations 2005 apply" would otherwise give the Merchant
//!   Shipping (Control of Noise at Work) Regs `Not(at_work)`.
//! - L3/L7: jurisdiction codes (england, scotland, united_kingdom ...) never
//!   appear inside branches or under Not; the law's application
//!   ([`super::application`]) becomes a single root `territorial` gate
//! - L5: government actors are regulators, not regulated persons, so their codes
//!   are dropped. Trees are only screened for Making laws (duties on regulated
//!   persons), so a mention naming only government actors means the regulated
//!   person wasn't extracted; it is not evidence the law binds only government.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use super::applicability::ApplicabilityNode;
use super::application::JURISDICTION_CODES;

/// One fitness mention with the text it was extracted from.
#[derive(Debug, Clone)]
pub struct MentionInput {
    /// `AppliesTo`, `ExtendsTo` or `DisappliesTo`
    pub polarity: String,
    /// Reconciled entity names (display form) and ISO dates
    pub entities: Vec<String>,
    /// Text of the provision plus its sub-provisions
    pub text: String,
}

/// Law-level context for compilation.
#[derive(Debug, Clone, Default)]
pub struct LawContext {
    /// The law's own title (grounding context: a code in the title is the law's subject)
    pub title: String,
    /// Nations where the law applies (root gate). `None` = unknown, no gate.
    pub application: Option<Vec<String>>,
}

pub fn is_jurisdiction(code: &str) -> bool {
    JURISDICTION_CODES.contains(&code)
}

/// Is `code` part of the subject named by the law's own title?
pub fn is_title_subject(code: &str, ctx: &LawContext) -> bool {
    !ctx.title.is_empty() && is_grounded(code, &ctx.title)
}

/// Root application gate, placed first under the root And.
fn with_application_gate(tree: Option<ApplicabilityNode>, ctx: &LawContext) -> Option<ApplicabilityNode> {
    let Some(regions) = ctx.application.as_ref().filter(|r| !r.is_empty()) else {
        return tree;
    };
    let gate = ApplicabilityNode::match_any("territorial", regions.clone());
    let mut top = vec![gate];
    match tree {
        Some(ApplicabilityNode::And { children }) => top.extend(children),
        Some(other) => top.push(other),
        None => {}
    }
    Some(ApplicabilityNode::and(top).normalize())
}

/// Codes for government actors. These regulate; they are not the persons a
/// law applies to, so they never appear as applicability conditions (L5).
pub const GOV_ACTOR_CODES: &[&str] = &[
    "secretary_of_state",
    "local_authority",
    "scottish_ministers",
    "welsh_ministers",
    "enforcement_authority",
    "public_authority",
    "local_planning_authority",
    "planning_authority",
    "appropriate_authority",
    "competent_authority",
    "national_park_authority",
    "national_authority",
    "relevant_authority",
    "responsible_authority",
    "charging_authority",
    "highway_authority",
    "authority",
    "public_body",
    "statutory_body",
    "nature_conservation_body",
    "regulator",
    "local_government",
    "inspector",
    "chief_inspector",
    "authorised_officer",
    "council",
    "sepa",
    "scottish_environment_protection_agency",
    "national_river_authority",
    "appropriate_agency",
    "agency",
    "executive",
    "commission",
    "assembly",
    "minister",
    "ministers",
    "magistrates_court",
    "rule-making_authority",
];

static STOPWORDS: &[&str] = &["of", "the", "and", "or", "for", "in", "to", "a", "an", "at", "on", "by", "with"];

/// Law-level self reference: "these Regulations", "this Act", ...
const SELF_REF: &str =
    r"\b(?:these\s+regulations|this\s+(?:act|order|measure|scheme|instrument)|these\s+(?:rules|byelaws))\b";

static LAW_COMMENCEMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i){SELF_REF}[^.;]{{0,80}}\b(?:come|comes|came|shall\s+come)\s+into\s+(?:force|operation)"
    ))
    .unwrap()
});

static LAW_SUNSET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i){SELF_REF}[^.;]{{0,80}}\b(?:(?:cease|ceases|shall\s+cease)\s+to\s+have\s+effect|expire|expires|shall\s+expire)\b"
    ))
    .unwrap()
});

static LAW_DISAPPLY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?i)(?:^|[.;:—–]\s*|\(\d+\)\s*|\bsubject\s+to\s+[^,]{{0,60}},\s*){SELF_REF}[^.;]{{0,60}}?\b(?:do|does|shall|will)\s+not\s+(?:apply|extend)|\bnothing\s+in\s+{SELF_REF}[^.;]{{0,60}}?\b(?:appl|extend|prevent|affect)"
    ))
    .unwrap()
});

/// Is this a law-level disapplication (the law itself is the subject)?
pub fn is_law_level_disapplication(text: &str) -> bool {
    LAW_DISAPPLY_RE.is_match(text.trim_start())
}

/// "construction" meaning interpretation ("Interpretation and construction",
/// "construction of these Regulations", "construed").
static CONSTRUCTION_INTERPRETATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\binterpretation\s+and\s+construction\b|\bconstruction\s+of\s+(?:this|these|the|any|references?)\b|\bconstrued\b")
        .unwrap()
});

/// "construction" in the building/civil-engineering sense.
static CONSTRUCTION_BUILDING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bconstruction\s+(?:work|works|site|sites|industry|project|projects|products?|phase|operations?|activities|\(design)|\bbuilding\s+work|\bdemolition\b|\bconstruct(?:ion|ing)\s+(?:of\s+)?(?:a|an|the|any|new)?\s*(?:building|road|sewer|dam|bridge|tunnel|pipe-?line|dwelling|reservoir|railway|harbour|structure)s?\b")
        .unwrap()
});

/// "construction" meaning how a product is built ("Construction and Use",
/// "constructed of safety glass", "constructed or adapted").
static CONSTRUCTION_PRODUCT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bconstruction\s+and\s+use\b|\bconstruct(?:ed|ion)\s+(?:of|so\s+as|or\s+adapted|and\s+equipment)\b|\b(?:vehicle|trailer|ship|vessel|aircraft|appliance|equipment|machinery)\b[^.;]{0,40}\bconstruct")
        .unwrap()
});

/// Citation of another instrument: "the Mines Regulations 2014",
/// "Water (Scotland) Act 1980", "the Health and Safety at Work etc. Act 1974".
static CITATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    let word = r"(?:[A-Z][\w’'\-]*|\([A-Z][^)]*\)|and|of|for|the|in|to|on|at|etc\.?|&)";
    Regex::new(&format!(
        r"{word}(?:\s+{word})*\s+(?:Act|Regulations|Order|Rules|Measure)(?:\s+\([A-Z][^)]*\))?\s+\d{{4}}"
    ))
    .unwrap()
});

static CITED_AS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\bmay\s+be\s+cited\s+as\s+(?:the\s+)?["“‘']?([^"“”‘’.;]+?(?:Act|Regulations|Order|Rules|Measure|Scheme)(?:\s+\((?:Northern\s+Ireland|Scotland|Wales|England)\))?(?:\s+\d{4})?)"#)
        .unwrap()
});

/// The short title from a citation provision ("... may be cited as the X Regulations 2015").
pub fn title_from_citation(text: &str) -> Option<String> {
    CITED_AS_RE
        .captures(text)
        .map(|c| c[1].trim().to_string())
}

/// Remove citations of other legislation so their titles don't ground codes.
pub fn strip_citations(text: &str) -> String {
    CITATION_RE.replace_all(text, " ").into_owned()
}

static CONSTRUCTION_WORD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bconstruct(?:ion|ions|ing|ed|s)?\b").unwrap());

/// Normalise an entity display name to a tree code.
pub fn to_code(entity: &str) -> String {
    entity.to_lowercase().replace(' ', "_")
}

pub fn is_gov_actor(code: &str) -> bool {
    GOV_ACTOR_CODES.contains(&code)
}

pub fn is_iso_date(s: &str) -> bool {
    s.len() == 10
        && s.as_bytes().get(4) == Some(&b'-')
        && s.as_bytes().get(7) == Some(&b'-')
        && s[..4].chars().all(|c| c.is_ascii_digit())
}

/// Does `text` contain a word starting with `prefix`?
fn has_word_prefix(text: &str, prefix: &str, whole_word: bool) -> bool {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(pos) = text[start..].find(prefix) {
        let at = start + pos;
        let end = at + prefix.len();
        let before_ok = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
        let after_ok = !whole_word || end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = end;
    }
    false
}

/// Is `code` anchored in `text`? The head word of the code (first non-stopword)
/// must appear as a word prefix (first ≤6 chars; whole word if shorter than 4).
/// `construction*` codes need the building sense of the word (L4).
pub fn is_grounded(code: &str, text: &str) -> bool {
    if code == "construction" || code.starts_with("construction_") {
        if !CONSTRUCTION_WORD_RE.is_match(text) {
            return false;
        }
        if CONSTRUCTION_BUILDING_RE.is_match(text) {
            return true;
        }
        return !CONSTRUCTION_INTERPRETATION_RE.is_match(text) && !CONSTRUCTION_PRODUCT_RE.is_match(text);
    }
    let lower = text.to_lowercase();
    let head = code
        .split(['_', ' ', '-'])
        .find(|w| !w.is_empty() && !STOPWORDS.contains(w));
    let Some(head) = head else { return true };
    if head.len() < 4 {
        return has_word_prefix(&lower, head, true);
    }
    let cut = head.char_indices().nth(6).map(|(i, _)| i).unwrap_or(head.len());
    has_word_prefix(&lower, &head[..cut], false)
}

pub fn is_law_commencement(text: &str) -> bool {
    LAW_COMMENCEMENT_RE.is_match(text)
}

pub fn is_law_sunset(text: &str) -> bool {
    LAW_SUNSET_RE.is_match(text)
}

/// Compile all mentions for one law. Returns `None` when nothing survives.
pub fn compile_law(
    mentions: &[MentionInput],
    entity_dims: &HashMap<String, String>,
    ctx: &LawContext,
) -> Option<ApplicabilityNode> {
    let mut applies_nodes = Vec::new();
    let mut disapplies_nodes = Vec::new();
    let mut from_dates: Vec<&str> = Vec::new();
    let mut to_dates: Vec<&str> = Vec::new();

    for mention in mentions {
        let applies = matches!(mention.polarity.as_str(), "AppliesTo" | "ExtendsTo");
        let disapplies = mention.polarity == "DisappliesTo";

        let stripped = strip_citations(&mention.text);
        let mut grounded: Vec<(String, String)> = Vec::new(); // (dimension, code)
        for entity in &mention.entities {
            if is_iso_date(entity) {
                if applies && is_law_commencement(&mention.text) {
                    from_dates.push(entity);
                } else if disapplies && is_law_sunset(&mention.text) {
                    to_dates.push(entity);
                }
                continue;
            }
            let code = to_code(entity);
            if !is_grounded(&code, &stripped) && !is_grounded(&code, &ctx.title) {
                continue;
            }
            let dim = entity_dims
                .get(&entity.to_lowercase())
                .cloned()
                .unwrap_or_else(|| "material".to_string());
            grounded.push((dim, code));
        }
        // L5: government actors are regulators, never applicability conditions.
        // L3/L7: jurisdiction is carried by the root application gate only.
        grounded.retain(|(_, c)| !is_gov_actor(c) && !is_jurisdiction(c));
        // L3 (subject): never disapply what the law's own title is about
        if disapplies {
            grounded.retain(|(_, c)| !is_title_subject(c, ctx));
        }
        if grounded.is_empty() {
            continue;
        }
        let mut by_dim: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (dim, code) in grounded {
            by_dim.entry(dim).or_default().push(code);
        }

        let node = ApplicabilityNode::and(
            by_dim
                .into_iter()
                .map(|(dim, codes)| ApplicabilityNode::match_any(&dim, codes))
                .collect(),
        );
        if disapplies {
            if is_law_level_disapplication(&mention.text) {
                disapplies_nodes.push(node);
            }
        } else if applies {
            applies_nodes.push(node);
        }
    }

    let mut top = Vec::new();
    if !applies_nodes.is_empty() {
        let applies_codes: HashSet<String> = applies_nodes.iter().flat_map(codes_of).collect();
        top.push(ApplicabilityNode::or(applies_nodes));

        let filtered: Vec<ApplicabilityNode> = disapplies_nodes
            .into_iter()
            .filter(|n| !codes_of(n).iter().any(|c| applies_codes.contains(c)))
            .collect();
        if !filtered.is_empty() {
            top.push(ApplicabilityNode::or(filtered).negate());
        }
    }

    let from = from_dates.into_iter().min();
    let to = to_dates.into_iter().max().filter(|t| from.is_none_or(|f| *t >= f));
    if from.is_some() || to.is_some() {
        top.push(ApplicabilityNode::time_window(from, to));
    }

    let tree = (!top.is_empty()).then(|| ApplicabilityNode::and(top).normalize());
    // With no surviving conditions the tree is the application gate alone
    // (territory-only, lint L8: universal within those nations unless a condition was lost)
    with_application_gate(tree, ctx)
}

/// Repair a previously compiled tree when the source mentions are no longer
/// available. Applies the fixes that don't need provision text:
/// L1 normalise, L2 drop `to` (no law-level sunset is recoverable without text),
/// L4 drop `construction*` unless the law's title grounds it, L5 drop government actors.
pub fn repair_tree(node: ApplicabilityNode, ctx: &LawContext) -> Option<ApplicabilityNode> {
    fn go(node: ApplicabilityNode, ctx: &LawContext, negated: bool) -> Option<ApplicabilityNode> {
        match node {
            ApplicabilityNode::Match { dimension, codes, match_op } => {
                let codes: Vec<String> = codes
                    .into_iter()
                    .filter(|c| !is_gov_actor(c) && !is_jurisdiction(c))
                    .filter(|c| !(negated && is_title_subject(c, ctx)))
                    .filter(|c| {
                        !(c == "construction" || c.starts_with("construction_")) || is_grounded(c, &ctx.title)
                    })
                    .collect();
                (!codes.is_empty()).then_some(ApplicabilityNode::Match { dimension, codes, match_op })
            }
            ApplicabilityNode::And { children } => {
                let kids: Vec<_> = children.into_iter().filter_map(|c| go(c, ctx, negated)).collect();
                (!kids.is_empty()).then(|| ApplicabilityNode::and(kids))
            }
            ApplicabilityNode::Or { children } => {
                let kids: Vec<_> = children.into_iter().filter_map(|c| go(c, ctx, negated)).collect();
                (!kids.is_empty()).then(|| ApplicabilityNode::or(kids))
            }
            ApplicabilityNode::Not { child } => go(*child, ctx, !negated).map(ApplicabilityNode::negate),
            ApplicabilityNode::Conditional { condition, then } => {
                match (go(*condition, ctx, negated), go(*then, ctx, negated)) {
                    (Some(c), Some(t)) => Some(ApplicabilityNode::Conditional { condition: Box::new(c), then: Box::new(t) }),
                    (None, Some(t)) => Some(t),
                    _ => None,
                }
            }
            ApplicabilityNode::TimeWindow { from, inner, .. } => {
                from.map(|f| ApplicabilityNode::TimeWindow { from: Some(f), to: None, inner })
            }
        }
    }
    // Several TimeWindows under the root And: keep one, from the earliest date
    let repaired = go(node, ctx, false)?.normalize();
    let repaired = match repaired {
        ApplicabilityNode::And { children } => {
            let (windows, mut rest): (Vec<_>, Vec<_>) =
                children.into_iter().partition(|c| matches!(c, ApplicabilityNode::TimeWindow { .. }));
            let earliest = windows
                .iter()
                .filter_map(|w| match w {
                    ApplicabilityNode::TimeWindow { from, .. } => from.clone(),
                    _ => None,
                })
                .min();
            if let Some(f) = earliest {
                rest.push(ApplicabilityNode::time_window(Some(&f), None));
            }
            ApplicabilityNode::and(rest)
        }
        other => other,
    };
    with_application_gate(Some(repaired), ctx)
}

/// All codes in a subtree.
pub fn codes_of(node: &ApplicabilityNode) -> Vec<String> {
    match node {
        ApplicabilityNode::Match { codes, .. } => codes.clone(),
        ApplicabilityNode::And { children } | ApplicabilityNode::Or { children } => {
            children.iter().flat_map(codes_of).collect()
        }
        ApplicabilityNode::Not { child } => codes_of(child),
        ApplicabilityNode::Conditional { condition, then } => {
            let mut codes = codes_of(condition);
            codes.extend(codes_of(then));
            codes
        }
        ApplicabilityNode::TimeWindow { inner, .. } => inner.as_ref().map(|n| codes_of(n)).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mention(polarity: &str, entities: &[&str], text: &str) -> MentionInput {
        MentionInput {
            polarity: polarity.into(),
            entities: entities.iter().map(|s| s.to_string()).collect(),
            text: text.into(),
        }
    }

    fn dims() -> HashMap<String, String> {
        [
            ("employer", "personal"),
            ("secretary of state", "personal"),
            ("local authority", "personal"),
            ("scotland", "territorial"),
            ("premises", "territorial"),
            ("construction", "material"),
            ("construction work", "material"),
            ("waste", "material"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
    }

    fn m(dim: &str, codes: &[&str]) -> ApplicabilityNode {
        ApplicabilityNode::match_any(dim, codes.iter().map(|c| c.to_string()).collect())
    }

    #[test]
    fn grounding_accepts_head_word_prefix() {
        assert!(is_grounded("employer", "Every employer shall ensure"));
        assert!(is_grounded("employer", "the employers of such persons"));
        assert!(is_grounded("waste", "controlled waste"));
        assert!(!is_grounded("waste", "This section does not apply to regulations under section 51."));
    }

    #[test]
    fn grounding_short_words_need_whole_word() {
        assert!(is_grounded("gas", "a gas appliance"));
        assert!(!is_grounded("gas", "the gasket shall be"));
    }

    #[test]
    fn construction_hallucination_dropped() {
        // UK_asp_2021_4 s.49(2): the fine-tuned model emitted `construction`
        assert!(!is_grounded("construction", "This section does not apply to regulations under section 51."));
        assert!(!is_grounded("construction_work", "Subsection (1) does not apply to—"));
    }

    #[test]
    fn construction_interpretation_sense_dropped() {
        assert!(!is_grounded("construction", "Interpretation and construction"));
        assert!(!is_grounded("construction", "In the construction of these Regulations, references are to be construed"));
    }

    #[test]
    fn construction_product_sense_dropped() {
        // UK_uksi_1986_1078 Road Vehicles (Construction and Use) Regulations
        assert!(!is_grounded("construction_work", "Road Vehicles (Construction and Use) Regulations"));
        assert!(!is_grounded("construction_work", "a windscreen shall be constructed of specified safety glass"));
        assert!(!is_grounded("construction", "a vehicle constructed or adapted to carry more than 8 passengers"));
    }

    #[test]
    fn construction_building_sense_kept() {
        assert!(is_grounded("construction", "Construction (Design and Management) Regulations"));
        assert!(is_grounded("construction", "These Regulations apply to construction work."));
        assert!(is_grounded("construction_work", "does not apply to construction work carried out by the armed forces"));
        assert!(is_grounded("construction", "the construction of a building"));
    }

    #[test]
    fn commencement_and_sunset_need_self_reference() {
        assert!(is_law_commencement("These Regulations come into force on 1st April 2024."));
        assert!(is_law_commencement("This Act comes into force on the day after Royal Assent."));
        assert!(!is_law_commencement("This Part comes into force on the day after Royal Assent."));
        assert!(is_law_sunset("These Regulations cease to have effect on 1st January 2030."));
        assert!(is_law_sunset("This Order expires at the end of 31st March 2022."));
        assert!(!is_law_sunset("Paragraph (2) ceases to have effect on 26th November 2016."));
        assert!(!is_law_sunset("a stop notice ceases to have effect at the end of the period of 7 days"));
    }

    #[test]
    fn gov_actors_never_gate() {
        let tree = compile_law(
            &[
                mention("AppliesTo", &["employer"], "These Regulations apply to every employer"),
                mention("AppliesTo", &["local authority", "waste"], "a local authority shall collect waste"),
            ],
            &dims(),
            &LawContext::default(),
        )
        .unwrap();
        assert_eq!(tree, ApplicabilityNode::or(vec![m("personal", &["employer"]), m("material", &["waste"])]));
    }

    #[test]
    fn gov_only_mention_dropped() {
        // UK_uksi_2019_421: only `enforcement_authority` was extracted; it must not gate
        let tree = compile_law(
            &[
                mention("AppliesTo", &["enforcement authority"], "An enforcement authority must enforce these Regulations"),
                mention("AppliesTo", &["employer"], "Every employer shall"),
            ],
            &dims(),
            &LawContext::default(),
        )
        .unwrap();
        assert_eq!(tree, m("personal", &["employer"]));
    }

    #[test]
    fn repair_applies_text_free_fixes() {
        let branch = ApplicabilityNode::and(vec![m("personal", &["secretary_of_state", "employer"]), m("material", &["construction"])]);
        let tree = ApplicabilityNode::and(vec![
            ApplicabilityNode::or(vec![branch.clone(), branch]),
            ApplicabilityNode::time_window(Some("2015-04-01"), None),
            ApplicabilityNode::time_window(None, Some("2016-11-26")),
            ApplicabilityNode::time_window(Some("2013-11-26"), None),
        ]);
        let repaired = repair_tree(tree, &LawContext::default()).unwrap();
        assert_eq!(
            repaired,
            ApplicabilityNode::and(vec![
                m("personal", &["employer"]),
                ApplicabilityNode::time_window(Some("2013-11-26"), None),
            ])
        );
    }

    #[test]
    fn application_gate_at_root_and_jurisdiction_lifted() {
        // UK_asp_2021_4: Not(scotland) disapplied the law's own jurisdiction (L3)
        let ctx = LawContext { application: Some(vec!["scotland".into()]), ..Default::default() };
        let tree = compile_law(
            &[
                mention("AppliesTo", &["employer", "Scotland"], "Every employer in Scotland shall"),
                mention("DisappliesTo", &["Scotland"], "Section 3 does not apply in Scotland to a relevant body"),
                mention("AppliesTo", &["waste"], "waste collected in England and Wales"),
            ],
            &dims(),
            &ctx,
        )
        .unwrap();
        assert_eq!(
            tree,
            ApplicabilityNode::and(vec![
                m("territorial", &["scotland"]),
                ApplicabilityNode::or(vec![m("personal", &["employer"]), m("material", &["waste"])]),
            ])
        );
    }

    #[test]
    fn jurisdiction_only_law_is_gate_only() {
        let ctx = LawContext { application: Some(vec!["england".into()]), ..Default::default() };
        let tree = compile_law(
            &[mention("AppliesTo", &["England"], "These Regulations apply in relation to England only.")],
            &dims(),
            &ctx,
        );
        assert_eq!(tree, Some(m("territorial", &["england"])));
        // Without an application, nothing to publish
        let tree = compile_law(
            &[mention("AppliesTo", &["England"], "These Regulations apply in relation to England only.")],
            &dims(),
            &LawContext::default(),
        );
        assert!(tree.is_none());
    }

    #[test]
    fn law_level_disapplication_detection() {
        assert!(is_law_level_disapplication("These Regulations do not apply to domestic premises."));
        assert!(is_law_level_disapplication("(2) These Regulations shall not apply to the master or crew of a ship"));
        assert!(is_law_level_disapplication("Subject to paragraph (3), these Regulations do not apply to work on a ship"));
        assert!(is_law_level_disapplication("Nothing in these Regulations applies to the armed forces"));
        assert!(!is_law_level_disapplication("Regulation 9 does not apply to construction work on domestic premises"));
        assert!(!is_law_level_disapplication("This regulation does not apply to a domestic client."));
        assert!(!is_law_level_disapplication("Paragraph (2) of this regulation does not apply where these Regulations apply"));
    }

    #[test]
    fn law_never_disapplies_its_title_subject() {
        // UK_uksi_2007_3075 reg.4(6): boundary with the land-based Noise at Work Regs
        let ctx = LawContext {
            title: "Merchant Shipping and Fishing Vessels (Control of Noise at Work) Regulations".into(),
            application: Some(vec!["england".into()]),
        };
        let mut d = dims();
        d.insert("at work".into(), "conditional".into());
        let tree = compile_law(
            &[
                mention("AppliesTo", &["employer"], "Every employer shall"),
                mention(
                    "DisappliesTo",
                    &["at work", "waste"],
                    "These regulations shall not apply to activities to which the Control of Noise at Work Regulations 2005 apply, or to waste",
                ),
            ],
            &d,
            &ctx,
        )
        .unwrap();
        assert_eq!(
            tree,
            ApplicabilityNode::and(vec![
                m("territorial", &["england"]),
                m("personal", &["employer"]),
                m("material", &["waste"]).negate(),
            ])
        );
        // repair_tree applies the same guard to stored trees
        let stored = ApplicabilityNode::and(vec![
            m("personal", &["employer"]),
            m("conditional", &["at_work"]).negate(),
        ]);
        assert_eq!(
            repair_tree(stored, &ctx),
            Some(ApplicabilityNode::and(vec![m("territorial", &["england"]), m("personal", &["employer"])]))
        );
    }

    #[test]
    fn citations_do_not_ground() {
        let text = "These Regulations come into force immediately after the Mines Regulations 2014.";
        assert!(!is_grounded("mine", &strip_citations(text)));
        let text = "orders made under section 17 of the Water (Scotland) Act 1980";
        assert!(!is_grounded("scotland", &strip_citations(text)));
        assert!(is_grounded("scotland", &strip_citations("These Regulations extend to Scotland only.")));
    }

    #[test]
    fn title_extracted_from_citation_provision() {
        assert_eq!(
            title_from_citation("These Regulations may be cited as the Construction (Design and Management) Regulations 2015 and come into force on 6th April 2015"),
            Some("Construction (Design and Management) Regulations 2015".into())
        );
        assert_eq!(
            title_from_citation("This Act may be cited as the Explosives Act (Northern Ireland) 1970."),
            Some("Explosives Act (Northern Ireland) 1970".into())
        );
        assert_eq!(title_from_citation("These Regulations come into force on 1st April 2004."), None);
    }

    #[test]
    fn title_grounds_the_law_subject() {
        // CDM 2015: transitional provisions don't repeat "construction" but the title does
        let ctx = LawContext { title: "Construction (Design and Management) Regulations".into(), ..Default::default() };
        let tree = compile_law(
            &[mention("AppliesTo", &["construction project"], "These Regulations apply to a relevant project with the modifications")],
            &dims(),
            &ctx,
        );
        assert_eq!(tree, Some(m("material", &["construction_project"])));
    }

    #[test]
    fn duplicate_mentions_compile_once() {
        let text = "does not apply to any vehicle on the premises of a waste site";
        let tree = compile_law(
            &[
                mention("AppliesTo", &["waste"], text),
                mention("AppliesTo", &["waste"], text),
                mention("AppliesTo", &["waste"], text),
            ],
            &dims(),
            &LawContext::default(),
        )
        .unwrap();
        assert_eq!(tree, m("material", &["waste"]));
    }

    #[test]
    fn provision_sunset_and_non_commencement_dates_ignored() {
        // UK_anaw_2017_2: `to 2017-04-15` came from a provision-level "ceases to have effect"
        let tree = compile_law(
            &[
                mention("AppliesTo", &["employer"], "Every employer shall"),
                mention("AppliesTo", &["2017-04-03"], "The following provisions come into force on the day on which this Act receives Royal Assent—"),
                mention("DisappliesTo", &["2017-04-15"], "the prohibition imposed under subsection (3)(c) ceases to have effect"),
                mention("AppliesTo", &["2021-03-04"], "No regulations may be made under section 1(1) after the end of the period"),
            ],
            &dims(),
            &LawContext::default(),
        )
        .unwrap();
        assert_eq!(tree, m("personal", &["employer"]));
    }

    #[test]
    fn law_commencement_earliest_and_law_sunset_kept() {
        let tree = compile_law(
            &[
                mention("AppliesTo", &["employer"], "Every employer shall"),
                mention("AppliesTo", &["2024-10-01"], "These Regulations come into force on 1st October 2024 except regulation 5"),
                mention("AppliesTo", &["2024-04-06"], "This regulation and regulation 3 of these Regulations come into force on 6th April 2024"),
                mention("DisappliesTo", &["2029-10-01"], "These Regulations cease to have effect on 1st October 2029."),
            ],
            &dims(),
            &LawContext::default(),
        )
        .unwrap();
        assert_eq!(
            tree,
            ApplicabilityNode::and(vec![
                m("personal", &["employer"]),
                ApplicabilityNode::time_window(Some("2024-04-06"), Some("2029-10-01")),
            ])
        );
    }

    #[test]
    fn sunset_before_commencement_dropped() {
        let tree = compile_law(
            &[
                mention("AppliesTo", &["employer"], "Every employer shall"),
                mention("AppliesTo", &["2024-10-01"], "These Regulations come into force on 1st October 2024"),
                mention("DisappliesTo", &["2020-01-01"], "These Regulations cease to have effect on 1st January 2020"),
            ],
            &dims(),
            &LawContext::default(),
        )
        .unwrap();
        assert_eq!(
            tree,
            ApplicabilityNode::and(vec![
                m("personal", &["employer"]),
                ApplicabilityNode::time_window(Some("2024-10-01"), None),
            ])
        );
    }

    #[test]
    fn disapplies_conflicting_with_applies_filtered() {
        let tree = compile_law(
            &[
                mention("AppliesTo", &["construction work"], "These Regulations apply to construction work"),
                mention("DisappliesTo", &["construction work"], "Regulation 9 does not apply to construction work on domestic premises"),
                mention("DisappliesTo", &["waste"], "These Regulations do not apply to waste"),
                mention("DisappliesTo", &["employer"], "Regulation 5 does not apply to an employer of fewer than 5"),
            ],
            &dims(),
            &LawContext::default(),
        )
        .unwrap();
        assert_eq!(
            tree,
            ApplicabilityNode::and(vec![m("material", &["construction_work"]), m("material", &["waste"]).negate()])
        );
    }

    #[test]
    fn nothing_grounded_returns_none() {
        let tree = compile_law(
            &[mention("AppliesTo", &["construction"], "This section does not apply to regulations under section 51.")],
            &dims(),
            &LawContext::default(),
        );
        assert!(tree.is_none());
    }
}
