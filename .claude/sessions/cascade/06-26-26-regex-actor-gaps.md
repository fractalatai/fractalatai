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

## Investigation needed

1. For each top-missing actor, check if the dictionary trigger words appear in the provision text
2. If triggers appear but regex doesn't fire — regex pattern bug
3. If triggers don't appear — either add new triggers or accept as LLM-only extraction
4. For implied actors — consider inheritance rules (employee → employer correlation)

## Key files

- `docs/actor-dictionary.yaml` — canonical labels + trigger words
- `crates/fractalaw-core/src/taxa/` — regex actor extraction patterns
- `scripts/actor_aliases.py` — gold label normalisation
- `gold_benchmarks` table in Postgres — 3,955 gold actor entries

## Dependencies

- ✅ gold_benchmarks table populated with normalised labels
- ✅ provision_actors table populated for benchmark laws
