# Session: Enrichment Gap Investigation (#17)

**Date**: 2026-03-05
**Issue**: [#17 — Investigate 270/452 enrichment gap — 182 laws with text but no DRRP matches](https://github.com/fractalaw/fractalaw/issues/17)
**Objective**: Determine why 182 of 452 laws with provision text produce zero DRRP taxa, categorize the causes, and file follow-up issues for actionable gaps.

## Key Findings

The reported 193-law gap (464 LanceDB, 271 with `duty_holder`) was **mostly illusory**. Two bugs masked the real numbers:

### Bug 1: Panic in `clause_structure::find_modal()` (FIXED)

`clause_structure.rs:259` used `MatchSpan` positions from the original `cleaned_text` to index into the extracted `clause` substring. When the clause was shorter than the span offsets, it panicked with "byte index out of bounds". This **crashed enrichment** mid-batch, silently leaving all subsequent laws in that run unenriched.

**Fix**: Added bounds check + `is_char_boundary()` guard in `find_modal()`. Falls back to regex when span is out of bounds. (5 laws unblocked)

### Bug 2: Enrichment skip check used `duty_holder` only (FIXED)

`cmd_taxa_enrich()` line 2161 used `WHERE duty_holder IS NULL OR len(duty_holder) = 0` to find laws needing enrichment. But **108 laws** have DRRP data (Responsibilities, Powers) without any Duties — `duty_holder` is empty even though enrichment completed successfully. These were miscounted as "no DRRP".

**Fix**: Changed check to `WHERE duty_type IS NULL` (set by enrichment regardless of which DRRP types are found). Also fixed the post-enrichment count query.

### Actual Gap Breakdown (193 total)

| Category | Count | Description |
|----------|-------|-------------|
| **False gap: enriched, no Duty type** | 108 | Have Responsibility/Power but no Duty — `duty_holder` check missed them |
| **Missing LRT: no DuckDB row** | 47 | In LanceDB but no DuckDB record (need LRT sync from sertantai) |
| **Blocked by panic bug** | 5 | Now enriched successfully after fix |
| **Genuinely no DRRP** | 33 | Non-regulatory: amendments, safety zones, commencement orders, revoked text |

### The 33 Genuinely No-DRRP Laws

Nearly all are tiny (2-6 body paragraphs) and non-regulatory:
- **8 Safety Zones Orders** — designate zones around offshore installations (2-3 paras)
- **10 pure amendment SIs** — modify other legislation, no standalone duties
- **3 commencement orders** — bring sections of other Acts into force
- **4 revocation/designation orders** — administrative instruments
- **3 climate change budget/levy orders** — set numerical targets, no duties
- **5 other** — includes UK_uksi_2009_716 (Chemicals CHIP, 18 of 25 provisions revoked/dots)

These are correctly excluded from DRRP — they don't create duties, rights, responsibilities, or powers.

## Corrected Numbers (post-fix)

| Metric | Before | After |
|--------|--------|-------|
| LanceDB laws | 464 | 464 |
| DuckDB laws with any DRRP | 271 (duty_holder only) | **384** (duty_type) |
| DuckDB laws with duty_holder | 271 | 276 |
| True enrichment gap | ~193 | **33** (non-regulatory) + **47** (missing LRT) |
| Enrichment coverage | 60% | **83%** of LanceDB laws (384/464) |

## Changes Made

### `crates/fractalaw-core/src/taxa/clause_structure.rs`
- `find_modal()`: Added bounds + char-boundary check before using `MatchSpan` on clause substring. Falls back to regex when out of bounds.

### `crates/fractalaw-cli/src/main.rs`
- `cmd_taxa_enrich()` skip check: `duty_holder IS NULL OR len(duty_holder) = 0` → `duty_type IS NULL`
- Post-enrichment count: `duty_holder IS NOT NULL AND len(duty_holder) > 0` → `duty_type IS NOT NULL`

## Progress

- [x] Query DuckDB for the 193 gap laws
- [x] Jurisdiction/type/year breakdown
- [x] Sample 20-30 laws, inspect provision text
- [x] Categorize causes (2 bugs found + 3 real categories)
- [x] Quantify each category
- [x] Fix panic bug in clause_structure.rs
- [x] Fix enrichment skip check (duty_holder → duty_type)
- [x] Re-enrich 39 previously-blocked laws
- [ ] File follow-up issue for 47 missing LRT records

## Recommendations

1. **#17 can be closed** — the gap is understood and mostly resolved by the two bug fixes
2. **New issue needed**: 47 LanceDB laws with no DuckDB row — need LRT sync from sertantai (`query_lrt` + `upsert_legislation`)
3. **#22 and #23 are still valid** but less urgent — the main gap was bugs, not pattern coverage. The 33 genuinely no-DRRP laws are correctly excluded.
4. **Next priority should shift** to either:
   - The 47 missing-LRT sync issue (data completeness)
   - #15 (Taxa QA report) for ongoing validation
   - #22/#23 only if p-dimension coverage is the bottleneck

## Status: **Done**
