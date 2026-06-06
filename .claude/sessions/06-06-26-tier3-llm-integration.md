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

## Expected outcome

- Multi-actor inherited provisions get correct holder/recipient classification
- Recipient data available for downstream filtering ("show me protections for workers")
- `extraction_method = "agentic"` distinguishes LLM-resolved from deterministic

## Files to modify

| File | Change |
|------|--------|
| `crates/fractalaw-cli/src/main.rs` | Gemini API call after Tier 1, response parser, actors struct update |
| `crates/fractalaw-cli/Cargo.toml` | Already has reqwest, serde, serde_json |

## References

- Tier 3 POC: `.claude/skills/tier1-qa/tier3_poc.py`
- QA skill: `.claude/skills/tier1-qa/run_qa.py`
- Actor dictionary: `docs/ACTOR-DICTIONARY.md`
- Design doc: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` (Appendix A)
- Phase 2A session: `.claude/sessions/06-06-26-gap-c-phase-2a-actors-struct.md`
- LanceDB rebuild session: `.claude/sessions/06-06-26-lancedb-rebuild-tier3.md`
