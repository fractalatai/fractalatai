# Gemini Review: Development/Production Interface Rules

**Date:** 2026-06-09
**Model:** Gemini 2.5 Flash

## Key findings

### Rules 1,2,4,5 are sound
Confidence hierarchy enforces the ratchet correctly.

### Rule 3 gap — taxonomy migration was premature
The Obligation/Liberty migration entrenched the mixed taxonomy problem rather than solving it. Reverting is the right call.

### Regex confidence overlap risk
If any regex provision hits 0.85+, it would be protected from the classifier. **Must confirm: max(regex_confidence) < 0.85.**

### Mixed actor decomposition gap (CRITICAL)
If a provision has BOTH Gvt: and Org: actors, the decomposition is ambiguous:
- Obligation + Gvt: = Responsibility
- Obligation + Org: = Duty
- **Which wins when both are present?**

Need an explicit rule. Options:
- Gvt: takes precedence (the provision is about government regulation)
- Store multiple DRRP types (drrp_types is already a list)
- Use the active actor's prefix (the doer determines the type)

### Operational concerns
- Resource contention on shared LanceDB during concurrent dev/prod runs
- Rollback granularity — full restore is a blunt instrument
- Separate staging environment would be ideal but adds complexity

## Recommendations adopted
1. Decompose at classification time — write DRRP not Obligation/Liberty ✓
2. Revert the 1,881 agentic provisions to DRRP ✓ (with backup)
3. Add explicit rule for mixed-actor decomposition
4. Confirm regex confidence cap
