# Gemini Review: Decoupled Sync Watch Architecture

**Date:** 2026-06-09
**Model:** Gemini 2.5 Flash

## Summary

Endorses the decoupled architecture. The three-phase approach (ingest, enrich, publish) is robust and well-suited for the constraints.

## Key Feedback

### 1. Ack pattern — good pragmatic start
- Zenoh `put` at `fractalaw/@{tenant}/ack/{law_name}` is sufficient
- No granularity (no "embedding"/"classifying" stages) — fine for now
- Future evolution: `status` topic with JSON payloads if sertantai needs progress

### 2. Pending queue — DuckDB column is right
- Add `enrichment_pending` boolean to LRT table
- Also add `enrichment_added_at` (timestamp) and `enrichment_retry_count` (integer)
- Co-located with law metadata, durable, simple to query
- Avoid separate tables or file-based queues — unnecessary complexity

### 3. Batch trigger — start manual, automate later
- **Immediate**: manual/cron for `enrich --pending` and `publish --pending`
- **Later**: hybrid "threshold OR timer" — trigger when N laws queued OR quiet for 30-60s
- Timer via `tokio::time::sleep` in a background task checking the queue

### 4. Compaction — after every batch, with threshold
- Compact after every `enrich --pending` run
- Add heuristic: skip if fragment count < 10-20
- Monitor disk and compaction times, adjust as needed

### 5. Ack-then-publish-later — handle "processing" state in UI
- Real concern: sertantai shows incomplete data between ack and publish
- Sertantai UI should show "Enrichment Pending" badge, not incomplete DRRP as final
- The absence of a publish event = "still working on it"
- Display "Last Updated" timestamp to manage expectations

### 6. Missing items
1. **Error handling**: enrichment_retry_count + dead letter for persistent failures
2. **Observability**: pending_laws_count, enrichment_batch_duration, lancedb_fragment_count
3. **Concurrency**: start sequential, consider rayon for parallel within a batch later
4. **Idempotency**: ensure enrich --pending and publish --pending are safe to re-run
5. **Backfill**: one-off command for existing regex-only laws (already partially done)
6. **Zenoh auth**: verify only authorized instances can send acks (likely already handled)
