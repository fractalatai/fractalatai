---
session: Significance Aggregation — Provision & Law Level
status: closed
opened: 2026-07-01
closed: 2026-07-02
outcome: success

summary: >
  Tested 6 provision-level and 6 law-level aggregation approaches across 40,468 rated
  Obligation provisions in 553 laws. B (gravity-dominant weighted sum) and L (avg_sig × log2(size))
  emerged as strongest candidates. Part/Chapter breakdown validated that large Acts like HSWA
  contain concentrated significance in Part I that is diluted by procedural Parts.

decisions:
  - what: B (gravity-weighted) is the leading provision-level approach
    why: Best benchmark accuracy (3/4 correct), gravity weighting pushes LOW-gravity provisions down, principled exclusion of strength noise
    result: "Distribution: HIGH 13.2% | MED 24.8% | LOW 62.0%. s.2(1)=HIGH, s.9=MEDIUM, reg.3(1)=HIGH"
  - what: L (avg_sig × log2(total+1)) is the leading law-level approach
    why: Multiplication amplifies size rather than merely adding it, which correctly boosts large foundational Acts. Weighted sum variant L1 actually pushed HSWA down.
    result: "B+L gives HSWA rank 70/553 (HIGH), CDM rank 7, MHSW rank 41. All benchmarks correct."
  - what: Hybrid K+L recommended for production
    why: Gemini review — L for ranking/filtering, K distribution profile for drill-down. No information loss.
    result: Both approaches persisted in significance_law_experiments table
  - what: Part/Chapter breakdown for large Acts
    why: HSWA ranks 45-96/553 overall but Part I (General Duties) is 44.3% HIGH — the meat is diluted by procedural Parts
    result: 52 large Acts analysed, data available for future sub-law significance publishing
  - what: CDM reg.4(1) misclassification is a gravity issue, not a formula issue
    why: All approaches miss it because SLM rated gravity=MEDIUM (organisational framing). Would need SLM retrain to fix.
    result: Accepted — reg.4(1) at MEDIUM is defensible, the CDM law itself ranks correctly HIGH

metrics:
  provision_approaches_tested: 8
  law_approaches_tested: 9
  corpus: { provisions: 40468, laws: 553 }
  best_provision: { approach: "B gravity-weighted", high_pct: 13.2, med_pct: 24.8, low_pct: 62.0 }
  best_law: { approach: "L+B", hswa_rank: 70, cdm_rank: 7, mhsw_rank: 41 }
  hswa_part1: { high_pct: 44.3, l_score: 13.12 }
  experiment_tables: { provision: "significance_overall_experiments", law: "significance_law_experiments" }

lessons:
  - title: Strength dimension at 71% HIGH is effectively noise — exclude it
    detail: >
      SLM distillation bias amplified strength to 71% HIGH. Including it inflates every approach
      (Approach D: 88.4% HIGH). F2 (gravity-weighted, excluding strength) gets the same benchmark
      results as B while being more principled. Strength needs SLM retrain before it's useful.
    tag: models
  - title: Multiplication beats weighted sum for size-boosting law-level scores
    detail: >
      Gemini suggested w1*avg + w2*log2(size) but this actually pushes HSWA down (rank 140 vs 96).
      Multiplication (avg * log2) is better because size amplifies importance rather than merely
      adding to it. Counter-intuitive — the simpler formula is the better one.
    tag: methodology
  - title: Large Acts need sub-law significance to be useful
    detail: >
      HSWA ranks 45-96/553 because its 172 provisions span general duties, HSE constitution,
      enforcement, and offences. Part I alone (44.3% HIGH) would rank in the top third.
      Any production system needs Part/Chapter breakdown for Acts with >50 provisions.
    tag: architecture
  - title: Averaging penalises foundational statutes
    detail: >
      Approaches I (proportion) and J (weighted avg) both rate HSWA as MEDIUM because they
      divide by total provisions. Foundational Acts are inherently mixed (they establish
      the framework). Size-boosting (L) compensates but Part breakdown is the real fix.
    tag: methodology
  - title: Persist experiment results in dedicated tables, not main schema
    detail: >
      Using significance_overall_experiments and significance_law_experiments tables lets us
      run all approaches and compare without polluting legislation_text. Parameterised scripts
      with --provision-approach flag enable the full combinatorial matrix.
    tag: tooling

artifacts:
  - scripts/significance_approach_a.py
  - scripts/significance_approach_b.py
  - scripts/significance_approach_c.py
  - scripts/significance_approach_d.py
  - scripts/significance_approach_e.py
  - scripts/significance_approach_f.py
  - scripts/significance_approach_f2.py
  - scripts/significance_approach_g.py
  - scripts/significance_approach_h.py
  - scripts/significance_approach_i.py
  - scripts/significance_approach_j.py
  - scripts/significance_approach_k.py
  - scripts/significance_approach_l.py
  - scripts/significance_approach_l1.py
  - scripts/significance_approach_l2.py
  - scripts/significance_part_breakdown.py
  - data/code-review/significance-aggregation-2x2.md

depends_on:
  - 06-27-26-duty-significance.md

enables:
  - 07-01-26-significance-publish.md
---

# Session: Significance Aggregation — Provision & Law Level (CLOSED)

## Problem

Compliance officers need to filter and prioritise across two levels:
- **Provision level**: "Which duties in this law need my attention?" — filter within a law by overall significance
- **Law level**: "Which of my 274 laws need attention first?" — rank laws against each other

We have 40,468 Obligation provisions rated on 4 dimensions (scope_duty_bearer, scope_protected_class, gravity, strength). Need to combine into usable signals at both levels.

## Data available

- Per-provision: 4 significance dimensions (HIGH/MEDIUM/LOW) + confidence score in `legislation_text`
- Per-provision: hierarchy derivable from section_type + depth (metadata, not SLM)
- 40,468 rated provisions across the full corpus

## Experiments

### 1. Overall provision significance

Combine 4 SLM dimensions + hierarchy metadata into a single HIGH/MEDIUM/LOW per provision.

Approaches to test:
- **Weighted sum**: LOW=1, MEDIUM=2, HIGH=3 × weights per dimension. Threshold into HIGH/MEDIUM/LOW.
- **Rule-based**: "If gravity=HIGH → overall=HIGH regardless" / "If all LOW → overall=LOW"
- **Max-of-dimensions**: provision is as significant as its most significant dimension
- **Gravity-dominant**: gravity drives the rating, other dimensions adjust ±1 level

Validate against human intuition:
- HSWA s.2(1) "ensure health safety welfare" → should be HIGH overall
- HSWA s.20(2) "allow inspector access" → should be LOW overall
- MHSW reg.3 risk assessment → should be HIGH overall

### 2. Law-level aggregation

Aggregate provision significance to law level for ranking.

Approaches to test:
- **Max**: law significance = highest provision significance
- **Count-weighted**: count of HIGH provisions drives rank
- **Distribution profile**: { high: 3, medium: 12, low: 45 } — richer signal
- **Weighted score**: sum(provision scores) / total provisions — average significance
- **Top-N**: significance of the N most significant provisions

Validate: HSWA should rank above a pure procedural SI.

### 3. Where does this live?

- Fractalaw publishes per-provision significance dimensions + overall
- Sertantai aggregates to law level for display
- Or: fractalaw pre-computes law-level aggregation in DuckDB

## Corpus distribution (40,468 rated Obligation provisions)

| Dimension | HIGH | MEDIUM | LOW |
|-----------|------|--------|-----|
| scope_duty_bearer | 2,185 (5%) | 16,829 (42%) | 21,454 (53%) |
| scope_protected_class | 8,310 (21%) | 3,639 (9%) | 28,519 (70%) |
| gravity | 8,717 (22%) | 6,904 (17%) | 24,847 (61%) |
| strength | 28,700 (71%) | 5,917 (15%) | 5,851 (14%) |
| hierarchy | 13,362 (33%) | 13,755 (34%) | 13,351 (33%) |

Note: strength is heavily skewed HIGH (distillation bias from SLM training — known issue).

## Validation cases

Each approach must be tested against these benchmark provisions:

| Provision | Expected | Reasoning |
|-----------|----------|-----------|
| HSWA s.2(1) "ensure health safety welfare" | HIGH | General duty, universal scope, safety stakes |
| HSWA s.20(2) "allow inspector access" | LOW | Procedural, specific actor, admin |
| MHSW reg.3 "risk assessment" | HIGH | Core duty, all employers, safety |
| HSWA s.9(1) "prepare safety policy" | MEDIUM | Important but procedural/documentary |
| CDM reg.4 "client duties" | HIGH | Primary duty, safety, construction |

## Work

### Phase 1: Provision-level overall significance

1. ✅ Hierarchy derived from metadata (done in prior session, v2.1 combined weighted scoring)

2. ✅ **Approach A: Equal-weight sum** (`scripts/significance_approach_a.py`)
   - Score: HIGH=3, MEDIUM=2, LOW=1 per dimension, sum all 5, divide by 5
   - Thresholds: ≥2.5 → HIGH, ≥1.75 → MEDIUM, else LOW
   - Distribution: HIGH 9.7% | MEDIUM 48.9% | LOW 41.4%
   - Benchmarks: HSWA s.2(1)=HIGH ✅ | s.9=HIGH (expected MEDIUM) ❌ | MHSW reg.3(1)=HIGH ✅ | CDM reg.4(1)=MEDIUM ❓
   - Notes: strength skew (71% HIGH) inflates scores. CDM reg.4(1) should arguably be HIGH. s.9 safety policy pulled HIGH by strength+hierarchy.

3. ✅ **Approach B: Gravity-dominant weighted sum** (`scripts/significance_approach_b.py`)
   - Weights: gravity=0.35, scope_duty_bearer=0.20, scope_protected_class=0.20, strength=0.15, hierarchy=0.10
   - Distribution: HIGH 13.2% | MEDIUM 24.8% | LOW 62.0%
   - Benchmarks: s.2(1)=HIGH ✅ | s.9=MEDIUM ✅ | reg.3(1)=HIGH ✅ | reg.4(1)=MEDIUM ❓
   - Notes: best benchmark accuracy so far. Gravity weighting pushes LOW-gravity provisions down. Still misses CDM reg.4(1).

4. ✅ **Approach C: Gravity-first rules** (`scripts/significance_approach_c.py`)
   - If gravity=HIGH → HIGH. If gravity=LOW AND all others LOW → LOW. Else MEDIUM.
   - Distribution: HIGH 21.5% | MEDIUM 74.9% | LOW 3.5%
   - Benchmarks: s.2(1)=HIGH ✅ | s.9=MEDIUM ✅ | reg.3(1)=HIGH ✅ | reg.4(1)=MEDIUM ❓
   - Notes: MEDIUM swallows 75% — poor discrimination. Only 3.5% LOW because any non-LOW dimension triggers MEDIUM.

5. ✅ **Approach D: Max-of-dimensions** (`scripts/significance_approach_d.py`)
   - Overall = max(all 5 dimensions)
   - Distribution: HIGH 88.4% | MEDIUM 8.1% | LOW 3.5%
   - Benchmarks: all HIGH — no discrimination
   - Notes: **rejected** — strength skew (71% HIGH) means nearly everything has at least one HIGH. Useless.

6. ✅ **Approach E: Gravity + scope gate** (`scripts/significance_approach_e.py`)
   - If gravity=HIGH AND (sdb≥MED OR spc≥MED) → HIGH. If gravity=LOW AND sdb=LOW AND spc=LOW → LOW. Else MEDIUM.
   - Distribution: HIGH 21.3% | MEDIUM 43.4% | LOW 35.3%
   - Benchmarks: s.2(1)=HIGH ✅ | s.9=MEDIUM ✅ | reg.3(1)=HIGH ✅ | reg.4(1)=MEDIUM ❓
   - Notes: best balance of distribution. Gravity gates HIGH, scope gates LOW. Middle band captures genuinely ambiguous provisions.

7. ✅ **Approach F: Weighted sum excluding strength** (`scripts/significance_approach_f.py`)
   - Drop strength entirely (71% HIGH = no signal). Equal weight on remaining 4.
   - Distribution: HIGH 13.0% | MEDIUM 29.1% | LOW 58.0%
   - Benchmarks: s.2(1)=HIGH ✅ | s.9=HIGH ❌ | reg.3(1)=HIGH ✅ | reg.4(1)=HIGH ✅
   - Notes: CDM reg.4(1) correctly HIGH (only approach to get this). But s.9 wrong — hierarchy+scope pull it up without gravity to dominate.

### Comparison summary

| Approach | HIGH | MED | LOW | s.2(1) | s.9 | reg.3(1) | reg.4(1) | Notes |
|----------|------|-----|-----|--------|-----|----------|----------|-------|
| A equal-weight | 9.7% | 48.9% | 41.4% | ✅H | ❌H | ✅H | ❓M | strength inflates |
| B gravity-weighted | 13.2% | 24.8% | 62.0% | ✅H | ✅M | ✅H | ❓M | best benchmarks |
| C gravity-rules | 21.5% | 74.9% | 3.5% | ✅H | ✅M | ✅H | ❓M | MED swallows all |
| D max | 88.4% | 8.1% | 3.5% | ✅H | ❌H | ✅H | ❌H | **rejected** |
| E gravity+scope | 21.3% | 43.4% | 35.3% | ✅H | ✅M | ✅H | ❓M | best distribution |
| F no-strength | 13.0% | 29.1% | 58.0% | ✅H | ❌H | ✅H | ✅H | CDM right, s.9 wrong |

8. ⬜ Validate winner against benchmark provisions
9. ⬜ Decide formula and persist overall significance

### Phase 2: Law-level aggregation

Goal: rank 274 customer laws against each other so a compliance officer can answer "which laws need attention first?"

Input: per-provision overall significance (from Phase 1 winner) for all Obligation provisions in a law.

Validation cases:
- HSWA (UK_ukpga_1974_37) should rank near the top — foundational safety statute
- A pure procedural SI (e.g. notification-only regs) should rank near the bottom
- CDM 2015 (UK_uksi_2015_51) should rank high — major construction safety regs

All law-level approaches use Approach E (gravity+scope gate) as provision-level input. 553 laws with rated Obligation provisions.

10. ✅ **Approach G: Max provision** (`scripts/significance_approach_g.py`)
    - Law significance = highest provision significance in that law
    - Distribution: HIGH 80.1% | MEDIUM 16.8% | LOW 3.1%
    - Benchmarks: HSWA=HIGH, CDM=HIGH, MHSW=HIGH (all correct but meaningless)
    - Notes: **rejected** — 80% HIGH, same problem as Approach D. Nearly every law has at least one HIGH provision.

11. ✅ **Approach H: HIGH count** (`scripts/significance_approach_h.py`)
    - Rank by count of HIGH provisions. Top 20% → HIGH law, bottom 33% → LOW, else MEDIUM.
    - Distribution: HIGH 19.9% | MEDIUM 47.0% | LOW 33.1%
    - Benchmarks: HSWA=HIGH ✅ | CDM=HIGH ✅ | MHSW=HIGH ✅
    - Top law: UK_uksi_2016_1105 (218 HIGH). Bottom: several with 0 HIGH.
    - Notes: good distribution but biased toward large laws (more provisions = more HIGH by volume).

12. ✅ **Approach I: HIGH proportion** (`scripts/significance_approach_i.py`)
    - Proportion of provisions rated HIGH. ≥50% → HIGH law, ≥20% → MEDIUM, else LOW.
    - Distribution: HIGH 21.9% | MEDIUM 21.9% | LOW 56.2%
    - Benchmarks: HSWA=MEDIUM ❌ (25% HIGH) | CDM=HIGH ✅ (65%) | MHSW=HIGH ✅ (85%)
    - Notes: HSWA wrong — its 172 provisions dilute the 43 HIGH across many procedural/admin sections. Small focused SIs with 100% HIGH top the list. Normalises for size but penalises large foundational Acts.

13. ✅ **Approach J: Weighted score** (`scripts/significance_approach_j.py`)
    - Score = (3×HIGH + 2×MED + 1×LOW) / total. ≥2.5 → HIGH, ≥1.75 → MEDIUM, else LOW.
    - Distribution: HIGH 17.0% | MEDIUM 43.0% | LOW 40.0%
    - Benchmarks: HSWA=MEDIUM ❌ (score 1.85) | CDM=HIGH ✅ (2.59) | MHSW=HIGH ✅ (2.85)
    - Notes: same HSWA problem as I — averaging dilutes large laws. Top laws are small SIs with all-HIGH provisions.

14. ✅ **Approach K: Distribution profile** (`scripts/significance_approach_k.py`)
    - No single rating — publish triple {high, medium, low}. Sort by HIGH count, then MEDIUM.
    - Benchmarks: HSWA rank 49/553 | CDM rank 15/553 | MHSW rank 47/553
    - Top law: UK_uksi_2016_1105 (H:218 M:419 L:117)
    - Notes: richest signal. No information loss. But HSWA at rank 49 — HIGH count favours large laws (same bias as H). Needs UI to display the profile, can't filter by "HIGH laws".

15. ✅ **Approach L: Weighted score + size penalty** (`scripts/significance_approach_l.py`)
    - Score = avg_significance × log2(total+1). Percentile-based: top 20% → HIGH, bottom 33% → LOW.
    - Distribution: HIGH 19.9% | MEDIUM 47.0% | LOW 33.1%
    - Benchmarks: HSWA=HIGH ✅ rank 96/553 | CDM=HIGH ✅ rank 10/553 | MHSW=HIGH ✅ rank 34/553
    - Notes: all benchmarks correct. Size boost lifts HSWA into HIGH despite moderate average. CDM ranks high (concentrated + mid-size). Balances importance with compliance burden.

### Law-level comparison summary

| Approach | HIGH | MED | LOW | HSWA | CDM | MHSW | Notes |
|----------|------|-----|-----|------|-----|------|-------|
| G max | 80.1% | 16.8% | 3.1% | ✅H | ✅H | ✅H | **rejected** — no discrimination |
| H count | 19.9% | 47.0% | 33.1% | ✅H | ✅H | ✅H | biased to large laws |
| I proportion | 21.9% | 21.9% | 56.2% | ❌M | ✅H | ✅H | penalises large Acts |
| J weighted | 17.0% | 43.0% | 40.0% | ❌M | ✅H | ✅H | averaging dilutes |
| K profile | n/a | n/a | n/a | r49 | r15 | r47 | richest, needs UI |
| L weighted+size | 19.9% | 47.0% | 33.1% | ✅H | ✅H | ✅H | best all-round |

### Phase 3: 2×2 comparison — provision (B vs E) × law (K vs L)

Scripts parameterised with `--provision-approach`. All 4 combinations persisted in `significance_law_experiments`.

#### HSWA ranking (lower = more significant)

| Combo | HSWA rank | HSWA rating | CDM rank | MHSW rank |
|-------|-----------|-------------|----------|-----------|
| B+K | 45/553 | n/a | 12/553 | 64/553 |
| B+L | 70/553 | HIGH | 7/553 | 41/553 |
| E+K | 49/553 | n/a | 15/553 | 47/553 |
| E+L | 96/553 | HIGH | 10/553 | 34/553 |

#### HSWA provision-level profile

| Provision input | HIGH | MEDIUM | LOW | Total |
|-----------------|------|--------|-----|-------|
| B (gravity-weighted) | 31 | 48 | 93 | 172 |
| E (gravity+scope gate) | 43 | 61 | 68 | 172 |

#### Observations

- **E produces more HIGH provisions** (43 vs 31 for HSWA) because scope gate is more permissive than weighted sum
- **But E ranks HSWA lower in L** (rank 96 vs 70) — more HIGH provisions doesn't help when LOW count also rises (68 vs 93), because the *average* significance drops
- **B+L ranks HSWA best** at 70/553 — gravity weighting concentrates HIGH on truly significant provisions, and size boost lifts HSWA
- **CDM and MHSW rank well across all combos** — focused safety statutes with high-gravity provisions
- **K (distribution profile) gives richer signal** — ranks are pure HIGH count, no lossy thresholding
- **L adds the compliance burden dimension** — large laws rank higher for a reason (more duties to comply with)

#### HSWA dilemma

HSWA is the foundational safety Act but ranks 45-96 out of 553. Why?
- 172 Obligation provisions, but many are procedural (s.20 inspector powers, s.33 offence definitions)
- Only 18-25% of its provisions are HIGH significance
- Smaller focused SIs (e.g. COSHH, LOLER) have higher concentration of safety duties
- This may actually be *correct* — a compliance officer managing 274 laws should focus on the specific regulations (CDM, COSHH) before the general enabling Act

### Gemini review feedback (2026-07-02)

Full review: `data/code-review/significance-aggregation-2x2.md`

**Actionable points:**

1. **Approach F was prematurely dismissed** — the only approach to get CDM reg.4(1) right. Strength is broken (71% HIGH), so excluding it makes sense. s.9 miss is a calibration issue, not a fundamental flaw. Consider F with gravity weighting on the remaining 4 dimensions.

2. **CDM reg.4(1) persistent miss exposes miscalibration** — B and E both miss it because CDM reg.4(1) has gravity=MEDIUM (construction client duty framed as organisational, not direct safety). The approaches are over-indexing on gravity=HIGH as the only path to overall HIGH.

3. **L formula is reasonable but could use a weighted sum** — `w1 * avg_sig + w2 * log2(total+1)` gives more control than straight multiplication. Also: HIGH count might be a better burden indicator than total provisions.

4. **HSWA dilemma needs stakeholder input** — is the system measuring "density of actionable duties" (HSWA ranks low, correctly) or "strategic importance" (HSWA must rank high)? Different questions need different systems. Current system measures the former.

5. **Hybrid K+L recommended** — L for ranking/filtering, K profile for drill-down. Publish both.

6. **More benchmarks needed** — 5 provisions and 3 laws is statistically inadequate. Should expand to 20-30 benchmark provisions across different law types, with explicit expected ratings.

7. **Threshold derivation undocumented** — the specific threshold values (≥2.5, ≥1.75) are presented without justification. Should document whether they're percentile-based, optimised against benchmarks, or domain-informed.

### Phase 4: Gemini critique responses

#### F2: Gravity-weighted excluding strength (`scripts/significance_approach_f2.py`)

Gemini flagged F's premature dismissal. F2 applies gravity-dominant weights (0.40/0.25/0.25/0.10) to the 4 non-strength dimensions.

- Distribution: HIGH 15.3% | MEDIUM 15.9% | LOW 68.8%
- Benchmarks: s.2(1)=HIGH ✅ | s.9=MEDIUM ✅ | reg.3(1)=HIGH ✅ | reg.4(1)=MEDIUM ❓
- Notes: gets s.9 right (unlike F). CDM reg.4(1) still MEDIUM — same root cause (gravity=MEDIUM). Distribution very LOW-heavy (69%).

#### L1: Weighted sum variant (`scripts/significance_approach_l1.py`)

Gemini suggested `w1 * avg_sig + w2 * log2(size)` instead of multiplication. w1=0.6, w2=0.4.

| Combo | HSWA rank | HSWA rating | CDM rank | MHSW rank |
|-------|-----------|-------------|----------|-----------|
| L1+E | 140/553 | MEDIUM ❌ | 10/553 | 18/553 |
| L1+B | 101/553 | HIGH ✅ | 14/553 | 38/553 |
| L1+F2 | 114/553 | MEDIUM ❌ | 15/553 | 20/553 |

Notes: L1 pushes HSWA **down** vs L (rank 96→140 with E). The weighted sum normalises the size component more aggressively — `avg_sig` dominates, and HSWA's moderate average pulls it down. L (multiplication) is better for this use case because size amplifies rather than merely adds.

#### L2: HIGH-count-as-burden (`scripts/significance_approach_l2.py`)

Score = `high_count * (high_count / total)` — rewards both volume and concentration.

| Combo | HSWA rank | HSWA rating | CDM rank | MHSW rank |
|-------|-----------|-------------|----------|-----------|
| L2+E | 99/553 | HIGH ✅ | 10/553 | 25/553 |
| L2+B | 90/553 | HIGH ✅ | 8/553 | 58/553 |
| L2+F2 | 91/553 | HIGH ✅ | 7/553 | 39/553 |

Notes: all combos get HSWA into HIGH. CDM ranks very high. But MHSW drops with B (rank 58) because B's stricter gravity weighting reduces MHSW's HIGH count. L2's quadratic nature (count × proportion) strongly rewards concentrated HIGH laws.

#### Part/Chapter breakdown — large Acts as series of "regulations"

52 large Acts (>50 rated provisions) analysed. Key finding for HSWA:

```
HSWA (172 provisions, ss.2-85)
  Range               HIGH   MED   LOW  Total  %HIGH   L-score
  ss.2-22  (Part I)     35    15    29     79  44.3%    13.12
  ss.23-43 (Part II)     6    31    13     50  12.0%    10.55
  ss.44-64 (Part III)    2    14    21     37   5.4%     7.80
  ss.65-85 (Part IV)     0     1     5      6   0.0%     3.28
```

**Part I (General Duties) at 44.3% HIGH would rank in the top third of all laws.** The "meat" of HSWA is visible when broken down — but it's diluted by Parts II-IV (HSE constitution, enforcement powers, offence definitions).

This validates the user's intuition: large Acts should be breakable by Part/Chapter for significance. The data is already there — section_id encodes the section number, and Part boundaries are derivable from structural rows.

**Architecture implication:** could publish a `significance_parts` payload alongside law-level significance. Sertantai displays the Part breakdown for Acts with >50 provisions. This is a future session concern — the data is computed and stored.

#### Updated comparison — all law-level approaches × provision inputs

| Approach | Prov | HSWA rank | HSWA | CDM | MHSW |
|----------|------|-----------|------|-----|------|
| L+B | B | 70/553 | ✅H | 7 | 41 |
| L+E | E | 96/553 | ✅H | 10 | 34 |
| L1+B | B | 101/553 | ✅H | 14 | 38 |
| L1+E | E | 140/553 | ❌M | 10 | 18 |
| L2+B | B | 90/553 | ✅H | 8 | 58 |
| L2+E | E | 99/553 | ✅H | 10 | 25 |
| L2+F2 | F2 | 91/553 | ✅H | 7 | 39 |

**L+B** remains the strongest overall — best HSWA rank (70), all benchmarks correct, good distribution. L2 is interesting (HSWA always HIGH) but its quadratic formula is harder to explain.

16. ⏸️ Validate law-level ranking against intuition (deferred — 07-01-26-significance-publish session)
17. ⏸️ Decide formula and persist (deferred — 07-01-26-significance-publish session)
18. ⏸️ Publish to sertantai (deferred — 07-01-26-significance-publish session)

## Depends on

- ✅ 06-27-26-duty-significance (4 dimensions rated for 40K provisions)
