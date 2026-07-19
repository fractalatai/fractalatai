//! JSP cross-reference extraction — legislation, JSPs, and standards.
//!
//! Extracts typed references from JSP provision text:
//! - Legislation: "Health and Safety at Work Act 1974", "Electricity at Work Regulations 1989"
//! - JSP cross-refs: "JSP 375 Volume 1, Chapter 23", "JSP 850"
//! - Standards: "BS 7671", "BS EN ISO 14001"
//! - HSE guidance: "HSG85", "HSE L22", "INDG139"

use regex::Regex;
use std::sync::LazyLock;

/// A cross-reference extracted from JSP text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JspReference {
    /// What kind of thing is referenced.
    pub target_type: &'static str,
    /// The matched text as written.
    pub citation: String,
    /// Byte offset of the match start in the original text.
    pub offset: usize,
    /// Resolved identifier (law_name for legislation, source_id for JSPs).
    /// Set during resolution, not during extraction.
    pub resolved_id: Option<String>,
}

// ── Legislation patterns ────────────────────────────────────────────

static LEGISLATION_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // Named Acts: "Health and Safety at Work etc. Act 1974"
        // Starts with a capital letter, allows lowercase words (at, of, and, etc.),
        // ends at "Act YYYY" or "Order YYYY". Limited to 80 chars to avoid runaway.
        (
            Regex::new(r"(?:the\s+)?([A-Z][A-Za-z.,()&\s]{3,80}?(?:Act|Order)\s+\d{4})").unwrap(),
            "legislation",
        ),
        // Named Regulations/Rules: "Electricity at Work Regulations 1989"
        (
            Regex::new(r"(?:the\s+)?([A-Z][A-Za-z.,()&\s]{3,80}?(?:Regulations?|Rules?)\s+\d{4})").unwrap(),
            "legislation",
        ),
    ]
});

// ── JSP cross-reference patterns ────────────────────────────────────

static JSP_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // "JSP 375 Volume 1, Chapter 23" / "JSP 375, Volume 1, Chapter 23"
        // "JSP 375 Vol 1 Chapter 23"
        (
            Regex::new(
                r"JSP\s+(\d{3})\s*(?:,\s*)?(?:Vol(?:ume)?\s*(\d+)\s*(?:,\s*)?)?(?:Ch(?:apter)?\s*(\d+))?"
            ).unwrap(),
            "jsp",
        ),
        // "JSP 850" (bare JSP number)
        (
            Regex::new(r"JSP\s+(\d{3})\b").unwrap(),
            "jsp",
        ),
    ]
});

// ── Standard/guidance patterns ──────────────────────────────────────

static STANDARD_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // British Standards: "BS 7671", "BS EN ISO 14001", "BS EN 60079"
        (
            Regex::new(r"BS\s+(?:EN\s+)?(?:ISO\s+)?(\d+)(?::(\d{4}))?").unwrap(),
            "standard",
        ),
        // ECE Regulations: "ECE Regulation 100"
        (
            Regex::new(r"ECE\s+Regulation\s+(\d+)").unwrap(),
            "standard",
        ),
        // HSE guidance series: "HSG85", "HSG107"
        (
            Regex::new(r"\bHSG\s*(\d+)\b").unwrap(),
            "guidance",
        ),
        // HSE L-series ACoPs: "HSE L22", "L8"
        (
            Regex::new(r"\b(?:HSE\s+)?L(\d+)\b").unwrap(),
            "guidance",
        ),
        // HSE INDG series: "INDG139"
        (
            Regex::new(r"\bINDG\s*(\d+)\b").unwrap(),
            "guidance",
        ),
    ]
});

/// Extract all cross-references from JSP provision text.
///
/// Returns deduplicated references sorted by offset.
pub fn extract_references(text: &str) -> Vec<JspReference> {
    let mut refs = Vec::new();

    // Extract legislation references
    for (re, target_type) in LEGISLATION_PATTERNS.iter() {
        for m in re.find_iter(text) {
            let citation = m.as_str().trim().to_string();
            // Skip very short matches that are likely false positives
            if citation.len() < 10 {
                continue;
            }
            refs.push(JspReference {
                target_type,
                citation,
                offset: m.start(),
                resolved_id: None,
            });
        }
    }

    // Extract JSP cross-references
    for (re, target_type) in JSP_PATTERNS.iter() {
        for m in re.find_iter(text) {
            refs.push(JspReference {
                target_type,
                citation: m.as_str().trim().to_string(),
                offset: m.start(),
                resolved_id: None,
            });
        }
    }

    // Extract standards/guidance
    for (re, target_type) in STANDARD_PATTERNS.iter() {
        for m in re.find_iter(text) {
            refs.push(JspReference {
                target_type,
                citation: m.as_str().trim().to_string(),
                offset: m.start(),
                resolved_id: None,
            });
        }
    }

    // Sort by offset, dedup by citation
    refs.sort_by_key(|r| r.offset);
    refs.dedup_by(|a, b| a.citation == b.citation && a.offset == b.offset);

    // Remove overlapping matches — keep the longest match at each position
    let mut filtered: Vec<JspReference> = Vec::new();
    for r in refs {
        if let Some(last) = filtered.last() {
            // If this match starts within the previous match, skip it (shorter overlap)
            if r.offset < last.offset + last.citation.len() {
                // Keep whichever is longer
                if r.citation.len() > last.citation.len() {
                    filtered.pop();
                    filtered.push(r);
                }
                continue;
            }
        }
        filtered.push(r);
    }

    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_act_reference() {
        let refs = extract_references("compliance with the Health and Safety at Work Act 1974");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target_type, "legislation");
        assert!(refs[0].citation.contains("Health and Safety at Work"));
        assert!(refs[0].citation.contains("1974"));
    }

    #[test]
    fn extracts_regulations_reference() {
        let refs = extract_references(
            "in accordance with the Electricity at Work Regulations 1989",
        );
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target_type, "legislation");
        assert!(refs[0].citation.contains("Electricity at Work Regulations 1989"));
    }

    #[test]
    fn extracts_jsp_with_volume_chapter() {
        let refs = extract_references(
            "as set out in JSP 375 Volume 1, Chapter 9 (Dangerous Substances)",
        );
        assert!(refs.iter().any(|r| r.target_type == "jsp"
            && r.citation.contains("JSP 375")
            && r.citation.contains("Chapter 9")));
    }

    #[test]
    fn extracts_bare_jsp_number() {
        let refs = extract_references("see JSP 850 for further details");
        assert!(refs.iter().any(|r| r.target_type == "jsp" && r.citation.contains("JSP 850")));
    }

    #[test]
    fn extracts_british_standard() {
        let refs = extract_references("in accordance with BS 7671 - IET Wiring Regulations");
        assert!(refs.iter().any(|r| r.target_type == "standard" && r.citation.contains("BS 7671")));
    }

    #[test]
    fn extracts_hsg_guidance() {
        let refs = extract_references("HSG85 - Electricity at work");
        assert!(refs.iter().any(|r| r.target_type == "guidance" && r.citation.contains("HSG85")));
    }

    #[test]
    fn extracts_hse_l_series() {
        let refs = extract_references("HSE L22 – Safe use of work equipment");
        assert!(refs.iter().any(|r| r.target_type == "guidance" && r.citation.contains("L22")));
    }

    #[test]
    fn extracts_indg() {
        let refs = extract_references("INDG139 - Using electric storage batteries safely");
        assert!(refs.iter().any(|r| r.target_type == "guidance" && r.citation.contains("INDG139")));
    }

    #[test]
    fn extracts_multiple_references() {
        let text = "compliance with the Electricity at Work Regulations 1989 and JSP 375 Volume 1, Chapter 22";
        let refs = extract_references(text);
        assert!(refs.iter().any(|r| r.target_type == "legislation"));
        assert!(refs.iter().any(|r| r.target_type == "jsp"));
    }

    #[test]
    fn no_references_in_plain_text() {
        let refs = extract_references("All electrical equipment must be safely maintained.");
        assert!(refs.is_empty());
    }

    #[test]
    fn extracts_dsear() {
        let refs = extract_references(
            "in accordance with the Dangerous Substances and Explosive Atmospheres Regulations 2002 (DSEAR)",
        );
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target_type, "legislation");
    }

    #[test]
    fn extracts_fire_safety_order() {
        let refs = extract_references(
            "the Regulatory Reform (Fire Safety) Order 2005",
        );
        assert!(refs.iter().any(|r| r.target_type == "legislation"
            && r.citation.contains("Order 2005")));
    }

    #[test]
    fn real_jsp_provision_related_docs() {
        // From JSP-375-CH23 para.41 — the related documents list
        let text = "Health and Safety at Work, etc. Act 1974 \
                    Management of Health and Safety at Work Regulations 1999 \
                    Electricity at Work Regulations 1989";
        let refs = extract_references(text);
        let leg_refs: Vec<_> = refs.iter().filter(|r| r.target_type == "legislation").collect();
        assert!(leg_refs.len() >= 3, "expected at least 3 legislation refs, got {}", leg_refs.len());
    }
}
