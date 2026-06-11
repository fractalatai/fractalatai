# Session: Position Classifier — Actor Role Prediction (CLOSED)

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

### Actor struct extension — overload `reason` field

**Rejected approach**: separate LanceDB columns (`classifier_positions`, `position_agreement`). Adds schema complexity, separates the signal from the actor it belongs to.

**Adopted approach**: use the existing `reason` field in the actors struct to carry the classifier's position prediction when it disagrees with the regex position. No schema migration needed — `reason` is already nullable Utf8.

Current actors struct:
```json
{"label": "Org: Employer", "position": "active", "relates_to": null, "label_source": "canonical", "reason": null}
```

When regex and classifier **agree**, `reason` stays null:
```json
{"label": "Org: Employer", "position": "active", "label_source": "canonical", "reason": null}
```

When regex and classifier **disagree**, `reason` carries the classifier signal:
```json
{"label": "Org: Employer", "position": "counterparty", "label_source": "canonical", "reason": "classifier:active@0.82"}
```

This provides:
- **QA signal**: `reason` is non-null → regex and classifier disagree, needs review
- **Training feed**: disagreements are candidate examples for Gemini re-classification
- **Provenance**: the regex position stays in `position`, the classifier prediction is in `reason`
- **No schema change**: `reason` field already exists in the Arrow List<Struct>
- **Sertantai compatibility**: sertantai already receives `reason` in the actors struct

**Convention for `reason` field**:
- `null` — no classifier signal (agree or not yet classified)
- `"classifier:active@0.82"` — classifier predicts active with 82% confidence
- `"classifier:counterparty@0.71"` — classifier predicts counterparty with 71% confidence
- `"classifier:other@0.65"` — classifier predicts other (beneficiary/mentioned) with 65% confidence
- Existing LLM reasons (from agentic tier) remain as free text — no prefix

Sertantai can detect classifier disagreements by checking `reason.startsWith("classifier:")`.

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

### Step 1: Extract training data from agentic provisions — DONE

Queried 1,815 agentic provisions with 3,258 actor-position pairs. Features: embedding(384) + modal(13) + drrp(5) + category(10) + offset(1) = 413 dims.

### Step 2: Train position classifier — DONE

3-class LogisticRegression (active/counterparty/other):
- 74% precision, 70% recall on active class
- 68% overall accuracy
- Trained on Gemini gold-standard agentic data

### Step 3: Export to JSON weights — DONE

`docs/position_classifier_v1.json` (26 KB). Same softmax(X @ W + b) pattern as DRRP classifier.

### Step 4: Wire detection into enrichment pipeline — DONE (`0f8ee7a`)

Phase 4 in embed+classify pass. Guardrails from Gemini review:
- **Skip agentic provisions** — already gold-standard, don't touch
- **Don't overwrite LLM reasons** — if `reason` has non-classifier content, leave it
- **Don't auto-override `position`** — regex position stays as source of truth
- **Detection only** — counts disagreements, doesn't write back yet

HSWA test: 883 actors classified, 346 provision disagreements (39%).

### Step 5: Write-back — reason field update — NEXT

On disagreement, write `reason = "classifier:active@0.82"` into the actors struct. Requires rebuilding the Arrow `List<Struct>` per provision and merge_insert.

Sertantai detects disagreements via `reason.startsWith("classifier:")`.

### Step 6: QA report — disagreements

Query for provisions where any actor has `reason` starting with `classifier:`. These are:
- Candidates for dictionary/heuristic improvement
- Training data for Gemini re-classification (human-reviewed first)
- QA review items for human validation

## Progress

### Committed: `c9a77e4` — Position classifier module + schema

- `PositionClassifier` in `fractalaw-ai/src/position_classifier.rs`
- `build_position_features()` helper (413-dim vector)
- JSON weights at `docs/position_classifier_v1.json`
- 2 unit tests passing

### Committed: `0f8ee7a` — Position detection wired into enrich --pending

- Phase 4 runs per (provision, actor) during embed+classify pass
- Skip agentic, skip existing LLM reasons
- Counts disagreements (HSWA: 883 actors, 346 disagreements)
- Reverted: removed `classifier_positions`/`position_agreement` LanceDB columns (no schema change needed — using actors struct `reason` field instead)

### Committed: `ddec2c6` — Position classifier write-back

**Full write-back implemented.** On disagreement, rebuilds the Arrow `List<Struct>` for the actors column and writes `reason = "classifier:active@0.82"` via merge_insert.

Guardrails:
- Skip agentic provisions (gold-standard positions from Gemini)
- Don't overwrite existing LLM reasons (non-`classifier:` prefixed)
- Only write on disagreement (reason stays null when regex + classifier agree)

HSWA verified:
```
s.3(1): Org: Employer  position=active        reason=None              (agree)
s.3(1): Ind: Person    position=counterparty   reason=classifier:other@0.70  (disagree)
```
346/899 HSWA provisions have at least one actor disagreement (39%).

**Convention**: sertantai detects disagreements via `reason.startsWith("classifier:")`. Format: `classifier:{predicted_position}@{confidence}`.

### Next

- [ ] Step 6: QA report — query disagreements across QQ corpus, quantify by actor category
- [ ] Re-enrich + publish QQ corpus with position classifier active
- [ ] Retrain with more data — use human-validated disagreements to improve from 70% recall
