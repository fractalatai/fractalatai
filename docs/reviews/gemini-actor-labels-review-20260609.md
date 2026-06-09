# Gemini Review: Actor Label Matching Approach

**Date:** 2026-06-09
**Model:** Gemini 2.5 Flash

## Summary

Approach is sound. Key recommendations:

### 1. Matching algorithm
- **Don't just substring match** — build an alias/trigger_phrases dictionary
- Each canonical label gets a curated list of aliases (e.g., `Gvt: Authority: Enforcement` → `["enforcing authority", "enforcement body", "the regulator"]`)
- Multi-stage scoring: exact match > substring > token similarity > edit distance
- Prefer longer/more specific trigger phrases on tie-break
- This fixes the Secretary of State / self-employed person bugs

### 2. Dictionary as Single Source of Truth
- Store in a version-controlled file (`dictionary.yaml` or similar)
- Both fractalaw and sertantai pull from the same source
- Each entry: `canonical_label`, `prefix_category`, `trigger_phrases[]`
- Git provides history, diffing, rollback

### 3. Discoveries: ALWAYS human-gated
- Never auto-add to dictionary
- Discovery queue → human review → add as new label OR add as alias to existing
- Prevents dictionary pollution and maintains trust

### 4. Biggest risk
- Matching accuracy — if the matcher is wrong, the whole pipeline is wrong
- Mitigate with test suite and golden dataset for regression testing
