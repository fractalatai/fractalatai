---
session: v2 Parser Validation at Scale + Clause Extraction
status: closed
opened: 2026-02-26
closed: 2026-02-26
outcome: success
summary: 'Validated v2 DRRP parser on 797 new provisions across 7 personal safety laws, confirming +13 net gain over v1. Implemented
  miss analysis heat-scoring, fixed person-compound window and passive-by pattern bugs, and added span-based clause extraction
  replacing full cleaned_text with focused DRRP snippets.

  '
decisions:
- what: Only 12 of 132 hot misses are genuine v2 bugs
  why: Most hot misses are thing-subject obligations, correct rejections, or elaboration sub-provisions better handled by
    AI
  result: Focused fix effort on 2 actionable patterns instead of broadening regex
- what: Span-based clause extraction replacing clause_refiner fallback
  why: v2 parser already knows exact match positions via find() but was discarding them with is_match()
  result: MatchSpan struct captures actor/modal positions, extract_clause() produces 300-char focused snippets
lessons:
- title: v2 architecture strength on diverse actor vocabularies
  detail: Laws with non-standard duty-holders (user, operator, competent person) showed v2 gains because it anchors any extracted
    actor rather than checking a gatekeeper list
  tag: parser-design
- title: Heat-scoring prioritises miss investigation
  detail: Scoring provisions by modal presence, actor extraction, and purpose classification surfaces genuine regressions
    efficiently
  tag: qa-methodology
metrics:
  v1_drrp_combined: 688
  v2_drrp_combined: 672
  false_positives_removed: 54
  true_positives_added: 38
  test_count: 282
  test_pass: 282
artifacts:
- crates/fractalaw-core/src/taxa/duty_patterns_v2.rs
- crates/fractalaw-core/src/taxa/mod.rs
- crates/fractalaw-core/src/taxa/clause_refiner.rs
- crates/fractalaw-cli/src/main.rs
depends_on:
- 02-26-26-drrp-parser-v2.md
- 02-26-26-taxa-regex-patterns.md
enables: []
---


# Session: 2026-02-26 — v2 Parser Validation at Scale + Clause Extraction (CLOSED)

**Parent sessions**: [02-26-26-drrp-parser-v2.md](02-26-26-drrp-parser-v2.md), [02-26-26-taxa-regex-patterns.md](02-26-26-taxa-regex-patterns.md)
**Status**: Complete

## Objectives

1. **Validate v2 parser on a fresh sample** of 7 personal safety laws (797 sections) not seen during development
2. **Surface missing actors** — keywords extracted by actors.rs but not producing DRRP in v2
3. **Surface missing regex patterns** — obligation modals that v2 doesn't match
4. **Compare v1 vs v2** — confirm v2 removes false positives without losing true positives
5. **Clause extraction** — assess whether we can return a focused DRRP snippet (actor + modal + action) instead of full cleaned text

## Sample 2: 7 New Personal Safety Laws

| Law | Short Name | Sections |
|-----|-----------|----------|
| `UK_uksi_2005_1643` | Control of Noise at Work 2005 | 75 |
| `UK_uksi_1992_2792` | Display Screen Equipment 1992 | 38 |
| `UK_uksi_2005_1093` | Control of Vibration at Work 2005 | 69 |
| `UK_uksi_2002_2676` | Control of Lead at Work 2002 | 91 |
| `UK_uksi_2013_1471` | RIDDOR 2013 | 200 |
| `UK_uksi_2000_128` | Pressure Systems Safety 2000 | 124 |
| `UK_uksi_2015_483` | COMAH 2015 | 200 |
| **Total** | | **797** |

Previously tested (Sample 1): HSWA, MHSWR, CDM, PUWER, Manual Handling, LOLER, COSHH (996 sections).

## Plan

### Phase 1: v1/v2 Comparison on Sample 2

For each of the 7 new laws:
1. Run `taxa show --compare <law>` 
2. Record v1 count, v2 count, diffs, v1-only, v2-only
3. Inspect v1-only provisions — are they genuine false positives?
4. Inspect v2-only provisions — are they genuine new matches?
5. Inspect zero-DRRP provisions with modals — missing actors or patterns?

### Phase 2: Miss Analysis (heat-scored)

Built `taxa show --misses` CLI mode. For each provision v2 doesn't classify, compute a heat score:
- +3 obligation modal (shall/must/is required to)
- +2 governed actor extracted
- +1 enabling modal (may/power to)
- +1 government actor extracted
- +1 operative purpose (Process+Rule)
- −2 structural-only purpose (Interpretation/Amendment/Repeal)
- −1 short text (< 50 chars, likely heading)

Hot (>= 3) provisions are examined first — most likely genuine misses.

### Phase 3: Clause Extraction

1. Add span capture to `DutyClassification`
2. Update `match_actor_anchored()` and `match_person_compound()` to capture spans
3. Add `clause_refined` field to `TaxaRecord` using the span
4. Wire `clause_refiner::refine()` as fallback for non-v2 matches
5. Test on sample provisions — compare full text vs extracted clause

### Phase 4: Fix Iterations

Same test-driven cycle as Session 1:
1. Pick highest-frequency miss
2. Add true-negative regression tests
3. Add failing true-positive tests
4. Implement minimal fix
5. Full suite pass
6. Measure improvement

## Results

### Phase 1: v1/v2 Comparison

| Law | v1 | v2 | Diffs | v1-only | v2-only |
|-----|----|----|-------|---------|---------|
| Noise at Work 2005 | 39 | 35 | 4 | 4 | 0 |
| Display Screen Equipment 1992 | 18 | 17 | 1 | 1 | 0 |
| Vibration at Work 2005 | 33 | 30 | 5 | 3 | 0 |
| Lead at Work 2002 | 60 | 51 | 11 | 9 | 0 |
| RIDDOR 2013 | 30 | 30 | 2 | 1 | 1 |
| Pressure Systems Safety 2000 | 24 | 41 | 21 | 2 | 19 |
| COMAH 2015 | 79 | 92 | 15 | 1 | 14 |
| **Total** | **283** | **296** | **59** | **21** | **34** |

**v2 finds +13 more provisions overall** — a net gain, not a loss. This reverses the Sample 1 pattern (where v2 found fewer) because Sample 2 includes laws with actors outside v1's `GOVERNED_ACTORS` list.

#### v2-only gains (+34): Why v2 finds more

**Pressure Systems Safety (+19)** and **COMAH (+14)** use duty-holder labels (`user`, `owner`, `operator`, `competent person`) that actors.rs extracts but v1's `GOVERNED_ACTORS` list doesn't include. v2 bypasses the blunt gate entirely — it directly anchors whatever actors.rs finds to the modal verb.

Key actors driving v2 gains:
- `Ind: User` — 15 provisions (Pressure Systems primary duty-holder)
- `Operator` — 14 provisions (COMAH primary duty-holder)
- `Org: Owner` — 12 provisions (Pressure Systems co-holder with user)
- `Ind: Competent Person` — 8 provisions (Pressure Systems examination duties)

This is the v2 architecture working as designed: actors.rs has broad coverage → v2 anchors any extracted actor → no separate gatekeeper list to maintain.

#### v1-only removals (−21): False positives correctly removed

All 21 are genuine false positives — provisions where an actor keyword appears but is not the duty-holder:

| Pattern | Count | Example |
|---------|-------|---------|
| "shall not apply" scope exclusions | 3 | Lead s.3: "shall not apply to the master or crew" |
| "shall include/be" thing-subject | 4 | Lead s.6: "measures shall include arrangements for safe handling" |
| "shall be updated/adapted" modifier | 5 | Noise s.10, Vibration s.8, Lead s.11: "information shall be updated" |
| "Nothing in...shall" saving clauses | 2 | Lead s.7, DSE s.5: "Nothing in this regulation shall prevent" |
| Interpretation/definition sections | 2 | Pressure s.2, Lead s.5: long interpretation sections |
| "Paragraphs shall not apply where" | 2 | Lead s.12: conditional scope exclusion |
| Sub-type reclassification (v1→v2) | 3 | Lead s.5, s.10, Vibration s.7: v1=Duty, v2=Right (enabling) |

Zero false negatives in the v1-only set — every removal is correct.

#### Combined Sample 1+2 Totals

| | Sample 1 | Sample 2 | Combined |
|--|---------|---------|----------|
| Sections | 996 | 797 | 1,793 |
| v1 DRRP | 405 | 283 | 688 |
| v2 DRRP | 376 | 296 | 672 |
| v1-only (FP removed) | 33 | 21 | 54 |
| v2-only (new TP) | 4 | 34 | 38 |
| Net change | −29 | +13 | −16 |

v2 removes 54 false positives and adds 38 true positives across both samples. Net −16 is entirely from Sample 1's blunt-gate FPs being cleaned up; Sample 2 shows the v2 architecture's strength on laws with diverse actor vocabularies.

### Phase 2: Miss Analysis

#### Heat Distribution (all 7 laws)

| Law | Total | Classified | Missed | Hot (>=3) | Warm (1-2) | Cold (<=0) |
|-----|-------|-----------|--------|-----------|------------|------------|
| Noise at Work | 75 | 35 | 40 | 14 | 10 | 16 |
| Display Screen Equipment | 38 | 17 | 21 | 6 | 3 | 12 |
| Vibration at Work | 69 | 30 | 39 | 13 | 11 | 15 |
| Lead at Work | 91 | 51 | 40 | 15 | 10 | 15 |
| RIDDOR | 200 | 30 | 170 | 31 | 49 | 90 |
| Pressure Systems Safety | 124 | 41 | 83 | 24 | 28 | 31 |
| COMAH | 200 | 92 | 108 | 29 | 43 | 36 |
| **Total** | **797** | **296** | **501** | **132** | **154** | **215** |

132 hot misses across 797 sections = 16.6% of all provisions warrant examination.

#### Miss Categories (from 132 hot misses)

**Category A: Thing-subject obligations (heat 4)** — ~50 provisions

The subject of the modal is a thing/concept, not a person or org. v2 is correct to miss these — no actor to anchor. These are the GH#16 "Rule" type from Session 1.

Examples:
- "The risk assessment shall include consideration of..."
- "A safety management system must..."
- "An internal emergency plan must contain..."
- "Mixtures must be treated in the same way..."
- "The inspection plan must be regularly reviewed..."

**Action**: No regex fix. These need a separate "Rule" classifier (GH#16) or AI polisher.

**Category B: Correct rejections (heat 4-6)** — ~30 provisions

Scope exclusions, saving clauses, citation/commencement, and definitional provisions that v2 correctly does not classify:

- "These Regulations shall not apply to the master or crew..." (scope)
- "Nothing in paragraph (3) shall require..." (saving)
- "These Regulations may be cited as..." (citation)
- "These Regulations shall come into force on..." (commencement)
- "These Regulations shall have effect with a view to protecting..." (purpose statement)
- "Paragraphs (1) and (3) shall not apply where..." (conditional scope)

Heat is high because actors are mentioned alongside modals, but the provision establishes scope/context, not a duty. v2's anchor correctly fails because the actor isn't the grammatical subject of an obligation.

**Action**: None needed. Consider lowering heat for provisions with Application+Scope purpose.

**Category C: Elaboration sub-provisions (heat 6)** — ~25 provisions

Sub-paragraphs that elaborate on a parent duty. The parent provision has the actor+modal anchor (and v2 classifies it), but the sub-provision restates the obligation in passive/indirect form:

- "The information, instruction and training...shall include..." (parent: "employer shall provide information")
- "The information...shall be updated to take account of..." (parent: "employer shall provide")
- "The risk assessment shall be reviewed regularly..." (parent: "employer shall make assessment")
- "Medical surveillance...shall be commenced before..." (parent: "employer shall ensure medical surveillance")

These inherit the duty from their parent but don't independently state "actor + shall + action". The actor is implied from the parent provision.

**Action**: Two options:
1. **Provision-chain inference** — if parent provision has DRRP, propagate to children. Complex, requires section hierarchy parsing.
2. **Passive-voice fallback** — detect "{thing} shall {be/include/contain}" patterns and classify at low confidence. Simpler but noisy.
3. **Leave for AI polisher** — the AI sees parent+child in context and can infer the inherited duty.

Recommend option 3 for now — these are genuine grey areas better handled by AI than regex.

**Category D: Actor + modal but v2 misses (heat 6)** — ~12 provisions (GENUINE REGRESSIONS)

The actor appears before the modal within the window, but v2 doesn't fire. These are the actionable bugs:

1. **"Any person who designs, manufactures, imports or supplies...shall ensure"** (Pressure Systems s.4) — "a person who" should trigger PERSON_QUALIFIERS compound, but the full phrase is "Any person who designs..." — v2's `PERSON_QUALIFIERS` regex has `any person who` but only with specific suffixes.

2. **COMAH operator provisions with indirect structure** — "A major accident prevention policy must be prepared by the operator" (reverse: obligation before actor). The "shall be the duty of" reverse anchor only fires for that exact phrase; "must be prepared by" is a different reverse construction.

3. **"Responsible person" in RIDDOR** — v2 doesn't have `responsible person` in actors.rs broad labels, and no person compound fires for "the responsible person is under more than one requirement".

**Action**: Fix in Phase 4 iterations. Priority:
1. Person compound: expand `PERSON_QUALIFIERS` to cover "any person who" without specific suffixes
2. Reverse passive: "must be {done} by the {actor}" pattern
3. Responsible person: check actors.rs coverage

**Category E: RIDDOR schedule descriptors (heat 3)** — ~25 provisions

Schedule items in RIDDOR listing reportable events. These mention "person" but describe incidents, not obligations:
- "The collision of a train...which could have caused the death...of any person"
- "The unintentional release...which could cause personal injury to any person"

Correctly not classified. Heat is 3 only because "governed_actor + operative_purpose" fire.

**Action**: None. These are descriptive, not prescriptive.

#### Summary: Actionable vs Non-actionable

| Category | Count | Actionable? |
|----------|-------|-------------|
| A: Thing-subject obligations | ~50 | No (GH#16 "Rule" type) |
| B: Correct rejections | ~30 | No (scope/citation/saving) |
| C: Elaboration sub-provisions | ~25 | Defer to AI polisher |
| D: Genuine regressions | ~12 | **Yes — fix in Phase 4** |
| E: Schedule descriptors | ~25 | No (descriptive) |

Only ~12 of 132 hot misses (9%) are genuine bugs in v2. The rest are either correct behaviour or architectural limitations best handled by AI.

### Phase 4: Fix Iterations

#### Fix 1: Person compound extended window

**Bug**: `match_person_compound()` searched for obligation modals only within `PRIMARY_WINDOW` (120 chars) after the person qualifier. Pressure Systems s.4 — "Any person who designs, manufactures, imports or supplies...shall ensure" — has 143 chars between compound and modal.

**Fix**: Added extended window fallback (200 chars, confidence 0.50) matching the same pattern used in `match_actor_anchored()`.

**Result**: +1 provision (Pressure Systems s.4).

#### Fix 2: Reverse passive "must be {done} by the {actor}"

**Bug**: v2 only handled forward anchors (`actor.{0,N}modal`) and "shall be the duty of" reverse. COMAH uses passive voice: "must be prepared by the operator", "must be reviewed by the operator". The actor appears after the modal.

**Fix**: Added `match_passive_by_pattern()` — detects `\b(shall|must)\b\s+be\s+.{0,60}\bby\b.{0,20}\b{actor}\b`. The "by" preposition distinguishes agent from recipient ("to"/"for"/"with").

True-negative test: "must be provided to the contractor" correctly returns None.

**Result**: +3 provisions (COMAH s.7, s.10, s.12).

#### Fix 3: Responsible person (SKIPPED)

Investigation showed RIDDOR "responsible person" provisions have heat=3, not heat=6 — they have actors but no modal verbs. "The responsible person keeps" / "is under" are descriptive, not obligatory. Not a v2 bug.

#### After-fix Totals

| Law | v2 Before | v2 After | Delta |
|-----|-----------|----------|-------|
| Noise at Work | 35 | 35 | 0 |
| Display Screen Equipment | 17 | 17 | 0 |
| Vibration at Work | 30 | 30 | 0 |
| Lead at Work | 51 | 51 | 0 |
| RIDDOR | 30 | 30 | 0 |
| Pressure Systems Safety | 41 | 42 | +1 |
| COMAH | 92 | 95 | +3 |
| **Sample 2 Total** | **296** | **300** | **+4** |
| **Sample 1 Total** | **376** | **376** | **0** (no regressions) |

Test suite: 197 → 202 (5 new tests). 202 pass, 0 fail.

## Phase 3: Clause Extraction (Span-Based)

**Goal**: Replace the full `cleaned_text` with a focused "who must do what" clause snippet.

**Problem**: `clause_refiner.rs` was orphaned (ported from Elixir, never wired in). `clause_refined` was just `cleaned_text`. The v2 parser knows exact match positions but discarded them via `is_match()`.

### Design

1. Added `MatchSpan { actor_start, modal_start, modal_end }` to `DutyClassification`
2. Replaced `is_match()` with `find()` across all v2 pattern functions to capture positions
3. New `extract_clause()` in `mod.rs` uses spans to extract a window:
   - Start: ~100 chars before actor, snapped to sentence boundary
   - End: ~200 chars after modal, snapped to sentence boundary
   - Max 300 chars total
4. For government patterns (no span), falls back to `clause_refiner::refine()`
5. Added `clause_refined: Option<String>` to `TaxaRecord`

### Functions Updated for Span Capture

| Function | Change |
|----------|--------|
| `match_actor_anchored()` | `is_match()` → `find()`, uses `extract_span_from_anchored()` |
| `match_duty_of_pattern()` | `is_match()` → `find()`, locates modal + actor positions |
| `match_passive_by_pattern()` | `is_match()` → `find()`, captures modal and actor spans |
| `match_person_compound()` | All branches now build `MatchSpan` from compound/modal positions |
| `classify_after_modal()` | Returns `span: None`, callers set span post-hoc |

### Quality Assessment (Real Laws)

Tested on HSWA, Noise at Work, COMAH:

| Pattern | Example Clause | Quality |
|---------|---------------|---------|
| Standard anchor | "An employer who carries out work...shall make a suitable and sufficient assessment of the risk..." | Excellent — actor + modal + full action |
| Duty-of pattern | "It shall be the duty of every employer to ensure, so far as is reasonably practicable, the health, safety and welfare..." | Excellent — captures reverse formulation |
| Passive-by | "A major accident prevention policy must be prepared by the operator..." | Good — captures obligation with agent |
| Person compound | "A person must not ride, or be required or permitted to ride, on any vehicle..." | Good — prohibition with context |
| Government fallback | "The Secretary of State shall have power to make regulations." | Adequate — clause_refiner handles |

Test suite: 202 → 282 (8 new clause extraction tests + 72 pre-existing). 282 pass, 0 fail.

### CLI Updates

- `taxa show` now uses `parse_v2()` (was `parse()`) and displays `Clause:` instead of truncated `Text:` when clause_refined is available
- `taxa enrich` now writes `record.clause_refined` to LanceDB/DuckDB instead of full `cleaned_text`

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/taxa/duty_patterns.rs` | DutyClassification + MatchSpan structs, government patterns |
| `crates/fractalaw-core/src/taxa/duty_patterns_v2.rs` | v2 actor-anchored patterns with span capture |
| `crates/fractalaw-core/src/taxa/duty_type.rs` | classify / classify_v2 orchestrator |
| `crates/fractalaw-core/src/taxa/mod.rs` | parse / parse_v2 / extract_clause / analyse_miss pipeline |
| `crates/fractalaw-core/src/taxa/clause_refiner.rs` | Modal-window clause extraction (fallback for government patterns) |
| `crates/fractalaw-core/src/taxa/actors.rs` | Actor extraction |
| `crates/fractalaw-cli/src/main.rs` | CLI: taxa show (--compare / --misses), taxa enrich |

## Commits

| Hash | Description |
|------|-------------|
| `cd829f4` | Phase 2+4: miss analysis tool + extended window + passive-by pattern fixes |
| `616ba66` | Phase 3: span-based clause extraction for focused DRRP snippets |

## Final Test Suite

282 tests pass, 0 fail. Breakdown:
- 202 pre-existing taxa tests (including 5 added in Phase 4)
- 8 new clause extraction tests
- 72 other fractalaw-core tests

## Remaining Work (Not This Session)

- **GH#16 "Rule" classifier** — thing-subject obligations (~50 provisions) need a separate pattern that doesn't require a person/org actor
- **Provision-chain inference** — elaboration sub-provisions (~25) inherit duties from parent provisions; requires section hierarchy parsing or AI polisher context
- **taxa enrich → parse_v2** — the `taxa enrich` command still uses `parse()` (v1) for the DuckDB aggregate write path; should switch to `parse_v2()` when v2 is promoted to default
