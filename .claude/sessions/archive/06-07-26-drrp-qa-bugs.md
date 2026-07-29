---
session: "DRRP QA Bug Investigation"
status: closed
opened: 2026-06-07
closed: 2026-06-07
outcome: partial

summary: >
  Investigated two bugs found during DRRP QA (33% precision). Bug 1: DRRP type = none on
  duty-bearing provisions -- fixed participial person patterns and sub-paragraph separated
  patterns. Bug 2: all regex actors marked active -- fixed with position heuristic deriving
  Hohfeldian positions from match span (actor before modal = active, after = counterparty).
  QA precision remained at 33% after fixes because remaining failures are parser coverage
  gaps (schedule fragments, passive voice, thing-subject obligations). Confirmed regex ceiling
  reached. Wrote Classification Cascade Strategy v0.2 documenting the Regex-SLM-LLM architecture.

decisions:
  - what: "Regex position heuristic: derive position from match span"
    why: "Actor before modal verb = active, actor after modal = counterparty. Eliminates need for LLM calls on multi-actor regex provisions."
    result: "28 Tier 3 LLM calls for OH&S instead of 244 without heuristic"
  - what: "Accept regex ceiling and move to tiered cascade"
    why: "Regex cannot solve thing-subject obligations, passive voice, narrative duty references, or schedule fragments"
    result: "Classification Cascade Strategy v0.2: Regex (free) then own model (cheap) then LLM (expensive)"
  - what: "\"Done\" stamping at high confidence"
    why: "Never re-parse provisions that have been confidently classified -- saves compute and prevents regression"
    result: "Documented in cascade strategy, implementation deferred to next phase"

lessons:
  - title: "Regex parser has a hard ceiling"
    detail: "Clear cases handled well, but thing-subject obligations, passive voice, narrative duty refs, and schedule fragments are beyond regex capability."
    tag: architecture
  - title: "Position heuristic is a cheap proxy for LLM"
    detail: "Match span position (before vs after modal) correctly classifies most multi-actor regex provisions without any API calls."
    tag: optimization
  - title: "QA precision can be misleading"
    detail: "33% precision across the whole pipeline obscures that position classification is working correctly -- failures are DRRP=none coverage gaps, not position bugs."
    tag: quality
---

# Session: 2026-06-07 — DRRP QA Bug Investigation (CLOSED)

## Context

**Prior session**: `.claude/sessions/cascade/06-07-26-hohfeldian-position-build.md`
**QA results**: `data/qa-results/drrp-qa-all-20260607-144621.json`
**QA precision**: 33% (3/10 correct) — but measuring two different bugs

## Bug 1: DRRP type = "none" on duty-bearing provisions (PRIORITY)

Provisions with clear duty language ("shall ensure", "shall prepare") get `drrp_types: []` (none). The purpose classifier in `parse_v2()` is missing them.

**Examples from QA:**
- `UK_uksi_2016_588:sch.1.para.221` — "employer ensures that" → classified as none
- `UK_uksi_1998_2451:art.7(1)` — "shall ensure" → classified as none
- `UK_ukpga_1969_10:s.23` — "Right of local authority" → classified as none
- `UK_nisr_1997_195:sch.1.para.89` — "Duty Holder to demonstrate" → classified as none

**Investigation path:**
1. Pull the actual provision text for each failing case
2. Run through purpose classifier to see which gate rejected it
3. Check if the text is a schedule/paragraph that doesn't match the clause patterns
4. Fix the purpose classifier regex or skip gates

**Likely cause:** Schedule paragraphs and articles may not match the purpose classifier's section-level patterns. The classifier was built for `s.2(1)` style provisions, not `sch.1.para.221` or `art.7(1)`.

## Bug 2: All regex actors marked "active" — position classification gap

For regex provisions with multiple actors, all get `position: active` because the regex pipeline can't distinguish active from counterparty. This is by design for Tier 1, but produces incorrect results.

**Example from QA:**
- `UK_uksi_2010_1140:reg.4(5)` — Employer=active (correct), Employee=active (should be counterparty)
- `UK_uksi_2013_1471:reg.4(1)` — Person=active (should be counterparty), Responsible Person=active (correct)

**Root cause:** Tier 3 LLM only fires on inherited multi-actor provisions. Regex multi-actor provisions are never sent to the LLM for position classification.

**Options:**
1. **Expand Tier 3 scope** — fire on ANY provision with multiple actors, not just inherited. This means all regex provisions with >1 actor get an LLM call. Significant API cost increase (~4,000+ provisions vs ~500 inherited).
2. **Heuristic first** — for provisions with DRRP type = Duty, the actor matching the `governed_actors` pattern is likely `active` and others are `counterparty`. Government actors in a Duty provision are often the counterparty (enforcer, not duty-bearer). This could get 80% right without LLM.
3. **Selective Tier 3** — only send regex provisions where the actor mix is ambiguous (e.g., Employer + Employee in a Duty provision — who bears it?). Single-actor provisions don't need classification.
4. **Accept for now** — regex provisions are already "active" which is the safe default. The position model is most valuable for Tier 3 provisions where the LLM can make the distinction.

**Recommendation:** Option 2 (heuristic) + Option 3 (selective Tier 3 for ambiguous cases). The heuristic handles the obvious cases (Employee is always counterparty in an Employer duty), and Tier 3 handles the edge cases.

## Shipped (2026-06-07)

### Bug 1 fixes
- **Participial person patterns** (commit `a45bfb7`): `PERSON_QUALIFIERS` now matches "any person [verb]ing" (installing, carrying, having)
- **Sub-paragraph separated patterns** (commit `75288c3`): allows em-dash + sub-paragraph marker between "any person" and qualifier ("Any person— (a) who... shall ensure")
- 444 core tests pass, zero regressions

### Bug 2 fix: Regex position heuristic (commit `da47b3c`)
- Derives Hohfeldian positions from match span: actor before modal = active, actor after modal = counterparty
- Eliminates need for LLM calls on multi-actor regex provisions
- Tier 3 LLM reserved for inherited provisions only (28 calls for OH&S vs 244 without heuristic)
- 3 position tests added to core

### QA results after fixes
- 3 QA runs at 10 samples each: consistent 33% precision
- Failures now dominated by parser coverage gaps (DRRP=none on provisions with duties) not position classification
- Position heuristic working correctly when DRRP match exists
- Remaining failures: schedule fragments, passive voice, thing-subject obligations, narrative duty refs

### Regex ceiling reached
Investigation confirmed the regex parser handles the clear cases but cannot solve:
- Thing-subject obligations ("policy must") — actor not in subject position
- Passive voice ("can be taken") — no explicit actor
- Narrative duty references ("the duty he owes them") — "duty" as a noun
- Schedule fragments — text only meaningful with parent clause context

### Classification Cascade Strategy (v0.2)
- Strategy doc: `docs/CLASSIFICATION-CASCADE-STRATEGY.md`
- Gemini review: `docs/reviews/gemini-cascade-strategy-review-20260607.md`
- Architecture: Regex (free) → Own model (cheap) → LLM (expensive)
- "Done" stamping at high confidence — never re-parse
- Customer priority routing — full cascade only on registered laws
- Forward feedback: LLM findings → regex pattern improvements
- Active learning with diversity sampling for Tier 2 training
- Golden dataset (500-1000 provisions) needed for threshold calibration

### Also this session
- Hohfeldian position model implemented (commit `4a1e544`): `position` field replaces `role`
- HM Forces moved from government to governed defs
- `--skip-recent` flag for enrichment (24h window)
- `enrich-and-publish` skill created
- `drrp-qa` skill created (renamed from `tier1-qa`)
- OH&S enriched with position heuristic + published to sertantai
- QQ applicable laws enriched and published (71,908 provisions)
- NAS backup taken (20260607)
- Sertantai briefing doc updated with final Hohfeldian schema

## What's next

1. **`--force-low-confidence` flag** — only re-parse provisions below confidence threshold
2. **`classification_tier` + `classification_version` columns** — lineage tracking
3. **Correct "none" confidence** — high confidence stamp for genuinely non-DRRP provisions
4. **Golden dataset** — 500-1000 annotated provisions for calibration
5. **Tier 2 prototype** — NN on embeddings for DRRP type
6. **Forward feedback tooling** — systematic LLM → regex improvement cycle

## References

- Purpose classifier: `fractalaw-core/src/taxa/purpose.rs`
- DRRP parser: `fractalaw-core/src/taxa/mod.rs` (`parse_v2`)
- Pattern matcher: `fractalaw-core/src/taxa/duty_patterns_v2.rs`
- Cascade strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY.md`
- Gemini cascade review: `docs/reviews/gemini-cascade-strategy-review-20260607.md`
- Gemini actors review: `docs/reviews/gemini-actors-struct-review-20260607.md`
- QA results: `data/qa-results/drrp-qa-all-20260607-*.json`
- QA skill: `.claude/skills/drrp-qa/`
- Prior session: `.claude/sessions/cascade/06-07-26-hohfeldian-position-build.md`
