# Session: QQ Corpus Parse & Publish (PENDING)

## Plan

Walk-through of the full QQ corpus pipeline, end to end:

### Step 1: Parse full QQ corpus (regex + classifier, no LLM)

```bash
taxa parse --laws <all QQ laws> --force
taxa classify --laws <all QQ laws>
```

Verify: benchmark accuracy holds at 86.0% on the 16 benchmark laws after full corpus parse.

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

### Step 5: Publish to sertantai

```bash
sync publish --tenant dev --changed
```

Verify published data on sertantai side.

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
