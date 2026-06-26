# Session: Regex Actor Pattern Gaps (ACTIVE)

## Problem

Regex only finds 35% of gold benchmark actors (1,428/3,955). The remaining 2,527 actors have canonical labels in the dictionary but regex doesn't extract them from provision text.

## Top unmatched actors

```
Org: Undertaking (78), Org: Manufacturer (53), Gvt: Authority (53),
Org: Employer (32), Ind: Public (30), Org: Economic Operator (29),
Spc: Appellant (27), Org: Importer (27), Spc: Scheme Administrator (21),
Gvt: Minister (18), Gvt: Authority: Enforcement (18), Spc: Applicant (17),
Ind: Employee (14), Ind: Owner (12), Ind: Occupier (10)
```

## Two distinct sub-problems

1. **Trigger gaps** (~2,200) — actors exist in dictionary but their trigger words don't appear in the provision text. Either the text uses different phrasing, or the actor is implied not stated.
2. **Implied actors** (~300) — actor is logically present but not textually mentioned (e.g. "An employee is entitled..." implies an employer as counterparty).

## Findings

### Progress: 986 → 1,637 matched actors (66% improvement)
- ALIASES expansion: +442 (label normalisation)
- Gold cleanup: -135 (non-actors removed)
- Label remapping: +197 (SC:/Org: category mismatches)

### Three categories of remaining 2,200 unmatched

**1. Trigger words present but regex doesn't fire (~1,600)**
Top: Gvt: Authority (53), Org: Responsible Undertaking (46), Ind: Public (30), Org: Economic Operator (29). The word is in the text but `run_patterns()` doesn't match. Likely: regex pattern doesn't cover the specific phrasing, or a prior pattern consumes the match first.

**2. Implied actors (~200)**
Actor is logically present but not textually stated. Deterministic correlative rules found:

| When regex finds | Infer | Position | Coverage |
|-----------------|-------|----------|----------|
| Employee active (Obligation) | Employer | counterparty | 19/28 (68%) |
| Member State active (EU reg) | Responsible Undertaking | counterparty | 16/22 (73%) |
| Enforcement Authority active | Public | beneficiary | 7/11 (64%) |

These are Hohfeldian correlatives — codeable as deterministic rules without LLM.

**3. Genuine LLM-only extractions (~400)**
Complex inferences the LLM makes from context that no regex or rule can replicate.

## Progress

| Iteration | Matched | % of gold | Change |
|-----------|---------|-----------|--------|
| Start | 986 | 24% | Original aliases |
| +ALIASES | 1,428 | 36% | 80+ label mappings |
| +Gold cleanup | 1,637 | 43% | Non-actors removed, label remapping |
| +Dictionary fixes | **1,743** | **46%** | Authority/undertaking patterns widened |

## Completed work

1. ✅ ALIASES expansion (scripts/actor_aliases.py) — 80+ mappings
2. ✅ Gold cleanup — 135 non-actors removed
3. ✅ Label remapping — SC:/Org: category mismatches, specific authority → generic
4. ✅ Dictionary pattern fixes — Gvt: Authority (added relevant/hazardous prefixes), Org: Responsible Undertaking (added bare undertaking)
5. ✅ Correlative inference patterns identified → `cascade/06-26-26-correlative-inference.md` (PENDING)
6. ✅ Benchmark re-run after fixes: 1,743 matched, 87.3% DRRP, 57.4% classifier position

## Remaining ~2,000 unmatched

Mostly implied actors (word not in text) — correlative inference session's domain. Remaining trigger-present gaps are diminishing returns (~50 actors across many small patterns).

## Key files

- `docs/actor-dictionary.yaml` — canonical labels + trigger words
- `crates/fractalaw-core/src/taxa/` — regex actor extraction patterns
- `scripts/actor_aliases.py` — gold label normalisation
- `gold_benchmarks` table in Postgres — 3,955 gold actor entries

## Dependencies

- ✅ gold_benchmarks table populated with normalised labels
- ✅ provision_actors table populated for benchmark laws
