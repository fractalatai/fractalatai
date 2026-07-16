---
session: Evidence Publish
status: closed
opened: 2026-07-15
closed: 2026-07-15
outcome: success

summary: >
  Built the Zenoh publish pipeline for L4 Evidence patterns, mirroring the controls
  publish pattern. Arrow schema, key expressions, CLI command, and sertantai spec
  all complete. Test publish of 3 patterns for Confined Spaces confirmed end-to-end
  Arrow IPC encoding and Zenoh delivery. Full corpus publish deferred until sertantai
  builds the receiver.

decisions:
  - what: Artefacts stay as JSON array string in Arrow payload
    why: >
      Artefacts are variable-length (1-3 per control) with nested fields. Flattening
      to individual Arrow rows would denormalise judgement/strategy fields. Keeping as
      a JSON string lets sertantai unpack on receipt, same pattern as linked_provisions
      in the controls payload.
    result: Single artefacts_json Utf8 column, sertantai unpacks to artefact_templates table

  - what: Publish both validated and flagged patterns
    why: >
      98.6% are validated. The 18 flagged have soft issues (Activity+High LR, missing
      judgement fields on auto-corrected controls). Better to publish all and let
      sertantai surface flags in the review UI than to silently drop 1.4%.
    result: WHERE status IN ('validated', 'flagged') in the publish SQL

metrics:
  test_publish: { law: "UK_uksi_1997_1713", patterns: 3, peers: 1 }
  arrow_columns: 20
  corpus_ready: { laws: 220, patterns: 1333 }

lessons:
  - title: Evidence publish is structurally identical to controls publish
    detail: >
      The publish method, CLI command, and key expression all follow the same pattern
      as controls. The only difference is the SQL extraction query (different JSON
      fields). The fractalaw-sync crate's publish pattern (query_arrow → encode_arrow_ipc
      → session.put) is fully reusable across payload types.
    tag: architecture

artifacts:
  - crates/fractalaw-sync/src/zenoh_sync.rs
  - crates/fractalaw-sync-cli/src/sync.rs
  - crates/fractalaw-sync-cli/src/main.rs
  - /var/home/jason/Desktop/sertantai-legal/docs/zenoh/ZENOH-EVIDENCE-SPEC.md
  - .claude/skills/evidence-creation/SKILL.md

depends_on:
  - 07-15-26-evidence-records.md
  - 07-13-26-phase4-publish.md

enables:
  - Sertantai evidence_patterns Postgres table + Zenoh subscriber
  - Baserow Evidence Vault template population
  - Customer evidence delivery (filter to Legal Register)
---

# Session: Evidence Publish (CLOSED)

## Problem

1,333 evidence patterns for 220 QQ laws are in the DuckDB `suggested_evidence` staging table (98.6% validated). The data needs publishing to sertantai via Zenoh, mirroring the controls publish pipeline built in `07-13-26-phase4-publish.md`. Without publish, the evidence patterns are trapped in fractalaw's local DuckDB.

## Todo

- ✅ Arrow schema + Zenoh key + publish method + CLI command — compiles clean
- ✅ Zenoh evidence spec for sertantai — `sertantai-legal/docs/zenoh/ZENOH-EVIDENCE-SPEC.md`
- ✅ Test publish on Confined Spaces 1997 — 3 patterns, peer connected, Arrow IPC sent
- ✅ Updated evidence-creation skill with publish CLI commands
- ⏸️ Publish full QQ corpus to sertantai (deferred — sertantai receiver not yet built, use `/evidence-creation` skill when ready)

## Dependencies

- ✅ Evidence corpus complete (1,333 patterns, 220 laws, 98.6% validated) — `07-15-26-evidence-records.md`
- ✅ Controls publish pattern exists to borrow — `publish-controls` in `crates/fractalaw-sync-cli/src/sync.rs`
- ⬜ Sertantai Postgres evidence table + Zenoh subscriber (external — needed for round-trip, not for publish)
