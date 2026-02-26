//! Clause confidence scorer for regex-extracted DRRP clauses.
//!
//! Produces a `0.0..=1.0` score based on heuristics about how well
//! the regex pipeline captured the clause. Low-confidence entries
//! get queued for AI refinement.
//!
//! Ported from `Taxa.RegexClauseConfidence`.

use std::sync::LazyLock;

use regex::Regex;

// ── Scoring weights ──────────────────────────────────────────────────
//
// | Signal                          | Weight |
// |---------------------------------|--------|
// | V2 capture group matched        | +0.25  |
// | Clause ends cleanly (no `...`)  | +0.25  |
// | Clause length adequate (>30 ch) | +0.20  |
// | Strong modal verb (shall/must)  | +0.15  |
//
// Base score is 0.0, signals are additive. Max from this module is 0.85.

static STRONG_MODAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:shall|must)\b").unwrap());

/// Score the confidence of a regex-extracted clause.
///
/// - `has_captured_action`: Whether a V2 capture group matched.
///
/// Returns a float between `0.0` and `0.85`.  Max 0.85 from regex alone;
/// the remaining 0.15 headroom is reserved for AI refinement.
pub fn score(clause: &str, has_captured_action: bool) -> f32 {
    if clause.is_empty() {
        return 0.0;
    }

    let mut s: f32 = 0.0;

    if has_captured_action {
        s += 0.25;
    }
    if clause.ends_with(['.', ';']) {
        s += 0.25;
    }
    if clause.len() > 30 {
        s += 0.20;
    }
    if STRONG_MODAL.is_match(clause) {
        s += 0.15;
    }

    (s * 100.0).round() / 100.0
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_clause() {
        assert_eq!(score("", false), 0.0);
    }

    #[test]
    fn short_no_ending_no_capture() {
        // < 30 chars, no sentence-end punctuation, no capture, no modal
        assert_eq!(score("employer may do things", false), 0.0);
    }

    #[test]
    fn full_quality_clause() {
        let clause = "The employer shall ensure the health and safety of all employees.";
        let s = score(clause, true);
        // span + clean ending + adequate length + strong modal = 0.25 + 0.25 + 0.20 + 0.15
        assert!((s - 0.85).abs() < 0.01);
    }

    #[test]
    fn no_capture_but_clean() {
        let clause = "The employer shall ensure the health and safety of employees.";
        let s = score(clause, false);
        // clean ending + adequate length + strong modal = 0.25 + 0.20 + 0.15 = 0.60
        assert!((s - 0.60).abs() < 0.01);
    }

    #[test]
    fn no_sentence_end_with_capture() {
        let clause = "The employer shall ensure safety and welfare for all";
        let s = score(clause, true);
        // capture + adequate length + strong modal = 0.25 + 0.20 + 0.15 = 0.60
        assert!((s - 0.60).abs() < 0.01);
    }

    #[test]
    fn may_only_clause() {
        let clause = "The employee may request a review of the assessment.";
        let s = score(clause, false);
        // clean ending + adequate length = 0.25 + 0.20 = 0.45
        assert!((s - 0.45).abs() < 0.01);
    }
}
