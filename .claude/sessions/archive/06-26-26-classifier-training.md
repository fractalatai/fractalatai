---
session: "Position Classifier Training"
status: closed
opened: 2026-06-26
closed: 2026-06-26
outcome: partial

summary: >
  Trained position classifier v2 with correct features (Obligation/Liberty instead of
  DRPP, 4 classes instead of 3) but only achieved 59.8% CV accuracy. GBT comparison
  showed identical ceiling — bottleneck is feature quality, not model architecture.
  Embedding dominates at 79.9% feature importance. Actor recall identified as the real
  #1 problem (only 35% of gold actors found by regex). Matched actors improved from
  986 to 1,758 through aliases, gold cleanup, and dictionary fixes.

decisions:
  - what: "Feature quality, not model architecture, is the bottleneck"
    why: "LR and LightGBM produce identical 60% accuracy. GBT feature importance shows embedding at 79.9% with other features barely contributing."
    result: "Deferred model changes. Focused on richer features (dependency parsing) and actor recall."
  - what: "Actor recall is the #1 problem"
    why: "Only 35% of gold actors found by regex. Classifier accuracy is misleadingly measured on the 35% that are found."
    result: "Actor recall improved to 46% via aliases, gold cleanup, and dictionary widening. Correlative inference added 615 actors at 86.7% position accuracy."

lessons:
  - title: "65% actor recall makes classifier accuracy misleading"
    detail: "Optimising the classifier on 35% of gold actors ignores the 65% it never sees. Solving recall is prerequisite to meaningful accuracy work."
    tag: data-quality
  - title: "Embedding dominance means per-provision, not per-actor, signal"
    detail: "The 384-dim embedding is the same for all actors in a provision. Need per-actor features (dep parsing, grammatical role) to break past 60%."
    tag: pipeline
---

# Session: Position Classifier Training (CLOSED)

## Context

Position classifier v2 trained with correct features (Obligation/Liberty, 4 classes) but only 59.8% CV accuracy. The classifier writes to `cls_position` in `provision_actors` without overriding regex — it's now a signal, not a decision.

## Work

1. ✅ Evaluate v2 classifier against gold benchmarks (57.7% position, 986/4,062 matched)
2. ✅ Disagreement analysis (classifier right 60% when disagreeing)
3. ✅ Feature importance + GBT comparison
4. ✅ Deep-dive agree+wrong cases (325 cases analysed)
5. ➡️ Better features (dependency parsing) → `cascade/06-26-26-dependency-parsing.md` (PENDING)
6. ⬜ Fine-tuned local LLM as future tier (deferred — feature quality is the bottleneck, not model)

## Deep-dive: 325 agree+wrong cases

Both regex and classifier predict the same wrong position. Three dominant error patterns:

### Pattern 1: mentioned→active (183 cases, 56%)

Actors mentioned in definitions, references, amendments, and structural provisions — not in duty-creating clauses. Both tiers see the actor label + modal language and assume active, but the provision is describing/referencing, not creating a duty.

Examples: "duty of the Scottish Ministers" in a repeal clause, "powers of the Secretary of State" in a cross-reference, HSE described as performing functions "on behalf of the Crown".

Breakdown: Gvt 79, Ind 51, Spc 23, Org 21, EU 9.

### Pattern 2: counterparty→active (62 cases, 19%)

Actors who hold claims (counterparty) but both tiers predict active. The text mentions the actor prominently but in a receiving role — the actor benefits from the provision rather than bearing the duty.

Examples: "authority responsible for maintaining the service" — authority receives the service, Secretary of State bears the duty.

Breakdown: Gvt 35, Ind 16, EU 6, Spc 5.

### Pattern 3: beneficiary→active or counterparty (24 cases, 7%)

Actors who benefit without a direct legal relation but both tiers assign a legal role.

### What would fix these

1. **Provision purpose classification** — is this provision creating a duty, or is it a definition/reference/amendment? The existing purpose classifier could gate this. If purpose=structural/definition, position should be mentioned regardless of actor presence.
2. **Grammatical role** — dependency parsing would show whether the actor is the subject of a duty verb or mentioned in a subordinate clause/cross-reference.
3. **Section type signal** — 151 errors come from sub_article, 128 from sub_section. These structural types are more likely to be references. Currently the classifier has no section_type feature.

## GBT vs LR comparison (2026-06-26)

| Model | CV Accuracy |
|-------|------------|
| LR | 61.0% |
| LightGBM | 60.5% |

GBT is NOT better — same features, same ceiling. Feature importance from GBT:
- Embedding: 79.9%
- Offset: 14.0%
- Category: 4.9%
- DRRP: 1.0%
- Modal: 0.2%

**Conclusion**: bottleneck is feature quality, not model architecture. The embedding dominates. Non-embedding features barely contribute. Need richer features (dependency parsing, grammatical role) or a domain-specific embedding (Legal-BERT) to break past 60%.

## Actor recall analysis (the real #1 problem)

3,076 gold actors NOT found in provision_actors. Breakdown:

| Category | Count | Cause |
|----------|-------|-------|
| Canonical labels regex missed | 200 | Regex has the pattern but text uses indirect reference (e.g. "An employee is entitled..." — employer implied, not stated) |
| Natural-language actors not in dictionary | ~2,500 | "responsible undertaking", "scheme administrator", "Member States", "compliance body", "appellant" etc. |
| Gold quality issues | ~375 est. | Gemini classifying things as actors: "electrical equipment", "civil explosive" |

### Top 30 missing actor labels (from gold)
```
Member States (76), Member State (54), Org: Manufacturer (53),
responsible undertaking (38), undertaking (38), Scottish Ministers (38),
electrical equipment (36), Org: Employer (32), participant (30),
hazardous substances authority (27), Org: Importer (27), appellant (27),
Health and Safety Executive (25), scheme administrator (21),
economic operator (20), relevant persons (20),
Office for Nuclear Regulation (19), Gvt: Authority: Enforcement (18),
compliance body (18), authorised person (17), Company (17),
Commission (16), competent authority (16), Authority (16)
```

### Actions (iterative)
1. ✅ **ALIASES expansion** — shared `scripts/actor_aliases.py` with 80+ mappings. Matched actors: 986 → 1,428 (+45%)
2. ✅ **Gold cleanup** — 107 non-actors removed (electrical equipment, civil explosive, etc.)
3. ➡️ **Regex pattern gaps** — 2,527 actors not found by regex. Moved to `cascade/06-26-26-regex-actor-gaps.md` (PENDING)
4. ➡️ **Implied actors** — included in regex actor gaps session

## Classifier v2 stats

- Architecture: Logistic Regression, softmax 4-class
- Features: 411 dims (embedding 384 + modal 13 + DRRP 3 + category 10 + offset 1)
- Training: 4,060 samples from 1,711 benchmark provisions
- CV accuracy: 59.8%
- Class distribution: active 1,282, counterparty 905, beneficiary 374, mentioned 1,499
- Weights: `docs/position_classifier_v2.json`

## Gemini critical review (2026-06-26)

### Core problems identified

1. **65% actor recall is the #1 problem** — classifier accuracy is misleadingly high because we only classify 35% of gold actors. The rest aren't even found by regex. This must be solved before optimising the classifier.

2. **LR is fundamentally limited** — linear model can't capture non-linear interactions between features. The 384-dim embedding dominates; the other 27 features have minimal impact via LR.

3. **Feature engineering is too superficial** — modal keyword presence and text offset aren't enough. Need grammatical role (subject/object), dependency parsing, clause structure, verb voice (active/passive).

4. **DRRP feature is per-provision, but position is per-actor** — misleading when a provision has multiple actors with different roles. All actors get the same DRRP signal.

5. **Text offset feature is broken** — `text.find(label)` is fragile, 0.5 default is noise. Replace with proper NER span extraction.

### Prioritised action plan (from Gemini)

**Tier 1 — Foundational (must do)**
1. Solve actor recall (65% missed) — custom NER or expanded regex patterns
2. Upgrade embedding — Legal-BERT or domain-specific model instead of all-MiniLM-L6-v2
3. Switch to GBT (XGBoost/LightGBM) — non-linear, handles mixed features
4. Deep-dive the 182 "agree + wrong" cases — find systematic patterns

**Tier 2 — Significant impact**
5. Dependency parsing features — grammatical role, main verb, active/passive voice
6. Fix positional features — NER entity span, not text.find()
7. Address beneficiary class imbalance — oversampling or focal loss

**Tier 3 — Refinements**
8. Make DRRP actor-specific (not per-provision)
9. Coreference resolution (pronouns → canonical actors)
10. Try MLP if GBT plateaus

### Realistic ceiling assessment
80% achievable without LLM IF actor recall is solved + rich features + GBT. The 20% LLM tier handles truly ambiguous cases.

## From benchmark-qa session (all done)

- ✅ `benchmark_report.py` rewritten to query provision_actors + gold_benchmarks
- ✅ `gold_benchmarks` table populated (4,062 rows)
- ✅ Per-actor-category breakdown — classifier better on Ind/Spc/other/EU, regex better on Gvt/Org
- ✅ Disagreement analysis — when disagreeing, classifier right 60% (195 vs 131)

## Since suspension (2026-06-26)

Actor recall blocker largely resolved:
- Matched gold actors: 986 → 1,758 (78% increase)
- ALIASES: 80+ label mappings
- Gold cleanup: 135 non-actors removed
- Dictionary: authority + undertaking patterns widened
- Correlative inference: 615 actors inferred at 86.7% position accuracy

Updated benchmark (all 20 laws, 1,758 matched):

| Tier | Position | DRRP |
|------|----------|------|
| Regex | 51.3% | 87.3% |
| Classifier | 57.4% | 87.3% |
| Inferred | 86.7% | 41.7% |

Gemini Tier 1 item 1 (actor recall) addressed. Ready for items 3-4:
- **Item 3**: Switch to GBT (XGBoost/LightGBM)
- **Item 4**: Deep-dive the 325 agree+wrong cases

## Dependencies

- ✅ provision_actors table with cls_* columns populated
- ✅ Benchmark QA baseline updated: 1,758 matched actors
- ✅ gold_benchmarks table in Postgres
- ✅ Actor recall improved from 24% to 46%
