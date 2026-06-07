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

### OH&S publish + sertantai validation
- Removed flat actor columns (`governed_actors`, `government_actors`) from zenoh provision payload (commit `5a51c34`)
- Published OH&S family to sertantai: **14,100 provisions across 68 making laws**
- Sertantai analysis confirmed:
  - 4,800 provisions with actors, 8,849 total actor entries
  - Extraction methods: regex (4,185), inherited (802), agentic (251)
  - Roles: holder (8,588), mentioned (112), recipient (90), beneficiary (59)
  - 100% canonical labels
  - **1,409 provisions with struct data but no flat columns** — entirely new coverage from Tier 1 tree walk
  - **37% more actors per provision** vs flat columns (1.49 vs 1.09)
  - Flat columns were silently dropping ~30% of holder actors

### Crown/HM Forces classification discovery
- Crown and HM Forces are in `GOVERNMENT_DEFS` but act as duty holders in many provisions (HSWA s.48 binds the Crown)
- Gemini confirmed: Crown is **both** authority and duty holder depending on context
- The actors struct `role` field handles this per-provision — but current role taxonomy doesn't map to DRRP

## Critical design issue: role taxonomy must align with DRRP

The `role` field currently uses: `primary-holder | holder | recipient | beneficiary | mentioned`

This fails to capture the core DRRP taxonomy we extract per provision. `holder` is doing all the lifting — a duty-bearing employer and a power-wielding inspector both get `role: holder`, losing the distinction that DRRP already captures.

### Proposed role taxonomy

```
duty-holder        -- bears a duty (must act) — maps to DRRP "Duty"
right-holder       -- holds a right — maps to DRRP "Right"
responsibility-holder -- bears a responsibility — maps to DRRP "Responsibility"
power-holder       -- exercises a power — maps to DRRP "Power"
recipient          -- receives protection/information/training
beneficiary        -- benefits without active obligation
mentioned          -- referenced but no active role
```

Plus `primary-` prefix for the LLM's primary pick (e.g., `primary-duty-holder`).

### Why this matters
- Sertantai currently uses the `Gov:`/`Ind:`/`Org:` label prefix to infer governed vs government — a proxy for what the role should tell it directly
- `drrp_types` already classifies each provision as Duty/Right/Responsibility/Power — the role should reflect which actor holds which DRRP type
- Crown/HM Forces dual nature is naturally handled: `power-holder` in enforcement provisions, `duty-holder` in workplace safety provisions
- Removes the need for the governed/government dictionary split as the primary classification axis

### Implementation scope
- Schema: role field values change
- Tier 3 prompt: ask LLM to classify using DRRP roles
- Tier 1/regex: map from `drrp_types` on the provision to set role (e.g., if provision is a Duty, actors are duty-holders)
- Tests: update canned responses and assertions
- Sertantai briefing: update role taxonomy
- LanceDB: re-enrich to populate new roles

## Known issues

- HSWA s.19/s.21: Inspector/Person role assignments need human review
- HSC (Health and Safety Commission) not in actor dictionary
- `recipient_type` currently hardcoded to `protected_person`
- `(inferred)` suffix in sertantai `baserow.ex` needs removing

## What's next

1. **DRRP role taxonomy** — extend role values to align with Duty/Right/Responsibility/Power
2. Re-enrich OH&S with DRRP roles + reason + primary-holder
3. Publish to sertantai for human review
4. Sertantai migration — consume actors struct with DRRP roles, drop flat columns
5. Broader corpus enrichment

## References

- Tier 3 POC: `.claude/skills/tier1-qa/tier3_poc.py`
- QA skill: `.claude/skills/tier1-qa/run_qa.py`
- Actor dictionary: `docs/ACTOR-DICTIONARY.md`
- Design doc: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` (Appendix A)
- Sertantai briefing: `~/Desktop/sertantai-legal/backend/data/fractalaw-actors-struct-migration.md`
- Phase 2A session: `.claude/sessions/06-06-26-gap-c-phase-2a-actors-struct.md`
- LanceDB rebuild session: `.claude/sessions/06-06-26-lancedb-rebuild-tier3.md`
