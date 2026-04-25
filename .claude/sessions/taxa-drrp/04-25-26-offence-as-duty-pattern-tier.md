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

- [ ] Create `duty_patterns_offence.rs` with `match_offence_as_duty()`
- [ ] Add sub-pattern regexes for each offence-creating variant
- [ ] Add penalty exclusion heuristic
- [ ] Add actor extraction from "for X to" and "X commits" patterns
- [ ] Wire into `duty_type.rs::classify()` as tier 4
- [ ] Add true-positive tests using real provision text from Firearms Act, Dogs NI, OSA
- [ ] Add true-negative tests for penalty/sentencing provisions
- [ ] Run full taxa test suite
- [ ] Re-enrich affected families, measure corpus-wide improvement
- [ ] Update gap analysis session docs with results

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/taxa/duty_patterns_offence.rs` | NEW — offence-as-duty matcher |
| `crates/fractalaw-core/src/taxa/duty_type.rs` | Wire in new tier |
| `crates/fractalaw-core/src/taxa/mod.rs` | Module declaration |
| `crates/fractalaw-core/src/taxa/duty_patterns.rs` | Reference — existing pattern structure |
| `crates/fractalaw-core/src/taxa/duty_patterns_v2.rs` | Reference — actor-anchored patterns |

---

**Session status**: Open. Ready to implement.
