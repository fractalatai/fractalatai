---
session: QQ Corpus Completion
status: pending
opened: 2026-07-04
---

# Session: QQ Corpus Completion (PENDING)

## Problem

174 of 274 QQ laws are parsed, classified, and LLM-validated. The remaining work was blocked by LanceDB disk exhaustion — now resolved (PgStore hub + SSD). Three gaps remain:

1. **~100 QQ laws missing LAT** — sertantai hasn't published provision text for these. Need to check which now have LAT available, pull, and enrich.
2. **89 QQ laws need embeddings** — trivial now on pgvector (no fragment bloat).
3. **Human adjudication** — 3,808 LLM corrections across 310 audit logs need sampled review.
4. **Publish** — enriched QQ laws to sertantai via Zenoh.

## Work

1. ⬜ Check which of the ~100 missing-LAT QQ laws now have LAT on sertantai
2. ⬜ Pull LAT for newly available laws → embed → parse → classify
3. ⬜ Embed the 89 QQ laws that failed previously (pgvector, batch)
4. ⬜ Classify + validate any newly-embedded laws
5. ⬜ Sampled human adjudication of LLM corrections (audit logs in data/audit/)
6. ⬜ Publish completed QQ laws to sertantai (`sync publish --tenant dev --family QQ`)

## Dependencies

- ✅ PgStore hub (pgvector embeddings) — operational
- ✅ SSD installed — disk pressure eliminated
- ✅ Triage gate in sync watch — new arrivals classified automatically
- ✅ LLM validation command (`taxa validate`) — working
- ✅ Human review skill (`/human-review`) — working
- ⬜ Sertantai LAT availability for missing QQ laws (external)

## Context

Lifted from `06-23-26-qq-corpus-parse-publish.md` (closed as partial). That session completed:
- 174 QQ laws: parsed, classified, LLM-validated
- 3,808 corrections in 310 audit logs
- Rule provisions cleaned (432 → 0)
- Benchmark gold labels written

The architecture has changed since: LanceDB → PgStore for hub, SSD eliminates disk pressure, triage gates enrichment for new arrivals.
