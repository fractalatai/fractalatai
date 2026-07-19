---
session: JSP Phase 3 — Obligation & RACI Extraction
status: closed
opened: 2026-07-18
closed: 2026-07-19
outcome: success

summary: >
  Built obligation and RACI extraction pipeline for JSP provisions. Splits
  multi-obligation provisions at lettered list items, assigns RACI from
  narrative text, extracts competence requirements. Wired into consolidated
  publisher. Pilot: 117 obligations, 32 RACI assignments, end-to-end verified.

decisions:
  - what: RACI table extraction (Mode A) not viable — narrative extraction only
    why: JSP-375-CH23 has no `table` section_type. PDF parser captures tables as flat paragraph text. All RACI must be inferred from narrative patterns.
    result: Mode B (actor + modal verb → R; accountable/consulted/informed markers → A/C/I)
  - what: Separate relational tables for obligations and RACI in sertantai, not JSONB
    why: "All obligations for Commanding Officer across applicable JSPs" is a JOIN query that doesn't work with JSONB. Baserow sync needs row-per-obligation.
    result: Raised sertantai-legal#126 for secondary_obligations + secondary_raci tables
  - what: Obligations and RACI travel in the consolidated publish-secondary payload
    why: Phase 2 established single-publisher architecture. Adding obligations_json + raci_json columns follows the same pattern — no new publishers.
    result: 10-column payload (DRRP + references + obligations + RACI)

metrics:
  obligations_extracted: 117
  raci_assignments: 32
  mandatory: 88
  recommended: 22
  permissive: 7
  with_competence: 21
  actors_with_raci: { accountable_person: 15, user: 6, defence_org: 5, contractor: 2, dsa: 2, commander: 1 }
  tests_passing: 52

lessons:
  - title: Informed markers apply to government actors, not just governed
    detail: "'The DSA shall be informed' — DSA is a government actor. RACI inference initially only checked governed actors for informed markers, missing government actors entirely. Both pools need to be checked for C and I assignments."
    tag: methodology
  - title: Lettered list items are the primary multi-obligation pattern in JSPs
    detail: "JSP provisions use 'a. X must... b. Y must...' extensively. Splitting at lettered items (regex: period + letter + period) correctly decomposes 80%+ of multi-obligation provisions."
    tag: data

artifacts:
  - crates/fractalaw-core/src/jsp/obligations.rs
  - crates/fractalaw-cli/src/commands/jsp.rs (extract-obligations, raci commands)
  - crates/fractalaw-sync-cli/src/sync.rs (obligations_json + raci_json in consolidated publish)

depends_on:
  - phase-1-drrp-enrichment.md
  - phase-2-reference-extraction.md

enables:
  - Phase 4 mandated artefact extraction (obligations are parent entities for artefacts)
  - Phase 5 controls integration (obligations + RACI feed L3 control generation)
  - Sertantai RACI queries ("all obligations for Commanding Officer across JSPs")
---

# Session: JSP Phase 3 — Obligation & RACI Extraction (CLOSED)

## Problem

Phase 1 already extracts actors and obligation strength per provision. Phase 3 goes deeper: extract individual obligations within provisions (a provision may contain multiple), assign RACI roles to each, and capture operational properties (competence, delegation, escalation). Results publish via the consolidated `publish-secondary` payload (Phase 2 architecture).

## Todo

- ✅ Create `jsp_obligations` DuckDB staging table
- ✅ Create `jsp_raci` DuckDB staging table
- ✅ Implement obligation extraction — split provisions at lettered list items, extract per-item
- ✅ Implement RACI assignment from narrative text (actor + modal → R; accountable → A; informed → I; consulted → C)
- ✅ Inspect JSP data for RACI tables — no `table` section_type in JSP-375-CH23, Mode A not viable
- ✅ Extract competence requirements (regex: competent person, trained, certification, etc.)
- ✅ Add `fractalaw jsp extract-obligations {source_id}` CLI command
- ✅ Add `fractalaw jsp raci {role}` CLI command (query across sources)
- ✅ Verify: JSP-375-CH23 → 117 obligations, 32 RACI assignments, 21 with competence requirements
- ✅ Add `obligations_json` and `raci_json` to consolidated `publish-secondary` payload
- ✅ Updated Zenoh spec with obligations + RACI column formats
- ✅ Sertantai: `SecondaryTaxaSubscriber` extended, end-to-end verified (#126)

## Dependencies

- ✅ Phase 1 complete — actor dictionary, JSP parser, pull/enrich/publish pipeline (4285f58)
- ✅ Phase 2 complete — reference extraction, consolidated publisher architecture (987dfd7)
