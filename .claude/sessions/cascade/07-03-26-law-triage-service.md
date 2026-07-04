# Session: Law Triage Service (ACTIVE)

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
6. ⬜ Test on QQ corpus — validate against known making/non-making laws
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
