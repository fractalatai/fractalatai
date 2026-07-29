---
session: "Benchmark QA"
status: closed
opened: 2026-06-26
closed: 2026-06-26
outcome: success

summary: >
  Rewrote benchmark_report.py to query provision_actors + gold_benchmarks tables.
  Established per-tier baselines: regex 51.2% position / 84.6% DRRP, classifier 57.7%
  position / 84.6% DRRP across 986 matched actors from 4,062 gold entries. Disagreement
  analysis shows classifier is right 60% of the time when tiers disagree. 18.5% of
  matched actors have both tiers agreeing on the wrong answer.

decisions:
  - what: "Benchmark from provision_actors + gold_benchmarks SQL"
    why: "Per-actor per-tier benchmarking requires normalised tables, not JSONB parsing"
    result: "Simple SQL joins for accuracy, confusion matrices, and per-category breakdowns"

lessons:
  - title: "Classifier adds value despite low absolute accuracy"
    detail: "When regex and classifier disagree, classifier is right 60% of the time (195 vs 131). The signal is valuable for LLM elevation targeting."
    tag: pipeline
  - title: "Per-category accuracy varies widely"
    detail: "Classifier better on Ind/Spc/other/EU actors. Regex better on Gvt/Org. Different actor categories need different tier strategies."
    tag: data-quality
---

# Session: Benchmark QA (CLOSED)

## Context

`provision_actors` table enables per-tier, per-actor benchmarking with simple SQL. Gold benchmarks on NAS have 4,061 (section_id, actor_label) pairs with correct DRRP + position.

## Work

1. ✅ Rewrite `benchmark_report.py` to query `provision_actors` + `gold_benchmarks`
2. ✅ Per-tier accuracy — regex 51.2% / classifier 57.7% position, both 84.6% DRRP
3. ✅ Load gold into `gold_benchmarks` Postgres table (4,062 rows)
4. ✅ Position confusion matrix per tier
5. ✅ Per-actor-category breakdown
6. ✅ Disagreement analysis

## Results (from provision_actors + gold_benchmarks, all 20 benchmarks)

| Metric | Regex | Classifier |
|--------|-------|-----------|
| DRRP | 84.6% | 84.6% |
| Position | **51.2%** | **57.7%** |
| Matched actors | 986/4,062 | Same |

### Disagreement analysis (986 actors with both tiers)
| Outcome | Count | % |
|---------|-------|---|
| Agree + correct | 374 | 37.9% |
| Agree + wrong | 182 | 18.5% |
| Disagree, regex right | 131 | 13.3% |
| Disagree, classifier right | 195 | 19.8% |
| Disagree, both wrong | 104 | 10.5% |

Key: when they disagree, classifier is right more often (195 vs 131).
When they agree, 67% correct (374/556). 18.5% agree on the wrong answer.

### Per-category position accuracy
| Category | Total | Regex | Classifier |
|----------|-------|-------|-----------|
| Ind | 398 | 42.2% | 52.8% |
| Gvt | 365 | 59.7% | 55.1% |
| Org | 109 | 72.5% | 70.6% |
| Spc | 47 | 44.7% | 72.3% |
| other | 44 | 11.4% | 72.7% |
| EU | 21 | 66.7% | 71.4% |

Classifier better on Ind, Spc, other, EU. Regex better on Gvt, Org.

## Dependencies

- ✅ provision_actors populated for all 20 benchmarks
- ✅ Both regex and classifier signals populated
