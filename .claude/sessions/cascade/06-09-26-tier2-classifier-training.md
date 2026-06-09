# Session: Tier 2 Embedding Classifier Training

## Context

**Prior session**: `.claude/sessions/cascade/06-08-26-cascade-v03-implementation.md`
**Strategy**: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`

Golden dataset exceeded target: 1,515 regulation-level confirmed examples across 99 laws and 21+ regulatory families. All stamped `agentic` at 0.90 confidence from Gemini Tier 2 enrichment and QA write-back.

## Objective

Train a lightweight classifier on the 384-dim embeddings already in LanceDB. This replaces LLM calls for Tier 2 — microsecond inference, zero API cost, runs alongside the enrichment pipeline.

## Training data

- **Source**: LanceDB provisions where `extraction_method = 'agentic'`
- **Features**: 384-dim embedding vectors (already computed, `all-MiniLM-L6-v2`)
- **Labels**: DRRP type + actor positions from the actors struct
- **Size**: 1,515 regulation-level examples across 99 laws
- **Split**: 80% train / 10% val / 10% test (stratified by family)

## What to classify

### Task 1: DRRP type
- Input: 384-dim embedding
- Output: Duty | Right | Responsibility | Power | none
- This is the primary classification — currently 100% accurate from regex for single-actor

### Task 2: Actor position (per actor)
- Input: 384-dim embedding + actor label
- Output: active | counterparty | beneficiary | mentioned
- This is where regex fails on multi-actor provisions

## Model options

| Approach | Complexity | Expected quality |
|---|---|---|
| Logistic regression | Minimal | Good baseline |
| MLP (2-layer) | Low | Better decision boundaries |
| kNN on embeddings | Zero training | Nearest-neighbour transfer |

Start with logistic regression — simplest, interpretable, fast to train and evaluate.

## Implementation plan

1. Export training data from LanceDB (embeddings + labels)
2. Train logistic regression on DRRP type
3. Evaluate on held-out test set
4. If >80% accuracy: wire into enrichment pipeline as Tier 2
5. Train actor position classifier (harder, may need MLP)

## Integration

The trained model replaces the Ollama/Gemini Tier 2 call:
```
Tier 1: Regex sieve (free)
    ↓ multi-actor or DRRP=none
Tier 2: Embedding classifier (microseconds, no API)
    ↓ low confidence
Tier 3: Gemini QA (customer laws only)
```

## References

- Golden dataset: LanceDB `extraction_method = 'agentic'` (1,515 reg-level)
- Embeddings: `all-MiniLM-L6-v2` 384-dim, already in LanceDB
- Cascade strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`
- Prior session: `.claude/sessions/cascade/06-08-26-cascade-v03-implementation.md`
