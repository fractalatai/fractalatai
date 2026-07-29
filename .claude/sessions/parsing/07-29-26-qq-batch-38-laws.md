---
session: "QQ Batch Parse — 38 triaged laws"
status: closed
opened: 2026-07-29
closed: 2026-07-29
outcome: success

summary: >
  Processed a new batch of 38 QQ laws triaged by sync-watch. 13 triaged as not_making
  (11 obvious amendments deleted, 2 reclassified as uncertain after finding obligations
  in triage counts). 27 laws enriched through the full pipeline: parse, dep features,
  embed, classify, infer, reconcile, position SLM, significance SLM, fitness SLM,
  backfill, and publish. Also fixed a critical reconcile bug, added --laws to the
  position SLM script, and fixed several other issues discovered during the run.

decisions:
  - what: "Reclassified UK_ssi_2008_221 and UK_uksi_1998_3084 from not_making to uncertain"
    why: "Both had with_obligation > 0 in triage counts but were classified not_making at only 17% confidence — triage false negatives"
    result: "Both kept for enrichment, enrichment found substantive provisions"
  - what: "Ran all 13,744 pending_slm actors globally rather than scoping to batch"
    why: "Position script had no --laws filter; cleared the entire backlog while pod was up"
    result: "Added --laws to runpod_slm_batch.py for future batches"
  - what: "Let significance + fitness run in parallel on one GPU despite contention"
    why: "Fitness was already half done when we noticed the 5x slowdown"
    result: "Added 'never run models in parallel' to runpod skill; run serially in future"

lessons:
  - title: "Reconcile was ignoring the position classifier entirely"
    detail: "The cascade went LLM > inferred > SLM > regex-only, skipping the agree/classifier/pending_slm logic. 3,907 disagreements were silently resolved by taking regex. Fixed and added 16 unit tests."
    tag: bug
  - title: "cls_confidence was never written by the position classifier"
    detail: "The value was computed (pred.confidence) but not persisted to the cls_confidence column. Extended upsert_provision_actors to accept Optional<f32> confidence."
    tag: bug
  - title: "Running two Ollama models in parallel on one GPU is ~5x slower than serial"
    detail: "OLLAMA_NUM_PARALLEL handles within-model parallelism. Cross-model parallelism causes GPU contention. Serial: ~7 min total. Parallel: ~30 min total."
    tag: operations
  - title: "triaged_at is the batch identifier for sync-watch runs"
    detail: "Laws from the same sync-watch run share a tight time window on triaged_at. Query by date to find a batch, narrow by time window for multiple runs on the same day."
    tag: operations
  - title: "541 Obligation provisions missing significance due to sequencing"
    detail: "SLM changed some provisions from DRRP=none to DRRP=Obligation. These only became Obligation after significance had already run. Need: backfill -> significance -> derive_hierarchy -> backfill, with significance running AFTER the final reconcile+backfill."
    tag: gap

artifacts:
  - crates/fractalaw-cli/src/commands/taxa.rs (reconcile fix + tests)
  - crates/fractalaw-store/src/pg.rs (cls_confidence in upsert)
  - crates/fractalaw-store/src/provision_store.rs (trait change)
  - scripts/ml/runpod_slm_batch.py (--laws filter)
  - scripts/ml/runpod_significance_batch.py (NoneType crash fix)
  - scripts/ml/runpod_fitness_batch.py (use gemma3-fitness model)
  - .claude/skills/customer-batch-parse/SKILL.md (triaged_at, --laws, slim Step 7)
  - .claude/skills/runpod-batch-inference/SKILL.md (--laws, no parallel models)

stats:
  batch_size: 38 triaged, 27 enriched, 11 not_making deleted
  provisions: 14,890 total, 13,135 substantive
  actors: 6,543 reconciled (96.2% SLM, 2.8% pending_llm)
  significance: 1,985/2,424 Obligation provisions rated (82%)
  fitness: 610 mentions extracted
  published: 13,862 provisions across 27 laws
  runpod_gpu: RTX 5090, ~45 min total pod time
  bugs_fixed: 5 (reconcile, cls_confidence, significance crash, fitness model, position --laws)
  tests_added: 16 reconcile unit tests
---

## Todo

- [x] Step 0: Identify batch from DuckDB (38 laws triaged today)
- [x] Step 0a: Review not_making (11 AMD deleted, 2 reclassified)
- [x] Step 0c: Parse (27 laws, 14,890 provisions)
- [x] Step 1: Dep features (6,472 actors)
- [x] Step 2: Embed (13,135 provisions)
- [x] Step 3: Classify (position classifier + cls_confidence fix)
- [x] Step 4: Infer (71 correlative actors)
- [x] Step 5: Reconcile (fixed to use classifier signal)
- [x] Step 6: Stats check
- [x] Step 7a: Position SLM (13,744 actors globally, 9.4/s)
- [x] Step 7b: Significance SLM (1,985 provisions, crashed once, restarted)
- [x] Step 7c: Fitness SLM (610 provisions)
- [x] Step 8: Re-reconcile + backfill
- [x] Step 8b: Derive hierarchy
- [x] Step 9: Final backfill + fitness compile
- [x] Step 10: Final stats (all QA PASS)
- [x] Step 11: Publish (27 laws LRT + 13,862 provisions LAT)
- [x] Step 12: Clear enrichment queue (262 remaining from older batches)
- [ ] Gap: 541 Obligation provisions need significance (next RunPod session)
