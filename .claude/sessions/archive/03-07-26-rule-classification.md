---
session: Rule Classification with Actor Back-Linking (#16)
status: closed
opened: 2026-03-07
closed: 2026-03-07
outcome: success
summary: 'Added Rule as Tier 4 in the classify cascade for thing-subject obligations (~8% of provisions). Implemented duty_patterns_rule.rs
  with 45 thing keywords and 29 person guard keywords. Actor back-linking infers the dominant governed actor for Rule provisions.
  CDM 2015 gained 26 Rule provisions (+41%). Also migrated 7 fitness columns onto LanceDB schema.

  '
decisions:
- what: Rule as Tier 4 after Government v2 in classify cascade
  why: Thing-subject obligations ("Traffic routes must be suitable") have modal verbs but no person subject, falling through
    to Unknown
  result: Rule type with ThingObligation sub-type at confidence 0.55
- what: Actor back-linking via frequency-based inference
  why: Rule provisions have no actor subject but the law typically has a dominant governed actor from non-Rule DRRP entries
  result: Most frequent governed actor assigned as "{actor} (inferred)" to Rule holder fields
lessons:
- title: Thing-keyword negative guard prevents person provisions from being misclassified as Rule
  detail: 29 person keywords checked closer to modal than the thing keyword, ensuring person-subject provisions stay in earlier
    tiers
  tag: parser-design
- title: LanceDB schema migration requires add_columns() before enrichment
  detail: Fitness columns added to schema.rs but not to the live LanceDB table caused merge_insert to fail with schema mismatch
  tag: migration
metrics:
  thing_keywords: 45
  person_guard_keywords: 29
  cdm_rule_provisions: 26
  cdm_drrp_improvement_pct: 41
  tests_passing: 361
  tests_added: 12
artifacts:
- crates/fractalaw-core/src/taxa/duty_patterns_rule.rs
- crates/fractalaw-core/src/taxa/duty_type.rs
- crates/fractalaw-core/src/taxa/duty_patterns.rs
- crates/fractalaw-core/src/taxa/mod.rs
- crates/fractalaw-cli/src/main.rs
depends_on:
- 03-05-26-denormalize-fitness-lrt.md
enables: []
---


# Session: Rule Classification with Actor Back-Linking (#16) (CLOSED)

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

## Phase 3: LanceDB Fitness Column Migration

The fitness extraction code (#22/#26) writes 7 per-provision columns to LanceDB, but the table schema didn't have them yet. `taxa enrich` failed with:

```
merge_insert taxa: lance error: Append with different schema:
  unexpected=[fitness_polarity, fitness_property, fitness_place, fitness_sector,
              fitness_person, fitness_plant, fitness_process]
```

### Fix

Used LanceDB Python `add_columns()` API to add 7 `List<Utf8>` nullable columns initialized to null:

```python
fields = [pa.field(name, pa.list_(pa.utf8()), nullable=True)
          for name in ['fitness_polarity', 'fitness_person', 'fitness_process',
                       'fitness_place', 'fitness_plant', 'fitness_property',
                       'fitness_sector']]
table.add_columns(fields)  # version 1758
```

### Verification

- `taxa enrich --force --laws UK_uksi_2015_51` — succeeds, writes DRRP + fitness data
- DuckDB shows `duty_type = [Duty, Responsibility, Right, Rule]` with `Ind: Person (inferred)` in holders
- LanceDB fitness columns populated: 5 provisions with polarity data (AppliesTo/DisappliesTo)

## Status: **Complete** — 361 tests pass, enrichment pipeline end-to-end verified
