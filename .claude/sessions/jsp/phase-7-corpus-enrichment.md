---
session: JSP Phase 7 — Full Corpus Enrichment
status: closed
opened: 2026-07-20
closed: 2026-07-21
outcome: success

summary: >
  Ran the full JSP enrichment pipeline across the entire corpus — 11,351
  provisions across 157 chapters in 10 JSP families. All 6 enrichment
  layers applied and published to sertantai. Backed up to NAS.

decisions:
  - what: Run Phase 7 before Phase 6 (SLM enhancement)
    why: SLM fine-tuning needs hundreds of training examples across multiple chapters. A single pilot chapter (32 RACI, 117 obligations) is too thin. Corpus enrichment first provides the training data.
    result: Phase 6 suspended, Phase 7 run first. 1,719 RACI and 5,028 obligations now available for SLM training.
  - what: Batch script with list-secondary discovery rather than hardcoded source_ids
    why: 158 chapters across 10 JSPs — manual enumeration is fragile. list-secondary queries sertantai via Zenoh for the live source registry.
    result: scripts/jsp_corpus_enrich.py with --pull/--enrich/--publish/--stats. Discovered 158 chapters, pulled 157.

metrics:
  provisions_pulled: 11351
  sources_pulled: 157
  provisions_enriched: 6021
  obligations: 5028
  raci_assignments: 1719
  references: { total: 1969, resolved: 1742, rate: 0.88 }
  artefacts: 922
  terms: 1351
  term_conflicts: 1670
  jsp_controls: 922
  by_strength: { mandatory: 3226, recommended: 953, permissive: 297 }
  top_actors: { dsa: 312, commander_manager: 255, accountable_person: 222, defence_org: 200, co: 139, hoe: 132, user: 112, tlb: 66, sofs: 52, contractor: 45 }
  top_artefacts: { risk_assessment: 554, occurrence_report: 82, procedure: 67, safety_case: 59, audit: 35, training: 31, emergency: 25, inspection: 22, method_statement: 18, permit: 14 }

lessons:
  - title: 53% of provisions have classifiable obligations — the rest are headings, context, or descriptive
    detail: "6,021 enriched out of 11,351 provisions (53%). The non-enriched provisions are structural (headings, section titles, empty text nodes) or descriptive (context paragraphs without modal verbs). This is expected — legislation shows a similar ratio."
    tag: data
  - title: Risk assessments dominate the artefact taxonomy at corpus scale
    detail: "554 of 922 artefacts (60%) are risk assessments. JSPs mandate risk assessments for nearly every hazard category. The next most common is occurrence reports (82, 9%). This distribution should inform the L3 Controls pipeline — risk assessment controls will be the largest cluster for consolidation."
    tag: data
  - title: 1,670 term conflicts is noise, not signal at this stage
    detail: "Most 'conflicts' are capitalisation variants (Approved Codes Of Practice vs Approved Codes of Practice) or context-specific expansions (ADS = Approved Dosimetry Service vs DRPS Approved Dosimetry Services). Real semantic conflicts are a small subset. Phase 6 SLM should normalise before flagging."
    tag: methodology

artifacts:
  - scripts/jsp_corpus_enrich.py
  - crates/fractalaw-sync-cli/src/main.rs (list-secondary command)
  - crates/fractalaw-sync-cli/src/sync.rs (cmd_sync_list_secondary)
  - docs/manual/JSP-SUMMARY.md (updated with corpus numbers)

depends_on:
  - phase-1-drrp-enrichment.md
  - phase-2-reference-extraction.md
  - phase-3-obligation-raci.md
  - phase-4-mandated-artefacts.md
  - phase-5-semantics-controls.md

enables:
  - Phase 6 SLM enhancement (corpus-level training data now available)
  - Sertantai Baserow sync (full corpus published)
  - QQ contractor-applicable JSP queries (45 contractor obligations across 10 JSPs)
---

# Session: JSP Phase 7 — Full Corpus Enrichment (CLOSED)

## Problem

Phases 1-5 were developed and tested on a single pilot chapter (JSP-375-CH23, 174 provisions). The full JSP corpus is 13,854 provisions across 158 chapters in 10 JSP families. This session runs the complete pipeline across all chapters: pull, enrich, extract-refs, extract-obligations, extract-artefacts, extract-terms, controls, and publish.

## Todo

- ✅ Add `list-secondary` command to fractalaw-sync-cli (source discovery via Zenoh)
- ✅ Batch script `scripts/jsp_corpus_enrich.py` (--pull, --enrich, --publish, --stats)
- ✅ Pull all JSP chapters — 11,351 provisions across 157 sources
- ✅ Enrich all 157 chapters (DRRP + refs + obligations + RACI + artefacts + terms + controls)
- ✅ Publish all 157 sources to sertantai (157/157)
- ✅ Term conflict detection — 1,670 conflicts across JSPs
- ✅ Corpus stats: 6,021 enriched, 1,969 refs (88% resolved), 5,028 obligations, 1,719 RACI, 922 artefacts, 1,351 terms, 922 controls

## Dependencies

- ✅ Phase 1-5 complete — pipeline verified on pilot chapter
- ⬜ Phase 6 (SLM enhancement) — suspended, runs AFTER corpus enrichment provides training data
