# Session: Actor Position Coverage (PENDING)

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

## Key files

- `fractalaw-ai/src/position_classifier/` — trained position model
- `docs/position_classifier_v*.json` — model weights
- `fractalaw-cli/src/main.rs:5271` — position classifier skips empty actors
- `fractalaw-core/src/taxa/duty_patterns_rule.rs` — thing-subject regex (Rule tier)
- `fractalaw-actors-struct-migration.md` — position definitions (Hohfeldian model)
