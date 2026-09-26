//! Amendment-insertion text (fractalatai #57).
//!
//! When a law amends another instrument, the text it inserts or substitutes
//! ("after paragraph (aa) insert— …", "for subsection (2) there is substituted– …")
//! belongs to the amended instrument, not to the amending law. Such provisions
//! (the instruction and every sub-provision under it) get
//! [`super::ProvisionScope::Amendment`] and no tier extracts DRRP, actors,
//! significance or fitness from them.

use std::sync::LazyLock;

use regex::Regex;

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
        | ^\s*(?:\(\w+\)\s*)?in\s+(?:the\s+)?(?:section|subsection|regulation|paragraph|sub-paragraph|schedule|article|rule|part|chapter)\s+[^;\n]{0,160}[—–:-]\s*$
                                                          # locator stem: 'In section 34 (duty of care)—'
        ",
    )
    .unwrap()
});

/// "…are to be free of charge; and accordingly those Acts are amended as follows"
/// (UK_asp_2005_13 s.12(1)): an operative provision of this law that also announces
/// amendments. Its own clause stands; the amendments are in the following provisions.
static MIXED_OPERATIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\S[^;]{20,}[;,]\s*and\s+accordingly\b[^.;]{0,160}\bamended\b").unwrap()
});

/// Is this provision text an amendment instruction?
pub fn is_amendment_instruction(text: &str) -> bool {
    AMENDMENT_RE.is_match(text) && !MIXED_OPERATIVE_RE.is_match(text)
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

/// Is `section_id` amendment text: an amendment instruction itself, or under one?
/// `text_of` returns a provision's own text.
pub fn is_amendment_text(section_id: &str, text_of: impl Fn(&str) -> Option<String>) -> bool {
    text_of(section_id).is_some_and(|t| is_amendment_instruction(&t))
        || ancestors(section_id)
            .iter()
            .any(|a| text_of(a).is_some_and(|t| is_amendment_instruction(&t)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
        assert!(is_amendment_instruction("in paragraph (4) after “Ministers” insert “must review the national waste management plan at least once every six years”"));
        assert!(!is_amendment_instruction("A person has a duty to provide information to SEPA in writing"));
        assert!(!is_amendment_instruction("Every employer shall ensure that a suitable notice is displayed"));
        assert!(!is_amendment_instruction("The well operator must ensure that no well operation is commenced"));
        // Locator stems introducing amendments (UK_ssi_2012_148 reg.2(3), reg.2(3)(a))
        assert!(is_amendment_instruction("In section 34 (duty of care etc. as respects waste)—"));
        assert!(is_amendment_instruction("in subsection (1)—"));
        assert!(is_amendment_instruction("(2) In Schedule 3 (exempt activities)—"));
        // ... but not definition stems or ordinary provisions
        assert!(!is_amendment_instruction("In this regulation—"));
        assert!(!is_amendment_instruction("In these Regulations— “the 1990 Act” means the Environmental Protection Act 1990"));
        assert!(!is_amendment_instruction("In section 34 the duty applies to any person who imports waste."));
        // Mixed operative provision (UK_asp_2005_13 s.12(1), a gold benchmark)
        assert!(!is_amendment_instruction(
            "Oral health assessments and dental examinations carried out on or after 1st April 2006 in accordance with \
             arrangements made under section 17C of the 1978 Act are to be free of charge; and accordingly those Acts are amended as follows."
        ));
        assert!(is_amendment_instruction("The 1978 Act is amended as follows."));
    }

    #[test]
    fn sub_provisions_under_an_insert_stem() {
        // UK_ssi_2012_148 reg.2(3): the inserted duty text has no instruction words itself
        let texts: HashMap<&str, &str> = [
            ("L:reg.2(3)", "In section 34 (duty of care etc. as respects waste)—"),
            ("L:reg.2(3)(a)", "in subsection (2), after paragraph (aa) insert—"),
            ("L:reg.2(3)(a)(ii)", "to prevent any contravention by any other person of subsection (2A)"),
            ("L:reg.5(1)", "A person has a duty to provide information to SEPA in writing"),
        ]
        .into_iter()
        .collect();
        let text_of = |s: &str| texts.get(s).map(|t| t.to_string());
        assert!(is_amendment_text("L:reg.2(3)(a)(ii)", text_of));
        assert!(is_amendment_text("L:reg.2(3)(a)", text_of));
        assert!(!is_amendment_text("L:reg.5(1)", text_of));
    }
}
