---
session: Query LRT CLI
status: closed
opened: 2026-07-13
closed: 2026-07-13
outcome: success

summary: >
  Discovered pull-lrt CLI command already exists — was implemented but undocumented.
  Created /lrt-sync skill to make it discoverable. Tested on 19 orphan laws — works
  for valid laws, sertantai returns malformed IPC for the 19 missing laws (sertantai-side fix needed).

decisions:
  - what: The architecture is NOT tightly coupled — pull-lrt exists as standalone CLI
    why: Initial assumption was that query_lrt was buried inside sync-watch. Investigation found cmd_sync_pull_lrt already wired up as a top-level command.
    result: No code change needed. Created skill for discoverability.

lessons:
  - title: Document existing CLI commands as skills so they're discoverable
    detail: pull-lrt existed for months but wasn't used because nobody knew about it. A session was created to build something that already existed. Skills prevent this — they surface capabilities when the context matches.
    tag: tooling

  - title: Arrow IPC decode failures usually mean the law doesn't exist on sertantai's side
    detail: "failed to fill whole buffer" means sertantai returned bytes that aren't valid Arrow IPC — likely an error message or empty response for a law it doesn't know about.
    tag: infrastructure

artifacts:
  - .claude/skills/lrt-sync/SKILL.md

depends_on:
  - 07-13-26-fitness-reconcile-publish.md

enables:
  - Any future batch import can ensure LRT exists before enrichment
  - Sertantai-side fix for the 19 malformed responses
---

# Session: Query LRT CLI (CLOSED)

## Problem

`query_lrt()` exists in the Zenoh sync library but is tightly coupled to sync-watch — it only runs as part of the LAT ingestion flow. Any process that writes provisions to Postgres without going through sync-watch (batch imports, fitness extraction, ad-hoc scripts) can create provisions without a DuckDB LRT record. This causes 19 laws with fitness data that can't be published because they have no LRT row.

The fix: expose `query_lrt` as a standalone CLI command. Any process can then ensure a law has an LRT record before operating on it. This is an architecture improvement — decoupling LRT creation from the sync-watch flow.

## Current Architecture

```
sync-watch (monolithic flow):
  sertantai sends LAT event
    → query_lrt() — pull law metadata from sertantai
    → upsert_legislation() — write to DuckDB
    → pull LAT provisions — write to Postgres
    → triage
    → publish triage result
```

`query_lrt` is step 1 of this flow. It should be callable independently.

## Work

1. ✅ `fractalaw-sync pull-lrt --laws <LAWS>` already exists — was wired up but not documented/used
2. ✅ Tested on known-good law (UK_ukpga_1981_69) — works correctly, 1 row merged
3. ✅ Tested on missing laws — sertantai returns malformed Arrow IPC ("failed to fill whole buffer"). This is a sertantai-side issue, not a fractalaw issue.
4. ⬜ Investigate sertantai queryable for these 19 laws (sertantai-side fix)
5. ⬜ Once sertantai fixed: re-pull, re-aggregate fitness, re-publish

## Dependencies

- ✅ `query_lrt()` exists in `fractalaw-sync/src/zenoh_sync.rs`
- ✅ `upsert_legislation()` exists in DuckStore
- ✅ Sertantai queryable at `fractalaw/@dev/sertantai/lrt/{law_name}`
- ✅ 19 laws identified that need LRT records
