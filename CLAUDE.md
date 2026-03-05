# Fractalaw

Local-first fractal architecture for ESH (environment, safety, health) regulatory data.

## Project Structure

Rust workspace monorepo with 6 crates:

- `fractalaw-core` — Arrow schemas, shared types (pure Rust, no optional deps)
- `fractalaw-store` — DuckDB, LanceDB, DataFusion integration (feature-gated)
- `fractalaw-ai` — ONNX Runtime embeddings/classification (feature-gated)
- `fractalaw-sync` — Arrow Flight sync, Lance delta sync, Loro CRDTs (flight feature-gated)
- `fractalaw-host` — Wasmtime WASI Component Model runtime
- `fractalaw-cli` — Binary entry point (`fractalaw` binary), enables DuckDB + DataFusion

WIT interfaces live in `/wit/` (fractal:data, fractal:ai, fractal:events, fractal:audit).

Session logs live in `/.claude/sessions/` — one markdown file per working session documenting decisions, progress, and next steps.

## Build

The workspace build requires the C/C++ toolchain (brew gcc) because `fractalaw-cli` enables DuckDB + DataFusion. `.cargo/config.toml` configures `CC`, `CXX`, and `LIBRARY_PATH` automatically.

```bash
# Workspace build (requires C toolchain — .cargo/config.toml handles paths)
cargo check --workspace
cargo test --workspace

# Pure-Rust crates only (no C toolchain needed)
cargo check -p fractalaw-core
cargo test -p fractalaw-core

# With additional native deps
cargo check -p fractalaw-store --features full
cargo check -p fractalaw-ai --features onnx
cargo check -p fractalaw-sync --features flight

# Run the CLI
cargo run -p fractalaw-cli -- stats
cargo run -p fractalaw-cli -- law UK_ukpga_1974_37
cargo run -p fractalaw-cli -- query "SELECT name, year FROM legislation LIMIT 10"

# Taxa enrichment and QA
cargo run -p fractalaw-cli -- taxa enrich              # enrich all unenriched laws
cargo run -p fractalaw-cli -- taxa enrich --force      # re-enrich all (recomputes taxa_hash)
cargo run -p fractalaw-cli -- taxa qa                  # QA report for all enriched laws
cargo run -p fractalaw-cli -- taxa qa --laws UK_uksi_1999_3242
cargo run -p fractalaw-cli -- taxa qa --family "OH&S: Occupational"
```

## Feature Gates

Heavy C/C++ dependencies are behind optional features on library crates. The CLI binary enables what it needs:

| Crate | Feature | Dependencies |
|-------|---------|-------------|
| fractalaw-store | `duckdb` | duckdb (bundled C++) |
| fractalaw-store | `lancedb` | lancedb |
| fractalaw-store | `datafusion` | datafusion |
| fractalaw-store | `full` | all of the above |
| fractalaw-ai | `onnx` | ort (ONNX Runtime) |
| fractalaw-sync | `flight` | arrow-flight, tonic, prost |

## Conventions

- Edition 2024, resolver v2
- License: AGPL-3.0-or-later
- Arrow is the universal in-memory format — all data exchange uses Arrow RecordBatch
- Error handling: `thiserror` for library errors, `anyhow` for application/CLI errors
- Async runtime: tokio
- Logging: tracing
- Tests live next to source (`#[cfg(test)] mod tests`)

## Data Stores

Law metadata lives in **DuckDB** (`legislation` table, queried via `fractalaw query`). Per-provision text and taxa enrichment data lives in **LanceDB** (`legislation_text` table, NOT accessible via the CLI query command). See `.claude/skills/lancedb-validation/SKILL.md` for query patterns, cross-referencing workflows, and pyarrow recipes.

## Taxa Pipeline

The taxa enrichment pipeline extracts structured DRRP (Duties, Rights, Responsibilities, Powers) from provision text:

1. **Enrich** (`taxa enrich`): Reads provision text from LanceDB, runs `parse_v2()` per provision (purpose classification → skip gates → clause decomposition → DRRP extraction), aggregates per-law results into DuckDB `legislation` table. Uses `taxa_hash` / `published_hash` for change tracking.
2. **QA** (`taxa qa`): Re-runs `parse_v2()` live and produces a 4-section validation report: Coverage Summary, Purpose Distribution (with anomaly flags), Gate Analysis (skip_drrp sub-gates + descriptive_summary), and Anomalies. Filters by `--laws` or `--family`.
3. **Publish** (`sync publish`): Sends enriched DuckDB taxa to sertantai via Zenoh. `--changed` publishes only laws where `taxa_hash != published_hash`.

Key modules: `fractalaw-core/src/taxa/` (purpose classification, clause structure, DRRP parsing), `fractalaw-cli/src/main.rs` (enrich/qa/eyeball commands).

## Environment

- OS: Fedora Bluefin DX (atomic/immutable Linux)
- Rust: installed via rustup (userspace)
- WASM: wasm32-wasip1 + wasm32-wasip2 targets, cargo-component, wasm-tools
- C/C++ tools: brew (gcc, cmake, protobuf) — required for workspace build (see `.cargo/config.toml`)
- IDE: Zed (Flatpak)
