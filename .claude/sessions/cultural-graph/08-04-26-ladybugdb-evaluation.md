---
session: LadybugDB Graph Storage Evaluation
status: pending
opened: 2026-08-04
---

# Session: LadybugDB Graph Storage Evaluation (PENDING)

## Problem

The cultural graph production data is in DuckDB (11,170 narratives, 68K edges). DuckDB handles analytics well (site profiles, temporal trajectories, governance metrics) but graph traversal queries (path finding, centrality, community detection) need a native graph database. Kuzudb development has stopped; its fork LadybugDB is the candidate. Need to evaluate LadybugDB for the graph query side of the hybrid dual-ingest architecture.

## Todo

- ⬜ Evaluate LadybugDB — Arrow integration, Cypher support, maturity, Rust bindings
- ⬜ Prototype dual-ingest — same production JSONL feeds both DuckDB and LadybugDB
- ⬜ Test graph queries — path finding, betweenness centrality, community detection on cultural edges
- ⬜ Decide: LadybugDB viable or wait for alternative?

## Dependencies

- ✅ Production data in DuckDB (11,170 narratives)
- ⬜ LadybugDB stability assessment
