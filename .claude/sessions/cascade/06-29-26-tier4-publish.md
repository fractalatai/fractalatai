# Session: Tier 4 — Backfill & Publish (PENDING)

## Problem

After all classification tiers are complete, provision_actors holds the reconciled truth but legislation_text (which sertantai reads) is stale. Backfill aggregates provision_actors → legislation_text, then publish sends to sertantai.

## Work

1. ⬜ Run backfill on QQ corpus
2. ⬜ Define descriptive stats for this tier:
   - Provisions backfilled / provisions with reconciled actors
   - legislation_text.actors populated / total in-scope provisions
   - extraction_method distribution in legislation_text
   - Laws published / laws backfilled
3. ⬜ Publish provisions to sertantai
4. ⬜ Verify sertantai received data (spot check)
5. ⬜ Passing descriptive stats = session close signal
6. ⬜ Update corpus-stats skill with Tier 4 checks

## QA checks (close signal)

- count(provisions with reconciled actors in provision_actors BUT no actors in legislation_text) = 0
- All published laws have provisions_published_at set
- Sertantai confirms receipt

## Depends on

- 06-29-26-tier3-slm
