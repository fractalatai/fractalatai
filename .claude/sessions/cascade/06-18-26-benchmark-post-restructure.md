# Session: Benchmark Post-Restructure (SUSPENDED)

## Resume Point (2026-06-18)

**Blocker**: Lance panic on multi-law `taxa parse --force` — List<Struct> offset corruption in `drrp_history` column (#45). Single-law runs work. Multi-law panics. Need to either drop and re-add the column as empty, or downgrade to a simpler type (JSON string).

**To resume**: fix #45 first, then re-run the benchmark.

### Progress before suspension

- ✅ `taxa parse --laws UK_ukpga_1974_37 --force` — works (single law)
- ✅ `taxa classify --laws UK_ukpga_1974_37` — works (118 classified, 116 flagged for LLM)
- ❌ `taxa parse --laws <all 16 benchmark laws> --force` — panics on second/third law
- Workaround applied: removed drrp_history from regex parse write path
- Still panics — the corrupted drrp_history data from migration causes Lance read failures during merge_insert

### Fix needed before resuming

Drop `drrp_history` column from LanceDB and rebuild table. Then re-add as empty column. The migrated data (134K pre-populated entries) is causing the corruption. The column will be populated correctly by the classifier pass going forward.

```bash
# When resuming:
# 1. Drop drrp_history, rebuild table
# 2. Re-run migration with empty history (no bootstrapping)
# 3. taxa parse --laws <benchmark_laws> --force
# 4. taxa classify --laws <benchmark_laws>
# 5. Compact
# 6. Run benchmark
```

## Context

**Prior session**: cascade-transition-rules (CLOSED) — pipeline restructured into `taxa parse → taxa classify → taxa escalate`.

**Trigger**: Need to verify the restructured pipeline produces correct results. Run the benchmark against the corrected 3-class gold standard (`data/benchmarks/`). Last benchmark was 86.3% accuracy before the restructure.

## Benchmark laws

UK_eudr_2013_59, UK_ukpga_1974_37, UK_uksi_2005_1541, UK_uksi_2015_310, UK_ukpga_2004_20, UK_uksi_2014_1638, UK_uksi_2016_1101, UK_uksi_1999_3242, UK_uksi_2002_2788, UK_asp_2005_13, UK_uksi_2010_2214, UK_uksi_2006_1380, UK_eudr_2014_68, UK_ukpga_1990_10, UK_ukpga_1981_69, UK_ukpga_1997_8
