# Session: Cascade Transition Rules — Codify in Code

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

1. **Correct cascade order**: regex → classifier → LLM. Each tier sees the output of the previous tier.
2. **Loose coupling**: each tier (regex, classifier, LLM) must be able to run independently for testing and improvement. `taxa parse`, `taxa classify`, `taxa llm` as separate subcommands that can run standalone or in sequence.
3. **Clear naming**: `gap-c`, `--pending`, Phase 3/4 etc. are cryptic. Use names that describe what they do: `taxa parse` (regex), `taxa classify` (embedding classifier), `taxa escalate` (LLM on disagreements).
4. **DRRP provenance at provision level**: a `drrp_history` field (per Gemini recommendation) that records what each tier said, not just who won.
5. **Disagreement detection**: when regex and classifier disagree, the provision is flagged for LLM escalation. When both obligation and enabling modals are present (#41), flag for LLM.
6. **No silent overrides**: every tier ADDS signal. The final `drrp_types` is determined by explicit resolution rules, not by whoever runs last.

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

The final `drrp_types` is determined by the highest-tier non-none entry. Disagreements are visible in the history.

### Disagreement rules

A provision is flagged for LLM escalation when:
- Regex and classifier disagree on DRRP type (regex=Obligation, classifier=Liberty)
- Both obligation and enabling modals are present in the text (#41)
- Classifier confidence is below a threshold on a provision where regex found DRRP
- No actor extracted but modal present (implied actor — LLM needs context)

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

1. **Stage 1**: Extract `taxa parse`, `taxa classify`, `taxa escalate` as separate functions. Keep them callable from `taxa enrich` for backward compatibility.
2. **Stage 2**: Wire the cascade — `taxa classify` reads regex output, flags disagreements. `taxa escalate` reads flagged provisions.
3. **Stage 3**: Add `drrp_history` field to LanceDB schema. Each tier appends to it.
4. **Stage 4**: Rename cryptic flags (`--gap-c` → `--escalate`, `TIER2_PROVIDER` → `LLM_PROVIDER`).
5. **Stage 5**: Add CLI subcommands so each tier can run independently.

## Key files

- `crates/fractalaw-cli/src/main.rs` — pipeline orchestration (~7K lines, needs decomposition)
- `crates/fractalaw-core/src/taxa/mod.rs` — `parse_v2()`, purpose gates
- `crates/fractalaw-store/src/lance.rs` — LanceDB read/write
- `docs/drrp_classifier_v8.json` — classifier weights
