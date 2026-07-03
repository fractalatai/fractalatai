---
session: 'DRRP Parser v2: Actor-Anchored Classification'
status: closed
opened: 2026-02-26
closed: 2026-02-26
outcome: success
summary: 'Replaced the v1 blunt boolean gate (has_actor AND has_obligation) with actor-anchored regex patterns requiring the
  actor keyword to appear before the modal verb within a 120-char window. Eliminated 33 false positives (scope exclusions,
  thing-subject rules, saving clauses) while retaining 92.8% of v1 matches across 7 ESH laws.

  '
decisions:
- what: Actor-anchored patterns with 120-char forward window
  why: Disconnected actor+obligation checks produced false positives when actor was object, not subject
  result: 33 v1-only false positives removed; 4 v2-only new detections (2 legitimate)
- what: Reverse anchor for HSWA "shall be the duty of" formulation
  why: HSWA uses inverted syntax where modal precedes actor; needs backward 40-char window
  result: HSWA s.2/s.3/s.4 duties correctly classified
- what: '"person" requires compound predicates (person who/must/shall)'
  why: Bare "person" is too frequent; 62% false positive rate. Compound form is specific enough
  result: 3 CDM prohibition provisions correctly classified with zero false positives
- what: Preserve v1 code for side-by-side comparison
  why: Both parsers run simultaneously via parse_compare(); validate v2 before retiring v1
  result: taxa show --compare flag with diff display and summary stats
lessons:
- title: Syntactic anchoring dramatically reduces false positives
  detail: Actor-as-object provisions ("apply to the employer", "require the employer") correctly rejected by position check
  tag: regex
- title: Window size P90=132 chars covers 88.6% of actor-to-modal distances
  detail: Measured empirically across 237 provisions in 7 ESH laws; 200-char extended window adds 5.5%
  tag: empirical
metrics:
  v1_matches: 405
  v2_matches: 376
  false_positives_removed: 33
  new_detections: 4
  retention_rate: 92.8%
  tests_passing: 197
  new_tests: 35
artifacts:
- crates/fractalaw-core/src/taxa/duty_patterns_v2.rs
- crates/fractalaw-core/src/taxa/duty_type.rs
- crates/fractalaw-core/src/taxa/actors.rs
- crates/fractalaw-core/src/taxa/mod.rs
- crates/fractalaw-cli/src/main.rs
depends_on:
- 02-26-26-taxa-regex-patterns
enables:
- 02-26-26-v2-promotion-enrichment
---


# Session: 2026-02-26 — DRRP Parser v2: Actor-Anchored Classification (CLOSED)

**Parent session**: [02-26-26-taxa-regex-patterns.md](02-26-26-taxa-regex-patterns.md)
**Status**: Complete
**Commit**: `673d578` — "Implement v2 actor-anchored DRRP parser with comparison harness"

## Problem

The v1 DRRP parser has a fundamental design flaw: actor extraction (`actors.rs`) and pattern matching (`duty_patterns.rs`) are disconnected. Two independent boolean checks — `has_governed_actor(text) && has_obligation(text)` — mean "The contractor must ensure" and "information must be provided to the contractor" both classify as Duty equally. This produces false positives whenever an actor keyword appears in a provision but isn't the grammatical subject of the modal verb.

Categories of false positives in v1:
- **Scope exclusions**: "These Regulations shall not apply to the employer..." (14 cases)
- **Thing-subject rules**: "The measures shall include..." (8 cases)
- **Saving clauses**: "Nothing in paragraph (2) shall require the employer..." (5 cases)
- **Modifier clauses**: "Section 3(2) shall be modified..." (4 cases)
- **Definitional/context**: actor mentioned but not as duty-holder (2 cases)

## Solution: Actor-Anchored Patterns

v2 replaces the blunt gate with syntactic anchoring: the actor keyword must appear **before** the modal verb within a character-distance window. For each extracted actor, v2 dynamically builds regex patterns like `{actor}.{0,120}{modal}{obligation_language}`.

### Window Size (Empirical)

Measured actor-to-modal distances across 237 provisions in 7 ESH laws:

| Threshold | Coverage |
|-----------|----------|
| 120 chars | 88.6% (primary window) |
| 200 chars | 94.1% (extended, −0.15 confidence) |

P50=19, P75=59, P90=132.

### Pattern Types

1. **Forward anchor**: `{actor}.{0,120}{modal}` — standard subject-before-verb
2. **Reverse anchor**: `shall be the duty of.{0,40}{actor}` — HSWA "It shall be the duty of every employer" formulation
3. **Person compound predicates**: bare "person" is too broad, so requires qualifying phrases:
   - "a person who/must/shall", "every/no/any person who"
   - "the duty of every/any person" (reverse anchor for HSWA)
4. **Definitional exclusion**: "shall be regarded as" / "shall be treated as" rejected
5. **Sub-type ordering**: Prohibition > SFAIRP > RiskAssessment > GeneralDuty > Information > Training > Prescriptive > Enabling (most specific first)

## Implementation

### Files Changed

| File | Change |
|------|--------|
| `crates/fractalaw-core/src/taxa/actors.rs` | `ActorMatch { label, keyword, offset }` struct; `run_patterns()` captures keyword via `regex.find()`; backward-compat `governed_labels()` / `government_labels()` accessors |
| `crates/fractalaw-core/src/taxa/duty_patterns_v2.rs` | **New** (~710 lines) — actor-anchored matcher with regex cache, 28 tests |
| `crates/fractalaw-core/src/taxa/duty_type.rs` | `classify_v2()` — same gov tiers, governed tier uses v2 anchored patterns; 3 tests |
| `crates/fractalaw-core/src/taxa/mod.rs` | `parse_v2()`, `parse_compare()`, `CompareRecord` struct; 4 tests |
| `crates/fractalaw-cli/src/main.rs` | `taxa show --compare` flag with side-by-side diff display and summary stats |

v1 code is preserved — both run side-by-side. `GOVERNED_ACTORS` not yet deleted (deferred until v2 replaces v1 in `taxa enrich`).

### Test Coverage

197 taxa tests pass (35 new + 162 existing):
- 28 in `duty_patterns_v2` (forward/reverse anchors, person compounds, window boundaries, actor-as-object rejection)
- 3 in `duty_type` (classify_v2 employer, actor-as-object, government unchanged)
- 4 in `mod` (parse_v2, parse_compare agreement/difference detection)

## Validation Results (7-Law Sample)

```
Law                    v1   v2   Diffs  v1-only  v2-only
─────────────────────  ───  ───  ─────  ───────  ───────
UK_ukpga_1974_37       100   98      4        3        1
UK_uksi_1999_3242       53   44     12       10        1
UK_uksi_2015_51         72   70      5        3        1
UK_uksi_1998_2306       60   59      1        1        0
UK_uksi_1992_2793       10    7      3        3        0
UK_uksi_1998_2307       23   22      1        1        0
UK_uksi_2002_2677       87   76     15       12        1
─────────────────────  ───  ───  ─────  ───────  ───────
TOTALS                 405  376     41       33        4
```

- **33 v1-only**: all genuine false positives correctly removed by v2
- **4 v2-only**: ~2 legitimate, ~2 false positive (minor edge cases)
- **Retention**: 92.8% of v1 matches preserved

## Next Steps

- **Switch `taxa enrich` to v2**: replace `taxa::parse()` with `taxa::parse_v2()` in the enrichment pipeline once validated at scale
- **Delete `GOVERNED_ACTORS`**: remove the blunt gate list once v1 is fully retired
- **Government tier anchoring**: low priority — gov patterns already embed actor keywords
- **GH Issue #16 (Rules)**: 42 thing-subject provisions ("steps must be taken") need a separate "Rule" classifier — orthogonal to actor-anchored work
