---
session: JSP Phase 2 — Reference Extraction
status: pending
opened: 2026-07-18
---

# Session: JSP Phase 2 — Reference Extraction (PENDING)

## Problem

JSP text contains dense cross-references to legislation, other JSPs, and standards — embedded in prose, not structured. These need to be extracted, resolved against the fractalaw corpus and sertantai source registry, and published to enrich `source_links` with provision-level granularity.

## Todo

- ⬜ Implement regex patterns for legislation references in JSP text
- ⬜ Implement regex patterns for JSP cross-references (JSP NNN)
- ⬜ Implement regex patterns for standard references (BS/EN/ISO)
- ⬜ Resolve extracted citations against fractalaw corpus
- ⬜ Resolve JSP cross-references against sertantai source registry
- ⬜ Create `jsp_references` DuckDB staging table
- ⬜ Add `fractalaw jsp extract-refs {source_id}` CLI command
- ⬜ Add `fractalaw jsp trace {section_id}` CLI command
- ⬜ Publish resolved references to sertantai `source_links`
- ⬜ Verify: extract refs from JSP-375-CH23, confirm resolution rate

## Dependencies

- ⬜ Phase 1 complete (DRRP enrichment operational)
