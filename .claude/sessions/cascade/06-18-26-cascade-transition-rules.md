# Session: Cascade Transition Rules — Codify in Code (CLOSED)

## Outcome

Pipeline restructured from tangled two-pass architecture to loosely coupled cascade: `taxa parse → taxa classify → taxa escalate`. Each tier runs independently via CLI subcommands. drrp_history records provenance. Disagreements flagged for LLM. Cryptic flags renamed.

**Stage 1**: DONE (`7de07d8`) — drrp_history schema + migration
**Stage 2**: DONE (`010cc69`) — extract 4 functions + CLI subcommands
**Stage 3**: DONE (`fcd0fc4`, `c8c49a6`) — cascade wiring + disagreement detection. Deferred: #42, #43, #44
**Stage 4**: DONE (`3d52f29`) — `--gap-c` → `--escalate`, `TIER2_PROVIDER` → `LLM_PROVIDER`
**Stage 5**: DONE (`53f4a5b`) — error handling (embed/classify failures don't stop enrichment)

### Stage 1 completion checklist

| Requirement | Status | Commit |
|-------------|--------|--------|
| `drrp_history` List<Struct> schema in schema.rs | ✅ | `7de07d8` |
| Field count test updated (58→59) | ✅ | `7de07d8` |
| Migration script (scripts/migrate_drrp_history.py) | ✅ | `7de07d8` |
| 134,293 provisions populated from existing data | ✅ | data migration |
| Warning in ensure_gap_c_columns() if column missing | ✅ | `7de07d8` |
| Write path in enrich_single_law (regex tier writes to drrp_history) | ✅ | `7de07d8` |
| Arrow IPC backup before rebuild | ✅ | backups/ |
| Workspace compiles, 476 tests pass | ✅ | `7de07d8` |

### Stage 2 completion checklist

| Requirement | Status | Commit |
|-------------|--------|--------|
| `cmd_taxa_parse` — regex parse + inheritance + write | ✅ | `010cc69` |
| `cmd_taxa_embed` — compute embeddings for missing provisions | ✅ | `010cc69` |
| `cmd_taxa_classify` — DRRP + position classifiers | ✅ | `010cc69` |
| `cmd_taxa_escalate` — LLM classification (Ollama/Gemini) | ✅ | `010cc69` |
| `cmd_taxa_enrich` rewired as orchestrator (parse→embed→classify→escalate) | ✅ | `010cc69` |
| CLI subcommands (taxa parse, taxa embed, taxa classify, taxa escalate) | ✅ | `010cc69` |
| Each function can run independently via CLI | ✅ | `010cc69` |
| `--force`, `--pending`, `--gap-c` backward compatible | ✅ | `010cc69` |
| LLM code extracted from enrich_single_law into cmd_taxa_escalate | ✅ | `010cc69` |
| Workspace compiles, 476 tests pass | ✅ | `010cc69` |

### Stage 3 completion checklist

| Requirement | Status | Commit |
|-------------|--------|--------|
| Classifier always records prediction in drrp_history | ✅ | `fcd0fc4` |
| drrp_history written via merge_insert (List<Struct> with tier/drrp/confidence/timestamp) | ✅ | `fcd0fc4` |
| Both-modals detection (obligation + enabling in same provision) | ✅ | `fcd0fc4` |
| Disagreements flagged as `extraction_method = "pending_llm"` | ✅ | `fcd0fc4` |
| Gap fills (regex=none, classifier confident) still work | ✅ | `fcd0fc4` |
| Disagreement count logged to stderr | ✅ | `fcd0fc4` |
| No silent overrides — disagreements held for LLM | ✅ | `fcd0fc4` |
| Workspace compiles, 476 tests pass | ✅ | `fcd0fc4` |

### Stage 3 remaining gaps

| # | Gap | Status | Ref |
|---|-----|--------|-----|
| 1 | `taxa escalate` doesn't consume `pending_llm` flags | ✅ FIXED | `c8c49a6` |
| 2 | `taxa escalate` doesn't write to drrp_history | KNOWN LIMITATION — single entry per write pass, last writer wins | #42 |
| 3 | Low-confidence classifier on regex=none not flagged for LLM | ✅ FIXED — flagged as `pending_llm` | `c8c49a6` |
| 4 | No actor + modal provisions not detected as LLM candidates | DEFERRED — needs actor access in classifier pass | #43 |
| 5 | Resolution rules not codified via drrp_history | DEFERRED — consumer-side concern | #44 |
| 6 | drrp_history doesn't clear own tier on re-run | KNOWN LIMITATION — latest timestamp per tier is authoritative | #42 |

### Stage 4 completion checklist

| Requirement | Status | Commit |
|-------------|--------|--------|
| `--gap-c` renamed to `--escalate` | ✅ | `3d52f29` |
| CLI help text updated for `--escalate` | ✅ | `3d52f29` |
| `TIER2_PROVIDER` renamed to `LLM_PROVIDER` | ✅ | `3d52f29` |
| "Gap C" comments updated to "Escalation" in main.rs | ✅ | `3d52f29` |
| `ensure_gap_c_columns()` function name kept (cross-crate, rename later) | noted | — |
| Workspace compiles, 476 tests pass | ✅ | `3d52f29` |

### Stage 5 completion checklist

| Requirement | Status | Commit |
|-------------|--------|--------|
| `cmd_taxa_embed` failure in orchestrator: log and continue | ✅ | `53f4a5b` |
| `cmd_taxa_classify` failure in orchestrator: log and continue | ✅ | `53f4a5b` |
| Per-law errors in parse: already handled (retry count) | ✅ | pre-existing |
| Per-law errors in escalate: already handled (retry count) | ✅ | pre-existing |
| Failed stages retryable independently via CLI subcommands | ✅ | Stage 2 |

## Context

**Prior sessions**: gold-standard-correction (CLOSED), offence-provision-gating (CLOSED)
**Trigger**: The cascade transition rules (regex → classifier → LLM) were documented in the gold correction session but never implemented in code. The pipeline makes ad-hoc decisions about when each tier runs.

## Definitions

### Enrichment pipeline stages

The pipeline runs in two passes within `cmd_taxa_enrich`:

**Pass 1: Per-law enrichment** (`enrich_single_law`, runs once per law)
- Step 1: **Regex parse** — `parse_v2()` on every provision. Extracts actors from YAML dictionary. Classifies DRRP via regex pattern matching (governed v2 → government v1/v2 → offence → rule). Assigns actor positions via span heuristic. Writes `extraction_method="regex"`.
- Step 2: **Tier 1 inheritance** — child provisions inherit DRRP from parent when they have no direct classification. Writes `extraction_method="inherited"`.
- Step 3: **LLM classification** (optional, requires `--gap-c` + `TIER2_PROVIDER` env var) — routes multi-actor and DRRP=none provisions to an LLM (Ollama or Gemini) for position + DRRP classification. Writes `extraction_method="local"` or `"agentic"`.
- Step 4: **Write to LanceDB + DuckDB** — per-provision taxa written to LanceDB, per-law aggregates to DuckDB.

**Pass 2: Embed + classify** (runs once after all laws in Pass 1, triggered by `--pending` or `--force`)
- Phase 1: **Embed** — compute embeddings for provisions without them (`--pending` only, skipped by `--force`).
- Phase 2: **Write embeddings** — merge_insert to LanceDB.
- Phase 3: **DRRP classifier** — logistic regression (v8) on provisions with embeddings. Predicts Obligation/Liberty/none. Writes `drrp_types` + `extraction_method="classifier"` for provisions where it overrides regex.
- Phase 4: **Position classifier** — per (provision, actor) pair. Predicts active/counterparty/other. Appends `| classifier:{position}@{confidence}` to each actor's reason field.

### Tiers (source hierarchy)

| Tier | extraction_method | source_tier() | What sets it |
|------|-------------------|---------------|-------------|
| 1 | regex | 1 | Pass 1 regex enrichment |
| 2 | inherited | 2 | Pass 1 Tier 1 parent inheritance |
| 3 | local | 3 | Local LLM (Ollama) |
| 4 | classifier | 4 | Pass 2 Phase 3 DRRP classifier |
| 5 | agentic_unvalidated | 5 | Gemini without QA validation |
| 6 | agentic | 6 | Gemini with QA validation (gold standard) |

Higher tier = higher quality. Source-tier protection prevents lower tiers from overwriting higher tiers.

### Cascade — desired vs actual

**Desired cascade** (what we keep saying):
```
regex → classifier → LLM
```
Each tier adds signal. Disagreements between tiers escalate to the next.

**Actual code order**:
```
Pass 1: regex → inheritance → LLM (optional)
Pass 2: classifier (optional)
```
The LLM runs BEFORE the classifier. They never see each other's results. The classifier can't flag disagreements for LLM review because the LLM already ran (or didn't). The desired cascade doesn't exist in the architecture.

### The architecture problem

The logical cascade (regex → classifier → LLM) requires:
1. Regex runs first (done)
2. Classifier sees regex output and adds signal (partially done — Phase 3/4)
3. Disagreements between regex and classifier escalate to LLM (NOT POSSIBLE — LLM runs before classifier)

**Options**:
- **A) Move classifier into Pass 1** — run the classifier per-law, after regex, before LLM. Requires the embedding to exist already (won't work for new laws without embeddings).
- **B) Multi-pass enrichment** — Pass 1 does regex + classifier. A separate pass sends disagreements to LLM. Two enrichment runs required.
- **C) Accept reverse order** — LLM runs on regex gaps first (`--gap-c`), classifier runs afterwards (`--force`/`--pending`). The classifier doesn't override LLM (source-tier protection: agentic=6 > classifier=4). Disagreements between regex and classifier are logged but not auto-escalated — they become candidates for a FUTURE LLM run.

Option C matches the current architecture. The "cascade" becomes:
```
regex → classifier → [log disagreements] → LLM (separate run, human-triggered)
```

## Current state (what the code does)

### Regex (always runs)
- `parse_v2()` runs on every provision
- Writes `drrp_types`, `actors` with `reason = "regex:{position}@{confidence}"`
- Purpose gate skips structural provisions (Enactment, Interpretation, Offence, Amendment, etc.)
- Legal fiction rejection suppresses "shall be treated/deemed" false positives

### Classifier (runs in `--force` and `--pending`)
- Phase 3: DRRP classifier (v8, Obligation/Liberty/none)
- Only runs on provisions where `tier < source_tier("classifier")`
- Confidence threshold: 0.7 for gaps (regex=none), 0.9 for overrides
- Writes `drrp_types` and `extraction_method = "classifier"`
- Phase 4: Position classifier — appends `| classifier:{position}@{confidence}` to reason

### LLM (runs in `--gap-c` with GEMINI_API_KEY)
- Only runs on inherited provisions and provisions with multi-actor or no DRRP
- Receives provision text + optional parent context
- Context gap: no sibling provisions (#38)

## Decision: Option B — Multi-pass pipeline restructure

Gemini review: `data/code-review/gemini-cascade-architecture-review.md`

**We go Option B.** The code doesn't match the mental model of `regex → classifier → LLM`. That mismatch is a blocker to improving the pipeline — every conversation about transition rules describes a cascade that doesn't exist. Fix the architecture so the code matches how we think about it.

### Design principles

1. **Correct cascade order**: regex → classifier → LLM. Each tier sees the output of the previous tier. — ✅ Stage 2 restructured the code into this order
2. **Loose coupling**: each tier (regex, classifier, LLM) must be able to run independently for testing and improvement. `taxa parse`, `taxa classify`, `taxa escalate` as separate subcommands that can run standalone or in sequence. — ✅ Stage 2 extracted separate functions + CLI subcommands
3. **Clear naming**: `gap-c`, `--pending`, Phase 3/4 etc. are cryptic. Use names that describe what they do. — **Stage 4** (not started). Old flags still exist alongside new subcommands.
4. **DRRP provenance at provision level**: a `drrp_history` field that records what each tier said, not just who won. — ✅ Stage 1 (schema + migration) + Stage 3 (classifier writes to it). **GAP**: `taxa parse` writes regex entry; `taxa classify` writes classifier entry; `taxa escalate` does NOT yet write LLM entry.
5. **Disagreement detection**: when regex and classifier disagree, the provision is flagged for LLM escalation. When both obligation and enabling modals are present (#41), flag for LLM. — ✅ Stage 3. **GAP**: `taxa escalate` doesn't consume the flags yet.
6. **No silent overrides**: every tier ADDS signal. The final `drrp_types` is determined by explicit resolution rules, not by whoever runs last. — **PARTIAL**: classifier gap-fills are written directly. Disagreements are flagged as `pending_llm`. But the resolution rules (highest-tier wins, higher-tier none overrides) are not codified — they're implicit in the threshold logic.

### Target architecture

```
taxa parse [--laws ...]        # Step 1: regex parse, actor extraction, DRRP classification
                                # Writes: drrp_types, actors with reason, extraction_method="regex"
                                # Can run independently for testing regex changes

taxa classify [--laws ...]     # Step 2: embedding classifier (requires embeddings to exist)
                                # Reads: provisions with embeddings
                                # Predicts: DRRP type + actor positions
                                # Writes: appends to drrp_history, updates reason provenance
                                # Flags disagreements with regex for escalation
                                # Can run independently for testing classifier changes

taxa escalate [--laws ...]     # Step 3: LLM on flagged provisions (requires GEMINI_API_KEY)
                                # Reads: provisions flagged by classifier (disagreements, ambiguous)
                                # Sends to LLM with full context (provision + siblings)
                                # Writes: final drrp_types, actors, extraction_method="agentic"
                                # Can run independently for testing LLM prompts

taxa enrich [--laws ...]       # Convenience: runs parse → classify → escalate in sequence
taxa enrich --pending          # For new laws via sync watch (includes embedding)
```

### Embedding generation

Embeddings are computed from provision TEXT, not from DRRP classifications. They should be computed once when text arrives (via sync or import) and never recomputed during classification. This decouples embedding from the classification cascade.

```
taxa embed [--laws ...]        # Standalone: compute embeddings for provisions without them
```

### DRRP provenance (Gemini recommendation)

Add a provision-level `drrp_history` field that records what each tier said:

```json
[
  {"tier": "regex", "drrp": "Obligation", "confidence": 0.80, "timestamp": "..."},
  {"tier": "classifier", "drrp": "Liberty", "confidence": 0.72, "timestamp": "..."},
  {"tier": "llm", "drrp": "Obligation", "confidence": 0.95, "timestamp": "..."}
]
```

**Resolution rules for `drrp_types`:**
- The final `drrp_types` is determined by the highest-tier entry — **NOT IMPLEMENTED**: current logic uses confidence thresholds, not drrp_history resolution
- A higher-tier `none` DOES override a lower-tier DRRP — **NOT IMPLEMENTED**: classifier skips when it predicts none
- When a tier re-runs, it clears and re-adds its OWN entry (latest timestamp per tier wins) — **NOT IMPLEMENTED**: drrp_history currently appends, doesn't clear previous entries for the same tier
- `extraction_method` simplified to just "who won" — ✅ this is how it works (extraction_method reflects the tier that set drrp_types)

### Transition rules

When stages run independently (`taxa classify --laws X`), they process what's specified. When run together via `taxa enrich`, the transition rules determine what gets passed forward between stages. Without these rules, LLM would run on the full stack — expensive and pointless.

**Regex → Classifier**: all provisions with embeddings. The classifier adds signal to everything it can — it's cheap (microseconds per provision). No filtering needed.
- ✅ Implemented: `cmd_taxa_classify` processes all provisions with embeddings where `tier < source_tier("classifier")` (`fcd0fc4`)

**Classifier → LLM**: only flagged provisions. This is the critical filter — it determines what costs money and time. A provision is flagged for LLM when:
- Regex and classifier disagree on DRRP type (regex=Obligation, classifier=Liberty) — ✅ flagged as `pending_llm` (`fcd0fc4`)
- Both obligation and enabling modals are present in the text (#41) — ✅ detected, flagged (`fcd0fc4`)
- Classifier confidence is below a threshold on a provision where regex found DRRP — ✅ below-threshold provisions are skipped (not flagged, regex stands). **GAP**: these aren't explicitly flagged for LLM — they silently keep the regex result.
- Regex=none AND classifier predicts DRRP at low confidence (weak signal worth verifying) — **NOT IMPLEMENTED**: low-confidence classifier predictions on regex=none provisions are silently dropped. No flag.
- No actor extracted but modal present (implied actor — LLM needs sibling context #38) — **NOT IMPLEMENTED**: this detection isn't in the classifier. It's a separate signal that would need to be checked during `cmd_taxa_parse` or as a post-parse filter.

**`taxa escalate` reads flagged provisions**: `cmd_taxa_escalate` currently runs on its own candidate selection logic (multi-actor, DRRP=none with actors). It does NOT yet consume the `pending_llm` flag set by the classifier. **GAP**: the escalate function needs to query for `extraction_method = 'pending_llm'` provisions.

Everything else stays at its current classification. The LLM only sees provisions that need resolving.

### Naming cleanup

| Current | Proposed | Why |
|---------|----------|-----|
| `--gap-c` | `--escalate` or separate `taxa escalate` | "gap-c" is meaningless |
| `TIER2_PROVIDER` | `LLM_PROVIDER` | Tier 2 is the classifier, not the LLM |
| Phase 3 / Phase 4 | `taxa classify` (DRRP + position in one step) | Phase numbers are internal |
| `enrich_single_law` | `parse_law` | It does more than enrichment but the core is parsing |
| `--pending` | `--new-laws` or just detect automatically | "pending" is vague |
| `source_tier()` | keep | This is clear |
| `extraction_method` | keep | This is clear |

### Gemini's other observations (to address)

1. **Formalize disagreement resolution rules** — define what counts as a disagreement and what the resolution policy is (LLM override, confidence-weighted, etc.)
2. **Layered pipeline control** — consider a state machine or pipeline definition rather than implicit checks in main.rs
3. **Enhanced LLM context** (#38) — sibling provisions, section/chapter context for LLM escalation
4. **Benchmark cascade value** — A/B test whether LLM escalation actually improves accuracy over just regex + classifier

### Scope of refactor

This is a significant refactor of `main.rs` (the largest file in the codebase). It should be done in stages:

1. ~~**Stage 1**~~: DONE (`7de07d8`). `drrp_history` field added to LanceDB schema. Migration script for existing data. 134K provisions populated.
2. ~~**Stage 2**~~: DONE (`010cc69`). `cmd_taxa_parse`, `cmd_taxa_classify`, `cmd_taxa_escalate`, `cmd_taxa_embed` extracted as separate functions + CLI subcommands. Orchestrator rewired.
3. ~~**Stage 3**~~: DONE (`fcd0fc4`). Wire the cascade — `taxa classify` reads regex output and appends to `drrp_history`, flags disagreements.
4. ~~**Stage 4**~~: DONE (`3d52f29`). Renamed `--gap-c` → `--escalate`, `TIER2_PROVIDER` → `LLM_PROVIDER`.
5. ~~**Stage 5**~~: DONE (`53f4a5b`). Error handling — embed/classify failures logged, pipeline continues.

### `taxa enrich` orchestration

When `taxa enrich` chains the stages:
```
taxa enrich --laws X
  → taxa parse --laws X
  → taxa embed --laws X (only provisions without embeddings)
  → taxa classify --laws X (all provisions with embeddings)
  → taxa escalate --laws X (only flagged provisions, if LLM_PROVIDER set)
```

For `--pending` (new laws via sync watch), `taxa embed` runs because new provisions lack embeddings. For `--force` (re-enrichment), `taxa embed` is skipped — embeddings already exist.

If any stage fails (e.g., LLM API error), the pipeline logs the error and continues — the provision keeps its current best classification. Failed provisions can be retried later by running the failing stage independently.

## Key files

- `crates/fractalaw-cli/src/main.rs` — pipeline orchestration, now decomposed:
  - `cmd_taxa_parse()` — regex parse + inheritance + write (~line 4698)
  - `cmd_taxa_embed()` — compute embeddings (~line 4752)
  - `cmd_taxa_classify()` — DRRP + position classifiers (~line 4914)
  - `cmd_taxa_escalate()` — LLM classification (~line 5412)
  - `cmd_taxa_enrich()` — orchestrator (~line 5470)
- `crates/fractalaw-core/src/taxa/mod.rs` — `parse_v2()`, purpose gates
- `crates/fractalaw-store/src/lance.rs` — LanceDB read/write, `drrp_history` column check
- `crates/fractalaw-core/src/schema.rs` — `drrp_history` List<Struct> schema definition
- `docs/drrp_classifier_v8.json` — classifier weights
- `scripts/migrate_drrp_history.py` — schema migration script
