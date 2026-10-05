//! Correlative actor inference rules.
//!
//! When regex finds Actor A with a specific position, infer Actor B
//! with a correlative position (Hohfeldian correlatives).

use std::collections::HashMap;

use super::actors::is_government;

pub(crate) static RULES_YAML: &str = include_str!("../../data/correlative-rules.yaml");

/// A correlative inference rule.
#[derive(Debug, serde::Deserialize)]
pub struct CorrelativeRule {
    pub trigger_actor: String,
    pub trigger_position: String,
    #[serde(default)]
    pub trigger_drrp: Option<String>,
    pub inferred_actor: String,
    pub inferred_category: String,
    pub inferred_position: String,
    #[serde(default)]
    pub inferred_drrp: Option<String>,
}

/// An inferred actor ready for upsert.
#[derive(Debug)]
pub struct InferredActor {
    pub section_id: String,
    pub actor_label: String,
    pub actor_category: String,
    pub drrp: Option<String>,
    pub position: String,
}

/// Load correlative rules from the embedded YAML.
pub fn load_rules() -> Vec<CorrelativeRule> {
    serde_yaml::from_str(RULES_YAML).expect("failed to parse correlative-rules.yaml")
}

/// Apply correlative rules to a set of actors grouped by section_id.
///
/// For each section, checks if any existing actor matches a rule trigger.
/// If so, and if the inferred actor doesn't already exist in that section,
/// produces an `InferredActor`.
///
/// Only regex-tier actors are eligible triggers (no cascading).
pub fn apply_rules(
    rules: &[CorrelativeRule],
    actors_by_section: &HashMap<String, Vec<(String, String, Option<String>, Option<String>)>>,
    // section_id → Vec<(actor_label, actor_category, regex_drrp, regex_position)>
) -> Vec<InferredActor> {
    let mut inferred = Vec::new();

    for (section_id, actors) in actors_by_section {
        // Collect existing labels in this section for duplicate check
        let existing_labels: std::collections::HashSet<&str> =
            actors.iter().map(|(label, _, _, _)| label.as_str()).collect();

        for rule in rules {
            // Check if any actor in this section matches the trigger
            let triggered = actors.iter().any(|(label, _cat, drrp, pos)| {
                label == &rule.trigger_actor
                    && pos.as_deref() == Some(rule.trigger_position.as_str())
                    && match &rule.trigger_drrp {
                        Some(td) => drrp.as_deref() == Some(td.as_str()),
                        None => true,
                    }
            });

            if !triggered {
                continue;
            }

            // Don't infer if the actor already exists in this section
            if existing_labels.contains(rule.inferred_actor.as_str()) {
                continue;
            }

            // Inherit DRRP from trigger if not specified in rule
            let drrp = rule.inferred_drrp.clone().or_else(|| {
                actors
                    .iter()
                    .find(|(label, _, _, _)| label == &rule.trigger_actor)
                    .and_then(|(_, _, drrp, _)| drrp.clone())
            });

            inferred.push(InferredActor {
                section_id: section_id.clone(),
                actor_label: rule.inferred_actor.clone(),
                actor_category: rule.inferred_category.clone(),
                drrp,
                position: rule.inferred_position.clone(),
            });
        }
    }

    inferred
}

/// Access wording: a duty to make something available to, or open to
/// inspection or copying by, a governed party (fractalatai #67).
static ACCESS_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(
        r"(?i)(for (public )?inspection by|available (to|for) (inspection by )?(members of )?the public|open to (public )?inspection|available for (public )?inspection|facilities for (obtaining|taking|making) cop(y|ies)|(supply|furnish|provide|send) .{0,80}cop(y|ies) .{0,80}(on|upon) (request|payment|application)|(on|upon) (request|payment|application).{0,80}(supply|furnish|provide|send) .{0,40}cop(y|ies))",
    )
    .unwrap()
});
static PUBLIC_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)\b(the public|members of the public|public inspection)\b").unwrap());
static PERSON_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"(?i)\b(any|a|every) person\b|\bpersons\b").unwrap());

/// The governed actor an access clause is addressed to, by label.
fn addressed(label: &str, text: &str) -> bool {
    (label.contains("Public") && PUBLIC_RE.is_match(text)) || (label == "Ind: Person" && PERSON_RE.is_match(text))
}

/// Implied rights (fractalatai #67). Where a provision's wording grants a
/// governed party access (inspection, copies, supply on request) and a
/// government actor holds an active Obligation, in the provision or an
/// ancestor stem, the governed actor named in the clause holds an implied
/// **Liberty** (position active). Legal types that as a Right; the authority's
/// Obligation stays a Responsibility.
///
/// Only upgrades actors already extracted (never invents one), only governed
/// actors not already active, and never fires on enforcement wording (serving
/// notice on an operator), which has no access clause.
///
/// `texts`: section_id → provision text. `actors_by_section`: section_id →
/// (label, category, drrp, position), as for [`apply_rules`].
pub fn infer_access_rights(
    texts: &HashMap<String, String>,
    actors_by_section: &HashMap<String, Vec<(String, String, Option<String>, Option<String>)>>,
) -> Vec<InferredActor> {
    let gov_obliged = |sid: &str| {
        actors_by_section.get(sid).is_some_and(|actors| {
            actors.iter().any(|(label, _, drrp, pos)| {
                is_government(label) && drrp.as_deref() == Some("Obligation") && pos.as_deref() == Some("active")
            })
        })
    };
    let mut out = Vec::new();
    for (sid, actors) in actors_by_section {
        let Some(text) = texts.get(sid) else { continue };
        if !ACCESS_RE.is_match(text) {
            continue;
        }
        if !(gov_obliged(sid) || super::amendment::ancestors(sid).iter().any(|a| gov_obliged(a))) {
            continue;
        }
        for (label, category, _drrp, pos) in actors {
            if is_government(label) || pos.as_deref() == Some("active") || !addressed(label, text) {
                continue;
            }
            out.push(InferredActor {
                section_id: sid.clone(),
                actor_label: label.clone(),
                actor_category: category.clone(),
                drrp: Some("Liberty".to_string()),
                position: "active".to_string(),
            });
        }
    }
    out
}

/// Layer-1b correlative types (DRRP-CLASSIFICATION.md, #72).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrelativeType {
    /// The duty is owed to this actor (counterparty of an Obligation, or #67)
    ClaimRight,
    /// Exposed to a government actor's Power
    Liability,
    /// Can't prevent a governed actor's Right
    NoRight,
    /// The duty protects this actor (beneficiary of an Obligation)
    Protected,
}

/// One `actors[].correlatives` item: `{type, to}`, where `to` is the active holder's label.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Correlative {
    #[serde(rename = "type")]
    pub kind: CorrelativeType,
    pub to: String,
}

/// A reconciled actor on one provision, as layer 1b reads it.
#[derive(Debug, Clone)]
pub struct PositionedActor {
    pub label: String,
    /// Reconciled type: `Obligation`, `Liberty`, `none`
    pub drrp: String,
    /// Reconciled position: `active`, `counterparty`, `beneficiary`, `mentioned`
    pub position: String,
    /// Active Liberty inferred by the #67 access rule
    pub access_inferred: bool,
}

impl PositionedActor {
    fn holds(&self, drrp: &str) -> bool {
        self.position == "active" && self.drrp == drrp
    }
}

/// Derive layer-1b correlatives for every actor in a law
/// (DRRP-CLASSIFICATION.md layer 1b, #72). Keyed by (section_id, label);
/// every actor gets an entry, `[]` when it holds none.
///
/// - Non-active actors pair with each active holder in the same provision:
///   counterparty → `claim_right` (Obligation), `liability` (government
///   Liberty = Power) or `no_right` (governed Liberty = Right); beneficiary of
///   an Obligation → `protected`; mentioned → nothing. No active holder
///   (holder unknown) → nothing.
/// - An active actor gets one only in the #67 case: an inferred access Liberty
///   holds a `claim_right` against the government actors whose access
///   Obligation it is, in the provision or else the nearest ancestor stem.
///
/// Derived from positions only: never feeds DRRP or the verdict.
pub fn derive_correlatives(
    sections: &HashMap<String, Vec<PositionedActor>>,
) -> HashMap<(String, String), Vec<Correlative>> {
    let gov_obligors = |sid: &str| -> Vec<String> {
        sections
            .get(sid)
            .map(|actors| {
                actors
                    .iter()
                    .filter(|a| a.holds("Obligation") && is_government(&a.label))
                    .map(|a| a.label.clone())
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut out = HashMap::new();
    for (sid, actors) in sections {
        let holders: Vec<&PositionedActor> =
            actors.iter().filter(|a| a.holds("Obligation") || a.holds("Liberty")).collect();
        for a in actors {
            let mut cs: Vec<Correlative> = Vec::new();
            match a.position.as_str() {
                "counterparty" | "beneficiary" => {
                    for h in holders.iter().filter(|h| h.label != a.label) {
                        let kind = match (a.position.as_str(), h.drrp.as_str()) {
                            ("counterparty", "Obligation") => CorrelativeType::ClaimRight,
                            ("counterparty", _) if is_government(&h.label) => CorrelativeType::Liability,
                            ("counterparty", _) => CorrelativeType::NoRight,
                            ("beneficiary", "Obligation") => CorrelativeType::Protected,
                            _ => continue,
                        };
                        cs.push(Correlative { kind, to: h.label.clone() });
                    }
                }
                "active" if a.access_inferred && a.holds("Liberty") => {
                    let mut obligors = gov_obligors(sid);
                    if obligors.is_empty() {
                        obligors = super::amendment::ancestors(sid)
                            .iter()
                            .map(|anc| gov_obligors(anc))
                            .find(|o| !o.is_empty())
                            .unwrap_or_default();
                    }
                    cs.extend(obligors.into_iter().map(|to| Correlative { kind: CorrelativeType::ClaimRight, to }));
                }
                _ => {}
            }
            cs.sort();
            cs.dedup();
            out.insert((sid.clone(), a.label.clone()), cs);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(label: &str, drrp: Option<&str>, pos: &str) -> (String, String, Option<String>, Option<String>) {
        (label.into(), label.split(':').next().unwrap().into(), drrp.map(String::from), Some(pos.into()))
    }

    #[test]
    fn access_rights_inferred_for_addressed_governed_actor() {
        let mut texts = HashMap::new();
        let mut actors = HashMap::new();
        // SSI 2006/209 reg.35(1): obligation and access clause in one provision
        texts.insert("L:reg.35(1)".into(), "A local authority shall make available for inspection by the public, in such places as it reasonably considers".into());
        actors.insert("L:reg.35(1)".into(), vec![
            act("Gvt: Authority: Local", Some("Obligation"), "active"),
            act("Ind: Public", Some("Obligation"), "counterparty"),
        ]);
        // reg.35(2): copies for any person
        texts.insert("L:reg.35(2)".into(), "A local authority shall afford to any person, facilities for obtaining copies of entries, on payment of reasonable charges".into());
        actors.insert("L:reg.35(2)".into(), vec![
            act("Gvt: Authority: Local", Some("Obligation"), "active"),
            act("Ind: Person", Some("Obligation"), "counterparty"),
        ]);
        // EPA 1990 s.20(7): duty in the stem, access clause in the child
        texts.insert("E:s.20(7)".into(), "It shall be the duty of each enforcing authority—".into());
        actors.insert("E:s.20(7)".into(), vec![act("Gvt: Authority: Enforcement", Some("Obligation"), "active")]);
        texts.insert("E:s.20(7)(a)".into(), "to secure that the registers maintained by them under this section are available, at all reasonable times, for inspection by the public free of charge".into());
        actors.insert("E:s.20(7)(a)".into(), vec![act("Ind: Public", None, "mentioned")]);
        let mut got: Vec<(String, String)> = infer_access_rights(&texts, &actors)
            .into_iter()
            .map(|a| { assert_eq!((a.drrp.as_deref(), a.position.as_str()), (Some("Liberty"), "active")); (a.section_id, a.actor_label) })
            .collect();
        got.sort();
        assert_eq!(got, vec![
            ("E:s.20(7)(a)".into(), "Ind: Public".into()),
            ("L:reg.35(1)".into(), "Ind: Public".into()),
            ("L:reg.35(2)".into(), "Ind: Person".into()),
        ]);
    }

    #[test]
    fn access_rights_not_inferred_for_enforcement_or_without_gov_duty() {
        let mut texts = HashMap::new();
        let mut actors = HashMap::new();
        // Enforcement: notice served on the operator, no access clause
        texts.insert("L:reg.9".into(), "The authority shall serve a notice on the operator requiring the operator to take such steps".into());
        actors.insert("L:reg.9".into(), vec![
            act("Gvt: Authority: Enforcement", Some("Obligation"), "active"),
            act("Org: Operator", Some("Obligation"), "counterparty"),
        ]);
        // Access wording but no government duty anywhere up the tree
        texts.insert("L:reg.10".into(), "The register shall be available for inspection by the public".into());
        actors.insert("L:reg.10".into(), vec![act("Ind: Public", None, "mentioned")]);
        // Access clause, government duty, but the governed actor isn't the addressee
        texts.insert("L:reg.11".into(), "The Agency must make the plan available for inspection by the public and send a copy to the operator".into());
        actors.insert("L:reg.11".into(), vec![
            act("Gvt: Agency", Some("Obligation"), "active"),
            act("Org: Operator", Some("Obligation"), "counterparty"),
        ]);
        assert!(infer_access_rights(&texts, &actors).is_empty());
    }

    fn pa(label: &str, drrp: &str, pos: &str) -> PositionedActor {
        PositionedActor { label: label.into(), drrp: drrp.into(), position: pos.into(), access_inferred: false }
    }

    fn corr(got: &HashMap<(String, String), Vec<Correlative>>, sid: &str, label: &str) -> Vec<(CorrelativeType, String)> {
        got[&(sid.to_string(), label.to_string())].iter().map(|c| (c.kind, c.to.clone())).collect()
    }

    #[test]
    fn worked_examples_layer_1b() {
        use CorrelativeType::*;
        let mut s = HashMap::new();
        // HSWA s.2(1): protected-interest duty → beneficiary → protected
        s.insert("H:s.2(1)".to_string(), vec![pa("Org: Employer", "Obligation", "active"), pa("Ind: Employee", "Obligation", "beneficiary")]);
        // HSWA s.2(2)(c): employees receive the training → counterparty → claim_right
        s.insert("H:s.2(2)(c)".to_string(), vec![pa("Org: Employer", "Obligation", "active"), pa("Ind: Employee", "Obligation", "counterparty")]);
        // HSWA s.3(1): persons not employed, protected
        s.insert("H:s.3(1)".to_string(), vec![pa("Org: Employer", "Obligation", "active"), pa("Ind: Person", "Obligation", "beneficiary")]);
        // HSWA s.9: no charge levied on employees → counterparty → claim_right
        s.insert("H:s.9".to_string(), vec![pa("Org: Employer", "Obligation", "active"), pa("Ind: Employee", "none", "counterparty")]);
        // PPE Regs reg.4: "ensure PPE is provided to employees" → counterparty
        s.insert("P:reg.4(1)".to_string(), vec![pa("Org: Employer", "Obligation", "active"), pa("Ind: Employee", "Obligation", "counterparty")]);
        // Water Act s.82(2)(c)-style Power: a counterparty is liable
        s.insert("W:s.82(2)(c)".to_string(), vec![pa("Gvt: Minister", "Liberty", "active"), pa("Org: Undertaker", "Liberty", "counterparty")]);
        // A governed Right: the counterparty can't prevent it
        s.insert("R:s.5".to_string(), vec![pa("Ind: Owner", "Liberty", "active"), pa("Org: Occupier", "none", "counterparty")]);
        // Notice service: the operator notifies the authority (a regulator holds a claim_right)
        s.insert("N:reg.7".to_string(), vec![pa("Org: Operator", "Obligation", "active"), pa("Gvt: Authority: Enforcement", "Obligation", "counterparty"), pa("Gvt: Minister", "none", "mentioned")]);
        // Holder unknown: no active holder → nothing
        s.insert("U:s.1".to_string(), vec![pa("Ind: Person", "Obligation", "counterparty")]);
        let got = derive_correlatives(&s);
        assert_eq!(corr(&got, "H:s.2(1)", "Ind: Employee"), vec![(Protected, "Org: Employer".into())]);
        assert_eq!(corr(&got, "H:s.2(1)", "Org: Employer"), vec![]);
        assert_eq!(corr(&got, "H:s.2(2)(c)", "Ind: Employee"), vec![(ClaimRight, "Org: Employer".into())]);
        assert_eq!(corr(&got, "H:s.3(1)", "Ind: Person"), vec![(Protected, "Org: Employer".into())]);
        assert_eq!(corr(&got, "H:s.9", "Ind: Employee"), vec![(ClaimRight, "Org: Employer".into())]);
        assert_eq!(corr(&got, "P:reg.4(1)", "Ind: Employee"), vec![(ClaimRight, "Org: Employer".into())]);
        assert_eq!(corr(&got, "W:s.82(2)(c)", "Org: Undertaker"), vec![(Liability, "Gvt: Minister".into())]);
        assert_eq!(corr(&got, "R:s.5", "Org: Occupier"), vec![(NoRight, "Ind: Owner".into())]);
        assert_eq!(corr(&got, "N:reg.7", "Gvt: Authority: Enforcement"), vec![(ClaimRight, "Org: Operator".into())]);
        assert_eq!(corr(&got, "N:reg.7", "Gvt: Minister"), vec![]);
        assert_eq!(corr(&got, "U:s.1", "Ind: Person"), vec![]);
    }

    #[test]
    fn one_pair_per_active_holder_and_no_co_holder_pairs() {
        use CorrelativeType::*;
        let mut s = HashMap::new();
        // A counterparty to a government Power and a governed Right gets both
        s.insert("X:s.1".to_string(), vec![
            pa("Gvt: Agency", "Liberty", "active"),
            pa("Ind: Owner", "Liberty", "active"),
            pa("Org: Operator", "none", "counterparty"),
        ]);
        // Co-holders of one Obligation get nothing against each other; a beneficiary of a Liberty gets nothing
        s.insert("X:s.2".to_string(), vec![
            pa("Org: Employer", "Obligation", "active"),
            pa("Org: Operator", "Obligation", "active"),
        ]);
        s.insert("X:s.3".to_string(), vec![pa("Gvt: Agency", "Liberty", "active"), pa("Ind: Public", "none", "beneficiary")]);
        let got = derive_correlatives(&s);
        assert_eq!(corr(&got, "X:s.1", "Org: Operator"), vec![(Liability, "Gvt: Agency".into()), (NoRight, "Ind: Owner".into())]);
        assert_eq!(corr(&got, "X:s.2", "Org: Employer"), vec![]);
        assert_eq!(corr(&got, "X:s.3", "Ind: Public"), vec![]);
    }

    #[test]
    fn access_inferred_liberty_claims_against_stem_obligor() {
        use CorrelativeType::*;
        let mut s = HashMap::new();
        // EPA 1990 s.20(7): the authority's duty in the stem, the public's access in (a)
        s.insert("E:s.20(7)".to_string(), vec![pa("Gvt: Authority: Enforcement", "Obligation", "active")]);
        let mut public = pa("Ind: Public", "Liberty", "active");
        public.access_inferred = true;
        s.insert("E:s.20(7)(a)".to_string(), vec![public.clone()]);
        // Same-provision case; an LLM-active Liberty (not inferred) gets nothing
        s.insert("L:reg.35(1)".to_string(), vec![pa("Gvt: Authority: Local", "Obligation", "active"), public, pa("Ind: Person", "Liberty", "active")]);
        let got = derive_correlatives(&s);
        assert_eq!(corr(&got, "E:s.20(7)(a)", "Ind: Public"), vec![(ClaimRight, "Gvt: Authority: Enforcement".into())]);
        assert_eq!(corr(&got, "L:reg.35(1)", "Ind: Public"), vec![(ClaimRight, "Gvt: Authority: Local".into())]);
        assert_eq!(corr(&got, "L:reg.35(1)", "Ind: Person"), vec![]);
    }

    #[test]
    fn pair_type_matches_holder_class() {
        // legal's check: liability only against government, no_right only against governed
        let mut s = HashMap::new();
        s.insert("X:s.1".to_string(), vec![
            pa("Gvt: Agency", "Liberty", "active"),
            pa("Ind: Owner", "Liberty", "active"),
            pa("Org: Operator", "none", "counterparty"),
        ]);
        for c in derive_correlatives(&s).values().flatten() {
            match c.kind {
                CorrelativeType::Liability => assert!(is_government(&c.to)),
                CorrelativeType::NoRight => assert!(!is_government(&c.to)),
                _ => {}
            }
        }
        let json = serde_json::to_string(&Correlative { kind: CorrelativeType::ClaimRight, to: "Org: Employer".into() }).unwrap();
        assert_eq!(json, r#"{"type":"claim_right","to":"Org: Employer"}"#);
    }

    #[test]
    fn load_rules_parses() {
        let rules = load_rules();
        assert!(rules.len() >= 2);
        assert_eq!(rules[0].trigger_actor, "Ind: Employee");
        assert_eq!(rules[0].inferred_actor, "Org: Employer");
    }

    #[test]
    fn apply_employee_employer_rule() {
        let rules = load_rules();
        let mut sections = HashMap::new();
        sections.insert(
            "test:s.1".to_string(),
            vec![(
                "Ind: Employee".to_string(),
                "Ind".to_string(),
                Some("Obligation".to_string()),
                Some("active".to_string()),
            )],
        );

        let inferred = apply_rules(&rules, &sections);
        assert_eq!(inferred.len(), 1);
        assert_eq!(inferred[0].actor_label, "Org: Employer");
        assert_eq!(inferred[0].position, "counterparty");
        assert_eq!(inferred[0].drrp, Some("Obligation".to_string()));
    }

    #[test]
    fn no_duplicate_inference() {
        let rules = load_rules();
        let mut sections = HashMap::new();
        sections.insert(
            "test:s.1".to_string(),
            vec![
                (
                    "Ind: Employee".to_string(),
                    "Ind".to_string(),
                    Some("Obligation".to_string()),
                    Some("active".to_string()),
                ),
                (
                    "Org: Employer".to_string(),
                    "Org".to_string(),
                    Some("Obligation".to_string()),
                    Some("counterparty".to_string()),
                ),
            ],
        );

        let inferred = apply_rules(&rules, &sections);
        assert_eq!(inferred.len(), 0, "should not infer actor that already exists");
    }
}
