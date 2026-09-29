---
session: Enrichment Provenance
status: closed
opened: 2026-09-27
closed: 2026-09-29
outcome: success

summary: >
  Built fractalatai #63: every enrichment stage (Rust CLI, sync-cli triage, Python and pod scripts) records its run, git version,
  method, model and prompt version per law, plus the lat_hash/struct_hash of the rows it read, via hub SQL functions.
  `sync publish --pg` sends legal's per-family provenance column. Verified end to end: legal received 363 (Tier 0) + 24
  (live-fix) per-family events, all on the current LAT hashes.

decisions:
  - what: Record in SQL functions in the hub
    why: Rust and Python (including pod scripts over the tunnel) must record identically
    result: fractalaw_start_run / fractalaw_record_stage / fractalaw_lat_hash / fractalaw_struct_hash
  - what: enriched_against = the family's earliest stage
    why: Output built on an older LAT is only as fresh as its oldest input
    result: Legal's lat_held_stale_enrichment can flag it
  - what: Record adjudication and no-duty-text as their own provenance
    why: Verdicts must stay auditable
    result: stage adjudication (human-review); model fractalaw-law-drrp:no_duty_text

metrics:
  legal_events: { tier0: 363, livefix: 24 }
  stages_per_law_typical: 13-17

lessons:
  - title: Provenance only exists for stages that actually ran
    detail: Laws enriched before #63 carry none until their stages re-run; archived LAT can't be re-attested.
    tag: data

artifacts:
  - crates/fractalaw-core/build.rs
  - crates/fractalaw-core/src/provenance.rs
  - crates/fractalaw-store/src/pg_provenance.rs
  - crates/fractalaw-cli/src/provenance.rs
  - scripts/ml/fractalaw_provenance.py

depends_on:
  - 09-26-26-robust-lat-sync.md
---

# Session: Enrichment Provenance (CLOSED)

## Problem

sertantai-legal can now ingest enrichment provenance: an optional `provenance` column in the law-level taxa payload, live on @dev (legal `ae31d1e`). Fractalaw records none of it. There's no run id, no git sha in the binaries, model versions are implicit, and nothing records which LAT version each family read. So legal can't flag enrichment that a model upgrade or a re-parse has made stale (`lat_held_stale_enrichment`), and can't explain a verdict. fractalatai #63.

## Todo

- ✅ Git sha stamped at build: `fractalaw-core/build.rs` → `provenance::VERSION`, used by both binaries
- ✅ Hub schema (`fractalaw-store/src/pg_provenance.rs`): `enrichment_runs`, `enrichment_provenance` (latest per law × family × stage), and SQL functions `fractalaw_start_run`, `fractalaw_record_stage`, `fractalaw_lat_hash`, `fractalaw_struct_hash`. The hashes are computed in SQL from the rows each stage read, and match the Rust contract and legal's manifest.
- ✅ Capture on every stage command: taxa parse/embed/classify/escalate/infer/reconcile/slm/validate/backfill (DRRP roll-up only for laws it wrote; significance roll-up for all); fitness extract/reconcile/application/compile (not `--out`); sync-cli triage. Recording never fails a command, and URLs are masked in the stored command line.
- ✅ Explicit versions: rules content hash (actor dictionary + correlative rules), classifier `v8+v3`, `all-MiniLM-L6-v2`/onnx, `gemini-2.5-flash` or `gemma3:4b` (per `LLM_PROVIDER`), `gemma3-position`, significance roll-up `approach-L;LOW<=6.14;HIGH>=11.06`; prompts `code:<sha>` (Rust) / `sha:<prompt hash>` (Python)
- ✅ Python stages record via `scripts/ml/fractalaw_provenance.py`: runpod_slm / significance / fitness, gemini_llm_batch, compute_dep_features, derive_hierarchy. The pod skills now upload the module and pass `FRACTALAW_VERSION`.
- ✅ Per-provision significance method: `legislation_text.significance_method` (`slm:gemma3-significance`), written by the significance batch; earlier rows count as `unrecorded`
- ✅ `provision_method_counts`: taxa from `provision_actors` (amendment scope excluded), fitness per mention (ft/llm/slm/regex/propagated/polarity_only), significance from `significance_method`
- ✅ Publisher (`--pg`): `provenance` column, one entry per family in the payload. The run/version come from the family's latest stage; `enriched_against` comes from its **earliest** stage, so the oldest input decides staleness.
- ✅ Tests: core (entry building, versions), store on the scratch DB (SQL hash = Rust contract; record → entries), sync-cli (payload column), Python recorder on the scratch DB, `taxa reconcile` end-to-end on the scratch DB (run, version, masked URL, hash = hub)
- ✅ Cross-check with legal on a real publish (Tier 0, 2026-09-28): 363 enriched events (taxa, fitness and significance × 121 laws), each with run_id, version, lat_hash, struct_hash and provenance; all hashes match legal's current LAT. The first publish lacked the column because the binary was stale, and was republished.
- ✅ First real recording: the Tier 0 run (all stages, incl. the pod scripts and the Gemini LLM). Other laws have no provenance until their stages re-run.

## Dependencies

- ✅ Legal's side: `lat_events`, TaxaSubscriber reads `provenance` (legal `ae31d1e`, @dev)
- ✅ #62 hashes available per law (`lat_sync_state`; `fractalaw_core::lat_sync::{lat_hash, struct_hash}`)
- ⬜ #62 hub sync run (so `lat_sync_state` holds the current hashes for every law)

## Contract (legal's implementation, 2026-09-27)

- Keys per entry: `family`, `enrichment_run_id`, `fractalaw_version`, `enriched_against {lat_hash, struct_hash}`, `run_started_at`, `stages [{stage, method, model, model_version, prompt_version, ran_at}]`, `provision_method_counts`. Legal ignores other keys.
- `enriched_against` must be what the family's stages **read**, not the hub's current hashes.
- Stage lists by family are in #63, as agreed with legal. Reconcile and the law-level roll-up are the verdict-producing stages; the significance roll-up version includes the frozen thresholds.
