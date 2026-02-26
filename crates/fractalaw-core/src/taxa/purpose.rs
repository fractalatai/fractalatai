//! Purpose classifier for UK ESH legal text.
//!
//! Purpose identifies WHAT the law does (function-based), as opposed to
//! `duty_type` which identifies WHO has obligations (role-based).
//!
//! 15 purpose categories, using `+` as separator (avoids CSV issues).
//!
//! Ported from `Taxa.PurposeClassifier`.

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
];

// ── Pattern definitions ──────────────────────────────────────────────

const RAW_PATTERNS: &[(&str, &str)] = &[
    (
        ENACTMENT,
        // "Commencement Information" is editorial metadata from legislation.gov.uk, not
        // actual commencement content. Use negative lookahead to exclude it, and require
        // "commencement" to appear near citation/force context or as a heading.
        r"(?i)(?:(?:Act|Regulations?|Order) may be cited as|(?:Act|Regulations?|Order).*?shall have effect|(?:Act|Regulations?|Order) shall come into (?:force|operation)|comes? into force|has effect.*?on or after|[Cc]itation and commencement|commencement (?:date|order|provision|of this)|[Cc]ommencement\s*$)",
    ),
    (
        INTERPRETATION,
        r#"(?i)(?:[A-Za-z\d ][""\u{201c}].*?(?:means|includes|does not include|is (?:information|the)|are|to be read as|are references to|consists)|[""\u{201c}].*?[""\u{201d}] is|In thi?e?se? [Rr]egulations?.*?[\u{2014}\u{2014}\u{2014}]|has?v?e? the (?:same )?(?:respective )?meanings?|[Ff]or the purposes? of (?:this Act|determining|these Regulations)|(?:any reference|references?).*?to|[Ii]nterpretation|for the meaning of)"#,
    ),
    (
        APPLICATION_SCOPE,
        r"(?i)(?:Application|(?:Act|Part|Chapter|[Ss]ections?|[Rr]egulations?|[Pp]aragraphs?|Article).*?apply?i?e?s?|(?:Act|Part|Chapter|[Ss]ections?|[Rr]egulations?|[Pp]aragraphs?|[Ss]chedules?).*?has effect|does not apply|shall.*?apply|shall have effect|shall have no effect|ceases to have effect|provisions of.*?apply|apply to any work outside|apply to a self-employed person|shall bind the Crown)",
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
        result.push(PROCESS_RULE);
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
        assert!(result.contains(&APPLICATION_SCOPE));
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
}
