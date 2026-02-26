# Session: 2026-02-26 — Promote v2 + Run Enrichment + Clause Quality Evaluation

**Parent sessions**: [02-26-26-v2-validation-at-scale.md](02-26-26-v2-validation-at-scale.md), [02-26-26-drrp-parser-v2.md](02-26-26-drrp-parser-v2.md)
**Status**: Active

## Objectives

1. **Promote v2 to default**: switch `taxa enrich` from `parse()` to `parse_v2()`, retire the v1 `GOVERNED_ACTORS` blunt gate
2. **Run enrichment across all 452 full-text laws** in LanceDB (regex only, no AI)
3. **Evaluate clause_refined quality** — build a systematic way to assess the focused DRRP clauses

## Current State

| Store | Count | Detail |
|-------|-------|--------|
| LanceDB `legislation_text` | 452 laws, 97,522 sections | Full provision text loaded |
| DuckDB `legislation` | 19,318 laws total | 2,615 with taxa data (13.5%) |
| Taxa parser | v1 (`parse()`) in `taxa enrich` | v2 (`parse_v2()`) in `taxa show` |

The enrichment pipeline reads from LanceDB (`legislation_text`), runs the taxa classifier, and writes:
- Per-provision results back to LanceDB (taxa columns on `legislation_text`)
- Per-law aggregates to DuckDB (`legislation` table — holders, roles, DRRP entries)

No AI is needed — the regex pipeline (`parse_v2()`) handles classification and clause extraction entirely in Rust. The AI polisher is a separate downstream step.

## Plan

### Step 1: Switch `taxa enrich` to `parse_v2()`

In `crates/fractalaw-cli/src/main.rs` line ~997: change `taxa::parse(&text)` → `taxa::parse_v2(&text)`.

This gives enrichment:
- Actor-anchored DRRP (fewer false positives)
- Span-based `clause_refined` (focused snippets instead of full text)

### Step 2: Delete `GOVERNED_ACTORS` blunt gate

In `crates/fractalaw-core/src/taxa/duty_patterns.rs`:
- Remove `GOVERNED_ACTORS` constant
- Remove `has_governed_actor()` function
- Remove `match_governed()` function (the v1 governed tier)
- Keep `has_obligation()`, `has_prohibition()`, `has_enabling()` — still used by v1 government tier

In `crates/fractalaw-core/src/taxa/duty_type.rs`:
- Remove `classify()` (v1 orchestrator) — only keep `classify_v2()`
- Or: rewrite `classify()` to call `classify_v2()` internally

In `crates/fractalaw-core/src/taxa/mod.rs`:
- Remove `parse()` or rewrite as alias for `parse_v2()`
- `parse_compare()` can be removed (no more v1 to compare against)

### Step 3: Run enrichment on all 452 laws

```bash
cargo run -p fractalaw-cli -- taxa enrich
```

This will auto-select all laws without taxa data (or re-run on all). ~97k sections through the regex pipeline. No AI, no network, pure CPU.

### Step 4: Clause quality evaluation

Build a `taxa show --clauses` mode that scores each clause_refined:

**Quality signals** (positive):
- Contains the actor keyword from the matched actor
- Contains a modal verb (shall/must/may)
- Contains action language after the modal
- Ends at a sentence boundary (`.` or `;`)
- Length is 50-300 chars (not too short, not maxed out)
- Starts with the actor or a sentence-initial capital

**Quality signals** (negative):
- Starts with `...` (mid-sentence truncation)
- Ends with `...` (truncated before sentence end)
- Contains parenthetical references `(a)`, `(b)` without the main clause
- Is just a sub-paragraph fragment
- Repeats text from a different provision

Score 0-10, display distribution, show worst examples for manual review.
