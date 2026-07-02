# Zenoh LAT Wire Format — Arrow IPC

**Version**: 0.1
**Date**: 2026-02-27
**Status**: Draft — implementing in both sertantai-legal and fractalaw

Fractalaw queries sertantai over Zenoh request/reply for legislation text (LAT) data. This document specifies the Arrow IPC payload format for `*/lat/{name}` responses.

For the full LAT schema, see [SCHEMA.md Table 3: legislation_text](SCHEMA.md#table-3-legislation_text--legal-article-table-lat--semantic-path).

---

## Transport

```
fractalaw                          sertantai-legal
    |                                     |
    |-- Query(key_expr) ----------------->|
    |                                     |-- fetch from PostgreSQL
    |                                     |-- build Arrow RecordBatch
    |                                     |-- serialize as IPC stream
    |<-- Reply(key_expr, bytes) ----------|
```

- **Key expression**: `fractalaw/@{tenant}/data/legislation/lat/{law_name}`
- **Payload**: Arrow IPC **streaming format** (not file format)
- **Content type**: `application/vnd.apache.arrow.stream`
- **On error**: empty payload (0 bytes) — fractalaw treats this as "no data"

---

## Arrow IPC Streaming Format

The payload is a single Arrow IPC stream containing one or more RecordBatches. Explorer (Polars) can produce this via:

```elixir
# Explorer DataFrame → Arrow IPC bytes
{:ok, ipc_bytes} = Explorer.DataFrame.dump_ipc_stream(df)
```

Fractalaw decodes with:

```rust
let reader = arrow::ipc::reader::StreamReader::try_new(Cursor::new(bytes), None)?;
let batches: Vec<RecordBatch> = reader.into_iter().collect::<Result<_, _>>()?;
```

The stream format is a sequence of: `[Schema, RecordBatch*, EOS]`. A single RecordBatch per law is typical and preferred.

---

## Schema — Required Columns

All columns that sertantai has available for the law. This is the **transfer schema** — fractalaw's full `legislation_text` table has additional computed columns (embeddings, taxa, AI) that are never sent over the wire.

| # | Column | Arrow Type | Nullable | Notes |
|---|--------|-----------|----------|-------|
| 1 | `section_id` | `Utf8` | no | **Primary key.** Structural citation: `{law_name}:{citation}[{extent}]` |
| 2 | `law_name` | `Utf8` | no | Parent law identifier, e.g. `UK_uksi_2004_1309` |
| 3 | `section_type` | `Utf8` | no | Enum: `title`, `part`, `chapter`, `heading`, `section`, `sub_section`, `article`, `sub_article`, `paragraph`, `sub_paragraph`, `schedule`, `commencement`, `table`, `note`, `signed` |
| 4 | `text` | `Utf8` | no | Legal text content |
| 5 | `sort_key` | `Utf8` | no | Zero-padded hierarchical sort key, e.g. `001.002.001` |
| 6 | `position` | `Int32` | no | 1-indexed document order within the law |
| 7 | `depth` | `Int32` | no | Nesting depth (0 = top-level) |
| 8 | `hierarchy_path` | `Utf8` | **yes** | Slash-separated path, e.g. `part.1/heading.2/section.3`. NULL for root. |
| 9 | `extent_code` | `Utf8` | **yes** | Geographic extent: `E+W+S`, `E+W`, `S`, `NI`. NULL = inherits law default. |
| 10 | `language` | `Utf8` | no | Language code: `en`. Default `en` for all UK laws. |

### Optional Columns

Include if available. Fractalaw's `upsert_lat()` uses `merge_insert` keyed on `section_id` — extra columns are merged automatically.

| # | Column | Arrow Type | Nullable | Notes |
|---|--------|-----------|----------|-------|
| 11 | `part` | `Utf8` | yes | Materialised path: part number/letter |
| 12 | `chapter` | `Utf8` | yes | Chapter number |
| 13 | `heading_group` | `Utf8` | yes | Cross-heading group label |
| 14 | `provision` | `Utf8` | yes | Section/regulation number |
| 15 | `paragraph` | `Utf8` | yes | Paragraph number |
| 16 | `sub_paragraph` | `Utf8` | yes | Sub-paragraph number |
| 17 | `schedule` | `Utf8` | yes | Schedule/annex number |
| 18 | `amendment_count` | `Int32` | yes | F-code annotation count |
| 19 | `modification_count` | `Int32` | yes | C-code annotation count |
| 20 | `commencement_count` | `Int32` | yes | I-code annotation count |
| 21 | `extent_count` | `Int32` | yes | E-code annotation count |
| 22 | `editorial_count` | `Int32` | yes | Editorial annotation count |
| 23 | `updated_at` | `Timestamp(µs, UTC)` | no | Last update time (microsecond precision OK — fractalaw stores as nanoseconds internally) |
| 24 | `created_at` | `Timestamp(µs, UTC)` | yes | Record creation time |

### Columns NOT Sent (computed locally by fractalaw)

These exist in fractalaw's `legislation_text` table but are never part of the wire format:

- `embedding`, `embedding_model`, `embedded_at` — computed by ONNX pipeline
- `token_ids`, `tokenizer_model` — computed by tokenizer pipeline
- `legacy_id` — migration artifact
- `drrp_types`, `governed_actors`, `government_actors`, `duty_family`, `duty_sub_type`, `popimar`, `purposes`, `clause_refined`, `taxa_confidence`, `taxa_classified_at` — computed by taxa enrichment pipeline
- `ai_holder`, `ai_clause`, `ai_qualifier`, `ai_clause_ref`, `ai_confidence`, `ai_model`, `ai_polished_at` — computed by AI polisher

---

## Explorer Implementation Guide

### Building the DataFrame

```elixir
# Query sections for a specific law from PostgreSQL
sections = LegislationText
  |> where([s], s.law_name == ^law_name)
  |> order_by([s], s.sort_key)
  |> Repo.all()

# Build Explorer DataFrame with the wire schema columns
df = Explorer.DataFrame.new(%{
  section_id:        Enum.map(sections, & &1.section_id),
  law_name:          Enum.map(sections, & &1.law_name),
  section_type:      Enum.map(sections, & &1.section_type),
  text:              Enum.map(sections, & &1.text),
  sort_key:          Enum.map(sections, & &1.sort_key),
  position:          Enum.map(sections, & &1.position),
  depth:             Enum.map(sections, & &1.depth),
  hierarchy_path:    Enum.map(sections, & &1.hierarchy_path),
  extent_code:       Enum.map(sections, & &1.extent_code),
  language:          Enum.map(sections, fn _ -> "en" end),
  # optional columns
  part:              Enum.map(sections, & &1.part),
  chapter:           Enum.map(sections, & &1.chapter),
  heading_group:     Enum.map(sections, & &1.heading_group),
  provision:         Enum.map(sections, & &1.provision),
  paragraph:         Enum.map(sections, & &1.paragraph),
  sub_paragraph:     Enum.map(sections, & &1.sub_paragraph),
  schedule:          Enum.map(sections, & &1.schedule),
  amendment_count:   Enum.map(sections, & &1.amendment_count),
  modification_count: Enum.map(sections, & &1.modification_count),
  commencement_count: Enum.map(sections, & &1.commencement_count),
  extent_count:      Enum.map(sections, & &1.extent_count),
  editorial_count:   Enum.map(sections, & &1.editorial_count),
  updated_at:        Enum.map(sections, & &1.updated_at)
})
```

### Type Casting

Explorer infers types. Ensure integer columns are `:s32` (signed 32-bit) not `:s64`:

```elixir
df = df
  |> Explorer.DataFrame.mutate(
    position: cast(position, :s32),
    depth: cast(depth, :s32),
    amendment_count: cast(amendment_count, :s32),
    modification_count: cast(modification_count, :s32),
    commencement_count: cast(commencement_count, :s32),
    extent_count: cast(extent_count, :s32),
    editorial_count: cast(editorial_count, :s32)
  )
```

### Serializing to Arrow IPC Stream

```elixir
{:ok, ipc_bytes} = Explorer.DataFrame.dump_ipc_stream(df)
```

### Zenoh Reply

```elixir
Zenohex.Query.reply(query, key_expr, ipc_bytes)
```

---

## Expected Payload Sizes

| Law | Sections | Approx IPC bytes |
|-----|----------|-----------------|
| UK_uksi_2004_1309 (small SI) | ~30 | ~5 KB |
| UK_ukpga_1974_37 (HSWA 1974) | ~350 | ~80 KB |
| UK_ukpga_2006_46 (Companies Act) | ~2,000+ | ~500 KB |
| Typical SI | 50–150 | 10–30 KB |

Arrow IPC is compact for columnar text data — dictionary encoding on repeated strings (law_name, section_type, language) compresses well.

---

## Fractalaw Receive Side

On receipt, fractalaw:

1. Decodes Arrow IPC stream → `Vec<RecordBatch>`
2. Calls `LanceStore::upsert_lat(batches)`:
   - If `legislation_text` table doesn't exist → `create_table` (fresh install)
   - If table exists → `merge_insert` keyed on `section_id`:
     - Matched rows: update all provided columns
     - Unmatched rows: insert
   - Embeddings and taxa columns are preserved (merge_insert only touches columns present in the batch)

```bash
# CLI usage
fractalaw sync pull-lat --tenant dev --laws UK_uksi_2004_1309

# Multiple laws
fractalaw sync pull-lat --tenant dev --laws UK_uksi_2004_1309,UK_ukpga_1974_37
```

---

## Change Notifications — `events/sync`

Sertantai publishes a JSON event on data changes. Fractalaw subscribes and auto-pulls.

- **Key expression**: `fractalaw/@{tenant}/events/sync`
- **Payload**: JSON

```json
{
  "table": "lat",
  "action": "persist",
  "metadata": {
    "law_name": "UK_ukpga_1974_37",
    "count": 350
  },
  "timestamp": "2026-02-27T15:30:00Z"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `table` | string | Data table that changed: `lat`, `lrt`, `amendments` |
| `action` | string | What happened: `persist` |
| `metadata.law_name` | string | Law identifier |
| `metadata.count` | integer? | Row count (optional) |
| `timestamp` | string | ISO 8601 UTC |

Fractalaw reacts to `table: "lat"` events by calling `query_lat(law_name)` → `upsert_lat()`.

```bash
# Long-running watcher (Ctrl+C to stop)
fractalaw sync watch-lat --tenant dev
```

---

## Validation Checklist

Before first transfer, verify with a test law (e.g., `UK_uksi_2004_1309`):

- [ ] `section_id` values match fractalaw's citation format (`{law_name}:{citation}[{extent}]`)
- [ ] `sort_key` values produce correct document order when sorted lexicographically
- [ ] `position` is 1-indexed and monotonically increasing per law
- [ ] `section_type` values are from the enum above (no `heading_group` — that's a column, not a type)
- [ ] `text` is non-empty for all rows
- [ ] Integer columns are `Int32` (Arrow `i32`), not `Int64`
- [ ] Nullable columns use Arrow null representation (not empty strings)
- [ ] `Explorer.DataFrame.dump_ipc_stream/1` produces bytes that decode cleanly:
  ```elixir
  {:ok, bytes} = Explorer.DataFrame.dump_ipc_stream(df)
  {:ok, roundtrip} = Explorer.DataFrame.load_ipc_stream(bytes)
  assert Explorer.DataFrame.shape(df) == Explorer.DataFrame.shape(roundtrip)
  ```
