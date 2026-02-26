# Session: 2026-02-26 — Taxa Parser Refinement

## Context

**Phase**: 3 (MicroApp Runtime) — Refinement
**Parent session**: [02-24-26-drrp-parsing.md](02-24-26-drrp-parsing.md)
**Objective**: Refine the taxa classification pipeline — both regex patterns and AI polish — to improve precision and reliability.

## Background

The Phase C validation (parent session) successfully:
1. Migrated taxa regex parsing from Elixir to Rust (`fractalaw-core::taxa`)
2. Built DRRP polisher guest component with ONNX local inference
3. Wired LanceDB-only polisher pipeline (no DuckDB dependencies)
4. Validated infrastructure: WASM guest → Rust host → ONNX Runtime (100% local)
5. Generated evaluation report comparing regex vs AI clause quality

**Current state**: The **stack is proven**. Both regex and AI pipelines work end-to-end. Now we need to **finesse the technique** — improve pattern quality, model training, and clause extraction accuracy.

## Evaluation Summary (Phase C, 2026-02-26)

**Dataset**: 172 provisions with both taxa (regex) and AI polish (ONNX) across 9 laws

**Taxa (Regex) Strengths**:
- Fast processing (~10-50ms per provision)
- Comprehensive DRRP taxonomy: types, actors, duty family, POPIMAR, purposes
- Pattern-based confidence scores (avg 0.55-0.60)
- Clause extraction: 200-3000+ chars per provision
- Good for initial structural analysis

**AI Polish (ONNX) Current Issues**:
- **CRITICAL**: Severely truncating clauses (5-320 chars vs taxa 200-3000+)
- Common outputs: "shall" (5 chars), "the scottish ministers must" (27 chars)
- Root cause: Span extraction underfitting
  - Trained on only 200 examples (7K+ available)
  - Max sequence length: 128 tokens (too short for legislative text)
  - Only 3 epochs (validation loss still decreasing)
  - Clause span accuracy: 60.5%

**Infrastructure Validated**:
- WASM guest → Rust host → ONNX Runtime pipeline: ✅
- 100% local inference (0 API calls): ✅
- LanceDB query/mutation routing: ✅
- Model loading and inference: ✅

**Problem is purely model quality, not architecture.**

## Refinement Opportunities

### 1. Taxa Regex Patterns

**Current implementation**: `crates/fractalaw-core/src/taxa/` (18 modules migrated from Elixir)

**Known issues from evaluation**:
- Some clauses too verbose (include preamble, context)
- Actor extraction sometimes misses nuanced categories
- Confidence scoring could be more granular
- Purpose classification overlaps (multiple purposes per clause common)

**Improvement areas**:
- Refine modal verb window boundaries (currently fixed-width around "shall"/"must"/"may")
- Add more actor patterns for specialized roles (inspectors, commissioners, etc.)
- Improve clause refinement to strip boilerplate ("For the purposes of...", "Subject to...")
- Better handling of nested clauses (subclauses with their own modal verbs)
- Add pattern variants for Scottish/Northern Ireland legislative style

**Files to modify**:
- `crates/fractalaw-core/src/taxa/duty_type_lib.rs` — Core engine for modal search
- `crates/fractalaw-core/src/taxa/clause_refiner.rs` — Clause extraction window logic
- `crates/fractalaw-core/src/taxa/actor_definitions.rs` — Actor pattern library
- `crates/fractalaw-core/src/taxa/duty_type_defn_*.rs` — Duty type pattern sets

### 2. AI Model Training

**Current model**: DistilBERT, 200 examples, 3 epochs, max_length=128, INT8 quantized to 63.7 MB

**Critical improvements needed**:
1. **Dataset size**: Use full 7K+ silver-labeled provisions (not 200)
2. **Model architecture**: Switch to DeBERTa-v3-base (better for legal text)
3. **Sequence length**: Increase to 512 tokens (handle longer provisions)
4. **Training epochs**: 5-10 epochs until convergence
5. **Training supervision**: Add taxa `clause_refined` as additional signal for span extraction
6. **Loss weighting**: Increase clause span loss weight (currently 1.0, others 0.5/0.3)

**Training script**: `scripts/train_drrp_model.py`
**Export script**: `scripts/export_onnx.py`
**Model directory**: `models/deberta-v3-drrp/`

**Training data generation**:
```bash
# Export fresh training data from LanceDB (grows as more laws scraped)
cargo run -p fractalaw-cli -- export-training-data --output data/drrp_training.parquet

# Current: 7,019 matched provisions (6.4% LAT coverage)
# As more laws scraped: 50K+ provisions possible
```

### 3. Evaluation Methodology

**Current evaluation**: Side-by-side manual comparison of 172 provisions

**Quantitative metrics to add**:
- Clause extraction precision/recall (taxa vs ground truth)
- Holder classification accuracy (AI vs taxa categories)
- F1 score for DRRP type classification
- Inter-annotator agreement (if manual labels available)
- Clause length distribution (detect over/under extraction)

**Evaluation script**: `scripts/evaluate_polisher.sh` (basic version exists)

**Proposed evaluation framework**:
1. Manual annotation of 50-100 provisions as gold standard
2. Automated metrics: precision, recall, F1 for each component
3. Qualitative review: readability, coherence, usefulness
4. A/B testing: taxa-only vs taxa+AI for downstream tasks

## Architecture Context

### Taxa Pipeline (Rust, Pure Regex)

```
Legislative text (LanceDB)
  ↓
Text cleaner (normalize whitespace, strip HTML)
  ↓
Duty type classifier (Government/Governed/Unknown)
  ↓ (parallel)
  ├─ Actor extraction (32 patterns: 16 gov + 16 governed)
  ├─ POPIMAR classifier (7 categories: Policy/Organising/Planning/etc.)
  ├─ Purpose classifier (enforcement/exemption/definitional/procedural/etc.)
  ├─ Making detector (10 signal patterns for regulation-making clauses)
  └─ Clause refiner (modal verb window → focused clause text)
  ↓
Taxa classification record (10 fields)
  ↓
Write to LanceDB (merge on section_id)
```

**CLI command**:
```bash
cargo run -p fractalaw-cli -- taxa enrich
```

### AI Polisher Pipeline (WASM Guest + ONNX Runtime)

```
Provisions with taxa data (LanceDB)
  ↓
WASM guest: drrp-polisher (WIT: fractal:data)
  ↓
Query LanceDB (via host function)
  ↓
For each provision:
  - Build prompt: drrp_type, holder, source_text, article
  - Call ai-inference::generate (via host function)
  ↓
Rust host: route to ONNX extractor (if attached)
  ↓
ONNX Runtime: DeBERTa INT8 inference
  - 6 outputs: clause_start, clause_end, qualifier_start, qualifier_end, has_qualifier, holder_logits
  ↓
Decode spans: argmax → tokenizer.decode() → refined clause text
  ↓
Guest writes back to LanceDB (ai_holder, ai_clause, ai_qualifier, ai_confidence, ai_model)
```

**CLI command**:
```bash
cargo run -p fractalaw-cli -- run guests/drrp-polisher/target/wasm32-wasip1/debug/drrp_polisher.wasm
```

### Data Storage

**LanceDB** (`data/lancedb/legislation_text.lance`): 77,598 rows, 47 columns
- Source text: `law_name`, `section_id`, `provision`, `text`
- Taxa outputs (10 cols): `drrp_types`, `governed_actors`, `government_actors`, `duty_family`, `duty_sub_type`, `popimar`, `purposes`, `clause_refined`, `taxa_confidence`, `taxa_classified_at`
- AI outputs (7 cols): `ai_holder`, `ai_clause`, `ai_qualifier`, `ai_clause_ref`, `ai_confidence`, `ai_model`, `ai_polished_at`

**DuckDB** (`data/fractalaw.duckdb`): Analytical queries, law-level aggregates
- `legislation` table: 1,606 laws with taxa summary stats
- Lance extension: Query LanceDB via SQL (`SELECT * FROM lance.legislation_text WHERE ...`)

## Current Coverage

**Total provisions in LanceDB**: 77,598
**Provisions with taxa data**: 676 (from 5 laws)
**Provisions with AI polish**: 172 (25.4% of taxa-enriched)

**Laws with taxa data** (from 12-law sample):
- UK_asp_2019_15 (Climate Change Scotland): 152 provisions
- UK_nisr_2014_301 (Domestic Renewable Heat): 200 provisions
- UK_nisr_2015_387 (CO2 Storage Licensing): 160 provisions
- UK_nisr_2015_388 (CO2 Storage Access): 112 provisions
- UK_nisi_2003_419 (Energy NI Order): 52 provisions

**Major UK ESH laws WITHOUT taxa data** (exist in LanceDB as raw text):
- UK_ukpga_1974_37 (HSWA 1974)
- UK_uksi_1999_3242 (MHSWR 1999)
- UK_uksi_1989_635 (Electricity at Work 1989)
- UK_uksi_2015_51 (CDM 2015)
- UK_uksi_2002_2677 (COSHH 2002)
- UK_uksi_1998_2307 (LOLER 1998)
- UK_uksi_1998_2306 (PPEWR 1992)

**Next step**: Run `taxa enrich` on these 7 major laws to expand training/evaluation dataset.

## Session Goals

This session focuses on **refinement and improvement**, not building new infrastructure. The stack works — now we tune it.

**Priorities**:
1. **Run taxa enrichment** on 7 major UK ESH laws (HSWA, MHSWR, Electricity at Work, CDM, COSHH, LOLER, PPEWR)
2. **Analyze taxa regex output** for these laws — identify common failure modes
3. **Refine regex patterns** based on findings (clause boundaries, actor extraction, etc.)
4. **Prepare GPU training run**: Export full training dataset, update training script for 512 tokens + DeBERTa-v3-base
5. **Build quantitative evaluation**: Metrics framework for precision/recall/F1

**Out of scope** (for this session):
- New WIT interfaces or host functions (infrastructure is complete)
- DuckDB schema changes or Phase D export pipeline
- New guest components or WASM modules
- Zenoh/pub-sub integration (Phase 4 concern)

## Key Files Reference

### Taxa Implementation (Rust)
- `crates/fractalaw-core/src/taxa/mod.rs` — Public API: `parse()`, `TaxaClassification`
- `crates/fractalaw-core/src/taxa/taxa_parser.rs` — Orchestrator
- `crates/fractalaw-core/src/taxa/duty_type_lib.rs` — Core modal search engine
- `crates/fractalaw-core/src/taxa/clause_refiner.rs` — Clause extraction
- `crates/fractalaw-core/src/taxa/actor_lib.rs` — Actor extraction engine
- `crates/fractalaw-core/src/taxa/actor_definitions.rs` — 32 actor patterns
- `crates/fractalaw-core/src/taxa/duty_type_defn_*.rs` — Duty type patterns (3 files)
- `crates/fractalaw-core/src/taxa/popimar*.rs` — POPIMAR classifier (2 files)
- `crates/fractalaw-core/src/taxa/purpose_classifier.rs` — Purpose patterns
- `crates/fractalaw-core/src/taxa/making_detector*.rs` — Making signal detector (2 files)
- `crates/fractalaw-core/src/taxa/text_cleaner.rs` — Text normalization
- `crates/fractalaw-core/src/taxa/duty_actor.rs` — Actor struct

### AI Training & Inference
- `crates/fractalaw-ai/src/extractor.rs` — ONNX inference pipeline (`DrrpExtractor`)
- `scripts/train_drrp_model.py` — PyTorch training script (3-head model)
- `scripts/export_onnx.py` — ONNX export + INT8 quantization
- `models/deberta-v3-drrp/` — Model artifacts (weights, tokenizer, labels)

### CLI & Host
- `crates/fractalaw-cli/src/main.rs` — Entry point, `cmd_taxa_enrich()`, `cmd_run()`
- `crates/fractalaw-host/src/lib.rs` — Host runtime, ONNX routing, LanceDB queries

### Evaluation
- `scripts/evaluate_polisher.sh` — Basic evaluation report
- `data/evaluation_detailed_20260226.txt` — Side-by-side comparison (680 lines)
- `.claude/sessions/02-24-26-drrp-parsing.md` — Parent session with Phase A/B/C details

## Next Steps (Checklist)

- [ ] Run `taxa enrich` on 7 major UK ESH laws
- [ ] Generate taxa quality report for HSWA 1974 (spot-check provisions)
- [ ] Identify top 3 regex pattern improvement areas
- [ ] Refine clause refiner boundaries (modal verb window tuning)
- [ ] Update training script for 512 tokens + DeBERTa-v3-base
- [ ] Export full 7K+ training dataset
- [ ] Run GPU training (external machine)
- [ ] Evaluate new model on held-out test set
- [ ] Build quantitative metrics framework
- [ ] Document improvements and lessons learned

---

**Session started**: 2026-02-26
**Status**: Active
