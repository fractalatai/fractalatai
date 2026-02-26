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
pub mod duty_patterns_v2;
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

    /// Focused clause extract — the "who must do what" snippet.
    /// Extracted from the matched span (v2) or via modal-window refiner (v1/government).
    pub clause_refined: Option<String>,

    /// Confidence score for the extracted clause (0.0..=1.0).
    /// Based on heuristics: span capture, clean ending, length, modal strength.
    pub taxa_confidence: f32,
}

/// Run the full Taxa classification pipeline on raw legislative text.
///
/// Delegates to `parse_v2()` — the actor-anchored pipeline is now the default.
pub fn parse(raw_text: &str) -> TaxaRecord {
    parse_v2(raw_text)
}

/// Run the actor-anchored Taxa classification pipeline on raw legislative text.
///
/// Steps:
/// 1. Clean the text (HTML strip, normalise whitespace)
/// 2. Classify purpose (EARLY GATE — skip DRRP if non-DRRP purpose)
/// 3. Extract actors (only if DRRP-bearing)
/// 4. Classify duty type via actor-anchored v2 patterns
/// 5. Classify POPIMAR categories (only if DRRP-bearing)
/// 6. Extract focused clause from match span
pub fn parse_v2(raw_text: &str) -> TaxaRecord {
    if raw_text.trim().is_empty() {
        return TaxaRecord::default();
    }

    let cleaned = text_cleaner::clean(raw_text);
    let purposes = purpose::classify(&cleaned);

    if should_skip_drrp(&purposes) {
        return TaxaRecord {
            cleaned_text: cleaned,
            purposes,
            ..Default::default()
        };
    }

    let extracted = actors::extract_actors(&cleaned);
    let lower = cleaned.to_lowercase();
    let cr = duty_type::classify(&lower, &extracted.governed, &extracted.government);

    let dt_labels: Vec<&str> = cr.duty_types.iter().map(|d| d.as_str()).collect();
    let popimar = popimar::classify_with_duty_types(&cleaned, &dt_labels);

    // Extract focused clause from match span or fall back to modal-window refiner
    let clause_refined = extract_clause(&cleaned, cr.classification.as_ref());

    // Score clause confidence
    let has_span = cr.classification.as_ref().is_some_and(|c| c.span.is_some());
    let taxa_confidence = clause_refined
        .as_deref()
        .map(|c| confidence::score(c, has_span))
        .unwrap_or(0.0);

    TaxaRecord {
        cleaned_text: cleaned,
        governed_actors: extracted.governed_labels(),
        government_actors: extracted.government_labels(),
        duty_types: cr.duty_types,
        popimar,
        purposes,
        classification: cr.classification,
        clause_refined,
        taxa_confidence,
    }
}

// ── Clause extraction ────────────────────────────────────────────────

/// Maximum length for a refined clause.
const MAX_CLAUSE_LEN: usize = 300;

/// How far before the actor to start the clause window.
const SUBJECT_WINDOW: usize = 100;

/// How far after the modal to extend the clause window.
const ACTION_WINDOW: usize = 200;

/// Sentence-end pattern for truncation.
static SENTENCE_END_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"[.;]").unwrap());

/// Extract a focused clause from the cleaned text using match span data.
///
/// If a `MatchSpan` is available (governed v2 patterns), extracts a window:
/// - Start: up to `SUBJECT_WINDOW` chars before actor, snapped to sentence boundary
/// - End: up to `ACTION_WINDOW` chars after modal end, snapped to sentence boundary
/// - Capped at `MAX_CLAUSE_LEN` total
///
/// If no span (government patterns or v1), falls back to `clause_refiner::refine()`.
fn extract_clause(
    cleaned_text: &str,
    classification: Option<&duty_patterns::DutyClassification>,
) -> Option<String> {
    let dc = classification?;

    if let Some(span) = dc.span {
        // Span-based extraction — we know exactly where actor and modal are.
        // Span offsets come from regex on lowercased text; snap to char
        // boundaries in the original cleaned_text (safe for rare multi-byte chars).
        let text_len = cleaned_text.len();
        let actor_start = snap_char_boundary_down(cleaned_text, span.actor_start);
        let modal_end = snap_char_boundary_down(cleaned_text, span.modal_end);

        // Start: SUBJECT_WINDOW before actor, snapped to sentence start
        let raw_start = actor_start.saturating_sub(SUBJECT_WINDOW);
        let raw_start = snap_char_boundary_down(cleaned_text, raw_start);
        let window_before = &cleaned_text[raw_start..actor_start];
        // Find last sentence boundary (. or ; followed by space+uppercase) in the window
        let start = if let Some(pos) = find_last_sentence_start(window_before) {
            raw_start + pos
        } else if raw_start == 0 {
            0
        } else {
            // Snap to next word boundary
            snap_to_word_start(cleaned_text, raw_start)
        };

        // End: ACTION_WINDOW after modal end, snapped to sentence end
        let raw_end =
            snap_char_boundary_down(cleaned_text, (modal_end + ACTION_WINDOW).min(text_len));
        let window_after = &cleaned_text[modal_end..raw_end];
        let end = if let Some(pos) = find_first_sentence_end(window_after) {
            // Include the punctuation
            (modal_end + pos + 1).min(text_len)
        } else {
            snap_to_word_end(cleaned_text, raw_end)
        };

        // Cap at MAX_CLAUSE_LEN
        let effective_end = if end - start > MAX_CLAUSE_LEN {
            let capped = start + MAX_CLAUSE_LEN;
            snap_to_word_end(cleaned_text, capped)
        } else {
            end
        };

        let clause = cleaned_text[start..effective_end].trim().to_string();
        if clause.is_empty() {
            return None;
        }

        // Add ellipsis if we truncated
        let clause = if start > 0 && !clause.starts_with(|c: char| c.is_uppercase()) {
            format!("...{clause}")
        } else {
            clause
        };
        let clause = if effective_end < text_len && !clause.ends_with(['.', ';', '!', '?']) {
            format!("{clause}...")
        } else {
            clause
        };

        Some(clause)
    } else {
        // No span — fall back to clause_refiner for government patterns
        let refined = clause_refiner::refine(cleaned_text, Some(cleaned_text), None);
        if refined.is_empty() {
            None
        } else {
            Some(refined)
        }
    }
}

/// Find the position of the last sentence start in `window` (after `. ` or `; ` + uppercase).
fn find_last_sentence_start(window: &str) -> Option<usize> {
    let mut last_pos = None;
    for (i, _) in window.match_indices(['.', ';']) {
        // Check if followed by whitespace + uppercase
        let rest = &window[i + 1..];
        let trimmed = rest.trim_start();
        if !trimmed.is_empty() && trimmed.starts_with(|c: char| c.is_uppercase()) {
            // Position of the uppercase char in the original window
            let ws_len = rest.len() - trimmed.len();
            last_pos = Some(i + 1 + ws_len);
        }
    }
    last_pos
}

/// Find the position of the first sentence-ending punctuation in `window`.
fn find_first_sentence_end(window: &str) -> Option<usize> {
    SENTENCE_END_RE.find(window).map(|m| m.start())
}

/// Snap a byte offset down to the nearest valid char boundary.
fn snap_char_boundary_down(text: &str, offset: usize) -> usize {
    let mut pos = offset.min(text.len());
    while pos > 0 && !text.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Snap a byte offset forward to the next word boundary (space).
fn snap_to_word_start(text: &str, offset: usize) -> usize {
    if offset >= text.len() {
        return text.len();
    }
    // Find next space after offset, then skip it
    text[offset..].find(' ').map_or(offset, |p| {
        let pos = offset + p + 1;
        pos.min(text.len())
    })
}

/// Snap a byte offset backward to the previous word boundary (space).
fn snap_to_word_end(text: &str, offset: usize) -> usize {
    if offset >= text.len() {
        return text.len();
    }
    // Find last space before offset
    text[..offset].rfind(' ').map_or(offset, |p| p)
}

// ── Miss analysis ────────────────────────────────────────────────────

use std::sync::LazyLock;

static MODAL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(?:shall|must|is required to|has a duty)\b").unwrap()
});

static ENABLING_MODAL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(?:may|power to|entitled to|authorise|authorize)\b").unwrap()
});

/// A provision that v2 did not classify, with diagnostic metadata.
#[derive(Debug, Clone)]
pub struct MissRecord {
    /// Cleaned text.
    pub cleaned_text: String,
    /// Heat score (higher = more likely to be a genuine missed duty).
    pub heat: i8,
    /// Breakdown of what contributed to the heat score.
    pub signals: Vec<&'static str>,
    /// Governed actors extracted by actors.rs.
    pub governed_actors: Vec<String>,
    /// Government actors extracted by actors.rs.
    pub government_actors: Vec<String>,
    /// Purposes detected.
    pub purposes: Vec<&'static str>,
    /// Whether a modal verb is present in the text.
    pub has_modal: bool,
    /// Whether an enabling modal is present.
    pub has_enabling: bool,
}

/// Analyse a provision that v2 did NOT classify — compute a heat score
/// indicating how likely it is to be a genuine missed duty.
///
/// Heat scoring:
/// - +3 obligation modal present (shall/must/is required to)
/// - +2 governed actor extracted
/// - +1 enabling modal present (may/power to/entitled to)
/// - +1 government actor extracted
/// - +1 operative purpose (Process+Rule)
/// - −2 structural purpose only (Interpretation/Amendment/Repeal)
/// - −1 very short text (< 50 chars — likely heading)
pub fn analyse_miss(raw_text: &str) -> MissRecord {
    let cleaned = text_cleaner::clean(raw_text);
    let purposes = purpose::classify(&cleaned);
    let extracted = actors::extract_actors(&cleaned);
    let lower = cleaned.to_lowercase();

    let has_modal = MODAL_RE.is_match(&lower);
    let has_enabling = ENABLING_MODAL_RE.is_match(&lower);

    let governed_labels = extracted.governed_labels();
    let government_labels = extracted.government_labels();

    let mut heat: i8 = 0;
    let mut signals: Vec<&'static str> = Vec::new();

    if has_modal {
        heat += 3;
        signals.push("modal");
    }

    if !governed_labels.is_empty() {
        heat += 2;
        signals.push("governed_actor");
    }

    if has_enabling {
        heat += 1;
        signals.push("enabling_modal");
    }

    if !government_labels.is_empty() {
        heat += 1;
        signals.push("government_actor");
    }

    if purposes.contains(&purpose::PROCESS_RULE) {
        heat += 1;
        signals.push("operative_purpose");
    }

    let is_structural = !purposes.is_empty()
        && purposes.iter().all(|p| {
            [
                purpose::INTERPRETATION,
                purpose::AMENDMENT,
                purpose::REPEAL_REVOCATION,
                purpose::TRANSITIONAL,
            ]
            .contains(p)
        });
    if is_structural {
        heat -= 2;
        signals.push("structural_purpose");
    }

    if cleaned.len() < 50 {
        heat -= 1;
        signals.push("short_text");
    }

    MissRecord {
        cleaned_text: cleaned,
        heat,
        signals,
        governed_actors: governed_labels,
        government_actors: government_labels,
        purposes,
        has_modal,
        has_enabling,
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

    // ── parse_v2 tests ────────────────────────────────────────────────

    #[test]
    fn parse_v2_employer_duty() {
        let record = parse_v2("The employer shall ensure the health and safety of employees.");
        assert!(record.duty_types.contains(&DutyType::Duty));
        assert!(
            record
                .governed_actors
                .iter()
                .any(|a| a.contains("Employer"))
        );
    }

    #[test]
    fn parse_v2_rejects_actor_as_object() {
        let record = parse_v2("information must be provided to the contractor before work begins");
        assert!(
            record.duty_types.is_empty(),
            "v2 should reject contractor-as-object, got: {:?}",
            record.duty_types
        );
    }

    // ── True-negative regression tests (Iteration 5: a person must) ───

    #[test]
    fn person_regarded_as_competent_no_drrp() {
        // MHSWR reg 7 — definitional: "a person shall be regarded as competent"
        let text = "A person shall be regarded as competent for the purposes of \
                    paragraphs (1) and (8) where he has sufficient training and \
                    experience or knowledge and other qualities to enable him \
                    properly to assist in undertaking the measures referred to \
                    in that paragraph.";
        let record = parse(text);
        assert!(
            record.duty_types.is_empty(),
            "definitional 'shall be regarded' should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn person_scope_exclusion_no_drrp() {
        // PUWER reg 3 — application/scope: "shall not apply to a person"
        let text = "The requirements imposed by these Regulations shall not apply \
                    to a person in respect of work equipment supplied by him by \
                    way of sale, agreement for sale or hire-purchase agreement.";
        let record = parse(text);
        assert!(
            record.duty_types.is_empty(),
            "scope exclusion mentioning 'a person' should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    // ── True-negative regression tests (Iteration 1: contractor) ─────
    // Full-pipeline tests: provisions mentioning "contractor" that should
    // NOT produce DRRP output. These guard against false positives when
    // expanding the governed actor patterns.

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

    // ── True-negative regression tests (Iteration 3: client) ─────────

    #[test]
    fn client_definition_no_drrp() {
        // CDM 2015 reg 4(8) — definitional, no modal
        let text = "Where there is more than one client in relation to a project, \
                    one or more of the clients may agree in writing to be treated \
                    for the purposes of these Regulations as the only client.";
        let record = parse(text);
        assert!(record.duty_types.is_empty());
    }

    // ── True-positive tests (Iteration 5: a person must) ──────────────

    #[test]
    fn person_must_not_ride_prohibition() {
        // CDM 2015 reg 28(1) — "a person must not ride"
        let text = "A person must not ride, or be required or permitted to ride, \
                    on any vehicle being used for the purposes of construction \
                    work unless that vehicle is suitable for carrying that person.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Duty),
            "person prohibition should classify as Duty, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn person_must_not_remain_prohibition() {
        // CDM 2015 reg 28(2) — "a person must not remain"
        let text = "A person must not remain, or be required or permitted to \
                    remain, on any vehicle during the loading or unloading of \
                    any loose material unless a safe place of work is provided \
                    and maintained for that person.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Duty),
            "person prohibition should classify as Duty, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn person_must_not_carry_out_work_prohibition() {
        // CDM 2015 reg 32(2) — "a person must not carry out work"
        let text = "Where a work activity may give rise to a particular risk of \
                    fire, a person must not carry out work unless suitably \
                    instructed.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Duty),
            "person fire prohibition should classify as Duty, got: {:?}",
            record.duty_types
        );
    }

    // ── True-positive tests (Iteration 3: client) ────────────────────

    #[test]
    fn client_duty_manage_project() {
        // CDM 2015 reg 4(1) — clear duty on client
        let text = "a client must make suitable arrangements for managing a \
                    project, including the allocation of sufficient time and \
                    other resources.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Duty),
            "client obligation should classify as Duty, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn client_duty_maintain_arrangements() {
        // CDM 2015 reg 4(3) — duty on client
        let text = "a client must ensure that these arrangements are maintained \
                    and reviewed throughout the project.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Duty),
            "client obligation should classify as Duty, got: {:?}",
            record.duty_types
        );
    }

    // ── Clause extraction tests ─────────────────────────────────────

    #[test]
    fn clause_refined_simple_employer_duty() {
        let record = parse_v2("The employer shall ensure the health and safety of employees.");
        let clause = record.clause_refined.unwrap();
        assert!(
            clause.contains("employer"),
            "clause should contain actor: {clause}"
        );
        assert!(
            clause.contains("shall"),
            "clause should contain modal: {clause}"
        );
        assert!(
            clause.contains("ensure"),
            "clause should contain action: {clause}"
        );
    }

    #[test]
    fn clause_refined_long_provision_is_truncated() {
        let text = "It shall be the duty of every employer to ensure, so far as \
                    is reasonably practicable, the health, safety and welfare at \
                    work of all his employees. The matters to which that duty \
                    extends include in particular the provision and maintenance \
                    of plant and systems of work that are, so far as is reasonably \
                    practicable, safe and without risks to health. The arrangement \
                    for ensuring, so far as is reasonably practicable, safety and \
                    absence of risks to health in connection with the use, handling, \
                    storage and transport of articles and substances.";
        let record = parse_v2(text);
        let clause = record.clause_refined.unwrap();
        assert!(
            clause.len() <= 320,
            "clause too long: {} chars",
            clause.len()
        ); // 300 + ellipsis
        assert!(
            clause.contains("employer"),
            "clause should contain actor: {clause}"
        );
    }

    #[test]
    fn clause_refined_person_compound() {
        let text = "A person must not ride, or be required or permitted to ride, \
                    on any vehicle being used for the purposes of construction work.";
        let record = parse_v2(text);
        let clause = record.clause_refined.unwrap();
        assert!(
            clause.contains("person"),
            "clause should contain actor: {clause}"
        );
        assert!(
            clause.contains("must not"),
            "clause should contain modal: {clause}"
        );
    }

    #[test]
    fn clause_refined_government_fallback() {
        let text = "The Secretary of State shall have power to make regulations.";
        let record = parse_v2(text);
        let clause = record.clause_refined.unwrap();
        assert!(
            clause.contains("Secretary"),
            "clause should contain actor: {clause}"
        );
        assert!(
            clause.contains("shall"),
            "clause should contain modal: {clause}"
        );
    }

    #[test]
    fn clause_refined_none_for_empty() {
        let record = parse_v2("");
        assert!(record.clause_refined.is_none());
    }

    #[test]
    fn clause_refined_none_for_no_drrp() {
        let record = parse_v2("the quick brown fox jumped over the lazy dog");
        assert!(record.clause_refined.is_none());
    }

    #[test]
    fn clause_refined_passive_by_pattern() {
        let text = "An internal emergency plan must be prepared by the operator \
                    before the establishment is put into operation.";
        let record = parse_v2(text);
        let clause = record.clause_refined.unwrap();
        assert!(
            clause.contains("operator"),
            "clause should contain actor: {clause}"
        );
        assert!(
            clause.contains("must"),
            "clause should contain modal: {clause}"
        );
    }

    #[test]
    fn clause_refined_duty_of_pattern() {
        let text = "It shall be the duty of every employer to ensure, so far as \
                    is reasonably practicable, the health, safety and welfare at \
                    work of all his employees.";
        let record = parse_v2(text);
        let clause = record.clause_refined.unwrap();
        assert!(
            clause.contains("employer"),
            "clause should contain actor: {clause}"
        );
        assert!(
            clause.contains("shall"),
            "clause should contain modal: {clause}"
        );
    }
}
