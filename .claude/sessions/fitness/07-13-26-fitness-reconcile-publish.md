---
session: Fitness Reconcile and Publish
status: closed
opened: 2026-07-13
closed: 2026-07-13
outcome: success

summary: >
  Reconciled three extraction tiers into final entities, re-propagated, compiled
  expression trees, fixed two compiler bugs (Or not And, filter conflicting
  DisappliesTo), cleaned 42 orphan law names, published 695 laws with compiled
  applicability trees to sertantai. Fitness pipeline complete end-to-end.

decisions:
  - what: Reconciliation priority ft > regex > slm via COALESCE
    why: Fine-tuned model has highest precision (93.3%), regex dictionaries are curated, base SLM is noisy (~60% precision).
    result: 14,258/14,258 mentions have reconciled entities (100%).

  - what: Scope dimensions union across tiers, not COALESCE
    why: COALESCE picks one tier's dimensions — misses temporal from regex when ft wins. Union merges all tiers' dimensions.
    result: 228 laws gained temporal scope dimension that was previously hidden.

  - what: AppliesTo mentions are OR'd, DisappliesTo are Not(Or(...)) with conflict filtering
    why: Each provision's applicability is a disjunctive path (any one = law applies). Provision-level exceptions that conflict with AppliesTo codes are section-specific overrides, not law-wide exclusions.
    result: Trees evaluate correctly. NERC Act: 10→6 disapplies after filtering conflicts.

  - what: Orphan law names renamed to canonical (not deleted)
    why: Provisions only existed under orphan names (UK_YYYY_NNN). Canonical names (UK_uksi_YYYY_NNN) had DuckDB LRT but no provisions. Rename preserves the data.
    result: 42 orphans resolved — 37 auto-matched, 4 manually identified, 1 deleted (duplicate).

metrics:
  reconciliation:
    total_mentions: 14258
    entities_coverage: "100%"
    tiers: { regex: 8706, slm: 6604, ft: 14255 }
  propagation:
    law_level: 399662
    part_level: 8816
    provision_coverage: "62.7%"
  publish:
    laws_compiled: 696
    laws_published: 695
    spec_version: "2.3"
  data_cleanup:
    orphan_laws_renamed: 41
    orphan_laws_deleted: 1
    remaining_orphans: 0

lessons:
  - title: Scope dimensions must be unioned across tiers, not COALESCE'd
    detail: COALESCE picks the first non-null tier. If ft has {material, personal} but regex has {temporal}, COALESCE drops temporal. Union preserves all dimensions from all tiers.
    tag: architecture

  - title: AppliesTo mentions are disjunctive, not conjunctive
    detail: Each provision's applicability is an independent path into the law. "Applies to employers" OR "applies to construction" — a customer matching either one should see the law as relevant. The initial And compiled trees that could never evaluate to true.
    tag: architecture

  - title: Provision-level DisappliesTo must not become law-level exclusions
    detail: "Section 42 does not apply to construction on Crown land" is a section-specific exception. Promoting it to a law-level Not(construction) prevents the law from ever matching a construction customer, even though other sections DO apply to construction. Heuristic filter: drop DisappliesTo codes that also appear in AppliesTo.
    tag: data

  - title: Non-standard law names from batch imports cause orphan records
    detail: 42 laws imported with UK_YYYY_NNN naming (missing type prefix) created provisions that couldn't be published or enriched. The canonical UK_uksi_YYYY_NNN existed in DuckDB but had no provisions. Rename is the fix, not delete.
    tag: data

  - title: Finish the build before publishing — every payload change forces sertantai migration + full republish
    detail: Published benchmarks before the compiler was built, then republished after adding compiled_applicability, then republished again after fixing Or/And, then again after fixing DisappliesTo. Each iteration required sertantai code changes.
    tag: methodology

artifacts:
  - crates/fractalaw-cli/src/commands/fitness.rs
  - crates/fractalaw-sync-cli/src/sync.rs
  - /var/home/jason/Desktop/sertantai-legal/docs/zenoh/ZENOH-SPEC.md
  - /var/home/jason/Desktop/sertantai-legal/docs/controls/FITNESS-APPLICABILITY.md

depends_on:
  - 07-11-26-fitness-slm-finetune.md
  - 07-13-26-fitness-expression-compiler.md
  - 07-13-26-query-lrt-cli.md

enables:
  - Sertantai customer applicability matching (695 laws with compiled trees)
  - Customer onboarding: "does this law apply to me?" answered from expression tree evaluation
  - Fitness data live in sertantai for all enriched laws
---

# Session: Fitness Reconcile and Publish (CLOSED)

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
11. ✅ Fixed compiler: AppliesTo → Or (any match = law applies), DisappliesTo → Not(Or(...)) (single grouped exclusion)
12. ✅ Fixed compiler: provision-level DisappliesTo codes that conflict with AppliesTo codes are dropped (section-specific exceptions, not law-wide exclusions). NERC Act: 10→6 disapplies after filtering.
13. ✅ Recompiled 696 laws, republished 4 benchmarks
14. ✅ Sertantai confirmed: trees evaluate correctly. Bulk published 695 laws.

## Dependencies

- ✅ Phase 5: three tiers populated — regex (8,706), slm (6,604), ft (14,255)
- ✅ scope_unit on all mentions
- ✅ fitness extract CLI independent of DRRP
- ✅ Per-tier columns prevent overwrite
