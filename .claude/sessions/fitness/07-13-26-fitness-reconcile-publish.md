---
session: Fitness Reconcile and Publish
status: active
opened: 2026-07-13
---

# Session: Fitness Reconcile and Publish (ACTIVE)

## Problem

Phase 5b: three extraction tiers exist (regex 8,706, slm 6,604, ft 14,255) but the final `entities` column is empty on all but 10 rows. Need to reconcile tiers into the final column, re-propagate to child provisions with clean data, aggregate to DuckDB LRT, and publish to sertantai.

This is the last step before the fitness data is usable — without reconciliation, the rules engine has no `entities` to compile into expression trees.

## Work

1. ✅ Reconcile: COALESCE(ft > regex > slm) into final `entities` + `scope_dimensions`. 14,258/14,258 have entities (100%).
2. ✅ Re-populated scope_unit (lost in earlier re-insert): 974 law, 95 Part/Chapter/Schedule, 13,189 provision.
3. ✅ Re-propagated: 399,662 law-level + 8,816 Part-level = 408,478 propagated. 62.7% provision coverage.
4. ✅ Aggregated to DuckDB LRT: 654 laws with fitness_entities, fitness_scope_dimensions, mention counts.
5. ✅ Added fitness columns to Zenoh publish payload (fitness_entities, fitness_scope_dimensions, mention counts). Published 654 laws.
6. ✅ Validated: 4 benchmark laws all have entities, scope dimensions, polarity counts in DuckDB + sertantai.
7. ✅ Fixed temporal gap: reconciliation was COALESCE (one tier wins) not UNION (merge all tiers). 228 laws now have temporal scope. Re-published 209.
8. ✅ Updated ZENOH-SPEC.md to v2.2 with new fitness fields.
9. ✅ Created sertantai implementation guide: `docs/controls/FITNESS-APPLICABILITY.md` with Elixir evaluator code, customer profile schema, hierarchy expansion.
10. ✅ 42 orphan laws with non-standard naming (UK_YYYY_NNN instead of UK_uksi_YYYY_NNN) identified and cleaned. 37 renamed (1:1 match), 4 manually resolved (wsi/ssi/uksi), 1 deleted (duplicate). Zero orphans remaining.

## Dependencies

- ✅ Phase 5: three tiers populated — regex (8,706), slm (6,604), ft (14,255)
- ✅ scope_unit on all mentions
- ✅ fitness extract CLI independent of DRRP
- ✅ Per-tier columns prevent overwrite
