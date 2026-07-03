# fractalaw-core

Pure Rust crate — no optional deps, no C/C++ toolchain needed. Foundation for all other crates.

## What's Here

### Arrow Schemas (`src/arrow_schema.rs`)
Canonical Arrow schemas for `legislation` (LRT) and `legislation_text` (LAT) tables. All data exchange uses Arrow RecordBatch.

### Taxa Pipeline (`src/taxa/`)
The DRRP (Duties, Rights, Responsibilities, Powers) extraction engine:

- `purpose.rs` — Purpose classification (Substantive, Procedural, Definitional, etc.)
- `clause.rs` — Clause decomposition for multi-duty provisions
- `drrp.rs` — DRRP type extraction from classified clauses
- `actors.rs` — Actor matching from `data/actor-dictionary.yaml` (compiled in via `include_str!`)
- `correlatives.rs` — Hohfeldian correlative inference from `data/correlative-rules.yaml`
- `duty_patterns.rs` — Regex patterns for duty/right/power extraction
- `hierarchy.rs` — Structural hierarchy significance derivation

### Key Design

- `parse_v2()` is the main entry point: purpose → skip gates → clause decomposition → DRRP extraction
- Actor dictionary and correlative rules are **embedded at compile time** — the YAML files live at `data/actor-dictionary.yaml` and `data/correlative-rules.yaml` within this crate
- All types are `Send + Sync` for async pipeline use
