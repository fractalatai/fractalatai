//! POPIMAR taxonomy classifier for UK ESH legal text.
//!
//! POPIMAR (Policy, Organisation, Planning, Implementation, Monitoring,
//! Audit, Review) is a management-system framework used to categorise legal
//! requirements by the type of management action they mandate.
//!
//! Ported from `Taxa.Popimar` + `Taxa.PopimarLib`.

use std::sync::LazyLock;

use regex::Regex;

// ── Categories ───────────────────────────────────────────────────────

/// All 16 POPIMAR categories in display/priority order.
pub const CATEGORIES: &[&str] = &[
    "Policy",
    "Organisation",
    "Organisation - Control",
    "Organisation - Communication & Consultation",
    "Organisation - Collaboration, Coordination, Cooperation",
    "Organisation - Competence",
    "Organisation - Costs",
    "Records",
    "Permit, Authorisation, License",
    "Aspects and Hazards",
    "Planning & Risk / Impact Assessment",
    "Risk Control",
    "Notification",
    "Maintenance, Examination and Testing",
    "Checking, Monitoring",
    "Review",
];

/// Duty types that trigger POPIMAR classification.
const RELEVANT_DUTY_TYPES: &[&str] = &[
    "Duty",
    "Right",
    "Responsibility",
    "Discretionary",
    "Process, Rule, Constraint, Condition",
];

// ── Pattern definitions (one regex per category) ─────────────────────

/// (category_label, regex_pattern) pairs — compiled once via COMPILED.
const RAW_PATTERNS: &[(&str, &str)] = &[
    (
        "Policy",
        r"(?i)(?:[Pp]olicy?i?e?s?|[Oo]bjectives?|[Ss]trateg)",
    ),
    (
        "Organisation",
        r"(?i)(?:[Oo]rg.? chart|[Oo]rganisation chart|making of appointments?|(?:must|may|shall)\s?(?:jointly)?\s?appoint|person.*?appointed|appoint a person)",
    ),
    (
        "Organisation - Control",
        r"(?i)(?:[Pp]rocess|[Pp]rocedure|[Ww]ork instruction|[Mm]ethod statement|[Ii]nstruction|comply?i?e?s? with.*?(?:duties|requirements)|is responsible for|has control over|(?:supervised?|supervising))",
    ),
    (
        "Organisation - Communication & Consultation",
        r"(?i)(?:[Cc]ommunicat|[Cc]onsult|(?:send a copy of it|be sent) to|must identify to|publish a report|must (?:immediately )?inform|report to|(?:by|to) provide?i?n?g?.*?information|made available to (?:the public)|supplied (?:in writing|with a copy)|aware of the contents of)",
    ),
    (
        "Organisation - Collaboration, Coordination, Cooperation",
        r"(?i)(?:[Cc]ollaborat|[Cc]oordinat|[Cc]ooperat)",
    ),
    (
        "Organisation - Competence",
        r"(?i)(?:[Cc]ompeten(?:t|ce|cy)\s|[Tt]raining|[Ii]nformation, instruction and training|[Ii]nformation.*?provided to every person|provide.*?information|person satisfies the criteria|skills, knowledge and experience|organisational capability|instructe?d?)",
    ),
    (
        "Organisation - Costs",
        r"(?i)(?:[Cc]ost[- ]benefit|[Nn]ett? cost|[Ff]ee[\s[:punct:]]|[Cc]harge|[Ff]inancial loss)",
    ),
    (
        "Records",
        r"(?i)(?:(?:[Rr]ecord|[Rr]eport [^t]|[Rr]egister)|[Ll]ogbook|[Ii]nventory|[Dd]atabase|(?:[Ee]nforcement|[Pp]rohibition|[Ii]mprovement) notice|[Dd]ocuments?|(?:marke?d?i?n?g?|labelled)|must be kept|certificate|health and safety file)",
    ),
    (
        "Permit, Authorisation, License",
        r"(?i)(?:[Pp]ermit[\s[:punct:]]|[Aa]uthorisation|[Aa]uthorised [^r]|[Ll]i[sc]en[sc]ed?|[Ll]i[sc]en[sc]ing)",
    ),
    (
        "Aspects and Hazards",
        r"(?i)(?:[Aa]spects and impacts|[Hh]azard)",
    ),
    (
        "Planning & Risk / Impact Assessment",
        r"(?i)(?:[Aa]nnual plan|[Ss]trategic plan|[Bb]usiness plan|[Pp]lan of work|construction phase plan|written plan|measures? to be specified in the plan|(?:project|action) plan|project is planned|[Ii]mpact [Aa]ssessment|[Rr]isk [Aa]ssessment|assessment of any risks|suitable and sufficient assessment|[Ii]n making the assessment|(?:reassess|reassessed|reassessment)|general principles of prevention|identify and eliminate)",
    ),
    (
        "Risk Control",
        r"(?i)(?:avoid the need|suitable and sufficient steps|steps as are reasonable in the circumstances must be taken|taken? all reasonable steps|takes immediate steps|[Rr]isk [Cc]ontrol|control.*?risk|[Rr]isk mitigation|use the best available techniques not entailing excessive cost|eliminates.*?the risk|reduces? the risk|provided to.*?employees|provision and use of|safety management system|corrective measures?|meets the requirements?|standards for the construction|shall make full and proper use|measures?.*?specified.*?plan|take such measures)",
    ),
    (
        "Notification",
        r"(?i)(?:given?.*?notice|accident report|[Nn]otify|[Nn]otification|[Aa]pplication for|publish.*?a notice)",
    ),
    (
        "Maintenance, Examination and Testing",
        r"(?i)(?:[Mm]aintenance|[Mm]aintaine?d?|[Ee]xamination|[Tt]esting|[Ii]nspecti?o?n?e?d?)",
    ),
    (
        "Checking, Monitoring",
        r"(?i)(?:[Cc]heck|[Mm]onitor|medical exam|at least once every.*?years|kept available for inspection)",
    ),
    (
        "Review",
        r"(?i)(?:[Mm]anagement review|(?:[Rr]eviewed|is [Rr]evised)|(?:conduct|carry out|carrying out) (?:a|the) review|review the (?:assessment))",
    ),
];

static COMPILED: LazyLock<Vec<(&str, Regex)>> = LazyLock::new(|| {
    RAW_PATTERNS
        .iter()
        .map(|(cat, pat)| (*cat, Regex::new(pat).unwrap()))
        .collect()
});

// ── Public API ───────────────────────────────────────────────────────

/// Classify text into POPIMAR categories.
///
/// Returns a sorted list of matching category names.
pub fn classify(text: &str) -> Vec<&'static str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut result: Vec<&str> = COMPILED
        .iter()
        .filter(|(_, re)| re.is_match(text))
        .map(|(cat, _)| *cat)
        .collect();
    sort_categories(&mut result);
    result
}

/// Classify text, defaulting to "Risk Control" if no matches but
/// relevant duty types are present.
pub fn classify_with_duty_types(text: &str, duty_types: &[&str]) -> Vec<&'static str> {
    let result = classify(text);
    if result.is_empty() && has_relevant_duty_types(duty_types) {
        vec!["Risk Control"]
    } else {
        result
    }
}

/// Sort categories by the canonical POPIMAR priority order.
pub fn sort_categories(cats: &mut Vec<&str>) {
    cats.sort_by_key(|c| CATEGORIES.iter().position(|&k| k == *c).unwrap_or(99));
}

fn has_relevant_duty_types(duty_types: &[&str]) -> bool {
    duty_types.iter().any(|dt| RELEVANT_DUTY_TYPES.contains(dt))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_match() {
        let result = classify("The organisation shall establish a health and safety policy.");
        assert!(result.contains(&"Policy"));
    }

    #[test]
    fn risk_control_match() {
        let result =
            classify("The employer shall take such measures as are necessary to reduce the risk.");
        assert!(result.contains(&"Risk Control"));
    }

    #[test]
    fn training_competence_match() {
        let result =
            classify("The employer shall ensure every employee receives adequate training.");
        assert!(result.contains(&"Organisation - Competence"));
    }

    #[test]
    fn risk_assessment_match() {
        let result =
            classify("Every employer shall make a suitable and sufficient assessment of risks.");
        assert!(result.contains(&"Planning & Risk / Impact Assessment"));
    }

    #[test]
    fn notification_match() {
        let result = classify("The employer shall notify the enforcing authority of any accident.");
        assert!(result.contains(&"Notification"));
    }

    #[test]
    fn records_match() {
        let result = classify("A record of the assessment must be kept available for inspection.");
        assert!(result.contains(&"Records"));
        assert!(result.contains(&"Checking, Monitoring"));
    }

    #[test]
    fn multiple_categories() {
        let text = "The employer shall provide training and maintain a record of risk assessments.";
        let result = classify(text);
        assert!(result.len() >= 2);
    }

    #[test]
    fn empty_text() {
        assert!(classify("").is_empty());
    }

    #[test]
    fn default_risk_control_with_duty_types() {
        let result = classify_with_duty_types("some generic text without keywords", &["Duty"]);
        assert_eq!(result, vec!["Risk Control"]);
    }

    #[test]
    fn no_default_without_relevant_duty_types() {
        let result = classify_with_duty_types("some generic text", &["Transitional Arrangement"]);
        assert!(result.is_empty());
    }

    #[test]
    fn category_sort_order() {
        let mut cats = vec!["Review", "Policy", "Risk Control"];
        sort_categories(&mut cats);
        assert_eq!(cats, vec!["Policy", "Risk Control", "Review"]);
    }
}
