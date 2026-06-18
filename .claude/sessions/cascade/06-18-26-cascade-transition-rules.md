# Session: Cascade Transition Rules — Codify in Code

## Context

**Prior sessions**: gold-standard-correction (CLOSED), offence-provision-gating (CLOSED)
**Trigger**: The cascade transition rules (regex → classifier → LLM) were documented in the gold correction session but never implemented in code. The pipeline makes ad-hoc decisions about when each tier runs.

## Current state (what the code does)

### Regex (always runs)
- `parse_v2()` runs on every provision
- Writes `drrp_types`, `actors` with `reason = "regex:{position}@{confidence}"`
- Purpose gate skips structural provisions (Enactment, Interpretation, Offence, Amendment, etc.)
- Legal fiction rejection suppresses "shall be treated/deemed" false positives

### Classifier (runs in `--force` and `--pending`)
- Phase 3: DRRP classifier (v8, Obligation/Liberty/none)
- Only runs on provisions where `tier < source_tier("classifier")`
- Confidence threshold: 0.7 for gaps (regex=none), 0.9 for overrides
- Writes `drrp_types` and `extraction_method = "classifier"`
- Phase 4: Position classifier — appends `| classifier:{position}@{confidence}` to reason

### LLM (runs in `--gap-c` with GEMINI_API_KEY)
- Only runs on inherited provisions and provisions with multi-actor or no DRRP
- Receives provision text + optional parent context
- Context gap: no sibling provisions (#38)

## What's wanted (the 5 rules)

### Rule 1: Regex always runs first ✓
Implemented. `parse_v2()` runs on every provision.

### Rule 2: Classifier always runs second (when embedding exists)
**GAP**: Classifier only runs in `--force`/`--pending`, not in normal enrichment.
**GAP**: Classifier doesn't append to reason when it AGREES with regex — only Phase 4 (position) does.
**GAP**: Phase 3 (DRRP) doesn't write to reason at all — only overwrites `drrp_types`.

### Rule 3: Disagreements are LLM escalation candidates
**GAP**: No mechanism to flag disagreements for LLM review.
**GAP**: No detection of "both obligation and enabling modals present" (#41) as an LLM candidate.
**WANTED**: A `needs_llm_review` flag or queue that the Tier 3 pass can consume.

### Rule 4: drrp_types reflects highest-tier non-none result
**PARTIAL**: Confidence thresholding (0.7/0.9) partially implements this.
**GAP**: When regex and classifier disagree, the classifier silently overrides if above threshold. Should hold for LLM instead.

### Rule 5: QA findings tracked at provision level
**GAP**: No systematic tracking. Findings discovered in CLI output and lost.
**WANTED**: A skill that captures QA findings per provision.

## The gap (to close)

1. **Phase 3 should write DRRP provenance to reason** — currently only Phase 4 (position) writes to reason. Phase 3 should record what the DRRP classifier predicted, even when agreeing with regex.

2. **Disagreement detection** — when regex says X and classifier says Y:
   - Record both in reason: `regex:Obligation@0.80 | classifier:Liberty@0.72`
   - Flag for LLM review (don't override silently)
   - When both modals present (obligation + enabling), flag for LLM

3. **LLM candidate queue** — a column or flag in LanceDB that marks provisions needing LLM review. The Tier 3 pass consumes this queue.

4. **Run classifier in normal enrichment** — not just `--force`/`--pending`. Every enrichment should run Phase 3/4 on provisions with embeddings.

## Approach

1. Add DRRP provenance to Phase 3 (parallel to Phase 4's position provenance)
2. Add disagreement detection between regex and classifier DRRP predictions
3. Add "both modals present" detection as an LLM escalation signal
4. Create an `llm_candidate` column or use extraction_method = "pending_llm"
5. Wire classifier into normal enrichment (not just --force/--pending)

## Key files

- `crates/fractalaw-cli/src/main.rs` — Phase 3 (DRRP classify), Phase 4 (position classify), enrichment flow
- `crates/fractalaw-core/src/taxa/mod.rs` — `parse_v2()`, purpose gates
- `docs/drrp_classifier_v8.json` — current classifier weights
