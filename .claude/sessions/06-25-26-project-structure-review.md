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

~~This session is a review + plan, not an execution. Output: a proposed structure and decisions on the architectural questions above.~~

Updated 2026-07-02: promoted to execution session.

## Build Plan

### Phase 1: Folder restructure (file moves + documentation)

No Rust code changes. All `git mv` for history preservation. `cargo check` must pass after every step.

#### 1.1 Clean up dead directories
- ⬜ Delete empty `apps/` directory

#### 1.2 Reorganise `data/`
- ⬜ Create `data/sertantai/` — move all CSV law lists (`AMD-*.csv`, `LAT-*.csv`, `qq-applicable-laws.csv`)
- ⬜ Create `data/seed/` — move Parquet seed files (`amendment_annotations.parquet`, `annotation_totals.parquet`)
- ⬜ Create `data/audit/` — move `data/llm-audit/` contents (per Gemini suggestion — distinct retention from runtime DBs)
- ⬜ Move stray markdown docs out of `data/` into `docs/`: `LAT-SCHEMA-FOR-SERTANTAI.md`, `LAT-TRANSFORMS-FOR-SERTANTAI.md`, `EU-LAW-SUPPORT-BRIEFING.md`, `clause_eyeball.md`, `gemini-briefing-gguf-export.md`
- ⬜ Update `.gitignore` — ensure runtime files (DuckDB, LanceDB, Modelfile, slm-adapter) stay gitignored at their current paths
- ⬜ Verify: no broken references in Rust code or scripts to moved files

#### 1.3 Reorganise `scripts/`
- ⬜ Create `scripts/migrations/` — move one-off migration scripts: `migrate_*.py`, `rebuild_lance_actors.py`
- ⬜ Create `scripts/maintenance/` — move active maintenance: `compact_lance.py`, `compact_lance_no_backup.py`, `backup_lancedb.py`, `corpus_stats.py`
- ⬜ Create `scripts/ml/` — move ML training + eval: `train_*.py`, `finetune_*.py`, `export_*.py`, `eval_*.py`, `retrain_*.py`, `runpod_*.py`
- ⬜ Create `scripts/benchmarks/` — move benchmark scripts: `benchmark_*.py`, `generate_benchmarks*.py`, `correct_gold_standard.py`
- ⬜ Create `scripts/experiments/` — move significance experiments: `significance_approach_*.py`, `significance_part_breakdown.py`
- ⬜ Keep in `scripts/` root: `gemini_*.py`, `actor_aliases.py`, `compute_dep_features.py` (active operational scripts)
- ⬜ Update any hardcoded script paths in skills or session docs

#### 1.4 Reorganise `docs/`
- ⬜ Create `docs/architecture/` — move: `SCHEMA.md`, `SCHEMA-2.0.md`, `SCHEMA-DIAGRAM.md`, `ZENOH-SYNC.md`, `PAGEINDEX-RESEARCH.md`, `GAP-C-AGENTIC-EXTRACTION-PLAN.md`
- ⬜ Create `docs/operations/` — move: `TAXA-PATTERN-RUNBOOK.md`, `FITNESS-DICTIONARY-RUNBOOK.md`, `CLASSIFICATION-CASCADE-STRATEGY*.md`
- ⬜ Create `docs/dictionaries/` — move: `actor-dictionary.yaml`, `ACTOR-DICTIONARY.md`, `correlative-rules.yaml`, `drrp_classifier_v*.json`, `position_classifier_v*.json`
- ⬜ Keep `docs/manual/` as-is (significance docs, customer-facing)
- ⬜ Keep `docs/reviews/` and `docs/howto/` as-is
- ⬜ Verify: Rust code loads `actor-dictionary.yaml` and `correlative-rules.yaml` — update paths if hardcoded

#### 1.5 CLAUDE.md refresh
- ⬜ Slim top-level `CLAUDE.md` to architecture overview + pointers to crate docs
- ⬜ Create `crates/fractalaw-core/CLAUDE.md` — taxa pipeline, parsing rules, DRRP model, Arrow schemas
- ⬜ Create `crates/fractalaw-cli/CLAUDE.md` — CLI operations guide (enrichment, publish, QA, backfill commands)
- ⬜ Update `.claude/skills/` — fix any paths broken by moves

#### 1.6 Verify
- ⬜ `cargo check --workspace` passes
- ⬜ `cargo test --workspace` passes
- ⬜ All skills reference correct paths
- ⬜ Commit: "Restructure project: organise data/, scripts/, docs/"

---

### Phase 2: CLI binary split

Rust code changes. Split the monolithic `fractalaw` binary into focused binaries with minimal deps.

#### 2.1 Analyse command → dependency mapping
- ⬜ Map each CLI subcommand to its crate dependencies:
  - `taxa *` (parse, classify, reconcile, infer, enrich, backfill, qa, eyeball, status) → core + store + ai
  - `sync *` (publish, watch, pull-lat) → core + store + sync
  - `embed`, `validate` → core + store + ai
  - `law`, `query`, `stats` → core + store
  - `host run` → core + host
- ⬜ Decide binary names: `fractalaw` (taxa + query), `fractalaw-sync`, `fractalaw-host`

#### 2.2 Extract shared CLI scaffolding
- ⬜ Factor out common CLI setup (tracing, clap styles, store opening) into a shared module or thin lib in fractalaw-cli
- ⬜ Keep `commands/` module structure — each binary picks the commands it needs

#### 2.3 Create `fractalaw-sync` binary
- ⬜ New `[[bin]]` in `fractalaw-cli/Cargo.toml` or new crate `crates/fractalaw-sync-cli/`
- ⬜ Dependencies: `core + store(duckdb, pg) + sync(zenoh)` — no ONNX, no wasmtime, no DataFusion
- ⬜ Commands: `sync publish`, `sync watch`, `sync pull-lat`
- ⬜ Verify: `cargo build --bin fractalaw-sync` compiles without ONNX/wasmtime

#### 2.4 Create `fractalaw-host` binary
- ⬜ New `[[bin]]` or crate for WASM host
- ⬜ Dependencies: `core + host` — no store, no ai, no sync
- ⬜ Commands: `host run`
- ⬜ Verify: `cargo build --bin fractalaw-host` compiles without DuckDB/ONNX/Zenoh

#### 2.5 Slim the main `fractalaw` binary
- ⬜ Remove sync and host deps from the main binary's feature set
- ⬜ Dependencies: `core + store(full) + ai(onnx)` — no Zenoh, no wasmtime
- ⬜ Commands: `taxa *`, `embed`, `validate`, `law`, `query`, `stats`
- ⬜ Verify: `cargo build --bin fractalaw` no longer pulls Zenoh or wasmtime

#### 2.6 Measure impact
- ⬜ Compare build times: full workspace before vs after
- ⬜ Compare `target/` size after clean build
- ⬜ Document results in session

#### 2.7 Verify + commit
- ⬜ `cargo check --workspace` passes
- ⬜ `cargo test --workspace` passes
- ⬜ All three binaries run their `--help` correctly
- ⬜ Commit: "Split CLI into fractalaw + fractalaw-sync + fractalaw-host"
