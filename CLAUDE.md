# Fractalaw

Local-first fractal architecture for ESH (environment, safety, health) regulatory data.

## Project Structure

Rust workspace monorepo with 6 crates:

- `fractalaw-core` — Arrow schemas, DRRP parser, shared types (pure Rust, no optional deps). See `crates/fractalaw-core/CLAUDE.md`.
- `fractalaw-store` — DuckDB, LanceDB, PgStore, DataFusion (feature-gated)
- `fractalaw-ai` — ONNX Runtime embeddings/classification (feature-gated)
- `fractalaw-sync` — Zenoh pub/sub, Arrow IPC sync (feature-gated)
- `fractalaw-host` — Wasmtime WASI Component Model runtime
- `fractalaw-cli` — Binary entry point (`fractalaw` binary). See `crates/fractalaw-cli/CLAUDE.md`.

### Directory Layout

```
crates/           # Rust workspace crates
docs/
  architecture/   # Schema docs, Zenoh protocol, design plans
  operations/     # Runbooks, cascade strategy
  dictionaries/   # Classifier configs, dictionary docs
  manual/         # Customer-facing docs (significance methodology etc.)
  actor-dictionary.yaml    # Actor definitions (compiled into binary via include_str!)
  correlative-rules.yaml   # Hohfeldian correlative rules (compiled into binary)
scripts/
  migrations/     # Historical one-off migration scripts
  maintenance/    # Active: compact_lance, backup_lancedb, corpus_stats
  ml/             # Training, fine-tuning, ONNX export, RunPod batch
  benchmarks/     # Benchmark generation, evaluation, gold standard
  experiments/    # Significance aggregation approach scripts
  *.py            # Active operational scripts (gemini_*, actor_aliases)
data/
  sertantai/      # CSV law lists from sertantai (LAT-*, AMD-*, qq-applicable-laws)
  seed/           # Parquet seed data for first-run import
  audit/          # LLM audit log JSON files
  benchmarks/     # Tier2 benchmark Parquet files
  code-review/    # Gemini review outputs
  (runtime)       # DuckDB, LanceDB, models — gitignored
wit/              # WIT interface definitions
guests/           # WASM guest components
```

Session logs live in `.claude/sessions/` — one markdown file per working session.

## Build

The workspace build requires the C/C++ toolchain (brew gcc) because `fractalaw-cli` enables DuckDB + DataFusion. `.cargo/config.toml` configures `CC`, `CXX`, and `LIBRARY_PATH` automatically.

```bash
cargo check --workspace
cargo test --workspace

# Pure-Rust crates only (no C toolchain needed)
cargo check -p fractalaw-core

# Run the CLI
cargo run -p fractalaw-cli -- stats
cargo run -p fractalaw-cli -- law UK_ukpga_1974_37
cargo run -p fractalaw-cli -- query "SELECT name, year FROM legislation LIMIT 10"
```

## Feature Gates

Heavy C/C++ dependencies are behind optional features on library crates:

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

- **Postgres** (port 5433) — hub primary store: 183K+ provisions, embeddings, actors, significance ratings
- **DuckDB** (`data/fractalaw.duckdb`) — law-level LRT metadata, taxa aggregates, publish hashes
- **LanceDB** (`data/lancedb/`) — per-provision text with 384-dim embeddings (read-only for outbound data)

Data flow: LAT (provision text) → LanceDB → DRRP parser → taxa columns → Postgres → DuckDB (LRT) → Zenoh publish → sertantai.

## Environment

- OS: Fedora Bluefin DX (atomic/immutable Linux)
- Rust: installed via rustup (userspace)
- WASM: wasm32-wasip1 + wasm32-wasip2 targets, cargo-component, wasm-tools
- C/C++ tools: brew (gcc, cmake, protobuf) — required for workspace build (see `.cargo/config.toml`)
- IDE: Zed (Flatpak)
