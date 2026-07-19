---
session: JSP Phase 2 — Reference Extraction
status: closed
opened: 2026-07-18
closed: 2026-07-19
outcome: success

summary: >
  Built cross-reference extraction pipeline for JSP provisions — regex extraction,
  resolution against fractalaw corpus, and consolidated single-publisher architecture.
  Pilot on JSP-375-CH23: 63 references extracted, 52 resolved (82%), published to
  sertantai in unified DRRP+references payload.

decisions:
  - what: Consolidate all secondary enrichment into a single publisher and Zenoh key
    why: Separate publishers per enrichment type (DRRP, references, RACI, artefacts) would create 4-5 publishers and subscribers. One payload that grows per phase is simpler to operate and maintain.
    result: Single `publish-secondary` command JOINs `jsp_enrichment` + `jsp_references` at publish time. Removed `publish-secondary-refs` and `jsp/references` key expression.
  - what: Fuzzy match legislation citations by title keywords + year
    why: JSP text uses varied citation forms ("the Electricity at Work Regulations 1989", "Electricity at Work Regulations 1989"). Exact match fails. Keyword extraction + year filter + score threshold (0.6) resolves reliably.
    result: 9/10 legislation references resolved to correct law_names
  - what: Normalise JSP cross-references to source_id format
    why: JSP text uses "JSP 375 Volume 1, Chapter 23" but sertantai uses "JSP-375-CH23". Deterministic normalisation avoids LLM calls.
    result: All 43 JSP cross-references normalised

metrics:
  references_extracted: 63
  references_resolved: 52
  resolution_rate: 0.82
  by_type: { jsp: 43, legislation: 10, guidance: 8, standard: 2 }
  legislation_matched: 9
  tests_passing: 42

lessons:
  - title: DuckDB list-of-structs serialises as non-JSON VARCHAR
    detail: "DuckDB's `list({target_type: x, target_id: y})::VARCHAR` produces Python-dict-style output (single quotes, no JSON quotes on keys), not valid JSON. Sertantai subscriber needs to handle this format or fractalaw needs to use `to_json()` explicitly."
    tag: data
  - title: Legislation regex needs period in character class for 'etc.'
    detail: "Health and Safety at Work etc. Act 1974 — the 'etc.' has a period that breaks `[A-Za-z\\s,()&]` character class. Adding `.` to the class fixes it. JSPs reference Acts with non-standard title fragments frequently."
    tag: methodology
  - title: Consolidate enrichment publishers early, not after proliferation
    detail: "Phase 1 created publish-secondary for DRRP. Phase 2 initially created publish-secondary-refs for references. Recognised the pattern would create 4-5 publishers by Phase 5. Consolidated immediately — one payload, one subscriber, columns grow per phase."
    tag: architecture

artifacts:
  - crates/fractalaw-core/src/jsp/references.rs
  - crates/fractalaw-core/src/jsp/resolve.rs
  - crates/fractalaw-cli/src/commands/jsp.rs (extract-refs, trace commands)
  - crates/fractalaw-sync-cli/src/sync.rs (consolidated publish-secondary)
  - crates/fractalaw-sync/src/zenoh_sync.rs (cleaned up — removed jsp_references key)

depends_on:
  - phase-1-drrp-enrichment.md

enables:
  - Phase 3 obligation & RACI extraction (reference graph available for source traceability)
  - Phase 5 traceability gap analysis (resolved references identify which laws JSPs implement)
---

# Session: JSP Phase 2 — Reference Extraction (CLOSED)

## Problem

JSP text contains dense cross-references to legislation, other JSPs, and standards — embedded in prose, not structured. These need to be extracted, resolved against the fractalaw corpus and sertantai source registry, and published to enrich `source_links` with provision-level granularity.

## Todo

- ✅ Implement regex patterns for legislation references (Acts, Regulations, Orders)
- ✅ Implement regex patterns for JSP cross-references (JSP NNN Vol/Chapter)
- ✅ Implement regex patterns for standards (BS/EN/ISO) and HSE guidance (HSG/L/INDG)
- ✅ Create `jsp_references` DuckDB staging table
- ✅ Add `fractalaw jsp extract-refs {source_id}` CLI command
- ✅ Verify: JSP-375-CH23 → 63 references (43 JSP, 10 legislation, 8 guidance, 2 standards)
- ✅ Resolve legislation citations against fractalaw corpus — 9/10 matched (title+year fuzzy)
- ✅ Resolve JSP cross-references — normalised to source_id format (JSP-375-CH23)
- ✅ Resolution results stored in DuckDB jsp_references (52/63 = 82% resolved)
- ✅ Add `fractalaw jsp trace {target_id}` CLI command — queries by law_name, source_id, or citation text
- ✅ Consolidate into single publisher (see below)

## Consolidation: Single Publisher for All Secondary Enrichment

Decision: unify `publish-secondary` (DRRP) and `publish-secondary-refs` (references) into
a single publish command and Zenoh key. One publisher, one subscriber, one payload per source.
Future phases (RACI, mandated artefacts) add columns to the same payload — no new publishers.

### Consolidation todos

- ✅ Merge `publish-secondary` to JOIN `jsp_enrichment` + `jsp_references` at publish time
- ✅ Add `references_json` column to the Arrow payload (DuckDB list-of-structs as VARCHAR)
- ✅ Remove `publish-secondary-refs` command (consolidated into `publish-secondary`)
- ✅ Remove `jsp/references/{source_id}` Zenoh key expression (unused)
- ✅ Remove `publish_jsp_references` method from ZenohSync (unused)
- ✅ Update Zenoh spec (`ZENOH-SECONDARY-SOURCES.md` v1.1) — single enrichment schema
- ✅ Verify: `publish-secondary` sends 117 rows with DRRP + references in one payload
- ✅ Sertantai: `SecondaryTaxaSubscriber` updated to handle `references_json` column

### Design note

The enrichment payload grows per phase:
- Phase 1: `drrp_types`, `governed_actors`, `government_actors`, `obligation_strength`, `modal_verb`, `clause_refined`
- Phase 2: + `references_json`
- Phase 3: + `raci_json` (future)
- Phase 4: + `mandated_artefacts_json` (future)

All per-provision data. One row per section_id. One publish. One subscriber.
DuckDB JOIN at publish time: `jsp_enrichment LEFT JOIN (jsp_references grouped by section_id)`.

## Dependencies

- ✅ Phase 1 complete (DRRP enrichment operational, committed 4285f58)
