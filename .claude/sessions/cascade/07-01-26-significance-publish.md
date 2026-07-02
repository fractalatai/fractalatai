---
session: Significance Publish to Sertantai
status: closed
opened: 2026-07-01
closed: 2026-07-02
outcome: success

summary: >
  Built the full significance publish pipeline from Postgres/DuckDB through Zenoh to sertantai.
  Published 40,468 rated provisions and 274 QQ corpus laws with provision-level overall significance
  (Approach B), law-level rating + K-profile (Approach L), and Part-level breakdown for large Acts.

decisions:
  - what: Persist significance_overall in Postgres (not compute-on-publish)
    why: Gemini review confirmed — queryability and avoiding 40K recomputation on each publish. Backfill step is idempotent for recomputation.
    result: Column added, 40,468 provisions populated
  - what: Law-level significance in DuckDB legislation table (not separate table)
    why: DuckDB is the LRT store — law-level significance is LRT metadata. taxa backfill already bridges Postgres→DuckDB.
    result: 6 columns added (rating, score, high/med/low counts, total)
  - what: Part breakdown as JSON blob on enrichment payload (not sertantai-computed)
    why: Gemini rejected pushing aggregation logic to client — classic anti-pattern. Pre-compute and publish.
    result: significance_parts JSON blob for Acts with >50 rated provisions
  - what: No Postgres trigger for significance — idempotent backfill step
    why: Gemini agreed — triggers are hard to debug. Backfill handles both new laws and recomputation after formula/SLM changes.
    result: backfill_significance() + query_significance_parts() methods on PgStore

metrics:
  provisions_published: 94421
  laws_published: { enrichment: 274, provisions: 244 }
  provisions_rated: 40468
  laws_with_significance: { total: 553, qq_corpus: 221 }
  qq_distribution: { high: 62, medium: 113, low: 46, no_sig: 53 }
  zenoh_fields: { provision_level: 7, law_level: 7 }

lessons:
  - title: DuckDB lock conflicts when running parallel publish commands
    detail: >
      Running enrichment and provisions publish simultaneously caused a DuckDB lock error —
      both commands open DuckDB (provisions needs it for law name resolution). Run sequentially
      or chain with &&. Not a bug — DuckDB is single-writer by design.
    tag: infrastructure
  - title: Part-to-provision mapping via sort_key ordering works cleanly
    detail: >
      Assigning provisions to Parts by finding the most recent Part structural row
      in sort_key order is a simple SQL subquery and produces correct results across
      all tested Acts. No need for explicit Part boundary tables.
    tag: methodology
  - title: Arrow IPC publish contract is additive — new columns arrive automatically
    detail: >
      Adding significance columns to query_provision_taxa SELECT is sufficient — the Arrow
      IPC serialization picks up the new columns, and sertantai's decoder gets them without
      code changes. Only the ElectricSQL schema needs updating to store them.
    tag: architecture
  - title: Zenoh spec needs to be bidirectional
    detail: >
      The original ZENOH-SPEC.md only covered sertantai→fractalaw (queryables). Updated to
      v2.0 covering both directions with full field schemas and Arrow type documentation.
      The spec was stale — should be updated with every contract change.
    tag: architecture

artifacts:
  - crates/fractalaw-store/src/pg.rs
  - crates/fractalaw-store/src/provision_store.rs
  - crates/fractalaw-cli/src/main.rs
  - crates/fractalaw-cli/src/commands/sync.rs
  - scripts/pg_schema.sql
  - docs/manual/SIGNIFICANCE-TECHNICAL-SPEC.md
  - docs/manual/SIGNIFICANCE-METHODOLOGY.md
  - docs/manual/SIGNIFICANCE-SUMMARY.md
  - /var/home/jason/Desktop/sertantai-legal/docs/ZENOH-SPEC.md

depends_on:
  - 06-27-26-duty-significance.md
  - 07-01-26-significance-aggregation.md

enables:
  - Sertantai compliance register with significance filtering and law ranking
  - Customer onboarding with prioritised duty registers
---

# Session: Significance Publish to Sertantai (CLOSED)

## Problem

40,468 Obligation provisions have significance ratings (4 SLM dimensions + hierarchy) in Postgres, plus aggregation experiments at provision and law level. None of this is published to sertantai yet. Sertantai needs it for the compliance register — filtering provisions by significance and ranking laws by compliance burden.

## Aggregation session conclusions

From `07-01-26-significance-aggregation.md` — 8 provision-level and 9 law-level approaches tested:

### Provision-level overall significance

**Winner: Approach B (gravity-dominant weighted sum)**
- Weights: gravity=0.35, scope_duty_bearer=0.20, scope_protected_class=0.20, strength=0.15, hierarchy=0.10
- Score: HIGH=3, MEDIUM=2, LOW=1 per dimension, weighted sum, thresholds ≥2.5→HIGH, ≥1.75→MEDIUM, else LOW
- Distribution: HIGH 13.2% | MEDIUM 24.8% | LOW 62.0%
- Benchmarks: s.2(1)=HIGH ✅ | s.9=MEDIUM ✅ | reg.3(1)=HIGH ✅ | reg.4(1)=MEDIUM (accepted — gravity=MEDIUM from SLM)

### Law-level significance

**Winner: Hybrid K+L**
- **L rating** (for filtering): `avg_significance × log2(total_obligations + 1)`, percentile-based thresholds (top 20%→HIGH, bottom 33%→LOW)
- **K profile** (for drill-down): `{high_count, medium_count, low_count, total_obligations}`
- Using provision-level B as input: HSWA rank 70/553 (HIGH), CDM rank 7, MHSW rank 41

### Part/Chapter breakdown for large Acts

HSWA Part I (General Duties) is 44.3% HIGH but diluted to 25% overall by procedural Parts. 52 Acts with >50 provisions identified. Sub-law significance by Part gives compliance officers the signal they need for foundational statutes.

### Known limitations

- **Strength dimension** skewed 71% HIGH (SLM distillation bias) — included at reduced weight (0.15) but effectively noise. Future SLM retrain needed.
- **CDM reg.4(1)** misclassified as MEDIUM by all approaches — SLM rated gravity=MEDIUM. Accepted.
- **Benchmarks** limited to 5 provisions / 3 laws — expand in future.

## What to publish

### Per-provision (Zenoh `/taxa/provisions/{law_name}`)

For Obligation provisions only:
- `significance_scope_duty_bearer`: HIGH/MEDIUM/LOW
- `significance_scope_protected_class`: HIGH/MEDIUM/LOW
- `significance_gravity`: HIGH/MEDIUM/LOW
- `significance_strength`: HIGH/MEDIUM/LOW
- `significance_hierarchy`: HIGH/MEDIUM/LOW (metadata-derived)
- `significance_confidence`: 0-1 (SLM logprobs average)
- `significance_overall`: HIGH/MEDIUM/LOW (Approach B formula)

### Per-law (Zenoh `/taxa/enrichment/{law_name}`)

- `significance_rating`: HIGH/MEDIUM/LOW (Approach L)
- `significance_score`: float (the raw L score for custom sorting)
- `significance_high_count`: int
- `significance_medium_count`: int
- `significance_low_count`: int
- `significance_total_obligations`: int

### Per-Part (new channel? or nested in enrichment?)

For Acts with >50 Obligation provisions:
- Part identifier (section range or Part name)
- Per-Part: `{high_count, medium_count, low_count, total, pct_high}`

## Architecture decisions

Gemini review: `data/code-review/significance-publish-architecture.md`

### 1. ✅ Provision-level `significance_overall` → Postgres column

New column on `legislation_text` alongside the 5 dimension columns.

Gemini concurs but flags the **caching problem**: `significance_overall` is derived from dimensions. If dimensions change (SLM retrain) or formula changes (weight tuning), it must be recomputed. Solution: the backfill step is idempotent — rerunning it recomputes all provisions. No Postgres trigger.

### 2. ✅ Law-level significance → DuckDB `legislation` table

New columns on `legislation` (the LRT store).

Gemini flags the **DuckDB/Postgres consistency risk**: law-level is derived from Postgres provision data. The existing `taxa backfill` command already bridges this gap — it aggregates `provision_actors` (Postgres) into `legislation_text` (Postgres) and then into DuckDB. Significance follows the same flow. DuckDB is effectively a materialised view refreshed by backfill.

### 3. ✅ Part/Chapter breakdown → JSON blob on enrichment payload (Option A)

Gemini **rejected Option C** (sertantai computes) — classic anti-pattern pushing business logic to the client. Pre-compute and publish.

Option A (JSON blob) chosen over Option B (separate channel) for simplicity. The Part breakdown is per-law metadata, not a separate data domain. Schema:
```json
"significance_parts": [
  {"part": "ss.2-22", "high": 35, "medium": 15, "low": 29, "total": 79},
  ...
]
```
Only published for Acts with >50 Obligation provisions. Sertantai can display or ignore.

### 4. ✅ Backfill pathway → dedicated idempotent step in `taxa backfill`

No Postgres trigger (Gemini agreed — hard to debug). The backfill step:
1. Computes `significance_overall` from 5 dimension columns (pure SQL UPDATE)
2. Aggregates provision-level → law-level L score + K profile
3. Computes Part breakdown for large Acts
4. Writes law-level to DuckDB

Idempotent — safe to re-run after SLM retrain or formula change. Handles both new laws and recomputation of existing ones.

## Work

### Phase 1: Production schema + persist

5. ✅ Add `significance_overall` column to `legislation_text` in Postgres
6. ✅ Compute and persist `significance_overall` for all 40,468 provisions (Approach B formula) — HIGH 5,359 | MED 10,023 | LOW 25,086
7. ✅ Add law-level significance columns to DuckDB `legislation` table (rating, score, high/med/low counts, total)
8. ✅ Compute and persist law-level significance for all 553 laws (L formula + K profile) — HSWA=HIGH rank 70, CDM rank 7, MHSW rank 41
9. ✅ Wire into `taxa backfill` — `backfill_significance()` runs after `backfill_from_actors()`, computes significance_overall from dimensions (pure SQL). New trait methods on ProvisionStore.

### Phase 2: Publish contract

10. ✅ Extend provisions publish payload with 7 significance fields (query_provision_taxa SELECT in pg.rs)
11. ✅ Extend enrichment publish payload with 6 law-level significance fields (DuckDB SELECT in sync.rs)
12. ⏸️ Update sertantai schema to receive and store all significance fields (deferred — sertantai-side, ingestion proven)
13. ✅ Test publish with HSWA end-to-end — enrichment (1 law, 6 sig fields) + provisions (855 provisions, 7 sig fields) both published to Zenoh successfully

### Phase 3: Corpus publish + verify

14. ✅ Publish full QQ corpus — 274/274 enrichment + 94,421 provisions across 244 laws (30 had no enriched provisions)
15. ⏸️ Verify sertantai displays provision-level significance in compliance register (deferred — sertantai UI)
16. ⏸️ Verify sertantai can filter/sort by law-level significance (deferred — sertantai UI)

### Phase 4: Part/Chapter breakdown

19. ✅ Add `query_significance_parts` method to PgStore — SQL assigns provisions to Parts via sort_key ordering
20. ✅ Add `significance_parts` column to DuckDB legislation table (VARCHAR — JSON blob)
21. ✅ Wire into backfill — compute Part breakdown for Acts with >50 Obligation provisions
22. ✅ Add `significance_parts` to enrichment publish payload
23. ✅ Update ZENOH-SPEC.md with Part breakdown schema
24. ✅ Test with HSWA end-to-end — Part breakdown: pt.I (31H/35M/69L), pt.III (0H/3M/10L), pt.IV (0H/10M/14L)

### Future (not this session)
- Strength dimension SLM retrain (fix 71% HIGH skew)
- Expand benchmark provisions from 5 to 20-30
- Customer-specific dimension weighting

## Depends on

- ✅ 06-27-26-duty-significance (4 dimensions + hierarchy rated for 40K provisions)
- ✅ 07-01-26-significance-aggregation (B+L chosen as production formulas)
- ⬜ Sertantai schema update for significance fields
