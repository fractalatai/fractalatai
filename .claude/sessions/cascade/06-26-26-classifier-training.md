# Session: Position Classifier Training (ACTIVE)

## Context

Position classifier v2 trained with correct features (Obligation/Liberty, 4 classes) but only 59.8% CV accuracy. The classifier writes to `cls_position` in `provision_actors` without overriding regex — it's now a signal, not a decision.

## Work

1. ✅ Evaluate v2 classifier against gold benchmarks (57.7% position, 986/4,062 matched)
2. ✅ Disagreement analysis (classifier right 60% when disagreeing)
3. ⬜ Feature importance analysis
4. ⬜ Retrain with GBT if LR doesn't reach >80%
5. ⬜ Fine-tuned local LLM as future tier

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

### Actions needed
1. **Dictionary expansion** — add missing actors (undertaking, appellant, scheme administrator etc.) to actor-dictionary.yaml
2. **Gold cleanup** — remove non-actor entries (electrical equipment, civil explosive) from benchmarks
3. **ALIASES expansion** — map natural-language gold labels to canonical (Member States → EU: Member States, etc.)
4. **Regex improvement** — handle implied actors (employee provision → infer employer)

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

## Dependencies

- ✅ provision_actors table with cls_* columns populated
- ✅ Benchmark QA baseline: regex 51.2%, classifier 57.7% position
- ✅ gold_benchmarks table in Postgres
