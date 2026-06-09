# Session: Sync Watch Enrichment — Decoupled Ingest + Batch Enrich

## Context

**Prior session**: `.claude/sessions/cascade/06-09-26-actor-labels.md`
**Trigger**: `sync watch` processes new laws with regex-only DRRP. No embeddings, no classifier, no LLM. Need to close the gap.

## Current sync watch pipeline

```
sertantai event → ensure LRT → pull LAT → regex enrich (gap_c=false) → publish
```

Provisions arrive with null embeddings. The classifier (86.4% accuracy, Obligation/Liberty/none) never runs. Only regex DRRP extraction fires.

## Problem

Regex alone gives ~50-60% DRRP coverage. The classifier raises this to ~85%+. But the classifier needs 384-dim MiniLM embeddings + 13 modal features. Embeddings are currently computed separately via `fractalaw embed` (batch, CPU, ~43 emb/s).

## Option 1: Embed Inline

Compute embeddings as provisions arrive in `sync watch`, immediately after LAT upsert.

### Pipeline

```
sertantai event → ensure LRT → pull LAT → compute embeddings → regex enrich → publish
```

### Implementation

The ONNX embedding model is already loaded in `fractalaw-ai` (`all-MiniLM-L6-v2`, 384-dim). The `embed` command uses it via `OnnxEmbedder`. Wire it into the watch loop:

1. After `lance.upsert_lat(batches)`, query back the new provisions with null embeddings
2. Compute embeddings in batches (batch size 32-64)
3. Write embeddings back via `merge_insert`

### Performance

- CPU: ~43 embeddings/s → 500-provision law takes ~12s
- GPU (RTX 3090, future): ~4000 emb/s → 500 provisions in <1s
- Acceptable for sync watch — events arrive minutes apart, not seconds

### Pros

- Embeddings always available for downstream (classifier, similarity search, clustering)
- Simple extension point — once embedded, any model can consume them
- No separate batch job needed for new laws

### Cons

- Adds 5-30s per law on CPU (depends on provision count)
- ONNX model stays loaded in memory (~100MB) for the lifetime of the watch session
- LanceDB write amplification from merge_insert (mitigated by compaction)

## Option 2: Embed + Classify Inline

Compute embeddings AND run the classifier as provisions arrive. Full production pipeline in watch mode.

### Pipeline

```
sertantai event → ensure LRT → pull LAT → compute embeddings → regex enrich → classify → publish
```

### Implementation

Everything from Option 1, plus:

4. After embeddings are computed, load the classifier model (`drrp_classifier_v6.pkl`)
5. For each provision: build feature vector (384-dim embedding + 13 modal indicators)
6. Predict Obligation/Liberty/none
7. Decompose to DRRP types using active actor prefix (Gvt → Responsibility/Power, Org → Duty/Right)
8. Write back to LanceDB with `extraction_method = "classifier"`, `taxa_confidence = 0.85`
9. Respect confidence protection — don't overwrite agentic (0.90) or QA-corrected data

### Classifier in Rust vs Python

The classifier is currently Python (`classify.py` — scikit-learn LogisticRegression). Two paths:

**A. Call Python from Rust** (pragmatic):
- Shell out to `classify.py --law UK_uksi_...` after embedding
- Python loads pickle, runs inference, writes to LanceDB
- Pro: reuses existing tested code, no porting
- Con: process spawn overhead, Python startup (~1-2s), two LanceDB writers

**B. Port classifier to Rust** (clean):
- LogisticRegression is just `sigmoid(X @ W + b)` — trivial to implement
- Export weights from pickle as JSON/binary, load in Rust
- Embed + classify in same process, single LanceDB writer
- Pro: fast, no Python dependency, single process
- Con: upfront work to export weights and reimplement modal feature extraction

**C. ONNX export** (middle ground):
- Export scikit-learn model to ONNX via `skl2onnx`
- Run alongside the embedding model in `ort` (already a dependency)
- Pro: reuses ONNX runtime, no Python at runtime
- Con: need to build the feature vector in Rust (embedding concat + modal features)

### Performance

- CPU embedding: ~12s for 500 provisions
- Classifier inference: microseconds per provision (logistic regression)
- Total: ~12-15s per law on CPU (embedding dominates)

### Pros

- New laws get full DRRP classification immediately
- Published data is production quality from the start
- No manual batch step needed
- Sertantai sees classified provisions within seconds of the sync event

### Cons

- More complex watch loop (embedding + classification + write-back)
- Need to handle confidence protection correctly in the watch context
- Classifier model needs to be loadable from Rust (one of the three paths above)

## Recommendation

Option 2C (embed + classify via ONNX) is the cleanest:
- Single Rust process, single ONNX runtime instance
- Embedding model + classifier model both loaded as ONNX sessions
- Modal features extracted in Rust (already done in `taxa/mod.rs` — `has_shall`, `has_must`, etc.)
- No Python dependency at runtime
- ~12-15s per law on CPU, sub-second on GPU

## Gemini Review (2026-06-09)

Full review: `docs/reviews/gemini-sync-watch-enrichment-review-20260609.md`

**Verdict**: Option 2C (Embed + Classify via ONNX). Key feedback:

1. **Option 2 over 1** — embedding alone leaves the coverage gap open, classification adds minimal complexity
2. **2C over 2A/2B** — reuses existing `ort`, no Python runtime, single process, `skl2onnx` automates export
3. **skl2onnx reliable** for LogisticRegression — watch feature order, export entire pipeline if StandardScaler used
4. **Always embed, skip classification only** — embeddings valuable for downstream; confidence protection at classifier step
5. **Single merge_insert** for embedding + taxa — atomic, less write amplification
6. **Long-running ONNX is fine** — monitor RSS, keep `ort` updated
7. **Missing**: error handling/retries, backfill for existing regex-only laws, observability metrics

## Architectural Pivot: Decouple Ingestion from Enrichment

Gemini's review assumed inline processing. User feedback surfaced a critical issue: **the sync watch loop is sequential, and sertantai dispatches batches of laws every few seconds.** Inline embedding (~15s/law) would create unacceptable backpressure — 50 laws would take ~12 minutes, blocking the event loop.

### The Real Problem

The current watch loop tightly couples ingestion with enrichment with publish. This worked for regex-only (fast) but breaks for embedding + classification (slow).

### Disk Constraints

- Embeddings: 384 × f32 = 1.5 KB/provision → ~240 MB for 161K provisions
- LanceDB merge_insert creates ~25x write amplification
- Compaction essential after batch writes (`compact_lance.py`)
- Disk is 116 GB, routinely tight — can't let fragments bloat unchecked

### New Architecture: Three Phases

```
Phase 1 — Ingest (fast, per-event, ~2s/law):
  sync event → pull LAT → store in LanceDB → ack to sertantai → next event

Phase 2 — Enrich (batched, async, at fractalaw's pace):
  embed batch → classify batch → regex DRRP → compact LanceDB

Phase 3 — Publish (batched):
  publish provisions + dictionary → sertantai subscriber picks up results
```

**Key principle**: fractalaw sends a lightweight ack ("got it, I have the LAT") immediately. How and when fractalaw enriches is its own concern. Sertantai knows the job is accepted and will receive results eventually.

### Phase 1: Ingest + Ack

The watch loop stays fast:
1. Receive sync event
2. Ensure LRT in DuckDB
3. Pull LAT → LanceDB (`upsert_lat`)
4. Publish ack (zenoh put: `fractalaw/@{tenant}/ack/{law_name}` with JSON status payload)
5. Mark law as pending: set `enrichment_pending = true`, `enrichment_added_at = NOW()` in DuckDB

No regex, no embedding, no classification. Just store the text and acknowledge.

**Ack payload** (JSON, for future evolution to richer status):
```json
{"law_name": "UK_uksi_2003_164", "state": "ingested", "provisions": 84}
```

Sertantai can use this to show an "Enrichment Pending" badge in the UI rather than displaying incomplete DRRP as final data.

### Phase 2: Enrich Batch

Triggered initially via **manual CLI or cron**. Automate later with threshold/timer.

```
fractalaw taxa enrich --pending        # process the queue
```

Processing:
1. Query DuckDB for laws with `enrichment_pending = true`, ordered by `enrichment_added_at`
2. For each law:
   a. Compute embeddings (ONNX, batch_size=64, ~43 emb/s CPU)
   b. Run classifier (ONNX, 397-dim input → Obligation/Liberty/none)
   c. Run regex DRRP enrichment
   d. Single merge_insert: embedding + drrp_types + extraction_method + taxa_confidence
   e. On success: clear `enrichment_pending`, update DuckDB LRT
   f. On failure: increment `enrichment_retry_count`, log error, continue to next law
3. After batch: compact LanceDB if fragment count > 10
4. Laws that fail 3+ times stay pending but are skipped (dead letter) — logged for manual review

**Idempotency**: safe to re-run. Confidence protection ensures already-classified provisions aren't downgraded. Embeddings overwrite nulls or identical values. merge_insert is upsert-safe.

### Phase 3: Publish Batch

```
fractalaw sync publish --pending --provisions --tenant dev --connect tcp/localhost:7447
```

After enrichment completes:
1. Query DuckDB for laws enriched since last publish (`enrichment_pending = false AND provisions_published_at < updated_at`)
2. Batch publish provisions + dictionary (fires sertantai subscriber)
3. Mark as published (`provisions_published_at = NOW()`)

**Sertantai UI**: when provisions arrive, the "Enrichment Pending" badge clears and full DRRP data is displayed.

### Coexistence with Development Flow

Two enrichment paths must coexist:

| | Development | Production |
|---|---|---|
| **Command** | `taxa enrich --gap-c --laws UK_...` | `taxa enrich --pending` |
| **Scope** | Named laws, manual | Pending queue, batch |
| **Pipeline** | Regex → Tier 1 → Tier 2 (Gemini) → Tier 3 (Gemini) | Embed → Classify (ONNX) → Regex |
| **Confidence** | 0.90 (agentic) | 0.85 (classifier) |
| **Embeddings** | Not computed | Always computed |
| **Actors** | Unconstrained LLM + dictionary matcher | Regex-only (from text patterns) |
| **When** | QA, iteration, new dictionary entries | Automated batch after ingestion |

**Rules to prevent conflicts:**

1. **`--gap-c --laws` clears `enrichment_pending`** for named laws. Dev-enriched laws don't need the production pipeline.

2. **`--pending` always embeds, even for dev-enriched laws.** Embeddings are useful for similarity search and future models regardless of how DRRP was classified. Single merge_insert writes the embedding without touching taxa columns if confidence is already >= 0.85.

3. **Confidence protection is the arbiter.** Both paths use the same mechanism:
   - Production classifier at 0.85 never overwrites agentic at 0.90
   - Dev re-enrichment at 0.90 overwrites classifier at 0.85
   - QA corrections at 0.90 persist through both paths

4. **`extraction_method` tracks provenance.** `regex`, `inherited`, `classifier`, `agentic`, `agentic_unvalidated` — each path writes its own method. Downstream consumers (sertantai, QA) can filter by method.

5. **`--pending` skips laws with no pending flag.** Dev-enriched laws (no `enrichment_pending`) are invisible to the production queue unless explicitly marked.

6. **Publish works for both.** `sync publish --pending` publishes recently enriched laws regardless of which path enriched them. `sync publish --laws/--family` continues to work for manual publishes.

### Why This Is Better

| Aspect | Inline (rejected) | Decoupled (proposed) |
|--------|-------------------|---------------------|
| Sertantai latency | ~15s/law, 12min for 50 laws | ~2s/law ack, results arrive later |
| Disk | Fragment bloat per-law | Batch + compact, controlled |
| Compaction | After every law (expensive) | Once per batch (efficient) |
| Failure recovery | Crash mid-law loses everything | LAT stored, re-enrich from queue |
| Publish | Per-law (chatty) | Batch (proven with QQ, 76K provisions) |

## Refined Plan: Option 2C

### Step 1: DuckDB schema — pending queue columns

Add to `legislation` table:
- `enrichment_pending` (BOOLEAN, default false)
- `enrichment_added_at` (TIMESTAMP, nullable)
- `enrichment_retry_count` (INTEGER, default 0)

Add `ensure_enrichment_queue_columns()` in `fractalaw-store/src/duck.rs`, called from watch startup.

### Step 2: Simplify sync watch to ingest-only + ack

Strip enrichment and publish out of the watch loop:
1. Pull LRT if needed
2. Pull LAT → LanceDB
3. Ack to sertantai (zenoh put with JSON: `{"law_name": "...", "state": "ingested", "provisions": N}`)
4. Set `enrichment_pending = true`, `enrichment_added_at = NOW()` in DuckDB
5. Continue to next event

Add `keys::ack(tenant, law_name)` → `fractalaw/@{tenant}/ack/{law_name}` and `publish_ack()` to zenoh_sync.rs.

### Step 3: Export classifier to ONNX

```python
import pickle, skl2onnx
from skl2onnx.common.data_types import FloatTensorType

model = pickle.load(open('data/drrp_classifier_v6.pkl', 'rb'))
initial_type = [('features', FloatTensorType([None, 397]))]
onnx_model = skl2onnx.convert_sklearn(model, initial_types=initial_type)
with open('models/drrp_classifier_v6.onnx', 'wb') as f:
    f.write(onnx_model.SerializeToString())
```

Validate: compare predictions on test set between pickle and ONNX. Feature order must be: 384 embedding dims then 13 modal features.

### Step 4: Build `taxa enrich --pending` command

New enrichment mode that processes the pending queue:
1. Query DuckDB: `WHERE enrichment_pending = true AND enrichment_retry_count < 3 ORDER BY enrichment_added_at`
2. For each law:
   a. **Always** compute embeddings for provisions with null embeddings (ONNX, batch_size=64)
   b. Read existing `taxa_confidence` per provision
   c. For provisions with `taxa_confidence < 0.85` (not already dev-enriched or QA-corrected):
      - Build 397-dim feature vectors (384 embedding + 13 modal indicators)
      - Run classifier (ONNX → Obligation/Liberty/none)
      - Run regex DRRP enrichment
   d. Single merge_insert per law: embedding (always) + taxa columns (only where confidence allows)
   e. On success: `enrichment_pending = false`, update DuckDB LRT
   f. On failure: `enrichment_retry_count += 1`, log error, continue to next law
3. After batch: compact LanceDB if fragment count > 10
4. Laws with retry_count >= 3 are dead-lettered — logged for manual review

**Idempotency**: safe to re-run. Confidence protection prevents downgrades. merge_insert is upsert-safe.

### Step 4b: Update `taxa enrich --gap-c --laws` (dev flow)

Ensure the existing dev enrichment path:
1. Clears `enrichment_pending = false` for named laws (so `--pending` skips them)
2. Does NOT clear embeddings (if present, leave them)
3. Writes at confidence 0.90 (agentic) as today — classifier can't overwrite later

### Step 5: Modal feature extraction in Rust

The 13 modal features already detected in `taxa/mod.rs`:
- `has_shall`, `has_must`, `has_may`, `has_require`, `has_ensure`
- `has_prohibit`, `has_duty`, `has_right`, `has_power`, `has_responsible`
- `has_penalty`, `has_offence`, `has_exempt`

Concatenate with embedding → 397-dim feature vector for classifier.

### Step 6: Build `sync publish --pending` command

Publish all newly enriched laws:
1. Query DuckDB: `WHERE enrichment_pending = false AND (provisions_published_at IS NULL OR provisions_published_at < updated_at)`
2. Batch publish provisions + dictionary (fires sertantai subscriber)
3. Mark as published (`provisions_published_at = NOW()`)

### Step 7: Observability

Add logging/metrics to `enrich --pending`:
- Count of pending laws at start
- Per-law: embedding time, classification time, merge_insert time
- LanceDB fragment count before/after compact
- Dead-lettered laws (retry_count >= 3)
- Total batch duration

### Step 8 (future): Auto-trigger in watch

Deferred — start with manual/cron. When ready, add to the watch loop:
- Background tokio task checks queue every 60s
- If pending laws > 0 and last event was > 60s ago, trigger enrich + publish
- Or: threshold trigger when pending count > N

### Files to modify

| File | Change |
|------|--------|
| `crates/fractalaw-store/src/duck.rs` | `ensure_enrichment_queue_columns()` — 3 new columns |
| `crates/fractalaw-cli/src/main.rs` | Simplify watch loop (ingest+ack), add `--pending` enrich/publish modes |
| `crates/fractalaw-ai/src/lib.rs` | Expose batch embedding API for enrich command |
| `crates/fractalaw-sync/src/zenoh_sync.rs` | `keys::ack()`, `publish_ack()` with JSON payload |
| `models/drrp_classifier_v6.onnx` | Exported classifier model (gitignored) |
| `scripts/export_classifier_onnx.py` | One-shot export script |

### Verification

1. Export classifier to ONNX, validate predictions match pickle on test set
2. Add queue columns, verify `ensure_enrichment_queue_columns()` is idempotent
3. Simulate: send events to watch, verify LAT stored + ack sent + law marked pending
4. Run `taxa enrich --pending` — verify embed + classify + compact + queue cleared
5. Run `sync publish --provisions --pending` — verify sertantai receives batch
6. Failure test: kill mid-batch, re-run — verify idempotent recovery
7. Monitor disk: LanceDB size before/after embed, after compact
8. Monitor memory: watch process RSS over 1-hour session with ONNX loaded

## Progress

### Committed: `a10fe8e` — Decoupled sync watch infrastructure

**Steps completed:**
- [x] Step 1: DuckDB queue columns (`ensure_enrichment_queue_columns`)
- [x] Step 2: Simplified watch loop (ingest + ack only, no enrich/publish)
- [x] Step 4b: Dev flow (`--gap-c --laws`) clears `enrichment_pending`
- [x] Step 6: `sync publish --provisions --pending` flag
- [x] Zenoh ack key + `publish_ack()` with JSON payload
- [x] `taxa enrich --pending` flag (queries queue, dead-letters at 3 retries)
- [x] Resilient error handling (retry_count on failure, continues to next law)
- [x] 19/19 tests passing, fmt + clippy clean

**Steps remaining:**
- [ ] Step 3: Export classifier to ONNX (`skl2onnx`)
- [ ] Step 5: Modal feature extraction in Rust (397-dim vector)
- [ ] Step 7: Observability logging (pending count, per-law timings, fragment count)
- [ ] Wire ONNX classifier into `enrich --pending` (embed + classify in Rust)
- [ ] Step 8 (future): Auto-trigger in watch (timer/threshold)

**What works now:**
- `sync watch` ingests LAT + acks in ~2s/law (no blocking)
- `taxa enrich --pending` processes the queue (regex-only for now, ONNX classifier next)
- `sync publish --provisions --pending` publishes enriched laws as a batch
- Dev flow (`--gap-c --laws`) coexists — clears pending flag, confidence protection prevents downgrades
