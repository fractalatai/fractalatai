---
session: JSP Phase 4 — Mandated Artefact Extraction
status: pending
opened: 2026-07-18
---

# Session: JSP Phase 4 — Mandated Artefact Extraction (PENDING)

## Problem

JSPs mandate specific things — safety cases, risk assessments, permits, hazard logs, emergency plans — each with operational properties (owner, approver, reviewer, frequency, required content, acceptance criterion). These mandated artefacts are the bridge to the L3 Controls and L4 Evidence pipelines. Need structured extraction into a generic artefact abstraction.

## Todo

- ⬜ Create `jsp_mandated_artefacts` DuckDB staging table
- ⬜ Implement regex patterns for artefact detection (safety case, risk assessment, permit, etc.)
- ⬜ LLM extraction of artefact properties (owner, approver, reviewer, frequency, content, criterion)
- ⬜ Add `fractalaw jsp extract-artefacts {source_id}` CLI command
- ⬜ Add `fractalaw jsp artefacts {jsp}` CLI command
- ⬜ Add `fractalaw jsp artefacts --type {type}` CLI command
- ⬜ Zenoh publish: `jsp/artefacts/{source_id}`
- ⬜ Sertantai: add `mandated_artefacts` JSONB column to `secondary_source_provisions`
- ⬜ Verify: extract artefacts from JSP-375-CH23, validate artefact type taxonomy

## Dependencies

- ⬜ Phase 3 complete (obligations needed as parent entities for artefacts)
