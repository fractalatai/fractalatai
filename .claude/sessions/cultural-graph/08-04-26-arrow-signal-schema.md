---
session: Arrow IPC Signal Schema Design
status: pending
opened: 2026-08-04
---

# Session: Arrow IPC Signal Schema Design (PENDING)

## Problem

In the fractalaw architecture, the edge device processes narratives and emits a signal (Arrow RecordBatch) to the hub. The raw narrative never leaves the edge. The signal schema determines what the hub can query and aggregate. Need to define the Arrow IPC format for cultural graph triples, aligning with the fractalaw sync infrastructure (Zenoh pub/sub, Arrow IPC).

## Todo

- ⬜ Define Arrow RecordBatch schema for cultural graph signal
- ⬜ Map to fractalaw-sync Zenoh topic structure
- ⬜ Define signal metadata (model version, extraction confidence, schema version)
- ⬜ Prototype: edge emits Arrow IPC → hub receives and loads into DuckDB/LadybugDB

## Dependencies

- ✅ Production extraction pipeline working
- ✅ DuckDB schema defined (narratives, entities, edges tables)
- ⬜ fractalaw-sync Zenoh infrastructure (existing for DRRP, needs cultural graph topics)
