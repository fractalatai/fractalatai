---
session: "Benchmark laws: LAT sync and gold-label protection"
status: closed
opened: 2026-09-30
closed: 2026-09-30
outcome: success
related: [62, 74, "sertantai-legal#166"]
summary: >
  The 20 benchmark laws had been excluded from the #62 LAT sync, so they held old text
  (e.g. 809 of the Water Industry Act's 8,288 rows). All 20 were synced with
  --allow-benchmark, after a gold snapshot and backups. Gold labels were carried
  forward through a new adjudicated actor tier where the text change was cosmetic.
  The laws were re-parsed and republished, and legal verified them clean.
decisions:
  - what: "Protect gold before syncing: snapshot, carry forward cosmetic changes as adjudicated, queue substantive changes for review"
    why: "Jason approved legal's plan"
    result: "benchmark_gold_snapshot_20260930 (3,901 labels); 191 adjudicated rows; 86 queued (superseded by gold v2, #74)"
  - what: "Adjudicated actor tier (adj_drrp/adj_position/adj_note), top of reconcile"
    why: "The spec's human-adjudication tier didn't exist for actors"
    result: "e988078; taxa infer no longer deletes SLM-only or adjudicated rows"
lessons:
  - title: "Re-parsing benchmark laws must not re-score unchanged rows"
    detail: "dep + classify re-scored 734 classifier positions, and current reconcile rules moved 646 finals. They were restored from backup on unchanged-text rows (memory: feedback_benchmark_reparse)."
    tag: process
---

# Session: Benchmark laws LAT sync and gold-label protection (CLOSED)

## Problem

Legal found 9,714 rows in 14 benchmark laws that fractalaw never held. The #62 sync only reports on benchmark laws and never applies to them.

## Todo

- ✅ Dry run: only the 20 benchmark laws were stale (715 in sync)
- ✅ 4 large laws synced (backup `lat_bm4_pre_sync_20260930/`), then 16 more (backup `lat_bm16_pre_sync_20260930/`)
- ✅ Gold snapshot + adjudicated tier + `scripts/benchmarks/carry_forward_gold.py`
- ✅ Re-parsed; classifier/dep tiers and finals restored on unchanged-text rows; all 20 making; republished; legal verified 27,428 rows
- ❌ v1 review queue (86), legacy-id remap (887) and re-reconcile question: dropped, superseded by gold v2 (#74)
