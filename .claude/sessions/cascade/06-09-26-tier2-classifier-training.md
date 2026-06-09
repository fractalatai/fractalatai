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

## Challenge: First Training Attempt

### Baseline results

Logistic regression on 384-dim embeddings, 5-class DRRP type classification:

| Metric | Value |
|---|---|
| Dataset | 397 examples (agentic with embeddings) |
| Train/Test | 317 / 80 |
| **Overall accuracy** | **51.2%** |

| Class | Precision | Recall | Support |
|---|---|---|---|
| Duty | 0.55 | 0.79 | 28 |
| Power | 0.41 | 0.39 | 18 |
| Responsibility | 0.00 | 0.00 | 7 |
| Right | 0.00 | 0.00 | 6 |
| none | 0.57 | 0.57 | 21 |

### Root causes

**1. Embedding coverage gap**: 1,515 agentic provisions exist but only 397 have embeddings. 1,118 (74%) have no embedding vectors — they were enriched by Gemini Tier 2 or QA write-back which updates DRRP/actors but doesn't compute embeddings. The golden dataset is large but the trainable subset is small.

**2. Class imbalance**: Responsibility (7 test) and Right (6 test) have too few examples. The model can't learn these classes and predicts zero for both. The Gemini enrichment was dominated by Duty provisions — the most common DRRP type in UK safety law.

**3. Semantic overlap in embedding space**: Provisions about duties and rights often use similar language — the difference is a single modal word ("shall" vs "may"). The 384-dim text embedding captures overall topic similarity but doesn't isolate the legal modal that determines DRRP type. A duty provision and a power provision about the same subject (e.g., workplace safety) will have very similar embeddings.

### What we won't do

- **Use regex-labelled provisions for training** — regex is only 22% accurate on multi-actor provisions. Training on unreliable labels would teach the model to reproduce regex errors.
- **Optimise the model architecture yet** — too early. The data problems must be fixed first. Tuning hyperparameters or switching to MLP won't help when Responsibility has 7 examples.

## Plan: Fix the Data Before the Model

### Phase 1: Fill the embedding gap

1,118 agentic provisions need embeddings computed. These provisions have Gemini-verified DRRP types and actor positions — high-quality labels — but no feature vectors for training.

**Action**: Run the embedding model (`all-MiniLM-L6-v2`) on the 1,118 provisions that have `extraction_method = 'agentic'` but `embedding IS NULL`. This is a CPU-bound ONNX inference task (~10 minutes), no API cost. The embedding pipeline already exists (`fractalaw embed`).

**Expected outcome**: Training set grows from 397 → ~1,515.

### Phase 2: Balance the classes

After filling embeddings, the class distribution will still be skewed toward Duty. We need more Responsibility, Right, and Power examples.

**Action**: Target enrichment + QA on laws known to contain these DRRP types:
- **Right**: Employment rights SIs (working time, parental leave, discrimination)
- **Responsibility**: Government-facing regulations (environmental permits, planning)
- **Power**: Enforcement provisions, inspector powers, ministerial orders

Use `TIER2_PROVIDER=gemini` on small targeted laws from these families, then QA with write-back. Each pass adds confirmed examples in the underrepresented classes.

**Target**: At least 50 examples per class in the training set.

### Phase 3: Feature engineering

If embeddings alone don't separate classes after phases 1-2, augment the feature vector with legal-domain signals:
- Modal word indicators (has_shall, has_must, has_may, has_power_to, has_entitled)
- Actor count (single vs multi)
- Government actor present (yes/no) — correlates with Responsibility
- Section type (article vs sub_article)

These are cheap to compute (regex on text) and encode the exact signals that differentiate DRRP types. The embedding captures topic; the indicators capture obligation type.

### Phase 4: Retrain and evaluate

With ~1,500 examples across balanced classes and augmented features:
- Logistic regression as baseline
- If <80%: try MLP (2-layer)
- If <80%: try kNN (nearest-neighbour, no training)
- Target: >80% accuracy overall, >60% per class

### Success criteria

The Tier 2 classifier is ready for production when:
1. Overall accuracy >80% on held-out test set
2. No class has 0% recall (every DRRP type gets some predictions)
3. Inference time <1ms per provision
4. Integrated into the enrichment pipeline as `TIER2_PROVIDER=classifier`

## References

- Golden dataset: LanceDB `extraction_method = 'agentic'` (1,515 reg-level, 397 with embeddings)
- Embeddings: `all-MiniLM-L6-v2` 384-dim, already in LanceDB for 96K provisions
- Baseline model: `data/drrp_classifier_v1.pkl` (51.2% accuracy — not production-ready)
- Cascade strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`
- Prior session: `.claude/sessions/cascade/06-08-26-cascade-v03-implementation.md`
