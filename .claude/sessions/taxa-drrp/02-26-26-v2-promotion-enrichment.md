# Session: 2026-02-26 — Promote v2 + Run Enrichment + Clause Quality Evaluation

**Parent sessions**: [02-26-26-v2-validation-at-scale.md](02-26-26-v2-validation-at-scale.md), [02-26-26-drrp-parser-v2.md](02-26-26-drrp-parser-v2.md)
**Status**: Complete

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

## Results

### Step 1: Switch `taxa enrich` to `parse_v2()` — DONE (`750f40d`)

Changed `cmd_taxa_enrich()` from `taxa::parse()` to `taxa::parse_v2()`. All enrichment now uses the actor-anchored v2 pipeline with span-based clause extraction.

### Step 2: Retire v1 governed tier — DONE (`750f40d`)

Removed from `duty_patterns.rs`:
- `GOVERNED_ACTORS` constant (the blunt gate list)
- `has_governed_actor()` function
- `match_governed()` function + all 6 governed regex statics
- ~20 governed tests

Removed from `mod.rs`:
- `parse_compare()`, `CompareRecord`, and their tests

Removed from `main.rs`:
- `--compare` flag and all compare logic from `cmd_taxa_show`

Net: ~300 lines of v1 governed code deleted. 258 tests pass.

### Step 3: Clause quality evaluation — DONE (`02a97b1`)

Added `taxa_confidence: f32` to `TaxaRecord`, wired `confidence::score()` into `parse_v2()`.

Added `taxa show --clauses` mode with:
- Confidence distribution histogram (High >= 0.80, Medium 0.45-0.79, Low < 0.45)
- Span vs refiner coverage stats
- Low-quality clause listing with details
- High-quality sample display

HSWA results: 49/98 high quality, 35 medium, 14 low. Span-based extraction covers 93% of provisions.

### Step 4: `--force` flag + enrichment run — DONE (`e226589`)

Added `--force` flag to `taxa enrich`:
- Clears DuckDB taxa columns with `UPDATE SET NULL` (not DELETE)
- Pre-fetches distinct `law_name` from LanceDB to only process laws with text (452 vs 19,318)

Fixed 3 Unicode char boundary panics discovered during enrichment:
- `main.rs`: `truncate_at_char_boundary()` for clause preview strings (multi-byte `‐`)
- `mod.rs`: `snap_char_boundary_down()` for span-based clause extraction (`❌` emoji)
- `clause_refiner.rs`: `snap_down()`/`snap_up()` for modal window arithmetic

Added DuckDB safety deny rules to `.claude/settings.json`.

**Enrichment result**: 452 laws processed, **270 laws now have DRRP taxa data** (182 had no provisions matching DRRP patterns).

## Updated State

| Store | Count | Detail |
|-------|-------|--------|
| LanceDB `legislation_text` | 452 laws, 97,522 sections | Full provision text loaded |
| DuckDB `legislation` | 19,318 laws total | **270 with v2 taxa data** (was 2,615 v1) |
| Taxa parser | v2 (`parse_v2()`) everywhere | v1 fully retired |

## Commits

| Hash | Description |
|------|-------------|
| `750f40d` | Retire v1 governed tier: remove GOVERNED_ACTORS, match_governed, parse_compare, --compare CLI |
| `02a97b1` | Add clause quality evaluation: taxa_confidence on TaxaRecord + taxa show --clauses |
| `e226589` | Fix Unicode char boundary panics, add --force flag to taxa enrich |

## Final Test Suite

258 tests pass, 0 fail (down from 282 due to v1 removal — ~24 governed/compare tests deleted).

## Remaining Work (Future Sessions)

- **GH#16 "Rule" classifier** — thing-subject obligations (~50 provisions) need a separate pattern that doesn't require a person/org actor
- **Provision-chain inference** — elaboration sub-provisions (~25) inherit duties from parent provisions; requires section hierarchy parsing or AI polisher context
- **Phase C: DRRP map in LanceDB + LanceDB-only polisher** — store per-provision taxa columns in LanceDB, rewrite polisher guest to work entirely against LanceDB (see plan: elegant-whistling-sparrow.md)
- **270 vs 452 gap** — 182 laws have LanceDB text but no DRRP matches; investigate whether they contain regulatory obligations in forms v2 doesn't handle (thing-subject, passive voice without "by", etc.)
