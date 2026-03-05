# Session: Taxa Hash Change Tracking (#21)

**Date**: 2026-03-01
**Issue**: [#21 — Add taxa_hash for change-tracking between enrich and publish](https://github.com/fractalaw/fractalaw/issues/21)
**Objective**: Add content hashing to detect which laws actually changed during enrichment, so `sync publish` only publishes deltas instead of the full corpus.

## Problem

After a mass re-enrichment (`taxa enrich --force`), there's no way to know which laws actually changed. `sync publish --all` re-publishes all 400+ laws even if only ~30 produced different taxa output. Wasteful but not dangerous — sertantai is idempotent. Still, we want clean delta detection as the publish pipeline matures.

## Key Files

- `crates/fractalaw-cli/src/main.rs` — `enrich_single_law()`, `cmd_sync_publish()`, `cmd_taxa_enrich()`
- `crates/fractalaw-store/src/duck.rs` — DuckDB schema, `execute()` for UPDATE statements
- `crates/fractalaw-sync/src/zenoh_sync.rs` — `publish_taxa()`, key expressions

## Design

### Schema

Add two columns to `legislation`:
- `taxa_hash` VARCHAR — content hash of the 11 published taxa columns, set on enrich
- `published_hash` VARCHAR — copy of taxa_hash set after successful publish

A law needs publishing when `taxa_hash IS NOT NULL AND taxa_hash != published_hash` (or `published_hash IS NULL`).

### Hash scope

The 11 taxa columns that get published to sertantai:
`duty_holder`, `rights_holder`, `responsibility_holder`, `power_holder`, `duty_type`, `role`, `role_gvt`, `duties`, `rights`, `responsibilities`, `powers`

### Hash function

Fast non-crypto hash (e.g. xxhash or SipHash via std) — we're comparing content identity, not protecting against tampering. Concatenate sorted column values into a canonical string and hash.

## Changes Made

### `crates/fractalaw-core/src/schema.rs`
- Added `taxa_hash` (Utf8, nullable) and `published_hash` (Utf8, nullable) fields to `legislation_schema()` under section 1.10b
- Updated field count test: 89 → 91

### `crates/fractalaw-store/src/duck.rs`
- Added `ensure_taxa_hash_columns()` — idempotent `ALTER TABLE ADD COLUMN IF NOT EXISTS` for both columns
- Added 2 tests: `ensure_taxa_hash_columns_adds_two_columns`, `ensure_taxa_hash_columns_idempotent`

### `crates/fractalaw-cli/src/main.rs`
- Added `compute_taxa_hash()` — SipHash (via `DefaultHasher`) over 11 taxa columns, returns 16-char hex string. BTreeSets iterated in order; DRRP entry Vecs sorted before hashing for determinism. Column separators (`0xFF`) prevent cross-column collisions.
- Modified `enrich_single_law()`: computes `taxa_hash`, queries existing hash, skips UPDATE if identical. Sets `taxa_hash` in the UPDATE SQL.
- Modified `cmd_taxa_enrich()`: calls `ensure_taxa_hash_columns()` on startup. `--force` now also clears `taxa_hash`.
- Added `--changed` flag to `Publish` CLI variant: selects laws where `taxa_hash IS NOT NULL AND (published_hash IS NULL OR taxa_hash != published_hash)`
- Modified `cmd_sync_publish()`: calls `ensure_taxa_hash_columns()` on startup. After successful publish, sets `published_hash = taxa_hash`.
- Modified `cmd_sync_watch()`: calls `ensure_taxa_hash_columns()` on startup. After successful publish, sets `published_hash = taxa_hash`.
- Added 2 tests: `taxa_hash_deterministic`, `taxa_hash_changes_on_different_input`

## Usage

```bash
# Re-enrich all laws (only laws with changed taxa get DuckDB UPDATE)
fractalaw taxa enrich --force

# Publish only laws whose taxa changed since last publish
fractalaw sync publish --changed --tenant dev

# Publish all (existing behavior unchanged)
fractalaw sync publish --all --tenant dev
```

## Progress

- [x] Investigate current DuckDB schema and enrichment UPDATE path
- [x] Add `taxa_hash` + `published_hash` columns
- [x] Compute hash in `enrich_single_law()`, skip UPDATE if unchanged
- [x] Wire `--changed` flag into `cmd_sync_publish()`
- [x] Update `sync watch` to set `published_hash` after publish
- [x] Unit tests for hash determinism, column creation, idempotency
- [ ] Integration test: re-enrich unchanged law → no UPDATE, no publish
- [ ] Integration test: re-enrich changed law → UPDATE with new hash, publish picks it up

## Status: **Done**

All code changes implemented, all tests pass (337 total), clippy/fmt clean. Integration tests deferred to manual validation with live data (`taxa enrich --force` → `sync publish --changed`).
