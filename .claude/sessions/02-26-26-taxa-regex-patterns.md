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

### Iteration 2 — Assess "worker" for GOVERNED_ACTORS

**Target**: Gap A — 18 provisions with `Ind: Worker` actor label but no DRRP.

**Step 1b (audit)**:
- 37 provisions mention "worker" across 7 ESH laws
- 13 have modal + no DRRP (would be affected)
- Examined obligation subjects in all 13

**Finding: "worker" is almost always the object/beneficiary, NOT the duty-holder.**
- MHSWR reg 16A/17A/18A: "the **hirer** shall..." — hirer has duty toward agency worker
- CDM reg 4: "changing rooms **must be provided**...if a worker" — passive, worker is a condition
- CDM reg 6: "The **notice** must...be read by any worker" — notice is subject
- CDM reg 23: "A **cofferdam** must be...workers can gain shelter" — cofferdam is subject

Only 0/13 provisions have worker as the actual duty-holder. Adding "worker" would misattribute obligations — classifying these as "Governed / Prescriptive" when the duty is on the hirer, employer, or is an impersonal passive.

**Decision: SKIP.** "Worker" is a Gap C pattern (passive voice / actor-less), not Gap A (actor missing from list). No change made.

### Iteration 3 — Add "client" to GOVERNED_ACTORS

**Target**: Gap A — 9 provisions with `SC: Client` actor label but no DRRP.

**Step 1b (audit)**:
- 44 provisions mention "client" across 7 ESH laws (all CDM 2015)
- 27 already have DRRP (true positive — working)
- 4 have modal + no DRRP (would be affected)
- 13 no modal (unaffected)

All 4 affected provisions have client as the **subject** of the obligation:
- Reg 4(1): "A client **must** make suitable arrangements"
- Reg 4(3): "A client **must** ensure these arrangements are maintained"
- Reg 8(7): "duties...must be carried out" (domestic client context)
- Reg 16(3): "A domestic client...must comply"

No false-positive risk. "Client" is specific, no compound-word collisions.

**True-negative regression tests** (3 in duty_patterns, 1 in mod.rs): all passed before change.
**Failing true-positive tests** (2 in mod.rs): confirmed failing.

**Step 3 (change)**: Added `"client"` to `GOVERNED_ACTORS`.

**Step 4 (full suite)**: 147 passed, 0 failed.

**Step 5 (measurement)**:

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| CDM 2015 miss rate | 86/159 (54%) | 82/159 (52%) | -4 provisions |
| Overall miss rate | 221/558 (39.6%) | 217/558 (38.9%) | -4 provisions |
| Cumulative from baseline | 243 → 221 | 243 → 217 | 26 fixed (10.7%) |

Test suite: 141 → 147.

### Iteration 4 — Assess "competent person" for GOVERNED_ACTORS

**Target**: Gap A — provisions with `Ind: Competent Person` actor label but no DRRP.

**Step 1b (audit)**:
- 15 provisions mention "competent person" across 7 ESH laws
- 12 already have DRRP (true positive — working)
- 2 had modal + no DRRP, but 1 was fixed by re-enriching with current code
- 1 remaining miss: CDM reg 5(2) "The CDM co-ordinator must not...unless the worker is competent or under the supervision of a competent person"

**Finding**: The duty is on "CDM co-ordinator", not the competent person. Same false-attribution problem as "worker". And only 1 provision would benefit.

**Bonus discovery**: Re-enriching MHSWR and CDM with the current code (contractor + client) fixed 6 additional provisions that the previous enrichment had missed — overall 217 → 211.

**Decision: SKIP.** Only 1 remaining miss, and duty is misattributed. Not worth the risk.

**Running totals**: 243 → 211 misses (43.5% → 37.8%). 32 fixed (13.2%). Test suite: 147.

### Iteration 5 — Add "a person must" to GOVERNED_ACTORS

**Target**: Gap A — provisions where "a person must" is the grammatical subject of a prohibition/obligation.

**Step 1 (audit)**:
- "a person" appears in 59 provisions across 7 laws (not matched by existing predicates)
- Bare "a person" would affect 13 provisions — but 8 are false positives (62% FP rate). Too broad.
- Compound "a person must" appears in exactly 3 provisions — all genuine prohibitions. 100% precision.
- "a person shall" appears in 5 provisions — 3 already have DRRP, 2 are definitional ("shall be regarded"). Not safe.

The 3 provisions:
- CDM reg 28(1): "A person **must not** ride...on any vehicle"
- CDM reg 28(2): "A person **must not** remain...on any vehicle"
- CDM reg 32(2): "a person **must not** carry out work unless suitably instructed"

**Step 1b (true-negative regression tests)**:
- `person_definitional_no_match` — "a person shall be regarded as competent" (duty_patterns.rs)
- `person_as_object_no_match` — "require a person to repeat" (duty_patterns.rs)
- `person_scope_exclusion_no_match` — "shall not apply to a person" (duty_patterns.rs)
- `person_regarded_as_competent_no_drrp` — full pipeline (mod.rs)
- `person_scope_exclusion_no_drrp` — full pipeline (mod.rs)

All 5 passed before the change.

**Step 2 (failing tests)**:
- `person_must_not_ride_prohibition` — CDM reg 28(1)
- `person_must_not_remain_prohibition` — CDM reg 28(2)
- `person_must_not_carry_out_work_prohibition` — CDM reg 32(2)

All 3 failed as expected (empty duty_types).

**Step 3 (change)**: Added `"a person must"` to `GOVERNED_ACTORS`. Compound predicate — only matches when "a person" is immediately followed by "must", avoiding false positives from "a person" as object.

**Step 4 (full suite)**: 155 passed, 0 failed.

**Step 5 (measurement)**:

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| CDM 2015 DRRP count | 82 | 85 | +3 provisions |
| Overall DRRP count | 483 | 486 | +3 provisions |
| Test suite | 152 | 155 | +8 tests (5 TN + 3 TP) |

Note: miss rate measurement via `taxa show` is unreliable because the Text field is truncated and only classified provisions are shown. DRRP count is the trustworthy metric.

**Running totals**: Baseline 424 DRRP → 486 DRRP (+62, +14.6%). Test suite: 132 → 155.

### Iteration 6 — actors.rs fixes: agency worker false positive + boundary matching

**Two issues found during government actor survey:**

**6a — "agency worker" false positive**: The generic `[Aa]gency` pattern in `actors.rs` matched "agency worker" and "temporary work agency" as `Gvt: Agency`. These are employment terms, not government agencies. Fix: added both to the blacklist. Named government agencies (HSE, Environment Agency, etc.) unaffected — they have specific patterns.

**6b — Boundary matching at start/end of string**: All ~40 actor patterns use `(?:[\s[:punct:]])` as word boundaries, requiring a character before/after the keyword. After `text_cleaner::clean()` trims whitespace, keywords at position 0 or end-of-string silently fail. Fix: pad text with spaces in `run_patterns()`.

**Government actor survey result**: Zero actionable gaps. Every provision with a government actor + modal verb already gets DRRP. The 60 no-DRRP government provisions are all correctly unclassified (headings, definitions, amendments).

**Measurement**: DRRP count unchanged at 486 (boundary fix adds 11 newly-visible actor extractions but no new DRRP for this sample). Test suite: 155 → 159.

**Commits**: `f0fb35a` (agency blacklist), `72cd58a` (boundary fix).

---

**Session started**: 2026-02-26
**Status**: Active — ready for Iteration 7
