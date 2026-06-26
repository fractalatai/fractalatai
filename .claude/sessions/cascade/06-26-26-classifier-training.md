# Session: Position Classifier Training (PENDING)

## Context

Position classifier v2 trained with correct features (Obligation/Liberty, 4 classes) but only 59.8% CV accuracy. The classifier writes to `cls_position` in `provision_actors` without overriding regex — it's now a signal, not a decision.

## Work

1. Evaluate v2 classifier against gold benchmarks using provision_actors
2. Analyse where classifier disagrees with regex — is the classifier or regex right?
3. Feature importance analysis (are non-embedding features contributing?)
4. Consider retraining with larger dataset if benchmarks supply enough data
5. Try GBT (XGBoost/LightGBM) if LR doesn't reach >80% — Gemini recommended this as fallback
6. Consider fine-tuned local LLM (gemma3:4b) as future tier between classifier and Gemini

## Classifier v2 stats

- Architecture: Logistic Regression, softmax 4-class
- Features: 411 dims (embedding 384 + modal 13 + DRRP 3 + category 10 + offset 1)
- Training: 4,060 samples from 1,711 benchmark provisions
- CV accuracy: 59.8%
- Class distribution: active 1,282, counterparty 905, beneficiary 374, mentioned 1,499
- Weights: `docs/position_classifier_v2.json`

## Gemini review feedback

- Fix data/logic first — LR may suffice once features work correctly
- Non-embedding features matter — modal, DRRP, category will contribute
- 2,200 provisions adequate for LR. Watch beneficiary class imbalance
- Confidence thresholding for LLM escalation (< 0.7 → escalate)
- GBT as fallback if LR < 80%

## From benchmark-qa session (all done)

- ✅ `benchmark_report.py` rewritten to query provision_actors + gold_benchmarks
- ✅ `gold_benchmarks` table populated (4,062 rows)
- ✅ Per-actor-category breakdown — classifier better on Ind/Spc/other/EU, regex better on Gvt/Org
- ✅ Disagreement analysis — when disagreeing, classifier right 60% (195 vs 131)

## Dependencies

- ✅ provision_actors table with cls_* columns populated
- ✅ Benchmark QA baseline: regex 51.2%, classifier 57.7% position
- ✅ gold_benchmarks table in Postgres
