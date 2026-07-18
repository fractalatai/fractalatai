---
session: JSP Phase 1 — DRRP Enrichment
status: closed
opened: 2026-07-18
closed: 2026-07-18
outcome: success

summary: >
  Built JSP DRRP enrichment pipeline end-to-end. Separate JSP parser module
  (actors, modal patterns) in fractalaw-core. Pull/publish commands in
  fractalaw-sync-cli following existing architecture. Verified round-trip:
  pull 174 provisions → enrich 117 → publish 117 → sertantai updated 117.

decisions:
  - what: JSP parser is a separate module from taxa, not a mode flag on parse_v2
    why: Different modal conventions (will/is to as mandatory), different actor dictionary, different output type. Polluting the legislation pipeline risks regressions.
    result: fractalaw-core/src/jsp/ with own actors.rs, patterns.rs, mod.rs
  - what: Zenoh code stays in fractalaw-sync-cli, not fractalaw-cli
    why: fractalaw-cli is local-only (DuckDB/LanceDB/PG). All Zenoh operations live in fractalaw-sync-cli. Violated this initially, backed out.
    result: pull-secondary and publish-secondary in fractalaw-sync-cli
  - what: --all-chapters not --family for JSP parent/child grouping
    why: "family" means regulatory domain classification for legislation. JSP parent/child is document grouping — different concept, different name.
    result: fractalaw jsp enrich JSP-375 --all-chapters
  - what: Arrow batch ingestion via Parquet temp file (CTAS), not row-by-row SQL insert
    why: Row-by-row INSERT with string escaping fails on provision text containing quotes. DuckDB CTAS from Parquet handles all data types safely.
    result: pull-secondary uses CREATE TABLE AS SELECT from read_parquet()

metrics:
  provisions_pulled: 174
  provisions_enriched: 117
  provisions_published: 117
  mandatory: 77
  recommended: 22
  permissive: 6
  actors_found: 7
  tests_passing: 20
  sertantai_issue: "#125 (SecondaryTaxaSubscriber section_id lookup)"

lessons:
  - title: Never put Zenoh code in fractalaw-cli
    detail: "fractalaw-cli is local-only. All Zenoh operations (pull, publish, watch, subscribe) belong in fractalaw-sync-cli. The architecture boundary is strict — fractalaw-cli reads/writes local stores, fractalaw-sync-cli handles all network operations."
    tag: architecture
  - title: DuckDB row-by-row SQL insert breaks on real text data
    detail: "String escaping for SQL INSERT is fragile with provision text containing apostrophes, quotes, special characters. Use Arrow→Parquet→CTAS instead — DuckDB handles all types safely via read_parquet()."
    tag: data
  - title: Ash.get looks up by primary key, not by named field
    detail: "SecondaryTaxaSubscriber used Ash.get(Resource, section_id) but section_id is not the PK (id UUID is). Must use Ash.Query.filter(section_id == ^val) |> Ash.read_one() for lookup by unique text field."
    tag: sertantai

artifacts:
  - crates/fractalaw-core/data/jsp-actor-dictionary.yaml
  - crates/fractalaw-core/src/jsp/mod.rs
  - crates/fractalaw-core/src/jsp/actors.rs
  - crates/fractalaw-core/src/jsp/patterns.rs
  - crates/fractalaw-cli/src/commands/jsp.rs
  - crates/fractalaw-sync/src/zenoh_sync.rs (secondary key expressions + methods)
  - crates/fractalaw-sync-cli/src/sync.rs (pull-secondary, publish-secondary)
  - crates/fractalaw-sync-cli/src/main.rs (PullSecondary, PublishSecondary commands)

depends_on:
  - sertantai-legal second-tier-duties Phase 3 (Zenoh queryables)

enables:
  - Phase 2 reference extraction (provisions now in DuckDB for cross-reference analysis)
  - Phase 3 obligation & RACI extraction (JSP parser infrastructure in place)
  - Full corpus enrichment (repeat pull/enrich/publish for all 158 JSP chapters)
---

# Session: JSP Phase 1 — DRRP Enrichment (CLOSED)

## Problem

13,854 JSP provisions are parsed into sertantai's Postgres with null enrichment columns. Fractalaw needs to enrich these with JSP-specific DRRP classification and publish results back to sertantai.

## Architecture: Follow the Existing Separation

| | `fractalaw` (fractalaw-cli) | `fractalaw-sync` (fractalaw-sync-cli) |
|---|---|---|
| **Purpose** | Local parsing, enrichment, classification | All Zenoh pub/sub, pull, publish |
| **Reads** | DuckDB, LanceDB, Postgres (local) | DuckDB, Postgres (for publish payload) |
| **Writes** | DuckDB, LanceDB (local staging) | Zenoh only (to sertantai) |
| **Zenoh** | NEVER | Always |

JSP pipeline:
```
fractalaw-sync pull-secondary   →  DuckDB jsp_provisions
fractalaw jsp enrich            →  DuckDB jsp_enrichment
fractalaw-sync publish-secondary →  Zenoh taxa/secondary/{id}
```

## Todo

- ✅ Add `jsp-actor-dictionary.yaml` to `crates/fractalaw-core/data/`
- ✅ Separate JSP parser module (`fractalaw-core/src/jsp/`) — "will"/"is to" as mandatory modals
- ✅ Zenoh key expressions + query/publish methods in `fractalaw-sync` crate
- ✅ `fractalaw jsp enrich {source_id}` — reads DuckDB, writes DuckDB (no Zenoh)
- ✅ `fractalaw jsp stats` — reads DuckDB staging table
- ✅ Updated sertantai Zenoh spec (`ZENOH-SECONDARY-SOURCES.md` v1.1)
- ✅ `fractalaw-sync pull-secondary` — pull provisions from sertantai into DuckDB staging
- ✅ `fractalaw-sync publish-secondary` — publish enrichment from DuckDB to sertantai
- ✅ Sertantai: SecondaryTaxaSubscriber (raised #125 for section_id lookup bug, fixed)
- ✅ Verify: end-to-end — pull 174 → enrich 117 → publish 117 → sertantai updated 117

## Backout: Zenoh removed from fractalaw-cli

Early implementation incorrectly added Zenoh dependencies and connection logic to `fractalaw-cli/commands/jsp.rs`. Backed out — `fractalaw-cli` never touches Zenoh. Pull and publish moved to `fractalaw-sync-cli`.
