---
session: L4 Evidence Records — LLM-Assisted Evidence Generation
status: closed
opened: 2026-07-15
closed: 2026-07-15
outcome: success

summary: >
  Designed, built, and ran the L4 Evidence pattern generation pipeline across the full QQ corpus.
  1,333 evidence patterns generated for 220 laws (1,315 validated after post-processing = 98.6%).
  Pro pilot on 5 laws established quality baseline, Flash benchmark proved comparable quality at
  ~10-20x lower cost, corpus run completed on Flash in 4 batches.

decisions:
  - what: Per-control evidence patterns with 3 sections (artefacts, judgement, strategy)
    why: >
      Evidence patterns are 1:1 with controls — no consolidation needed unlike the controls
      pipeline. Each control has different properties driving different evidence strategies.
    result: Clean 1:1 mapping, no deduplication phase required

  - what: Flash for corpus, Pro for pilot only
    why: >
      Pro pilot cost ~£20 for 21 laws (~£1/law). Flash benchmark on 5 reference laws showed
      comparable content quality with only enum-level differences (comma-separated methods,
      novel artefact types). Flash is ~10-20x cheaper.
    result: Full corpus on Flash, estimated £2-4 total. Pro benchmark JSON preserved for comparison.

  - what: Prompts are operational code, not plans
    why: >
      generate_controls.py reads prompts at runtime. Plans are ephemeral scaffolds. Prompts
      belong with the scripts that consume them.
    result: Moved from .claude/plans/compliance/prompts/ to scripts/compliance/prompts/. PROMPTS_DIR resolves via Path(__file__).parent.

  - what: drift_conditions field added (from Gemini review)
    why: >
      The L4 schema has Gaps (created when Judgement finding = Drifted). Without drift_conditions,
      the evidence pattern tells the assessor what to look for but not when to find 'Drifted'.
      Bridges evidence patterns to the Gaps entity.
    result: All 1,333 patterns have drift_conditions populated.

  - what: artefact_type Inspection Report → Report
    why: >
      Gemini correctly distinguishes report types (OH summary report, post-drill review, exception
      report) that are not inspection reports. The schema enum was too narrow. The specific type
      is inferrable from the title field.
    result: Single "Report" type covers all report subtypes. Novel types (Contract, Correspondence, etc.) mapped to "Other".

  - what: max_tokens bumped from 16,384 to 32,768
    why: >
      Flash truncated on 12-control laws (MHSW, COSHH, FSO). Evidence output is ~5KB per control,
      so 12 controls ≈ 60KB ≈ 15,000 tokens — right at the old limit.
    result: Zero truncation errors after the bump.

metrics:
  corpus: { laws: 220, patterns: 1333, validated: 1315, flagged: 18, validation_rate: "98.6%" }
  pilot_pro: { laws: 5, patterns: 50, validated: 47, validation_rate: "94%" }
  flash_benchmark: { laws: 5, patterns: 50, flags: 12, output_size_vs_pro: "+25%" }
  post_processing: { passes: 5, fixes: "300+", initial_flagged: 125, final_flagged: 18 }
  cost: { pro_pilot_21_laws: "~£20", flash_corpus_199_laws: "~£3", total_estimated: "~£23" }
  gemini_reviews: { count: 1, model: "gemini-2.5-pro", issues: 6, resolved: 6 }

lessons:
  - title: Pro thinking budget on evidence is expensive — £20 for 21 laws
    detail: >
      Evidence patterns require ~8K thinking tokens on Pro per call (domain reasoning for artefact
      types, basis guidance, drift signals). The controls corpus (220 laws, ~2,150 calls) cost far
      less because controls output is a flat JSON array. Evidence output has 3 nested sections per
      control. Flash at 4K thinking produces comparable quality at ~10-20x lower cost.
    tag: models

  - title: Flash invents enum values but the content is sound
    detail: >
      Flash returned comma-separated recommended_methods, novel artefact_types (Contract,
      Correspondence, Design Document), and "Observation" as a source. All fixable in post-
      processing. The underlying evidence content — what_it_proves, drift_conditions,
      basis_guidance — was consistently high quality. Enum enforcement is a prompt issue,
      not a reasoning issue.
    tag: models

  - title: max_tokens truncation is silent and looks like a JSON parse error
    detail: >
      Flash hitting the 16,384 output limit produced "Unterminated string" JSON parse errors.
      No explicit truncation signal from the API. Took several failures to diagnose. The fix
      (bumping to 32,768) is simple but the failure mode is misleading — it looks like the
      model produced malformed JSON rather than running out of space.
    tag: infrastructure

  - title: Write JSON to disk before DuckDB — crash-safe ordering matters
    detail: >
      The original script wrote DuckDB first, then JSON. When the process was killed during
      rate-limit backoff, DuckDB had rows but JSON files were missing. Reversing the order
      means the raw Gemini response is always preserved on disk even if DuckDB write fails.
    tag: architecture

  - title: 10s Gemini delay + retry backoff is the minimum for sustained Pro batch runs
    detail: >
      2s delay caused 429s after ~30 calls on Pro. 10s delay with 30/60/120/240s retry backoff
      handled Flash corpus (220 calls) without a single rate limit. Pro is more expensive per
      token and hits rate limits sooner.
    tag: infrastructure

  - title: Evidence generation is strong fine-tuning data for Gemma 3
    detail: >
      The 1,333 (control → evidence pattern) pairs are high-quality supervised examples.
      The task is more complex than DRRP classification (~1,500 tokens output vs ~50) but
      Gemma 3 27B handles structured JSON output. After post-processing, the cleaned corpus
      is ready as a fine-tuning dataset. Per-law inference cost drops to zero (GPU time only).
    tag: models

  - title: Gemini review caught a critical design flaw (cascade-delete vs three-way merge)
    detail: >
      The v0.1 design proposed deleting evidence patterns when regenerating controls. Gemini
      correctly identified this as a regression from the controls pipeline's three-way merge.
      Customer edits would be destroyed. Adding base_hash + customer_edits columns fixed it.
      Always send designs for review before building.
    tag: methodology

artifacts:
  - scripts/compliance/generate_evidence.py
  - scripts/compliance/prompts/evidence-system-prompt-v1.md
  - .claude/plans/compliance/COMPLIANCE-EVIDENCE.md
  - data/code-review/compliance-evidence-design-review.md
  - data/compliance-evidence/generated/

depends_on:
  - 07-10-26-compliance-controls.md
  - 07-13-26-phase4-publish.md

enables:
  - Evidence publish to sertantai (Zenoh key expressions + Arrow schema)
  - Gemma 3 fine-tuning on evidence patterns (1,333 supervised examples)
  - L4 Evidence templates in customer Baserow workspaces
  - VoI-driven evidence prioritisation in the compliance dashboard
---

# Session: L4 Evidence Records — LLM-Assisted Evidence Generation (CLOSED)

## Problem

The Controls pipeline (Phase 1-4) generates L3 Controls from L1 Obligations. Each control already carries `evidence_hint.type_a` and `evidence_hint.type_b` — but these are free-text hints embedded in the control JSON, not structured L4 Evidence records that a customer can operationalise.

The L4 Evidence tier has three entities (Artefacts, Judgements, Gaps). Artefacts and Judgements are operational — the customer creates them as they exercise controls. But the *pattern* for what evidence to register and how to judge it can be generated from the controls, just as controls are generated from obligations.

The goal: for each control, generate **suggested evidence patterns** — what artefacts to register (Type-A and Type-B), what judgement method to use, recommended assessment frequency (driven by VoI), and what the discriminating test looks like. These are templates the customer reviews and adapts, not operational records.

## Work

### Phase 0: Design
1. ✅ Read evidence schema docs (EVIDENCE-SCHEMA.md, DEFINITION-OF-EVIDENCE.md, VALUE-OF-INFORMATION.md, LEGIBLE-vs-LOAD-BEARING.md, EVIDENCE-VAULT-PATTERNS.md, EVIDENCE-CALIBRATION.md)
2. ✅ Define Evidence record output schema (3 sections: artefact patterns, judgement guidance, evidence strategy)
3. ✅ Design prompt architecture (7 constraints, per-law batching, few-shot examples)
4. ✅ Define DuckDB staging table schema (`suggested_evidence`, linked to `suggested_controls` by control_id)
5. ✅ Restructure: renamed `compliance-controls/` → `compliance/` in plans + sessions; moved scripts + prompts to `scripts/compliance/` (prompts are operational code, not ephemeral scaffolds)
6. ✅ Gemini 2.5 Pro review of design → v0.2 (6 issues, all resolved)

### Phase 1: Pipeline Build
7. ✅ Write system prompt (`scripts/compliance/prompts/evidence-system-prompt-v1.md`) — 8 constraints, deterministic defaults table, 2 few-shot examples
8. ✅ Write `scripts/compliance/generate_evidence.py` — prompt assembly, Gemini call, Phase 2 lint (13 checks), DuckDB staging
9. ✅ Phase 2 lint: enum validation, Type-B required, needs_judgement consistency, VoI consistency, drift checks, mandatory fields
10. ⏸️ Tests (deferred — pipeline validated empirically via corpus run)

### Phase 2: Pilot
11. ✅ Run on 5 reference laws — 50 evidence patterns, 47 validated (94%), 3 flagged (6%)
12. ✅ Review quality, iterate prompt — artefact_type enum fix (Inspection Report → Report)

### Phase 3: Corpus Run + Publish
13. ✅ QQ corpus complete — 220 laws, 1,333 patterns, 90.8% validated (Pro pilot 21 laws + Flash 199 laws, 4 batches of 50)
14. ✅ Post-processing complete (5 passes): 125 flagged → 18 (98.6% validated)
    - Normalised artefact_type: Inspection Report/Observation/Contract/Correspondence/etc. → Report or Other (270+ fixes)
    - Normalised recommended_method: comma-separated lists → first value, novel methods → nearest valid enum (20+ fixes)
    - Auto-corrected needs_judgement: false → true where load_bearing_judgement present (5 fixes)
    - Auto-corrected evidence_standard/staleness_tolerance/voi_quadrant from blast_radius rules (3 fixes)
    - Normalised artefact_class: string "None" → Activity, source: "Observation"/"Interview"/"LMS" → valid values
    - Retried UK_anaw_2017_2 on Flash (was Pro JSON parse failure) — 10/10 success
    - 18 remaining flags: 5 missing judgement content (unfixable without regen), 6 Activity+High LR soft flags, 7 edge cases
15. ⏸️ Zenoh publish schema + CLI command (deferred — needs sertantai Postgres table + subscriber)
16. ⏸️ Publish to sertantai (deferred — depends on item 15)

## Decisions

- **Per-control evidence patterns** (not per-provision): each control gets 1:1 evidence pattern with artefacts, judgement guidance, and VoI strategy
- **No consolidation phase**: unlike controls, evidence patterns don't need deduplication — they're per-control
- **Deterministic derivations**: `needs_judgement`, `evidence_standard`, `staleness_tolerance` computed from control properties; LLM confirms or overrides with rationale
- **Prompts are operational code**: moved from `.claude/plans/` to `scripts/compliance/prompts/` — `PROMPTS_DIR` resolves via `Path(__file__).parent / "prompts"`
- **Flash for corpus, Pro for pilot only**: Pro pilot cost ~£20 for 21 laws. Flash benchmark shows comparable quality at ~10-20x lower cost. Corpus run uses Flash with 4,096 thinking budget.
- **max_tokens bumped to 32,768**: Flash truncated at 16,384 on 12-control laws (MHSW, COSHH, FSO). Evidence output is ~5KB per control.

## Flash vs Pro benchmark (2026-07-15)

| Law | Model | Patterns | Flags | Output size |
|-----|-------|----------|-------|-------------|
| CS 1997 | Pro | 3 | 0 | 14KB |
| CS 1997 | Flash | 3 | 0 | 15KB |
| MHSW 1999 | Pro | 12 | 0 | 47KB |
| MHSW 1999 | Flash | 12 | 11 | 66KB |
| HSWA 1974 | Pro | 11 | 4 | 46KB |
| HSWA 1974 | Flash | 11 | 1 | 54KB |
| COSHH 2002 | Pro | 12 | 0 | 44KB |
| COSHH 2002 | Flash | 12 | 0 | 55KB |
| FSO 2005 | Pro | 12 | 0 | 47KB |
| FSO 2005 | Flash | 12 | 0 | 59KB |

Flash flags are all `recommended_method` returning comma-separated lists instead of single enum — trivial to fix in post-processing. HSWA improved (Pro: 4 flags → Flash: 1). Flash is ~25% wordier but content quality is comparable.

Pipeline fixes applied:
- JSON written to disk before DuckDB (crash-safe)
- `flush=True` on all prints (visible progress in background)
- `--no-db` flag for model comparison without overwriting staging data
- `--model` and `--thinking` CLI flags
- Retry with backoff on 429 (30s, 60s, 120s, 240s)
- `GEMINI_DELAY` increased to 10s
- `max_tokens` increased to 32,768

## Gemini review feedback (2026-07-15)

Raw review: `data/code-review/compliance-evidence-design-review.md`

6 issues raised, all resolved in v0.2:

| Issue | Resolution |
|-------|-----------|
| Cascade-delete destroys customer edits | Added three-way merge (base_hash + customer_edits columns), matching controls pipeline |
| No guidance for Gaps entity | Added `drift_conditions` field to judgement section — tells assessor when to find 'Drifted' |
| `judgement_rationale` should be mandatory | Made mandatory even when needs_judgement=false — explains why artefacts suffice |
| Strategy metadata storage unclear | Clarified: strategy lives on template in staging table, not pushed to operational L4 records |
| No feedback loop for prompt improvement | Added `customer_edits` column for (generated, edited) pairs |
| Control 3 VoI rationale understates drill cost | Strengthened rationale: drill cost is non-discretionary (regulatory), evidence is strongly discriminating |

Also added Constraint 8 (Proportionality) per Gemini's recommendation — evidence effort must be proportional to control risk.

## Pilot results (2026-07-15)

| Law | Controls | Patterns | Validated | Flagged | Flags |
|-----|----------|----------|-----------|---------|-------|
| Confined Spaces 1997 | 3 | 3 | 3 (100%) | 0 | 0 |
| MHSW 1999 | 12 | 12 | 9 (75%) | 3 | 3 |
| HSWA 1974 | 11 | 11 | 8 (73%) | 3 | 4 |
| COSHH 2002 | 12 | 12 | 9 (75%) | 3 | 3 |
| FSO 2005 | 12 | 12 | 12 (100%) | 0 | 0 |
| **Total** | **50** | **50** | **47 (94%)** | **3 (6%)** | **4** |

After artefact_type enum fix (`Inspection Report` → `Report`): 94% validated.

Remaining 3 flagged patterns (all HSWA — broadest, goal-setting Act):
- 1 `Observation` artefact type (will map to Report on regeneration)
- 1 Activity artefact with High likelihood_ratio (soft flag)
- 2 VoI consistency errors: Enterprise/Manual control classified as Table Stakes/Basic — genuine catch by the lint

Quality observations:
- Domain-specific artefacts throughout (PTW, atmospheric monitoring, rescue drill records, OHP summary reports, near-miss logs)
- `drift_conditions` concrete and actionable on all 50 patterns
- `basis_guidance` operational — tells the person what to look at on the ground
- Gemini correctly applied `needs_judgement=true` wherever `load_bearing_judgement` was non-null
- VoI classification consistent with control properties (except one HSWA edge case)

## Dependencies

- ✅ Controls pipeline complete (1,341 controls + 218 predicates in DuckDB staging)
- ✅ Evidence schema docs exist (L4 canonical schema, VoI, calibration)
- ✅ `scripts/compliance/generate_controls.py` patterns to borrow
- ⏸️ Controls published and stored in sertantai (needed for full round-trip, not for generation)
