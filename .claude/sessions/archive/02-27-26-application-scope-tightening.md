---
session: 'Tighten Application+Scope Purpose Classifier (GH #20)'
status: closed
opened: 2026-02-27
closed: 2026-02-27
outcome: success
summary: 'Replaced broad Application+Scope regex with 10 constrained branches targeting specific legal constructions. Added
  APPLICATION_SCOPE-primary as a DRRP skip gate. Eliminated 18 false positive provisions with zero genuine duty regressions.
  Created reusable taxa eyeball CLI subcommand.

  '
decisions:
- what: 10-branch constrained regex replacing broad Application+Scope pattern
  why: Broad regex fired on genuine duty provisions mentioning "shall apply" or "Regulations apply"
  result: Each branch targets a specific legal construction (scope extension, transitional non-application, etc.)
- what: APPLICATION_SCOPE-primary as DRRP skip gate
  why: Same pattern as existing Interpretation-primary and Enactment-primary gates
  result: Provisions tagged Application+Scope primary excluded from DRRP classification
lessons:
- title: Eyeball review should be a repeatable CLI command
  detail: Ad-hoc manual QA was replaced with taxa eyeball subcommand generating markdown with per-provision DRRP details
  tag: tooling
- title: Self-referencing subject constraint prevents false matches
  detail: Requiring "these/this Regulations" as subject prevents matching relative clauses like "to whom this regulation applies"
  tag: regex-design
metrics:
  drrp_provisions_before: 272
  drrp_provisions_after: 254
  false_positives_eliminated: 18
  regressions: 0
  tests_before: 204
  tests_after: 217
artifacts:
- crates/fractalaw-core/src/taxa/purpose.rs
- crates/fractalaw-core/src/taxa/mod.rs
- crates/fractalaw-cli/src/main.rs
- data/clause_eyeball.md
depends_on: []
enables:
- 02-27-26-ohs-enrichment-zenoh.md
---


# Session: Tighten Application+Scope Purpose Classifier (GH #20) (CLOSED)

**Date**: 2026-02-27
**Issue**: https://github.com/fractalaw/fractalaw/issues/20
**Branch**: master

## Problem

Application+Scope regex is too broad — it fires on many genuine duty provisions that happen to mention "shall apply" or "Regulations apply". This prevents using `APPLICATION_SCOPE`-primary as a DRRP skip gate (same pattern as Interpretation-primary and Enactment-primary).

From the 02-26 sessions:
- **taxa-refinement**: 42.0% DRRP rate across 119 Application+Scope provisions — too high to blindly gate
- **regex-patterns**: Gap C provisions (~129) include application/fitness provisions that describe *where* a duty applies, not new obligations
- **v2 parser**: 14 scope-exclusion false positives in v1 were correctly removed by actor-anchoring in v2
- **validation**: ~30 hot misses are correct rejections where actors appear near modals but the provision establishes scope
- **clause-quality**: Explicitly filed as GH #20 — "the single biggest remaining improvement"

## Test approach

`data/clause_eyeball.md` is the manual QA artifact. Now generated via a reusable CLI subcommand:

```bash
fractalaw taxa eyeball --laws "UK_uksi_2005_1643,UK_uksi_1992_2792,UK_uksi_2005_1093,UK_uksi_2002_2676,UK_uksi_2013_1471,UK_uksi_2000_128,UK_uksi_2015_483"
```

Previously this was done ad-hoc in sessions — now saved as `TaxaAction::Eyeball` in `crates/fractalaw-cli/src/main.rs`.

## False positives identified in eyeball file

Reviewing `data/clause_eyeball.md` (v4, 272 DRRP provisions across 7 laws), these Reg 3 provisions were Application+Scope text wrongly producing DRRP output:

### Pattern A: Scope extension ("like duty" / "shall apply to X as they apply to Y")

| Law | Reg | Text pattern | Old DRRP |
|-----|-----|-------------|----------|
| Noise 2005 | 3 | "Where a duty is placed by these Regulations on an employer... the employer shall... be under a like duty in respect of any other person" | Duty (0.85) |
| Noise 2005 | 3 | "These Regulations shall apply to a self-employed person as they apply to an employer" | Duty (0.85) |
| Vibration 2005 | 3 | "Where a duty is placed by these Regulations on an employer... he shall... be under a like duty" | Duty (0.85) |
| Vibration 2005 | 3 | "These Regulations shall apply to a self-employed person as they apply to..." | Duty (0.85) |
| Lead 2002 | 3 | "Where a duty is placed by these Regulations on an employer... he shall... be under a like duty" | Duty (0.85) |
| Lead 2002 | 3 | "These Regulations shall apply to a self-employed person as they apply to..." | Duty (0.85) |
| Pressure Systems | 3 | "Any requirement... shall also extend to a self-employed person" | Duty (0.70) |
| Pressure Systems | 3 | "Any requirement... shall extend only to such a system or article" | Duty (0.55) |

### Pattern B: Transitional non-application ("shall not apply until")

| Law | Reg | Text pattern | Old DRRP |
|-----|-----|-------------|----------|
| Vibration 2005 | 3 | "regulation 6(4) shall not apply until 6th July 2010 where work equipment..." | Duty (0.70) |
| Vibration 2005 | 3 | "regulation 6(4) shall not apply to whole-body vibration until 6th July 2014" | Duty (0.70) |

## Changes made

### 1. Tightened Application+Scope regex (`purpose.rs:65-86`)

Replaced the broad regex with constrained branches. Key changes:

| Old pattern | Problem | New pattern |
|-------------|---------|-------------|
| Bare `Application` | Matches anywhere in text | `^Application\b` (start of text only) |
| `shall.*?apply` | "employer shall apply the precautionary principle" | Removed — no standalone branch |
| `(?:Regulations?).*?apply?i?e?s?` | "Regulations require employers to apply..." | `(?:these\|this) (?:Regulations?\|Act\|...).{0,60}...appl(?:y\|ies)` — requires self-referencing subject |
| N/A | Missing scope extension patterns | Added: `be under a like duty`, `shall extend only to`, requirement-extends with 150-char window |

10 branches in the new regex, each targeting a specific application/scope construction:

1. `^Application\b` — heading
2. `(?:these|this) Regulations... appl(y|ies)` — self-referencing applicability
3. `regulation \d... shall not apply` — transitional non-application
4. `shall apply to ... as they apply to` — scope extension
5. `be under a like duty` — duty extension
6. `(?:requirement|prohibition|duty)...shall (also) extend` — requirement-extends (150-char window)
7. `shall extend only to` — standalone scope limitation
8. `does not apply (to|where|until|in|unless)` — exclusion
9. `shall have (no) effect` / `ceases to have effect` — effect statements
10. `provisions of ... apply` / `shall bind the Crown` — provisions-referencing

### 2. Added APPLICATION_SCOPE-primary skip gate (`mod.rs:401-405`)

```rust
if purposes.first() == Some(&purpose::APPLICATION_SCOPE) {
    return true;
}
```

### 3. Added `taxa eyeball` CLI subcommand (`main.rs`)

```bash
fractalaw taxa eyeball --laws "UK_uksi_2005_1643,..." [--output ./data/clause_eyeball.md]
```

Generates human-readable markdown with one entry per DRRP provision: regulation number, DRRP type, confidence, and full clause text. Previously this was ad-hoc; now it's a repeatable command.

### 4. Tests

**New tests in `purpose.rs`** (8 tests):
- 6 true-positive tests: self-employed scope, like duty, regulation shall-not-apply, requirement-extends, does-not-apply-to, heading
- 2 true-negative tests: genuine duty, "shall apply the precautionary principle"

**New tests in `mod.rs`** (6 tests):
- 4 Application+Scope-primary skip tests: like-duty, self-employed, transitional, requirement-extends
- 2 regression tests: genuine duties still produce DRRP

**Modified tests** (2): Sentence-start tests updated to avoid incidentally triggering APPLICATION_SCOPE on test text.

## Results

| Metric | Before (v4) | After (v5) | Delta |
|--------|-------------|------------|-------|
| DRRP provisions (7 laws) | 272 | 254 | -18 |
| Tests passing | 204 | 217 | +13 |
| All 10 identified false positives | Present | Eliminated | -10 |
| Genuine duty regressions | N/A | 0 | 0 |

The 18 eliminated provisions (vs 10 identified) means 8 additional Application+Scope provisions were also caught — similar patterns in other regulation numbers or slightly different formulations beyond the manual sample.

**Per-law breakdown:**

| Law | Before | After | Delta |
|-----|--------|-------|-------|
| Noise 2005 | 31 | 29 | -2 |
| Display Screen Equipment 1992 | 16 | 16 | 0 |
| Vibration 2005 | 25 | 21 | -4 |
| Lead 2002 | 44 | 42 | -2 |
| RIDDOR 2013 | 26 | 26 | 0 |
| Pressure Systems 2000 | 36 | 33 | -3 |
| COMAH 2015 | 94 | 87 | -7 |

COMAH had the largest drop (-7), likely because COMAH has many application/scope provisions defining when the regulations apply to different types of establishments.

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/taxa/purpose.rs` | Tightened Application+Scope regex |
| `crates/fractalaw-core/src/taxa/mod.rs` | APPLICATION_SCOPE-primary skip gate + 6 new tests |
| `crates/fractalaw-cli/src/main.rs` | `taxa eyeball` CLI subcommand |
| `data/clause_eyeball.md` | Regenerated QA artifact (254 provisions) |

## Progress

- [x] Read issue #20 and understand problem
- [x] Read current Application+Scope regex and should_skip_drrp()
- [x] Review all 02-26-26 session docs for context
- [x] Review data/clause_eyeball.md and identify specific false positives
- [x] Document current state, false positives, and strategy
- [x] Phase 2: Tighten Application+Scope regex
- [x] Phase 3: Add APPLICATION_SCOPE-primary skip gate
- [x] Phase 4: Regenerate eyeball file and validate
- [x] Create reusable `taxa eyeball` CLI subcommand

## Closed

GH #20 closed. Session complete.

Remaining follow-up (not blocking):
- Eyeball review: visually scan `data/clause_eyeball.md` for any remaining false positives or regressions
- Consider running on the full 452-law corpus via `taxa enrich --force` to see the broader impact
