---
session: JSP Phase 5 — Semantic Enrichment & Controls Integration
status: pending
opened: 2026-07-18
---

# Session: JSP Phase 5 — Semantic Enrichment & Controls Integration (PENDING)

## Problem

Final phase — term extraction with cross-JSP conflict detection, provision embeddings for semantic search, traceability gap analysis, and the big integration: Mode 1b control generation from JSP mandated artefacts feeding into the L3 Controls and L4 Evidence pipelines.

## Todo

- ⬜ Create `jsp_terms` DuckDB staging table
- ⬜ Implement term extraction from glossary sections
- ⬜ Implement inline definition extraction
- ⬜ Add `fractalaw jsp terms --conflicts` CLI command
- ⬜ Embed JSP provisions (pgvector, all-MiniLM-L6-v2)
- ⬜ Traceability gap analysis: legislative obligations with no JSP implementation
- ⬜ Add `fractalaw jsp gaps {jsp}` CLI command
- ⬜ Mode 1b control generation from mandated artefacts
- ⬜ Add `source_normativity` field to `suggested_controls`
- ⬜ Consolidate JSP-derived controls with legislative controls (HDBSCAN)
- ⬜ L4 Evidence generation from JSP-enriched controls
- ⬜ Add `fractalaw jsp controls {source_id}` CLI command
- ⬜ Zenoh publish: `jsp/terms/{source_id}`
- ⬜ Sertantai: create `secondary_terms` Ash resource
- ⬜ Verify: full pipeline on pilot JSP family (JSP-375), end-to-end

## Dependencies

- ⬜ Phase 4 complete (mandated artefacts needed for Mode 1b controls)
- ⬜ L3 Controls pipeline operational (COMPLIANCE-CONTROLS.md)
- ⬜ L4 Evidence pipeline operational (COMPLIANCE-EVIDENCE.md)
