---
session: 'Gap C — Session 3: main-repo integration'
status: closed
opened: 2026-04-15
closed: 2026-04-15
outcome: success
summary: 'Scoping document for Session 3 of the Gap C work, blocked on Session 2 hand-off. Defined four deliverables: S7 (detector
  integration with remote-inference provider, holder_inferred_from schema, deprecated polisher disabling, and placeholder
  routing), S8 (validation against OHS gap analysis with precision >= 96% guard), S8b (Phase 2 C4+C6 head-swap evaluation),
  and S9 (final cleanup removing deprecated DistilBERT artefact). S7 is the largest single deliverable with a natural split
  seam defined.

  '
decisions:
- what: S7 absorbs holder_inferred_from, remote provider, polisher disabling, and placeholder routing
  why: Shipping schema alongside its producer avoids dead columns; consolidation is natural
  result: Largest deliverable with S7a/S7b split seam defined if needed
- what: Confidence threshold starts high (0.85)
  why: Precision is the constraint; below-threshold detections suppressed not emitted
  result: Config value, not magic number; lower only with eval evidence
- what: Offline fallback to regex-only
  why: If endpoint unreachable, enrichment must proceed without blocking
  result: Detector error treated as below-threshold; logged at WARN level
- what: 50 Fix-2 provisions tracked as expected pickups
  why: These are genuine v2 matcher bugs the detector was designed to cover, not surprise detections
  result: Counted in recall improvement, not treated as novel detections requiring review
lessons:
- title: Define split seams for large deliverables upfront
  detail: S7 consolidates five pieces of work; defining the S7a/S7b boundary in advance prevents scope paralysis at execution
    time
  tag: process
metrics:
  deliverables_planned: 4
  precision_floor: 96%
  recall_baseline: 48.6%
  fix2_provisions: 50
  confidence_threshold: 0.85
artifacts:
- crates/fractalaw-ai/src/extractor.rs
- crates/fractalaw-core/src/taxa/
- crates/fractalaw-cli/src/main.rs
depends_on:
- 04-15-26-gap-c-session-2-training-repo-todo
- 04-15-26-gap-c-orchestration
enables:
- Gap C project completion
- Deprecated artefact removal (S9)
---

# Gap C \u2014 Session 3: main-repo integration (CLOSED)

**Date**: 2026-04-15
**Status**: Not started — blocked on Session 2 hand-off
**Orchestration**: [04-15-26-gap-c-orchestration.md](04-15-26-gap-c-orchestration.md)
**Spec**: [04-15-26-gap-c-ai-research.md](04-15-26-gap-c-ai-research.md)

## Scope

Stages S7, S8 from the research doc. All work in the `fractalaw` repo.
Consume the training hand-off, wire the detector into the pipeline, prove
the gain, protect precision.

## Entry criteria — Session 2 hand-off required

Cannot start until Session 2 delivers:
- [ ] HF Hub private-repo revision pin (commit hash / tag)
- [ ] `model_version` string baked into `metadata.json`
- [ ] `min_fractalaw_ai_version` declared in `metadata.json`
- [ ] Model card with per-sub-type precision/recall on held-out laws
- [ ] Label-vocabulary delta list (new concrete roles, if any)
- [ ] Held-out eval shows precision ≥ 96%

If any of those is missing, do **not** start Session 3 — loop back to
Session 2.

## Deliverables

### S7 — detector integration

**Goal**: detector head wired into `taxa::parse_v2` as a post-skip-gate
fallback. Must not regress precision. Feature-flagged and
threshold-gated from day one.

**Scope note**: S7 is the largest single deliverable across all three
sessions. It absorbs several pieces of work that cluster naturally
together: the `holder_inferred_from` schema (deferred from Session 1),
the remote-inference provider, disabling the deprecated local polisher,
the regex-placeholder routing rule, and the detector integration itself.
The consolidation is deliberate — shipping schema alongside its producer
avoids dead columns. If S7 proves too large to execute as one PR at
implementation time, the natural split seam is:

- **S7a**: schema extension (`holder_inferred_from`) + remote-inference
  provider scaffolding + deprecated-polisher short-circuit. Pure
  plumbing; no behavioural change to enrichment yet.
- **S7b**: `parse_v2` integration + detector call path + placeholder
  routing + tests. The behavioural change, gated behind feature flag.

Tasks:
- [ ] **Land the `holder_inferred_from` schema** (deferred from Session
  1 per scope change; field design in research doc §4.1a):
  - Add `holder_inferred_from: Option<String>` to
    `fractalaw_ai::DrrpExtraction` with
    `#[serde(skip_serializing_if = "Option::is_none")]`.
  - Extend the DuckDB LRT per-provision DRRP storage with a nullable
    `holder_inferred_from TEXT` column. Additive migration, safe on
    existing data.
  - Update the Zenoh publish payload in `fractalaw-sync` to include
    the field (omit when null). Contract note in the sync session log.
  - Update `taxa show` / `taxa qa` output to display the field when set.
  - Tests: round-trip a DRRP with and without `holder_inferred_from`
    through DuckDB and the Zenoh publish stub.
- [ ] Apply label-vocabulary delta from Session 2 hand-off:
  - Update `holder_labels.json` in the main repo (new path, not
    `models/deberta-v3-drrp/` which is deprecated and awaiting S9).
  - Extend `actors.rs` with any new concrete role patterns to keep
    regex and model holder sets aligned.
- [ ] **Disable the deprecated local polisher call** in
  `crates/fractalaw-cli/src/main.rs` (currently around line 657, which
  loads `DrrpExtractor` if the local artefact exists). The
  `best_clause_acc: 0.0` means any `ai_clause` values it wrote to
  DuckDB are garbage; stopping the call is a prerequisite for the
  remote detector's outputs to replace them cleanly during
  re-enrichment. Full removal waits for S9 — here we just
  short-circuit the call path.
- [ ] **Extend `fractalaw-ai` with a remote-inference provider** (per
  research doc §0.2 — this is an extension, not a rewrite):
  - New module, e.g. `fractalaw-ai::remote::RemoteInferenceClient`.
  - HTTPS client targeting the RunPod/Modal/HF endpoint URL, pinned in
    config (`FRACTALAW_DETECTOR_ENDPOINT` env var or similar).
  - Handles retry, timeout, and authentication (API token via env).
  - Returns the detector's structured output (`holder_class`,
    `drrp_type`, confidence, spans).
  - Existing local-ONNX path (`DrrpExtractor`) stays untouched for
    edge micro-apps that use MiniLM/lightweight classifiers.
- [ ] **Version compatibility check** — the remote endpoint returns a
  `model_version` header/field. The client compares against a
  compatible range declared in `fractalaw-ai` and refuses unknown
  majors. Tests covering compatible-accept and major-mismatch-reject.
- [ ] Extend `fractalaw-core::taxa::parse_v2`:
  - After the skip-gate pass, if a provision produced no DRRP AND is
    not a skip-gate hit, call the detector via an injected trait
    (keep `fractalaw-core` free of HTTP/runtime deps — detection
    crosses to `fractalaw-ai` through a trait boundary).
  - Use `fractalaw-store::context::fetch_context` (from Session 1
    S3) to build the context window.
  - On detector hit above threshold: emit DRRP, populate
    `holder_inferred_from` with the appropriate citation from the
    context's `sources`.
  - On below-threshold or detector error: no emission. Provision stays
    a Gap C miss.
- [ ] Feature flag on the `fractalaw-ai` crate:
  `--features remote-detector` (or similar) — off by default until
  regression-free. CLI opt-in via `taxa enrich --detector`.
- [ ] Confidence threshold: start **high** (e.g. 0.85 softmax
  probability on the holder class). Make it a config value, not a
  magic number.
- [ ] Offline behaviour: if the endpoint is unreachable, the detector
  returns an error and `parse_v2` treats it as below-threshold (no
  emission). Enrichment proceeds with regex-only for the affected
  provisions. Logged at WARN level.
- [ ] **Regex-placeholder routing rule (per research doc §8).** Extend
  `parse_v2` so that when the regex path emits a DRRP with holder in
  the placeholder set (`": He"`, extensible), the provision is routed
  through the detector with retrieved context:
  - Detector resolves to concrete role above threshold → replace the
    placeholder holder with the concrete role, populate
    `holder_inferred_from` with the source citation, emit the DRRP.
  - Detector below threshold or errors → **suppress the entry from
    publish** (don't emit `": He"` to sertantai). Log at INFO level
    so the suppression rate is visible.
  - Define the placeholder set in one place (e.g. a constant in
    `fractalaw-core`), not scattered.
- [ ] Tests:
  - Detector suppresses below-threshold predictions (no DRRP emitted).
  - Detector emits correct `holder_inferred_from` for a C2 parent-
    inherited example.
  - Detector doesn't run on skip-gate hits.
  - Integration test: enrich a known C2 provision end-to-end, assert
    DRRP appears with correct holder and inference source.
  - **Placeholder routing test**: feed a provision that regex resolves
    to `": He"`, assert detector is called and either a concrete role
    is emitted or the entry is suppressed (never `": He"` in the
    output).

### S8 — validation and precision guard

**Goal**: prove Gap C recall improved; prove precision didn't regress.
Produce inference-source distribution logs for reviewer spot-check.

Tasks:
- [ ] Re-run the OHS gap analysis (`taxa-gap-analysis/04-14-26-ohs-occupational-safety.md`
  workflow) against the detector-enabled enrichment. Report:
  - New recall % (baseline 48.6%)
  - New precision % (baseline 96.4% — must hold)
  - Gap A / Gap C proportion of FN (baseline Gap C = 78% of FN)
  - **C2 sub-type coverage** — Phase 1 target. Other sub-types
    (C1, C3, C4, C5, C6) expected mostly unchanged in Phase 1; they
    come in with Phase 2 head-swap retrain.
- [ ] **Account for the 50 Fix-2 provisions** identified in the OHS
  gap analysis as expected pickups, not surprise detections. Fix-2
  was the bucket of genuine v2 matcher bugs (subordinate clause +
  pronoun reference, epistemic "may" rejection) that the AI detector
  was designed to cover. When validating S8:
  - Count these explicitly in the recall improvement figure.
  - Don't treat them as novel detections requiring human review.
  - If the detector *misses* any of the Fix-2 provisions, flag it —
    these should be easier than general Gap C.
- [ ] Run against at least one other family's gap analysis (e.g. the
  non-OHS family that had the lowest recall) to check generalisation.
- [ ] Inference-source distribution report: for the detector-emitted
  DRRPs, histogram of `holder_inferred_from` values. If one source
  (e.g. HSWA s.2) dominates unreasonably, flag for review.
- [ ] Review a stratified sample of ~50 detector-emitted DRRPs with a
  domain-expert eye — are the holders correct? Are the
  `holder_inferred_from` citations valid?
- [ ] If precision drops below 96%: raise the confidence threshold and
  re-evaluate. If raising doesn't recover, *do not ship* — loop back
  to Session 2 with the failure analysis.
- [ ] If precision holds: flip the feature flag to on-by-default in a
  follow-up PR, document in the enrichment runbook.
- [ ] **Sertantai backfill verification** (research doc §8 transition
  rule). After re-enrichment with AI enabled and re-publish:
  - Query sertantai for any remaining DRRP entries with holder =
    `": He"` or holder ending with `":"` (the `"Gvt: Ministry:"`
    artefact). Expected count after backfill: **zero**.
  - If non-zero: identify the laws that haven't been re-enriched +
    re-published and run them. No separate migration script —
    re-publish is the fix.
  - Spot-check: pick 5 laws that historically had `": He"` entries,
    confirm each is now either a concrete role (AI resolved) or
    suppressed (below threshold).

### S9 — Final cleanup (deferred until S8 passes)

**Goal**: remove the deprecated DistilBERT polisher artefact and any
dead code now that the remote detector is proven. Do this **last**,
after S8 has validated the new pipeline — the old artefact stays in
place as ignored-but-present reference during S7 and S8, so rollback
is painless if something goes wrong.

Tasks:
- [ ] Rename or remove `models/deberta-v3-drrp/`. Directory is
  misnamed (DistilBERT inside, not DeBERTa) and the artefact never
  worked (`best_clause_acc: 0.0`, 200 training examples). Lean
  **delete** — the training repo is the authoritative source for any
  historical model; the deprecated local artefact doesn't need
  preservation.
- [ ] Review `crates/fractalaw-ai/src/extractor.rs`:
  - If no micro-app or code path depends on `DrrpExtractor::extract`
    (the polisher entry point), remove it.
  - If edge micro-apps still use `DrrpExtractor::load` for a
    different model, retain the struct but narrow its documentation
    to reflect the actual surviving use.
- [ ] Remove any `onnx` feature-gated code that only served the
  deprecated artefact. Audit `Cargo.toml` features.
- [ ] Update CLAUDE.md and `.claude/plans/micro-apps.md` if they
  reference the DistilBERT DRRP Polisher as a live anchor example;
  replace with the remote Gap C detector as the current implementation
  of that micro-app pattern.
- [ ] **Retire regex-era placeholders from `actors.rs`.** Once S8
  confirms the detector resolves pronoun cases reliably, remove
  `": He"` (and any other placeholder labels that served only as
  Gap C pre-AI workarounds) from `actors.rs`. At that point the two
  vocabularies (§4.1) converge: regex detects only concrete roles,
  AI handles everything else. Update tests that asserted `": He"`
  outputs — the new behaviour is "route to detector" not "emit
  placeholder".
- [ ] Grep the codebase for any lingering references to `deberta-v3`,
  `DrrpExtractor::extract`, or the deprecated model path. Clean up.
- [ ] Verify nothing in CI/tests depends on the removed artefact.

Exit criteria: repo is consistent — no misleading names, no dead code,
no dangling references to the deprecated artefact.

## Exit criteria — project complete

Per the orchestration doc:
- OHS gap analysis re-run shows recall > 48.6% (baseline), precision ≥
  96%.
- Gap C proportion of FN reduced from 78%.
- Inference-source distribution log available; stratified human spot-
  check confirms holders and citations.
- S9 cleanup complete — no dead code or misnamed artefacts remain.

On pass: update orchestration doc status to Complete, mark all three
session docs Complete, and write a short close-out note referencing the
before/after metrics and the GH issues closed.

## Open items

- How to surface `holder_inferred_from` in `taxa show` / `taxa qa` UX.
  Worth showing as a second line under each DRRP where present.
- Whether the detector should also run on *skip-gated* provisions as
  a diagnostic (not for publish). Useful for catching mis-gated
  provisions, but adds noise. Defer unless the gap analysis flags a
  need.
- Monitoring: once the detector is on by default, add a metric to the
  enrichment logs for "detector fired N times, emitted M, suppressed K"
  so drift can be noticed without re-running the full gap analysis.
