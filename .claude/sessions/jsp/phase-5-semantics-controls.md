---
session: JSP Phase 5 — Semantics, Gap Analysis & Controls
status: closed
opened: 2026-07-18
closed: 2026-07-20
outcome: success

summary: >
  Built term extraction, traceability gap analysis, and JSP-derived controls.
  Terms extracted via acronym parenthetical and quoted definition patterns.
  Controls are additive in the same suggested_controls table with source_id
  column for defence-sector filtering. Full 6-layer consolidated publish
  verified end-to-end with sertantai. 75 JSP tests passing.

decisions:
  - what: JSP controls in the same suggested_controls table, not a separate table
    why: Controls are controls — same schema, same Baserow sync, same customer queries. source_id column distinguishes origin. Defence customers see both; non-defence filter by source_id IS NULL.
    result: 1,556 legislation controls + 32 JSP controls in one table, additive
  - what: JSP controls NOT published via controls key expression
    why: They travel in the consolidated secondary enrichment payload. Sertantai creates control records from mandated artefact + obligation + RACI data. Keeps the controls subscriber clean — it only handles Gemini-generated legislation controls.
    result: Controls created sertantai-side from enrichment data, not fractalaw-side publish
  - what: Term extraction uses two-step approach (find acronym paren, scan backward)
    why: Single-pass regex with lazy quantifier fails in Rust regex crate for expansion capture. Two-step (find parenthetical, then scan backward for capital-letter-anchored expansion) is more robust.
    result: 29 terms from JSP-375-CH23 pilot, some noise in backward scan (Phase 6 SLM target)
  - what: SLM/LLM enhancement deferred to Phase 6
    why: Regex-based extraction across all phases is the foundation. LLM calls for property extraction, control title refinement, and term cleaning are a separate pipeline (Gemini batch). Cleaner to do all SLM work in one session.
    result: Phase 6 created as pending session for SLM enhancement

metrics:
  terms_extracted: 29
  laws_in_gap_analysis: 8
  jsp_controls_generated: 32
  legislation_controls_untouched: 1556
  total_controls: 1588
  enrichment_layers: 6
  tests_passing: 75
  payload_columns: 12

lessons:
  - title: Rust regex lazy quantifier behaves differently from Python for expansion capture
    detail: "Pattern `[A-Za-z\\s]+?` in Rust regex matches minimally (1 char) where Python would backtrack to find the longest match before the anchor. Two-step approach (find anchor, scan backward) is more reliable for acronym expansion extraction."
    tag: methodology
  - title: String contains check for 'Vol' false-positives on 'Voltage'
    detail: "'Voltage'.contains('Vol') is true. The Chapter/Volume filter for JSP cross-reference false positives needs word boundaries — 'Vol ' with trailing space, not bare 'Vol'."
    tag: data
  - title: Controls additive pattern is simpler than hierarchical IMPLEMENTS relationship
    detail: "Originally planned JSP controls to IMPLEMENT legislation controls via related_control_ids. In practice, just putting them in the same table with source_id is sufficient. The semantic linking (related_control_ids) can be added later via embedding similarity — don't need it for the controls to be useful."
    tag: architecture

artifacts:
  - crates/fractalaw-core/src/jsp/terms.rs
  - crates/fractalaw-core/src/jsp/artefacts.rs (Phase 4, committed in this session)
  - crates/fractalaw-cli/src/commands/jsp.rs (extract-terms, terms, controls, gaps commands)
  - crates/fractalaw-sync-cli/src/sync.rs (terms_json in consolidated publish)
  - docs/manual/JSP-SUMMARY.md

depends_on:
  - phase-4-mandated-artefacts.md

enables:
  - Phase 6 SLM enhancement (LLM property extraction, control title refinement, term cleaning)
  - Full corpus enrichment (run all phases across 158 JSP chapters)
  - Sertantai Baserow sync for JSP controls and obligations
---

# Session: JSP Phase 5 — Semantics, Gap Analysis & Controls (CLOSED)

## Problem

Three remaining capabilities: term extraction with cross-JSP conflict detection, traceability gap analysis (legislative obligations with no JSP implementation), and JSP-derived controls. JSP controls are additive — they go in the same `suggested_controls` table as legislation controls but are sector-specific (defence only), linked to their source JSP and optionally to the legislative controls they implement. SLM/LLM enhancement of all phases is deferred to Phase 6.

## Todo

- ✅ Create `jsp_terms` DuckDB staging table
- ✅ Implement term extraction (acronym parenthetical + quoted definition patterns)
- ✅ Add `fractalaw jsp extract-terms` and `fractalaw jsp terms --conflicts` CLI commands
- ✅ Add `terms_json` to consolidated `publish-secondary` payload
- ✅ Verify: JSP-375-CH23 → 29 terms (ELV, PUWER, ALARP, DSEAR, PPE, etc.)
- ✅ Traceability gap analysis: law-level summary of referenced legislation
- ✅ Add `fractalaw jsp gaps {source_id}` CLI command
- ✅ Add `source_id` + `related_control_ids` columns to `suggested_controls`
- ✅ JSP control generation from mandated artefacts — additive, same table (1556 leg + 32 JSP)
- ✅ Add `fractalaw jsp controls {source_id}` CLI command
- ✅ Add `terms_json` to consolidated `publish-secondary` payload
- ✅ Verify: JSP-375-CH23 → 29 terms, 8 referenced laws, 32 controls generated, 75 tests passing

## Architecture: JSP Controls Are Additive

JSP controls live in the **same `suggested_controls` table** as legislation controls. They do not replace, override, or modify legislation controls. The distinction:

| | Legislation control | JSP control |
|---|---|---|
| `law_name` | `UK_uksi_1989_635` | `UK_uksi_1989_635` (the law the JSP implements) |
| `source_id` | NULL | `JSP-375-CH23` |
| `linked_provisions` | legislative section_ids | JSP section_ids |
| `related_control_ids` | NULL | IDs of legislation controls this implements |
| Visibility | All customers | Defence sector customers only |

A JSP control links to legislation via:
1. `law_name` — the law the JSP implements (from Phase 2 resolved references)
2. `related_control_ids` — the specific legislation controls it refines (semantic match)

Defence customers see both. Non-defence customers filter by `source_id IS NULL`.

## Dependencies

- ✅ Phase 4 complete — mandated artefact extraction (6dd1574)
- ✅ L3 Controls pipeline operational (1,556 controls across 221 laws)
- ✅ L4 Evidence pipeline operational
