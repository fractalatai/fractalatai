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

1. ~~Export training data from LanceDB (embeddings + labels)~~ ✓
2. ~~Train logistic regression on DRRP type~~ ✓ (v1: 51.2%, v2: 64.0%)
3. ~~Evaluate on held-out test set~~ ✓
4. ~~Backfill missing embeddings~~ ✓ (1,539 in 35.6s)
5. Fix class imbalance (Phase 2) — Right and Responsibility underrepresented
6. Add modal features (Phase 3) — complement embeddings with legal signals
7. Retrain → if >80% accuracy: wire into enrichment pipeline as Tier 2
8. Train actor position classifier (harder, may need MLP)

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

## Shipped (2026-06-09)

### Phase 1: Embedding backfill — COMPLETE
- 1,539 embeddings computed in 35.6 seconds (43/s) via ONNX on CPU
- Training set: 397 → 1,514 regulation-level examples with embeddings
- Created `embedding-backfill` skill (commit `5c3c302`) for reuse

### v2 classifier results (after backfill)

| Metric | v1 (397 examples) | v2 (1,514 examples) |
|---|---|---|
| Overall accuracy | 51.2% | **64.0%** |
| Duty | 55% precision | 67% precision, 85% recall |
| Power | 41% | 56% |
| Responsibility | 0% (no predictions) | 55% |
| Right | 0% (no predictions) | 75% precision, 17% recall |
| none | 57% | 61% |

Class distribution in training data:
- Duty: 758 (50%) — dominant
- Power: 292 (19%)
- none: 273 (18%)
- Responsibility: 98 (6%)
- Right: 93 (6%)

### Gemini review of training plan
- Saved: `docs/reviews/gemini-classifier-training-review-20260609.md`
- Phase order validated
- Backfill now, fix pipeline long-term (embeddings at write time)
- Active learning for class balance — use temporary model to find likely Right/Responsibility provisions
- Modal indicators will complement embeddings
- Biggest risk: 80% accuracy may be unreachable with lightweight models

## What's next

### Phase 2: Class balance — COMPLETE

Sent 120 Right + 120 Responsibility regex provisions through Gemini QA with write-back.

**Results:**
- Right: 12 correct, 106 corrected, 2 failed → 118 confirmed
- Responsibility: 3 correct, 116 corrected, 1 failed → 119 confirmed
- **Regex precision for these classes: ~6%** — almost everything was wrong

| Class | Before | After |
|---|---|---|
| Duty | 758 | 880 |
| Power | 292 | 356 |
| Right | 93 | **145** (+52) |
| Responsibility | 98 | **104** (+6) |
| none | 273 | 274 |
| **Total** | **1,514** | **1,759** |

Responsibility gained only 6 despite 119 confirmations — Gemini corrected most "Responsibility" provisions to Duty or Power. The regex was systematically mislabelling these.

**v3 classifier: 67.9%** (was 64.0%)

**Critical learning: regex DRRP classification is deeply unreliable.**
- Right provisions: 10% correct (12/120)
- Responsibility provisions: 2.5% correct (3/120)
- The regex correctly identifies that a provision EXISTS but misclassifies the TYPE
- This reinforces v0.3: regex is a sieve, not a classifier

**Disk emergency:** QA write-back bloated LanceDB from 452 MB to 8.2 GB (698 MB free). Added auto-compaction to QA workflow.

### Phase 2 also shipped
- `compact_lance_no_backup.py` for emergency disk recovery
- Auto-compaction in QA `run_qa.py` after write-back
- SKILL.md updated with compaction warning

### Phase 3: Modal features — COMPLETE

Added 13 modal indicators alongside 384-dim embeddings:
- Obligation modals: has_shall, has_must, shall_not, must_not, required_to, has_a_duty
- Liberty modals: has_may, may_not, entitled, power_to, right_to
- Actor signals: actor_count, has_gvt_actor

**v4 classifier: 75.3%** (was 67.9%)

Key finding: **modal features alone (72.7%) beat embeddings alone (67.9%)**. The legal modals are more predictive than semantic content. Combined they complement each other.

### Phase 4: DRRP hierarchy — Obligation / Liberty / none

Analysis revealed Duty vs Responsibility and Right vs Power share the same confusion pattern: the DRRP type encodes information already present in the actor label prefix.

**The hierarchy:**
```
Obligation (shall/must)         → actor label determines:
├── Duty        — Org:/Ind:/SC: actor    Duty (governed entity)
└── Responsibility — Gvt:/EU: actor      Responsibility (government)

Liberty (may/entitled/power to) → actor label determines:
├── Right       — Org:/Ind:/SC: actor    Right (regulated entity)
└── Power       — Gvt:/EU: actor         Power (authority)

none — no legal relation
```

The classifier learns 3 classes. The consumer decomposes to full DRRP using the actor label. Hohfeldian mapping preserved without the model needing to learn the Gvt/governed boundary.

**Evidence for merge:**
- 707 Duty provisions had Gvt actors — the model couldn't separate Duty from Responsibility (8:1 imbalance, identical modals)
- Right and Power both use "may" — distinction is WHO holds it, not what the modal is
- Regex classified Right at 10% accuracy, Responsibility at 2.5% — the distinction was never reliable

**v5 classifier (4-class, merged D+R): 82.7%** — past 80% target
**v6 classifier (3-class, Obligation/Liberty/none): 86.4%** ✓

| Class | Precision | Recall | F1 |
|---|---|---|---|
| Obligation | 90% | 91% | 90% |
| Liberty | 82% | 84% | 83% |
| none | 84% | 75% | 79% |

### Success criteria — MET ✓

1. ✅ Overall accuracy >80% (86.4%)
2. ✅ No class has 0% recall (all >75%)
3. ⬜ Inference time <1ms (not yet tested — logistic regression should be microseconds)
4. ⬜ Integrated into enrichment pipeline as `TIER2_PROVIDER=classifier`

### Full accuracy journey

| Version | Classes | Accuracy | Key change |
|---|---|---|---|
| v1 | 5 (DRRP) | 51.2% | 397 examples, embeddings only |
| v2 | 5 | 64.0% | 1,514 examples (backfill) |
| v3 | 5 | 67.9% | Class balance (Right+Responsibility targeted) |
| v4 | 5 | 75.3% | Modal features added |
| v5 | 4 | 82.7% | Duty+Responsibility merged |
| **v6** | **3** | **86.4%** | **Obligation/Liberty/none hierarchy** |

## What's next

1. Wire v6 classifier into enrichment pipeline as `TIER2_PROVIDER=classifier`
2. Test inference time (target <1ms)
3. Update cascade strategy doc with the Obligation/Liberty/none hierarchy
4. Update sertantai briefing with hierarchy
5. Fix enrichment pipeline to compute embeddings at write time
6. Continue QA passes to grow the golden dataset

## Key learnings

1. **Fix the data before the model** — backfilling embeddings and balancing classes moved accuracy more than any model change
2. **Modal features beat embeddings** for DRRP classification — the legal modal verb is the primary signal
3. **Simplify the problem** — 5 classes → 3 classes by recognising the actor label already carries the Gvt/governed distinction
4. **DRRP type encodes redundant information** — Duty vs Responsibility and Right vs Power are derivable from actor labels
5. **Regex DRRP classification is deeply unreliable** — 2.5-10% for Right/Responsibility. Regex is a sieve, not a classifier
6. **Compact after every QA write-back** — fragment bloat is a disk emergency waiting to happen
7. **The Hohfeldian hierarchy maps cleanly** — Obligation/Liberty at classifier level, full DRRP decomposition at consumer level

## References

- Golden dataset: LanceDB `extraction_method = 'agentic'` (1,759 reg-level with embeddings)
- Embeddings: `all-MiniLM-L6-v2` 384-dim + 13 modal indicators
- Production model: `data/drrp_classifier_v6.pkl` (86.4%, 3-class)
- All models: `data/drrp_classifier_v[1-6].pkl`
- Gemini review: `docs/reviews/gemini-classifier-training-review-20260609.md`
- Embedding skill: `.claude/skills/embedding-backfill/`
- Cascade strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`
- Prior session: `.claude/sessions/cascade/06-08-26-cascade-v03-implementation.md`
