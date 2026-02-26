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
/// Only skips when ALL purposes are structural/administrative — pure
/// definitions, amendments, or repeals. Multi-purpose provisions (e.g.,
/// Interpretation + Process+Rule) still get DRRP processing because
/// they often contain genuine duties alongside definitional framing.
///
/// Uses ALL strategy (not ANY) after false-negative validation showed
/// 85/189 skipped provisions had mixed purposes with real DRRP content.
/// ALL gives 104 clean skips (9.9%) with no false negatives, vs ANY's
/// 189 skips (18.1%) with 58 false negatives (30.7% error rate).
fn should_skip_drrp(purposes: &[&str]) -> bool {
    const SKIP_PURPOSES: &[&str] = &[
        purpose::INTERPRETATION,
        purpose::AMENDMENT,
        purpose::REPEAL_REVOCATION,
    ];

    !purposes.is_empty() && purposes.iter().all(|p| SKIP_PURPOSES.contains(p))
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
    fn amendment_with_modal_verbs_not_skipped() {
        // Amendment text containing "shall" also triggers Process+Rule.
        // With ALL strategy, mixed purposes are NOT skipped — the duty
        // content in the quoted substitution text gets DRRP-parsed.
        let text = r#"In section 3, for subsection (2) substitute— "The Scottish Ministers shall ensure targets are met.""#;
        let record = parse(text);
        assert!(record.purposes.contains(&purpose::AMENDMENT));
        assert!(record.purposes.contains(&purpose::PROCESS_RULE));
        // NOT skipped — mixed purposes, DRRP runs
        assert!(!record.duty_types.is_empty());
    }

    #[test]
    fn pure_amendment_skipped() {
        // A pure amendment provision with no modal verbs — only Amendment purpose.
        let text = "In section 3, for subsection (2) substitute the following provisions.";
        let record = parse(text);
        assert!(record.purposes.contains(&purpose::AMENDMENT));
        assert!(!record.purposes.contains(&purpose::PROCESS_RULE));
        // Pure amendment — gate skips DRRP
        assert!(record.duty_types.is_empty());
    }

    #[test]
    fn mixed_purposes_not_skipped() {
        // Multi-purpose provisions (Interpretation + Process+Rule) should NOT be
        // skipped — they often contain genuine duties alongside definitional framing.
        // Gate uses ALL strategy: only skips when ALL purposes are skip-purposes.
        let text = r#"For the purposes of interpretation, "employer" means a person who shall ensure safety."#;
        let record = parse(text);
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        assert!(record.purposes.contains(&purpose::PROCESS_RULE));
        // NOT skipped — DRRP classification runs because Process+Rule is present
        assert!(!record.duty_types.is_empty());
    }

    #[test]
    fn pure_skip_purposes_skipped() {
        // Provision with ONLY skip-purposes should still be skipped.
        let text =
            r#"In these Regulations— "employer" means a person who employs one or more employees."#;
        let record = parse(text);
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        // No non-skip purposes present — gate triggers
        assert!(record.duty_types.is_empty());
    }

    // ── True-negative regression tests (Iteration 1: contractor) ─────
    // Full-pipeline tests: provisions mentioning "contractor" that should
    // NOT produce DRRP output. These guard against false positives when
    // expanding the GOVERNED_ACTORS list.

    #[test]
    fn contractor_definition_no_drrp() {
        // CDM 2015 reg 2 interpretation — defines "contractor", not a duty
        let text = r#"In these Regulations— "contractor" means any person (including a non-domestic client) who, in the course or furtherance of a business, carries out, manages or controls construction work."#;
        let record = parse(text);
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        assert!(record.duty_types.is_empty());
    }

    #[test]
    fn contractor_appointment_cross_ref_no_drrp() {
        // CDM 2015 reg 8(1) — transitional, states who IS principal contractor
        let text = "Where, immediately before 6th April 2015 there is a principal \
                    contractor appointed for a relevant project under regulation \
                    14(2) of the 2007 Regulations, for the purposes of these \
                    Regulations that person is the principal contractor.";
        let record = parse(text);
        // No modal verb — no DRRP even though "contractor" is present
        assert!(record.duty_types.is_empty());
    }

    // ── True-positive tests (Iteration 1: contractor) ────────────────
    // These provisions have "contractor" + obligation modal and SHOULD
    // produce DRRP output. Written as failing tests first (step 2),
    // then the pattern change makes them pass (step 3).

    #[test]
    fn contractor_duty_plan_manage_monitor() {
        // CDM 2015 reg 15(2) — clear duty on contractor
        let text = "a contractor must plan, manage and monitor construction work \
                    carried out either by the contractor or by workers under the \
                    contractor's control, to ensure that, so far as is reasonably \
                    practicable, it is carried out without risks to health and safety.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Duty),
            "contractor obligation should classify as Duty, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn principal_contractor_duty_construction_phase_plan() {
        // CDM 2015 reg 12(1) — duty on principal contractor
        let text = "during the pre-construction phase, and before setting up a \
                    construction site, the principal contractor must draw up a \
                    construction phase plan, or make arrangements for a \
                    construction phase plan to be drawn up.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Duty),
            "principal contractor obligation should classify as Duty, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn contractor_prohibition() {
        // CDM 2015 reg 15(1) — prohibition on contractor
        let text = "a contractor must not carry out construction work in relation \
                    to a project unless satisfied that the client is aware of the \
                    duties owed by the client under these regulations.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Duty),
            "contractor prohibition should classify as Duty, got: {:?}",
            record.duty_types
        );
    }
}
