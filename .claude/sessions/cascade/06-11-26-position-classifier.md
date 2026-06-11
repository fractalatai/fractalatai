# Session: Position Classifier — Actor Role Prediction

## Context

**Prior session**: `.claude/sessions/cascade/06-09-26-sync-watch-enrichment.md`
**Trigger**: QA on Baserow revealed actor positions are wrong — s.3(1) HSWA has employer as counterparty instead of active. The regex position heuristic was fixed (`ae822a7`) but the classifier doesn't predict positions at all.

## Problem

The DRRP classifier predicts Obligation/Liberty/none but says nothing about which actor holds the obligation. The regex heuristic now correctly assigns the matched actor as active, but:

1. The classifier overwrites regex actors with its own merge_insert (positions lost)
2. Multi-duty-bearer provisions (e.g. "duty of every employer and every self-employed person") only get one active actor from regex
3. Non-regex provisions (structural fragments, inherited) have no position data

## Design: Position Classifier with Dual-Write

### Key insight from user

Instead of overriding regex positions, **capture the difference in the actors struct**. This provides:
- **QA signal**: where regex and classifier disagree, flag for review
- **Training feed**: disagreements become candidate training data for Gemini
- **Provenance**: each actor carries both regex-assigned and classifier-assigned positions

### Actor struct extension

Current actors struct:
```json
{"label": "Org: Employer", "position": "active", "relates_to": null, "label_source": "canonical", "reason": null}
```

Extended with classifier position:
```json
{
  "label": "Org: Employer",
  "position": "active",              // regex-assigned (existing)
  "position_source": "regex",        // who assigned the position
  "classifier_position": "active",   // classifier prediction (new)
  "classifier_confidence": 0.92,     // classifier confidence (new)
  "relates_to": null,
  "label_source": "canonical",
  "reason": null
}
```

When regex and classifier agree → high confidence, ship as-is.
When they disagree → flag for QA, potential Gemini training example.

### Training data

We have ~2,200 agentic provisions with Gemini-classified positions (the gold standard). These are the training set for the position classifier.

Features per (provision, actor) pair:
- 384-dim embedding of the provision text
- Actor label (one-hot or embedding of the label)
- Actor category (Org/Ind/Gvt/SC/Spc/etc.)
- Position of actor in text (relative offset: start/middle/end)
- DRRP type of the provision (Duty/Right/Responsibility/Power)
- Modal features (has_shall, has_must, etc.)

Target: position (active/counterparty/beneficiary/mentioned)

### Architecture

Binary classifier per position class, or multi-class:
- **Option A**: 4-class classifier (active/counterparty/beneficiary/mentioned)
- **Option B**: Binary "is this actor active?" classifier — simpler, most useful

Option B is pragmatic — the main signal sertantai needs is "who bears the obligation".

## Implementation Plan

### Step 1: Extract training data from agentic provisions

Query LanceDB for provisions with `extraction_method = 'agentic'` and actors with position labels. Build training examples: (provision_text, actor_label, drrp_type, features) → position.

### Step 2: Train position classifier

Python scikit-learn (same approach as DRRP classifier):
- LogisticRegression on embedding + actor features + modal features
- Train/test split on provision-level (not actor-level, to avoid leakage)
- Evaluate: precision/recall for active vs counterparty

### Step 3: Export to JSON weights

Same pattern as DRRP classifier — `softmax(X @ W + b)` from JSON.

### Step 4: Wire into enrichment pipeline

After DRRP classification, run position classifier per (provision, actor) pair. Write `classifier_position` and `classifier_confidence` to actors struct. Don't overwrite regex `position` — dual-write.

### Step 5: QA report — disagreements

Query for provisions where `position != classifier_position`. These are:
- Candidates for dictionary/heuristic improvement
- Training data for Gemini re-classification
- QA review items for human validation
