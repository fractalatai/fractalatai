# Session: Project Structure Review (ACTIVE)

## Problem

The fractalaw monorepo has grown organically from a single Rust workspace into a multi-service system. The folder structure, CLAUDE.md, and project boundaries haven't kept pace. Key tensions:

1. **Single CLAUDE.md covers everything** — the project is more than just the DRRP parsing service. The workspace includes sync infrastructure (Zenoh), WASM host runtime, AI models, store abstractions (DuckDB, LanceDB, PgStore), and CLI tooling. A single CLAUDE.md conflates all of these.

2. **Folder structure is flat** — `crates/`, `scripts/`, `data/`, `models/`, `docs/`, `.claude/sessions/` all sit at the top level. Sessions have subdirectories (`cascade/`, `store/`, `fitness/`, etc.) but the code doesn't mirror this separation.

3. **Architectural question** — should new services (e.g. a future REST API, a sertantai adapter, an edge sync daemon) live in this repo or in separate repos? The Rust workspace makes multi-crate easy, but the operational concerns (build times, disk usage, deployment) may argue for separation.

## Questions to answer

### 1. What are the logical services?
- **DRRP parsing engine** — fractalaw-core + fractalaw-ai (the classifier/embedder)
- **Provision store** — fractalaw-store (DuckDB, LanceDB, PgStore)
- **Sync infrastructure** — fractalaw-sync (Zenoh pub/sub, Arrow IPC)
- **WASM host** — fractalaw-host (wasmtime component runtime)
- **CLI** — fractalaw-cli (the glue that wires everything together)
- **Future**: REST API? Edge daemon? Sertantai adapter?

### 2. Should CLAUDE.md be split?
Options:
- Per-crate CLAUDE.md (each crate documents its own concerns)
- Per-service CLAUDE.md (parsing, sync, store)
- Top-level CLAUDE.md stays as index, crate-level ones add depth

### 3. Should the project be split into multiple repos?
Pros of monorepo: shared types (Arrow schemas), atomic cross-crate changes, single CI
Cons: 30+ GB target/, 6-minute builds, disk pressure, unrelated concerns coupled

### 4. Folder structure cleanup
- `scripts/` — mix of Python utilities, migration scripts, one-off tools
- `data/` — runtime data (DuckDB, LanceDB, audit logs, CSV lists) mixed with config
- `docs/` — actor dictionary, design docs
- `.claude/sessions/` — 60+ session files across 5 subdirectories
- `backups/` — gitignored but growing

## Review findings

### Current state (2026-06-25)

**Codebase**: 6 crates, 31,666 lines of Rust. Dependency graph is clean:
```
fractalaw-core (11.9K lines) — pure Rust, no optional deps
  ↑ used by all other crates

fractalaw-store (3.7K) — DuckDB, LanceDB, PgStore, DataFusion
fractalaw-ai (2.5K) — ONNX embeddings, DRRP classifier
fractalaw-sync (2.6K) — Zenoh pub/sub, Arrow IPC
  ↑ all depend on core only

fractalaw-host (1.3K) — Wasmtime WASI runtime (depends on store + ai)
fractalaw-cli (9.6K) — binary glue (depends on everything)
```

**Top-level directory assessment**:

| Dir | Purpose | Problem |
|-----|---------|---------|
| `crates/` | Rust workspace | Clean, well-structured |
| `data/` | Runtime data + CSV imports + documentation + Parquet seeds | **Mixed concerns**: 41 CSV law lists, 8 markdown docs, 5 Parquet seed files, runtime DBs, audit logs, benchmarks all in one dir |
| `scripts/` | 23 Python/Bash scripts | **Mixed lifecycle**: 7 one-off migrations, 3 maintenance, 4 ML training, 6 benchmarks, 2 schema — no organisation |
| `docs/` | Actor dictionary, design docs, classifier configs | Reasonable but schema docs split between `docs/` and `data/` |
| `models/` | ONNX models (gitignored) | Fine |
| `guests/` | WASM guest components | 4 test guests, no production use yet |
| `wit/` | WIT interface definitions | Single file, fine |
| `apps/` | Empty | Dead directory |
| `backups/` | Arrow/Parquet backups (gitignored) | Fine |
| `.claude/sessions/` | 81 session files across 6 dirs | Growing fast, valuable history |
| `.claude/skills/` | 17 skills | Useful operational knowledge |

### Problem areas

**1. `data/` is a dumping ground**
- CSV law lists (AMD-*, LAT-*, qq-applicable-laws.csv) are sertantai import mappings — not fractalaw config
- Markdown docs (ZENOH-LAT-ARROW-IPC.md, LAT-SCHEMA-FOR-SERTANTAI.md etc.) belong in `docs/`
- `data/code-review/` and `data/benchmarks/` are review artifacts
- Runtime data (DuckDB, LanceDB, llm-audit) mixed with checked-in seed data (Parquet)

**2. `scripts/` has no lifecycle management**
- 7 migration scripts are historical — ran once, never again
- Maintenance scripts (compact_lance.py) are active
- ML training scripts could be their own concern
- No README explaining which scripts are current

**3. Documentation is scattered**
- Schema docs split: `docs/SCHEMA.md`, `docs/SCHEMA-2.0.md`, `data/LAT-SCHEMA-FOR-SERTANTAI.md`
- Zenoh docs split: `docs/ZENOH-SYNC.md`, `data/ZENOH-LAT-ARROW-IPC.md`, `data/ZENOH-LAT-DELETION-SIGNAL.md`
- Actor dictionary docs: `docs/ACTOR-DICTIONARY.md` + `docs/actor-dictionary.yaml`

**4. CLAUDE.md covers too much**
- Build instructions, feature gates, conventions, data stores, taxa pipeline, environment — all in one file
- No per-crate documentation
- No separation between "what is this project" and "how to operate the pipeline"

**5. Architectural coupling in CLI**
- `fractalaw-cli` is 9.6K lines even after the module split
- It's the only binary — all commands (sync, taxa, classify, embed, validate, publish, WASM run) go through it
- The CLI depends on every crate with every feature enabled → 30+ GB target, 6-minute builds

### Architectural assessment

**Should we split repos?** Not yet. The crate dependency graph is clean. The heavy build comes from C/C++ deps (DuckDB, DataFusion, ONNX, wasmtime) not from code coupling. Splitting repos would duplicate the shared Arrow schemas in fractalaw-core.

**What would splitting gain?**
- Faster builds per-service (but we'd lose incremental compilation across crates)
- Independent deployment (not relevant yet — it's a single CLI binary)
- Clearer ownership (only matters with multiple developers)

**What would splitting cost?**
- Cross-crate type sharing becomes versioned dependency management
- Atomic changes across services become multi-repo PRs
- CI complexity multiplies

**Recommendation: keep monorepo, improve internal organisation.**

### Proposed structure

```
fractalaw/
├── CLAUDE.md                    # Project overview + architecture (slimmed down)
├── Cargo.toml                   # Workspace
├── crates/
│   ├── fractalaw-core/          # Arrow schemas, DRRP parser, types
│   │   └── CLAUDE.md            # Core crate docs (taxa pipeline, parsing rules)
│   ├── fractalaw-store/         # DuckDB, LanceDB, PgStore
│   ├── fractalaw-ai/            # ONNX embeddings, classifiers
│   ├── fractalaw-sync/          # Zenoh pub/sub
│   ├── fractalaw-host/          # WASM runtime
│   └── fractalaw-cli/           # CLI binary
│       └── CLAUDE.md            # CLI operations guide (enrichment, publish, QA)
├── docs/
│   ├── architecture/            # Schema docs, Zenoh protocol, cascade strategy
│   ├── operations/              # Runbooks (currently scattered as skills)
│   └── dictionaries/            # actor-dictionary.yaml, fitness dictionary
├── scripts/
│   ├── migrations/              # One-off migration scripts (historical)
│   ├── maintenance/             # compact_lance.py, backup_lancedb.py
│   └── ml/                      # Training, benchmarks, evaluation
├── data/
│   ├── seed/                    # Parquet seed data for first-run import
│   ├── sertantai/               # CSV law lists, LAT mappings (from sertantai)
│   └── (runtime: DuckDB, LanceDB, llm-audit — gitignored)
├── models/                      # ONNX models (gitignored)
├── wit/                         # WIT interfaces
└── guests/                      # WASM guest components
```

**Key changes:**
1. Per-crate CLAUDE.md for core + cli (the two that need operational docs)
2. `data/` split: `seed/` (checked in), `sertantai/` (import mappings), runtime (gitignored)
3. `scripts/` organised by lifecycle: `migrations/`, `maintenance/`, `ml/`
4. `docs/` organised by topic: `architecture/`, `operations/`, `dictionaries/`
5. Delete empty `apps/` directory
6. Top-level CLAUDE.md slimmed to architecture overview + pointers to crate docs

## Gemini feedback (2026-06-25)

1. **Monorepo: keep it.** Build issues come from monolithic CLI binary, not the repo structure. Splitting repos adds overhead not justified at this scale.

2. **data/ split: good.** Suggested adding `data/audit/` for llm-audit logs (distinct retention from runtime DBs).

3. **Split CLI into multiple binaries — "most impactful change".** Create separate `[[bin]]` entries or `apps/` crates. Example: `fractalaw-sync` only needs `core` + `sync`, avoiding ONNX/wasmtime/DuckDB. This directly addresses the 30GB target and 6-min builds.

4. **Risks: minimal.** Use `git mv` to preserve history. Phased approach. Update CLAUDE.md immediately.

5. **CLAUDE.md pattern: confirmed.** Top-level for architecture, per-crate for operations. `docs/operations/` for how-to guides.

### Key insight from Gemini

The **CLI binary split** is the highest-impact change. Current state: one binary enables every feature (DuckDB + DataFusion + LanceDB + PgStore + ONNX + Zenoh + wasmtime). Splitting into:
- `fractalaw` (or `fractalaw-taxa`) — core pipeline: parse, embed, classify, validate (needs core + store + ai)
- `fractalaw-sync` — zenoh sync: watch, publish, pull-lat (needs core + sync + store)
- `fractalaw-host` — WASM runtime (needs core + host)

Each binary only compiles its deps. A `fractalaw-sync` build wouldn't touch ONNX or wasmtime.

## Scope

This session is a review + plan, not an execution. Output: a proposed structure and decisions on the architectural questions above.
