# Session: P-Dimension Dictionary Expansion (#23)

**Date**: 2026-03-07
**Issue**: [#23 — Fitness: expand p-dimension dictionaries to close vocabulary gaps](https://github.com/fractalaw/fractalaw/issues/23)
**Depends on**: #7 (fitness denormalization) — closed 2026-03-05 (9eda38e)
**Priority context**: See [priority-reviews.md](../../plans/priority-reviews.md) — #23 is priority #1 post-#7

## Problem

The fitness extraction pipeline has 95.5% polarity detection on genuine Application+Scope provisions, but **p-dimension tagging coverage is thin**. The 6 dictionaries in `fitness.rs` contain ~80 regex patterns total — a hand-curated starter set biased toward OH&S (Occupational Health & Safety) legislation. Only 21.7% of early-section provisions got at least one p-dimension tag.

The dictionaries are **universal** — the same ~80 patterns run against all 53 law families. An Agriculture provision mentioning "pesticide applicator" gets zero Person tags because the dictionary was built from OH&S examples. A Radiological provision mentioning "classified person" or "outside worker" gets nothing either.

### Current dictionary sizes

| Dimension | Entries | Examples |
|-----------|---------|----------|
| Person | 20 | employer, self-employed person, competent person, master of ship |
| Process | 22 | construction work, diving operations, manual handling |
| Place | 19 | Great Britain, offshore installation, mine, factory |
| Plant | 13 | dangerous substances, PPE, work equipment, asbestos |
| Property | 9 | at work, 5+ employees, Crown service |
| Sector | 11 | construction, mining, offshore oil & gas, nuclear |
| **Total** | **~94** | |

### Why this matters now

With #7 shipped, fitness data flows end-to-end (LAT → LRT → publish). Every dictionary improvement immediately improves the data quality in sertantai. But new law families will keep arriving as LAT is populated — **dictionary expansion must be a repeatable process, not a one-off**.

## Design Constraint: Reproducibility

New families of full-text law will populate LAT in future. The dictionary expansion workflow must be:

1. **Repeatable** — when a new family arrives (e.g., "TRANSPORT: Aviation"), we need a systematic process to identify vocabulary gaps and expand dictionaries
2. **Auditable** — each expansion should be traceable to corpus evidence (which provisions prompted which new terms)
3. **Testable** — expanded dictionaries should be validated with `taxa qa` before and after

This rules out a pure ad-hoc approach. We need tooling that can be re-run per family.

## Design Decision: Family-Specialist Dictionaries

Different families of law use fundamentally different vocabulary:

| Family | Example Person terms | Example Process terms | Example Plant/Substance terms |
|--------|---------------------|----------------------|------------------------------|
| OH&S: Occupational | employer, worker, competent person | construction work, manual handling | PPE, work equipment |
| AGRICULTURE | pesticide applicator, farm worker, keeper | spraying, spreading, storage | pesticide, fertiliser, seed |
| OH&S: Mines & Quarries | mine manager, shotfirer, banksman | blasting, winding, tipping | explosive, detonator |
| OIL & GAS / Offshore | installation manager, OIM, permit holder | well operations, drilling | hydrocarbon, BOP |
| RADIOLOGICAL | classified person, outside worker, RPA | work with ionising radiation | sealed source, radioactive substance |
| TRANSPORT: Maritime | master, pilot, harbour master | navigation, loading, bunkering | vessel, cargo |
| FOOD | food business operator, authorised officer | slaughter, processing, preparation | food, feed, additive |

**Approach**: Two-tier dictionary architecture:

1. **Core dictionary** (universal) — terms that appear across many families. The current ~94 entries, refined and validated. Always applied.
2. **Family dictionaries** (specialist) — terms specific to one or a few families. Applied when the law's `family` column matches. Loaded alongside core.

This means `extract_tags()` needs access to the law's family context, or the dictionaries need to be pre-selected per enrichment run. The enrichment loop already has `law_name` → can look up `family` from DuckDB.

### Architecture options

**Option A: Runtime family lookup** — `extract_tags()` takes an optional `family: Option<&str>` parameter. Core dict always runs; if family matches a specialist dict, those patterns run too. Simple, no structural change to fitness.rs beyond an extra parameter.

**Option B: Dict composition at enrichment time** — `enrich_single_law()` looks up the law's family, builds a composed dictionary (core + specialist), passes it to `extract()`. More flexible but heavier API change.

**Option C: Exhaustive run** — always run all dictionaries (core + all specialists). Simpler but may produce false positives (e.g., "pilot" in non-maritime context). With word-boundary regex this may be acceptable — need to test.

**Recommendation**: Start with **Option A** (simplest), evaluate false positive rate. If cross-family collisions are rare (likely, given word-boundary matching), consider upgrading to Option C for simplicity.

## Workflow: Corpus Gap Audit

The repeatable process for expanding dictionaries (routes 1+2 from #23):

### Step 1: Identify gap provisions

For a target family (or all families), find provisions that:
- Have purpose `APPLICATION_SCOPE`
- Got polarity (AppliesTo/DisappliesTo/ExtendsTo)
- Got **zero** p-dimension tags

These are provisions where scope is being defined but the dictionaries miss the vocabulary.

**CLI command needed**: `fractalaw taxa audit-fitness` (or extend `taxa qa --fitness-gaps`)

### Step 2: Extract candidate terms

From gap provisions, extract noun phrases (simple regex: determiner + adjective* + noun) and rank by frequency. High-frequency terms not in any dictionary are candidates.

### Step 3: Human review + dictionary update

Review candidates, assign to dimension, add to core or specialist dictionary. Run `taxa qa` before and after to validate improvement.

### Step 4: Re-enrich

```bash
fractalaw taxa enrich --family "TARGET_FAMILY" --force
fractalaw taxa qa --family "TARGET_FAMILY"
```

## Implementation Plan

### Phase 1: Corpus audit tooling ✓

- [x] Add `taxa audit-fitness` CLI command
  - [x] Query LanceDB for APPLICATION_SCOPE provisions
  - [x] Run `parse_v2()` live
  - [x] Report provisions with polarity but zero p-dimension tags
  - [x] Group by family
  - [x] Extract and rank noun phrases from gap provisions
- [x] Add public dictionary accessors in `fitness.rs`: `all_canonical_terms()`, `all_terms_by_dimension()`
- [x] 5-section report: Coverage by Family, Gap Provisions, Candidate Terms, No-Polarity Provisions, Dictionary Utilisation
- [x] Filters: `--laws`, `--family`, `--limit` (default 10 gap provisions per family)
- [x] All 494 tests pass

### Phase 2: Expand core dictionary — Skipped

Skipped in favour of proving the family-specialist architecture with OH&S data from Phase 1.

### Phase 3: Family-specialist dictionaries (OH&S) ✓

- [x] Design dictionary composition architecture → **Option A** (runtime family lookup)
- [x] Implement family-aware dictionary selection
  - [x] `family: Option<&str>` param threaded through `extract()` → `extract_tags()` → `try_split_compound()` → `parse_v2()`
  - [x] `specialist_dicts_for(family)` returns specialist dicts when family starts with `"OH&S"`
  - [x] `all_canonical_terms(family)` and `all_terms_by_dimension(family)` include specialist terms
- [x] Create OH&S specialist dictionaries (3 dicts, 19 entries)
  - [x] `OHS_PERSON_DICT`: young person, new or expectant mother, principal contractor/designer, domestic client, client
  - [x] `OHS_PROCESS_DICT`: lifting operations, confined spaces, provision and use, working at height, work near voltage
  - [x] `OHS_PLANT_DICT`: pressure equipment, lifting equipment, machinery, lifts, electrical equipment, scaffolding, safety signs, first-aid
- [x] Thread family context through enrichment pipeline
  - [x] `enrich_single_law()` looks up family from DuckDB, passes to `parse_v2()`
  - [x] `cmd_taxa_audit_fitness()` passes family to `parse_v2()` and dictionary accessors
  - [x] Other CLI commands (qa, eyeball, show) pass `None` — no specialist dicts needed
- [x] 6 new tests for OH&S specialist matching (with/without family, non-OH&S family)
- [x] All 342 core tests pass (up from 336)

### Phase 4: Automation for new families ✓

- [x] Document the workflow in a runbook → `docs/FITNESS-DICTIONARY-RUNBOOK.md`
- [x] Ensure `taxa audit-fitness --family NEW_FAMILY` works out of the box — verified with FOOD and TRANSPORT: Maritime Safety (both run cleanly; zero APPLICATION_SCOPE provisions because purpose classifier is OH&S-biased — separate issue)
- [x] Auto-generate candidate dictionary: existing `extract_candidate_terms()` in Section 3 already does n-gram frequency analysis — no further automation needed for now

## Audit Results: OH&S Family

### Phase 1 baseline (before specialist dicts)

| Metric | Value |
|--------|-------|
| APPLICATION_SCOPE provisions | 398 |
| Polarity matched | 324 (81.4%) |
| At least one p-dimension tag | 184 (46.2%) |
| Gap provisions (polarity, zero tags) | 118 |

### Phase 3 result (after OH&S specialist dicts)

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Tagged% | 46.2% | 52.3% | +6.1pp |
| Gap provisions | 118 | 94 | -24 (20% reduction) |

Top remaining candidate terms: `carriage`, `specified`, `crown`, `safety`, `health`, `door gate or hatch`, `conformity assessment procedure`

## Key Files

- `crates/fractalaw-core/src/taxa/fitness.rs` — 6 core dicts + 3 OH&S specialist dicts, `specialist_dicts_for()`, `extract_tags(text, family)`, `extract(text, family)`, `all_canonical_terms(family)`, `all_terms_by_dimension(family)`
- `crates/fractalaw-core/src/taxa/mod.rs` — `parse_v2(raw_text, family)`, `TaxaRecord.fitness_rules`
- `crates/fractalaw-cli/src/main.rs` — `enrich_single_law()` (family lookup + pass-through), `cmd_taxa_audit_fitness()`, `extract_candidate_terms()`
- `docs/FITNESS-DICTIONARY-RUNBOOK.md` — repeatable 7-step workflow for dictionary expansion
- `.claude/sessions/fitness/03-01-26-fitness-index-design.md` — corpus validation baseline

## Corpus Statistics (baseline)

From the fitness index design session:

| Metric | Value |
|--------|-------|
| Total APPLICATION_SCOPE provisions (early-section) | 3,130 |
| Genuine (non-heading, non-empty) | 645 |
| Polarity matched | 616 (95.5%) |
| At least one p-dimension tag | 678 (21.7% of 3,130) |
| Place mentions | 753 (24.1%) — most common |
| Person mentions | 375 (12.0%) |
| Plant mentions | 201 (6.4%) |
| Property mentions | 91 (2.9%) |
| Process mentions | 54 (1.7%) |
| Sector mentions | 41 (1.3%) — least common |

## Status: **Closed** — all phases complete (967adfd + 66cadcd)

### Follow-up issues

- **#24** — Extend purpose classifier beyond OH&S for APPLICATION_SCOPE detection (blocks fitness expansion to other families)
- **#25** — Zenoh WAN sync: enable cross-network publish/subscribe

### Observations

- Non-OH&S families (FOOD, TRANSPORT: Maritime Safety) have zero APPLICATION_SCOPE provisions — the purpose classifier is currently OH&S-biased. Expanding fitness dictionaries for other families will require extending the purpose classifier first (#24).
- Remaining OH&S gaps (94) are mostly provisions about conformity assessment procedures, carriage regulations, and Crown application — terms that may not map cleanly to the 6 p-dimensions.
- The two-tier architecture scales well — adding a new family specialist is ~30 lines of code + a branch in `specialist_dicts_for()`.
