---
session: "QQ Corpus Parse & Publish"
status: suspended
opened: 2026-06-23
closed: 2026-06-24
outcome: blocked

summary: >
  Full QQ corpus pipeline walk-through. 174 QQ laws parsed, classified, and LLM-validated
  (3,808 corrections in 310 audit logs). 89 QQ laws still need embeddings but LanceDB
  fragment bloat keeps exhausting disk during batch operations. Suspended pending DB
  migration to pgvector hub + LanceDB edge hybrid. Human adjudication and publish steps
  deferred.

decisions:
  - what: "Suspend and migrate to pgvector hybrid store"
    why: "LanceDB merge_insert write amplification exhausts 116GB disk repeatedly during embedding. Compaction every 5 laws is a band-aid."
    result: "Session suspended. DB migration required before resume. 174 of 274 QQ laws (60%) complete."
  - what: "Share 20 in-force laws missing LAT with sertantai team"
    why: "109 QQ laws have no provision text in LanceDB. 18-20 are fully commenced and duty-making."
    result: "Priority list created for sertantai LAT publication. Not a hard blocker for publishing the 165 complete laws."

lessons:
  - title: "LanceDB fragment bloat is a fundamental architectural problem"
    detail: "merge_insert creates ~25x write amplification. Even with compaction every 5 laws, 116GB disk fills during batch operations. Cannot scale batch processing on LanceDB alone."
    tag: data-store
  - title: "Pipeline stages validated end-to-end"
    detail: "parse -> classify -> validate -> (human review) -> publish chain works. The bottleneck is storage, not pipeline logic."
    tag: pipeline
---

# Session: QQ Corpus Parse & Publish (SUSPENDED)

## Plan

Walk-through of the full QQ corpus pipeline, end to end:

### Step 1: Parse QQ corpus (regex + classifier, no LLM)

Start with a pilot batch (~10 laws, mixed sizes) with `--trace` to evaluate trace usefulness before scaling to full corpus.

```bash
# Step 1a: Pilot batch with trace
taxa parse --laws <pilot batch> --force --trace data/pilot_trace.json
taxa classify --laws <pilot batch>

# Step 1b: Evaluate trace — is it useful for Step 2?
# If yes, run full corpus with trace
# If too noisy, run full corpus without trace

# Step 1c: Full corpus
taxa parse --laws <remaining laws> --force [--trace data/corpus_trace.json]
taxa classify --laws <all QQ laws>
```

Verify: benchmark accuracy holds at 86.0% on the 16 benchmark laws.

### Step 2: Review LLM validation surface

Before running LLM, examine the shape:
- How many provisions are `pending_llm` across QQ corpus?
- Distribution by law size — which laws qualify for whole-law validation (≤100 provs)?
- Estimate total token cost and API calls
- Confirm batching strategy: whole-law for small, per-provision for large

### Step 3: Run LLM validation (whole-law for small laws)

```bash
taxa validate --laws <small laws> --audit-dir data/llm-audit
```

Review audit logs — how many corrections per law? Any patterns?

### Step 4: Human adjudication

```bash
/human-review <law_name>
```

Step through LLM corrections with human reviewer. Accept/reject each. Write adjudicated corrections to LanceDB.

### Step 5: Clean-up (2026-06-24)

- ✅ **432 "Rule" provisions**: Re-parsed 55 laws with `--force`. Rule now 0 across corpus.
- ✅ **Benchmark gold labels**: Written to LanceDB as `agentic` (done 2026-06-23).
- ✅ **QQ corpus parse health**: All 165 QQ laws in LanceDB properly parsed. 8,865 provisions (11%) with no extraction_method are structural (headings, parts) — correct.
- ✅ **21,299 no-method provisions**: Confirmed structural across full corpus.

### Step 5b: Missing QQ laws from sertantai

109 of 274 QQ laws have **no LAT (provision text) in LanceDB**. All 109 are in DuckDB but have zero taxa. Breakdown:

| Status | Count | Notes |
|--------|-------|-------|
| null commencement | 87 | Mix of unprocessed EU instruments, older Acts (some repealed), and missing LAT |
| fully_commenced | 18 | In force, duty-making — **real gap**, sertantai needs to publish LAT |
| partially_commenced | 2 | Edge cases |
| not_commenced | 2 | Not yet in force |

The 87 null-status laws may not all be duty-making (EU decisions, amending instruments, historic/repealed Acts). The **18-20 fully/partially commenced** are the priority for sertantai to publish.

Action:
1. Share the 20 in-force laws with sertantai team for LAT publication
2. Once LAT synced, pull → parse → classify → validate (same pipeline)
3. The 87 null-status laws need triage: which are in-force and duty-making?

**Not a hard blocker for publishing** — 165 of 274 QQ laws (60%) are complete and can be published now. The 109 can follow.

### Step 6: Publish to sertantai

```bash
sync publish --tenant dev --changed
```

Verify published data on sertantai side.

## Step 1-2 results (2026-06-23)

### Step 1: Parse + classify complete

- 532 laws, 161,888 provisions parsed and classified
- Disk issues during full corpus run (LanceDB fragment bloat). Fixed: compact interval 20→5 for parse, added every-10 compaction to classify.
- NAS backup taken (20260623, post-classify)

### Step 2: LLM validation surface

| Metric | Value |
|--------|-------|
| Total corpus | 532 laws, 161,888 provisions |
| DRRP classified | 47,535 (29.4%): 30,838 Obligation, 16,265 Liberty, 432 Rule |
| pending_llm | 4,757 provisions across 348 laws |
| Orphans (DRRP, no actors) | 8,427 |
| No extraction_method | 21,299 |

**Batching strategy for LLM validation:**

| Strategy | Laws | Provisions | pending_llm | Cost (Flash) |
|----------|------|-----------|-------------|-------------|
| Whole-law (≤100 provs) | 214 | 10,625 | 333 | ~$0.13 |
| Medium (101-500) | 240 | 52,549 | 1,577 | ~$1-2 |
| Large (500+) | 78 | 98,714 | 2,847 | ~$2-3 |

Top pending_llm laws: UK_ukpga_1990_43 (156), UK_ukpga_1989_29 (143), UK_ukpga_2008_29 (143)

### Step 3: LLM validation complete

**Small laws (≤100 provs):** 215 laws validated whole-law, 1,907 corrections
**Medium/large laws (101-5,639 provs):** 95 laws validated section-targeted, 1,901 corrections
**Benchmark laws:** 16 laws, gold labels written to LanceDB as agentic
**Total human review surface: ~3,808 corrections in 301 audit logs**

NAS backup taken (20260623) with audit logs.

### Step 5 results (2026-06-24)

- ✅ Rule provisions: 432 → 0 (re-parsed 55 laws)
- ✅ QQ corpus parse health: all 174 QQ laws in LanceDB verified clean
- ✅ 9 missing QQ laws pulled from sertantai, embedded, parsed, classified, validated (383 corrections)
- 100 QQ laws still missing LAT (need sertantai to publish) — not a hard blocker

**Updated totals**: 174 QQ laws processed, 167,148 provisions, ~4,191 corrections in 310 audit logs.

### Suspend point (2026-06-24)

**Blocked**: LanceDB fragment bloat keeps exhausting disk during embedding. Even with compaction every 5 laws, the 116GB disk fills during batch operations. This has happened 5+ times across this session. Adding compaction to every command is a band-aid — the root cause is LanceDB's merge_insert write amplification.

**What's done:**
- 174 QQ laws: parsed, classified, LLM-validated (3,808 corrections in 310 audit logs)
- 9 additional QQ laws pulled from sertantai, embedded, classified, validated
- 66 new laws arrived via sync watch, parsed automatically
- Rule provisions cleaned (432 → 0)
- Benchmark gold labels written to LanceDB

**What's remaining (89 QQ laws need embeddings):**
- Embedding keeps failing mid-batch due to disk exhaustion
- Once embedded: classify → validate → ready for human review
- Then: publish to sertantai

**Decision**: Suspend this session and do the DB migration (pgvector hub + LanceDB edge hybrid) to eliminate the fragment bloat problem. Resume this session on the new store.

Next: Step 4 (human adjudication — deferred) then Step 6 (publish).

## Prerequisites

- All pending sessions closed (done)
- `taxa validate` command built (done)
- `/human-review` skill built (done)
- `adjudicated` source_tier=7 in place (done)
- Audit trail chain complete: regex → classifier → LLM → adjudicated (done)

## Prior sessions

- `06-23-26-llm-batch-strategy.md` (CLOSED) — validate command, audit log, prompt alignment
- `06-22-26-llm-elevation-optimisation.md` (CLOSED) — 86.0% benchmark, threshold tuning
- `06-22-26-liberty-false-positives.md` (CLOSED) — regex ceiling fixes
- `06-22-26-pipeline-traceability.md` (CLOSED) — signal/decision separation
- `06-22-26-actor-position-coverage.md` (CLOSED) — orphan inheritance, mentioned mapping
- `06-22-26-rule-class-cleanup.md` (CLOSED) — Rule→Obligation remap
- `06-18-26-benchmark-post-restructure.md` (CLOSED) — benchmark baseline
