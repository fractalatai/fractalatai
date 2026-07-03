---
session: Gap C — orchestration
status: closed
opened: 2026-04-15
closed: 2026-04-15
outcome: success
summary: 'Cross-session coordination document for the Gap C (implicit-actor DRRP) work. Defined sequencing, hand-off artefacts,
  and go/no-go gates across three child sessions (main-repo prep, training-repo bootstrap, main-repo integration) spanning
  two repos. Established Phase 1 (C2 pilot) and Phase 2 (C4+C6 expansion) milestones with cost envelope under $500/year steady-state.

  '
decisions:
- what: Three-session structure with explicit hand-off packages
  why: Gap C is too large for one session and spans two repos with different stacks
  result: Session 1 (main prep), Session 2 (training), Session 3 (integration) with clear gate criteria
- what: Go/no-go gates between sessions
  why: Prevent downstream sessions from starting on unstable foundations
  result: Four gates defined with specific pass criteria including precision thresholds
- what: C2 count as critical number for Session 1 exit
  why: Phase 1 trains on C2 sub-type only; insufficient C2 data blocks training
  result: Gap C parquet must report C2 count before Session 2 starts
lessons:
- title: Orchestration docs prevent scope drift across sessions
  detail: Explicit hand-off artefacts and gate criteria keep three sessions aligned despite different repos and stacks
  tag: process
metrics:
  child_sessions: 3
  go_no_go_gates: 4
  annual_cost_envelope: <$500
artifacts:
- .claude/sessions/taxa-drrp/04-15-26-gap-c-session-1-main-prep.md
- .claude/sessions/taxa-drrp/04-15-26-gap-c-session-2-training-repo-todo.md
- .claude/sessions/taxa-drrp/04-15-26-gap-c-session-3-main-integration.md
- .claude/sessions/taxa-drrp/04-15-26-gap-c-ai-research.md
depends_on:
- 04-15-26-gap-c-ai-research
- 04-15-26-gap-c-critical-review
enables:
- 04-15-26-gap-c-session-1-main-prep
- 04-15-26-gap-c-session-2-training-repo-todo
- 04-15-26-gap-c-session-3-main-integration
---

# Gap C \u2014 orchestration (CLOSED)

**Date**: 2026-04-15
**Status**: Active
**Spec**: [04-15-26-gap-c-ai-research.md](04-15-26-gap-c-ai-research.md) — the research/decision record. Read that first.

## Purpose

Gap C (implicit-actor DRRP provisions) is too large to tackle in one session.
The research session landed the architecture and resolved the three open
questions (no meta-labels; `holder_inferred_from` schema extension; training
infra external). This doc coordinates the execution across three child
sessions and two repos.

## The three sessions

| Session | Scope | Repo | Doc |
|---------|-------|------|-----|
| 1 | Main-repo prep (S1, S2, S3) | `fractalaw` | [04-15-26-gap-c-session-1-main-prep.md](04-15-26-gap-c-session-1-main-prep.md) |
| 2 | Training-repo bootstrap (S4, S5, S6) — scoping / todo because the repo doesn't exist yet | *(new training repo, TBD)* | [04-15-26-gap-c-session-2-training-repo-todo.md](04-15-26-gap-c-session-2-training-repo-todo.md) |
| 3 | Main-repo integration (S7, S8) | `fractalaw` | [04-15-26-gap-c-session-3-main-integration.md](04-15-26-gap-c-session-3-main-integration.md) |

All three sessions reference the research doc for rationale. This
orchestration doc is the time-bound coordinator: sequencing, hand-off
artefacts, and go/no-go checks between sessions.

## Sequencing

```
   ┌────────────────┐
   │  Research doc  │  (spec — read before any session)
   └───────┬────────┘
           │
           ▼
   ┌────────────────┐
   │   Session 1    │  main repo: S1a → S1b → S3
   │   (main prep)  │  (S2 holder_inferred_from deferred to Session 3 S7)
   └───────┬────────┘
           │  hand-off package:
           │   • categorised Gap C parquet (S1a output) — C2 count critical
           │   • reconciled holder_labels.json (S1b output)
           │   • context-retrieval format spec + 2048-token budget (S3)
           ▼
   ┌────────────────┐
   │   Session 2    │  training repo (to be created):
   │   (training)   │  Phase 1 (C2 pilot): S4 → S5 → S6
   │                │  Phase 2 (C4+C6 head-swap): repeat after S8
   └───────┬────────┘
           │  hand-off back:
           │   • HF Hub private-repo revision pin
           │   • serverless endpoint URL + model_version
           │   • min_fractalaw_ai_version
           │   • model card (per-sub-type precision/recall)
           │   • any new concrete roles added to holder_labels.json
           ▼
   ┌────────────────┐
   │   Session 3    │  main repo: S7 → S8 → (S8b Phase 2 re-eval) → S9
   │ (integration)  │  S7 absorbs the deferred holder_inferred_from work
   └────────────────┘
```

## Hand-off artefacts

### Session 1 → Session 2

1. **Categorised Gap C parquet** — one row per Gap C provision, columns:
   `law_id`, `article`, `text`, `sub_type` (C1–C6), plus any context
   pointers identified during sub-type classification.
2. **Context-retrieval format spec** — how S3's helper serialises the
   `[clause | parent | section | act-general-duty]` context window into the
   text passed to the model. This is the contract; training must produce
   training examples in the same shape.
3. **`holder_labels.json` — reconciled and pinned.** The cleaned model
   vocabulary produced by Session 1 S1b: trailing-colon bug fixed,
   regex-era placeholders (`": He"`) excluded, and ~30+ concrete roles
   from `actors.rs` added. This is the **model vocabulary** (training
   output space), distinct from `actors.rs`'s regex detection vocabulary.
   See research doc §4.1.
4. **`holder_inferred_from` field design spec** — research doc §4.1a
   defines the field. The DuckDB LRT column and the `DrrpExtraction`
   struct field itself ship in **Session 3 S7** (deferred from Session
   1 to avoid dead schema without a producer). Training-time labels
   must carry an `inferred_from` field per example so the model can be
   supervised on provenance, matching the spec. Training repo doesn't
   need the column to exist in main at hand-off; it just needs to
   produce labels in the spec's shape.

### Session 2 → Session 3

1. **HF Hub revision pin** — specific commit hash / revision tag for the
   model artefact in the private repo.
2. **`model_version` string** — baked into `metadata.json`, read by
   `DrrpExtractor::load`.
3. **`min_fractalaw_ai_version`** — minimum main-repo crate version
   required to load this model. Prevents silent incompatibility.
4. **Model card** — per-sub-type precision/recall, held-out law list,
   training dataset provenance, known failure modes.
5. **Label-vocabulary delta** — list of any new concrete roles added to
   `holder_labels.json` during labelling, with rationale. Main must apply
   these deltas before integration.

## Go/no-go gates

Don't cross a boundary if the preceding session didn't clear its gate.

**Session 1 exit gate** (before starting Session 2):
- Gap C parquet exists and sub-type counts match the OHS gap-analysis
  numbers (3,275 for the OHS run). C2 count specifically must be
  sufficient for Phase 1 training — that's the critical number.
- **Reconciled `holder_labels.json` committed.** Trailing-colon bug
  fixed. `": He"` excluded. ~30+ concrete roles from `actors.rs` added.
  Diff list documented. This is a hard gate — Session 2 cannot start
  labelling against an unstable vocabulary.
- Context-retrieval helper has tests for C2 (required for Phase 1).
  C4 and C6 tests are nice-to-have; required before Phase 2 head-swap.
- Context format spec `docs/gap-c-context-format.md` committed with the
  2048-token budget pinned.
- Zero regression in existing taxa tests. Current precision 96.4% intact.
- **Not a Session 1 exit criterion**: `holder_inferred_from` schema —
  deferred to Session 3 S7 per scope change.

**Session 2 Phase 1 exit gate** (before starting Session 3):
- Held-out eval shows precision ≥ 96% on a C2-included test set.
- **C2 recall reported** — Phase 1's target sub-type; other sub-types
  (C1, C3, C4, C5, C6) expected mostly unchanged in Phase 1.
- Serverless endpoint is live and reachable via HTTPS with the
  documented `POST /detect` contract.
- `model_version` returned by the endpoint matches what's declared in
  the published metadata.
- No meta-labels in model output — confirmed by inspecting a sample of
  predictions. No `": He"` in outputs.

**Session 3 exit gate — Phase 1** (project milestone, not complete):
- OHS gap analysis re-run shows recall >48.6% (baseline), precision ≥96%,
  driven primarily by C2 improvements.
- 50 Fix-2 provisions accounted for as expected pickups.
- Inference-source distribution log is available for spot-check review.
- `holder_inferred_from` populated for detector-emitted entries;
  sertantai backfill verification (zero `": He"`, zero trailing-colon
  holders) passes.

**Session 3 exit gate — Phase 2** (project complete):
- Head-swap retrain produced a C4+C6 model and it's live at the
  endpoint.
- Gap analyses show further recall improvement without precision
  regression.
- S9 cleanup complete: deprecated artefact removed, `": He"` retired
  from `actors.rs`, no dead code.

## Risks to track across sessions

- **Taxonomy drift (S1↔S4)**: if labelling reveals C1–C6 is wrong,
  Session 2 may need to loop back to Session 1. Orchestration budget
  reflects this as a possible iteration.
- **Context-format drift (S3↔S6)**: the context-retrieval helper in main
  is the contract; training conforms to it, not vice-versa. If training
  wants to change the format, a PR into main must land first.
- **Label-vocab drift (S6↔S7)**: reactive ontology growth during training
  needs to be reflected in main before integration. Session 2 hand-off
  must enumerate any additions explicitly.
- **Precision regression (S7)**: the confidence threshold is the main
  lever. Start high, lower only with evidence from the eval harness.

## Cost envelope

See research doc §8 for the full cost envelope table (initial catchup,
ongoing inference, head-swap retrain, full retrain). Headline figures:

- **Initial catchup**: ~$1–10 (one-off, inference over ~3k Gap C
  provisions)
- **Ongoing inference**: <$1/month (20–100 provisions/month after
  catchup)
- **Head-swap retrain**: ~$5–20 per new role added to taxonomy
- **Full retrain**: ~$50–200 (rare, ~1×/year)
- **Total annual steady-state**: well under $500

Cheap relative to the quality impact. Budget impact is trivial; the
expensive part of this work is labelled training data, not compute.

## Living document

This doc is time-bound and will be updated as sessions progress. Each
session log should update this doc at its own exit gate with:
- Actual deliverables produced (vs. planned)
- Any hand-off package deviations
- Gate pass/fail with evidence

When all three sessions are complete, move this doc + the three session
docs into an archive subfolder or mark them all `Status: Complete`.
