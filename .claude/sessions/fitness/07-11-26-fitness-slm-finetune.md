---
session: Fitness SLM Fine-Tune
status: closed
opened: 2026-07-11
closed: 2026-07-12
outcome: success

summary: >
  Fine-tuned gemma3:4b for fitness entity extraction. Precision jumped from ~60%
  to 93.3%, F1 92.5%, scope accuracy 100%. Ran full extraction on 14,248 provisions.
  Three data safety incidents drove per-tier column architecture and save-as-you-go
  writes. All three extraction tiers (regex 8,706, slm 6,604, ft 14,255) now preserved.

decisions:
  - what: Enrich dictionaries before fine-tuning to avoid OH&S training bias
    why: 7,325 training examples were heavily OH&S/FIRE. Adding cross-domain terms (vehicle, building, land, LLP) grew training data to 8,706 (+19%) with broader family coverage.
    result: Fine-tuned model extracts cross-domain entities, not just OH&S patterns.

  - what: Fine-tuned model gets its own column (ft_entities), not overwriting slm_entities
    why: Base model SLM data was destroyed twice by operations that should have been scoped. Each tier must own its own column — regex_entities, slm_entities, ft_entities, llm_entities.
    result: All three tiers preserved for comparison. --force only clears its own tier via UPDATE SET NULL.

  - what: Save-as-you-go writes, not batch-at-end
    why: First batch run collected 14,245 results in memory, process died during write phase, all results lost. Second run with autocommit + immediate writes survived.
    result: Each result committed immediately. Process death loses at most one provision.

  - what: All RunPod artifacts must be on /workspace (network volume), never /tmp
    why: GGUF written to /tmp was lost when pod stopped. BF16 on /workspace survived, requiring re-quantise on next pod.
    result: Skills updated. Network volume namespaced: models/drrp/, models/fitness/, models/significance/.

metrics:
  fine_tuning:
    train_examples: 7835
    test_examples: 871
    epochs: 3
    steps: 2940
    duration: "45 min"
    gpu: "RTX 5090"
    loss_start: 0.56
    loss_end: 0.25
  evaluation:
    precision: "93.3%"
    recall: "91.7%"
    f1: "92.5%"
    scope_accuracy: "100%"
    base_model_precision: "~60%"
  extraction:
    provisions_processed: 14248
    success: 14245
    json_error: 3
    speed: "9.3/s"
    duration: "25 min"
  tier_coverage:
    regex_entities: 8706
    slm_entities: 6604
    ft_entities: 14255

lessons:
  - title: --force must never delete rows — UPDATE SET NULL on the tier's own columns only
    detail: Three separate incidents lost data. First --force did DELETE FROM (all rows gone). Second --force did DELETE WHERE extraction_method='regex' (deleted rows that also had SLM data). The only safe pattern is UPDATE SET column=NULL for the requesting tier.
    tag: architecture

  - title: Batch scripts must write results immediately, not collect and write at end
    detail: First full batch (14,245 results) collected in memory, process died during the write phase, all work lost. With autocommit=True and per-result writes, each provision is committed independently. Process death loses at most one result.
    tag: methodology

  - title: Each extraction tier needs its own DB column from day one
    detail: Started with single 'entities' column + extraction_method tag. Evolved to regex_entities, slm_entities, ft_entities. Should have started with per-tier columns. The DRRP pipeline learned this same lesson (regex_position, cls_position, slm_position).
    tag: architecture

  - title: Training data bias follows dictionary bias
    detail: Training on dictionary-extracted examples produces a model biased toward the domains those dictionaries cover. Must enrich dictionaries with cross-domain terms BEFORE building training data, not after.
    tag: models

  - title: Network volume must be namespaced by purpose
    detail: DRRP, significance, and fitness models/scripts/adapters were dumped flat in /workspace/. Created models/drrp/, models/fitness/, models/significance/ to prevent overwriting.
    tag: infrastructure

  - title: GGUF on /tmp is lost on pod stop — copy to /workspace immediately
    detail: llama-quantize writes to /tmp to avoid network mount IO errors. The Q4 GGUF must be copied to /workspace before the pod stops. Lost 2.4GB GGUF and had to re-quantise.
    tag: infrastructure

artifacts:
  - scripts/ml/finetune_fitness_16bit.py
  - scripts/ml/runpod_fitness_batch.py
  - crates/fractalaw-cli/src/commands/fitness.rs
  - .claude/skills/runpod-batch-inference/SKILL.md
  - .claude/skills/runpod-finetune/SKILL.md
  - models/gemma3-fitness-q4.gguf
  - data/ml/fitness_train.jsonl
  - data/ml/fitness_test.jsonl

depends_on:
  - 07-11-26-fitness-cross-domain-extraction.md
  - 07-11-26-fitness-entity-catalogue.md

enables:
  - Reconciliation step (merge regex + slm + ft tiers into final entities column)
  - Phase 6 rules engine (clean ft_entities for expression tree compilation)
  - Entity feedback loop (promote high-frequency ft entities to dictionaries)
  - Base vs fine-tuned quality comparison (both tiers preserved)
---

# Session: Fitness SLM Fine-Tune (CLOSED)

## Problem

Phase 5 of FITNESS-STRATEGY.md: the base gemma3:4b produced ~40% entity noise during extraction — procedural terms ("claim", "payment", "instrument") alongside genuine applicability subjects ("employer", "marine conservation zone"). The rules engine can't distinguish good from noisy entities; false positives in Match nodes cause incorrect law-customer matches.

7,325 dictionary-extracted mentions provide clean training data — the dictionaries are curated, so their entities are known-correct. Fine-tuning on these should teach the model what a correct fitness entity looks like.

## Work

1. ✅ Enriched dictionaries first (feedback loop): added cross-domain terms (vehicle, building, land, dwelling, LLP, motor vehicle, body corporate, building work, licence) to core dictionaries
2. ✅ Re-ran regex extraction: training examples grew 7,325 → 8,706 (+19% diversity)
3. ✅ Exported training data as JSONL with scope dimensions from entity catalogue. Train: 7,835 / Test: 871
4. ✅ Uploaded to RunPod RTX 5090 (32GB). Training: 45 min, 2,940 steps, loss 0.56→0.25.
5. ✅ Eval: precision 93.3%, recall 91.7%, F1 92.5%, scope accuracy 100% (vs ~60% base model)
6. ✅ GGUF: BF16 + Q4_K_M exported. BF16 persisted on network volume. Q4 download interrupted (1.2/2.4GB) — re-quantise from BF16 on next pod attach.
7. ✅ GGUF downloaded (2.3GB) and backed up to NAS. Re-quantised from BF16 on network volume after pod restart.
8. ✅ Loaded as gemma3-fitness in Ollama. Full extraction: 14,248 provisions, 9.3/s, 99.98% success. Written to ft_entities column.
9. ✅ Three tiers now comparable: regex (8,706), slm (6,604 restored from NAS backup), ft (14,255).
10. ⏸️ Re-propagate with cleaner entities (deferred — needs reconciliation step to merge tiers first)

## Dependencies

- ✅ 7,325 mentions with curated regex_entities (training data)
- ✅ `/runpod-finetune` skill for LoRA training + GGUF export
- ✅ `/runpod-batch-inference` skill for re-extraction
- ✅ fitness_mentions per-tier columns (slm_entities independent of regex_entities)
- ⬜ RunPod pod with RTX 5090 (32GB for 16-bit training)
