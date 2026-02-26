# Session: 2026-02-26 — Taxa Regex Pattern Improvement

## Context

**Parent session**: [02-26-26-taxa-refinement.md](02-26-26-taxa-refinement.md)
**Objective**: Iteratively improve the taxa regex patterns to reduce DRRP classification miss rate across UK ESH laws.
**Approach**: Test-driven, one pattern at a time, measure after each iteration.

## Baseline (7 UK ESH Laws, Post-Enrichment)

| Law | Provisions | DRRP | DRRP % | Modal | Miss | Miss % |
|-----|-----------|------|--------|-------|------|--------|
| HSWA 1974 | 234 | 97 | 41% | 109 | 44 | 40% |
| Electricity at Work 1989 | 85 | 8 | 9% | 30 | 23 | 77% |
| MHSWR 1999 | 127 | 44 | 35% | 71 | 28 | 39% |
| CDM 2015 | 282 | 53 | 19% | 159 | 108 | 68% |
| Fire Safety Order 2005 | 500 | 152 | 30% | 94 | 8 | 9% |
| PUWER 1998 | 192 | 60 | 31% | 86 | 29 | 34% |
| Manual Handling 1992 | 35 | 10 | 29% | 9 | 3 | 33% |
| **TOTAL** | **1,455** | **424** | **29.1%** | **558** | **243** | **43.5%** |

"Miss" = provision has a modal verb (shall/must/is required to) but no DRRP classification.

## Gap Taxonomy (243 Misses)

### Gap A — Actor extracted, no DRRP pattern match (96 provisions)

Actors.rs extracts the actor, but the actor label isn't in `duty_patterns.rs` `GOVERNED_ACTORS` list, so `has_governed_actor()` returns false and all governed duty patterns fail.

**Top missing actor labels** (extracted by actors.rs but not gated by duty_patterns.rs):

| Actor Label | Count | In `GOVERNED_ACTORS`? |
|-------------|-------|-----------------------|
| Ind: Person | 62 | No (only "person who", "every person", "no person") |
| Ind: Worker | 18 | No |
| SC: C: Contractor | 15 | No |
| SC: C: Principal Contractor | 12 | No |
| SC: Client | 9 | No |
| Gvt: Agency | 7 | No (government, not governed) |
| Org: Company | 2 | No |

**Root cause**: Two parallel actor systems with different coverage:
- `actors.rs` — 30+ governed actor patterns with regex boundary matching, returns structured labels
- `duty_patterns.rs` `GOVERNED_ACTORS` — 14-entry flat substring list used to gate DRRP patterns

**Concentrated in CDM 2015** (61/96) which uses "contractor", "client", "principal contractor", "worker" — none in the GOVERNED_ACTORS list.

### Gap B — Actor keyword in text but actors.rs misses it (27 provisions)

Known actor keywords present in text but actors.rs regex boundary patterns (`[\s[:punct:]]` on both sides) fail to match. Mostly bare "person" in contexts like "a person shall not disclose" where the substring "person" is present but actors.rs needs "person who" / "every person" / "no person" patterns.

### Gap C — Truly actor-less obligations (129 provisions)

Text has obligation modal ("shall"/"must") but subject is a thing, not a person/org:
- "All systems shall..." (Electricity at Work)
- "Equipment must be..." (various)
- "These Regulations shall apply to..." (application/fitness provisions)
- "Sanitary conveniences must be provided..." (CDM 2015)

Many of these are **application/fitness provisions** — they specify where a more general duty applies, not a new obligation. In sertantai these were called "Fitness" provisions.

## Strategy: Test-Driven Iteration

### Principles

1. **Test-first**: Every pattern change starts with a failing test using real provision text
2. **Regression guard**: Full test suite must pass before and after every change — no silent regressions
3. **Small, targeted changes**: One pattern at a time. Each addresses a specific, named gap
4. **Beware broad patterns**: "person" is too frequent — will create false positives. The existing predicates ("person who", "every person", "no person") are intentionally specific
5. **Confidence scoring**: New patterns get lower confidence so AI polisher knows where to focus
6. **Measure after each iteration**: Re-enrich, compare miss rate, stop at diminishing returns

### Iteration Cycle

```
1.  Pick highest-frequency miss pattern from the data
1b. BEFORE changing anything: audit existing test coverage around the
    area you're about to touch.  Add "true negative" regression tests
    for provisions that correctly get NO DRRP today — so you'll see
    breakages from the upcoming change before they ship.
2.  Write test(s) using real provision text for the NEW pattern (failing)
3.  Implement minimal regex change to pass the new test(s)
4.  Run full suite — new tests pass, zero regressions (including 1b tests)
5.  Re-enrich, measure improvement
6.  Repeat
```

Step 1b is critical: the foundation test suite is mostly happy-path. Before we add "contractor" to `GOVERNED_ACTORS`, we need tests confirming that provisions mentioning "contractor" in a non-duty context (e.g., application/scope, interpretation, procedural cross-references) correctly return no DRRP. Without those, a regression is invisible.

### Planned Iterations

**Iteration 1 — Gap A: Expand `GOVERNED_ACTORS` (targeted)**

Add specific, high-frequency actor keywords that actors.rs already recognises. NOT "person" (too broad). Candidates:
- "contractor" (15 hits) — safe, specific to construction/CDM
- "worker" (18 hits) — safe, specific to employment law
- "client" (9 hits) — safe, specific to CDM/construction
- "principal contractor" (12 hits) — safe, very specific
- "operator" — safe, specific to plant/equipment regulations
- "competent person" — safe, specific role in ESH law
- "owner" — moderately safe, specific to property/equipment

Each addition gets its own test with real provision text. One at a time, measure regression.

**Iteration 2 — Gap A: Government actor gaps**

"Gvt: Agency" (7 hits) — actors.rs extracts it but government patterns may not fire. Investigate and fix if needed.

**Iteration 3 — Gap B: Actor boundary matching**

Fix actors.rs boundary patterns that miss valid actors. May need to relax `[\s[:punct:]]` to also match start-of-string.

**Iteration 4 — Gap C: Actor-less obligations (careful)**

This is the hardest gap. Options:
- Add a low-confidence fallback in `match_governed()` for obligation modals without any actor
- Only fire if text is NOT an application/fitness provision (need negative pattern)
- May be better left to AI polisher rather than regex

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/taxa/duty_patterns.rs` | DRRP pattern matching, `GOVERNED_ACTORS` list, `has_governed_actor()` |
| `crates/fractalaw-core/src/taxa/duty_type.rs` | Orchestrates pattern tiers (gov_v1 → gov_v2 → governed → empty) |
| `crates/fractalaw-core/src/taxa/actors.rs` | Actor extraction with regex boundary matching, 30+ governed patterns |
| `crates/fractalaw-core/src/taxa/mod.rs` | Pipeline: clean → purpose → [gate] → actors → DRRP → POPIMAR |
| `crates/fractalaw-core/src/taxa/purpose.rs` | Purpose classification (Interpretation, Amendment, etc.) |

## Iteration Log

### Iteration 0 — Baseline Established

- Enriched all 7 ESH laws (3 were previously un-enriched)
- Measured: 243 misses out of 558 modal provisions (43.5%)
- Categorised into Gap A (96), Gap B (27), Gap C (129)
- Identified that "person" is too broad for GOVERNED_ACTORS — learned the hard way
- Current test suite: 132 taxa tests, all passing

### Iteration 1 — Add "contractor" to GOVERNED_ACTORS

**Target**: Gap A — 27 provisions (15 contractor + 12 principal contractor) in CDM 2015.

**Step 1b (true-negative regression tests)**:
- `contractor_heading_no_match` — CDM heading, no modal
- `contractor_cross_reference_no_match` — CDM reg 7(2), no modal
- `contractor_interpretation_no_match` — CDM reg 8(1), transitional
- `contractor_numbered_list_item_no_match` — CDM schedule item
- `contractor_definition_no_drrp` — CDM reg 2 interpretation (gate-skipped)
- `contractor_appointment_cross_ref_no_drrp` — CDM reg 8(1) full pipeline

All 6 passed before the change.

**Step 2 (failing tests)**:
- `contractor_duty_plan_manage_monitor` — CDM reg 15(2)
- `principal_contractor_duty_construction_phase_plan` — CDM reg 12(1)
- `contractor_prohibition` — CDM reg 15(1)

All 3 failed as expected (empty duty_types).

**Step 3 (change)**: Added `"contractor"` to `GOVERNED_ACTORS` in `duty_patterns.rs`. Single line, substring match covers both "contractor" and "principal contractor".

**Step 4 (full suite)**: 141 passed, 0 failed.

**Step 5 (measurement)**:

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| CDM 2015 miss rate | 108/159 (68%) | 86/159 (54%) | -22 provisions |
| Overall miss rate | 243/558 (43.5%) | 221/558 (39.6%) | -22 provisions (9.1%) |

22 CDM provisions now correctly classified. Zero false positives introduced. Test suite grew from 132 to 141.

---

**Session started**: 2026-02-26
**Status**: Active — ready for Iteration 2
