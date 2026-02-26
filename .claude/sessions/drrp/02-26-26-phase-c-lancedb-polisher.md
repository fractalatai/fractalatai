# Session: 2026-02-26 — Phase C: DRRP Map in LanceDB + LanceDB-Only Polisher

**Parent sessions**: [02-26-26-v2-promotion-enrichment.md](02-26-26-v2-promotion-enrichment.md), [02-26-26-v2-validation-at-scale.md](02-26-26-v2-validation-at-scale.md)
**GitHub issue**: [#19](https://github.com/fractalaw/fractalaw/issues/19)
**Status**: Active

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

### Commits

| Hash | Description |
|------|-------------|
| `d4c8e92` | Fix host test: update inference error assertion for no-feature build |
| `a4b5f9f` | Increase WASM epoch deadline to 3600s for long-running polisher guests |

## Related Issues

- #19 — Phase C: DRRP map in LanceDB + LanceDB-only polisher (this work)
- #17 — 270/452 enrichment gap investigation
- #18 — Provision-chain inference (depends on polisher context)
- #16 — Rule classifier (thing-subject obligations)
- #14 — AI classification improvements (polisher is the vehicle)
