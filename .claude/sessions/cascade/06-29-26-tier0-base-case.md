# Session: Tier 0 — Base Case Definition (PENDING)

## Problem

The pipeline has no defined base case. Regex parses every provision including schedules, definitions, headings, and cross-references that will never progress past regex tier. This inflates actor counts and makes coverage stats meaningless — "42% classifier coverage" includes actors that should never have been created.

## Work

1. ⬜ Define which provisions are "in scope" — the filter criteria (section_type, text length, content signals)
2. ⬜ Implement the base case filter as a shared function used by embed, regex parse, and all downstream
3. ⬜ Define descriptive stats for this tier:
   - Total provisions vs in-scope provisions
   - Excluded by reason (heading, schedule, definition, cross-ref, short text)
   - Per-law breakdown
4. ⬜ Clean up existing out-of-scope actors from provision_actors
5. ⬜ Run on QQ corpus — verify base case is stable
6. ⬜ Passing descriptive stats = session close signal
7. ⬜ Create/update corpus-stats skill with Tier 0 checks

## QA checks (close signal)

- Every provision is tagged in-scope or out-of-scope
- No out-of-scope provision has actors in provision_actors
- In-scope count is stable across re-runs (idempotent)
