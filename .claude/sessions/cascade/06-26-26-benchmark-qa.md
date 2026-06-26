# Session: Benchmark QA (ACTIVE)

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

## Baseline (from provision_actors, all 20 benchmarks)

| Metric | Regex | Classifier |
|--------|-------|-----------|
| DRRP | 93.0% | 93.0% |
| Position | **47.2%** | **56.9%** |
| Actor recall | 1,414/4,061 (35%) | Same |

Classifier adds +10% on position over regex. Key patterns:
- Regex never predicts beneficiary/mentioned — assigns active/counterparty to all
- Classifier correctly identifies 143 mentioned + 59 beneficiary that regex can't
- Classifier loses some counterparty accuracy (130 vs 190 correct)
- 2,647 gold actors not found by regex at all (gap-fill candidates)

### Regex position confusion matrix
```
gold↓ pipe→        active  counterparty
     active           477            89
counterparty          100           190
 beneficiary           36            48
   mentioned          393            80
```

### Classifier position confusion matrix
```
gold↓ pipe→        active  counterparty   beneficiary     mentioned
     active           473            53            28            12
counterparty          135           130            20             5
 beneficiary           17             8            59             0
   mentioned          215            67            48           143
```

## Dependencies

- ✅ provision_actors populated for all 20 benchmarks
- ✅ Both regex and classifier signals populated
