# Session: Benchmark QA (PENDING)

## Context

`provision_actors` table enables per-tier, per-actor benchmarking with simple SQL. Gold benchmarks on NAS have 4,061 (section_id, actor_label) pairs with correct DRRP + position.

## Work

1. Rewrite `benchmark_report.py` to query `provision_actors` (not legislation_text JSONB)
2. Report per-tier accuracy independently:
   - Regex: DRRP accuracy, position accuracy, actor recall
   - Classifier: DRRP accuracy, position accuracy (where cls_* populated)
   - Reconciled: legal relation accuracy (composite)
3. Load gold benchmarks into a `gold_benchmarks` table in Postgres for SQL joins
4. Position confusion matrix per tier
5. Per-actor-category breakdown (are Gvt actors harder than Org?)
6. Disagreement analysis: where regex and classifier disagree, who's right?

## Current baseline (from provision_actors)

- Regex DRRP: 81.3% (960/1,181 matched actors)
- Regex Position: 47.8% (721/1,509 matched actors)
- 1,509/4,061 gold actors matched by regex (37% recall)

## Dependencies

- provision_actors populated (done)
- Classifier signals populated for benchmarks (done for HSWA, need all 20)
