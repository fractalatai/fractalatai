---
session: Scoped LAT
status: closed
opened: 2026-09-29
closed: 2026-09-29
outcome: success

summary: >
  Built fractalatai #66: fractalaw honours legal's scoped LAT (#166). Manifest coverage/scope are parsed and stored per law.
  enabling_extent laws (extent evidence only) are synced but never triaged, enriched, rolled up or published. Partial
  (relevance) laws carry lat_coverage in provenance. The end-to-end test found legal's server stale (restarted). Then synced
  legal's Tier 1 writes: 26 hub laws, 136 renames from the log, 13,763 actors carried, 0 held.

decisions:
  - what: Filter enabling_extent at every entry point, not just watch
    why: Stage commands and publish are also run by hand and by pod scripts' law lists
    result: One store lookup (enabling_extent_laws) used by the CLI stage commands, fitness whole-corpus runs and both publish paths
  - what: Don't pull the 89 scoped Acts into the hub
    why: Fractalaw neither enriches them nor reads extent from them yet
    result: They stay in legal; if they arrive via watch or a pull, they're handled

metrics:
  tier1_sync: { laws: 26, legal_renames: 136, actors_carried: 13763, text_changed: 12, archived: 8, held: 0, gate_failures: 0 }

lessons:
  - title: Test against the live peer, not just its code
    detail: Legal's encoder had the fields, but the running server predated it. Only a live query showed the gap.
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/lat_sync.rs
  - crates/fractalaw-store/src/pg_lat_sync.rs
  - crates/fractalaw-sync-cli/src/lat_sync.rs
  - crates/fractalaw-cli/src/provenance.rs

depends_on:
  - 09-29-26-extent-reparse-sync.md
enables:
  - 09-29-26-reenrichment-backlog.md
---

# Session: Scoped LAT (CLOSED)

## Problem

Legal now serves scoped LAT (#166): the manifest flags `coverage` (full/partial) and `scope` purposes. `enabling_extent` is a few sections of a non-Making parent Act, kept only as extent evidence (the European Communities Act 1972 plus 89 Tier 1 Acts, 7,556 rows); it must never be triaged, enriched or turned into a Making verdict. `relevance` is part of a large Act, enriched in scope only. Fractalaw ignores both fields today, so `watch` would triage `enabling_extent` laws on events. fractalatai #66.

## Todo

- ✅ Manifest entries parse `coverage`/`scope` (object or JSON string); `lat_sync_state` stores coverage/scope_purposes/scope, refreshed for in-sync laws too (`b6508e5`)
- ✅ `watch`: enabling_extent laws are synced only (no triage, triage publish or enrichment_pending)
- ✅ Taxa stage commands (parse…backfill, slm, validate) and fitness extract/reconcile/application/compile drop enabling_extent laws (whole-corpus fitness runs exclude them); taxa + provisions publish skip them
- ✅ `relevance`/partial: provenance entries carry `lat_coverage` (scope-relative). Nothing in fractalaw flags missing Parts
- ✅ Tests: unit + store pass. End to end, after legal restarted its stale server: ECA 1972 in-sync coverage was recorded (partial, enabling_extent, section/2 + schedule/2); taxa and fitness reconcile skip it. Tier 1 synced: 26 hub laws applied, 136 renames from legal's log, 13,763 actors carried, 0 held, 0 gate failures; 4 reparse_needed → backlog. The 24 new Making Acts go to the backlog; the 89 scoped Acts are not pulled (no fractalaw use yet)
- ⏸️ Re-enrich the 6 Tier 0 big Acts once legal sets `relevance` scopes (in the re-enrichment backlog)

## Dependencies

- ✅ #62 sync + rename log (fixed `8116f36`)
- ✅ Legal #166 live (manifest fields; Tier 1 writes done)
- ⬜ Legal `relevance` scopes for the big Acts
