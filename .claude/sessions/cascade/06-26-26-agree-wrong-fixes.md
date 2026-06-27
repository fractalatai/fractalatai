# Session: Agree+Wrong Pattern Fixes (ACTIVE)

## Problem

325 cases where both regex and classifier agree on the wrong position. Three patterns identified with specific fixes.

## Pattern 1: mentioned→active (183 cases, 56%)

Actors in definitions, references, amendments, structural provisions — not duty-creating clauses. Both tiers see actor label + modal language and assume active.

**Fix: Provision purpose gating.** The existing purpose classifier already tags provisions as structural/definition/enactment. If purpose is NOT duty-bearing, override position to `mentioned` regardless of actor presence. This is a deterministic rule, not ML.

## Pattern 2: counterparty→active (62 cases, 19%)

Actors who hold claims (counterparty) but appear prominent in the text. Both tiers default to active for any actor they find.

**Fix: Grammatical role via dependency parsing.** If the actor is the object of the verb (not the subject), it's counterparty not active. "Authority responsible for maintaining the service" — authority is object of "responsible for", not subject of the duty verb.

## Pattern 3: beneficiary→active/counterparty (24 cases, 7%)

Actors who benefit without a direct legal relation.

**Fix: Section type feature.** 151 errors from sub_article, 128 from sub_section. Add `section_type` as a classifier feature — structural section types are more likely to have mentioned/beneficiary actors. Currently the classifier has no section_type signal.

## Review findings (2026-06-27)

### Purpose classifier assessment

15 purpose labels, regex-based. Key issue: `Process+Rule+Constraint+Condition` is the DEFAULT when nothing matches. 148 of 183 errors have this default — they're unclassified, not genuinely process/rule provisions.

Current `should_skip_drrp` gates DRRP extraction but NOT position assignment. Has a flawed `has_any_actor` override: "if actors present, allow DRRP even for structural purposes." This is wrong — actors appear in definitions too.

### Gemini critical review (2026-06-27)

1. **Process+Rule default is a red flag** — should be `Unclassified` with cautious handling, not assumed duty-bearing
2. **`has_any_actor` override is fundamentally flawed** — remove for structural purposes
3. **Gate must be on BOTH DRRP and position** — currently only gates DRRP
4. **Mixed provisions need modal override** — "In this regulation, 'employer' means a person who shall ensure..." contains a real duty inside a definition. Purpose gate must check for modal verbs before overriding to mentioned
5. **Gate at actor level, not provision level** — actor near "shall" = active. Actor in "means..." = mentioned

### Purpose distribution of 183 errors
| Purpose | Count |
|---------|-------|
| Process+Rule (DEFAULT) | 148 |
| Interpretation+Definition | 50 |
| Application+Scope | 18 |
| Enforcement+Prosecution | 13 |
| Amendment | 8 |
| Offence | 7 |

## Revised actions

1. ✅ **Fix the default**: `Unclassified` replaces `Process+Rule` as default. `STRUCTURAL_PURPOSES` constant defined.
2. ✅ **Remove `has_any_actor` override**: changed to `has_government_actor` for structural purposes.
3. ✅ **Position gating**: `should_default_to_mentioned(purposes, text)` — returns true for structural purposes without duty-bearing modals. Wired into `parse_provisions`.
4. ✅ **Modal check** (simpler version): any modal in provision → allow normal position assignment. Per-actor proximity deferred to dependency parsing session.

**Result: 82 agree+wrong errors fixed (325 → 243). Regex position 51.3% → 53.8%.**
5. ⬜ **Add section_type as classifier feature** (cheap win, 24 errors) — requires classifier retrain, carry to dependency parsing session alongside other feature improvements
6. ➡️ Dependency parsing for grammatical role → see `cascade/06-26-26-dependency-parsing.md`

## Expected impact (revised)

- Actions 1-3: ~148 + ~50 = ~198 errors fixed (deterministic rules)
- Action 4: handles mixed provisions without false negatives
- Action 5: ~24 errors (requires classifier retrain)
- Action 6: ~62 errors (requires new infrastructure)

## Dependencies

- ✅ Deep-dive analysis done in classifier training session
- ✅ Purpose classifier exists in fractalaw-core/taxa/purpose.rs
- ✅ `should_skip_drrp` exists in fractalaw-core/taxa/mod.rs
- Dependency parsing session for action 6
