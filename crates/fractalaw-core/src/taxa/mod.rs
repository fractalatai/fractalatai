//! Regex-based DRRP classification pipeline for UK ESH legislative text.
//!
//! Migrated from the sertantai Elixir `Taxa.*` modules. Each sub-module is
//! independently testable and operates on plain `&str` input — no DB, no
//! network, no AI.
//!
//! ## Pipeline stages
//!
//! 1. [`text_cleaner`] — HTML stripping, whitespace normalisation
//! 2. [`actors`] — actor (duty-holder) extraction
//! 3. [`duty_patterns`] — regex pattern definitions (government/governed)
//! 4. [`duty_type`] — top-level DRRP classifier
//! 5. [`clause_refiner`] — modal-window clause extraction
//! 6. [`confidence`] — regex clause confidence scoring
//! 7. [`popimar`] — POPIMAR management-system taxonomy
//! 8. [`purpose`] — function-based purpose classification
//! 9. [`making`] — Making / Not-Making pre-filter
//!
//! ## Usage
//!
//! ```
//! use fractalaw_core::taxa;
//!
//! let raw_text = "<p>The employer shall ensure the health and safety of employees.</p>";
//! let record = taxa::parse(raw_text);
//! assert!(!record.duty_types.is_empty());
//! ```

pub mod actors;
pub mod clause_refiner;
pub mod confidence;
pub mod duty_patterns;
pub mod duty_type;
pub mod making;
pub mod popimar;
pub mod purpose;
pub mod text_cleaner;

use duty_type::DutyType;

/// A fully classified Taxa record — the output of the pipeline.
#[derive(Debug, Clone, Default)]
pub struct TaxaRecord {
    /// Cleaned text that was analysed.
    pub cleaned_text: String,

    /// Governed actors found (employers, employees, etc.).
    pub governed_actors: Vec<String>,
    /// Government actors found (ministers, authorities, etc.).
    pub government_actors: Vec<String>,

    /// DRRP duty types (Duty, Right, Responsibility, Power).
    pub duty_types: Vec<DutyType>,

    /// POPIMAR management categories.
    pub popimar: Vec<&'static str>,

    /// Purpose categories (function-based classification).
    pub purposes: Vec<&'static str>,

    /// Pattern classification detail (if any).
    pub classification: Option<duty_patterns::DutyClassification>,
}

/// Run the full Taxa classification pipeline on raw legislative text.
///
/// Steps:
/// 1. Clean the text (HTML strip, normalise whitespace)
/// 2. Classify purpose (EARLY GATE — skip DRRP if non-DRRP purpose)
/// 3. Extract actors (only if DRRP-bearing)
/// 4. Classify duty type (DRRP) (only if DRRP-bearing)
/// 5. Classify POPIMAR categories (only if DRRP-bearing)
pub fn parse(raw_text: &str) -> TaxaRecord {
    if raw_text.trim().is_empty() {
        return TaxaRecord::default();
    }

    // Step 1: Clean
    let cleaned = text_cleaner::clean(raw_text);

    // Step 2: Purpose classification (EARLY GATE)
    let purposes = purpose::classify(&cleaned);

    // Step 3: Check if we should skip DRRP processing
    if should_skip_drrp(&purposes) {
        return TaxaRecord {
            cleaned_text: cleaned,
            purposes,
            ..Default::default()
        };
    }

    // Step 4: Extract actors (only for DRRP-bearing provisions)
    let extracted = actors::extract_actors(&cleaned);

    // Step 5: Classify duty type (DRRP)
    let lower = cleaned.to_lowercase();
    let cr = duty_type::classify(&lower);

    // Step 6: POPIMAR (use duty type labels for default logic)
    let dt_labels: Vec<&str> = cr.duty_types.iter().map(|d| d.as_str()).collect();
    let popimar = popimar::classify_with_duty_types(&cleaned, &dt_labels);

    TaxaRecord {
        cleaned_text: cleaned,
        governed_actors: extracted.governed,
        government_actors: extracted.government,
        duty_types: cr.duty_types,
        popimar,
        purposes,
        classification: cr.classification,
    }
}

/// Determine if DRRP classification should be skipped based on purpose.
///
/// Provisions with these purposes are structural/administrative and
/// rarely contain actionable duties/rights/responsibilities/powers:
/// - Interpretation/Definition (36.4% DRRP rate in current data)
/// - Amendment (37.0% DRRP rate)
/// - Repeal/Revocation (62.5% DRRP rate)
///
/// Skipping these reduces false positives and improves performance.
fn should_skip_drrp(purposes: &[&str]) -> bool {
    const SKIP_PURPOSES: &[&str] = &[
        purpose::INTERPRETATION,
        purpose::AMENDMENT,
        purpose::REPEAL_REVOCATION,
    ];

    purposes.iter().any(|p| SKIP_PURPOSES.contains(p))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_employer_duty() {
        let record = parse("The employer shall ensure the health and safety of employees.");
        assert!(record.duty_types.contains(&DutyType::Duty));
        assert!(
            record
                .governed_actors
                .iter()
                .any(|a| a.contains("Employer"))
        );
        assert!(!record.popimar.is_empty());
    }

    #[test]
    fn parse_government_responsibility() {
        let record = parse("The Secretary of State shall have power to make regulations.");
        assert!(record.duty_types.contains(&DutyType::Responsibility));
        assert!(
            record
                .government_actors
                .iter()
                .any(|a| a.contains("Minister"))
        );
    }

    #[test]
    fn parse_html_cleaned() {
        let record = parse("<p>The employer <b>shall</b> ensure safety.</p>");
        assert!(!record.cleaned_text.contains('<'));
        assert!(record.duty_types.contains(&DutyType::Duty));
    }

    #[test]
    fn parse_empty() {
        let record = parse("");
        assert!(record.duty_types.is_empty());
        assert!(record.governed_actors.is_empty());
    }

    #[test]
    fn parse_purpose_detected() {
        let record = parse("This Act may be cited as the Health and Safety at Work etc. Act 1974.");
        assert!(record.purposes.iter().any(|p| p.contains("Enactment")));
    }

    #[test]
    fn parse_full_pipeline_hswa_s2() {
        let text = "It shall be the duty of every employer to ensure, so far as \
                    is reasonably practicable, the health, safety and welfare at \
                    work of all his employees.";
        let record = parse(text);
        assert!(record.duty_types.contains(&DutyType::Duty));
        assert!(!record.popimar.is_empty());
        assert!(!record.purposes.is_empty());
    }

    // ── Purpose-based pre-filtering tests ───────────────────────────

    #[test]
    fn skip_interpretation_section() {
        let text =
            r#"In these Regulations— "employer" means a person who employs one or more employees."#;
        let record = parse(text);
        // Purpose detected
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        // DRRP classification skipped
        assert!(record.duty_types.is_empty());
        assert!(record.governed_actors.is_empty());
        assert!(record.government_actors.is_empty());
        assert!(record.popimar.is_empty());
    }

    #[test]
    fn skip_amendment_section() {
        let text = "In section 3, for subsection (2) substitute the following provisions.";
        let record = parse(text);
        // Purpose detected
        assert!(record.purposes.contains(&purpose::AMENDMENT));
        // DRRP classification skipped
        assert!(record.duty_types.is_empty());
        assert!(record.governed_actors.is_empty());
        assert!(record.government_actors.is_empty());
        assert!(record.popimar.is_empty());
    }

    #[test]
    fn skip_repeal_section() {
        let text = "The following Acts shall cease to have effect and are hereby repealed.";
        let record = parse(text);
        // Purpose detected
        assert!(record.purposes.contains(&purpose::REPEAL_REVOCATION));
        // DRRP classification skipped
        assert!(record.duty_types.is_empty());
    }

    #[test]
    fn process_drrp_section() {
        let text = "Every employer shall ensure the health and safety of employees.";
        let record = parse(text);
        // Purpose detected (Process+Rule)
        assert!(record.purposes.contains(&purpose::PROCESS_RULE));
        // DRRP classification runs
        assert!(!record.duty_types.is_empty());
        assert!(!record.governed_actors.is_empty());
    }

    #[test]
    fn amendment_with_modal_verbs_skipped() {
        // This is the false positive case: amendment text contains "shall"
        // but it's structural (inserting text), not a primary duty
        let text = r#"In section 3, for subsection (2) substitute— "The Scottish Ministers shall ensure targets are met.""#;
        let record = parse(text);
        // Purpose: Amendment detected
        assert!(record.purposes.contains(&purpose::AMENDMENT));
        // DRRP: Skipped (even though "shall" is present)
        assert!(record.duty_types.is_empty());
    }

    #[test]
    fn multiple_purposes_with_skip_purpose() {
        // If ANY purpose is in SKIP list, entire provision is skipped
        let text = r#"For the purposes of interpretation, "employer" means a person who shall ensure safety."#;
        let record = parse(text);
        // Multiple purposes may be detected
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        // But presence of INTERPRETATION triggers skip
        assert!(record.duty_types.is_empty());
    }
}
