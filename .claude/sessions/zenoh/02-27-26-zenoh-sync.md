# Session: Zenoh pub/sub sync

**Date**: 2026-02-27
**Depends on**: OHS enrichment session (taxa-drrp/02-27-26-ohs-enrichment-zenoh.md, closed)

## Goals

1. Add zenoh pub/sub to `fractalaw-sync` for LAN-based enrichment distribution
2. Publish taxa enrichment results as Arrow IPC over zenoh
3. Lay groundwork for sertantai subscriber integration

## Architecture decisions

### Publish is decoupled from parse/enrich

Zenoh publish is an **explicit, manual CLI action** — not triggered automatically by the taxa pipeline or micro-apps.

Rationale: the taxa parser is still maturing. Automatic publish-on-enrich would propagate buggy results that then need retracting. Better to enrich locally, validate, fix, re-enrich as needed, and only publish when satisfied.

```bash
# 1. Enrich locally (writes to DuckDB/LanceDB only)
fractalaw taxa enrich --family "OH&S: ..."

# 2. Review, validate, iterate...

# 3. Explicitly publish when ready
fractalaw sync publish --family "OH&S: ..."
```

Later, when the parser is more mature, tighter temporal coupling (e.g. post-run hooks) can be added.

### Micro-apps have no network access

Taxa/DRRP is a WASM guest component — it talks to the outside world only through WIT host interfaces (data-query, data-mutate, ai-inference). It cannot and should not do network I/O.

Zenoh is a **host-level concern**, living in `fractalaw-sync` and wired up at the CLI layer — the same level as the existing HTTP sync with sertantai. The micro-app doesn't know or care how its results get distributed.

### Data flow direction

```
Sertantai ──(full text)──→ LanceDB       (one-way pull, no publish)
DuckDB    ──(taxa JSONB)──→ Sertantai     (publish via zenoh/HTTP)
```

- **LanceDB** (`legislation_text`): local working store for per-provision text and enrichment. Populated from sertantai's scraped laws. Never published back.
- **DuckDB** (`legislation`): law-level metadata, mirrors sertantai's metadata table. The taxa columns (aggregated JSONB — `duty_holder`, `right_holder`, `responsibility_holder`, `power_holder`, etc.) are what get published back to sertantai.

`sync publish` reads taxa columns from DuckDB and sends them to sertantai. LanceDB does not participate in publish.

### Layer responsibilities

| Layer | Responsibility | Network access |
|-------|---------------|----------------|
| Guest (micro-app) | Parse, enrich, write to local stores via WIT | None |
| Host (fractalaw-host) | Run guests, provide WIT implementations | None (delegates to sync) |
| CLI (fractalaw-cli) | Orchestrate commands, wire up stores + sync | Yes |
| Sync (fractalaw-sync) | HTTP, zenoh, Arrow Flight transports | Yes |

## Background

### Current sync infrastructure

| Component | Status |
|-----------|--------|
| HTTP sync (reqwest) | Working — `sync pull` / `sync push` with sertantai REST API |
| Arrow Flight | Feature-gated in `fractalaw-sync`, not implemented |
| Zenoh | Planned (`.claude/plans/zenoh.md`), not implemented |
| Loro CRDTs | Dependency present in `fractalaw-sync/Cargo.toml`, not wired up |

### Zenoh plan (from `.claude/plans/zenoh.md`)

- **Phase A**: ZenohSync struct, pub/sub, query/reply, Arrow IPC serialization
- **Phase B**: CRDT integration with Loro
- **Phase C**: Hive router, lifecycle, CLI commands
- **Phase D**: Sertantai integration (Elixir zenohex NIF)
- **Phase E**: Multi-tenancy & mTLS
- **Phase F**: Edge Bees

### Topology

- **Sertantai** (always-on Elixir/Phoenix): Peer in zenoh mesh, publishes legislation updates and annotations
- **Hive** (intermittent Rust hub): Router mode, runs zenohd, hosts DuckDB/LanceDB
- **Bees** (field devices): Clients or peers, connect to Hive or Sertantai
- Key expressions: `fractalaw/@{tenant}/taxa/enrichment/{law_name}`

### For LAN testing, we need:

1. `zenoh` crate added to `fractalaw-sync/Cargo.toml` (feature-gated)
2. A publisher in fractalaw that emits enrichment results as Arrow IPC over zenoh pub/sub
3. Sertantai subscriber (Elixir side, using `zenohex` NIF or a sidecar)

### Current sertantai integration

- **Pull**: `GET /api/outbox/annotations?since={timestamp}` → `Vec<Annotation>`
- **Push**: `POST /api/inbox/polished` → `Vec<PolishedEntry>`, returns `{ accepted: u64 }`
- Implemented in `crates/fractalaw-sync/src/http.rs`

### fractalaw-sync dependencies (`Cargo.toml`)

```toml
[features]
flight = ["dep:arrow-flight", "dep:tonic", "dep:prost"]
http = ["dep:reqwest", "dep:serde", "dep:serde_json", "dep:chrono"]
# zenoh = ... (to be added)
```

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-sync/Cargo.toml` | Zenoh + zenoh feature gate |
| `crates/fractalaw-sync/src/lib.rs` | Module registration (zenoh_sync) |
| `crates/fractalaw-sync/src/zenoh_sync.rs` | ZenohSync struct, Arrow IPC helpers, key expressions |
| `crates/fractalaw-cli/Cargo.toml` | Enabled zenoh feature |
| `crates/fractalaw-cli/src/main.rs` | `sync publish` + `sync crdt` subcommands |
| `crates/fractalaw-sync/src/crdt_sync.rs` | CrdtSync engine, Loro docs over Zenoh |
| `.claude/plans/zenoh.md` | Full implementation plan |

## Progress

- [x] Phase A: ZenohSync struct, pub/sub, Arrow IPC serialization (8/8 tests pass)
  - `ZenohSync` with `new()`, `with_config()`, `publish_taxa()`, `subscribe_taxa()`
  - Arrow IPC encode/decode helpers (public, reusable)
  - Key expression builders: `fractalaw/@{tenant}/taxa/enrichment/{law_name}`
  - `sync publish` CLI command with `--tenant`, `--laws`, `--family` flags
  - Zenoh requires multi-thread tokio runtime (learned: `flavor = "multi_thread"`)
  - Subscriber type: `Subscriber<FifoChannelHandler<Sample>>` (not `Subscriber<()>`)
- [x] Phase B: Loro CRDT sync over Zenoh (11/11 tests pass, 19 total in fractalaw-sync)
  - `CrdtSync` struct: manages named Loro documents with zenoh transport
  - Document lifecycle: `create_doc`, `open_or_create`, `close_doc`, `list_docs`
  - Local mutations: `map_insert`, `map_get`, `list_push`, `get_doc_value`, `doc_version_vector`
  - Auto-publish: `subscribe_local_update` → `tokio::spawn` → zenoh `put` (sync→async bridge)
  - Remote sync: `start_sync()` background subscriber on `crdt/*/updates` wildcard
  - Late-joiner: `serve_snapshots()` queryable + `request_sync()` query with VV payload
  - Persistence: `.loro` snapshot files with atomic write (tmp + rename)
  - Key expressions: `fractalaw/@{tenant}/crdt/{doc_id}/{updates|snapshot}`
  - CLI: `sync crdt status/create/inspect/save` subcommands
  - Learned: `ExportMode<'a>` has lifetime — use `updates_owned(vv)` for decoded VV
  - Learned: `LocalUpdateCallback` is `Box<dyn Fn(&Vec<u8>) -> bool + Send + Sync>` (sync, not async)
  - Schema-agnostic: specific document schemas (risk assessments, etc.) deferred to later phases
- [x] Phase C: Hive router, lifecycle, CLI commands (26/26 tests pass)
  - `HiveSync` struct: composes `ZenohSync` + `CrdtSync` into unified lifecycle
  - State machine: `Idle → Syncing → Publishing → Listening → ShuttingDown`
  - `run_once()`: single sync cycle (load persisted CRDTs → sync → publish taxa → save → exit)
  - `run_continuous()`: initial sync+publish, then background listener for incoming taxa
  - `shutdown()`: save CRDT snapshots, abort background tasks, signal listener to exit
  - `SyncReport`: tracks crdt_docs_synced, taxa_published, taxa_received, warnings
  - `watch_state()` via `tokio::sync::watch` for external state monitoring
  - `with_configs()` constructor for test isolation with custom zenoh configs
  - Bug fix: `shutdown()` must not reset state to `Idle` — races with `run_continuous` listener task which checks for `ShuttingDown` via `watch::Receiver::borrow_and_update()` (sees latest value, not intermediate)
  - Learned: zenoh tests need `--test-threads=1` to avoid multicast scouting contention between parallel sessions
