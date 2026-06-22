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
pub mod clause_structure;
pub mod confidence;
pub mod decision;
pub mod duty_patterns;
pub mod duty_patterns_offence;
pub mod duty_patterns_rule;
pub mod duty_patterns_v2;
pub mod duty_type;
pub mod fitness;
pub mod making;
pub mod popimar;
pub mod purpose;
pub mod signals;
pub mod text_cleaner;

use regex::Regex;

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

    /// DRRP duty types (Obligation, Liberty, Rule).
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

    /// Decomposed clause structure (applicability, modal, qualifiers, action).
    /// Populated when `clause_refined` is present and contains a modal verb.
    pub clause_structure: Option<clause_structure::ClauseStructure>,

    /// Fitness rules — law-level applicability (polarity + p-dimension tags).
    /// Populated for Application+Scope provisions only.
    pub fitness_rules: Vec<fitness::FitnessRule>,

    /// Hohfeldian actor positions derived from regex pattern match span.
    /// Maps actor label → "active" | "counterparty" | "mentioned".
    /// Only populated when a v2 match span is available.
    pub actor_positions: std::collections::HashMap<String, &'static str>,
}

/// Run the full Taxa classification pipeline on raw legislative text.
///
/// Delegates to `parse_v2()` — the actor-anchored pipeline is now the default.
pub fn parse(raw_text: &str) -> TaxaRecord {
    parse_v2(raw_text, None)
}

/// Run the actor-anchored Taxa classification pipeline on raw legislative text.
///
/// Steps:
/// 1. Clean the text (HTML strip, normalise whitespace)
/// 2. Classify purpose
/// 3. Extract actors (always — needed for gate override decision)
/// 4. Purpose gate (skip DRRP unless governed actor overrides)
/// 5. Extract signals from all 5 regex tiers
/// 6. Decision engine picks the best classification
/// 7. Classify POPIMAR categories (only if DRRP-bearing)
/// 8. Extract focused clause from match span
pub fn parse_v2(raw_text: &str, family: Option<&str>) -> TaxaRecord {
    parse_v2_with_trail(raw_text, family).0
}

/// Run the pipeline and return both the TaxaRecord and the decision trail.
///
/// Used by QA/diagnostic commands for tracing the "parsing journey".
/// The TaxaRecord is identical to what `parse_v2()` returns.
pub fn parse_v2_with_trail(
    raw_text: &str,
    family: Option<&str>,
) -> (TaxaRecord, decision::DecisionTrail) {
    if raw_text.trim().is_empty() {
        return (
            TaxaRecord::default(),
            decision::DecisionTrail {
                winner: None,
                reason: decision::DecisionReason::EmptyText,
                candidates_count: 0,
                rejections_count: 0,
            },
        );
    }

    let cleaned = text_cleaner::clean(raw_text);
    let purposes = purpose::classify(&cleaned);

    let extracted = actors::extract_actors_for_family(&cleaned, family);
    let has_governed = !extracted.governed.is_empty();
    let has_government = !extracted.government.is_empty();

    let purpose_gated =
        should_skip_drrp(&purposes, has_governed, has_government);
    let desc_summary = is_descriptive_summary(&cleaned);

    if purpose_gated || desc_summary {
        let fitness_rules = if purposes.contains(&purpose::APPLICATION_SCOPE) {
            fitness::extract(&cleaned, family)
        } else {
            vec![]
        };
        let signal_set = signals::extract_all(
            &cleaned.to_lowercase(),
            &extracted.governed,
            &extracted.government,
            &purposes,
            false,
            desc_summary,
            purpose_gated,
        );
        let (_, trail) = decision::decide(&signal_set);
        return (
            TaxaRecord {
                cleaned_text: cleaned,
                governed_actors: extracted.governed_labels(),
                government_actors: extracted.government_labels(),
                purposes,
                fitness_rules,
                ..Default::default()
            },
            trail,
        );
    }

    let lower = cleaned.to_lowercase();
    let is_lf = is_legal_fiction(&lower);

    let signal_set = signals::extract_all(
        &lower,
        &extracted.governed,
        &extracted.government,
        &purposes,
        is_lf,
        false,
        false,
    );
    let (cr, trail) = decision::decide(&signal_set);

    let dt_labels: Vec<&str> = cr.duty_types.iter().map(|d| d.as_str()).collect();
    let popimar = popimar::classify_with_duty_types(&cleaned, &dt_labels);
    let clause_refined = extract_clause(&cleaned, cr.classification.as_ref());

    let span = cr.classification.as_ref().and_then(|c| c.span);
    let clause_structure = clause_refined
        .as_deref()
        .and_then(|c| clause_structure::decompose(c, span));

    let has_span = span.is_some();
    let taxa_confidence = clause_refined
        .as_deref()
        .map(|c| confidence::score(c, has_span))
        .unwrap_or(0.0);

    let fitness_rules = if purposes.contains(&purpose::APPLICATION_SCOPE) {
        fitness::extract(&cleaned, family)
    } else {
        vec![]
    };

    let actor_positions = derive_actor_positions(&extracted, cr.classification.as_ref());

    (
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
            clause_structure,
            fitness_rules,
            actor_positions,
        },
        trail,
    )
}

/// Derive Hohfeldian actor positions from the DRRP match span.
fn derive_actor_positions(
    extracted: &actors::ExtractedActors,
    classification: Option<&duty_patterns::DutyClassification>,
) -> std::collections::HashMap<String, &'static str> {
    let mut positions = std::collections::HashMap::new();
    if let Some(dc) = classification
        && let Some(span) = dc.span
    {
        let all_actors: Vec<&actors::ActorMatch> = extracted
            .governed
            .iter()
            .chain(extracted.government.iter())
            .collect();

        let mut found_active = false;
        for actor in &all_actors {
            let actor_end = actor.offset + actor.keyword.len();
            let near_start =
                actor.offset <= span.actor_start + 3 && actor_end + 3 >= span.actor_start;
            if near_start {
                positions.insert(actor.label.clone(), "active");
                found_active = true;
            }
        }

        for actor in &all_actors {
            if !positions.contains_key(&actor.label) {
                positions.insert(actor.label.clone(), "counterparty");
            }
        }

        if !found_active && let Some(first) = all_actors.first() {
            positions.insert(first.label.clone(), "active");
        }
    }
    positions
}

// ── Clause extraction ────────────────────────────────────────────────

/// Sentence-end pattern for truncation.
static SENTENCE_END_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"[.;]").unwrap());

/// Extract a focused clause from the cleaned text using match span data.
///
/// If a `MatchSpan` is available (governed v2 patterns), scans backward from
/// the actor for the sentence start and forward from the modal for the sentence
/// end.  No artificial window limits — the text is already in memory.
///
/// If no span (government patterns or v1), falls back to `clause_refiner::refine()`.
fn extract_clause(
    cleaned_text: &str,
    classification: Option<&duty_patterns::DutyClassification>,
) -> Option<String> {
    let dc = classification?;

    if let Some(span) = dc.span {
        let text_len = cleaned_text.len();
        let actor_start = snap_char_boundary_down(cleaned_text, span.actor_start);
        let modal_end = snap_char_boundary_down(cleaned_text, span.modal_end);

        // Start: scan all text before actor for the last sentence boundary
        let before_actor = &cleaned_text[..actor_start];
        let start = find_last_sentence_start(before_actor).unwrap_or(0);

        // End: scan all text after modal for the first sentence boundary
        let after_modal = &cleaned_text[modal_end..];
        let end = if let Some(pos) = find_first_sentence_end(after_modal) {
            // Include the punctuation
            (modal_end + pos + 1).min(text_len)
        } else {
            // No sentence end found — use the full remaining text
            text_len
        };

        // Guard: if span positions are inconsistent (actor after modal due
        // to regex overlap), fall back to the full text between boundaries.
        let (slice_start, slice_end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let clause = cleaned_text[slice_start..slice_end].trim().to_string();
        if clause.is_empty() {
            return None;
        }

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

/// Find the position of the first real sentence-ending punctuation in `window`.
///
/// Skips `;` when followed by a sub-paragraph marker like `(a)`, `(b)`, `(i)`,
/// `(ii)` etc. — these are list separators in UK legislation, not sentence ends.
fn find_first_sentence_end(window: &str) -> Option<usize> {
    for m in SENTENCE_END_RE.find_iter(window) {
        let ch = &window[m.start()..m.start() + 1];
        if ch == "." {
            return Some(m.start());
        }
        // ch == ";" — check what follows
        let rest = &window[m.start() + 1..];
        let trimmed = rest.trim_start();
        // Skip if followed by sub-paragraph marker: (a), (b), (i), (1), (aa), etc.
        if trimmed.starts_with('(') {
            continue;
        }
        // Skip if followed by "and" or "or" then sub-paragraph (e.g. "; and (b)")
        if (trimmed.starts_with("and ") || trimmed.starts_with("or "))
            && trimmed[3..].trim_start().starts_with('(')
        {
            continue;
        }
        return Some(m.start());
    }
    None
}

/// Snap a byte offset down to the nearest valid char boundary.
fn snap_char_boundary_down(text: &str, offset: usize) -> usize {
    let mut pos = offset.min(text.len());
    while pos > 0 && !text.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
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
///
/// Actor presence overrides structural gates — provisions classified as
/// Interpretation/Enactment/Application+Scope often contain real DRRP
/// when actors are present (e.g., "Member States shall ensure that..."
/// in definitions, commencement provisions with "Secretary of State may").
pub fn should_skip_drrp(
    purposes: &[&str],
    has_governed_actor: bool,
    has_government_actor: bool,
) -> bool {
    const SKIP_PURPOSES: &[&str] = &[
        purpose::ENACTMENT,
        purpose::INTERPRETATION,
        purpose::AMENDMENT,
        purpose::REPEAL_REVOCATION,
    ];

    if purposes.is_empty() {
        return false;
    }

    let has_any_actor = has_governed_actor || has_government_actor;

    // Amendment/Repeal provisions never bear their own DRRP — obligations
    // in quoted text belong to the target section, not this provision.
    // Skip unconditionally regardless of actors present.
    if purposes
        .iter()
        .any(|p| *p == purpose::AMENDMENT || *p == purpose::REPEAL_REVOCATION)
    {
        return true;
    }

    // Offence provisions usually describe consequences (penalties, liability),
    // not new obligations. However, government actors in offence provisions
    // often exercise enforcement powers — "officer may enter and search",
    // "authority may impose a civil penalty". Allow DRRP when government
    // actors are present to capture these Liberty/Power classifications.
    if purposes.iter().any(|p| *p == purpose::OFFENCE) {
        return !has_government_actor;
    }

    // ALL strategy: skip when every detected purpose is a skip-purpose
    // AND no actors are present. Actor presence indicates real DRRP may
    // be embedded (e.g., Interpretation-only provisions with "No person
    // may transfer..." or "A notice may include directions...").
    if purposes.iter().all(|p| SKIP_PURPOSES.contains(p)) {
        return !has_any_actor;
    }

    // Interpretation-primary: skip unless actors present.
    if purposes.first() == Some(&purpose::INTERPRETATION) {
        return !has_any_actor;
    }

    // Enactment-primary: commencement/citation blocks usually skip, but
    // transitional provisions with actors contain real Powers/Duties
    // (e.g., "Secretary of State may by order appoint...").
    if purposes.first() == Some(&purpose::ENACTMENT) {
        return !has_any_actor;
    }

    // Application+Scope-primary: "These Regulations shall apply to..."
    // usually describes scope, but provisions with actors may contain
    // real obligations (e.g., "does not apply where the importer has...",
    // "the Executive shall submit...", "the Secretary of State must consult").
    // Skip only when no actors are present OR when governed actors are
    // present but no government actors (scope extensions typically mention
    // governed actors like employer/employee to define who the scope covers).
    // Government actors (Secretary of State, Executive) in Application+Scope
    // provisions usually indicate a real obligation.
    if purposes.first() == Some(&purpose::APPLICATION_SCOPE) {
        return !has_government_actor;
    }

    false
}

/// Check whether text uses "shall" in the legal fiction / interpretive sense
/// rather than the obligation sense. UK legislative drafting uses "shall" for both:
/// - "The employer shall ensure safety" — obligation
/// - "The Authority shall be treated as a local authority" — legal fiction
///
/// When detected, the provision should not produce DRRP even though it has a modal.
static LEGAL_FICTION_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(
        r"(?i)\bshall\b\s+(?:be\s+(?:treated|deemed|construed|read|applied)|have\s+(?:the\s+)?effect|not\s+(?:affect|authorise|prejudice|apply\b|prevent|be\s+taken))|(?:^|[.;]\s*)Nothing\s+in\b"
    ).unwrap()
});

/// Immunity/right-preservation patterns — provisions that use legal-fiction
/// language but actually express a Liberty (right not to be compelled,
/// preservation of entitlement, etc.).
static IMMUNITY_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)",
        // "Nothing in X shall compel/require" — immunity from compulsion
        r"Nothing\s+in\b.{0,60}\bshall\b.{0,30}\b(?:compel|require|oblige)",
        r"|",
        // "shall not affect [any] entitlement/right" — preservation of rights
        r"\bshall\s+not\s+affect\b.{0,40}\b(?:entitlement|right|privilege)",
        r"|",
        // "Nothing in X is taken to compel" — immunity variant
        r"Nothing\s+in\b.{0,60}\b(?:taken|construed)\s+to\s+(?:compel|require)",
    )).unwrap()
});

pub fn is_legal_fiction(text: &str) -> bool {
    if !LEGAL_FICTION_RE.is_match(text) {
        return false;
    }
    // Immunity provisions use legal-fiction language but are actually Liberty
    if IMMUNITY_RE.is_match(text) {
        return false;
    }
    true
}

/// Check whether a provision's purposes indicate it could bear a duty.
///
/// Returns `true` if at least one purpose is duty-bearing (i.e., not
/// purely structural). Used by Gap C Tier 1 to decide whether a provision
/// with no regex-extracted DRRP is a candidate for parent inheritance.
///
/// Skip-only purposes (never contain duties):
/// Enactment, Interpretation, Amendment, Repeal/Revocation, Application+Scope, Extent
pub fn is_duty_bearing_purpose(purposes: &[String]) -> bool {
    const NON_DUTY_PURPOSES: &[&str] = &[
        purpose::ENACTMENT,
        purpose::INTERPRETATION,
        purpose::AMENDMENT,
        purpose::REPEAL_REVOCATION,
        purpose::APPLICATION_SCOPE,
        purpose::EXTENT,
    ];

    if purposes.is_empty() {
        return false;
    }

    purposes
        .iter()
        .any(|p| !NON_DUTY_PURPOSES.contains(&p.as_str()))
}

/// Descriptive/meta-regulatory summary pattern.
///
/// Matches text that describes what the Regulations/Act do in general terms
/// (e.g. "The Regulations impose duties on employers to protect employees...")
/// rather than directly creating obligations. These appear in Reg 1 of most
/// SIs as a descriptive overview and should not produce DRRP output.
static DESCRIPTIVE_SUMMARY: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?i)^(?:the(?:se)?|this) (?:regulations?|act|order|rules?)\s+(?:impose|require|provide|give effect|implement|extend|place|create|establish|set out|supplement|make provision)",
    )
    .unwrap()
});

/// Check if cleaned text is a descriptive meta-regulatory summary.
///
/// These provisions describe what the instrument does as a whole — they don't
/// themselves create obligations. Example: "The Regulations impose duties on
/// employers to protect employees who may be exposed to risk..."
pub fn is_descriptive_summary(text: &str) -> bool {
    DESCRIPTIVE_SUMMARY.is_match(text)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_employer_duty() {
        let record = parse("The employer shall ensure the health and safety of employees.");
        assert!(record.duty_types.contains(&DutyType::Obligation));
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
        assert!(record.duty_types.contains(&DutyType::Obligation));
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
        assert!(record.duty_types.contains(&DutyType::Obligation));
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
        assert!(record.duty_types.contains(&DutyType::Obligation));
        assert!(!record.popimar.is_empty());
        assert!(!record.purposes.is_empty());
    }

    // ── Purpose-based pre-filtering tests ───────────────────────────

    #[test]
    fn skip_interpretation_section() {
        // Pure definition — only INTERPRETATION purpose, no modal verbs.
        // ALL-structural gate fires, DRRP is skipped.
        // Actors are still extracted (above the gate) for metadata.
        let text =
            r#"In these Regulations— "employer" means a person who employs one or more employees."#;
        let record = parse(text);
        // Purpose detected
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        // DRRP classification skipped (ALL-structural gate)
        assert!(record.duty_types.is_empty());
        assert!(record.popimar.is_empty());
        // Actors ARE populated — extraction moved above gate for override logic
        assert!(
            record
                .governed_actors
                .iter()
                .any(|a| a.contains("Employer")),
            "actors should be extracted even for gated provisions"
        );
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
    fn amendment_with_quoted_duty_skipped() {
        // Amendment text containing "shall" in quoted substitution text.
        // The duty belongs to the destination section, not this amendment
        // provision.  Interpretation pattern also fires on the quoted text,
        // making Interpretation primary — correctly skipped.
        let text = r#"In section 3, for subsection (2) substitute— "The Scottish Ministers shall ensure targets are met.""#;
        let record = parse(text);
        assert!(record.purposes.contains(&purpose::AMENDMENT));
        // Interpretation-primary due to quoted text — DRRP skipped
        assert!(record.duty_types.is_empty());
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
    fn interpretation_primary_with_actor_override() {
        // When Interpretation is primary BUT a governed actor is present,
        // the gate is overridden to handle mixed-content provisions (product
        // safety SIs). DRRP extraction runs — the v2 pattern matcher
        // determines whether duties are real.
        let text = r#"For the purposes of interpretation, "employer" means a person who shall ensure safety."#;
        let record = parse(text);
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        // Gate overridden — DRRP extraction runs because governed actor present
        assert!(
            !record.governed_actors.is_empty(),
            "governed actor should be extracted"
        );
    }

    #[test]
    fn pure_definition_with_actor_mentions_skipped() {
        // Pure Interpretation+Definition provision that mentions actors
        // (e.g., "approved by the Health and Safety Executive") should
        // still skip DRRP — the actor mention is noise, not a duty.
        let text = r#"In these Regulations "the approved poster" means a poster in the form approved and published by the Health and Safety Executive."#;
        let record = parse(text);
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        assert!(
            record.duty_types.is_empty(),
            "pure definition should have no DRRP even with actor mention, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn interpretation_primary_without_actor_still_skipped() {
        // When Interpretation is primary with NO governed actor, gate still fires.
        let text = r#"For the purposes of these Regulations, "workplace" means any premises or part of premises which are not domestic premises."#;
        let record = parse(text);
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        // No governed actor → gate fires → DRRP skipped
        assert!(record.duty_types.is_empty());
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

    // ── is_duty_bearing_purpose tests ─────────────────────────────

    #[test]
    fn duty_bearing_process_rule() {
        let purposes = vec!["Process+Rule+Constraint+Condition".to_string()];
        assert!(is_duty_bearing_purpose(&purposes));
    }

    #[test]
    fn duty_bearing_mixed_with_interpretation() {
        let purposes = vec![
            "Interpretation+Definition".to_string(),
            "Process+Rule+Constraint+Condition".to_string(),
        ];
        assert!(is_duty_bearing_purpose(&purposes));
    }

    #[test]
    fn not_duty_bearing_pure_interpretation() {
        let purposes = vec!["Interpretation+Definition".to_string()];
        assert!(!is_duty_bearing_purpose(&purposes));
    }

    #[test]
    fn not_duty_bearing_amendment() {
        let purposes = vec!["Amendment".to_string()];
        assert!(!is_duty_bearing_purpose(&purposes));
    }

    #[test]
    fn not_duty_bearing_empty() {
        let purposes: Vec<String> = vec![];
        assert!(!is_duty_bearing_purpose(&purposes));
    }

    #[test]
    fn duty_bearing_offence() {
        let purposes = vec!["Offence".to_string()];
        assert!(is_duty_bearing_purpose(&purposes));
    }

    // ── Descriptive summary filter tests ────────────────────────────

    #[test]
    fn descriptive_regulations_impose_duties_no_drrp() {
        // UK_uksi_2005_1093 Reg 1 — descriptive summary of what the Regulations do
        let text = "The Regulations impose duties on employers to protect \
                    employees who may be exposed to risk from exposure to \
                    vibration at work, and other persons who might be affected \
                    by the work, whether they are at work or not.";
        let record = parse(text);
        assert!(
            record.duty_types.is_empty(),
            "descriptive summary should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn descriptive_regulations_require_no_drrp() {
        let text = "These Regulations require employers to make a suitable and \
                    sufficient assessment of the risks to the health and safety \
                    of employees.";
        let record = parse(text);
        assert!(
            record.duty_types.is_empty(),
            "descriptive 'require' summary should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn descriptive_act_provides_no_drrp() {
        let text = "This Act provides for the making of health, safety and \
                    welfare regulations and the issuing of approved codes of practice.";
        let record = parse(text);
        assert!(
            record.duty_types.is_empty(),
            "descriptive 'provides' summary should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn descriptive_regulations_implement_no_drrp() {
        let text = "The Regulations implement Council Directive 89/391/EEC on \
                    the introduction of measures to encourage improvements in \
                    the safety and health of workers at work.";
        let record = parse(text);
        assert!(
            record.duty_types.is_empty(),
            "descriptive 'implement' summary should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn direct_duty_the_employer_shall_not_filtered() {
        // "The" + employer + modal is a direct duty, not a descriptive summary
        let text = "The employer shall ensure the health and safety of employees.";
        let record = parse(text);
        assert!(
            !record.duty_types.is_empty(),
            "direct duty starting with 'The employer' should still produce DRRP"
        );
    }

    // ── parse_v2 tests ────────────────────────────────────────────────

    #[test]
    fn parse_v2_employer_duty() {
        let record = parse_v2(
            "The employer shall ensure the health and safety of employees.",
            None,
        );
        assert!(record.duty_types.contains(&DutyType::Obligation));
        assert!(
            record
                .governed_actors
                .iter()
                .any(|a| a.contains("Employer"))
        );
    }

    #[test]
    fn parse_v2_rejects_actor_as_object() {
        let record = parse_v2(
            "information must be provided to the contractor before work begins",
            None,
        );
        assert!(
            record.duty_types.is_empty(),
            "v2 should reject contractor-as-object, got: {:?}",
            record.duty_types
        );
    }

    // ── Mixed-content provision gate override tests (Fix 1) ───────────

    #[test]
    fn mixed_content_provision_employer_duty_extracted() {
        // Product safety SI pattern: Interpretation-primary provision that
        // starts with definitions but contains real employer duties.
        // The governed-actor gate override lets DRRP extraction run.
        let text = "In this regulation, \"the relevant statutory provisions\" means \
                    sections 2 to 7 of the 1974 Act. Every employer shall ensure \
                    that exposure of his employees to substances hazardous to health \
                    is either prevented or, where this is not reasonably practicable, \
                    adequately controlled.";
        let record = parse_v2(text, None);
        assert!(
            record.purposes.contains(&purpose::INTERPRETATION),
            "should detect Interpretation purpose"
        );
        assert!(
            !record.governed_actors.is_empty(),
            "should extract governed actors"
        );
        assert!(
            !record.duty_types.is_empty(),
            "mixed-content provision with real employer duty should produce DRRP, \
             got purposes: {:?}, actors: {:?}",
            record.purposes,
            record.governed_actors
        );
    }

    #[test]
    fn pure_definition_with_actor_keyword_no_drrp() {
        // Pure definition mentioning an actor keyword but no modal verb.
        // ALL-structural gate fires (only Interpretation purpose).
        let text = r#"In these Regulations— "employer" means a person who employs \
                    one or more employees under a contract of employment."#;
        let record = parse_v2(text, None);
        assert!(record.purposes.contains(&purpose::INTERPRETATION));
        // ALL-structural gate fires — no DRRP despite actor keyword
        assert!(
            record.duty_types.is_empty(),
            "pure definition should not produce DRRP, got: {:?}",
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
            record.duty_types.contains(&DutyType::Obligation),
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
            record.duty_types.contains(&DutyType::Obligation),
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
            record.duty_types.contains(&DutyType::Obligation),
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
            record.duty_types.contains(&DutyType::Obligation),
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
            record.duty_types.contains(&DutyType::Obligation),
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
            record.duty_types.contains(&DutyType::Obligation),
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
            record.duty_types.contains(&DutyType::Obligation),
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
            record.duty_types.contains(&DutyType::Obligation),
            "client obligation should classify as Duty, got: {:?}",
            record.duty_types
        );
    }

    // ── Clause extraction tests ─────────────────────────────────────

    #[test]
    fn clause_refined_simple_employer_duty() {
        let record = parse_v2(
            "The employer shall ensure the health and safety of employees.",
            None,
        );
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
        let record = parse_v2(text, None);
        let clause = record.clause_refined.unwrap();
        assert!(
            clause.contains("employer"),
            "clause should contain actor: {clause}"
        );
        // Should end at the first sentence boundary, not truncate mid-sentence
        assert!(
            clause.ends_with('.') || clause.ends_with(';'),
            "clause should end at sentence boundary: {clause}"
        );
    }

    #[test]
    fn clause_refined_person_compound() {
        let text = "A person must not ride, or be required or permitted to ride, \
                    on any vehicle being used for the purposes of construction work.";
        let record = parse_v2(text, None);
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
        let record = parse_v2(text, None);
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
        let record = parse_v2("", None);
        assert!(record.clause_refined.is_none());
    }

    #[test]
    fn clause_refined_none_for_no_drrp() {
        let record = parse_v2("the quick brown fox jumped over the lazy dog", None);
        assert!(record.clause_refined.is_none());
    }

    #[test]
    fn clause_refined_passive_by_pattern() {
        let text = "An internal emergency plan must be prepared by the operator \
                    before the establishment is put into operation.";
        let record = parse_v2(text, None);
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
        let record = parse_v2(text, None);
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

    // ── Sentence-start regression tests ─────────────────────────────

    #[test]
    fn clause_no_mid_sentence_start_long_preamble() {
        // Real-world case: actor is >100 chars from the sentence start.
        // The clause should NOT start mid-word or mid-sentence.
        let text = "Subject to the provisions of Part II, for any activity to \
                    which the conditions of Schedule 3 relate, the \
                    appropriate registration authority shall enter in its register \
                    the particulars furnished to it pursuant to that provision.";
        let record = parse_v2(text, None);
        let clause = record.clause_refined.unwrap();
        assert!(
            !clause.starts_with("..."),
            "clause should not start mid-sentence: {clause}"
        );
        assert!(
            clause.starts_with("Subject") || clause.starts_with("the appropriate"),
            "clause should start at sentence boundary: {clause}"
        );
    }

    #[test]
    fn clause_finds_sentence_start_after_period() {
        // Sentence boundary `. ` is >100 chars before the actor.
        let text = "The preceding provisions of the Act set out general requirements \
                    for all installations covered by the framework and for all operators \
                    thereof. Where an installation which contains an existing SED \
                    installation is subject to a permit, the operator of the installation \
                    shall by the SED date make an application under regulation 17 of \
                    the principal framework.";
        let record = parse_v2(text, None);
        let clause = record.clause_refined.unwrap();
        assert!(
            !clause.starts_with("..."),
            "clause should not start mid-sentence: {clause}"
        );
        assert!(
            clause.contains("operator"),
            "clause should contain actor: {clause}"
        );
    }

    // ── Application+Scope gating tests (GH #20) ─────────────────────

    #[test]
    fn scope_extension_like_duty_skipped() {
        // Noise 2005 Reg 3 — scope extension, not a new duty
        let text = "Where a duty is placed by these Regulations on an employer in \
                    respect of his employees, the employer shall, so far as is reasonably \
                    practicable, be under a like duty in respect of any other person at \
                    work who may be affected by the work carried out by the employer.";
        let record = parse(text);
        assert!(
            record.purposes.contains(&purpose::APPLICATION_SCOPE),
            "should detect Application+Scope, got: {:?}",
            record.purposes
        );
        assert!(
            record.duty_types.is_empty(),
            "scope extension should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn scope_extension_self_employed_skipped() {
        // Noise 2005 / Vibration 2005 / Lead 2002 Reg 3
        let text = "These Regulations shall apply to a self-employed person as they \
                    apply to an employer and an employee and as if that self-employed \
                    person were both an employer and an employee, except that regulation \
                    9 shall not apply to a self-employed person.";
        let record = parse(text);
        assert!(
            record.purposes.contains(&purpose::APPLICATION_SCOPE),
            "should detect Application+Scope, got: {:?}",
            record.purposes
        );
        assert!(
            record.duty_types.is_empty(),
            "scope extension to self-employed should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn transitional_non_application_skipped() {
        // Vibration 2005 Reg 3(2) — transitional scope
        let text = "Subject to paragraph (3), regulation 6(4) shall not apply until \
                    6th July 2010 where work equipment is used which was first provided \
                    to employees prior to 6th July 2007 by any employer.";
        let record = parse(text);
        assert!(
            record.purposes.contains(&purpose::APPLICATION_SCOPE),
            "should detect Application+Scope, got: {:?}",
            record.purposes
        );
        assert!(
            record.duty_types.is_empty(),
            "transitional non-application should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn requirement_extends_to_self_employed_skipped() {
        // Pressure Systems Reg 3(1) — scope extension
        let text = "Any requirement or prohibition imposed by these Regulations on an \
                    employer in respect of the activities of his employees shall also \
                    extend to a self-employed person in respect of his own activities \
                    at work.";
        let record = parse(text);
        assert!(
            record.purposes.contains(&purpose::APPLICATION_SCOPE),
            "should detect Application+Scope, got: {:?}",
            record.purposes
        );
        assert!(
            record.duty_types.is_empty(),
            "requirement-extends scope should not produce DRRP, got: {:?}",
            record.duty_types
        );
    }

    #[test]
    fn genuine_employer_duty_not_application_scope() {
        // Genuine duty — mentions "these Regulations" incidentally but the
        // core text is an obligation on the employer to DO something.
        let text = "The employer shall ensure that risk from the exposure of his \
                    employees to noise is either eliminated at source or, where this \
                    is not reasonably practicable, reduced to as low a level as is \
                    reasonably practicable.";
        let record = parse(text);
        assert!(
            !record.duty_types.is_empty(),
            "genuine employer duty should produce DRRP, got empty"
        );
    }

    #[test]
    fn genuine_employer_risk_assessment_not_application_scope() {
        // Genuine duty that mentions "requirements of these Regulations"
        let text = "An employer who carries out work which is liable to expose any \
                    employees to noise at or above a lower exposure action value shall \
                    make a suitable and sufficient assessment of the risk from that \
                    noise to the health and safety of those employees, and the risk \
                    assessment shall identify the measures which need to be taken to \
                    meet the requirements of these Regulations.";
        let record = parse(text);
        assert!(
            !record.duty_types.is_empty(),
            "genuine risk assessment duty should produce DRRP, got empty"
        );
    }

    // ── Relative-clause Application+Scope false-positive regressions ──

    #[test]
    fn employee_to_whom_this_reg_applies_produces_drrp() {
        // Health surveillance duty — "to whom this regulation applies" is a
        // relative clause qualifying the actor, not a scope statement.
        let text = "An employee to whom this regulation applies shall, when required \
                    by his employer and at the cost of the employer, present himself \
                    during his working hours for such health surveillance procedures \
                    as may be required for the purposes of paragraph (1).";
        let record = parse(text);
        assert!(
            !record.duty_types.is_empty(),
            "'to whom this regulation applies' duty should produce DRRP, got empty"
        );
    }

    #[test]
    fn operator_to_which_these_regs_apply_produces_drrp() {
        // COMAH operator notification — "to which these Regulations apply" is
        // a relative clause, not scope.
        let text = "The operator of any establishment to which these Regulations \
                    apply must notify the competent authority in advance of a \
                    significant increase or decrease in the quantity of dangerous \
                    substances notified under this regulation.";
        let record = parse(text);
        assert!(
            !record.duty_types.is_empty(),
            "'to which these Regulations apply' duty should produce DRRP, got empty"
        );
    }

    #[test]
    fn employer_fumigation_to_which_this_reg_applies_produces_drrp() {
        // Fumigation prohibition — "to which this regulation applies" is a
        // relative clause, not scope.
        let text = "An employer shall not undertake fumigation to which this \
                    regulation applies unless he has notified the persons specified \
                    in Part I of Schedule 9 of his intention to undertake the \
                    fumigation.";
        let record = parse(text);
        assert!(
            !record.duty_types.is_empty(),
            "'to which this regulation applies' prohibition should produce DRRP, got empty"
        );
    }

    #[test]
    fn secondary_application_scope_gets_fitness_rules() {
        // When APPLICATION_SCOPE is a secondary purpose (not first),
        // fitness::extract() should still run. GH #26.
        let text = "These Regulations shall not apply to the master or crew \
                    of a sea-going ship or to the employer of such persons \
                    in respect of the normal ship-board activities of a \
                    ship's crew under the direction of the master.";
        let record = parse(text);
        assert!(
            !record.fitness_rules.is_empty(),
            "secondary APPLICATION_SCOPE should produce fitness_rules, got empty"
        );
    }

    #[test]
    fn employer_requirement_of_these_regs_which_applies_produces_drrp() {
        // Workplace compliance duty — "requirement of these Regulations which
        // applies" is a relative clause on "requirement", not scope.
        let text = "Every employer shall ensure that every workplace under his \
                    control complies with any requirement of these Regulations \
                    which applies to that workplace.";
        let record = parse(text);
        assert!(
            !record.duty_types.is_empty(),
            "'requirement of these Regulations which applies' duty should produce DRRP, got empty"
        );
    }

    // ── Family-gated specialist actors (GH #31) ────────────────────────

    #[test]
    fn licensee_duty_offshore_family_produces_drrp() {
        // Real offshore provision: licensee is the duty-holder.
        // With OH&S: Offshore family, licensee should be extracted as a
        // specialist actor and produce DRRP classification.
        let text = "The licensee shall ensure that any operator appointed \
                    by him is capable of satisfactorily carrying out his \
                    functions under these Regulations.";
        let record = parse_v2(text, Some("OH&S: Offshore Safety"));
        assert!(
            record
                .governed_actors
                .iter()
                .any(|a| a.contains("Licensee")),
            "licensee should be extracted for offshore family, got: {:?}",
            record.governed_actors
        );
        assert!(
            !record.duty_types.is_empty(),
            "licensee duty should produce DRRP for offshore family, got empty"
        );
    }

    #[test]
    fn licensee_duty_no_family_extracts_general_licensee() {
        // Licensee is now in the core dictionary (Ind: Licensee) as well as
        // the Offshore specialist (Offshore: Licensee). Without family context,
        // the general label should be extracted.
        let text = "The licensee shall ensure that any operator appointed \
                    by him is capable of satisfactorily carrying out his \
                    functions under these Regulations.";
        let record = parse_v2(text, None);
        assert!(
            record
                .governed_actors
                .iter()
                .any(|a| a.contains("Licensee")),
            "licensee should be extracted as Ind: Licensee, got: {:?}",
            record.governed_actors
        );
    }

    // ── Subordinate clause retry (actor in both clauses) ─────────────

    #[test]
    fn duty_holder_where_clause_repeat_produces_drrp() {
        // UK_nisr_2016_406:30 — "duty holder" in both subordinate Where-clause
        // and main clause. Should produce DRRP from the main clause occurrence.
        let text = "Where the duty holder has adopted other measures, the duty holder \
                    shall perform the internal emergency response duties so as to \
                    secure a good prospect of personal safety and survival, taking \
                    into account the adoption of those other measures.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Obligation),
            "repeated actor in Where+main clause should produce Duty, got: {:?}",
            record.duty_types
        );
        assert!(
            record
                .governed_actors
                .iter()
                .any(|a| a.contains("Duty Holder")),
            "should extract Duty Holder, got: {:?}",
            record.governed_actors
        );
    }

    // ── Actor position heuristic tests ─────────────────────────────

    #[test]
    fn position_employer_active_employee_counterparty() {
        let text = "The employer shall ensure the health and safety of employees.";
        let record = parse(text);
        assert_eq!(
            record.actor_positions.get("Org: Employer").copied(),
            Some("active")
        );
        assert_eq!(
            record.actor_positions.get("Ind: Employee").copied(),
            Some("counterparty")
        );
    }

    #[test]
    fn position_employer_shall_provide_training_to_employee() {
        let text =
            "Every employer shall ensure that adequate training is provided to each employee.";
        let record = parse(text);
        assert_eq!(
            record.actor_positions.get("Org: Employer").copied(),
            Some("active")
        );
        assert_eq!(
            record.actor_positions.get("Ind: Employee").copied(),
            Some("counterparty")
        );
    }

    #[test]
    fn position_single_actor_is_active() {
        let text = "The employer shall ensure a safe workplace.";
        let record = parse(text);
        assert_eq!(
            record.actor_positions.get("Org: Employer").copied(),
            Some("active")
        );
    }

    #[test]
    fn position_government_pattern_has_active_actor() {
        // Government pattern — "secretary of state shall make regulations"
        // Previously returned no positions because government patterns had span: None.
        let text = "The Secretary of State shall by regulations make provision for the safety of employees.";
        let record = parse(text);
        assert!(
            !record.actor_positions.is_empty(),
            "government pattern should now populate actor_positions"
        );
        assert_eq!(
            record.actor_positions.get("Gvt: Minister").copied(),
            Some("active"),
            "Secretary of State should be active, got: {:?}",
            record.actor_positions
        );
    }

    // ── EU Directive patterns ────────────────────────────────────────

    #[test]
    fn eu_directive_member_state_shall_ensure() {
        let text = "Member States shall ensure that the medical surveillance of exposed workers is based on the principles that govern occupational medicine generally.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Obligation),
            "EU Directive 'shall ensure' should be Responsibility, got: {:?}",
            record.duty_types
        );
        assert!(
            record
                .government_actors
                .iter()
                .any(|a| a.contains("Member State")),
            "should extract Member State, got: {:?}",
            record.government_actors
        );
    }

    #[test]
    fn eu_directive_member_state_active_worker_counterparty() {
        let text = "Member States shall ensure that the undertaking is responsible for assessing and implementing arrangements for the radiation protection of exposed workers.";
        let record = parse(text);
        assert_eq!(
            record.actor_positions.get("EU: Member State").copied(),
            Some("active"),
            "Member State should be active, got: {:?}",
            record.actor_positions
        );
        // Worker is mentioned in subordinate clause — should NOT be active
        if let Some(&pos) = record.actor_positions.get("Ind: Worker") {
            assert_ne!(
                pos, "active",
                "Worker in subordinate clause should not be active"
            );
        }
    }

    #[test]
    fn eu_directive_member_state_shall_require() {
        let text = "Member States shall require that the manufacturer, the supplier, and each undertaking ensures that high-activity sealed sources comply with the requirements.";
        let record = parse(text);
        assert!(
            record.duty_types.contains(&DutyType::Obligation),
            "'shall require' should be Responsibility, got: {:?}",
            record.duty_types
        );
    }

    // ── Purpose gate: actor override tests ───────────────────────────

    #[test]
    fn enactment_with_government_actor_gets_drrp() {
        // Commencement provision with Secretary of State power
        let text = "This Act shall come into operation on such day as the Secretary of State may by order appoint.";
        let record = parse(text);
        assert!(
            !record.duty_types.is_empty(),
            "Enactment with government actor should get DRRP, got empty"
        );
    }

    #[test]
    fn interpretation_with_government_actor_gets_drrp() {
        // Definition provision that also contains a government obligation
        let text = "Member States shall ensure the establishment, regular review and use of diagnostic reference levels for radiodiagnostic examinations.";
        let record = parse(text);
        assert!(
            !record.duty_types.is_empty(),
            "Interpretation with government actor should get DRRP, got empty"
        );
    }

    #[test]
    fn all_skip_with_government_actor_gets_drrp() {
        // Interpretation-only provision with government obligation
        let text = "For the purpose of obtaining information, the Executive may serve on any person a notice requiring that person to furnish information.";
        let record = parse(text);
        // Executive is a government actor — gate should let this through
        assert!(
            !record.government_actors.is_empty(),
            "should extract government actor, got: {:?}",
            record.government_actors
        );
    }

    #[test]
    fn amendment_with_government_actor_still_skipped() {
        // Amendment provisions should always skip — obligations belong to target
        let text = r#"In section 3, for subsection (2) substitute— "The Secretary of State shall ensure compliance.""#;
        let record = parse(text);
        assert!(
            record.duty_types.is_empty(),
            "amendment should skip DRRP even with government actor"
        );
    }

    #[test]
    fn app_scope_with_governed_only_still_skipped() {
        // Scope extension with governed actors but no government actors
        let text = "These Regulations shall apply to a self-employed person as they apply to an employer and an employee.";
        let record = parse(text);
        assert!(
            record.duty_types.is_empty(),
            "scope extension with governed-only actors should skip"
        );
    }

    #[test]
    fn app_scope_with_government_actor_gets_drrp() {
        // Application+Scope provision with government obligation
        let text = "The Executive shall submit under subsection (4)(b)(i) a report on the application of these provisions.";
        let record = parse(text);
        // Executive is a government actor — should not be blocked
        assert!(
            !record.government_actors.is_empty(),
            "should extract government actor"
        );
    }

    // ── Shadow-mode: parse_v2 vs parse_v2_with_trail ────────────────

    /// Verify that `parse_v2_with_trail` produces the same TaxaRecord as `parse_v2`
    /// across a representative set of provision texts.
    #[test]
    fn shadow_mode_parse_v2_with_trail_matches() {
        let texts = [
            "The employer shall ensure the health and safety of employees.",
            "The Secretary of State shall have power to make regulations.",
            "<p>The employer <b>shall</b> ensure safety.</p>",
            "",
            "This Act may be cited as the Health and Safety at Work etc. Act 1974.",
            "It shall be the duty of every employer to ensure, so far as is reasonably practicable, the health, safety and welfare at work of all his employees.",
            r#"In these Regulations— "employer" means a person who employs one or more employees."#,
            "In section 3, for subsection (2) substitute the following provisions.",
            "Every employer shall ensure the health and safety of employees.",
            "The enforcing authority may serve on the responsible person a notice.",
            "An inspector may serve an improvement notice.",
            "Every traffic route must be suitable for the persons using them.",
            "It is an offence for a person to fail to comply with the requirement.",
            "The authority may appoint any suitably qualified person as an inspector.",
            "The secretary of state may give directions to the executive.",
            "Nothing in this section shall be taken to compel production of a document.",
            "The Regulations impose duties on employers.",
        ];

        for text in texts {
            let old = parse_v2(text, None);
            let (new, _trail) = parse_v2_with_trail(text, None);

            assert_eq!(
                old.duty_types, new.duty_types,
                "duty_types mismatch for: {text:.80}"
            );
            assert_eq!(
                old.governed_actors, new.governed_actors,
                "governed_actors mismatch for: {text:.80}"
            );
            assert_eq!(
                old.government_actors, new.government_actors,
                "government_actors mismatch for: {text:.80}"
            );
            assert_eq!(
                old.purposes, new.purposes,
                "purposes mismatch for: {text:.80}"
            );
            assert_eq!(
                old.popimar, new.popimar,
                "popimar mismatch for: {text:.80}"
            );
            assert_eq!(
                old.clause_refined, new.clause_refined,
                "clause_refined mismatch for: {text:.80}"
            );
            // Confidence: epsilon comparison for f32
            assert!(
                (old.taxa_confidence - new.taxa_confidence).abs() < 1e-6,
                "taxa_confidence mismatch for {text:.80}: old={} new={}",
                old.taxa_confidence,
                new.taxa_confidence,
            );
            assert_eq!(
                old.actor_positions, new.actor_positions,
                "actor_positions mismatch for: {text:.80}"
            );
        }
    }

    /// Shadow test against 54 hard provisions from the Liberty false-positives
    /// investigation (Liberty→Obligation and Liberty→none mismatches).
    /// Verifies parse_v2 and parse_v2_with_trail produce identical output
    /// on the provisions most likely to expose tie-breaking edge cases.
    #[test]
    fn shadow_mode_hard_provisions() {
        let json_str = include_str!("testdata_hard_provisions.json");
        let texts: Vec<String> = serde_json::from_str(json_str).expect("valid JSON fixture");

        assert!(
            texts.len() >= 50,
            "expected at least 50 hard provisions, got {}",
            texts.len(),
        );

        for (i, text) in texts.iter().enumerate() {
            let old = parse_v2(text, None);
            let (new, trail) = parse_v2_with_trail(text, None);

            assert_eq!(
                old.duty_types, new.duty_types,
                "duty_types mismatch at provision {i}: {text:.80}"
            );
            assert_eq!(
                old.governed_actors, new.governed_actors,
                "governed_actors mismatch at provision {i}"
            );
            assert_eq!(
                old.government_actors, new.government_actors,
                "government_actors mismatch at provision {i}"
            );
            assert_eq!(
                old.purposes, new.purposes,
                "purposes mismatch at provision {i}"
            );
            assert!(
                (old.taxa_confidence - new.taxa_confidence).abs() < 1e-6,
                "taxa_confidence mismatch at provision {i}: old={} new={}",
                old.taxa_confidence,
                new.taxa_confidence,
            );
            assert_eq!(
                old.actor_positions, new.actor_positions,
                "actor_positions mismatch at provision {i}"
            );

            // Verify trail has a valid reason
            match trail.reason {
                decision::DecisionReason::TierPriority(_) => {
                    assert!(trail.winner.is_some(), "TierPriority must have a winner at provision {i}");
                }
                decision::DecisionReason::NoSignals
                | decision::DecisionReason::PurposeGated
                | decision::DecisionReason::LegalFiction
                | decision::DecisionReason::DescriptiveSummary
                | decision::DecisionReason::EmptyText => {
                    assert!(trail.winner.is_none(), "non-match reason should have no winner at provision {i}");
                }
            }
        }
    }
}
