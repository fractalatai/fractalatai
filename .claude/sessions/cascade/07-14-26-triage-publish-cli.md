---
session: Triage Publish CLI
status: active
opened: 2026-07-14
---

# Session: Triage Publish CLI (ACTIVE)

## Problem

The `fractalaw-sync triage` command runs triage classification (regex-based making/not-making) and writes results to DuckDB, but has no Zenoh connectivity — it can't publish results back to sertantai. Triage publish is only wired inside `cmd_sync_watch`, which requires the long-running watch loop. After batch operations (e.g., 172 reparsed laws), there's no way to resend triage results without re-triggering the full watch pipeline. (GH issue #48)

## Current State

- `publish_triage()` exists in `ZenohSync` (`zenoh_sync.rs:503-534`) — publishes JSON to `fractalaw/@{tenant}/triage/{law_name}`
- `cmd_triage` (`main.rs:565-782`) — runs triage, writes DuckDB, but has no ZenohArgs, no `--publish` flag
- `cmd_sync_watch` (`sync.rs:841-903`) — the only place `publish_triage()` is called
- The `Triage` enum variant (`main.rs:138-151`) takes `--laws`, `--family`, `--all`, `--verbose` — no Zenoh args

## Approach

Both issue options implemented:

1. `triage --publish` — re-runs triage classification and publishes (needs `--pg` for provision texts)
2. `publish-triage` — reads existing triage results from DuckDB and publishes (no `--pg` needed)

Added `triage_counts VARCHAR` column to DuckDB so counts are persisted for later republish. All three write paths (cmd_triage, sync watch, queryable) now store counts.

## Work

1. ✅ Add `ZenohArgs` (flattened) and `--publish` flag to the `Triage` subcommand
2. ✅ Wire ZenohArgs through the command dispatch to `cmd_triage`
3. ✅ In `cmd_triage`, when `--publish` is set: create ZenohSync session, wait for peer, publish each result
4. ✅ Add `triage_counts` column to DuckDB — all write paths store counts as JSON
5. ✅ Add `PublishTriage` subcommand — reads from DuckDB, publishes without re-classification
6. ✅ Test: `publish-triage --laws UK_apni_1969_6 --tenant dev` — 1/1 published, 94% making
7. ⬜ Test: `triage --publish` with Postgres (blocked by tunnel on port 5433)
8. ⬜ Close GH issue #48

## Dependencies

- ✅ `publish_triage()` method exists in ZenohSync
- ✅ Triage CLI works and writes DuckDB
- ✅ ZenohArgs struct reusable (already flattened in Publish, Watch, etc.)
