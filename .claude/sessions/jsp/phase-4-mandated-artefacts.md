---
session: JSP Phase 4 — Mandated Artefact Extraction
status: closed
opened: 2026-07-18
closed: 2026-07-20
outcome: success

summary: >
  Built mandated artefact extraction from JSP obligations using a 12-type
  taxonomy. Regex detection linked to parent obligations, staged in DuckDB,
  wired into the consolidated 5-layer publish-secondary payload. Pilot:
  32 artefacts from JSP-375-CH23. 64 JSP tests passing.

decisions:
  - what: Defer LLM property extraction (owner, approver, frequency, criterion) to Phase 5
    why: Regex detection of artefact types is the foundation. Structured property extraction requires Gemini batch calls — different pipeline, larger scope. Phase 4 delivers the artefact inventory; Phase 5 enriches it.
    result: Phase 4 is regex extraction + linking. Phase 5 adds LLM properties + controls integration.
  - what: Separate relational table for artefacts in sertantai, not JSONB
    why: Same reasoning as obligations/RACI (Phase 3) — Baserow sync needs row-per-artefact, queries like "all risk assessments across applicable JSPs" need JOINs.
    result: Prompt prepared for sertantai Claude to create secondary_mandated_artefacts table
  - what: Artefact extraction runs on obligations, not raw provisions
    why: Obligations (Phase 3) are the parent entities. An artefact is mandated BY an obligation. Running on obligations gives the FK linkage and avoids double-counting from multi-obligation provisions.
    result: jsp_mandated_artefacts.obligation_id FK to jsp_obligations.obligation_id

metrics:
  artefacts_extracted: 32
  artefact_types_detected: 8
  by_type: { risk_assessment: 13, inspection_report: 8, occurrence_report: 3, permit: 2, training_record: 2, procedure: 2, method_statement: 1, safety_case: 1 }
  taxonomy_size: 12
  tests_passing: 64
  enrichment_layers: 5

lessons:
  - title: Risk assessments dominate JSP artefact requirements
    detail: "13 of 32 artefacts (41%) are risk assessments. JSP 375 Chapter 23 (Electrical Safety) mandates risk assessments for almost every hazard category. This will be the most common artefact type across the full corpus."
    tag: data
  - title: Artefact detection on obligations not provisions avoids double-counting
    detail: "A provision with 3 list-item obligations mentioning 'risk assessment' in each would produce 3 artefact rows (one per obligation) rather than 1 (from the provision). Running on obligations gives correct granularity and the obligation_id FK for free."
    tag: methodology

artifacts:
  - crates/fractalaw-core/src/jsp/artefacts.rs
  - crates/fractalaw-cli/src/commands/jsp.rs (extract-artefacts, artefacts commands)
  - crates/fractalaw-sync-cli/src/sync.rs (mandated_artefacts_json in consolidated publish)
  - docs/manual/JSP-SUMMARY.md

depends_on:
  - phase-3-obligation-raci.md

enables:
  - Phase 5 LLM property extraction (owner, approver, frequency, criterion per artefact)
  - Phase 5 controls integration (artefacts map directly to L3 Controls and L4 Evidence)
---

# Session: JSP Phase 4 — Mandated Artefact Extraction (CLOSED)

## Problem

JSPs mandate specific things — safety cases, risk assessments, permits, hazard logs, emergency plans. Each is a control specification with operational properties. This phase extracts artefact mentions from obligations (Phase 3), links them to parent obligations, and stages for publish via the consolidated payload. LLM extraction of detailed properties (owner, approver, frequency, acceptance criterion) is deferred to Phase 5.

## Todo

- ✅ Implement regex patterns for 12 artefact types (Risk Assessment, Safety Case, Permit, etc.)
- ✅ Create `jsp_mandated_artefacts` DuckDB staging table (FK to `jsp_obligations.obligation_id`)
- ✅ Add `fractalaw jsp extract-artefacts {source_id}` CLI command
- ✅ Add `fractalaw jsp artefacts --type {type}` CLI command (query across sources)
- ✅ Add `mandated_artefacts_json` to consolidated `publish-secondary` payload
- ✅ Verify: JSP-375-CH23 → 32 artefacts (13 Risk Assessment, 8 Inspection, 3 Occurrence Report, 2 Permit, 2 Training, 2 Procedure, 1 Method Statement, 1 Safety Case)
- ✅ Published 117 rows with all 5 enrichment layers end-to-end
- ⏸️ Sertantai: `secondary_mandated_artefacts` table + subscriber extension (deferred — sertantai-side work, prompt provided)

## Dependencies

- ✅ Phase 1 complete — actor dictionary, JSP parser, pull/enrich/publish (4285f58)
- ✅ Phase 2 complete — reference extraction, consolidated publisher (987dfd7)
- ✅ Phase 3 complete — obligation + RACI extraction, obligations are parent entities (888a978)
