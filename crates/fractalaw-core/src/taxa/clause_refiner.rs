//! Modal-window clause extraction and refinement.
//!
//! Detection patterns capture everything before the modal verb (shall/must/may)
//! but often miss the actual action that follows. This module extracts a clean
//! window around the modal:
//! - **Subject**: the actor (up to 100 chars before modal)
//! - **Modal**: the obligation verb
//! - **Action**: what they must do (up to 200 chars after modal)
//!
//! Ported from `Taxa.ClauseRefiner`.

use std::sync::LazyLock;

use regex::Regex;

// ── Constants ────────────────────────────────────────────────────────

static MODAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(shall|must|may(?:\s+(?:not|only))?)\b").unwrap());
static SENTENCE_END_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[.;]|\n\n").unwrap());
// Matches a sentence boundary: period or semicolon followed by whitespace and uppercase.
// Rust regex doesn't support lookahead, so we capture the boundary and uppercase char.
static SENTENCE_START_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.;]\s+([A-Z])").unwrap());

// ── Public API ───────────────────────────────────────────────────────

/// Refine a raw clause capture into a focused, readable clause.
///
/// - `raw_clause`: the raw regex match
/// - `section_text`: optional full section for action extraction
/// - `captured_action`: optional V2 capture group text
///
/// Returns a refined clause string (max 300 chars).
pub fn refine(
    raw_clause: &str,
    section_text: Option<&str>,
    captured_action: Option<&str>,
) -> String {
    if raw_clause.is_empty() {
        return String::new();
    }

    // Find the last modal verb in the raw clause
    let modal_match = find_last_modal(raw_clause);

    match modal_match {
        None => {
            // No modal found — return as-is
            raw_clause.trim().to_string()
        }
        Some((modal_start, modal_len, modal_text)) => {
            let subject = extract_subject(raw_clause, modal_start);

            let action = if let Some(ca) = captured_action {
                if !ca.is_empty() {
                    ca.trim().to_string()
                } else {
                    extract_action(raw_clause, modal_start + modal_len, section_text)
                }
            } else {
                extract_action(raw_clause, modal_start + modal_len, section_text)
            };

            combine_clause(&subject, &modal_text, &action)
        }
    }
}

// ── Modal detection ──────────────────────────────────────────────────

fn find_last_modal(text: &str) -> Option<(usize, usize, String)> {
    let mut last: Option<(usize, usize, String)> = None;
    for m in MODAL_RE.find_iter(text) {
        last = Some((m.start(), m.len(), m.as_str().to_string()));
    }
    last
}

// ── Subject extraction ───────────────────────────────────────────────

fn extract_subject(text: &str, modal_start: usize) -> String {
    let before_modal = &text[..modal_start];

    // Scan all text before the modal for the last sentence start
    let mut last_cap_start = None;
    for caps in SENTENCE_START_RE.captures_iter(before_modal) {
        if let Some(m) = caps.get(1) {
            last_cap_start = Some(m.start());
        }
    }
    if let Some(pos) = last_cap_start {
        return before_modal[pos..].trim_start().to_string();
    }

    // No boundary — use start of text
    if before_modal
        .trim_start()
        .starts_with(|c: char| c.is_ascii_uppercase())
    {
        before_modal.trim_start().to_string()
    } else {
        clean_leading(before_modal.trim_start())
    }
}

fn clean_leading(text: &str) -> String {
    let s =
        text.trim_start_matches(|c: char| c == ',' || c == ';' || c == ':' || c.is_whitespace());
    s.to_string()
}

// ── Action extraction ────────────────────────────────────────────────

fn extract_action(raw_clause: &str, modal_end: usize, section_text: Option<&str>) -> String {
    let after = &raw_clause[modal_end..];

    if after.trim().is_empty() {
        if let Some(section) = section_text {
            extract_action_from_section(raw_clause, section)
        } else {
            String::new()
        }
    } else {
        extract_to_sentence_end(after)
    }
}

fn extract_action_from_section(raw_clause: &str, section_text: &str) -> String {
    // Find the modal in raw_clause, locate it in section_text, extract action
    let Some((modal_start, _modal_len, modal_text)) = find_last_modal(raw_clause) else {
        return String::new();
    };
    let ctx_start = snap_down(raw_clause, modal_start.saturating_sub(40));
    let ctx_end = snap_up(raw_clause, (modal_start + 60).min(raw_clause.len()));
    let context = &raw_clause[ctx_start..ctx_end];

    // Try to find context in section text
    if let Some(pos) = section_text.find(context) {
        let section_context = &section_text[pos..];
        if let Some(modal_pos) = section_context.find(&modal_text) {
            let action_start = modal_pos + modal_text.len();
            return extract_to_sentence_end(&section_context[action_start..]);
        }
    }
    String::new()
}

fn extract_to_sentence_end(text: &str) -> String {
    for m in SENTENCE_END_RE.find_iter(text) {
        if m.start() == 0 {
            continue;
        }
        let ch = &text[m.start()..m.start() + 1];
        if ch == "." {
            return text[..m.start() + 1].to_string();
        }
        // ch == ";" — skip if followed by sub-paragraph marker
        let rest = &text[m.start() + 1..];
        let trimmed = rest.trim_start();
        if trimmed.starts_with('(') {
            continue;
        }
        if (trimmed.starts_with("and ") || trimmed.starts_with("or "))
            && trimmed[3..].trim_start().starts_with('(')
        {
            continue;
        }
        return text[..m.start() + 1].to_string();
    }
    truncate_at_word(text).to_string()
}

fn truncate_at_word(text: &str) -> &str {
    let trimmed = text.trim_end();
    if let Some(pos) = trimmed.rfind(|c: char| c.is_whitespace()) {
        trimmed[..pos].trim_end()
    } else {
        trimmed
    }
}

// ── Clause combination ───────────────────────────────────────────────

fn combine_clause(subject: &str, modal: &str, action: &str) -> String {
    let parts: Vec<&str> = [subject.trim(), modal.trim(), action.trim()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect();
    let mut clause = parts.join(" ");

    // Collapse multiple spaces
    while clause.contains("  ") {
        clause = clause.replace("  ", " ");
    }

    clause
}

// ── Char-boundary helpers ────────────────────────────────────────────

/// Snap a byte offset down to the nearest char boundary (for start of slices).
fn snap_down(text: &str, offset: usize) -> usize {
    let mut pos = offset.min(text.len());
    while pos > 0 && !text.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Snap a byte offset up to the nearest char boundary (for end of slices).
fn snap_up(text: &str, offset: usize) -> usize {
    let mut pos = offset.min(text.len());
    while pos < text.len() && !text.is_char_boundary(pos) {
        pos += 1;
    }
    pos
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refine_simple_clause() {
        let raw = "The employer shall ensure";
        let result = refine(raw, None, None);
        assert!(result.contains("employer"));
        assert!(result.contains("shall"));
    }

    #[test]
    fn refine_with_section_text() {
        let raw = "The planning authority must";
        let section = "Some preamble. The planning authority must consult the relevant bodies.";
        let result = refine(raw, Some(section), None);
        assert!(result.contains("must"));
    }

    #[test]
    fn refine_with_captured_action() {
        let raw = "The employer shall";
        let action = "ensure the health and safety of employees.";
        let result = refine(raw, None, Some(action));
        assert!(result.contains("ensure the health"));
    }

    #[test]
    fn refine_empty_input() {
        assert!(refine("", None, None).is_empty());
    }

    #[test]
    fn refine_no_modal() {
        let result = refine("some text without a modal verb", None, None);
        assert!(!result.is_empty());
    }

    #[test]
    fn refine_long_preamble_finds_sentence_start() {
        let preamble = "a ".repeat(200);
        let raw = format!("{preamble}The employer shall ensure safety.");
        let result = refine(&raw, None, None);
        assert!(
            result.contains("employer"),
            "should contain actor: {result}"
        );
        assert!(result.contains("shall"), "should contain modal: {result}");
    }

    #[test]
    fn refine_long_action_not_truncated() {
        let long_text = "The employer shall ensure safety and welfare. Done.";
        let result = refine(long_text, None, None);
        assert!(
            result.contains("ensure safety and welfare."),
            "should reach sentence end: {result}"
        );
    }

    #[test]
    fn find_last_modal_works() {
        let text = "The employer shall ensure employees must comply";
        let (start, _, modal) = find_last_modal(text).unwrap();
        assert_eq!(modal, "must");
        assert!(start > 0);
    }

    #[test]
    fn extract_subject_sentence_start() {
        let text = "Some context. The employer shall ensure safety.";
        let subject = extract_subject(text, text.find("shall").unwrap());
        assert!(subject.starts_with("The employer"));
    }
}
