//! Law-level DRRP rolled up from reconciled provision-level actor signals
//! (`provision_actors.drrp`), run by `taxa backfill` (fractalatai #55).
//!
//! Replaces the regex-only roll-up in `taxa parse` (`write_law_taxa`) for the
//! law-level columns sertantai-legal's `MakingResolver` reads:
//! Duty/Responsibility → making; Rights/Powers only → empowering;
//! empty lists → no_obligations; NULL → no verdict.
//!
//! Layers 4–5 of `docs/architecture/DRRP-CLASSIFICATION.md` (#68): only
//! **active** actors hold a type, the holder class comes from the actor
//! dictionary, and an Obligation with no known holder blocks a
//! no_obligations/empowering verdict (it may be a Duty we can't yet name).
//!
//! Policy (Jason, 2026-09-25): an amending instrument that inserts duties into a
//! principal instrument is not Making. Signals in amendment text (a provision
//! that is an amendment instruction, or sits under one) are excluded.

use std::collections::BTreeSet;

use super::correlatives::{CorrelativeType, PositionedActor, derive_correlatives};

/// One reconciled actor signal on one provision.
#[derive(Debug, Clone)]
pub struct ActorSignal {
    /// Full section id, e.g. `UK_uksi_2008_198:reg.2(3)(a)`
    pub section_id: String,
    /// Actor label, e.g. `Ind: Person`, `Gvt: Minister`
    pub actor_label: String,
    /// Reconciled DRRP: `Obligation`, `Liberty`, `none`
    pub drrp: String,
    /// Reconciled position: `active`, `counterparty`, `beneficiary`, `mentioned`
    pub position: String,
    /// Active Liberty inferred by the #67 access rule (a `claim_right` source, layer 1b)
    pub access_inferred: bool,
}

/// (holder, duty_type, clause, article), the DuckDB `duties`/`rights`/... struct.
pub type DrrpEntry = (String, String, String, String);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LawDrrp {
    pub duty_holders: BTreeSet<String>,
    pub rights_holders: BTreeSet<String>,
    pub responsibility_holders: BTreeSet<String>,
    pub power_holders: BTreeSet<String>,
    pub duty_types: BTreeSet<String>,
    pub roles: BTreeSet<String>,
    pub roles_gvt: BTreeSet<String>,
    pub duties: Vec<DrrpEntry>,
    pub rights: Vec<DrrpEntry>,
    pub responsibilities: Vec<DrrpEntry>,
    pub powers: Vec<DrrpEntry>,
    /// Signals dropped because they sit in amendment text
    pub excluded_amendment: usize,
    /// Obligation/Liberty signals on non-active actors: they hold nothing (layer 1)
    pub excluded_non_active: usize,
    /// In-scope provisions with an Obligation and no known holder
    pub holder_unknown: usize,
    /// Layer-1b correlative holders (#72): who is owed a duty, exposed to a
    /// Power, or protected by a duty. Never counted in the verdict.
    pub claim_holders: BTreeSet<String>,
    pub liability_holders: BTreeSet<String>,
    pub protected_holders: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Making,
    Empowering,
    NoObligations,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Making => "making",
            Self::Empowering => "empowering",
            Self::NoObligations => "no_obligations",
        }
    }
}

impl LawDrrp {
    /// The layer-5 verdict. `None` (no verdict) when there's no Duty or
    /// Responsibility but some Obligation has no known holder: it may be a
    /// Duty, so neither empowering nor no_obligations is evidenced.
    pub fn verdict(&self) -> Option<Verdict> {
        if !self.duties.is_empty() || !self.responsibilities.is_empty() {
            Some(Verdict::Making)
        } else if self.holder_unknown > 0 {
            None
        } else if !self.rights.is_empty() || !self.powers.is_empty() {
            Some(Verdict::Empowering)
        } else {
            Some(Verdict::NoObligations)
        }
    }
}

pub use super::amendment::{ancestors, is_amendment_instruction};

pub use super::actors::is_government;

fn clause_preview(text: &str) -> String {
    let clean: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() > 200 {
        format!("{}...", clean.chars().take(200).collect::<String>())
    } else {
        clean
    }
}

/// Aggregate one law. `holder_unknown` lists provisions with an Obligation
/// and no active actor; `text_of(section_id)` returns the provision's own text.
pub fn aggregate(
    signals: &[ActorSignal],
    holder_unknown: &[String],
    text_of: impl Fn(&str) -> Option<String>,
) -> LawDrrp {
    let mut law = LawDrrp::default();
    let in_amendment = |sid: &str| {
        text_of(sid).is_some_and(|t| is_amendment_instruction(&t))
            || ancestors(sid)
                .iter()
                .any(|a| text_of(a).is_some_and(|t| is_amendment_instruction(&t)))
    };

    for s in signals {
        if s.drrp != "Obligation" && s.drrp != "Liberty" {
            continue;
        }
        if in_amendment(&s.section_id) {
            law.excluded_amendment += 1;
            continue;
        }
        if s.position != "active" {
            law.excluded_non_active += 1;
            continue;
        }
        let gov = is_government(&s.actor_label);
        if gov {
            law.roles_gvt.insert(s.actor_label.clone());
        } else {
            law.roles.insert(s.actor_label.clone());
        }
        let (holders, entries, dt) = match (s.drrp.as_str(), gov) {
            ("Obligation", false) => (&mut law.duty_holders, &mut law.duties, "Obligation"),
            ("Obligation", true) => (&mut law.responsibility_holders, &mut law.responsibilities, "Responsibility"),
            ("Liberty", false) => (&mut law.rights_holders, &mut law.rights, "Liberty"),
            _ => (&mut law.power_holders, &mut law.powers, "Power"),
        };
        holders.insert(s.actor_label.clone());
        law.duty_types.insert(dt.to_string());
        let article = s.section_id.split_once(':').map(|(_, a)| a).unwrap_or(&s.section_id).to_string();
        let clause = text_of(&s.section_id).map(|t| clause_preview(&t)).unwrap_or_default();
        entries.push((s.actor_label.clone(), dt.to_uppercase(), clause, article));
    }
    law.holder_unknown = holder_unknown.iter().filter(|sid| !in_amendment(sid)).count();

    // Layer 1b (#72): correlatives over the same in-scope provisions
    let mut sections: std::collections::HashMap<String, Vec<PositionedActor>> = std::collections::HashMap::new();
    for s in signals.iter().filter(|s| !in_amendment(&s.section_id)) {
        sections.entry(s.section_id.clone()).or_default().push(PositionedActor {
            label: s.actor_label.clone(),
            drrp: s.drrp.clone(),
            position: s.position.clone(),
            access_inferred: s.access_inferred,
        });
    }
    for ((_, label), cs) in derive_correlatives(&sections) {
        for c in cs {
            match c.kind {
                CorrelativeType::ClaimRight => law.claim_holders.insert(label.clone()),
                CorrelativeType::Liability => law.liability_holders.insert(label.clone()),
                CorrelativeType::Protected => law.protected_holders.insert(label.clone()),
                CorrelativeType::NoRight => false,
            };
        }
    }
    law
}

/// The current (as-amended) verdict label sent as `current_verdict` (#73 R1a):
/// `revoked` when the whole law is revoked (legal's LRT `live`) or every
/// substantive provision is repealed; otherwise the live-provision roll-up's
/// verdict, or `holder_unknown`.
pub fn current_verdict(revoked: bool, law: &LawDrrp) -> &'static str {
    if revoked {
        "revoked"
    } else {
        law.verdict().map_or("holder_unknown", |v| v.as_str())
    }
}

/// Legal's LRT `live` label for a wholly revoked law (sertantai-legal LiveStatus;
/// includes revoked_unapplied, excludes prospective revocations).
pub fn live_is_revoked(live: Option<&str>) -> bool {
    live.is_some_and(|l| l.contains("Revoked / Repealed / Abolished"))
}

impl LawDrrp {
    /// The `current_*` payload for a label (#73 R1a): `revoked` sends empty
    /// lists, `holder_unknown` the holder-unknown shape, otherwise the roll-up.
    pub fn current_payload(&self, label: &str) -> LawDrrp {
        match label {
            "revoked" => LawDrrp::default(),
            "holder_unknown" => self.holder_unknown_payload(),
            _ => self.clone(),
        }
    }

    /// The payload for a law with no verdict because a holder is unknown:
    /// `duty_type` keeps the raw `Obligation` beside any known Right/Power
    /// types; Duty/Responsibility lists are empty (never NULL); known Rights
    /// and Powers and their holders are kept. sertantai-legal reads NULL as
    /// "not in this payload" and would keep a stale verdict; this shape
    /// overwrites it and clears the enrichment verdict (#68, legal e36161b, 03f0154).
    pub fn holder_unknown_payload(&self) -> LawDrrp {
        let mut duty_types = self.duty_types.clone();
        duty_types.insert("Obligation".to_string());
        LawDrrp {
            duty_types,
            duties: Vec::new(),
            responsibilities: Vec::new(),
            duty_holders: BTreeSet::new(),
            responsibility_holders: BTreeSet::new(),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sig(sid: &str, label: &str, drrp: &str) -> ActorSignal {
        sig_at(sid, label, drrp, "active")
    }

    fn sig_at(sid: &str, label: &str, drrp: &str, position: &str) -> ActorSignal {
        ActorSignal { section_id: sid.into(), actor_label: label.into(), drrp: drrp.into(), position: position.into(), access_inferred: false }
    }

    #[test]
    fn inserted_duties_excluded_own_duties_kept() {
        // UK_ssi_2012_148-like: duty inside an 'insert—' subtree; UK_ssi_2010_435-like own duty
        let texts: HashMap<&str, &str> = [
            ("L:reg.2(3)", "In section 34 (duty of care etc. as respects waste)—"),
            ("L:reg.2(3)(a)", "in subsection (2), after paragraph (aa) insert—"),
            ("L:reg.2(3)(a)(ii)", "to prevent any contravention by any other person"),
            ("L:reg.5(1)", "A person has a duty to provide information to SEPA in writing"),
        ]
        .into_iter()
        .collect();
        let law = aggregate(
            &[sig("L:reg.2(3)(a)(ii)", "Ind: Person", "Obligation"), sig("L:reg.5(1)", "Ind: Person", "Obligation")],
            &[],
            |s| texts.get(s).map(|t| t.to_string()),
        );
        assert_eq!(law.excluded_amendment, 1);
        assert_eq!(law.duties.len(), 1);
        assert_eq!(law.duties[0].3, "reg.5(1)");
        assert_eq!(law.verdict(), Some(Verdict::Making));
    }

    #[test]
    fn government_obligations_are_responsibilities() {
        let law = aggregate(
            &[sig("L:reg.3(3)", "Gvt: Devolved Admin", "Obligation"), sig("L:reg.3(1)", "Gvt: Devolved Admin", "Liberty")],
            &[],
            |_| Some("The National Assembly must consult".into()),
        );
        assert_eq!(law.responsibilities.len(), 1);
        assert_eq!(law.powers.len(), 1);
        assert!(law.duty_types.contains("Responsibility") && law.duty_types.contains("Power"));
        assert_eq!(law.verdict(), Some(Verdict::Making));
    }

    #[test]
    fn verdicts() {
        let rights_only = aggregate(&[sig("L:reg.2", "Operator", "Liberty")], &[], |_| Some("may make available".into()));
        assert_eq!(rights_only.verdict(), Some(Verdict::Empowering));
        let none = aggregate(&[sig("L:reg.2", "Operator", "none")], &[], |_| Some("definitions".into()));
        assert_eq!(none.verdict(), Some(Verdict::NoObligations));
        assert_eq!(none, LawDrrp::default());
    }

    #[test]
    fn non_active_actors_hold_nothing() {
        // Beneficiary/counterparty carrying the provision's Obligation (#68):
        // not a Duty, and not Making on its own
        let law = aggregate(
            &[
                sig_at("L:s.1", "Ind: Driver", "Obligation", "beneficiary"),
                sig_at("L:s.2", "Org: Company", "Obligation", "counterparty"),
                sig("L:s.3", "Gvt: Minister", "Liberty"),
            ],
            &[],
            |_| Some("text".into()),
        );
        assert!(law.duties.is_empty() && law.duty_holders.is_empty());
        assert_eq!(law.excluded_non_active, 2);
        assert_eq!(law.power_holders.len(), 1);
        assert_eq!(law.verdict(), Some(Verdict::Empowering));
    }

    #[test]
    fn holder_class_from_dictionary() {
        let law = aggregate(
            &[sig("L:s.1", "Crown", "Obligation"), sig("L:s.2", "HM Forces", "Liberty"), sig("L:s.3", "Spc: Notifying Authority", "Obligation")],
            &[],
            |_| Some("text".into()),
        );
        assert!(law.duties.is_empty() && law.rights.is_empty());
        assert_eq!(law.responsibilities.len(), 2);
        assert_eq!(law.powers.len(), 1);
    }

    #[test]
    fn holder_unknown_blocks_empowering_and_no_obligations() {
        let rights = [sig("L:s.1", "Public", "Liberty")];
        let unknown = ["L:s.2".to_string()];
        let law = aggregate(&rights, &unknown, |_| Some("records shall be kept".into()));
        assert_eq!(law.verdict(), None);
        let payload = law.holder_unknown_payload();
        assert_eq!(payload.duty_types, BTreeSet::from(["Liberty".to_string(), "Obligation".to_string()]));
        // Known Rights and their holders are kept; there are no Duties to blank
        assert_eq!(payload.rights.len(), 1);
        assert!(payload.rights_holders.contains("Public"));
        assert!(payload.duties.is_empty() && payload.responsibilities.is_empty());
        assert_eq!(aggregate(&[], &unknown, |_| Some("records shall be kept".into())).verdict(), None);
        // A Duty is still Making whatever else is unknown
        let duty = [sig("L:s.1", "Org: Employer", "Obligation")];
        assert_eq!(aggregate(&duty, &unknown, |_| Some("text".into())).verdict(), Some(Verdict::Making));
        // Holder-unknown text inside an amendment doesn't count
        let texts: HashMap<&str, &str> =
            [("L:s.2(1)", "records shall be kept"), ("L:s.2", "after subsection (1) insert—")].into_iter().collect();
        let law = aggregate(&rights, &["L:s.2(1)".to_string()], |s| texts.get(s).map(|t| t.to_string()));
        assert_eq!(law.holder_unknown, 0);
        assert_eq!(law.verdict(), Some(Verdict::Empowering));
    }

    #[test]
    fn current_verdict_labels() {
        let duty = aggregate(&[sig("L:s.1", "Org: Employer", "Obligation")], &[], |_| Some("t".into()));
        assert_eq!(current_verdict(false, &duty), "making");
        assert_eq!(current_verdict(true, &duty), "revoked", "revocation wins over any live roll-up");
        let unknown = aggregate(&[], &["L:s.2".to_string()], |_| Some("records shall be kept".into()));
        assert_eq!(current_verdict(false, &unknown), "holder_unknown");
        assert_eq!(current_verdict(false, &LawDrrp::default()), "no_obligations");
        assert!(duty.current_payload("revoked").duty_holders.is_empty());
        assert!(unknown.current_payload("holder_unknown").duty_types.contains("Obligation"));
        assert!(live_is_revoked(Some("❌ Revoked / Repealed / Abolished")));
        assert!(!live_is_revoked(Some("⭕ Part Revocation / Repeal")));
        assert!(!live_is_revoked(None));
    }

    #[test]
    fn correlative_holders_rolled_up_not_in_verdict() {
        let law = aggregate(
            &[
                sig_at("L:s.2(1)", "Org: Employer", "Obligation", "active"),
                sig_at("L:s.2(1)", "Ind: Employee", "Obligation", "beneficiary"),
                sig_at("L:s.4", "Org: Employer", "Obligation", "active"),
                sig_at("L:s.4", "Gvt: Authority: Enforcement", "Obligation", "counterparty"),
                sig_at("L:s.9", "Gvt: Minister", "Liberty", "active"),
                sig_at("L:s.9", "Org: Operator", "Liberty", "counterparty"),
                // amendment text is excluded, as for DRRP
                sig_at("L:s.20", "Org: Employer", "Obligation", "active"),
                sig_at("L:s.20", "Ind: Person", "Obligation", "counterparty"),
            ],
            &[],
            |sid| Some(if sid == "L:s.20" { "In section 3, for \"x\" substitute \"y\"".into() } else { "text".into() }),
        );
        assert_eq!(law.protected_holders.iter().collect::<Vec<_>>(), vec!["Ind: Employee"]);
        assert_eq!(law.claim_holders.iter().collect::<Vec<_>>(), vec!["Gvt: Authority: Enforcement"]);
        assert_eq!(law.liability_holders.iter().collect::<Vec<_>>(), vec!["Org: Operator"]);
        assert_eq!(law.verdict(), Some(Verdict::Making));
        assert!(law.duty_holders.contains("Org: Employer") && !law.duty_holders.contains("Ind: Employee"));
    }
}
