//! Normalise raw legislative text before classification.
//!
//! Steps:
//! 1. Strip HTML tags and entities.
//! 2. Collapse whitespace (including non-breaking spaces).
//! 3. Normalise quotation marks and dashes.
//! 4. Strip leading section-number prefixes like "(1)" or "1.".
//! 5. Trim leading/trailing whitespace.
//!
//! Ported from `Taxa.TextCleaner` (Elixir).

use std::borrow::Cow;
use std::sync::LazyLock;

use regex::Regex;

// ── Compiled patterns ────────────────────────────────────────────────

static HTML_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static HTML_NAMED_ENTITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&[a-z]+;").unwrap());
static HTML_NUMERIC_ENTITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&#\d+;").unwrap());
static MULTI_WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
// Matches leading provision numbering in UK legislation:
//   (1)            — sub-paragraph
//   2.             — regulation with dot
//   9.-(1)         — regulation.paragraph
//   -(1)           — dash-paragraph
//   - [F3 (1)      — dash, editorial footnote, paragraph
//   5. - [F3 (1)   — regulation, dash, editorial footnote, paragraph
//   5.- [F13 (1)   — regulation, editorial footnote, paragraph
//   [F18 5         — editorial footnote then bare sub-paragraph number
static LEADING_NUMBERING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(?:\d+[a-z]?\s*\.?\s*)?-?\s*(?:\[F\d+\s*]?\s*)?-?\s*(?:\d+[a-z]?\s*\.?\s*)?(?:\(\d+[a-z]?\)\s*\.?\s*)?",
    )
    .unwrap()
});

// ── Public API ───────────────────────────────────────────────────────

/// Clean and normalise raw legislative text for classification.
///
/// Returns a new `String` with HTML stripped, whitespace collapsed,
/// punctuation normalised, and leading section numbering removed.
pub fn clean(raw: &str) -> String {
    let s = strip_html(raw);
    let s = normalise_whitespace(&s);
    let s = normalise_punctuation(&s);
    let s = strip_leading_numbering(&s);
    s.trim().to_string()
}

// ── Private helpers ──────────────────────────────────────────────────

/// Strip HTML tags and entities, replacing them with spaces.
fn strip_html(text: &str) -> Cow<'_, str> {
    let s = HTML_TAG.replace_all(text, " ");
    let s = HTML_NAMED_ENTITY.replace_all(&s, " ");
    // If the first two passes returned borrowed (no match), this one
    // will too — zero allocations for clean text.
    HTML_NUMERIC_ENTITY.replace_all(&s, " ").into_owned().into()
}

/// Replace non-breaking spaces, em/en spaces, and collapse runs of
/// whitespace into a single ASCII space.
fn normalise_whitespace(text: &str) -> Cow<'_, str> {
    // Replace Unicode space variants with ASCII space first.
    let s = text.replace(['\u{00A0}', '\u{2002}', '\u{2003}'], " ");
    MULTI_WHITESPACE.replace_all(&s, " ").into_owned().into()
}

/// Normalise smart quotes and dashes to their ASCII equivalents.
fn normalise_punctuation(text: &str) -> Cow<'_, str> {
    // These are simple char replacements — no regex needed.
    if !text.contains([
        '\u{2018}', '\u{2019}', // smart single quotes
        '\u{201C}', '\u{201D}', // smart double quotes
        '\u{2013}', '\u{2014}', // en-dash, em-dash
    ]) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.replace(['\u{2018}', '\u{2019}'], "'")
            .replace(['\u{201C}', '\u{201D}'], "\"")
            .replace(['\u{2013}', '\u{2014}'], "-"),
    )
}

/// Strip a leading section-number prefix like "(1)", "2.", "(3a)" etc.
fn strip_leading_numbering(text: &str) -> Cow<'_, str> {
    LEADING_NUMBERING.replace(text, "")
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_html_tags() {
        assert_eq!(clean("<p>It shall be the duty</p>"), "It shall be the duty");
    }

    #[test]
    fn strips_html_named_entities() {
        assert_eq!(clean("health&amp;safety"), "health safety");
    }

    #[test]
    fn strips_html_numeric_entities() {
        assert_eq!(clean("section&#160;2"), "section 2");
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(
            clean("Every   employer    shall   ensure"),
            "Every employer shall ensure"
        );
    }

    #[test]
    fn replaces_non_breaking_spaces() {
        assert_eq!(
            clean("section\u{00A0}2 of\u{00A0}the Act"),
            "section 2 of the Act"
        );
    }

    #[test]
    fn normalises_smart_single_quotes() {
        assert_eq!(
            clean("the \u{2018}employer\u{2019} means"),
            "the 'employer' means"
        );
    }

    #[test]
    fn normalises_smart_double_quotes() {
        assert_eq!(
            clean("\u{201C}reasonably practicable\u{201D}"),
            "\"reasonably practicable\""
        );
    }

    #[test]
    fn normalises_en_dash() {
        assert_eq!(clean("health\u{2013}safety"), "health-safety");
    }

    #[test]
    fn normalises_em_dash() {
        assert_eq!(clean("health\u{2014}safety"), "health-safety");
    }

    #[test]
    fn strips_leading_parenthesised_number() {
        assert_eq!(
            clean("(1) It shall be the duty of every employer"),
            "It shall be the duty of every employer"
        );
    }

    #[test]
    fn strips_leading_number_with_dot() {
        assert_eq!(
            clean("2. Every employer shall ensure"),
            "Every employer shall ensure"
        );
    }

    #[test]
    fn strips_leading_number_with_letter() {
        assert_eq!(
            clean("(3a) The responsible person must"),
            "The responsible person must"
        );
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(clean("   It shall be the duty   "), "It shall be the duty");
    }

    #[test]
    fn empty_string_stays_empty() {
        assert_eq!(clean(""), "");
    }

    #[test]
    fn already_clean_text_unchanged() {
        let text = "Every employer shall ensure the safety of employees";
        assert_eq!(clean(text), text);
    }

    #[test]
    fn strips_dash_paragraph_number() {
        assert_eq!(
            clean("-(1) An employer who carries out work"),
            "An employer who carries out work"
        );
    }

    #[test]
    fn strips_editorial_footnote_paragraph() {
        assert_eq!(
            clean("- [F3 (1) Where a person is a user"),
            "Where a person is a user"
        );
    }

    #[test]
    fn strips_regulation_dot_dash_paragraph() {
        assert_eq!(
            clean("9.-(1) If the risk assessment indicates"),
            "If the risk assessment indicates"
        );
    }

    #[test]
    fn strips_bare_number_prefix() {
        assert_eq!(
            clean("2 The employer shall ensure"),
            "The employer shall ensure"
        );
    }

    #[test]
    fn strips_editorial_footnote_f5() {
        assert_eq!(
            clean("- [F5 (1) Where a person is employed"),
            "Where a person is employed"
        );
    }

    #[test]
    fn strips_editorial_footnote_then_bare_number() {
        // [F18 5 Where... — editorial footnote followed by bare sub-paragraph number
        assert_eq!(
            clean("[F18 5 Where the responsible person"),
            "Where the responsible person"
        );
    }

    #[test]
    fn combined_html_whitespace_punctuation_numbering() {
        let raw = "(1) <b>It shall be the duty</b> of every\u{00A0}employer to ensure, \
                   so far as is \u{201C}reasonably practicable\u{201D}, the health &amp; safety";
        let expected = "It shall be the duty of every employer to ensure, \
                        so far as is \"reasonably practicable\", the health safety";
        assert_eq!(clean(raw), expected);
    }

    // ── Real legislative text (golden tests from Elixir suite) ───────

    #[test]
    fn hswa_1974_s2_1() {
        let raw = "(1) It shall be the duty of every employer to ensure, so far as is \
                   reasonably practicable, the health, safety and welfare at work of all \
                   his employees.";
        let result = clean(raw);
        assert!(result.starts_with("It shall be the duty"));
        assert!(result.ends_with("his employees."));
        // No leading "(1)" remains
        assert!(!result.starts_with('('));
    }
}
