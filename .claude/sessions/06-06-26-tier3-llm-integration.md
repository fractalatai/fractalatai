# Session: Next — Tier 3 LLM Integration

## Context

**Meta-plan**: `.claude/plans/gap-c-tiered-resolution.md`
**Design doc**: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` v0.4 + Appendix A
**Prior**:
- Phase 1A: Tier 1 deterministic inheritance — 6,175 provisions, 76% precision
- Phase 1C: Tier 3 POC validated — 8/9 holder/recipient correct
- Phase 2A: Actors JSON struct shipped (commit `b2faef8`)
- LanceDB rebuilt with native Arrow `List<Struct>` actors (commit `487ef6c`)
- Full corpus re-enriched — 67,303 provisions with native actors (commit `6e5c5a3`)

## Objective

Wire Gemini 2.5 Flash into `enrich_single_law()` for provisions where Tier 1 can't distinguish holder from recipient, then re-run QA to measure precision improvement.

## When Tier 3 fires

After Tier 1 pass, for provisions where:
- `extraction_method == "inherited"`
- `governed_actors.len() > 1` (multiple actors inherited — can't tell which is holder vs recipient)

## Implementation

### 1. Gemini API client (Rust, reqwest)
- Call Gemini 2.5 Flash REST API via `reqwest` (already in workspace)
- Same prompt validated in POC (`.claude/skills/tier1-qa/tier3_poc.py`)
- Input: parent text + target text + actor list (from actors.rs enum)
- Output: JSON with per-actor role classification (holder/recipient/beneficiary/mentioned)
- GEMINI_API_KEY from environment

### 2. Integration into enrich_single_law()
- After Tier 1 inheritance pass, iterate inherited provisions with multiple actors
- Send to Gemini for holder/recipient classification
- Parse response → populate actors struct with full role data
- Update flat columns: `governed_actors` = holders only, `government_actors` = government holders only
- Set `extraction_method = "agentic"`

### 3. Rate limiting & error handling
- Sequential calls, one per provision
- Estimated ~500-1000 provisions in customer corpus
- 30-second timeout per call
- On failure: keep Tier 1 result, log warning, continue

### 4. QA
- Re-run `run_qa.py --sample-size 40` after integration
- Target: precision >85% (up from 76% at Tier 1)
- Tier 3 should correct Pattern 2 failures (recipient-as-holder)

## Shipped (2026-06-06)

### Tier 3 LLM integration (commit `acda60a`)
- Gemini 2.5 Flash wired into `enrich_single_law()` after Tier 1 pass
- Fires on inherited provisions with `governed_actors.len() > 1`
- REST API via reqwest, `thinkingBudget: 256` + `maxOutputTokens: 2048`
- Prompt constrains actor labels to dictionary values
- Parses JSON response → populates native Arrow actors struct with roles
- Updates flat columns (governed_actors = holders only) for backward compat
- Sets `extraction_method = "agentic"`
- HSWA test: **8/8 multi-actor provisions classified**

### Key debugging finding
- Gemini 2.5 Flash thinking tokens consume the `maxOutputTokens` budget
- With `maxOutputTokens: 512`, thinking used ~490 tokens leaving ~20 for output → `MAX_TOKENS` truncation
- Fix: `thinkingBudget: 256` caps thinking, `maxOutputTokens: 2048` gives headroom
- Python SDK (`google.genai`) handles this transparently; REST API needs explicit config

### Also this session (LanceDB rebuild)
- LanceDB rebuilt with native Arrow `List<Struct>` actors column (commit `487ef6c`)
- Fragment bloat reduced: 8.6 GB → 374 MB
- Full corpus re-enriched: 67,303 provisions with native actors (commit `6e5c5a3`)
- NAS backup skill created (`.claude/skills/nas-backup/`)
- Bulk enrichment skill created (`.claude/skills/bulk-enrichment/`)
- Compact script: `scripts/compact_lance.py`

### Label validation (commit `a64ab4b`)
- `all_actor_labels()` function in actors.rs returns all valid dictionary labels
- Per-actor `label_source`: `canonical` (in dictionary) or `invented` (LLM-created)
- `extraction_method = "agentic_unvalidated"` when any actor has invented label
- LLM signal preserved even with non-dictionary labels — sertantai can filter

### Actors struct finalised (commit `158b834`)
- `reason: Utf8?` per actor — LLM reasoning for the classification (null for regex/inherited)
- `role` taxonomy extended: `primary-holder` for LLM's primary pick among multiple holders
- Full struct: `{label, role, recipient_type, label_source, reason}`

### Test suite (commit `93b6be1`)
- Extracted `parse_gemini_response()` and `parse_tier3_actors()` as testable functions
- 12 unit tests with canned responses — no API calls for parsing iteration
- Covers: plain JSON, code fences, truncated responses, label validation, role mapping, primary-holder promotion

### OH&S corpus enrichment
- OH&S: Occupational / Personal Safety enriched with Tier 3
- **131 Tier 1 inherited + 17/17 Tier 3 classified (all validated, 0 invented)**

### Sertantai briefing
- Migration briefing written: `~/Desktop/sertantai-legal/backend/data/fractalaw-actors-struct-migration.md`
- Documents struct schema, role taxonomy, label_source, reason field
- Flags `(inferred)` suffix bug in `baserow.ex` — duplicates every actor in single-selects
- Migration path: phase 1 read struct alongside flat columns, phase 2 drop flat columns

## Known issues for next session

### Inference quality
- HSWA s.19/s.21: Inspector/Person role assignments need human review — complex enforcement chain (HSC → HSE → Inspector via warrant)
- HSC (Health and Safety Commission) not in actor dictionary — merged into HSE in 2008 but still referenced in HSWA
- `recipient_type` currently hardcoded to `protected_person` — need `regulated_actor` and `informed_party` classification

### Sertantai integration
- Sertantai needs to consume new actors struct (currently reads flat columns only)
- `(inferred)` suffix in `baserow.ex` `@holder_options` needs removing — doubles single-select options
- Automated QA (LLM checking LLM) is circular — real validation is human review through sertantai UI

## What's next

1. Sertantai migration — consume actors struct, remove `(inferred)` bug
2. Re-enrich OH&S with latest schema (reason + primary-holder populated)
3. Publish to sertantai via zenoh for human review in UI
4. Iterate on prompt/label quality based on UI review findings
5. Broader corpus enrichment with Tier 3

## References

- Tier 3 POC: `.claude/skills/tier1-qa/tier3_poc.py`
- QA skill: `.claude/skills/tier1-qa/run_qa.py`
- Actor dictionary: `docs/ACTOR-DICTIONARY.md`
- Design doc: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` (Appendix A)
- Sertantai briefing: `~/Desktop/sertantai-legal/backend/data/fractalaw-actors-struct-migration.md`
- Phase 2A session: `.claude/sessions/06-06-26-gap-c-phase-2a-actors-struct.md`
- LanceDB rebuild session: `.claude/sessions/06-06-26-lancedb-rebuild-tier3.md`
