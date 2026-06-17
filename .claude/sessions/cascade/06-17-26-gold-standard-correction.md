# Session: Gold Standard Correction + 3-Class Model Migration (PENDING)

## Context

**Prior sessions**:
- `.claude/sessions/cascade/06-11-26-drrp-qa-plan.md` (CLOSED)
- `.claude/sessions/cascade/06-17-26-actor-drift-surfacing.md` (CLOSED)

**Trigger**: Benchmark analysis revealed ~160 gold labels that are wrong or stale. Separately, the actor drift session identified that the 5-class DRRP model (Duty/Right/Responsibility/Power/none) creates unnecessary decomposition errors. The classifier already uses 3-class (Obligation/Liberty/none). Switching the entire pipeline to 3-class eliminates the decomposition problem and simplifies the gold standard correction.

## Two problems, one fix

### Problem 1: Stale gold labels

The benchmark Parquet files on NAS contain:
- **~95 provisions gold=Duty where pipeline says Responsibility** — not a pipeline error. The LLM was prompted without the governed/government distinction. These are actually Obligation.
- **~38 offence provisions gold=Duty** — "is guilty of an offence", no modal verb. These are not DRRP. Gold should be `none`.
- **~28 Rule provisions gold=Duty** — thing-subject ("A notice must be given"). Implied actor, context-dependent. Gold should be `none` or flagged as LLM-territory.

### Problem 2: 5-class creates decomposition errors

The Duty vs Responsibility distinction is derivable from actor type (governed/government). Storing it in `drrp_types` creates errors whenever:
- A government actor is mentioned but isn't the duty-bearer (95 cases)
- An actor is reclassified between governed and government (7 Spc actors this session)
- The LLM disagrees with our actor classification

**The fix**: switch the entire pipeline and gold standard to 3-class (Obligation/Liberty/none). The consumer derives Duty/Responsibility from actor labels at display time. No loss of signal.

## 3-Class Model

```
Obligation (shall/must/required to)  — someone bears a legal obligation
Liberty    (may/entitled to/power)   — someone has permission or discretion
none       — no legal relation (definitions, enactment, scope, offence)
```

The consumer decomposes at display time:
```
Obligation + governed active actor  → "Duty"           (customer sees this)
Obligation + government active actor → "Responsibility" (customer can filter)
Liberty    + governed active actor  → "Right"
Liberty    + government active actor → "Power"
```

## Scope

### Gold standard correction
1. Rewrite all benchmark Parquet files: Duty/Responsibility → Obligation, Right/Power → Liberty
2. Remove offence provisions (gold=none, not Obligation)
3. Remove Rule provisions (gold=none, context-dependent)
4. Write corrected files to NAS

### Pipeline migration (3-class)
1. `drrp_types` in LanceDB: `["Obligation"]` / `["Liberty"]` instead of `["Duty"]` / `["Right"]` etc.
2. `duty_type.rs`: `map_to_duty_type()` returns Obligation/Liberty instead of Duty/Right/Responsibility/Power
3. `DutyType` enum: simplify to `Obligation`, `Liberty`, `Rule` (keep Rule as structural)
4. Remove `decompose_drrp()` from `drrp_classifier.rs`
5. DuckDB columns: `duty_holder`/`rights_holder`/`responsibility_holder`/`power_holder` → `obligation_holder`/`liberty_holder` (sertantai breaking change)
6. LLM prompts: "classify as Obligation/Liberty/none"
7. Benchmark report: compare Obligation/Liberty/none

### Sertantai coordination
- DuckDB schema change requires sertantai code update
- The `obligation_holder` column replaces both `duty_holder` and `responsibility_holder`
- Sertantai derives Duty vs Responsibility from actor labels in the actors struct
- This is a coordinated release — pipeline + sertantai must update together

## Approach

### Phase 1: Gold standard (no pipeline change)
1. Script to rewrite benchmark Parquet: map 5-class → 3-class labels
2. Remove offence provisions (no modal = not DRRP)
3. Remove Rule provisions (thing-subject = context-dependent)
4. Write to NAS, re-run benchmark with current pipeline for new baseline

### Phase 2: Pipeline migration
1. Change `DutyType` enum to 3-class
2. Update `map_to_duty_type()` — Government and Governed both → Obligation (with obligation modal) or Liberty (with enabling modal)
3. Update LanceDB write paths
4. Update DuckDB schema + write paths
5. Remove `decompose_drrp()`
6. Update enrichment pipeline, QA commands, benchmark reports

### Phase 3: Sertantai
1. Update sertantai to read `obligation_holder`/`liberty_holder`
2. Derive Duty/Responsibility from actor struct `label` prefix
3. Coordinated deploy

## Expected outcome

After gold correction + 3-class migration:
- Benchmark accuracy: **~85%+** (the 95 decomposition errors disappear, offence/rule removed)
- The pipeline stores what it knows (Obligation/Liberty) not what it infers (Duty/Responsibility)
- No more governed/government classification debates at pipeline level
- Cleaner benchmark signal for measuring real improvements

## Key files

- `/mnt/nas/sertantai-data/data/fractalaw-benchmarks/tier2-*.parquet` — gold standard
- `crates/fractalaw-core/src/taxa/duty_type.rs` — DRRP type mapping
- `crates/fractalaw-core/src/taxa/mod.rs` — `DutyType` enum
- `crates/fractalaw-ai/src/drrp_classifier.rs` — already 3-class, remove `decompose_drrp()`
- `crates/fractalaw-cli/src/main.rs` — enrichment pipeline, DuckDB schema
- `scripts/benchmark_report.py` — benchmark comparison
- `docs/drrp_classifier_v7.json` — classifier weights (already 3-class)
