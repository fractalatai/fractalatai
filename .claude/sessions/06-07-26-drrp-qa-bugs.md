# Session: DRRP QA Bug Investigation

## Context

**Prior session**: `.claude/sessions/06-07-26-hohfeldian-position-build.md`
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

## What's next

1. **Bug 1 first** — investigate and fix purpose classifier misses on schedule/article provisions
2. **Bug 2** — implement heuristic position assignment for regex provisions, then selective Tier 3
3. Re-run QA after fixes to measure improvement

## References

- Purpose classifier: `fractalaw-core/src/taxa/purpose.rs`
- DRRP parser: `fractalaw-core/src/taxa/mod.rs` (`parse_v2`)
- Clause decomposition: `fractalaw-core/src/taxa/clause.rs`
- QA results: `data/qa-results/drrp-qa-all-20260607-144621.json`
- QA skill: `.claude/skills/drrp-qa/`
