---
session: "Actor Position Coverage"
status: closed
opened: 2026-06-22
closed: 2026-06-23
outcome: success

summary: >
  Fixed two actor position gaps: extended inheritance to orphan provisions (DRRP but no
  actors), and mapped classifier \"other\" predictions to \"mentioned\" instead of keeping
  regex position. 4,237 orphans identified corpus-wide. Position classifier confirmed as
  3-class (active/counterparty/other) with \"other\" never written before this fix.

decisions:
  - what: "Extend inheritance to orphaned provisions"
    why: "138 provisions had DRRP but zero actors because inheritance only ran on provisions with no DRRP AND no actors"
    result: "Orphans now inherit actors from parent clause during taxa escalate, keeping existing DRRP."
  - what: "Map classifier \"other\" to \"mentioned\""
    why: "Classifier had 3 classes but \"other\" was never written. When classifier disagrees and predicts other, the position should reflect that."
    result: "Position written as mentioned when classifier predicts other and disagrees with regex."

lessons:
  - title: "Check what the classifier actually produces, not what it was designed for"
    detail: "The position classifier was trained with 3 classes but only 2 were ever used. The other class was discovered to be silently discarded."
    tag: debugging
---

# Session: Actor Position Coverage (CLOSED)

## Problem

Benchmark actor position accuracy is 34.3% (70/204). The `beneficiary` and `mentioned` positions are never predicted — both show 0% precision and 0% recall, dragging the overall score.

### Confusion matrix

| gold \ pipeline | active | beneficiary | counterparty | mentioned |
|-----------------|--------|-------------|--------------|-----------|
| active | 37 | 0 | 15 | 0 |
| beneficiary | 15 | 0 | 17 | 0 |
| counterparty | 10 | 0 | 33 | 0 |
| mentioned | 54 | 0 | 23 | 0 |

The pipeline only produces `active` and `counterparty` — the position classifier was trained on a 2-class model (active vs counterparty). It has no concept of `beneficiary` or `mentioned`.

## Two sub-problems

### A. Position classifier only predicts active/counterparty

The confusion matrix shows `beneficiary` and `mentioned` are never predicted. The position classifier was trained on a 2-class model. Needs either retraining with 4 classes or heuristic fallback.

### B. Thing-subject orphans — 138 provisions with no actors at all

Discovered during Rule class cleanup (06-22-26-rule-class-cleanup.md):

- 176 provisions classified via the Rule regex tier (thing-subject obligations: "equipment shall be suitable", "routes must be...")
- 38 happened to have actors from regex (person keywords co-present)
- **138 have zero actors** — the position classifier skips them (`if actors.is_empty() { continue; }`)
- These are now Obligation (Rule→Obligation remap done) but have no duty-holder

The thing-subject regex detects the coarse obligation signal but cannot identify the implied actor. Only the classifier or LLM tier can resolve who the duty-holder is from context (usually the employer or responsible person from a parent clause).

**Options for resolving orphans:**
1. Flag with `extraction_method = "pending_llm"` so LLM tier picks them up
2. Have classifier attempt actor inference from embedding (new capability)
3. Inherit actors from parent clause (`inherited` tier already does this for other provisions)
4. Accept them as actor-orphaned Obligations — the DRRP signal is valid even without a resolved actor

## Investigation plan

1. Confirm the position classifier only outputs active/counterparty
2. Quantify orphaned thing-subject provisions across the full corpus (not just benchmark laws)
3. Determine whether beneficiary/mentioned can be derived heuristically
4. Evaluate whether parent-clause inheritance would naturally resolve most thing-subject orphans
5. Check how many benchmark provisions have beneficiary/mentioned gold labels — is 77 + 32 enough to train on?

## Investigation findings (2026-06-23)

### Position classifier: 3-class (active/counterparty/other)

Confirmed from `docs/position_classifier_v1.json`: classes = `["active", "counterparty", "other"]`. The "other" class exists but was **never written** — the classifier logged its opinion in the `reason` field but always kept the regex position.

### The classifier never overwrites positions

Line 5863 (pre-fix): `regex_pos.clone()` — always keeps regex position regardless of classifier prediction. The classifier is effectively a provenance annotation, not a decision-maker for positions.

### 4,237 orphans across full corpus

Provisions with DRRP but no actors:
- regex: 3,649 (thing-subject obligations)
- classifier: 293 (gap-filled DRRP but no actor to assign)
- pending_llm: 182
- unclassified: 113

### Inheritance gap

`enrich_single_law` inheritance only ran on provisions with no DRRP AND no actors. Orphans (DRRP but no actors) were excluded because the filter required `p.drrp_types.is_empty()`.

## Fixes applied (`9cd8b84`)

1. **Inheritance extended to orphans**: Provisions with DRRP but no actors now inherit actors from parent clause during `taxa escalate`. Keeps existing DRRP, only inherits actors.

2. **Classifier "other" → "mentioned"**: When the position classifier disagrees with the regex and predicts "other", the position is now written as "mentioned" instead of keeping the regex position.

These fixes don't require retraining the classifier. The remaining gap (beneficiary never predicted) would need either 4-class retraining or LLM assignment — both handled by the `taxa validate` + `/human-review` pipeline.

## Key files

- `fractalaw-ai/src/position_classifier.rs` — 3-class model (active/counterparty/other)
- `docs/position_classifier_v1.json` — model weights
- `fractalaw-cli/src/main.rs:5697-5875` — Phase 4 position classification
- `fractalaw-cli/src/main.rs:3692-3762` — inheritance escalation (now covers orphans)
