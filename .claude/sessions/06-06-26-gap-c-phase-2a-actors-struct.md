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

## Exit criteria

- HSWA enriched with actors struct populated (holders + recipients where Tier 3 fires)
- Tier 1 QA precision >85% (Pattern 2 failures resolved by Tier 3)
- Flat columns still populated (sertantai backward compat verified)
- Cost measured: Tier 3 API calls per law

## References

- Design doc: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` (actors struct, recipient model, Appendix A)
- Tier 3 POC: `.claude/skills/tier1-qa/tier3_poc.py`
- QA skill: `.claude/skills/tier1-qa/run_qa.py`
- Actor dictionary: `docs/ACTOR-DICTIONARY.md`
- Phase 1A results: `.claude/sessions/06-05-26-gap-c-phase-1a.md`
- Phase 1C results: `.claude/sessions/06-06-26-gap-c-phase-1c-tier3-poc.md`
