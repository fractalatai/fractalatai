//! Law-level DRRP rolled up from reconciled provision-level actor signals
//! (`provision_actors.drrp`), run by `taxa backfill` (fractalatai #55).
//!
//! Replaces the regex-only roll-up in `taxa parse` (`write_law_taxa`) for the
//! law-level columns sertantai-legal's `MakingResolver` reads:
//! Duty/Responsibility/Obligation → making; Rights/Powers only → empowering;
//! empty lists → no_obligations; NULL → no verdict.
//!
//! Policy (Jason, 2026-09-25): an amending instrument that inserts duties into a
//! principal instrument is not Making. Signals in amendment text (a provision
//! that is an amendment instruction, or sits under one) are excluded.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;

/// One reconciled actor signal on one provision.
#[derive(Debug, Clone)]
pub struct ActorSignal {
    /// Full section id, e.g. `UK_uksi_2008_198:reg.2(3)(a)`
    pub section_id: String,
    /// Actor label, e.g. `Ind: Person`, `Gvt: Minister`
    pub actor_label: String,
    /// Reconciled DRRP: `Obligation`, `Liberty`, `none`
    pub drrp: String,
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
    /// The verdict sertantai-legal's resolver records for this payload.
    pub fn verdict(&self) -> Verdict {
        if !self.duties.is_empty() || !self.responsibilities.is_empty() {
            Verdict::Making
        } else if !self.rights.is_empty() || !self.powers.is_empty() {
            Verdict::Empowering
        } else {
            Verdict::NoObligations
        }
    }
}

static AMENDMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?ix)
        \b(?:insert|substitute|add)(?:ed)?\s*[—–:-]\s*   # 'insert—' '... there is substituted–' introducing quoted text
        | \bfor\b[^.;]{0,160}?\bsubstitute\b             # 'for X substitute Y'
        | \b(?:after|before|at\s+the\s+end\s+of)\b[^.;]{0,160}?\binsert\b
        | \bomit\b
        | \b(?:is|are|shall\s+be)\s+(?:further\s+)?amended\b
        | \bthere\s+(?:is|are|shall\s+be)\s+(?:inserted|substituted|added)\b
        | \bhas\s+effect\s+as\s+if\b[^.;]{0,160}?\b(?:substituted|inserted)\b
        ",
    )
    .unwrap()
});

/// Is this provision text an amendment instruction?
pub fn is_amendment_instruction(text: &str) -> bool {
    AMENDMENT_RE.is_match(text)
}

/// Ancestors of a section id by stripping trailing `(...)` groups:
/// `X:reg.2(3)(a)(ii)` → [`X:reg.2(3)(a)`, `X:reg.2(3)`, `X:reg.2`].
pub fn ancestors(section_id: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = section_id;
    while cur.ends_with(')') {
        match cur.rfind('(') {
            Some(i) if i > 0 => {
                cur = &cur[..i];
                out.push(cur.to_string());
            }
            _ => break,
        }
    }
    out
}

/// Government-side actor (regulator/Crown/EU institution): its Obligation is a
/// Responsibility and its Liberty a Power.
pub fn is_government_actor(label: &str) -> bool {
    label.starts_with("Gvt") || label.starts_with("EU:")
}

fn clause_preview(text: &str) -> String {
    let clean: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() > 200 {
        format!("{}...", clean.chars().take(200).collect::<String>())
    } else {
        clean
    }
}

/// Aggregate one law. `text_of(section_id)` returns the provision's own text.
pub fn aggregate(signals: &[ActorSignal], text_of: impl Fn(&str) -> Option<String>) -> LawDrrp {
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
        let gov = is_government_actor(&s.actor_label);
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
    law
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sig(sid: &str, label: &str, drrp: &str) -> ActorSignal {
        ActorSignal { section_id: sid.into(), actor_label: label.into(), drrp: drrp.into() }
    }

    #[test]
    fn ancestors_strip_brackets() {
        assert_eq!(ancestors("L:reg.2(3)(a)(ii)"), vec!["L:reg.2(3)(a)", "L:reg.2(3)", "L:reg.2"]);
        assert!(ancestors("L:sch.2.para.226").is_empty());
    }

    #[test]
    fn amendment_instructions() {
        assert!(is_amendment_instruction("After section 97B of the 1968 Act insert—"));
        assert!(is_amendment_instruction("in subsection (5), for “the duty imposed by subsection (1)” substitute “a duty”"));
        assert!(is_amendment_instruction("for sub paragraphs (1) and (2) there is substituted– Subject to sub paragraph (3) below, it shall be an offence"));
        assert!(is_amendment_instruction("The Environmental Protection Act 1990 is amended as follows."));
        assert!(is_amendment_instruction("In regulation 65 (offences), omit paragraph (b)."));
        assert!(!is_amendment_instruction("A person has a duty to provide information to SEPA in writing"));
        assert!(!is_amendment_instruction("Every employer shall ensure that a suitable notice is displayed"));
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
            |s| texts.get(s).map(|t| t.to_string()),
        );
        assert_eq!(law.excluded_amendment, 1);
        assert_eq!(law.duties.len(), 1);
        assert_eq!(law.duties[0].3, "reg.5(1)");
        assert_eq!(law.verdict(), Verdict::Making);
    }

    #[test]
    fn government_obligations_are_responsibilities() {
        let law = aggregate(
            &[sig("L:reg.3(3)", "Gvt: Devolved Admin", "Obligation"), sig("L:reg.3(1)", "Gvt: Devolved Admin", "Liberty")],
            |_| Some("The National Assembly must consult".into()),
        );
        assert_eq!(law.responsibilities.len(), 1);
        assert_eq!(law.powers.len(), 1);
        assert!(law.duty_types.contains("Responsibility") && law.duty_types.contains("Power"));
        assert_eq!(law.verdict(), Verdict::Making);
    }

    #[test]
    fn verdicts() {
        let rights_only = aggregate(&[sig("L:reg.2", "Operator", "Liberty")], |_| Some("may make available".into()));
        assert_eq!(rights_only.verdict(), Verdict::Empowering);
        let none = aggregate(&[sig("L:reg.2", "Operator", "none")], |_| Some("definitions".into()));
        assert_eq!(none.verdict(), Verdict::NoObligations);
        assert_eq!(none, LawDrrp::default());
    }
}
