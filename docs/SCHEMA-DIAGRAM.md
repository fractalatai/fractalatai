# Fractalaw Schema Relationships

**Version**: 0.6
**Date**: 2026-02-25
**Companion to**: [SCHEMA.md](SCHEMA.md)

---

## Table Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              DuckDB                                        │
│                                                                            │
│  ┌─────────────────────┐    ┌──────────────┐    ┌───────────────────────┐  │
│  │  legislation (LRT)  │    │  law_edges   │    │    polished_drrp      │  │
│  │  ─────────────────  │    │  ──────────  │    │    ─────────────      │  │
│  │  89 cols, 1 row/law │    │  8 cols      │    │    11 cols             │  │
│  │  Hot path           │    │  Analytical  │    │    AI output           │  │
│  └─────────────────────┘    └──────────────┘    └───────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                             LanceDB                                        │
│                                                                            │
│  ┌──────────────────────────┐    ┌───────────────────────────┐             │
│  │  legislation_text (LAT)  │    │  amendment_annotations    │             │
│  │  ──────────────────────  │    │  ──────────────────────   │             │
│  │  28 cols, 1 row/section  │    │  9 cols, 1 row/annotation │             │
│  │  Semantic path           │    │  Semantic path             │             │
│  └──────────────────────────┘    └───────────────────────────┘             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Foreign Key Relationships

```
legislation (LRT)                       law_edges
┌──────────────────┐                    ┌───────────────────┐
│ name (PK)        │◄───────────────────│ source_name (FK)  │
│                  │◄───────────────────│ target_name (FK)  │
│                  │                    │ edge_type          │
│ enacted_by[]     │──┐                │ article_target     │
│ enacting[]       │  │ denormalised   │ affect_type        │
│ amending[]       │  │ into edge rows │ applied_status     │
│ amended_by[]     │  │                │ date               │
│ rescinding[]     │──┘                └───────────────────┘
│ rescinded_by[]   │
│                  │
│ function[]       │  contains 'Making' → triggers taxa enrichment
│                  │
│ duties[]         │  regex-extracted (List<DRRPEntry>)
│ rights[]         │  written by taxa enrich
│ responsibilities[]│
│ powers[]         │
│                  │
│ duties_ai[]      │  AI-refined (List<DRRPEntry>)
│ rights_ai[]      │◄──── aggregated from polished_drrp
│ responsibilities_ai[]│
│ powers_ai[]      │
└──────────────────┘
        │
        │ legislation.name
        ▼
┌──────────────────────────┐         ┌───────────────────────────┐
│ legislation_text (LAT)   │         │ amendment_annotations      │
│ ────────────────────     │         │ ──────────────────────     │
│ law_name (FK) ───────────│─ ─ ─ ─►│ law_name (FK)              │
│ section_id (unique)      │◄────────│ affected_sections[] (FK)   │
│ provision                │         │ code                       │
│ text                     │         │ code_type                  │
│ embedding                │         │ text                       │
└──────────────────────────┘         └───────────────────────────┘
        │
        │ law_name + provision text
        ▼
┌───────────────────────────┐
│ polished_drrp             │
│ ─────────────             │
│ law_name (FK)             │  FK → legislation.name
│ provision                 │
│ drrp_type                 │
│ holder                    │
│ ai_clause                 │
│ qualifier                 │
│ clause_ref                │
│ confidence                │
│ polished_at               │
│ model                     │
│ pushed                    │
└───────────────────────────┘
```

---

## DRRP Pipeline: End-to-End Data Flow

```
sertantai                                fractalaw
─────────                                ─────────

Scrapes legislation.gov.uk           ┌──────────────────────────────────┐
        │                            │                                  │
        ▼                            │  1. IMPORT / SYNC                │
Publishes LRT rows ──zenoh──────────►│     LRT rows arrive in DuckDB   │
  (with function[] =                 │     LAT sections arrive in       │
   ['Making', ...])                  │     LanceDB                     │
Publishes LAT sections ──zenoh──────►│                                  │
        │                            └──────────────┬───────────────────┘
        │                                           │
        │                                           ▼
        │                            ┌──────────────────────────────────┐
        │                            │                                  │
        │                            │  2. TAXA ENRICHMENT              │
        │                            │     Finds laws where function    │
        │                            │     contains 'Making'            │
        │                            │                                  │
        │                            │     For each law:                │
        │                            │     ┌─────────────────────────┐  │
        │                            │     │ Read LAT text sections  │  │
        │                            │     │         │               │  │
        │                            │     │         ▼               │  │
        │                            │     │ Run Rust regex parser   │  │
        │                            │     │ (fractalaw-core::taxa)  │  │
        │                            │     │         │               │  │
        │                            │     │         ▼               │  │
        │                            │     │ Write to LRT:           │  │
        │                            │     │  duty_holder[]          │  │
        │                            │     │  rights_holder[]        │  │
        │                            │     │  duty_type[]            │  │
        │                            │     │  role[], role_gvt[]     │  │
        │                            │     │  duties[]    ◄── rough  │  │
        │                            │     │  rights[]       DRRP    │  │
        │                            │     │  responsibilities[]     │  │
        │                            │     │  powers[]               │  │
        │                            │     └─────────────────────────┘  │
        │                            │                                  │
        │                            └──────────────┬───────────────────┘
        │                                           │
        │                                           ▼
        │                            ┌──────────────────────────────────┐
        │                            │                                  │
        │                            │  3. AI POLISHING                 │
        │                            │     drrp-polisher micro-app      │
        │                            │     (WASM guest)                 │
        │                            │                                  │
        │                            │     For each DRRPEntry in        │
        │                            │     duties[]/rights[]/etc.:      │
        │                            │     ┌─────────────────────────┐  │
        │                            │     │ Read DRRPEntry from LRT │  │
        │                            │     │ Read source text from   │  │
        │                            │     │   LAT (context)         │  │
        │                            │     │         │               │  │
        │                            │     │         ▼               │  │
        │                            │     │ Call AI inference        │  │
        │                            │     │ (ONNX model / Claude)   │  │
        │                            │     │         │               │  │
        │                            │     │         ▼               │  │
        │                            │     │ Write → polished_drrp   │  │
        │                            │     └─────────────────────────┘  │
        │                            │                                  │
        │                            │     Aggregate per law:           │
        │                            │     polished_drrp ──► LRT       │
        │                            │       duties_ai[]               │
        │                            │       rights_ai[]               │
        │                            │       responsibilities_ai[]     │
        │                            │       powers_ai[]               │
        │                            │                                  │
        │                            └──────────────┬───────────────────┘
        │                                           │
        │                                           ▼
        │                            ┌──────────────────────────────────┐
        │                            │                                  │
        │                            │  4. PUBLISH                      │
        │                            │     zenoh pub/sub                │
◄───────┼────────────────────────────│     → sertantai (updated LRT)   │
        │                            │     → Bees (hive consumers)     │
        │                            │                                  │
        │                            └──────────────────────────────────┘
```

---

## Before / After: Regex vs AI on the LRT

The LRT carries both versions side-by-side for comparison and validation:

```
legislation (LRT) — one row per law
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  REGEX-EXTRACTED (from taxa enrichment)                     │
│  ┌─────────────────┐ ┌────────────────┐                    │
│  │ duties[]        │ │ duty_holder[]  │  List<Utf8>        │
│  │ rights[]        │ │ rights_holder[]│  (flat holder names)│
│  │ responsibilities[]│ │ resp_holder[] │                    │
│  │ powers[]        │ │ power_holder[] │                    │
│  └─────────────────┘ └────────────────┘                    │
│   List<DRRPEntry>      List<Utf8>                          │
│                                                             │
│  AI-REFINED (from polisher, aggregated from polished_drrp) │
│  ┌─────────────────┐                                       │
│  │ duties_ai[]     │                                       │
│  │ rights_ai[]     │  Same List<DRRPEntry> struct          │
│  │ responsibilities_ai[]│  but with AI-refined clauses     │
│  │ powers_ai[]     │                                       │
│  └─────────────────┘                                       │
│                                                             │
│  DRRPEntry = { holder, duty_type, clause, article }        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Aggregation Query (polished_drrp → LRT *_ai)

The drrp-polisher runs this per affected law after processing all its entries:

```sql
UPDATE legislation SET
    duties_ai = (
        SELECT list(struct_pack(
            holder := holder, duty_type := UPPER(drrp_type),
            clause := ai_clause, article := clause_ref
        )) FROM polished_drrp
        WHERE law_name = ? AND drrp_type = 'duty'
    ),
    rights_ai = (
        SELECT list(struct_pack(...))
        FROM polished_drrp WHERE law_name = ? AND drrp_type = 'right'
    ),
    responsibilities_ai = (
        SELECT list(struct_pack(...))
        FROM polished_drrp WHERE law_name = ? AND drrp_type = 'responsibility'
    ),
    powers_ai = (
        SELECT list(struct_pack(...))
        FROM polished_drrp WHERE law_name = ? AND drrp_type = 'power'
    )
WHERE name = ?
```

---

## Cross-Store Relationships

```
                    DuckDB                          LanceDB
               ┌──────────────┐              ┌──────────────────┐
               │  legislation  │              │ legislation_text  │
               │  (LRT)       │◄─────────────│ (LAT)            │
               │              │  law_name     │                  │
               └──────┬───────┘              └────────┬─────────┘
                      │                               │
                      │ name                          │ section_id
                      │                               │
               ┌──────┴───────┐              ┌────────┴─────────┐
               │  law_edges   │              │ amendment_        │
               │              │              │ annotations       │
               │ source_name  │              │                   │
               │ target_name  │              │ affected_sections │
               └──────────────┘              └───────────────────┘
                      │
                      │ name
                      ▼
               ┌──────────────┐
               │ polished_    │
               │ drrp         │
               │              │───────► aggregates to LRT *_ai cols
               │ (AI output)  │───────► zenoh pub/sub
               └──────────────┘
```

---

## Store Characteristics

| Store | Tables | Persistence | Population |
|-------|--------|------------|------------|
| DuckDB | `legislation`, `law_edges`, `polished_drrp` | `data/fractalaw.duckdb` | `legislation` + `law_edges` loaded from Parquet at import. `polished_drrp` created empty at runtime, populated by drrp-polisher. |
| LanceDB | `legislation_text`, `amendment_annotations` | `data/lancedb/` | Loaded from Parquet at import. Embeddings added by `fractalaw embed`. |

---

## Making Flag → Taxa → Polisher Trigger

```
legislation.function[] contains 'Making'
        │
        │  fractalaw taxa enrich scans for this
        ▼
Laws needing DRRP classification
        │
        │  regex parser runs on LAT text
        ▼
LRT duties[]/rights[]/etc. populated
        │
        │  polisher picks up laws with DRRP entries but no *_ai data
        ▼
LRT duties_ai[]/rights_ai[]/etc. populated
        │
        │  zenoh publishes updated LRT
        ▼
sertantai + Bees receive enriched data
```
