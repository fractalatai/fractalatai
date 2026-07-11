---
session: Fitness SLM Extraction
status: closed
opened: 2026-07-11
closed: 2026-07-11
outcome: success

summary: >
  SLM batch extraction filled the dictionary gap: 6,604 provisions extracted via
  prompted gemma3:4b on RunPod RTX 5090 in 20 min. Entity coverage 35% → 97.7%.
  Fixed critical architecture flaw: per-tier columns (regex_entities, slm_entities)
  prevent overwrite. Added temporal date extraction for commencement provisions.
  Created /runpod-batch-inference skill with "verify writes before full batch" rule.

decisions:
  - what: Prompted base model, no fine-tuning
    why: gemma3:4b produces good extractions from prompting alone (98.98% success). Error rates flat across the full batch — no tail degradation on complex provisions.
    result: Fine-tuning deferred. 7,587 dictionary-extracted training pairs available if needed later.

  - what: Per-tier columns instead of single entities column with extraction_method tag
    why: Single column + --force deleted SLM data that regex would never re-create. Same architecture flaw DRRP fixed with regex_position/cls_position/slm_position. Each tier must write to its own column.
    result: regex_entities, slm_entities, llm_entities as independent columns. --force only clears regex tier.

  - what: Temporal date extraction as regex backfill step
    why: Commencement provisions ("comes into force on 30th November 2017") have polarity but no entity — the entity IS the date. Without it, the rules engine TimeWindow has no from date.
    result: 55 ISO dates extracted and stored with scope_dimension=temporal.

metrics:
  slm_batch:
    provisions: 6671
    success: 6603
    empty: 55
    json_error: 13
    success_rate: "98.98%"
    speed: "5.5/s"
    duration: "20 min"
    gpu: "RTX 5090"
    cost: "~$0.33"
  coverage:
    total_mentions: 14258
    regex_entities: 7325
    slm_entities: 6604
    temporal_dates: 55
    neither: 329
    coverage_rate: "97.7%"
  architecture_fix:
    data_lost_before_fix: 6604
    data_recovered_from_nas: 6604

lessons:
  - title: --force must be tier-scoped, never blanket delete
    detail: The original --force did DELETE FROM fitness_mentions with no tier filter. This destroyed 6,604 SLM results that took 20 min of GPU time. Same mistake DRRP made and fixed. Per-tier columns (regex_entities, slm_entities) make this structurally impossible.
    tag: architecture

  - title: Verify DB writes with --limit N before full batch, not --test
    detail: --test is a dry run that skips writes. It proves the extraction works but NOT that data lands in the DB. Must run --limit 10 WITHOUT --test and check the DB before committing to a full GPU batch. Added to /runpod-batch-inference skill.
    tag: methodology

  - title: Python -u flag required for nohup progress logging
    detail: Without -u (unbuffered), Python stdout buffers when redirected to a file via nohup. The log appears empty or frozen even while the script is processing. Always use python3 -u for batch scripts.
    tag: infrastructure

  - title: import torch hangs on some RunPod pods
    detail: GPU check via import torch can hang indefinitely. Use nvidia-smi subprocess instead — always returns within the timeout.
    tag: infrastructure

  - title: Commencement dates are temporal entities, not empty results
    detail: The SLM correctly returned empty for commencement provisions — there's no applicability entity, just a date. But the rules engine needs that date for TimeWindow evaluation. A simple regex date extractor (UK date formats) fills this gap as a backfill step.
    tag: data

artifacts:
  - scripts/ml/runpod_fitness_batch.py
  - crates/fractalaw-cli/src/commands/fitness.rs
  - crates/fractalaw-cli/Cargo.toml
  - .claude/skills/runpod-batch-inference/SKILL.md
  - .claude/skills/customer-batch-parse/SKILL.md

depends_on:
  - 07-11-26-fitness-cross-domain-extraction.md
  - 07-11-26-fitness-entity-catalogue.md

enables:
  - Phase 3 graph propagation (all provisions now have entities)
  - Phase 4 rule compiler (entities + scope dimensions available for expression tree compilation)
  - Entity feedback loop (promote high-frequency SLM entities to dictionaries)
  - Reconciliation step (merge regex + SLM tiers into final entities column)
---

# Session: Fitness SLM Extraction (CLOSED)

## Problem

Phase 2e of FITNESS-STRATEGY.md: use SLM to extract applicability entities from the 6,671 provisions where dictionaries found polarity but no entities. These provisions contain applicability language ("this Part applies to...", "subject to the provisions of this Act...") but the terms are too domain-specific or structurally complex for regex dictionaries.

Example gap provision (MCAA s.66): "For the purposes of this Part, it is a licensable marine activity to do any of the following — To deposit any substance or object within the UK marine licensing area..." — polarity detected, but no entity extracted because the specific activities (deposit, dredge, construct) aren't in the dictionaries as fitness terms (they're bespoke to this law).

The SLM approach mirrors the DRRP pipeline: send provision text + a structured question, get back entities with scope dimensions.

## Work

1. ✅ Design SLM prompt for fitness entity extraction (input: provision text + polarity, output: entities + scope dimensions as JSON array)
2. ✅ Validated prompt with Gemini Flash on 10 provisions (simple + complex benchmark provisions) — good quality, correct scope dimensions
3. ✅ Fixed Ollama locally (brew upgrade 0.30.6 → 0.31.1, llama-server binary restored)
4. ✅ Tested gemma3:4b base model on CPU — correct extraction at 4.3 tok/s. On RunPod GPU would be 50-100x faster.
5. ✅ Batch script written: `scripts/ml/runpod_fitness_batch.py` — same pattern as significance batch. Tested locally: 5/5 success, ~14s/provision on CPU. On GPU with 4 workers → ~40min for full batch.
6. ✅ Full batch on RunPod RTX 5090: 6,671 provisions, 5.5/s, 20 min. 6,603 success (98.98%), 55 empty, 13 JSON error.
7. ✅ 6,604 SLM mentions written to fitness_mentions. Entity coverage: 35% → 99.5% (67 remaining gap from empty/error).
8. ✅ No tail degradation — error rates flat throughout. Fine-tuning not needed for prompted extraction.
9. ⏸️ Promote high-frequency SLM-discovered entities to dictionaries (deferred — feedback loop, future session)
10. ✅ Created /runpod-batch-inference skill — "verify writes before full batch" rule
11. ✅ Fixed architecture: per-tier columns (regex_entities, slm_entities, llm_entities) prevent overwrite
12. ✅ Recovered SLM data from NAS backup after --force wipe
13. ✅ Added temporal date extraction: 55 commencement dates extracted as ISO dates with scope=temporal

## Decisions

- **Prompt-only batch first**: base gemma3:4b already produces good extractions from prompting. Fine-tune only if quality drops on the tail (bespoke/complex provisions).
- **RunPod not Gemini API**: SLM on GPU, not LLM via API. Same infrastructure as DRRP position/significance batches.
- **Training data available for fine-tune if needed**: 7,587 dictionary-extracted mentions with entities → training pairs.

## Dependencies

- ✅ Phase 2d: dictionaries expanded, 6,671 gap provisions identified (07-11-26-fitness-cross-domain-extraction)
- ✅ fitness_mentions table and `fitness extract` CLI operational
- ✅ Entity catalogue with 228 entries + scope dimensions
- ✅ Ollama 0.31.1 working locally with gemma3:4b (CPU, for testing)
- ⬜ RunPod pod available for GPU batch
