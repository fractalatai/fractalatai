# Session: Reconciliation Engine (PENDING)

## Context

The `provision_actors` table now has per-tier signal columns (regex_drrp, regex_position, cls_drrp, cls_position). Each tier writes independently. Reconciliation reads all signals and writes the final answer.

## Work

1. Build `taxa reconcile` command reading from provision_actors (not legislation_text JSONB)
2. Reconciliation rules:
   - LLM present → LLM wins
   - Regex + classifier agree → confirmed, high confidence
   - Disagree → use regex as interim, flag `extraction_method = "pending_llm"`
   - Classifier confidence < 0.7 → don't trust, use regex
   - Only regex → use regex
3. Write reconciled `drrp`, `position`, `extraction_method` to provision_actors
4. Backfill `legislation_text.drrp_types` / `actors` from provision_actors for sertantai compat
5. Wire LLM elevation: when reconcile flags `pending_llm`, `taxa validate` targets those actors
6. Re-run reconcile after LLM to incorporate results

## Carried from position classifier session
- ⬜ 51K false-mentioned actors in non-benchmark corpus — need corpus-wide re-parse + re-classify to populate provision_actors with correct signals before reconciliation can run

## Dependencies

- provision_actors table populated (done)
- taxa parse writes regex_* (done)
- taxa classify writes cls_* (done)
- Classifier quality sufficient for meaningful agree/disagree signals (pending — see classifier training session)
