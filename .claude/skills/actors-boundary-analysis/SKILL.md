# Skill: Actor Extraction Analysis (Gap B + False Positives)

## When This Applies

When investigating why `actors.rs` fails to extract an actor from text that clearly contains the keyword, or when actors are being misclassified (wrong label, false positive). This is "Gap B" in the taxa gap taxonomy.

## Architecture

### How actor extraction works

`crates/fractalaw-core/src/taxa/actors.rs` has two pattern lists:

- `GOVERNMENT_DEFS` — ~20 patterns for government actors (ministers, agencies, authorities)
- `GOVERNED_DEFS` — ~25 patterns for governed actors (employers, workers, contractors)

Each pattern is a regex with `(?:[\s[:punct:]])` word boundaries on both sides:
```rust
actor!("Org: Employer", r"(?:[\s[:punct:]])[Ee]mployers?(?:[\s[:punct:]])")
```

### The extraction pipeline

1. `apply_blacklist(text)` — removes known false-positive phrases (e.g. "public interest", "agency worker")
2. `run_patterns(text, patterns)` — pads text with spaces, runs each pattern, removes first match to prevent duplicates
3. Returns `ExtractedActors { governed, government }`

### Three protection mechanisms

1. **Blacklist** (`BLACKLIST` array) — regex patterns that strip phrases BEFORE actor matching. Use this when a word triggers a false positive in a specific context (e.g. "agency" in "agency worker").

2. **Pattern specificity** — more specific patterns run first (e.g. "Health and Safety Executive" before generic "Executive"). The progressive removal in `run_patterns` prevents double-counting.

3. **Space padding** — `run_patterns()` wraps text as `" {text} "` so boundary patterns match at start/end of string.

## How to Find Gap B Issues

### The visibility problem

`taxa show` **skips** provisions with no classification signal (no DRRP, no actors, no purposes). So if `actors.rs` fails to extract an actor, the provision is invisible in `taxa show` output. Gap B is hidden by default.

### Approach 1: Boundary test (direct)

Write Rust tests that call `extract_actors()` with text where the keyword is at string boundaries:

```rust
#[test]
fn keyword_at_start_of_string() {
    let actors = extract_actors("Employer shall ensure safety.");
    assert!(actors.governed.contains(&"Org: Employer".to_string()));
}

#[test]
fn keyword_at_end_of_string() {
    let actors = extract_actors(" duties of the employer");
    assert!(actors.governed.contains(&"Org: Employer".to_string()));
}
```

### Approach 2: Compare raw text against taxa show

1. Dump raw text with `fractalaw text --limit 500 <LAW>`
2. Search for governed actor keywords + modal verbs in the raw text
3. Compare against `taxa show` output — provisions that appear in raw text search but NOT in taxa show are Gap B candidates

### Approach 3: False positive detection

Run `taxa show` and look for actor labels that don't make sense in context:

```bash
# Dump all classifications
for law in <LAW_LIST>; do
  cargo run -p fractalaw-cli -- taxa show --limit 500 "$law" 2>/dev/null
done | grep "Government:" | sort | uniq -c | sort -rn
```

Look for labels in unexpected laws (e.g. "Gvt: Agency" in employment regulations → "agency worker" false positive).

## Fixing False Positives

Add to the `BLACKLIST` array in `actors.rs`:

```rust
static BLACKLIST: &[&str] = &[
    r"local authority collected municipal waste",
    r"[Pp]ublic (?:nature|sewer|importance|functions?|interest|[Ss]ervices)",
    r"[Rr]epresentatives? of",
    r"(?i)agency workers?",           // employment, not govt
    r"(?i)temporary work agency",     // employment, not govt
];
```

The blacklist uses `replace_all` — matched text is removed before actor patterns run. This is the right mechanism when:
- A broad pattern (e.g. `[Aa]gency`) is needed as a catch-all
- But a specific compound phrase (e.g. "agency worker") is a false positive
- The specific named patterns (e.g. "Environment Agency") run first and are unaffected

## Fixing Boundary Failures

The space-padding fix in `run_patterns()` handles start/end of string:

```rust
fn run_patterns(text: &str, patterns: &[(&str, Regex)]) -> Vec<String> {
    let mut remaining = format!(" {text} ");
    // ...
}
```

If you encounter a new boundary issue, check:
1. Does `text_cleaner::clean()` produce text starting/ending with the keyword?
2. Does the regex pattern have `(?:[\s[:punct:]])` boundaries?
3. Is the space padding in `run_patterns()` still present?

## Test Patterns

Tests live in `actors.rs` `mod tests`. Naming conventions:

```rust
// Boundary tests
fn keyword_at_start_of_string()
fn keyword_at_end_of_string()

// False positive tests (blacklist)
fn <context>_not_<wrong_label>()   // e.g. agency_worker_not_government_agency

// Extraction tests
fn extract_<actor>()               // e.g. extract_employer, extract_hse
```

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/taxa/actors.rs` | Actor patterns, blacklist, `extract_actors()` |
| `crates/fractalaw-core/src/taxa/text_cleaner.rs` | Text normalisation before actor extraction |
| `crates/fractalaw-core/src/taxa/duty_patterns.rs` | `GOVERNMENT_ACTORS` / `GOVERNED_ACTORS` — gates DRRP patterns |
| `crates/fractalaw-core/src/taxa/mod.rs` | `parse()` — calls `extract_actors(&cleaned)` at step 4 |

## Relationship to Gap A (GOVERNED_ACTORS)

Gap B (actors.rs extraction failures) is distinct from Gap A (GOVERNED_ACTORS list gaps):

- **Gap A**: `actors.rs` extracts the actor correctly, but the keyword isn't in `duty_patterns.rs` `GOVERNED_ACTORS`, so `has_governed_actor()` returns false and DRRP patterns don't fire. Fix: add keyword to `GOVERNED_ACTORS`.
- **Gap B**: `actors.rs` fails to extract the actor at all — boundary matching, blacklist over-removal, or missing pattern. Fix: fix the regex, adjust blacklist, or add new pattern.

Gap A analysis is documented in `.claude/skills/taxa-gap-analysis/SKILL.md`.
