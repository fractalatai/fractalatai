# Session: 2026-04-25 — Offence-as-Duty Pattern Tier (#34)

## Context

**Issue**: fractalaw/fractalaw#34
**Discovery**: `.claude/sessions/taxa-drrp/taxa-gap-analysis/04-21-26-public-safety.md`
**Skill**: [taxa-gap-analysis/SKILL.md](../../skills/taxa-gap-analysis/SKILL.md)
**Objective**: Add a new pattern tier to the taxa DRRP pipeline that detects duties expressed as offence-creating language rather than modal verbs.

## Problem

The taxa pipeline requires a modal verb (shall/must/may) as the entry point for all DRRP pattern tiers. UK legislation frequently expresses duties as offence-creating language instead:

- "It is an offence for a person to X" = person must not X (Duty: Prohibitive)
- "A person commits an offence if..." = person must not do the thing (Duty: Prohibitive)
- "It shall be unlawful for any person to..." = prohibition duty
- "A person shall be guilty of an offence if..." = prohibition duty

These provisions have no standalone modal verb — the obligation is implicit in the offence language.

## Scope

**953 provisions corpus-wide** have no DRRP (76% miss rate on this pattern class). 922 of those have no modal verb at all — completely invisible to all current tiers.

| Pattern | Total | No DRRP | Miss% |
|---------|-------|---------|-------|
| "it is an offence for" | 209 | 195 | 93% |
| "commits an offence if" | 206 | 196 | 95% |
| "shall be guilty of an offence" | ~164 | ~164 | ~100% |
| "guilty of an offence if" | 65 | 48 | 74% |
| "unlawful for" | 8 | 7 | 88% |
| **Total (deduplicated)** | **1,241** | **953** | **76%** |

### Top Affected Laws

| Law | Misses | Family |
|-----|--------|--------|
| UK_ukpga_1981_69 (Wildlife & Countryside) | 102 | WILDLIFE & COUNTRYSIDE |
| UK_ukpga_1968_27 (Firearms Act) | 56 | PUBLIC |
| UK_ukpga_2023_50 (Online Safety Act) | 32 | PUBLIC |
| UK_ukpga_1990_43 (Environmental Protection) | 25 | ENVIRONMENTAL PROTECTION |
| UK_ukpga_2009_23 (Marine & Coastal Access) | 22 | MARINE & RIVERINE |
| UK_uksi_2017_1012 | 20 | |
| UK_asp_2003_8 | 18 | |

Cross-family impact — not limited to PUBLIC.

## Design

### DRRP Classification

These are **Duty (Prohibitive)** — the offence language is the enforcement mechanism, but the underlying obligation is a prohibition on the named actor.

### Where It Fits

New tier in `duty_type.rs::classify()`, after governed v2 and gov v1/v2 but before rule:

```
1. Governed v2 (actor-anchored)        ← existing
2. Government v1 (keyword-based)       ← existing
3. Government v2 (extended)            ← existing
4. Offence-as-duty (NEW)               ← this session
5. Rule (thing-subject)                ← existing
6. No match
```

### Pattern Matcher Design

New file: `duty_patterns_offence.rs`

```rust
/// Match provisions that express duties as offence-creating language.
///
/// Detects patterns like:
///   "it is an offence for [actor] to [action]"
///   "[actor] commits an offence if [condition]"
///   "it shall be unlawful for [actor] to [action]"
///
/// Returns Governed / Prohibitive classification.
pub fn match_offence_as_duty(text: &str) -> Option<DutyClassification> {
    // ...
}
```

Sub-patterns to detect:

1. **"it is an offence for X to Y"** — extract X as actor, Y as prohibited action
2. **"X commits an offence if Y"** — extract X as actor, Y as prohibited condition
3. **"X shall be guilty of an offence if Y"** — extract X, Y
4. **"it shall be unlawful for X to Y"** — extract X as actor
5. **"X is guilty of an offence under..."** — penalty provision, NOT a duty (true negative — must exclude)

### Critical Distinction: Duty vs Penalty

Not all offence provisions are duties. Key distinction:

- **Duty**: "It is an offence for a person to fail to comply" → person has a duty to comply
- **Penalty**: "A person guilty of an offence is liable to a fine" → sentencing provision, not a duty

The matcher must exclude pure penalty/sentencing provisions. Heuristic: provisions containing "liable to", "liable on conviction", "imprisonment", "fine" without an action clause are penalties.

### Actor Extraction

The offence patterns embed the actor differently from modal-based patterns:

- "for **a person** to contravene" — actor between "for" and "to"
- "**a person** commits an offence if" — actor before "commits"
- "**any person who** contravenes" — standard compound

Some provisions name specific actors: "for **a pawnbroker** to take in pawn", "for **the holder** to fail to surrender". The matcher should extract these for DRRP holder attribution.

## Implementation Plan

- [x] Create `duty_patterns_offence.rs` with `match_offence_as_duty()`
- [x] Add sub-pattern regexes for each offence-creating variant
- [x] Add penalty exclusion heuristic
- [x] Add actor extraction from "for X to" and "X commits" patterns (span captures)
- [x] Wire into `duty_type.rs::classify()` as tier 4 (before Rule)
- [x] Add true-positive tests (10 tests — Firearms, Dogs, OSA, pawnbroker, holder)
- [x] Add true-negative tests (5 tests — penalty/sentencing, mere reference, no offence language)
- [x] Add integration tests in `duty_type.rs` (5 tests — classify through full pipeline)
- [x] Run full taxa test suite — 341 passed, 0 failed
- [x] Re-enrich PUBLIC family, measure improvement
- [ ] Re-enrich corpus-wide to measure full impact (other families)

## Results (PUBLIC Family)

| Metric | Before | After (expanded heuristic) | Delta |
|--------|--------|---------------------------|-------|
| TP | 1,158 | 1,240 | +82 |
| Precision | 80.5% | 81.4% | +0.9pp |
| Recall | 59.1% | 60.9% | +1.8pp |
| F1 | 68.1% | 69.7% | +1.6pp |

82 new offence-as-duty provisions correctly classified in PUBLIC. Modest gain because most of the 953 corpus-wide offence provisions are in other families (Wildlife & Countryside 102, Environmental Protection 25, Marine 22).

**Note on ground truth heuristic**: The original heuristic uses modal verbs only. Offence-creating provisions have no modal — they appear as FP under the old heuristic (precision drops from 80.5% to 76.4%). The expanded heuristic (modal OR offence language) correctly treats them as expected positives (precision 81.4%).

### Patterns Implemented

| Pattern | Regex | Confidence |
|---------|-------|-----------|
| "it is/shall be an offence for" | `OFFENCE_FOR` | 0.70 |
| "commits an offence if" | `COMMITS_OFFENCE` | 0.70 |
| "shall be/is guilty of an offence if" | `GUILTY_IF` | 0.65 |
| "it is/shall be unlawful for" | `UNLAWFUL_FOR` | 0.70 |

### Penalty Exclusion

Two-layer exclusion:
1. `PENALTY_PRIMARY` — rejects "A person guilty of an offence is liable to..." at sentence start
2. `is_penalty_dominant()` — rejects when penalty language appears before the offence pattern

## Next Steps

- [ ] Re-enrich corpus-wide to measure full impact across all families
- [ ] Update `taxa-gap-analysis/SKILL.md` confusion matrix heuristic to include offence language

---

**Session status**: Implementation complete. Commit db0481c. Closes #34.

Corpus-wide re-enrichment deferred — can be done as a batch operation.
