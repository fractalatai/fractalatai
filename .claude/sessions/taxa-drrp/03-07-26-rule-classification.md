# Session: Rule Classification with Actor Back-Linking (#16)

**Date**: 2026-03-07
**Issue**: [#16 — Add 'Rule' classification for thing-subject obligations](https://github.com/fractalaw/fractalaw/issues/16)
**Depends on**: #22 (closed), #26 (closed)

## Problem

~8% of obligation-bearing provisions have modal verbs (must/shall) but the grammatical subject is a THING, not a person. These fall through all four classification tiers to Unknown (empty `duty_types`), making them invisible in DRRP output.

Examples: "Traffic routes **must** be suitable...", "Washing facilities **must** be provided...", "A cofferdam **must** be of suitable design..."

## Phase 1: Rule Detection

### Changes

#### `crates/fractalaw-core/src/taxa/duty_type.rs`

- Added `Rule` variant to `DutyType` enum
- Updated `as_str()` → `"Rule"`, `priority()` → `5`
- Added Rule as Tier 4 in `classify()` cascade (after Government v2, before Unknown)
- Added `DutyFamily::Rule => vec![DutyType::Rule]` in `map_to_duty_type()`
- 2 new integration tests: `classify_thing_subject_as_rule`, `person_takes_precedence_over_thing`

#### `crates/fractalaw-core/src/taxa/duty_patterns.rs`

- Added `Rule` variant to `DutyFamily` enum
- Added `ThingObligation` variant to `DutySubType` enum

#### `crates/fractalaw-core/src/taxa/duty_patterns_rule.rs` (NEW)

Thing-subject obligation matcher:
- **THING_KEYWORDS**: 45 inanimate subject keywords (arrangements, routes, exits, equipment, systems, measures, rooms, facilities, site, structure, vessel, cofferdam, scaffolding, workplace, premises, etc.)
- **Detection**: Find modal (must/shall) → look backwards 80 chars for thing keyword → verify no person keyword closer to modal
- **PERSON_KEYWORDS**: 29 negative guard keywords (employer, employee, contractor, secretary of state, etc.)
- **Output**: `DutyClassification { family: Rule, sub_type: ThingObligation, confidence: 0.55 }`
- 10 unit tests

#### `crates/fractalaw-core/src/taxa/mod.rs`

- Registered `pub mod duty_patterns_rule;`

#### `crates/fractalaw-cli/src/main.rs`

- Added `DutyType::Rule` match arm → maps to `duties`/`duty_holders` (same as Duty)

## Phase 2: Actor Back-Linking

### Changes

#### `crates/fractalaw-cli/src/main.rs`

After the main provision loop, added a second pass:

1. Count governed actor frequency across all non-Rule DRRP entries
2. Find the most frequent governed actor for the law
3. Replace `"Unknown"` holders in Rule entries with `"{actor} (inferred)"`

Edge cases:
- Laws with no governed actors: Rule holders stay "Unknown"
- Single-actor laws: dominant actor assigned (works perfectly)
- Multi-actor laws: most frequent actor gets assigned (useful default)

## Results

### CDM 2015 (UK_uksi_2015_51)

- **26 Rule provisions detected** — welfare facilities, washing facilities, changing rooms, sanitary conveniences, drinking water, lighting
- DRRP provisions: 63 → 89 (+26, +41% improvement)

### OH&S Corpus (451 laws, 9,608 provisions)

- DRRP%: 34.4% (includes new Rule provisions)
- 361 tests pass (12 new)
- Fitness audit unchanged: 99.0% Polarity, 79.6% Tagged

## Key Files

| File | Change |
|------|--------|
| `crates/fractalaw-core/src/taxa/duty_type.rs` | `Rule` variant, classify cascade Tier 4, `map_to_duty_type()` |
| `crates/fractalaw-core/src/taxa/duty_patterns.rs` | `DutyFamily::Rule`, `DutySubType::ThingObligation` |
| `crates/fractalaw-core/src/taxa/duty_patterns_rule.rs` | **NEW**: thing-subject + modal matcher (45 keywords, 80-char window) |
| `crates/fractalaw-core/src/taxa/mod.rs` | Module registration |
| `crates/fractalaw-cli/src/main.rs` | `DutyType::Rule` match arm + actor back-linking pass |

## Status: **Complete** — 361 tests pass, CDM 2015 verified

## Note

LanceDB schema needs fitness columns added before `taxa enrich` can write — pre-existing issue from #22/#26 fitness work, not related to Rule changes.
