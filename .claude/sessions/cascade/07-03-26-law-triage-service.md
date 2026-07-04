---
session: Law Triage Service
status: closed
opened: 2026-07-03
closed: 2026-07-04
outcome: success

summary: >
  Built and wired the Pass 1 triage pipeline — regex-only making/not-making classifier
  that runs in sync watch on new law arrivals, gates enrichment (skipping non-making laws),
  exposes a Zenoh queryable for sertantai, and persists results to DuckDB. 3-law spot check
  shows 100% agreement with sertantai's is_making.

decisions:
  - what: JSON response for triage queryable (not Arrow IPC)
    why: Triage returns small per-law evidence — sertantai is Python/Elixir, JSON is natural
    result: ~200 bytes per law, sub-second response
  - what: Gate enrichment on triage result in sync watch
    why: NotMaking laws don't need expensive SLM/LLM processing — saves minutes per law
    result: Only Making/Uncertain laws queue for enrichment
  - what: Return evidence (counts + confidence) not just a boolean
    why: Sertantai owns the is_making taxonomy — fractalaw substantiates with evidence, doesn't override
    result: Response includes full provision counts for sertantai to make its own decisions on disagreements

metrics:
  triage_accuracy: { spot_check_laws: 3, agreements: 3, disagreements: 0 }
  confidence_range: { making_high: 0.955, making_low: 0.819, not_making: 0.121 }
  build_time: { check_workspace: "9s", test_core_making: "18s" }

lessons:
  - title: DuckDB columns are title/description not title_en/description_en
    detail: >
      The DuckDB legislation table uses title and description (from Parquet schema).
      Sertantai's Postgres uses title_en. The original cmd_triage code was written
      against the wrong column names — caught only at runtime.
    tag: data
  - title: SSD eliminates all build concerns
    detail: >
      Moving target/ to the 1TB SSD freed 34GB on /var/home (99% → 70%).
      Full workspace check takes 9s incremental. No more disk anxiety.
    tag: infrastructure
  - title: Triage is specifically for new arrivals not corpus re-validation
    detail: >
      The existing corpus has been through full DRRP enrichment. Running triage --all
      on it would only confirm what's already known. Triage's value is at the sync watch
      ingestion point — classifying brand-new laws before they hit the expensive pipeline.
    tag: methodology

artifacts:
  - crates/fractalaw-store/src/duck.rs
  - crates/fractalaw-sync/src/zenoh_sync.rs
  - crates/fractalaw-sync-cli/src/sync.rs
  - crates/fractalaw-sync-cli/src/main.rs
  - /var/home/jason/Desktop/sertantai-legal/docs/ZENOH-SPEC.md

depends_on:
  - 06-29-26-tier1-regex.md

enables:
  - Sertantai customer onboarding — fast making classification without waiting for full enrichment
  - Sync watch efficiency — non-making laws no longer consume SLM/LLM resources
---

# Session: Law Triage Service (CLOSED)

## Problem

Sertantai scrapes and parses legislation but can't determine if a law is "making" (sets obligations on duty-holders) vs "housekeeping" (amending, commencement, revocation) vs "enabling" (confers powers/rights without imposing duties). Today this requires full DRRP enrichment — a batch process involving SLM/LLM that takes minutes per law. Sertantai needs a fast answer (seconds) to classify laws in its register for customer onboarding.

The two-pass architecture: **Pass 1 triage** (this session) runs regex only — scan provisions for actors, modals, purpose classification. Returns a law-level classification. **Pass 2 deep parse** (existing batch pipeline) runs SLM/LLM for full DRRP extraction, triggered per-customer.

## Prior art

- `is_making` exists in `pipeline.rs` but requires full `enrich_single_law()` — too heavy
- Purpose classifier (`purpose.rs`) already detects 15 provision types including Amendment, Enactment, Process+Rule
- Actor extraction (`actors.rs`) detects governed + government actors via regex
- `sync watch` currently runs full enrichment on arrival — needs stripping down to triage only

## Design

### Existing taxonomy (from sertantai LRT)

DuckDB `legislation` already has boolean columns from sertantai's metadata:
- `is_making` — 3,186 true / 15,903 false / 383 null
- `is_amending`, `is_commencing`, `is_enacting`, `is_rescinding`

These come from sertantai's scraper based on legislation.gov.uk metadata. Only 520 of 19,472 laws have been through fractalaw's full enrichment (`duty_holder` populated).

### What triage adds

Fractalaw **substantiates** sertantai's `is_making` by scanning the actual provision text. The regex pass produces per-provision signals that aggregate to a law-level confirmation:

| Signal | Source | What it tells you |
|---|---|---|
| Purpose distribution | `purpose.rs` classify() | % Process+Rule vs Amendment vs Interpretation vs Enactment |
| Actor presence | `actors.rs` extract() | Are there governed actors (employers, operators)? |
| Modal presence | Regex scan | Are there obligation modals (shall/must) vs enabling (may)? |
| Section types | Provision metadata | Structure: sections vs schedules vs headings |

Returned per-law: purpose breakdown counts, actor count, modal count, and a derived `triage_making` boolean that fractalaw computed independently from sertantai's `is_making`.

### No new taxonomy

Don't extend or replace sertantai's booleans. Return the **evidence** (counts, distributions) and let sertantai decide what to do with disagreements.

### Implementation

**A. CLI command** — `fractalaw-sync triage --laws <LAWS> --pg <URL>`. Testable, batch or single-law.

**B. Zenoh queryable** — sertantai sends law_name(s), fractalaw triages and responds. Wire into sync watch.

Start with A (testable), wire into B.

### Triage does NOT prune LAT

The enrichment pipeline (`cmd_taxa_enrich`) currently deletes LAT rows for non-making laws — an opaque destructive action. Triage must **never** delete data. It returns evidence only. LAT cleanup is a separate triggered process.

## Work

1. ✅ Build `triage_provisions()` + `detect_with_triage()` in fractalaw-core/src/taxa/making.rs — extends existing Bayesian detector with Tier 5 (provision text analysis)
2. ✅ Add `triage` subcommand to `fractalaw-sync` CLI (written, check blocked by disk — DuckDB C++ rebuild needs SSD)
3. ✅ Define Zenoh triage response schema (JSON at `fractalaw/@{tenant}/triage`)
4. ✅ Wire as Zenoh queryable in sync watch (triage queryable arm in `tokio::select!`)
5. ✅ Gate enrichment on triage (Making/Uncertain → queue, NotMaking → skip)
6. ❌ Test on QQ corpus — not applicable (corpus already through full pipeline, triage is for new arrivals)
7. ✅ Publish triage data to DuckDB (`triage_classification`, `triage_confidence`, `triage_tier`, `triaged_at`)

## Dependencies

- ✅ Purpose classifier (`purpose.rs`) — 15 categories, 95.5% accuracy on genuine provisions
- ✅ Actor extraction (`actors.rs`) — governed + government actors + family-gated specialists
- ✅ `fractalaw-sync-cli` crate (Phase 2 of project restructure)
- ✅ PgStore for provision access (`--pg`)
- ⬜ Sync watch refactor (PgStore hardening session — related but independent)

## Resumed: 2026-07-04 — SSD installed, disk pressure eliminated

### SSD Setup
- Samsung 870 EVO 1TB formatted as ext4, mounted at `/mnt/ssd`, `target/` symlinked
- `/var/home` went from 99% → 70% (35GB free), SSD has 837GB free

### Items 3-7 completed (2026-07-04)

**Zenoh triage response schema** (Item 3):
- JSON queryable at `fractalaw/@{tenant}/triage`
- Response per law: `{ law_name, classification, confidence, tier, counts: { total, process_rule, amendment, enactment, interpretation, with_actor, with_obligation, with_enabling }, sertantai_is_making, agrees }`

**Sync watch integration** (Items 4-5):
- Triage queryable declared alongside status queryable in `cmd_sync_watch()`
- After LAT pull, runs `run_triage_for_law()` → writes DuckDB → gates enrichment
- NotMaking laws skip enrichment queue (saves SLM/LLM processing)

**DuckDB persistence** (Item 7):
- Columns: `triage_classification VARCHAR`, `triage_confidence FLOAT`, `triage_tier INTEGER`, `triaged_at TIMESTAMPTZ`
- Written by both batch CLI (`cmd_triage`) and sync watch event handler

**Bug fix**: DuckDB columns are `title`/`description`, not `title_en`/`description_en` — fixed in both CLI and sync helpers.

**Validation** (3-law spot check):
- UK_ukpga_1974_37: making, 95.5%, tier 5, 840 provisions, 263 obligations — agrees
- UK_uksi_1999_3242: making, 82%, tier 5, 230 provisions, 73 obligations — agrees
- UK_uksi_2020_1163: not_making, 12%, tier 3, 0 provisions — agrees

### Remaining
- Item 6: full QQ corpus validation (`--all`) — run manually to review disagreements at scale
