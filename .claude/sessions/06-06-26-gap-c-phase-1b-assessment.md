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

## Next Steps

- [ ] Investigate distance-0 provisions (potential bug or edge case)
- [ ] Decision: defer 1B, skip to 1C, or go to 2A?
- [ ] Precision QA: manually sample 50 inherited provisions for correctness
