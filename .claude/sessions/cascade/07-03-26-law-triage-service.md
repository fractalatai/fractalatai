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
3. ⬜ Define Zenoh triage response schema (for sertantai contract)
4. ⬜ Wire as Zenoh queryable in sync watch
5. ⬜ Strip enrichment queue logic from sync watch
6. ⬜ Test on QQ corpus — validate against known making/non-making laws
7. ⬜ Publish triage data to DuckDB (law-level classification columns)

## Dependencies

- ✅ Purpose classifier (`purpose.rs`) — 15 categories, 95.5% accuracy on genuine provisions
- ✅ Actor extraction (`actors.rs`) — governed + government actors + family-gated specialists
- ✅ `fractalaw-sync-cli` crate (Phase 2 of project restructure)
- ✅ PgStore for provision access (`--pg`)
- ⬜ Sync watch refactor (PgStore hardening session — related but independent)
