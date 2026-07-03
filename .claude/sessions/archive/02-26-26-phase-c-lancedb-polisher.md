---
session: 'Phase C: DRRP Map in LanceDB + LanceDB-Only Polisher'
status: closed
opened: 2026-02-26
closed: 2026-02-26
outcome: success
summary: 'Completed and validated Phase C pipeline end-to-end: rebuilt WASM guest for LanceDB-only operation, fixed host test
  and epoch deadline, ran polisher with ONNX inference on 340 provisions. Concluded that ONNX adds no value for clause extraction
  -- regex pipeline already produces better clauses (avg confidence 0.49 vs 0.37).

  '
decisions:
- what: ONNX adds no value for polisher clause extraction
  why: Head-to-head comparison showed ONNX worse 71.5% of the time; outputs are truncated stubs
  result: Future improvements should focus on regex clause quality, not AI refinement
- what: Increase WASM epoch deadline from 100s to 3600s
  why: Long-running polisher guests processing hundreds of provisions exceeded the original timeout
  result: 340 provisions processed without timeout
- what: clause_refined should be null for non-DRRP provisions
  why: Fallback to cleaned_text inflates taxa data count; 50k+ non-DRRP provisions get section headings as clause_refined
  result: Observation logged for future fix
lessons:
- title: DeBERTa classifier trained for holder extraction, not clause extraction
  detail: Model truncates instead of extracting meaningful text; 63.7MB model produces 3-20 char stubs
  tag: ml
- title: OFFSET pagination with shrinking result sets is fragile
  detail: As rows get polished, WHERE ai_clause IS NULL shrinks, shifting offsets; works but inefficient
  tag: architecture
metrics:
  provisions_with_taxa: 18139
  provisions_polished: 340
  provision_errors: 8
  empty_drrp_skips: 1820
  api_tokens_used: 0
  regex_avg_confidence: 0.49
  onnx_avg_confidence: 0.37
  onnx_worse_pct: 71.5%
artifacts:
- guests/drrp-polisher/src/lib.rs
- crates/fractalaw-host/src/lib.rs
- crates/fractalaw-store/src/lance.rs
- crates/fractalaw-cli/src/main.rs
depends_on:
- 02-26-26-v2-promotion-enrichment
enables:
- 02-26-26-clause-quality-improvement
---


# Session: 2026-02-26 — Phase C: DRRP Map in LanceDB + LanceDB-Only Polisher (CLOSED)

**Parent sessions**: [02-26-26-v2-promotion-enrichment.md](02-26-26-v2-promotion-enrichment.md), [02-26-26-v2-validation-at-scale.md](02-26-26-v2-validation-at-scale.md)
**GitHub issue**: [#19](https://github.com/fractalaw/fractalaw/issues/19)
**Status**: Complete

## Objective

Complete and validate the Phase C pipeline: taxa enrichment writes per-provision DRRP data to LanceDB, then the WASM polisher guest reads text + taxa context from LanceDB and writes AI-refined results back — no DuckDB in the polisher path.

## Current State (What's Already Built)

Most of Phase C was implemented in earlier sessions (`f60ce6e`, `7782232`):

| Step | Description | Status |
|------|-------------|--------|
| 1. Schema | 17 DRRP + AI columns on `legislation_text_schema()` (47 fields total) | Done |
| 2. LanceStore methods | `update_taxa()`, `update_polished()`, `query_unpolished()` | Done |
| 3. `taxa enrich` → LanceDB | `cmd_taxa_enrich()` writes per-provision taxa via `lance.update_taxa()` | Done |
| 4. Host LanceStore routing | `lancedb` feature, query/mutation routing for `legislation_text` | Done |
| 5. Polisher guest rewrite | LanceDB-only, provision-level, no DuckDB queries | Done |
| 6. CLI wiring | LanceStore in `RunOptions`, `cmd_run()` passes lance to host | Done |
| 7. WASM rebuild | Guest source changed after last build (Feb 25 15:46 vs commit `7782232` Feb 26 06:47) | **Needs rebuild** |
| 8. End-to-end test | `taxa enrich` → `run drrp-polisher.wasm` → verify AI results in LanceDB | **Not done** |

## What Remains

### 1. Rebuild WASM guest

```bash
cd guests/drrp-polisher && cargo component build --release
```

The guest was **completely rewritten** for LanceDB-only mode (`f60ce6e`, `7782232`) — new data model (`ProvisionRow` vs old `Annotation`), queries `legislation_text` instead of `drrp_annotations`, writes `ai_*` columns back to LanceDB instead of inserting into `polished_drrp` in DuckDB. The compiled WASM on disk still has the old DuckDB code. Must rebuild before running.

### 2. Fix pre-existing host test

`tests::ai_tests::generate_without_config_errors` fails — assertion expects error message containing "ANTHROPIC_API_KEY" but gets a different message. Not Phase C-related but should be fixed.

### 3. End-to-end validation

Run the full pipeline:
1. Verify taxa data exists in LanceDB: `SELECT COUNT(*) FROM legislation_text WHERE drrp_types IS NOT NULL`
2. Run polisher: `fractalaw run guests/drrp-polisher/target/wasm32-wasip1/release/drrp_polisher.wasm`
3. Verify AI results written back: `SELECT COUNT(*) FROM legislation_text WHERE ai_clause IS NOT NULL`
4. Spot-check a few provisions: compare regex `clause_refined` vs AI `ai_clause`

### 4. Verify host query routing

Confirm the host correctly routes:
- `SELECT ... FROM legislation_text` → LanceDB
- `UPDATE legislation_text SET ai_* ...` → LanceDB
- `SELECT ... FROM legislation` → DuckDB (unchanged)

### 5. Test with ONNX inference (optional)

The polisher can use ONNX local-first inference. If an ONNX model is available, test that path. Otherwise, Claude API will be used (requires `ANTHROPIC_API_KEY`).

## Architecture Recap

```
┌──────────────────────────────────────────────────┐
│  LanceDB (legislation_text)                       │
│  ┌────────────────┬──────────────┬──────────────┐│
│  │ Source text     │ Taxa (regex) │ AI (polished)││
│  │ section_id     │ drrp_types   │ ai_holder    ││
│  │ law_name       │ governed_*   │ ai_clause    ││
│  │ text           │ clause_refined│ ai_qualifier ││
│  │ provision      │ taxa_confidence│ ai_confidence││
│  └────────────────┴──────────────┴──────────────┘│
└──────────────────────────────────────────────────┘
        ▲ write taxa        ▲ write AI         │ read
        │                   │                  │
   taxa enrich         polisher guest    polisher guest
   (CLI, Rust)        (WASM, via host)  (WASM, via host)
                            │
                    ┌───────┴───────┐
                    │ AI inference  │
                    │ ONNX / Claude │
                    └───────────────┘

DuckDB (legislation) ← law-level aggregates from taxa enrich (separate concern)
```

## Data Inventory

| Store | Table | Rows | DRRP data |
|-------|-------|------|-----------|
| LanceDB | `legislation_text` | 97,522 sections | 270 laws with taxa columns populated |
| DuckDB | `legislation` | 19,318 laws | 270 with v2 taxa aggregates |

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/schema.rs` | `legislation_text_schema()` — 47 fields including DRRP + AI columns |
| `crates/fractalaw-store/src/lance.rs` | `update_taxa()`, `update_polished()`, `query_unpolished()`, `query_legislation_text()` |
| `crates/fractalaw-host/src/lib.rs` | LanceStore routing: `lance_query_impl()`, `lance_execute_impl()`, `lance_to_json_result()` |
| `crates/fractalaw-cli/src/main.rs` | `cmd_taxa_enrich()` (writes to LanceDB), `cmd_run()` (passes LanceStore to host) |
| `guests/drrp-polisher/src/lib.rs` | LanceDB-only polisher: queries text+taxa, calls AI, writes ai_* back |
| `guests/drrp-polisher/src/ipc.rs` | Arrow IPC deserialization helpers for guest |

## End-to-End Results

### Tasks Completed

1. **WASM rebuilt** — guest recompiled with LanceDB-only code
2. **Host test fixed** — `generate_without_config_errors` assertion updated for no-feature build (`d4c8e92`)
3. **Epoch deadline increased** — 100s → 3600s for long-running guests (`a4b5f9f`)
4. **Pipeline validated** — full `run drrp-polisher.wasm` with ONNX inference

### Pipeline Output

| Metric | Value |
|--------|-------|
| Provisions with DRRP taxa | 18,139 |
| Provisions polished (ONNX) | 340 |
| Provision errors | 8 |
| Empty-drrp skips | 1,820 |
| Tokens used (Claude API) | 0 |
| AI model | onnx (DeBERTa v3) |

### Coverage Gap: 340 / 18,139 = 1.9%

The ONNX prompt parser (`parse_drrp_prompt()`) requires both a DRRP type and an actor from the prompt text. It silently falls through when it can't parse, and with no Claude API configured there's no fallback. Reasons for low coverage:

1. **Empty `drrp_types` lists matching `IS NOT NULL`** — taxa enrich writes `[]` (empty list) for non-DRRP provisions, which LanceDB treats as not null. The guest skips these client-side.
2. **`clause_refined` written for all provisions** — falls back to `cleaned_text` when no DRRP match, inflating the apparent taxa data count.
3. **ONNX model format mismatch** — the DeBERTa model was trained for holder/clause extraction but may not handle all prompt variations from the new Phase C format.

### Observations

- `clause_refined` in `cmd_taxa_enrich` uses `.unwrap_or_else(|| record.cleaned_text.clone())` — this writes section headings ("Citation and commencement", "Interpretation") as clause_refined for 50k+ non-DRRP provisions. Should be `None`/null for non-DRRP provisions.
- The polisher's OFFSET pagination is fragile: as rows get polished (`ai_clause` set), the `WHERE ai_clause IS NULL` result shrinks, shifting offsets. Works but inefficient.

### ONNX vs Regex Clause Quality Comparison

Applied `confidence::score()` to both `clause_refined` (regex) and `ai_clause` (ONNX) for the 340 polished provisions:

|                  | Regex `clause_refined` | ONNX `ai_clause` |
|------------------|----------------------|-------------------|
| High (>= 0.60)  | 174 (51.2%)          | 49 (14.4%)        |
| Medium (0.40-0.59) | 77 (22.6%)        | 149 (43.8%)       |
| Low (< 0.40)    | 89 (26.2%)           | 142 (41.8%)       |
| **Average**      | **0.49**             | **0.37**          |

**Head-to-head**: ONNX worse 71.5%, better 28.5%, same 0%.

Worst ONNX outputs are stubs: "the scottish ministers may", "the". The DeBERTa model is a classifier trained for holder extraction, not clause extraction — it truncates instead of extracting meaningful text.

**Conclusion**: ONNX adds no value for the polisher task. The regex pipeline with span-based extraction already produces better clauses. Future polisher improvements should focus on improving regex clause quality (sentence boundaries, clean endings) rather than AI refinement.

### Commits

| Hash | Description |
|------|-------------|
| `d4c8e92` | Fix host test: update inference error assertion for no-feature build |
| `a4b5f9f` | Increase WASM epoch deadline to 3600s for long-running polisher guests |
| `13ea81c` | Update Phase C session doc with end-to-end polisher results |

## Next Session

[02-26-26-clause-quality-improvement.md](02-26-26-clause-quality-improvement.md) — Improve regex clause extraction quality: fix mid-sentence starts, truncated endings, sentence boundary snapping.

## Related Issues

- #19 — Phase C: DRRP map in LanceDB + LanceDB-only polisher (this work)
- #17 — 270/452 enrichment gap investigation
- #18 — Provision-chain inference (depends on polisher context)
- #16 — Rule classifier (thing-subject obligations)
- #14 — AI classification improvements (polisher is the vehicle)
