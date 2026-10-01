//! Purpose classifier for UK ESH legal text.
//!
//! Purpose identifies WHAT the law does (function-based), as opposed to
//! `duty_type` which identifies WHO has obligations (role-based).
//!
//! Two layers (docs/architecture/PURPOSE-CLASSIFICATION.md, agreed 2026-10-01):
//! - [`classify`]: the original multi-match patterns (ported from
//!   `Taxa.PurposeClassifier`). **Internal signals only**: DRRP gating
//!   (`should_skip_drrp`), scope and miss-heat were tuned on them, so they're
//!   kept unchanged until moving gating is measured against gold v2.
//! - [`primary`]: the **published** purpose, one per provision, in the agreed
//!   vocabulary. It's what `legislation_text.purposes` carries (an array of one).

use std::sync::LazyLock;

use regex::Regex;

// ── Purpose labels ───────────────────────────────────────────────────

pub const ENACTMENT: &str = "Enactment+Citation+Commencement";
pub const INTERPRETATION: &str = "Interpretation+Definition";
pub const APPLICATION_SCOPE: &str = "Application+Scope";
pub const EXTENT: &str = "Extent";
pub const EXEMPTION: &str = "Exemption";
pub const PROCESS_RULE: &str = "Process+Rule+Constraint+Condition";
pub const POWER_CONFERRED: &str = "Power Conferred";
pub const CHARGE_FEE: &str = "Charge+Fee";
pub const OFFENCE: &str = "Offence";
pub const ENFORCEMENT: &str = "Enforcement+Prosecution";
pub const DEFENCE_APPEAL: &str = "Defence+Appeal";
pub const LIABILITY: &str = "Liability";
pub const REPEAL_REVOCATION: &str = "Repeal+Revocation";
pub const AMENDMENT: &str = "Amendment";
pub const TRANSITIONAL: &str = "Transitional Arrangement";
pub const UNCLASSIFIED: &str = "Unclassified";

/// All purpose labels in priority order.
pub const ALL_PURPOSES: &[&str] = &[
    ENACTMENT,
    INTERPRETATION,
    APPLICATION_SCOPE,
    EXTENT,
    EXEMPTION,
    PROCESS_RULE,
    POWER_CONFERRED,
    CHARGE_FEE,
    OFFENCE,
    ENFORCEMENT,
    DEFENCE_APPEAL,
    LIABILITY,
    REPEAL_REVOCATION,
    AMENDMENT,
    TRANSITIONAL,
    UNCLASSIFIED,
];

/// Structural purposes where actors are typically mentioned, not duty-bearing.
pub const STRUCTURAL_PURPOSES: &[&str] = &[
    ENACTMENT,
    INTERPRETATION,
    AMENDMENT,
    REPEAL_REVOCATION,
    EXTENT,
    TRANSITIONAL,
    UNCLASSIFIED,
];

// ── Pattern definitions ──────────────────────────────────────────────

const RAW_PATTERNS: &[(&str, &str)] = &[
    (
        ENACTMENT,
        // "Commencement Information" is editorial metadata from legislation.gov.uk, not
        // actual commencement content. Use negative lookahead to exclude it, and require
        // "commencement" to appear near citation/force context or as a heading.
        r"(?i)(?:(?:Act|Regulations?|Order) may be cited as|(?:Act|Regulations?|Order).*?shall have effect|(?:Act|Regulations?|Order) shall come into (?:force|operation)|comes? into force|has effect.*?on or after|[Cc]itation and commencement|commencement (?:date|order|provision|of this)|[Cc]ommencement\s*$|^(?:\s*)Statutory Instruments|hereby makes the following|Signed by (?:authority|order) of the Secretary)",
    ),
    (
        INTERPRETATION,
        r#"(?i)(?:[A-Za-z\d ][""\u{201c}].*?(?:means|includes|does not include|is (?:information|the)|are|to be read as|are references to|consists)|[""\u{201c}].*?[""\u{201d}] is|In thi?e?se? [Rr]egulations?.*?[\u{2014}\u{2014}\u{2014}]|has?v?e? the (?:same )?(?:respective )?meanings?|[Ff]or the purposes? of (?:this Act|determining|these Regulations)|(?:any reference|references?).*?to|[Ii]nterpretation|for the meaning of)"#,
    ),
    (
        APPLICATION_SCOPE,
        // Tightened for GH #20: only match genuine application/scope provisions,
        // not duty text that incidentally mentions "shall apply".
        // Key constraint: the LAW must be the subject that applies/has effect,
        // not an actor applying something.
        //
        // Branches:
        // 1. "Application" as a heading (start of text)
        // 2. Self-referencing: "These/This Regulations shall [not] apply"
        //    Requires these/this at text-start, after a paragraph number,
        //    or after a sentence boundary / comma. This prevents matching
        //    relative clauses like "to whom this regulation applies" or
        //    "requirement of these Regulations which applies" where
        //    the law reference is inside a subordinate clause.
        // 3. Numbered provision non-application: "regulation 6(4) shall not apply"
        // 4. Scope extension: "shall apply to X as they apply to Y"
        // 5. Like duty: "be under a like duty"
        // 6. Requirement extends: "Any requirement...shall also extend to"
        // 7. Does not apply with preposition
        // 8. Effect statements
        // 9. Provisions-referencing apply
        // 10. Crown application
        // Tightened for GH #20 + fitness session:
        // - Self-ref branch: require "apply to/in/where/until" after the verb,
        //   rejecting "applies for the purpose of interpreting/determining"
        // - "provisions of" branch: require "apply to/in" (not bare "apply")
        // 11. EU: "Paragraphs 1 to 5 shall not apply", "Articles 21, 22 shall not apply"
        r"(?i)(?:^Application\b|(?:^|[.;,]\s+|\d\s+)(?:these|this) (?:Regulations?|Act|Order|Part|Rules?|section|provisions?|Directive).{0,60}(?:shall |do(?:es)? )?(?:not )?appl(?:y|ies) (?:to|in |where|until|unless)|(?:regulations?|sections?|paragraphs?|articles?) [\d(].{0,60}shall not apply|shall apply to .{0,60} as they apply to|be under a like duty|(?:any )?(?:requirement|prohibition|duty).{0,150}(?:shall (?:also )?extend|shall extend only)|shall extend only to|does not apply (?:to|where|until|in|unless)|shall have (?:no )?effect|ceases? to have effect|provisions of .{0,40}(?:shall )?apply (?:to|in)|shall bind the Crown)",
    ),
    (
        EXTENT,
        r"(?i)(?:(?:Act|Regulation|section)(?: does not | do not | )extends? to|(?:Act|Regulations?|Section).*?extends? (?:only )?to|[Oo]nly.*?extend to|do not extend to|shall not (?:extend|apply) to (?:Scotland|Wales|Northern Ireland))",
    ),
    (
        EXEMPTION,
        r"(?i)(?:shall not apply in any case where|by a certificate in writing exempt|\bexemption\b)",
    ),
    (
        PROCESS_RULE,
        r"(?i)(?:\bshall\b|\bmust\b|\brequired\b|\brequirements?\b|\bobligations?\b|\bprocedures?\b|\brules?\b|\bconditions?\b|\bduty\b|\bduties\b|\bcomply\b|\bprohibited\b|\bpermitted\b|\bmay not\b|\bstandards?\b|\bensure\b|\bmaintain\b|\bresponsible\b)",
    ),
    (
        POWER_CONFERRED,
        r"(?i)(?:functions.*(?:exercis(?:ed|able)|conferred)|exercising.*functions|power to make regulations|[Tt]he power under (?:subsection))",
    ),
    (
        CHARGE_FEE,
        r"(?i)(?:fees and charges|(?:fees?|charges?).*?(?:paid|payable)|by the (?:fee|charge)|failed to pay a (?:fee|charge)|fee.*?may not exceed|may charge.*?a fee|[Aa] fee charged)",
    ),
    (
        OFFENCE,
        r"(?i)(?:[Oo]ffences?[\s.,\u{2014}:]|(?:[Ff]ixed|liable to a) penalty)",
    ),
    (
        ENFORCEMENT,
        // "proceedings" alone is too broad (matches tribunal, disclosure contexts).
        // Require enforcement-specific context.
        r"(?i)(?:(?:criminal|summary|enforcement|prosecut\w+) proceedings|proceedings for (?:an )?offence|on (?:summary )?conviction|[Ee]nforcement)",
    ),
    (
        DEFENCE_APPEAL,
        r"(?i)(?:\b[Aa]ppeal\b|[Ii]t is a defence for a|may not rely on a defence|shall not be (?:guilty|liable)|[Ii]t shall (?:also )?.*?be a defence|rebuttable)",
    ),
    (LIABILITY, r"(?i)(?:\bliability\b|\bliable\b)"),
    (
        REPEAL_REVOCATION,
        r"(?i)(?:\.\s+\.\s+\.\s+\.\s+\.\s+\.\s+\.|(?:revoked|repealed)|(?:[Rr]epeals|revocations)|following Acts shall cease to have effect)",
    ),
    (
        AMENDMENT,
        r"(?i)(?:shall be inserted|there is inserted|insert the following after|shall be (?:inserted|substituted) the words|for.*?substitute|omit the (?:words?|entr(?:y|ies))|shall be amended|[Aa]mendments?|[Aa]mended as follows)",
    ),
    (
        TRANSITIONAL,
        r"(?i)(?:transitional provision|transitional arrangements?)",
    ),
];

static COMPILED: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    RAW_PATTERNS
        .iter()
        .map(|(purpose, pat)| (*purpose, Regex::new(pat).unwrap()))
        .collect()
});

// ── Public API ───────────────────────────────────────────────────────

/// Classify legal text and return all matching purposes.
///
/// If no patterns match, defaults to "Process+Rule+Constraint+Condition".
pub fn classify(text: &str) -> Vec<&'static str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut result: Vec<&str> = COMPILED
        .iter()
        .filter(|(_, re)| re.is_match(text))
        .map(|(purpose, _)| *purpose)
        .collect();

    if result.is_empty() {
        result.push(UNCLASSIFIED);
    }
    sort_purposes(&mut result);
    result.dedup();
    result
}

/// Classify a law title to determine purpose (quick heuristic).
pub fn classify_title(title: &str) -> Vec<&'static str> {
    if title.is_empty() {
        return Vec::new();
    }
    if title.contains("(Amendment") {
        return vec![AMENDMENT];
    }
    if title.contains("(Revocation)") || title.contains("(Repeal)") {
        return vec![REPEAL_REVOCATION];
    }
    if title.contains("(Commencement") {
        return vec![ENACTMENT];
    }
    if title.contains("(Application)") {
        return vec![APPLICATION_SCOPE];
    }
    if title.contains("(Transitional") {
        return vec![TRANSITIONAL];
    }
    if title.contains("(Extent)") || title.contains("(Extension") {
        return vec![EXTENT];
    }
    Vec::new()
}

/// Sort purposes by priority order.
pub fn sort_purposes(purposes: &mut Vec<&str>) {
    purposes.sort_by_key(|p| ALL_PURPOSES.iter().position(|&k| k == *p).unwrap_or(99));
}

// ── Published purpose (agreed 2026-10-01) ────────────────────────────

pub const REQUIREMENT: &str = "Requirement";
pub const PROCEDURE_DETAIL: &str = "Procedure+Detail";
pub const ESTABLISHMENT: &str = "Establishment+Constitution";

/// The published vocabulary in table order: machinery, operative, sanctions.
/// `Process+Rule+Constraint+Condition` is retired from it.
pub const PUBLISHED_PURPOSES: &[&str] = &[
    ENACTMENT,
    INTERPRETATION,
    APPLICATION_SCOPE,
    EXEMPTION,
    EXTENT,
    ESTABLISHMENT,
    AMENDMENT,
    REPEAL_REVOCATION,
    TRANSITIONAL,
    REQUIREMENT,
    POWER_CONFERRED,
    PROCEDURE_DETAIL,
    CHARGE_FEE,
    ENFORCEMENT,
    OFFENCE,
    DEFENCE_APPEAL,
    LIABILITY,
    UNCLASSIFIED,
];

/// Published purposes that are law-about-law (structural in the spec).
pub const MACHINERY_PURPOSES: &[&str] = &[
    ENACTMENT,
    INTERPRETATION,
    APPLICATION_SCOPE,
    EXEMPTION,
    EXTENT,
    ESTABLISHMENT,
    AMENDMENT,
    REPEAL_REVOCATION,
    TRANSITIONAL,
];

/// Checked in this order: the first match wins (precedence: machinery >
/// sanctions > Charge+Fee > Procedure+Detail test > Requirement / Power).
/// Exemption is checked before Application+Scope so a negative application
/// ("shall not apply to …") is an exemption, not scope.
static PRIMARY_PATTERNS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    let raw: &[(&str, &str)] = &[
        (ENACTMENT, RAW_PATTERNS[0].1),
        (
            INTERPRETATION,
            // Definitions, deeming, evidential effect, status by operation of law.
            // Not bare "for the purposes of" / "reference to": duties start that way too.
            r#"(?i)(?:^\s*Interpretation\b|["“'‘][^"”'’]{1,80}["”'’]\s*(?:\([^)]{0,40}\)\s*)?(?:means|includes|does not include|has the (?:same )?meaning|is to be (?:read|construed))|\bha(?:s|ve) the (?:same |respective )?meanings? (?:given|as|assigned)|\bIn th(?:is|ese) (?:Act|Regulations?|Order|Part|Chapter|section|regulation|article|Schedule)\s*(?:,|—|–|-)\s*$|\breferences? (?:in [^.;]{0,80})?to [^.;]{0,80}\b(?:are|is) (?:to be )?(?:read|construed|references?)\b|\b(?:shall|is to|are to|must) be (?:treated|deemed|regarded|taken) (?:as|to)\b|\bis (?:treated|deemed|regarded) (?:as|to)\b|\bconclusive evidence\b|\bshall be (?:admissible )?(?:in )?evidence\b|\bremains? in force\b|\bceases? to have effect\b)"#,
        ),
        (
            EXEMPTION,
            r"(?i)(?:shall not apply in any case where|by a certificate in writing exempt|\bexempt(?:ion|ed)?\b|\b(?:shall|does|do) not apply (?:to|in|where|unless)\b|\bnothing in [^.;]{0,80}\b(?:requires?|shall require|is to be taken to require|makes? [^.;]{0,40}\bliable)\b)",
        ),
        (
            APPLICATION_SCOPE,
            r"(?i)(?:^\s*Application\b|(?:^|[.;,]\s+|\d\s+)(?:these|this) (?:Regulations?|Act|Order|Part|Rules?|section|provisions?|Directive).{0,60}(?:shall |do(?:es)? )?appl(?:y|ies) (?:to|in |where|until|unless)|shall apply to .{0,60} as they apply to|be under a like duty|(?:any )?(?:requirement|prohibition|duty).{0,150}(?:shall (?:also )?extend|shall extend only)|shall extend only to|provisions of .{0,40}(?:shall )?apply (?:to|in)|\bbinds? the Crown\b|\bnothing in [^.;]{0,80}\b(?:shall )?(?:prejudice|affects?|derogate)\b)",
        ),
        (EXTENT, RAW_PATTERNS[3].1),
        (
            ESTABLISHMENT,
            r"(?i)(?:\bThere (?:shall be|is (?:hereby )?established|are established|continues? to be) (?:a|an|the) [^.;]{0,60}\b(?:body|Executive|Agency|Authority|Commission|Council|Board|Committee|Office)|\bshall (?:be|continue to be) a body corporate\b|\bis (?:hereby )?established\b|\b(?:principal|general) (?:objective|aim)s? of\b|\bThe (?:principal )?objectives? of the\b|\bconstitution of the\b)",
        ),
        (AMENDMENT, RAW_PATTERNS[13].1),
        (REPEAL_REVOCATION, RAW_PATTERNS[12].1),
        (TRANSITIONAL, r"(?i)(?:transitional (?:provision|arrangement)s?|\bsavings? (?:and transitional|provisions?)\b|transitional and saving)"),
        (
            ENFORCEMENT,
            r"(?i)(?:(?:criminal|enforcement|prosecut\w+) proceedings|proceedings for (?:an )?offence|\benforcing authority\b|\b(?:improvement|prohibition|enforcement|stop|compliance) notice\b|\binspector (?:may|shall)\b|\bpowers? of entry\b|\bmay (?:enter|inspect|seize|take samples)\b)",
        ),
        (
            OFFENCE,
            r"(?i)(?:\bcommits? an offence\b|\bguilty of an offence\b|\bit is an offence\b|\bliable,? on (?:summary )?conviction\b|\bon conviction on indictment\b|\b(?:fixed|civil|monetary|variable) (?:monetary )?penalt(?:y|ies)\b|\bpenalty notice\b|\bliable to (?:a fine|imprisonment)\b)",
        ),
        (
            DEFENCE_APPEAL,
            r"(?i)(?:\b[Aa]ppeal\b|[Ii]t (?:is|shall (?:also )?be) a defence|may not rely on a defence|shall not be guilty|\breview of (?:the|a|any) decision\b)",
        ),
        (
            LIABILITY,
            r"(?i)(?:\bcivil liability\b|\bshall not be liable\b|\bliable (?:in damages|to pay|for (?:any |the )?(?:loss|damage|injury|costs?))|\bcompensation\b|\bbreach of (?:a )?statutory duty\b|\bactionable\b)",
        ),
        (
            CHARGE_FEE,
            // As the signal pattern, without "by the fee/charge" (an incidental
            // mention: "accompanied by the fee" is a detail of an application)
            r"(?i)(?:fees and charges|(?:fees?|charges?).*?(?:paid|payable)|failed to pay a (?:fee|charge)|fee.*?may not exceed|may charge.*?a fee|[Aa] fee charged)",
        ),
        (
            PROCEDURE_DETAIL,
            // Qualifies a relation created elsewhere (refers to it), notice
            // service, parliamentary procedure (detail ruling, 2026-10-01)
            r"(?i)(?:\b(?:the|any|such|that|an?|each) (?:application|notice|record|report|assessment|plan|statement|information|register|certificate|request|return|notification|copy|copies|document|scheme|licence|permit|approval|consent|direction|order|regulations)s? (?:referred to in|mentioned in|required (?:by|under)|made under|given under|served under|kept under|prepared under|issued under|specified in|under) (?:this |that )?(?:section|regulation|paragraph|sub-?section|article|sub-?paragraph|Part|Schedule|provision)?\s*[\d(]|\b(?:laid before|approved by a resolution of|resolution of either House|subject to annulment|statutory instrument containing)\b|\bmay be (?:served|given|sent) (?:on|to) [^.;]{0,60}\bby (?:delivering|leaving|sending|post)\b|\bmust be (?:made|given|served|sent) (?:in writing|in the (?:prescribed|approved) form)\b)",
        ),
    ];
    raw.iter().map(|(p, r)| (*p, Regex::new(r).unwrap())).collect()
});

static OBLIGATION_MODAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:shall|must|is required to|are required to|it (?:shall|is) be the duty|no person shall)\b").unwrap()
});
static LIBERTY_MODAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:may|is entitled to|are entitled to|power to)\b").unwrap());
/// A government deadline to act stays a Requirement (detail ruling exception)
static GOV_DEADLINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bmust come into force (?:no later than|before|by)\b").unwrap());

/// The published purpose of a provision: one value in the agreed vocabulary.
/// `duty_types` is the regex DRRP result for the same text (Requirement /
/// Power Conferred follow the provision's main relation). Returns
/// `Unclassified` when nothing fits; [`inherit_from_stem`] then gives list
/// items and fragments their stem's purpose.
pub fn primary(text: &str, duty_types: &[super::duty_type::DutyType]) -> &'static str {
    use super::duty_type::DutyType;
    if !text.chars().any(|c| c.is_alphabetic()) {
        return UNCLASSIFIED;
    }
    if GOV_DEADLINE_RE.is_match(text) {
        return REQUIREMENT;
    }
    if let Some((p, _)) = PRIMARY_PATTERNS.iter().find(|(_, re)| re.is_match(text)) {
        return p;
    }
    let ob = duty_types.contains(&DutyType::Obligation);
    let li = duty_types.contains(&DutyType::Liberty);
    match (ob, li) {
        (true, false) => REQUIREMENT,
        (false, true) => POWER_CONFERRED,
        // Both: the main relation, taken as the first modal in the text
        (true, true) => match (OBLIGATION_MODAL_RE.find(text), LIBERTY_MODAL_RE.find(text)) {
            (Some(o), Some(l)) if l.start() < o.start() => POWER_CONFERRED,
            _ => REQUIREMENT,
        },
        (false, false) => {
            if OBLIGATION_MODAL_RE.is_match(text) {
                REQUIREMENT
            } else if LIBERTY_MODAL_RE.is_match(text) {
                POWER_CONFERRED
            } else {
                UNCLASSIFIED
            }
        }
    }
}

/// The stem rule: a provision with no purpose of its own (a list item or
/// fragment) takes its nearest classified ancestor's purpose. `ancestor_purposes`
/// is nearest first. A provision that does something itself keeps its own.
pub fn inherit_from_stem<'a>(own: &'a str, ancestor_purposes: impl IntoIterator<Item = &'a str>) -> &'a str {
    if own != UNCLASSIFIED {
        return own;
    }
    ancestor_purposes.into_iter().find(|p| *p != UNCLASSIFIED).unwrap_or(own)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_enactment() {
        let result =
            classify("This Act may be cited as the Health and Safety at Work etc. Act 1974.");
        assert!(result.contains(&ENACTMENT));
    }

    #[test]
    fn classify_interpretation() {
        let result = classify(r#"In these Regulations— "employer" means a person who employs"#);
        assert!(result.contains(&INTERPRETATION));
    }

    #[test]
    fn classify_application_scope() {
        let result =
            classify("These Regulations apply to every employer and self-employed person.");
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "self-referencing applicability should match, got: {:?}",
            result
        );
    }

    #[test]
    fn classify_application_scope_shall_apply_to_self_employed() {
        let text = "These Regulations shall apply to a self-employed person as they \
                    apply to an employer and an employee.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "scope extension should match, got: {:?}",
            result
        );
    }

    #[test]
    fn classify_application_scope_like_duty() {
        let text = "Where a duty is placed by these Regulations on an employer, \
                    he shall be under a like duty in respect of any other person.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "'be under a like duty' should match, got: {:?}",
            result
        );
    }

    #[test]
    fn classify_application_scope_regulation_shall_not_apply() {
        let text = "regulation 6(4) shall not apply until 6th July 2010 where \
                    work equipment is used.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "transitional non-application should match, got: {:?}",
            result
        );
    }

    #[test]
    fn classify_application_scope_requirement_extends() {
        let text = "Any requirement imposed by these Regulations on an employer \
                    shall also extend to a self-employed person.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "requirement-extends should match, got: {:?}",
            result
        );
    }

    #[test]
    fn classify_application_scope_does_not_apply_to() {
        let text = "This regulation does not apply to work equipment which is \
                    provided for use in normal ship-board activities.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "'does not apply to' should match, got: {:?}",
            result
        );
    }

    #[test]
    fn classify_no_false_application_scope_genuine_duty() {
        // Genuine duty — "shall ensure" is an obligation, not applicability
        let text = "The employer shall ensure the health and safety of employees.";
        let result = classify(text);
        assert!(
            !result.contains(&APPLICATION_SCOPE),
            "genuine duty should NOT match Application+Scope, got: {:?}",
            result
        );
    }

    #[test]
    fn classify_no_false_application_scope_shall_apply_precautionary() {
        // "shall apply" where an actor applies a principle — not scope
        let text = "The employer shall apply the general principles of prevention.";
        let result = classify(text);
        assert!(
            !result.contains(&APPLICATION_SCOPE),
            "actor applying a principle should NOT match Application+Scope, got: {:?}",
            result
        );
    }

    #[test]
    fn classify_amendment() {
        let result = classify("In section 3, for subsection (2) substitute the following.");
        assert!(result.contains(&AMENDMENT));
    }

    #[test]
    fn classify_offence() {
        let result = classify("It is an offence for any person to contravene this regulation.");
        assert!(result.contains(&OFFENCE));
    }

    #[test]
    fn classify_default_process_rule() {
        let result = classify("some general text about workplace safety procedures.");
        assert!(result.contains(&PROCESS_RULE));
    }

    #[test]
    fn classify_empty() {
        assert!(classify("").is_empty());
    }

    #[test]
    fn classify_title_amendment() {
        assert_eq!(
            classify_title("The Health and Safety (Amendment) Regulations 2024"),
            vec![AMENDMENT]
        );
    }

    #[test]
    fn classify_title_commencement() {
        assert_eq!(
            classify_title("The Environmental Protection Act 1990 (Commencement No. 1) Order"),
            vec![ENACTMENT]
        );
    }

    #[test]
    fn classify_title_no_match() {
        assert!(
            classify_title("The Workplace (Health, Safety and Welfare) Regulations 1992")
                .is_empty()
        );
    }

    #[test]
    fn multiple_purposes() {
        let text = "This Act may be cited as the Act 1974. The employer shall ensure safety.";
        let result = classify(text);
        assert!(result.len() >= 2);
        assert!(result.contains(&ENACTMENT));
        assert!(result.contains(&PROCESS_RULE));
    }

    #[test]
    fn sort_order() {
        let mut purposes = vec![AMENDMENT, ENACTMENT, PROCESS_RULE];
        sort_purposes(&mut purposes);
        assert_eq!(purposes, vec![ENACTMENT, PROCESS_RULE, AMENDMENT]);
    }

    // ── Real-world pattern tests (from MHSWR 1999) ──────────────────

    #[test]
    fn classify_interpretation_real_world() {
        // From UK_uksi_1999_3242:reg.1(2)
        let text = r#"2 In these Regulations—
"the 1996 Act" means the Employment Rights Act 1996 F4 ;
"the assessment" means, in the case of an employer or self-employed person"#;
        let result = classify(text);
        assert!(
            result.contains(&INTERPRETATION),
            "Should detect 'In these Regulations—' as Interpretation"
        );
    }

    #[test]
    fn classify_interpretation_any_reference() {
        // From UK_uksi_1999_3242:reg.1(3)
        let text = "3 Any reference in these Regulations to—\n(a) a numbered regulation";
        let result = classify(text);
        assert!(
            result.contains(&INTERPRETATION),
            "Should detect 'Any reference in these Regulations' as Interpretation"
        );
    }

    #[test]
    fn classify_enactment_cited_as() {
        // From UK_uksi_1999_3242:reg.1(1)
        let text = "1.—(1) These Regulations may be cited as the Management of Health and Safety at Work Regulations 1999";
        let result = classify(text);
        assert!(
            result.contains(&ENACTMENT),
            "Should detect 'may be cited as' as Enactment"
        );
    }

    #[test]
    fn classify_enactment_come_into_force() {
        let text = "shall come into force on 29th December 1999";
        let result = classify(text);
        assert!(
            result.contains(&ENACTMENT),
            "Should detect 'come into force' as Enactment"
        );
    }

    // ── False-positive regression tests ──────────────────────────────

    #[test]
    fn commencement_information_not_enactment() {
        // "Commencement Information" is editorial metadata from legislation.gov.uk
        let text = "4. Every employer shall ensure that lifting equipment is of adequate strength and stability for each load. Commencement Information I1 Reg. 4 in force at 5.12.1998";
        let result = classify(text);
        assert!(
            !result.contains(&ENACTMENT),
            "Editorial 'Commencement Information' should NOT trigger Enactment; got {:?}",
            result
        );
    }

    #[test]
    fn citation_and_commencement_heading_is_enactment() {
        let text = "Citation and commencement";
        let result = classify(text);
        assert!(
            result.contains(&ENACTMENT),
            "'Citation and commencement' heading should be Enactment"
        );
    }

    #[test]
    fn proceedings_alone_not_enforcement() {
        // "proceedings" in general tribunal context is not enforcement
        let text = "One or more assessors may be appointed for the purposes of any proceedings brought before an employment tribunal.";
        let result = classify(text);
        assert!(
            !result.contains(&ENFORCEMENT),
            "Tribunal proceedings should NOT trigger Enforcement; got {:?}",
            result
        );
    }

    #[test]
    fn summary_conviction_is_enforcement() {
        let text = "A person guilty of an offence is liable on summary conviction to a fine.";
        let result = classify(text);
        assert!(
            result.contains(&ENFORCEMENT),
            "'on summary conviction' should trigger Enforcement"
        );
    }

    // ── Application+Scope relative-clause regression tests ──────────

    #[test]
    fn no_false_scope_employee_to_whom_this_reg_applies() {
        // Health surveillance duty with relative clause qualifier
        let text = "An employee to whom this regulation applies shall, when required \
                    by his employer and at the cost of the employer, present himself \
                    during his working hours for such health surveillance procedures.";
        let result = classify(text);
        assert!(
            !result.contains(&APPLICATION_SCOPE),
            "'to whom this regulation applies' is a relative clause, not scope; got: {:?}",
            result
        );
    }

    #[test]
    fn no_false_scope_establishment_to_which_these_regs_apply() {
        // COMAH operator notification duty with relative clause qualifier
        let text = "The operator of any establishment to which these Regulations \
                    apply must notify the competent authority in advance of a \
                    significant increase or decrease in the quantity of dangerous \
                    substances.";
        let result = classify(text);
        assert!(
            !result.contains(&APPLICATION_SCOPE),
            "'to which these Regulations apply' is a relative clause, not scope; got: {:?}",
            result
        );
    }

    #[test]
    fn no_false_scope_fumigation_to_which_this_reg_applies() {
        // Fumigation duty with relative clause qualifier
        let text = "An employer shall not undertake fumigation to which this \
                    regulation applies unless he has notified the persons specified \
                    in Part I of Schedule 9.";
        let result = classify(text);
        assert!(
            !result.contains(&APPLICATION_SCOPE),
            "'to which this regulation applies' is a relative clause, not scope; got: {:?}",
            result
        );
    }

    #[test]
    fn no_false_scope_requirement_of_these_regs_which_applies() {
        // Workplace duty with "requirement of these Regulations which applies"
        let text = "Every employer shall ensure that every workplace under his \
                    control complies with any requirement of these Regulations \
                    which applies to that workplace.";
        let result = classify(text);
        assert!(
            !result.contains(&APPLICATION_SCOPE),
            "'requirement of these Regulations which applies' is a relative clause, not scope; got: {:?}",
            result
        );
    }

    #[test]
    fn genuine_scope_these_regs_shall_not_apply_still_matches() {
        // Genuine scope provision — should still match
        let text = "These Regulations shall not apply to or in relation to the \
                    master or crew of a sea-going ship.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "genuine 'These Regulations shall not apply' should match; got: {:?}",
            result
        );
    }

    #[test]
    fn genuine_scope_after_comma_still_matches() {
        // Genuine scope after comma — "Subject to..., these Regulations shall not apply"
        let text = "Subject to paragraph (3), these Regulations shall not apply \
                    until 6th July 2010.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "scope after comma should match; got: {:?}",
            result
        );
    }

    #[test]
    fn genuine_scope_after_number_still_matches() {
        // Genuine scope after paragraph number
        let text = "1 These Regulations shall not apply to sea-going ships.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "scope after paragraph number should match; got: {:?}",
            result
        );
    }

    // ── EU-specific APPLICATION_SCOPE tests ─────────────────────────

    #[test]
    fn classify_eu_these_provisions_shall_apply() {
        let text = "These provisions shall apply to the manufacture, placing on the \
                    market or use of such substances on their own, in mixtures or in articles.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "'These provisions shall apply to' should match; got: {:?}",
            result
        );
    }

    #[test]
    fn classify_eu_paragraphs_shall_not_apply() {
        let text = "Paragraphs 1 to 5 shall not apply to substances that have \
                    already been registered for that use.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "'Paragraphs 1 to 5 shall not apply' should match; got: {:?}",
            result
        );
    }

    #[test]
    fn classify_eu_articles_shall_not_apply() {
        let text = "Articles 21, 22 and 25 to 27 shall not apply to uses of \
                    substances regarded as registered according to Article 15.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "'Articles 21, 22 shall not apply' should match; got: {:?}",
            result
        );
    }

    #[test]
    fn classify_eu_this_directive_shall_apply() {
        let text = "This Directive shall apply to all sectors of activity, \
                    both public and private.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "'This Directive shall apply to' should match; got: {:?}",
            result
        );
    }

    #[test]
    fn classify_eu_paragraph_singular_shall_not_apply() {
        let text = "Paragraph 2 shall not apply to the use of substances in \
                    cosmetic products.";
        let result = classify(text);
        assert!(
            result.contains(&APPLICATION_SCOPE),
            "'Paragraph 2 shall not apply' should match; got: {:?}",
            result
        );
    }

    // ── Published purpose (agreed 2026-10-01) ───────────────────────

    use crate::taxa::duty_type::DutyType;

    fn p(text: &str) -> &'static str {
        primary(text, &[])
    }

    #[test]
    fn primary_machinery() {
        assert_eq!(p("These Regulations may be cited as the X Regulations 2024 and come into force on 1st April 2024 and extend to Great Britain."), ENACTMENT);
        assert_eq!(p(r#""premises" includes any place"#), INTERPRETATION);
        assert_eq!(p("A notice shall be treated as served if it is sent by post."), INTERPRETATION);
        assert_eq!(p("A certificate issued under this regulation shall be conclusive evidence of the matters stated in it."), INTERPRETATION);
        assert_eq!(p("An approval under paragraph (1) remains in force for the period specified in the approval."), INTERPRETATION);
        assert_eq!(p("These Regulations shall not apply to the master or crew of a ship."), EXEMPTION);
        assert_eq!(p("Nothing in this section requires an employer to keep a record."), EXEMPTION);
        assert_eq!(p("Nothing in this section makes the Crown criminally liable."), EXEMPTION);
        assert_eq!(p("This Act binds the Crown."), APPLICATION_SCOPE);
        assert_eq!(p("Nothing in these Regulations shall prejudice any other enactment."), APPLICATION_SCOPE);
        assert_eq!(p("These Regulations apply to every employer and self-employed person."), APPLICATION_SCOPE);
        assert_eq!(p("There shall be a body corporate to be known as the Health and Safety Executive."), ESTABLISHMENT);
        assert_eq!(p("The principal objective of the Regulator in carrying out its functions is to secure safety."), ESTABLISHMENT);
        assert_eq!(p("In section 3, for subsection (2) substitute the following."), AMENDMENT);
    }

    #[test]
    fn primary_sanctions() {
        assert_eq!(p("A person guilty of an offence under this section is liable on summary conviction to a fine."), OFFENCE);
        assert_eq!(p("It is an offence for a person to fail to discharge a duty to which he is subject."), OFFENCE);
        assert_eq!(p("It is a defence for an accused to prove that he took all reasonable precautions."), DEFENCE_APPEAL);
        assert_eq!(p("The operator shall not be liable for any loss arising from the closure."), LIABILITY);
        assert_eq!(p("An inspector may enter any premises at any reasonable time."), ENFORCEMENT);
    }

    #[test]
    fn primary_requirement_vs_procedure_detail() {
        // Creates its own duty, even a procedural one
        assert_eq!(primary("The Executive must consult the Secretary of State before issuing an approved code of practice.", &[DutyType::Obligation]), REQUIREMENT);
        assert_eq!(primary("Every employer shall make a suitable and sufficient assessment of the risks.", &[DutyType::Obligation]), REQUIREMENT);
        // Qualifies a duty created elsewhere
        assert_eq!(primary("An application under section 10 must be made in the prescribed form and be accompanied by the fee.", &[DutyType::Obligation]), PROCEDURE_DETAIL);
        assert_eq!(primary("The information referred to in paragraph (1) must be given in writing.", &[DutyType::Obligation]), PROCEDURE_DETAIL);
        assert_eq!(p("A statutory instrument containing regulations under this section is subject to annulment in pursuance of a resolution of either House of Parliament."), PROCEDURE_DETAIL);
        // Government deadline exception
        assert_eq!(p("The first domestic energy efficiency regulations must come into force no later than 1 April 2018."), REQUIREMENT);
        // Review clauses are a government Requirement
        assert_eq!(primary("The Secretary of State must review these Regulations and publish a report.", &[DutyType::Obligation]), REQUIREMENT);
    }

    #[test]
    fn primary_power_and_stems() {
        assert_eq!(primary("In carrying out that function the CMA may carry out, commission or support research.", &[DutyType::Liberty]), POWER_CONFERRED);
        assert_eq!(primary("The employee may request a copy of the record and the employer shall provide it.", &[DutyType::Obligation, DutyType::Liberty]), POWER_CONFERRED);
        // A list item with no purpose of its own takes its stem's
        let item = p("to secure that the registers maintained by them are available at all reasonable times;");
        assert_eq!(item, UNCLASSIFIED);
        assert_eq!(inherit_from_stem(item, [UNCLASSIFIED, REQUIREMENT]), REQUIREMENT);
        // An item that does something itself keeps it
        assert_eq!(inherit_from_stem(EXEMPTION, [REQUIREMENT]), EXEMPTION);
        assert_eq!(p("...."), UNCLASSIFIED);
    }
}
