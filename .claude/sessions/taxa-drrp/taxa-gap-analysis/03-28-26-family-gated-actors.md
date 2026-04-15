# Session: 2026-03-28 — Family-Gated Specialist Actors (#31) ✓ CLOSED

## Context

**GitHub Issue**: [#31 — Family-gated specialist actors in taxa pipeline](https://github.com/fractalaw/fractalaw/issues/31)
**Parent session**: [03-28-26-ohs-offshore-safety.md](03-28-26-ohs-offshore-safety.md)
**Objective**: Add family-gated specialist actor definitions to `actors.rs`, mirroring the `fitness.rs` pattern. First specialist: OFFSHORE with "licensee".

## Design

Mirror `fitness.rs::specialist_dicts_for(family)`:

```
actors.rs (current)                    actors.rs (proposed)
─────────────────                      ─────────────────────
GOVERNED_DEFS  ──→ always              GOVERNED_DEFS  ──→ always (core)
                                       OFFSHORE_DEFS  ──→ only OH&S: Offshore
                                       (future: MARITIME_DEFS, NUCLEAR_DEFS, ...)

extract_actors(text)                   extract_actors(text)          ← unchanged
                                       extract_actors_for_family(text, family) ← new
```

Downstream (`duty_type.rs`, `duty_patterns_v2.rs`) unchanged — they receive `&[ActorMatch]` regardless of source.

## Key Files

| File | Change |
|------|--------|
| `crates/fractalaw-core/src/taxa/actors.rs` | Add specialist defs + `extract_actors_for_family()` + `specialist_governed_for()` |
| `crates/fractalaw-core/src/taxa/mod.rs` | Thread `family` into actor extraction in `parse_v2()` + full-pipeline tests |

## Plan

1. Add `OFFSHORE_GOVERNED_DEFS` to `actors.rs` with "licensee" pattern
2. Add `specialist_governed_for(family) -> &[(&str, Regex)]` function
3. Add `extract_actors_for_family(text, family)` that runs core + specialist patterns
4. Add tests: licensee extracted for offshore family, NOT extracted for other families
5. Wire `family` through `mod.rs::parse_v2()` to use `extract_actors_for_family`
6. Check existing `parse_v2()` signature — already receives `family: Option<&str>`
7. Run `cargo test -p fractalaw-core --lib taxa`
8. Verify with `taxa show` on an offshore law with licensee provisions

## Progress

- [x] Implement `OFFSHORE_GOVERNED_DEFS` + `specialist_governed_for()`
- [x] Implement `extract_actors_for_family()`
- [x] Add unit tests in actors.rs (5 tests)
- [x] Add full-pipeline tests in mod.rs (2 tests)
- [x] Wire family through parse pipeline (`parse_v2` line 125)
- [x] Run tests — **301 passed, 0 failed**
- [x] Wire family through `cmd_taxa_show` (+ misses/clauses sub-commands) in CLI
- [x] Verify on live data — confirmed working

## Implementation Details

### actors.rs

- `OFFSHORE_GOVERNED_DEFS`: `&[(&str, &str)]` with "Offshore: Licensee" pattern matching `licen[cs]ees?`
- `OFFSHORE_GOVERNED_COMPILED`: `LazyLock<Vec<(&str, Regex)>>` — compiled once on first use
- `specialist_governed_for(family)`: returns compiled specialist patterns when family starts with "OH&S: Offshore", empty slice otherwise
- `extract_actors_for_family(text, family)`: runs core `GOVERNED_COMPILED` + specialist patterns, deduplicates by label

### mod.rs

- `parse_v2()` line 125: changed `extract_actors(&cleaned)` to `extract_actors_for_family(&cleaned, family)`
- Note: `analyse_miss()` (line 326) still uses `extract_actors()` — it's a QA diagnostic function that doesn't receive family context. Can be updated separately if needed.

### main.rs (CLI)

- `cmd_taxa_show`: added DuckDB family lookup via `query_arrow`, passes `family.as_deref()` to `parse_v2`
- `cmd_taxa_show_misses`: receives `family: Option<&str>` from parent, passes to `parse_v2`
- `cmd_taxa_show_clauses`: receives `law_family: Option<&str>` from parent, passes to `parse_v2`
- Note: `cmd_taxa_eyeball` and `cmd_taxa_qa` still pass `None` — they can be updated separately

### Tests added (7 total)

**actors.rs** (5 unit tests):
- `licensee_extracted_for_offshore_family` — licensee found with offshore family
- `licensee_not_extracted_without_family` — licensee NOT found with `None` family
- `licensee_not_extracted_for_other_family` — licensee NOT found for "FIRE: General"
- `offshore_family_still_extracts_core_actors` — core actors (employer) still work with family
- `family_none_same_as_extract_actors` — `extract_actors_for_family(t, None)` == `extract_actors(t)`

**mod.rs** (2 full-pipeline tests):
- `licensee_duty_offshore_family_produces_drrp` — licensee + offshore → DRRP classification
- `licensee_duty_no_family_no_specialist_actor` — licensee without family → no specialist extraction

## Verification

### Live data — `taxa show`

**UK_nisr_2007_247** (reg.5 — previously a miss):
```
  DRRP:    Duty
  Pattern: Governed / Prescriptive (70%)
  Governed:   Offshore: Licensee, Operator
  POPIMAR: Organisation - Costs, Permit, Authorisation, License, Risk Control
  Purpose: Process+Rule+Constraint+Condition
  Clause:  The licensee shall- ensure that any operator appointed by him...
```

Licensee hits across offshore laws:
- UK_nisr_2007_247: 3 provisions
- UK_uksi_2005_3117: 2 provisions
- UK_uksi_2015_398: 3 provisions

No leakage to non-offshore laws (UK_ukpga_1974_37: 0 hits).

## Decisions

- **`specialist_governed_for` returns compiled `&[(&str, Regex)]`** not raw def strings — avoids recompilation per call, matches how `GOVERNED_COMPILED` works
- **`analyse_miss()` left unchanged** — it's a QA diagnostic that doesn't receive family context; updating it is a separate concern
- **Label format "Offshore: Licensee"** — mirrors "SC: C: Contractor" hierarchical labelling

## Notes

- `parse_v2()` already accepted `family: Option<&str>` for fitness extraction — no signature change needed
- The `licen[cs]ees?` regex handles both "licence" (UK spelling) and "license" (US spelling), singular and plural
