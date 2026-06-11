# Gemini Review: Position Classifier Actor Struct Extension

**Date:** 2026-06-11
**Model:** Gemini 2.5 Flash

## Summary

Endorses the `reason` field overload as pragmatic given no-schema-migration constraint. Raises important concerns about mixed semantics and the training feedback loop.

## Key Feedback

### 1. Overloading `reason` — acceptable with caveats
- No schema migration is the biggest win
- Co-location with the actor is good for data integrity
- **Risk**: mixed semantics (free-text LLM reason vs structured classifier signal)
- **Risk**: if an agentic provision also has a classifier disagreement, the LLM reason is overwritten
- Document the convention clearly

### 2. `classifier:active@0.82` format — robust enough
- Concise, human-readable, easy to parse with `startsWith` and string splitting
- JSON would be more extensible but overkill for a single prediction
- Stick with the custom format

### 3. Conflict resolution — don't auto-override
- `position` field stays as regex-assigned (source of truth)
- UI should flag disagreements visually
- Classifier should NOT automatically override `position`
- Provide mechanism for human to choose correct position (manual override)

### 4. Training feedback loop — human-in-the-loop is crucial
- Disagreements are CANDIDATES for training, not training data
- Must go through human review before entering gold standard
- Do NOT feed classifier predictions back into training without validation
- The QA report (Step 5) must route to human review first

### 5. Missing/risky
- No way to distinguish "classifier agreed" from "classifier didn't run" (both null) — acceptable for now
- Loss of LLM reason when classifier disagrees on an agentic provision — flag for attention
- Consider `classifier:agree` marker in future if "ran and agreed" signal is needed
