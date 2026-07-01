# Session: Significance Aggregation — Provision & Law Level (PENDING)

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

## Work

1. ⬜ Derive hierarchy from metadata (section_type + depth → HIGH/MEDIUM/LOW)
2. ⬜ Experiment with provision-level overall significance formulas (SQL queries, don't persist)
3. ⬜ Validate provision-level against benchmark laws (HSWA, MHSW, CDM)
4. ⬜ Experiment with law-level aggregation approaches
5. ⬜ Validate law-level ranking against intuition
6. ⬜ Decide formula and persist overall significance
7. ⬜ Publish to sertantai

## Depends on

- ✅ 06-27-26-duty-significance (4 dimensions rated for 40K provisions)
