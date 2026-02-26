# Session: 2026-02-26 — Clause Quality Improvement

**Parent session**: [02-26-26-phase-c-lancedb-polisher.md](02-26-26-phase-c-lancedb-polisher.md)
**Status**: Active

## Objective

Improve regex `clause_refined` quality for the 18,139 DRRP provisions. Current baseline:

| Quality | Count | % |
|---------|-------|---|
| Good | 8,476 | 46.7% |
| Mid-sentence start | 5,928 | 32.7% |
| Truncated ending (`...`) | 3,735 | 20.6% |

Target: reduce mid-sentence starts and truncated endings significantly. Good should be >70%.

## Problem Analysis

### Mid-sentence starts (32.7% = 5,928 provisions)

The clause window starts at a byte offset before the actor, but doesn't snap to a sentence boundary. Examples:

- `"rrier of controlled waste for registration as a broker..."` — starts mid-word
- `"he appeal under section 43(2)(b) of that Act, make a written report..."` — starts after a pronoun
- `"activities authorised by the permit include the disposal of waste, the pollution control authority shall ensure..."` — starts in a subordinate clause

**Root cause**: `extract_clause()` in `mod.rs` and `extract_subject()` in `clause_refiner.rs` use a byte window (100 chars before actor) but don't reliably find the sentence start. The `SENTENCE_START_RE` regex looks for `[.;]\s+[A-Z]` but misses:
- Provisions that start at the beginning of the text (no preceding period)
- Numbered sub-paragraphs `(a)`, `(1)` as sentence starters
- Long preambles where the sentence start is >100 chars before the actor

### Truncated endings (20.6% = 3,735 provisions)

The clause ends with `...` because the action window (200 chars after modal) doesn't reach a sentence boundary. Examples:

- `"It shall be the duty of each licensing authority to establish and maintain a register for the purposes of paragraph (1) "` — cuts off before the full duty is stated
- `"it shall be an offence for an establishment or undertaking to carry on, after 31st "` — mid-date

**Root cause**: `extract_action()` uses a fixed 200-char window. `extract_to_sentence_end()` looks for `[.;]` but many provisions use long enumerated lists before reaching a period.

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/taxa/mod.rs` | `extract_clause()` — span-based window extraction |
| `crates/fractalaw-core/src/taxa/clause_refiner.rs` | `refine()`, `extract_subject()`, `extract_action()` — modal-window extraction |
| `crates/fractalaw-core/src/taxa/confidence.rs` | `score()` — clause quality scorer |

## Approach

1. **Audit**: Sample provisions from each problem category, identify patterns
2. **Fix sentence-start snapping**: Improve `extract_clause()` to find real sentence boundaries (capital after period, numbered paragraphs, start of text)
3. **Fix action window**: Extend or adapt the window to reach sentence endings, handle enumerated lists
4. **Test**: Run `taxa show --clauses` on HSWA and sample laws, verify improvement
5. **Re-enrich**: Run `taxa enrich --force` to update all 452 laws with improved clauses
6. **Measure**: Re-run the quality audit, compare before/after
