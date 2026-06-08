# Session: Cascade v0.3 Implementation — Regex as Sieve

## Context

**Strategy**: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`
**Gemini review**: `docs/reviews/gemini-cascade-v03-review-20260608.md`
**Prior session**: `.claude/sessions/cascade/06-08-26-local-model-tier2.md`

QA data (53 samples) proved regex confidence is not predictive. v0.3 demotes regex to sieve, routes by actor count. Gemini validates the approach.

## What to build

### 1. Revise Tier 2 filter (priority — core control flow)

Current filter: multi-actor AND all-active AND existing_conf < 0.80 AND DRRP not empty

New filter: (multi-actor) OR (single-actor AND DRRP=none AND has actors)

This routes the two failure categories (47% parser misses + 36% position wrong) to Tier 2.

### 2. Extend Tier 2 prompt for DRRP classification

Current Tier 2 prompt only classifies positions (active/counterparty/etc). For provisions with DRRP=none, it also needs to determine the DRRP type (Duty/Right/Responsibility/Power/none).

Add to prompt: "Also classify the DRRP type if the provision contains an obligation."

### 3. Revise confidence scoring

Current: `taxa_confidence` from regex match quality (not predictive).

New:
- Purpose gate structural skip → 0.90 (definitely not DRRP)
- Single-actor + DRRP match → 0.80 (regex reliable core)
- Single-actor + DRRP=none → 0.30 (elevate to Tier 2)
- Multi-actor → 0.30 (always elevate to Tier 2)
- Tier 2 validated → 0.80
- Tier 3 / QA correction → 0.90

### 4. Write DRRP type from Tier 2 back to LanceDB

Currently Tier 2 only writes actors. When it classifies DRRP type, write that back too.

## Verification

- Re-enrich MHR with new filter
- QA 20% of MHR provisions
- Target: >60% precision (up from 22% regex, 40% current)
- Each QA pass should show improvement as corrections accumulate

## Files to modify

| File | Change |
|------|--------|
| `crates/fractalaw-cli/src/main.rs` | Tier 2 filter, prompt, DRRP write-back, confidence scoring |
| `.claude/skills/drrp-qa/run_qa.py` | May need adjustment for new extraction_method values |

## References

- Strategy: `docs/CLASSIFICATION-CASCADE-STRATEGY-v0.3.md`
- Review: `docs/reviews/gemini-cascade-v03-review-20260608.md`
- QA results: `data/qa-results/drrp-qa-*.json`
- Prior session: `.claude/sessions/cascade/06-08-26-local-model-tier2.md`
