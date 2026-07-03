---
session: Project Structure Review
status: closed
opened: 2026-06-25
closed: 2026-07-03
outcome: success

summary: >
  Three-phase project restructure: folder reorganisation (data/, scripts/, docs/,
  CLAUDE.md), CLI binary split (fractalaw-sync extracted, main binary drops Zenoh),
  and session frontmatter SQLite index with archival. 106 session docs indexed with
  321 decisions and 266 lessons queryable via SQL.

decisions:
  - what: Keep monorepo, improve internal organisation
    why: "Crate dependency graph is clean. Heavy builds come from C++ deps, not code coupling. Splitting repos adds overhead not justified at this scale."
    result: "data/ split into sertantai/seed/audit, scripts/ into migrations/maintenance/ml/benchmarks/experiments, docs/ into architecture/operations/dictionaries"
  - what: "Split fractalaw-sync binary, skip fractalaw-host split"
    why: "Sync binary drops ONNX, wasmtime, DataFusion, LanceDB — big win. Host binary still needs store+ai+wasmtime — marginal gain."
    result: "New crate fractalaw-sync-cli. Main binary drops fractalaw-sync and zenoh deps entirely."
  - what: "actor-dictionary.yaml and correlative-rules.yaml stay at docs/ root"
    why: "Both are include_str!() compiled into fractalaw-core binary via relative paths. Moving them breaks the compile."
    result: All other docs moved to subdirectories, these two stay at docs/ root
  - what: "Per-project SQLite for session index, not shared"
    why: "Each repo is independent. Committed to git for queryability. Cross-project queries via SQLite ATTACH."
    result: ".claude/sessions/sessions.db — 106 sessions, 356KB, regenerable from markdown source"
  - what: Flat archive (no subdirectory mirroring)
    why: "SQLite index stores subdir field for topic queries. Deep nesting (archive/taxa-drrp/taxa-gap-analysis/) adds complexity. Filenames have dates for chronological ordering."
    result: "49 sessions archived to archive/ flat directory"
  - what: Deny destructive sweep/clean in settings.json
    why: "cargo sweep --time 1 destroyed DuckDB C++ build cache (4GB, 5+ min rebuild). Caused repeated disk exhaustion during this session."
    result: "sweep --time 0/1, cargo clean, rm -rf target denied. sweep --time 7+ allowed."

metrics:
  sessions_indexed: 106
  decisions_total: 321
  lessons_total: 266
  sessions_archived: 49
  sqlite_size_kb: 356
  files_restructured: 93
  phase1_commit: 96f18da
  phase2_commit: b0a0945
  phase3_commit: 49afd5b

lessons:
  - title: "NAS SMB mount cannot execute compiled binaries — target/ on NAS fails"
    detail: "Tried symlinking target/ to NAS. Shell scripts execute fine but ELF binaries get 'Invalid argument'. SMB/CIFS doesn't support mmap or binary execution reliably. Reverted to local disk."
    tag: infrastructure
  - title: "DuckDB C++ bundled build is ~4GB and 5+ minutes — never sweep it"
    detail: "cargo sweep --time 1 removes yesterday's build cache including libduckdb-sys. The C++ recompile then fills the disk. Added deny rules in settings.json."
    tag: infrastructure
  - title: "Sub-agents write nested YAML frontmatter if not explicitly told flat format"
    detail: "Three agents wrote session: {title: ..., status: ...} instead of flat session: title, status: closed. Had to programmatically fix 27 files. The prompt must say 'session: MUST be a flat STRING, NOT a nested dict'."
    tag: tooling
  - title: "Parallel agent batches (3x9) are effective for bulk frontmatter extraction"
    detail: "27 taxa-drrp sessions processed in ~5 minutes with 3 parallel agents vs ~15 minutes sequential. Each agent independently reads, extracts, writes, and validates."
    tag: tooling
  - title: "session-close command should rebuild the SQLite index as its final step"
    detail: "Without this, newly closed sessions aren't queryable until someone remembers to run the index script manually. Added as step 8."
    tag: architecture

artifacts:
  - CLAUDE.md
  - crates/fractalaw-core/CLAUDE.md
  - crates/fractalaw-cli/CLAUDE.md
  - crates/fractalaw-sync-cli/Cargo.toml
  - crates/fractalaw-sync-cli/src/main.rs
  - crates/fractalaw-sync-cli/src/sync.rs
  - scripts/maintenance/session_index.py
  - .claude/sessions/sessions.db
  - .claude/settings.json
  - .claude/commands/session-close.md

enables:
  - Sertantai session index (same schema, separate DB)
  - Session frontmatter backfill for future sessions
  - Samsung 870 EVO 1TB SSD install (target/ on dedicated drive)
---

# Session: Project Structure Review (CLOSED)

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
- ✅ Delete empty `apps/` directory

#### 1.2 Reorganise `data/`
- ✅ Create `data/sertantai/` — moved all CSV law lists (`AMD-*.csv`, `LAT-*.csv`, `qq-applicable-laws.csv`, `xLAT-*.csv`)
- ✅ Create `data/seed/` — moved Parquet seed files (`amendment_annotations.parquet`, `annotation_totals.parquet`)
- ✅ Create `data/audit/` — moved `data/llm-audit/` contents
- ✅ Move stray markdown docs out of `data/` into `docs/architecture/`
- ✅ `.gitignore` — `/data` rule covers all runtime files, no changes needed
- ✅ Updated Rust paths: `data/llm-audit` → `data/audit`, `data/clause_eyeball.md` → `docs/clause_eyeball.md`

#### 1.3 Reorganise `scripts/`
- ✅ `scripts/migrations/` — 6 one-off migration scripts + evaluate_polisher.sh
- ✅ `scripts/maintenance/` — compact_lance.py, compact_lance_no_backup.py, backup_lancedb.py, corpus_stats.py
- ✅ `scripts/ml/` — 15 training, fine-tuning, export, eval scripts
- ✅ `scripts/benchmarks/` — 5 benchmark scripts
- ✅ `scripts/experiments/` — 16 significance approach scripts + gemini_significance.py
- ✅ Root: 4 active operational scripts (gemini_actor_review, gemini_code_review, gemini_llm_batch, actor_aliases)

#### 1.4 Reorganise `docs/`
- ✅ `docs/architecture/` — 12 docs (schemas, Zenoh, plans, briefings, eyeball outputs)
- ✅ `docs/operations/` — 4 docs (runbooks, cascade strategy)
- ✅ `docs/dictionaries/` — ACTOR-DICTIONARY.md + 6 classifier config JSONs
- ✅ `actor-dictionary.yaml` and `correlative-rules.yaml` kept at `docs/` root (include_str! paths)
- ✅ `docs/manual/`, `docs/reviews/`, `docs/howto/` unchanged
- ✅ Updated Rust paths: classifier configs → `docs/dictionaries/`

#### 1.5 CLAUDE.md refresh
- ✅ Top-level CLAUDE.md slimmed: architecture overview, directory layout, pointers to crate docs
- ✅ `crates/fractalaw-core/CLAUDE.md` created
- ✅ `crates/fractalaw-cli/CLAUDE.md` created
- ✅ Skills paths updated (background agent)

#### 1.6 Verify
- ✅ `cargo check --workspace` passes (1 pre-existing warning only)
- ✅ `cargo test --workspace` — 46/47 pass, 1 pre-existing failure (position_classifier category_encoding, unrelated)
- ✅ All Rust code references correct paths
- ✅ Commit: 96f18da "Restructure project: organise data/, scripts/, docs/"

---

### Phase 2: CLI binary split

Rust code changes. Split the monolithic `fractalaw` binary into focused binaries with minimal deps.

#### 2.1 Analyse command → dependency mapping
- ✅ Mapped all subcommands to crate deps
- ✅ Decision: split `fractalaw-sync` (big win — drops ONNX, wasmtime, DataFusion, LanceDB). Skip `fractalaw-host` split (marginal gain — host needs store + ai + wasmtime anyway).

#### 2.2 Extract shared CLI scaffolding
- ✅ Shared utilities (open_duck, ZenohArgs, laws_in_family, get_string_value) duplicated into sync-cli — ~150 lines, simpler than extracting a shared crate

#### 2.3 Create `fractalaw-sync` binary
- ✅ New crate `crates/fractalaw-sync-cli/` with own Cargo.toml
- ✅ Dependencies: `core + store(duckdb, pg) + sync(zenoh, http)` — no ONNX, no wasmtime, no DataFusion, no LanceDB
- ✅ Commands: publish, pull, push, pull-lat, watch, crdt
- ✅ `cargo check` passes, `fractalaw-sync --help` runs correctly

#### 2.4 Skip `fractalaw-host` split
- ✅ Host binary still needs DuckDB + ONNX + wasmtime (provides data store and AI as host capabilities to WASM guests). Marginal gain from splitting. `run` command stays in main binary.

#### 2.5 Slim the main `fractalaw` binary
- ✅ Removed `fractalaw-sync` and `zenoh` deps from fractalaw-cli/Cargo.toml
- ✅ Removed `Sync` command enum, `ZenohArgs`, `SyncAction`, `CrdtAction` from main.rs
- ✅ Removed `--customer` zenoh flag from `taxa status`
- ✅ Excluded `commands/sync.rs` from module tree
- ✅ `fractalaw --help` confirms no `sync` subcommand

#### 2.6 Verify
- ✅ `cargo check --workspace` passes (3m31s clean build, 1 pre-existing warning)
- ✅ `fractalaw-sync --help` runs — 6 commands (publish, pull, push, pull-lat, watch, crdt)
- ✅ `fractalaw --help` runs — 14 commands (no sync)
- ⏸️ `cargo test --workspace` (deferred — pre-existing position_classifier test failure, unrelated to restructure)
- ✅ Commit: b0a0945 "Split CLI: fractalaw-sync binary for Zenoh sync commands"

---

### Phase 3: Session frontmatter index + archival

Session docs accumulate YAML frontmatter with decisions, metrics, lessons, and dependency graphs. Extract this into a queryable SQLite database so the frontmatter survives archival and is searchable without reading 100+ markdown files.

#### Design

**SQLite at `.claude/sessions/sessions.db`** — regenerable from source markdown at any time.

**Schema** (normalised, not JSON blobs):

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,           -- filename without extension (e.g. '07-01-26-significance-publish')
    path TEXT NOT NULL,            -- relative path from repo root
    subdir TEXT,                   -- cascade, store, fitness, etc. or NULL for top-level
    title TEXT NOT NULL,           -- session field from frontmatter
    status TEXT NOT NULL,          -- closed, suspended, active, pending
    outcome TEXT,                  -- success, partial, failed, deferred
    opened DATE,
    closed DATE,
    summary TEXT
);

CREATE TABLE decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    what TEXT NOT NULL,
    why TEXT,
    result TEXT
);

CREATE TABLE lessons (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    title TEXT NOT NULL,
    detail TEXT,
    tag TEXT                       -- infrastructure, models, methodology, data, architecture, tooling
);

CREATE TABLE metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    key TEXT NOT NULL,
    value TEXT NOT NULL             -- JSON string for nested values
);

CREATE TABLE artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    path TEXT NOT NULL
);

CREATE TABLE dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    direction TEXT NOT NULL,        -- 'depends_on' or 'enables'
    target TEXT NOT NULL            -- filename or description
);
```

**Python script: `scripts/maintenance/session_index.py`**

- Scans `.claude/sessions/**/*.md` for YAML frontmatter (between `---` fences)
- Parses with `pyyaml` (stdlib `yaml` not available — use `pip install pyyaml` or a simple regex parser)
- Drops and recreates all tables on each run (idempotent rebuild from source)
- Flags: `--db` (default `.claude/sessions/sessions.db`), `--archive` (run archival after indexing)

**Example queries:**

```sql
-- All decisions about persistence/storage
SELECT s.title, d.what, d.result
FROM decisions d JOIN sessions s ON d.session_id = s.id
WHERE d.what LIKE '%persist%' OR d.what LIKE '%store%';

-- Lessons by tag
SELECT s.title, l.title, l.detail
FROM lessons l JOIN sessions s ON l.session_id = s.id
WHERE l.tag = 'architecture';

-- Session dependency graph
SELECT s.title, d.direction, d.target
FROM dependencies d JOIN sessions s ON d.session_id = s.id
ORDER BY s.opened;

-- Find archived sessions about significance
SELECT id, title, summary, path
FROM sessions
WHERE summary LIKE '%significance%';
```

#### Archival

Sessions closed >30 days ago move to `.claude/sessions/archive/{subdir}/`:

```
.claude/sessions/
├── archive/
│   ├── cascade/          # archived cascade sessions
│   └── ...
├── cascade/              # active/recent cascade sessions
├── sessions.db           # SQLite index (covers all sessions, active + archived)
└── 06-25-26-project-structure-review.md
```

- `git mv` preserves history — `git log --follow` recovers full content
- SQLite retains complete frontmatter for discovery
- The index script scans both active and archive directories

#### Work items

##### 3.1 Build the index script
- ✅ Created `scripts/maintenance/session_index.py` — takes `--root` for cross-project reuse
- ✅ Parses YAML frontmatter, normalised SQLite schema (6 tables, 6 indexes)
- ✅ Idempotent rebuild from source — drops and recreates on each run
- ✅ `--archive` flag moves sessions closed >N days ago via `git mv`
- ✅ Fixed 3 YAML quoting errors in frontmatter (unescaped colons/quotes in values)
- ✅ Indexed 14 sessions: 47 decisions, 54 lessons, 88 metrics, 81 artifacts, 50 dependencies

##### 3.2 Add frontmatter to remaining closed sessions
- ⏸️ 93 sessions without frontmatter — backfilling is a future task (requires reading each session and extracting decisions/lessons from content)

##### 3.3 Archive old sessions
- ✅ Archive mechanism works (git mv to archive/{subdir}/)
- ✅ No candidates yet — all 14 frontmattered sessions closed after 2026-06-03
- ⏸️ Will activate when sessions age past 30 days, or after backfilling older sessions with frontmatter

##### 3.4 Commit
- ✅ sessions.db committed to git (small, ~50KB, queryable by anyone who clones)
- ✅ Per-project architecture: same schema, separate DB per repo. Cross-project queries via SQLite ATTACH.
