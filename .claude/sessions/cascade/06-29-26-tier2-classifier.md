# Session: Tier 2 — Position Classifier + Dep Features (PENDING)

## Problem

The v3 position classifier requires dep features (spaCy batch job) + embeddings. Two bugs: (1) classify silently skips actors when dep features are missing (no QA gate); (2) classify reads actors from legislation_text.actors JSONB instead of provision_actors, causing 50,837 actors with embeddings to get no classification.

## Work

1. ⬜ Fix classify to read actors from provision_actors (not legislation_text.actors JSONB)
2. ⬜ Add QA gate: log warning + count when actors skipped for feature mismatch
3. ⬜ Verify dep features computed for all in-scope actors
4. ⬜ Define descriptive stats for this tier:
   - Actors with dep features / total in-scope actors
   - Actors with cls_position / actors with dep features + embedding
   - Classifier coverage should be 100% of actors that have both embedding and dep features
5. ⬜ Run on QQ corpus — dep features + classify
6. ⬜ Passing descriptive stats = session close signal
7. ⬜ Update corpus-stats skill with Tier 2 checks

## QA checks (close signal)

- count(actors with embedding AND dep features BUT no cls_position) = 0
- count(dep_is_subject IS NULL WHERE regex_position IS NOT NULL AND in-scope) = 0
- Classifier coverage = 100% of eligible actors (not 42%)

## Depends on

- 06-29-26-tier1-regex
