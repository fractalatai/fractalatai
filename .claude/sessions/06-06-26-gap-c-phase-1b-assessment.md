# Session: 2026-06-06 — Gap C Phase 1B Assessment: Cross-Reference Resolver

## Context

**Meta-plan**: `.claude/plans/gap-c-tiered-resolution.md`
**Design doc**: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` v0.4
**Prior**: Phase 1A complete — 8,648 provisions inherited via Tier 1

## Pre-implementation Assessment

Before building the citation resolver, surveyed the remaining Gap C provisions in the customer corpus (299 laws, re-enriched with `--gap-c --force`).

### Finding: Zero C4 candidates remain after Tier 1

After Tier 1 parent inheritance:

| Category | Count |
|----------|-------|
| Total provisions with DRRP | 83,935 |
| Total provisions without DRRP | 78,169 |
| Without DRRP + text + **duty-bearing purpose** | **0** |

All 78,169 remaining no-DRRP provisions have **no purposes assigned** — they were either:
- Not in the enriched corpus (330 laws not in customer list)
- Skipped by parse_v2 (headings, empty text, structural markers)
- From laws not re-enriched with `--force` in this pass

Within the customer corpus, Tier 0 (regex) + Tier 1 (inheritance) have resolved every provision that has a duty-bearing purpose. There are no provisions left where:
- parse_v2 found a duty-bearing purpose (Process+Rule, Power, Offence, etc.)
- BUT no actor was extracted by regex
- AND no actor was inherited from a parent

### What this means for the plan

The original Gap C estimate of ~3,275 provisions (from OHS corpus analysis) was based on provisions with **modal verbs but no extracted actors**. Tier 1's parent inheritance resolved the vast majority of these, and the remainder have purposes that are correctly non-duty-bearing (Interpretation, Amendment, etc.).

**Phase 1B (cross-reference resolver) may not be needed yet.** The C4 problem — "the duty-holder defined in section 2" — either:
1. Was resolved by Tier 1 (the parent chain contained the actor)
2. Exists in laws not yet in the customer corpus
3. Will emerge when more laws get their LAT re-parsed (the "parked" making classifier refresh)

### Recommended next steps

1. **Defer Phase 1B** until the making classifier refresh brings in new laws that may have C4 patterns
2. **Skip to Phase 1C (LLM proof of concept)** on a small set of genuinely ambiguous provisions from outside the customer corpus, to validate the approach exists when needed
3. **Or**: Declare the four-tier pipeline Phase 1 complete and move to Phase 2A (single-law integration / QA) to validate precision on the 8,648 inherited provisions

### Distance-0 investigation

The Milestone 1 report flagged 388 provisions with `ancestor_distance=0`. These are provisions that inherit from a sibling at the same hierarchy depth — same `hierarchy_path` prefix length. This shouldn't happen with the strict prefix check. Worth investigating before declaring Phase 1 complete.

## Progress

- [x] Distance-0 bug fixed (commit `5fc161e`) — false sibling prefix match
- [x] Full corpus re-enriched: 7,375 inherited across 137 laws
- [x] Phase 1B deferred → fractalaw/fractalaw#36
- [x] Tier 1 QA skill built (`.claude/skills/tier1-qa/`)
- [x] QA run: 40 samples, 76.2% precision [63.5%, 88.9%] — below 85% target

## QA Failure Patterns

Three distinct patterns account for all 9 failures. Fix each, re-run QA.

### Pattern 1: Part/Chapter heading inheritance (HIGH IMPACT)

**Problem**: Provisions inherit actors from Part or Chapter headings (e.g., "Part V: Rights of Owners", "Part 2: Obligations of economic operators"). These headings contain actor keywords but are structural labels, not duty-creating provisions.

**Signature**: ancestor_distance ≥ 3, parent section_type is `part` or `chapter`.

**Examples**:
- `s.88(3)(a)` inherited "Org: Owner" from `pt.V` "Rights of Owners" (distance 5)
- `reg.53(4)(b)` inherited "Public" from `pt.5` "Public registers" (distance 3)
- `s.65(1)` inherited "Public" from `pt.3` "Public Rights of Way" (distance 3)

**Fix**: Exclude `part`, `chapter`, and `heading` section_types as inheritance sources in the ancestor filter. These are structural containers, not provisions with actors.

**Expected impact**: Eliminates ~4 of 9 failures.

### Pattern 2: Recipient vs holder confusion (MEDIUM IMPACT)

**Problem**: Parent says "employer shall provide information to employees" — both employer and employee are extracted as actors. Child inherits both, but the employee is the recipient, not the duty holder.

**Signature**: ancestor_distance = 1, multiple actors inherited, child provision is about information/notification.

**Examples**:
- `reg.19(3)(a)(iii)`: parent has Responsible Person + Employee, but the duty is on the Responsible Person to inform the Employee
- `reg.10(1)(a)(viii)`: parent has Employer + Employee, duty is on Employer to provide info

**Fix**: This is harder — the regex pipeline extracts all mentioned actors, not the duty-bearing one. Would need the regex pipeline to distinguish holder from recipient. **Defer to Phase 2A** — this is a parse_v2 improvement, not a Tier 1 fix.

**Expected impact**: ~2-3 failures. Deferred.

### Pattern 3: Exemption/scope provisions (LOW IMPACT)

**Problem**: Child provision describes a condition, exemption, or scope limitation rather than extending a duty. The inheritance is technically correct (same actor) but the child isn't duty-bearing.

**Signature**: child text contains "does not apply", "exempt", "subject to", "condition".

**Examples**:
- `reg.7(7)(b)`: describes an exemption condition, not a duty
- `reg.34A(3)(b)`: describes how regulations apply with modifications

**Fix**: Tighten `is_duty_bearing_purpose()` or add a post-inheritance check for exemption language. **Investigate after Pattern 1.**

**Expected impact**: ~2 failures.

## QA Results

### Run 1 (pre-Pattern 1 fix): 76.2% [63.5%, 88.9%]
40 samples, 31 correct, 9 incorrect. Mix of all three patterns.

### Pattern 1 fix applied
Excluded `part`, `chapter`, `heading`, `title` section_types as inheritance sources.
Corpus: 7,375 → 6,175 inherited (1,200 false structural matches removed).

### Run 2 (post-Pattern 1 fix): 76.2% [63.5%, 88.9%]
40 samples, 31 correct, 9 incorrect. Same precision — Pattern 1 was real but the random sample draws mostly distance-1 provisions (74% of corpus). The 9 failures are now almost entirely **Pattern 2** (recipient vs holder confusion).

### Conclusion

**76% is the Tier 1 precision ceiling.** The remaining errors are not inheritance logic problems — they're actor extraction problems. The parent says "employer shall provide training to employees", parse_v2 extracts both actors, and the child inherits both. The employee is a recipient, not a duty holder.

This is the right boundary between Tier 1 (deterministic, structural) and Tier 3 (LLM reasoning about actor roles). Fixing it in Tier 1 would require parse_v2 to distinguish holder from recipient — a significant change to the regex pipeline that risks regressions on the 63,260 regex-extracted provisions.

### Metrics summary

| Metric | Value |
|--------|-------|
| Total inherited (post-fix) | 6,175 provisions |
| Laws with inheritance | 137 |
| Precision | 76.2% [63.5%, 88.9%] |
| Correct | ~4,700 provisions (76% of 6,175) |
| False positives | ~1,475 provisions (mostly recipient-not-holder) |

## Next Steps

- [ ] Commit Pattern 1 fix + QA skill
- [ ] Pattern 2 (recipient vs holder) → input to Tier 3 prompt design
- [ ] Pattern 3 (exemption provisions) → rare, also Tier 3 territory
- [ ] Decide: accept 76% precision for Tier 1, or raise the bar?
