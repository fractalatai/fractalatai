---
session: JSP Phase 3 — Obligation & RACI Extraction
status: pending
opened: 2026-07-18
---

# Session: JSP Phase 3 — Obligation & RACI Extraction (PENDING)

## Problem

JSPs assign organisational roles to obligations using RACI-style responsibility assignment — not Hohfeldian DRRP. This needs structured extraction from both explicit RACI tables (Mode A) and narrative text (Mode B), with LLM enrichment for ambiguous cases (Mode C). Requires new sertantai Ash resources.

## Todo

- ⬜ Create `jsp_obligations` DuckDB staging table
- ⬜ Create `jsp_raci` DuckDB staging table
- ⬜ Implement Mode A: structured RACI table extraction
- ⬜ Implement Mode B: narrative obligation extraction (actor-anchored modal verb)
- ⬜ Extract operational properties: competence, delegation, escalation
- ⬜ Implement Mode C: LLM enrichment (passive voice, RACI disambiguation)
- ⬜ Source traceability: LLM mapping of JSP obligations → legislative provisions
- ⬜ Add `fractalaw jsp extract-obligations {source_id}` CLI command
- ⬜ Add `fractalaw jsp raci {role}` CLI command
- ⬜ Zenoh publish: `jsp/obligations/{source_id}`
- ⬜ Sertantai: create `secondary_obligations` and `secondary_raci` Ash resources
- ⬜ Sertantai: add enrichment columns (`obligation_strength`, `modal_verb`, `raci_summary`)
- ⬜ Verify: extract obligations + RACI from JSP-375-CH23, validate against manual reading

## Dependencies

- ⬜ Phase 1 complete (actor dictionary, policy context mode)
- ⬜ Phase 2 complete (cross-references needed for source traceability)
