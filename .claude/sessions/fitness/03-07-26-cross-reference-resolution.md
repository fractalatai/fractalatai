# Session: Cross-Reference Provision Resolution (#22)

**Date**: 2026-03-07
**Issue**: [#22 — Fitness: resolve cross-reference provisions to extract p-dimensions](https://github.com/fractalaw/fractalaw/issues/22)
**Depends on**: #23 (closed) — p-dimension dictionaries, #7 (closed) — fitness denormalization

## Problem

Some APPLICATION_SCOPE provisions don't contain their own applicability vocabulary — they reference other provisions instead. These produce zero p-dimension tags because the scope vocabulary (person, place, process, etc.) lives in the referenced provision, not the current one.

### Two problems

1. **Polarity fails**: `APPLIES_RE` requires `shall apply` or `appl(y|ies) (to|in|where|in)`. Bare "regulation X applies," (followed by comma/clause boundary) matches neither.
2. **P-dimensions empty**: Even when polarity succeeds (e.g., "does not apply"), the text has no scope vocabulary — only a cross-reference identifier.

## Investigation

### Corpus analysis: OH&S gap provisions

Ran `taxa audit-fitness --family "OH&S: Occupational / Personal Safety" --limit 0` and classified all 94 gap provisions (polarity detected, zero p-dimension tags):

| Category | Count | % | Description |
|----------|-------|---|-------------|
| **Cross-reference** | **63** | **67%** | References another provision by number |
| **Vocabulary gap** | **31** | **33%** | Scope vocabulary not in dictionaries |

Cross-reference patterns detected: `regulation N`, `paragraph (N)`, `sub-paragraph (N)`, `section N`, `article N`, `schedule N`, `part N`.

### No-polarity analysis: side finding (#26)

15 of 99 "no-polarity" provisions contain "shall (not) apply" — polarity should match but doesn't. Root cause: APPLICATION_SCOPE is not the first purpose → `parse_v2()` takes DRRP path, `fitness_rules` left empty. Filed as [#26](https://github.com/fractalaw/fractalaw/issues/26).

## Implementation: Phase 1 — Cross-Reference Detection ✓

### Changes

#### `crates/fractalaw-core/src/taxa/fitness.rs`

- **New regex** `CROSS_REF_RE`: detects `regulation/paragraph/sub-paragraph/section/article/schedule/part` followed by a number
- **New function** `detect_cross_refs(text)`: returns deduplicated list of matched cross-reference strings
- **New field** `cross_refs: Vec<String>` on `FitnessRule`: populated in `extract()` and `try_split_compound()`
- **5 new tests**: `cross_ref_regulation_detected`, `cross_ref_paragraph_detected`, `cross_ref_schedule_detected`, `cross_ref_multiple_deduplicated`, `no_cross_ref_when_none_present`

#### `crates/fractalaw-cli/src/main.rs`

- **FamilyStats**: new `cross_ref_count` and `cross_ref_provisions` fields
- **Gap detection**: provisions with polarity + zero tags now split into vocabulary gaps (no cross-refs) vs cross-reference provisions (has cross-refs)
- **Section 1**: new "CrossRef" column in coverage table
- **Section 2**: renamed to "Vocabulary Gaps (polarity, 0 tags, no cross-ref)"
- **Section 2b**: new "Cross-Reference Provisions (polarity, 0 tags, has cross-ref)"
- **Section 3**: candidate terms now extracted from vocabulary gaps only (cleaner signal)

### Results

| Metric | Before | After |
|--------|--------|-------|
| Total gaps | 94 | 31 vocab + 63 cross-ref = 94 |
| Section 2 (actionable) | 94 mixed | 31 vocabulary gaps |
| Section 2b (cross-refs) | — | 63 cross-reference provisions |
| Section 3 candidates | Noisy (cross-ref terms mixed in) | Clean (carriage, establishment, Crown, PPE) |

All 347 core tests pass (5 new). CLI compiles and audit output verified.

## Key Files

- `crates/fractalaw-core/src/taxa/fitness.rs` — `CROSS_REF_RE` (line ~112), `detect_cross_refs()` (line ~126), `FitnessRule.cross_refs` (line 86)
- `crates/fractalaw-cli/src/main.rs` — `FamilyStats` (cross_ref_count, cross_ref_provisions), Section 2b output

## Implementation: Phase 2 — Dictionary Expansion & Bug Fixes ✓

### Changes

#### `crates/fractalaw-core/src/taxa/fitness.rs`

**Cross-ref regex plural fix**: `CROSS_REF_RE` now handles plurals (`regulations?`, `paragraphs?`, etc.)

**Word-boundary plural bug**: `dict()` wraps patterns in `\b...\b`, which breaks on plurals (e.g., `\bfumigation\b` won't match "fumigations" because "n" is followed by "s"). Fixed by using `s?` in patterns.

**New core dictionary entries**:
- Process: 3 carriage patterns (`carriage of/class`, `national carriage`, `carriage by road/rail/sea/air/vehicles?`)
- Plant: `\bPPE\b` → "personal protective equipment"
- Place: `establishment(?:s)?` → "establishment"
- Property: `(?:bind|apply\s+to)\s+the\s+Crown` → "Crown application"

**New OH&S specialist entries**:
- OHS_PLANT_DICT: `door, gate or hatch` pattern
- New `OHS_PROCESS_EXT_DICT`: `fumigat(?:ions?|ants?|ing)` → "fumigation", `relevant\s+project` → "construction project"

**1 new test**: `cross_ref_plural_regulations`

### Results

| Metric | Post Phase 1 | Post Phase 2 | Change |
|--------|-------------|-------------|--------|
| Tagged% | 52.3% | 58.5% | +6.2pp |
| Vocabulary Gaps | 31 | 10 | -21 |
| Cross-Refs | 63 | 57 | -6 |

**Remaining 10 vocabulary gaps** (genuinely hard cases):
- HSWA 1974 abstract provisions (3): "prescribed process", "duty...things done in the course of trade", "activities...carried on"
- Corporate Manslaughter 2007 (3): "organisation owed a duty of care", "partnership...legal person", "person's death"
- Fire-fighting equipment (1): "equipment used exclusively for fire-fighting purposes"
- Statutory provisions/Schedule (1): meta-legal reference without numbered schedule
- Dismissal (1): employment law term
- Food and drink (1): Welsh SI about school food standards in OH&S family

## Future Phases

### Phase 3: Intra-law resolution (future, if needed)

For the 57 cross-reference provisions, some reference provisions within the same law that could be looked up and their p-dimensions extracted. This requires:
- Parsing provision references into structured form
- Matching against LanceDB `provision` column values
- Two-pass enrichment within `enrich_single_law()`

## Key Files

- `crates/fractalaw-core/src/taxa/fitness.rs` — `CROSS_REF_RE` (line ~115), `detect_cross_refs()` (line ~128), `FitnessRule.cross_refs` (line 86), dictionaries (lines 147–416)
- `crates/fractalaw-cli/src/main.rs` — `FamilyStats` (cross_ref_count, cross_ref_provisions), Section 2b output

## Status: **Closed** — #22 complete (Phases 1+2). Follow-ups: #26, #27, #28
