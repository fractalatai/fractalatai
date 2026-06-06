# Session: Next — Gap C Phase 2A: Actors Struct + Tier 3 Integration

## Context

**Meta-plan**: `.claude/plans/gap-c-tiered-resolution.md`
**Design doc**: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` v0.4 + Appendix A
**Prior sessions**:
- Phase 1A: Tier 1 deterministic inheritance — 6,175 provisions, 76% precision
- Phase 1B: Deferred (fractalaw/fractalaw#36) — zero C4 candidates in corpus
- Phase 1C: Tier 3 LLM POC — 8/9 correct, holder/recipient distinction validated, recipient model discovered

## Objective

Integrate the unified `actors` struct into the enrichment pipeline and wire Tier 3 LLM into `enrich_single_law()` for provisions where Tier 1 can't distinguish holder from recipient.

## Scope

### 1. Actors struct schema

Add `actors: List<Struct(label: Utf8, role: Utf8, recipient_type: Utf8)>` to LanceDB legislation_text.

Roles: `holder`, `recipient`, `beneficiary`, `mentioned`
Recipient types (role=recipient only): `protected_person`, `regulated_actor`, `informed_party`

**Non-breaking migration**: populate BOTH the new struct AND the existing flat columns (`governed_actors`, `government_actors`). Sertantai reads flat columns until it migrates.

### 2. Tier 3 LLM integration

Wire into `enrich_single_law()` after Tier 1:
- Provisions where Tier 1 inherited multiple actors → send to LLM for holder/recipient classification
- LLM returns actors with roles → populate the struct
- Externally-derived confidence (evidence_sections, reasoning_type)
- Valid actor enum injected into prompt (from actors.rs)
- Model routing: Gemini 2.5 Flash (proven in POC)

### 3. Tier 1 enrichment of actors struct

Tier 1 inherited provisions also get the struct populated:
- Inherited actor → role: holder
- No recipient data from Tier 1 (deterministic, can't distinguish)
- extraction_method: "inherited"

Regex provisions get a basic struct:
- All governed_actors → role: holder (existing behaviour, struct form)
- No recipient data from regex
- extraction_method: "regex"

### 4. QA

Re-run tier1-qa skill after integration to measure precision improvement.
Expected: Tier 3 corrects the Pattern 2 failures (recipient-as-holder), pushing precision above 85%.

## Key decisions already made

- Unified struct over flat columns (confirmed 2026-06-06)
- Non-breaking migration (additive, dual-write)
- beneficiary is a role, not a recipient_type (Gemini naming conflict resolved)
- Externally-derived confidence, not LLM self-reported
- Gemini 2.5 Flash for Tier 3 (validated in POC)
- `--gap-c` flag (opt-in, existing)

## Files likely to change

- `fractalaw-core/src/schema.rs` — add actors struct to legislation_text_schema
- `fractalaw-cli/src/main.rs` — Tier 3 call after Tier 1, actors struct builder, dual-write
- `fractalaw-store/src/lance.rs` — ensure_actors_column() migration
- New: LLM client module (Gemini API, prompt template, response parser)
- `fractalaw-sync/src/zenoh_sync.rs` — include actors struct in provision publish payload

## Shipped (2026-06-06)

### Actors JSON struct (commit `b2faef8`)
- `actors` column added to LanceDB legislation_text (Utf8 JSON)
- `ActorEntry` struct: `{label, role, recipient_type}`
- Populated for all provisions: regex → role=holder, inherited → role=holder
- Dual-write: flat columns (`governed_actors`, `government_actors`) still populated
- Added to provision publish payload (`query_provision_taxa`)
- HSWA verified: s.2(2)(a) shows `{"label":"Org: Employer","role":"holder"}`

### Also shipped this session
- Phase 1B deferred → fractalaw/fractalaw#36
- Distance-0 sibling fix (commit `5fc161e`)
- Pattern 1 heading exclusion fix (commit `f14932f`)
- Tier 1 QA skill with Bayesian inference (`.claude/skills/tier1-qa/`)
- Tier 3 POC validated (commit `51097cf`) — 8/9 holder/recipient correct
- Recipient model added to design doc (Appendix A, Gemini-reviewed)
- Unified actors struct confirmed as architectural direction
- Non-breaking migration strategy documented

## What's next

### Before Tier 3: Rebuild LanceDB table with proper Arrow schema

The `actors` column is currently stored as JSON string (Utf8) because LanceDB can't create `List<Struct>` via `add_columns()`. Before wiring in Tier 3 LLM calls, rebuild the LanceDB table with the actors struct as a native Arrow `List<Struct(label: Utf8, role: Utf8, recipient_type: Utf8)>`.

Rationale: building Tier 3 on a JSON column we know we'll migrate is technical debt from day one. The table rebuild gives us:
- Native Arrow struct for actors (queryable, type-safe)
- All Gap C columns native from the start (no `ensure_gap_c_columns` migration)
- Clean schema for the Tier 3 write path
- Opportunity to validate embeddings and clean up any schema drift

Risk: embeddings (97K rows, ~9 hours CPU). Mitigated by export-to-Parquet before rebuild, reimport embeddings from backup.

### Then: Tier 3 LLM integration
- Wire Gemini 2.5 Flash into `enrich_single_law()` after Tier 1
- Fires on inherited provisions with multiple actors
- Writes holder/recipient/beneficiary roles to native Arrow struct
- Updates flat columns with holder-only for backward compat
- Re-run QA → target >85% precision

## References

- Design doc: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` (actors struct, recipient model, Appendix A)
- Tier 3 POC: `.claude/skills/tier1-qa/tier3_poc.py`
- QA skill: `.claude/skills/tier1-qa/run_qa.py`
- Actor dictionary: `docs/ACTOR-DICTIONARY.md`
- Phase 1A results: `.claude/sessions/06-05-26-gap-c-phase-1a.md`
- Phase 1C results: `.claude/sessions/06-06-26-gap-c-phase-1c-tier3-poc.md`
- LanceDB backup strategy: memory `feedback_lancedb_enrichment.md`
