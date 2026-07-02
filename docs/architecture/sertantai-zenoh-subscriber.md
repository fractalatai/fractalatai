# Sertantai: Zenoh Taxa Subscriber — Design Handoff

**From**: fractalaw (Rust/Hive side)
**To**: sertantai (Elixir/Phoenix side)
**Date**: 2026-02-27
**Status**: Fractalaw publish path complete, ready for sertantai subscriber

---

## What This Is

Fractalaw can now publish law-level taxa enrichment data (DRRP: duties, rights, responsibilities, powers) over **Zenoh pub/sub** as **Arrow IPC** payloads. Sertantai needs a subscriber to receive this data and persist it to Postgres for the web UI.

This document specifies exactly what sertantai will receive on the wire.

---

## 1. Key Expression

**Pattern**: `fractalaw/@{tenant}/taxa/enrichment/{law_name}`

**Example**: `fractalaw/@acme/taxa/enrichment/UK_ukpga_1974_37`

**Wildcard to subscribe to all taxa for a tenant**:
```
fractalaw/@acme/taxa/enrichment/*
```

The `@` prefix creates a **hermetic namespace** — `fractalaw/@*/taxa/**` does NOT match across tenants. Each tenant subscription must be explicit.

### Extracting the law name from a key expression

The law name is always the last path segment:

```elixir
def law_name_from_key(key_expr) do
  key_expr |> String.split("/") |> List.last()
end
```

---

## 2. Payload Format: Arrow IPC Streaming

Each zenoh sample payload contains an **Arrow IPC streaming format** byte buffer encoding one `RecordBatch` (one row per law, though typically one law per message).

### Schema

| Column | Arrow Type | Nullable | Description |
|--------|-----------|----------|-------------|
| `name` | `Utf8` | No | Law identifier, e.g. `UK_ukpga_1974_37` |
| `duty_holder` | `List<Utf8>` | Yes | Aggregated actor names with duties |
| `rights_holder` | `List<Utf8>` | Yes | Actor names with rights |
| `responsibility_holder` | `List<Utf8>` | Yes | Actor names with responsibilities |
| `power_holder` | `List<Utf8>` | Yes | Actor names with regulatory powers |
| `duty_type` | `List<Utf8>` | Yes | Type classifiers (duty, obligation, requirement, etc.) |
| `role` | `List<Utf8>` | Yes | Semantic role labels extracted from provisions |
| `role_gvt` | `List<Utf8>` | Yes | Government/authority role flags |
| `duties` | `List<Struct>` | Yes | Detailed duty entries (see below) |
| `rights` | `List<Struct>` | Yes | Detailed right entries |
| `responsibilities` | `List<Struct>` | Yes | Detailed responsibility entries |
| `powers` | `List<Struct>` | Yes | Detailed power entries |

### Struct columns (`duties`, `rights`, `responsibilities`, `powers`)

Each element in the list is a struct with:

```
STRUCT(
  holder      VARCHAR,   -- e.g. "employer"
  duty_type   VARCHAR,   -- e.g. "duty", "obligation", "requirement"
  clause      VARCHAR,   -- extracted clause text
  article     VARCHAR    -- citation reference, e.g. "s.2(1)"
)
```

### Example payload (one row, conceptual)

```json
{
  "name": "UK_ukpga_1974_37",
  "duty_holder": ["employer", "self-employed person", "designer", "manufacturer"],
  "rights_holder": ["employee", "safety representative"],
  "responsibility_holder": ["Secretary of State", "Health and Safety Executive"],
  "power_holder": ["inspector"],
  "duty_type": ["duty", "obligation", "requirement"],
  "role": ["employer", "employee", "inspector", "designer"],
  "role_gvt": ["Secretary of State", "Health and Safety Executive"],
  "duties": [
    {"holder": "employer", "duty_type": "duty", "clause": "ensure health safety and welfare at work of employees", "article": "s.2(1)"},
    {"holder": "employer", "duty_type": "duty", "clause": "prepare a written statement of general policy with respect to health and safety", "article": "s.2(3)"}
  ],
  "rights": [
    {"holder": "employee", "duty_type": "right", "clause": "not to be subjected to any detriment for carrying out designated activities", "article": "s.44(1)"}
  ],
  "responsibilities": [
    {"holder": "Secretary of State", "duty_type": "responsibility", "clause": "make regulations for any of the general purposes of this Part", "article": "s.15(1)"}
  ],
  "powers": [
    {"holder": "inspector", "duty_type": "power", "clause": "enter any premises which he has reason to believe it is necessary for him to enter", "article": "s.20(2)(a)"}
  ]
}
```

### Publication SQL (what fractalaw queries from DuckDB)

```sql
SELECT name, duty_holder, rights_holder, responsibility_holder, power_holder,
       duty_type, role, role_gvt,
       duties, rights, responsibilities, powers
FROM legislation
WHERE name = '{law_name}'
```

One message per law. The `--family` flag filters to laws in a regulatory family (e.g., "OH&S: Occupational Health and Safety").

---

## 3. Decoding Arrow IPC in Elixir

Arrow IPC is a binary columnar format. Elixir doesn't have a native Arrow library, so you need one of these approaches:

### Option A: Rust NIF (recommended)

Write a small Rust NIF that wraps `arrow` crate's `StreamReader`:

```rust
// native/arrow_nif/src/lib.rs
use arrow::ipc::reader::StreamReader;
use rustler::{NifResult, Binary, OwnedBinary, Env};
use std::io::Cursor;

#[rustler::nif]
fn decode_taxa_ipc(env: Env, bytes: Binary) -> NifResult<rustler::Term> {
    let reader = StreamReader::try_new(Cursor::new(bytes.as_slice()), None)
        .map_err(|e| rustler::Error::Term(Box::new(e.to_string())))?;

    let mut rows = vec![];
    for batch in reader {
        let batch = batch.map_err(|e| rustler::Error::Term(Box::new(e.to_string())))?;
        // Convert batch to list of maps for Elixir
        // ... (iterate rows, extract columns)
    }

    Ok(rustler::Term::from(rows))
}
```

This gives you zero-copy decoding and reuses the exact same Arrow crate fractalaw uses.

### Option B: Python sidecar via Erlport

```python
# priv/python/arrow_decode.py
import pyarrow as pa
import json

def decode_taxa_ipc(ipc_bytes):
    reader = pa.ipc.open_stream(ipc_bytes)
    table = reader.read_all()
    return table.to_pydict()
```

```elixir
{:ok, pid} = :python.start([{:python_path, 'priv/python'}])
result = :python.call(pid, :arrow_decode, :decode_taxa_ipc, [ipc_bytes])
```

### Option C: Explorer (Elixir DataFrame library)

[Explorer](https://hexdocs.pm/explorer/) wraps Polars (Rust) and can read Arrow IPC:

```elixir
# Explorer can read Arrow IPC from binary
df = Explorer.DataFrame.load_ipc!(ipc_bytes)
```

Check if `Explorer.DataFrame.load_ipc!/1` accepts raw bytes — if so, this is the simplest path. Explorer is mature and well-maintained.

---

## 4. Zenoh Subscriber (Elixir)

### Dependency

```elixir
# mix.exs
defp deps do
  [
    {:zenohex, "~> 1.1"},  # check hex.pm for latest matching zenoh 1.x
    # ...
  ]
end
```

### Minimal subscriber GenServer

```elixir
defmodule Sertantai.Zenoh.TaxaSubscriber do
  use GenServer
  require Logger

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  def init(opts) do
    tenant = Keyword.fetch!(opts, :tenant)

    {:ok, session} = Zenohex.open(Zenohex.Config.default())
    key_expr = "fractalaw/@#{tenant}/taxa/enrichment/*"
    {:ok, subscriber} = Zenohex.Session.declare_subscriber(session, key_expr)

    Logger.info("Subscribed to #{key_expr}")

    # Start listener loop in a linked task
    Task.start_link(fn -> listen(subscriber) end)

    {:ok, %{session: session, subscriber: subscriber, tenant: tenant}}
  end

  defp listen(subscriber) do
    case Zenohex.Subscriber.recv(subscriber) do
      {:ok, sample} ->
        handle_sample(sample)
        listen(subscriber)

      {:error, reason} ->
        Logger.warning("Subscriber recv error: #{inspect(reason)}")
    end
  end

  defp handle_sample(sample) do
    law_name = sample.key_expr |> String.split("/") |> List.last()
    ipc_bytes = sample.payload

    Logger.info("Received taxa for #{law_name} (#{byte_size(ipc_bytes)} bytes)")

    # Decode Arrow IPC → Elixir map
    case decode_arrow_ipc(ipc_bytes) do
      {:ok, taxa_data} ->
        # Persist to Postgres
        Sertantai.Laws.upsert_taxa(law_name, taxa_data)

      {:error, reason} ->
        Logger.error("Failed to decode taxa for #{law_name}: #{inspect(reason)}")
    end
  end

  defp decode_arrow_ipc(bytes) do
    # Use whichever option from Section 3 above
    # e.g., ArrowNif.decode_taxa_ipc(bytes)
  end
end
```

### Supervisor tree

```elixir
# application.ex
children = [
  # ... existing children ...
  {Sertantai.Zenoh.TaxaSubscriber, tenant: "acme"}
]
```

---

## 5. Zenoh Session Config

### Default (LAN multicast scouting)

For LAN testing where fractalaw and sertantai are on the same network, use **default config** — both sides will discover each other via multicast on `224.0.0.224:7446`:

```elixir
{:ok, session} = Zenohex.open(Zenohex.Config.default())
```

No explicit endpoints needed. Both peers auto-discover.

### Explicit connect (cross-network)

If the Hive has a known address:

```elixir
config = Zenohex.Config.default()
  |> Zenohex.Config.set_connect(["tcp/192.168.1.50:7447"])

{:ok, session} = Zenohex.open(config)
```

---

## 6. Testing the Integration

### Step 1: Sertantai subscribes (start first)

Start sertantai with the TaxaSubscriber in the supervisor tree. It will begin listening on `fractalaw/@acme/taxa/enrichment/*` via multicast scouting.

### Step 2: Fractalaw publishes

```bash
# Publish OH&S family laws (the enriched ones)
fractalaw sync publish --family "OH&S: Occupational Health and Safety" --tenant acme
```

This queries DuckDB for all laws in that family that have taxa data, serialises each as Arrow IPC, and publishes one message per law to:
```
fractalaw/@acme/taxa/enrichment/UK_ukpga_1974_37
fractalaw/@acme/taxa/enrichment/UK_ukpga_1999_31
...etc
```

### Step 3: Verify

Sertantai's subscriber should log receipt of each law's taxa. Check Postgres for the persisted data.

### Dry-run: zenoh subscriber without sertantai

You can verify the publish side works with a simple zenoh subscriber in any language. Python example:

```python
import zenoh

session = zenoh.open(zenoh.Config())
sub = session.declare_subscriber("fractalaw/@acme/taxa/enrichment/*")

while True:
    sample = sub.recv()
    print(f"Received: {sample.key_expr} ({len(sample.payload)} bytes)")
```

---

## 7. What Sertantai Should Persist

### Postgres schema suggestion

```sql
CREATE TABLE taxa_enrichment (
  law_name        TEXT PRIMARY KEY,
  duty_holder     TEXT[],
  rights_holder   TEXT[],
  responsibility_holder TEXT[],
  power_holder    TEXT[],
  duty_type       TEXT[],
  role            TEXT[],
  role_gvt        TEXT[],
  duties          JSONB,   -- array of {holder, duty_type, clause, article}
  rights          JSONB,
  responsibilities JSONB,
  powers          JSONB,
  received_at     TIMESTAMPTZ DEFAULT NOW(),
  updated_at      TIMESTAMPTZ DEFAULT NOW()
);
```

The `List<Utf8>` columns map naturally to Postgres `TEXT[]`. The `List<Struct>` columns are easiest as `JSONB` arrays.

Use `INSERT ... ON CONFLICT (law_name) DO UPDATE` for idempotent upserts — fractalaw may re-publish the same law after re-enrichment.

---

## 8. What Comes Next

1. **Get sertantai subscribing** (this document) — you are here
2. **Test with a real publish** — fractalaw publishes OH&S family taxa from DuckDB
3. **Phoenix LiveView** — wire taxa_enrichment table to the laws UI
4. **Late-joiner queryable** — sertantai serves taxa from Postgres to Bees that missed the publish
5. **CRDT coordination** — sertantai becomes the merge point for mutable metadata (Phase D proper)

---

## 9. Key Expressions Reference

| Key Expression | Publisher | Subscriber | Payload |
|---------------|-----------|-----------|---------|
| `fractalaw/@{t}/taxa/enrichment/{law}` | Hive (fractalaw) | Sertantai | Arrow IPC (this doc) |
| `fractalaw/@{t}/crdt/{doc}/updates` | Any peer | All peers | Loro incremental bytes |
| `fractalaw/@{t}/crdt/{doc}/snapshot` | Queryable (any peer) | Late-joiner | Loro snapshot bytes |
| `fractalaw/@{t}/data/legislation/*/meta` | Sertantai | Hive | JSON or Arrow IPC (future) |
| `fractalaw/@{t}/events/*` | Any peer | Any peer | JSON (future) |

---

## 10. Contact / Source

- Zenoh sync implementation: `crates/fractalaw-sync/src/zenoh_sync.rs`
- CRDT sync: `crates/fractalaw-sync/src/crdt_sync.rs`
- Hive orchestrator: `crates/fractalaw-sync/src/hive.rs`
- CLI publish command: `crates/fractalaw-cli/src/main.rs` → `cmd_sync_publish()`
- Full architecture plan: `.claude/plans/zenoh.md`
