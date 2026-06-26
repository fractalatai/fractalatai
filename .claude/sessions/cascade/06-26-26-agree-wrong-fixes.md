# Session: Agree+Wrong Pattern Fixes (PENDING)

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

## Actions

1. ⬜ Add purpose-gating rule: if provision purpose is structural/definition → all actors = mentioned
2. ⬜ Add section_type as classifier feature (10 categories one-hot, cheap win)
3. ⬜ Dependency parsing for grammatical role (subject/object distinction) — see `cascade/06-26-26-dependency-parsing.md`

## Expected impact

- Pattern 1 fix (purpose gating): ~183 errors → mostly correct. Deterministic, no retraining.
- Pattern 3 fix (section_type feature): ~24 errors reduced. Requires classifier retrain.
- Pattern 2 fix (dep parsing): ~62 errors reduced. Requires new infrastructure.

## Dependencies

- ✅ Deep-dive analysis done in classifier training session
- ✅ Purpose classifier exists in fractalaw-core/taxa/purpose.rs
- Dependency parsing session for Pattern 2
