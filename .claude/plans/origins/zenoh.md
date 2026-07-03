# Zenoh Data Mesh: Connectivity, Sync & Multi-Tenancy

*2026-02-24 — Replacing the HTTP sync model with autonomous peer-to-peer data fabric*

## 1. The Paradigm Shift

The DRRP polisher session (02-21-26) modelled sertantai-to-fractalaw connectivity as HTTP request/response: `GET /api/outbox/annotations`, `POST /api/inbox/polished`. This is the wrong mental model. It implies:

- One side is "the server" and the other is "the client"
- Communication is synchronous and pull-based
- The hub must know the address of every peer
- Connectivity is assumed to be reliable and continuous

The correct model is **autonomous gossip sync** — peers (Bees) and hubs (Hives) form a data mesh where state propagates through intermittent pub/sub and CRDT merge operations. No node is "the server". Every node owns its data and publishes state changes when connectivity exists. Missing updates are recovered on reconnect.

**Zenoh** is the protocol that makes this work. It unifies pub/sub, query/reply, and storage into a single protocol stack designed for intermittent, heterogeneous networks spanning cloud-to-microcontroller.

### What Changes

| Aspect | Old (HTTP) | New (Zenoh) |
|--------|-----------|-------------|
| **Topology** | Client-server (hub GETs from sertantai) | Peer-to-peer mesh (all nodes equal) |
| **Communication** | Request/response, pull-based | Pub/sub + query/reply, push-based with pull recovery |
| **Connectivity** | Assumed reliable | Designed for intermittent |
| **Data format** | JSON over HTTP | Arrow IPC + Loro CRDT state vectors over Zenoh |
| **Conflict resolution** | Last-write-wins (implicit) | CRDTs with causal ordering (explicit) |
| **Discovery** | Hardcoded URLs | Multicast scouting + gossip |
| **Addressing** | REST endpoints | Zenoh key expressions with wildcard subscriptions |
| **Identity** | None (open endpoints) | mTLS certificates on Zenoh sessions |
| **Multi-tenancy** | Not modelled | Hermetic key expression namespaces |
| **Elixir integration** | Phoenix HTTP controllers | zenohex NIF — Elixir process per subscriber |

### What Stays the Same

- Arrow as the universal in-memory format
- Loro CRDTs for mutable metadata conflict resolution (already planned in fractal-plan.md §6.1.3)
- WIT interfaces for micro-app ↔ host communication (orthogonal to node-to-node sync)
- WASM micro-app sandboxing
- Audit trail (hash-chained, immutable)

---

## 2. Zenoh: The Protocol

[Eclipse Zenoh](https://zenoh.io/) (v1.7.2, January 2026) is a Rust-native protocol unifying **data in motion** (pub/sub), **data at rest** (storage/query), and **computations** (queryables).

### Why Zenoh (not MQTT, NATS, Arrow Flight, or raw gRPC)

| Requirement | Zenoh | MQTT | NATS | Arrow Flight |
|-------------|-------|------|------|-------------|
| P2P without broker | Yes (peer mode) | No (broker required) | No (server required) | No (client-server) |
| Embedded devices | zenoh-pico (C, µCs) | MQTT-SN | No | No |
| Intermittent connectivity | Native (auto-reconnect, miss detection) | Partial (QoS 1/2) | JetStream | No |
| Built-in storage | Yes (pluggable backends) | No | JetStream | No |
| Arbitrary binary payloads | Yes (ZBytes) | Yes | Yes | Arrow RecordBatch only |
| Key expression wildcards | Yes (`*`, `**`, `@` hermetic) | Yes (`+`, `#`) | Yes (`*`, `>`) | N/A |
| Rust-native async (tokio) | Yes | Via rumqttc | Via async-nats | Via tonic |
| Elixir binding | zenohex (NIF, v0.7.2) | emqtt | gnat | No native |
| Wire overhead | 5 bytes | ~8 bytes | ~12 bytes | gRPC framing |
| Latency (P2P) | 13-16 µs | Higher | Comparable | 100s µs (gRPC) |
| Throughput | 67 Gbps | ~38K msg/s | ~1M msg/s | 20+ Gbps/core |

**Arrow Flight is not replaced — it is complemented.** Flight remains the right choice for bulk analytical queries against DataFusion/DuckDB (Phase 4 federated query). Zenoh handles the real-time data fabric: event distribution, CRDT sync, sensor data, liveliness, and the gossip layer between Hives and Bees. Arrow IPC bytes flow over both — Zenoh for push, Flight for pull.

### Three Communication Patterns

```
┌─────────────────────────────────────────────────────────────┐
│                    Zenoh Protocol                            │
│                                                             │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │  Pub/Sub    │  │ Query/Reply  │  │  Storage           │  │
│  │             │  │              │  │  (Sub + Queryable) │  │
│  │  Publisher  │  │  Querier     │  │                    │  │
│  │  Subscriber │  │  Queryable   │  │  RocksDB, Memory,  │  │
│  │             │  │              │  │  InfluxDB, S3      │  │
│  │  Push data  │  │  Pull data   │  │  Persist + serve   │  │
│  │  on change  │  │  on demand   │  │  historical data   │  │
│  └─────────────┘  └──────────────┘  └───────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

- **Pub/Sub**: Bees publish sensor readings, CRDT updates, annotations. Hive subscribes to `tenant/@acme/**`.
- **Query/Reply**: Bee wakes up, queries Hive for "what changed since my last sync?" via queryable.
- **Storage**: Hive runs the storage plugin, persisting published data for late-joining or reconnecting Bees.

---

## 3. The Hive-Bee-Swarm Topology

### Naming

| Term | Role | Zenoh Mode | Example |
|------|------|-----------|---------|
| **Swarm** | Top-level coordination (optional cloud) | Router | Cloud VPS bridging geographically distributed Hives |
| **Hive** | Site/tenant hub, persistent state, heavy compute | Router | Bedroom box (Ryzen), or tenant's own server |
| **Sub-Hive** | Departmental or area hub within a site | Router or Peer | Floor-level aggregator in a large facility |
| **Bee** | Edge device, field tool, sensor gateway | Client or Peer | Tablet, laptop, RPi, embedded sensor |
| **Sertantai** | Elixir/Phoenix app (web UI, auth, Postgres) | Peer (via zenohex) | Always-on web server |

### Topology Diagram

```
                        ┌──────────────────┐
                        │     Swarm        │
                        │  (Cloud Router)  │  Optional — only needed
                        │  zenohd          │  for multi-site WAN
                        └────────┬─────────┘
                                 │ TCP/TLS (explicit connect)
                    ┌────────────┼────────────┐
                    │                         │
           ┌────────┴────────┐      ┌─────────┴───────┐
           │   Hive (Acme)   │      │  Hive (BigCorp)  │
           │   zenohd router │      │  zenohd router   │
           │   + DuckDB      │      │  + DuckDB        │
           │   + LanceDB     │      │  + ONNX          │
           │   + Wasmtime    │      │  + Wasmtime       │
           └──┬──────┬───┬───┘      └──┬───────┬───────┘
              │      │   │             │       │
         ┌────┘  ┌───┘   └───┐    ┌───┘       └────┐
         │       │           │    │                 │
    ┌────┴──┐ ┌──┴───┐ ┌────┴──┐ ┌──┴───┐    ┌────┴──┐
    │ Bee   │ │ Bee  │ │Sertantai│ │ Bee  │    │Sub-   │
    │tablet │ │laptop│ │(Elixir)│ │sensor│    │Hive   │
    │client │ │peer  │ │peer    │ │pico  │    │router │
    └───────┘ └──────┘ └───────┘ └──────┘    └──┬──┬─┘
                                                 │  │
                                            ┌────┘  └───┐
                                            │           │
                                       ┌────┴──┐  ┌────┴──┐
                                       │ Bee   │  │ Bee   │
                                       │floor-1│  │floor-2│
                                       └───────┘  └───────┘
```

### How Nodes Discover Each Other

1. **Local network (same LAN)**: Multicast scouting on `224.0.0.224:7446`. Bees discover the Hive automatically.
2. **Cross-network**: Bees configure `connect: { endpoints: ["tcp/hive.local:7447"] }` or use DNS-SD.
3. **Gossip propagation**: When a Bee connects to the Hive, the Hive gossips the existence of other peers. Bees can then form direct P2P connections for local data exchange (e.g., two tablets at the same site sharing inspection notes without routing through the Hive).
4. **WAN bridging**: Hives connect to the Swarm router via explicit TCP/TLS endpoints. The Swarm forwards subscriptions between Hives.

### Intermittent Connectivity Model

The Hive (bedroom box) is not always on. Sertantai (Elixir) is always on. Bees (field devices) have sporadic connectivity. This is the core design challenge.

```
Timeline:
  Sertantai: ████████████████████████████████████  (always on)
  Hive:      ░░░░████░░░░░░████░░░░████░░░░░░░░░  (intermittent)
  Bee:       ░░░░░░░░░░████░░░░░░░░░░░░░████░░░░  (sporadic)

When Hive wakes:
  1. Hive's zenohd starts → multicast scout → discovers Sertantai (peer)
  2. Zenoh advanced pub/sub: Hive's subscribers detect missed samples
  3. Late-joiner recovery: Hive queries Sertantai's caching publishers
  4. Loro CRDT merge: Hive imports state vectors accumulated by Sertantai
  5. Hive processes batch (run micro-apps: DRRP polisher, classifier, etc.)
  6. Hive publishes results → Sertantai subscribes and receives
  7. Hive goes back to sleep

When Bee reconnects:
  1. Bee connects to Hive (or Sertantai if Hive is off)
  2. Same recovery flow — missed samples, CRDT merge
  3. Bee gets enriched data (polished DRRP, classifications)
  4. Bee publishes field data (inspections, incidents)
  5. Bee disconnects
```

**Key Zenoh features enabling this:**

- **Advanced pub/sub** (`zenoh-ext`): Publishers cache samples, subscribers detect sequence gaps, late joiners query historical data.
- **Storage plugin**: Sertantai (always on) runs Zenoh storage, persisting all published data. When the Hive wakes, it queries storage for missed updates.
- **Auto-reconnect** (zenoh-pico, v1.3.3+): Embedded Bees automatically restore connections with declaration caching.

---

## 4. Key Expression Schema

Key expressions are Zenoh's addressing system. The hierarchy defines data organisation, access control boundaries, and subscription patterns.

### Namespace Design

```
fractalaw/@{tenant_id}/
├── data/
│   ├── legislation/
│   │   ├── {law_name}/meta          -- law metadata (LRT fields)
│   │   ├── {law_name}/text/{provision}  -- section text (LAT)
│   │   ├── {law_name}/drrp/{provision}  -- DRRP annotations
│   │   └── {law_name}/edges         -- amendment relationships
│   ├── site/
│   │   ├── {site_id}/compliance     -- compliance records
│   │   ├── {site_id}/incidents      -- incident reports
│   │   ├── {site_id}/permits        -- permit register
│   │   └── {site_id}/monitoring     -- sensor/monitoring data
│   └── ai/
│       ├── classifications/{law_name}  -- AI classification results
│       ├── embeddings/{law_name}/{provision}  -- vector embeddings metadata
│       └── centroids/{family}       -- classification centroids
├── crdt/
│   ├── {doc_id}/updates             -- incremental CRDT ops
│   ├── {doc_id}/vv                  -- version vector announcements
│   └── {doc_id}/snapshot            -- full snapshots for new peers
├── events/
│   ├── data-ingested                -- new data arrived
│   ├── regulatory-change            -- legislation changed
│   ├── compliance-gap               -- gap detected
│   └── polishing-complete           -- DRRP polishing done
├── liveliness/
│   ├── hive/{hive_id}              -- hive alive tokens
│   └── bee/{bee_id}                -- bee alive tokens
└── admin/
    ├── config                       -- pushed configuration
    └── status                       -- queryable status endpoints
```

### Hermetic Tenant Isolation

The `@` prefix on `{tenant_id}` creates a **hermetic namespace** — no wildcard pattern can match across tenants:

```
fractalaw/@acme/data/**        -- matches all Acme data
fractalaw/@bigcorp/data/**     -- matches all BigCorp data
fractalaw/@*/data/**           -- DOES NOT MATCH (@ chunks are hermetic)
```

A platform admin who needs cross-tenant visibility would subscribe to specific tenant namespaces explicitly, not via wildcards. This is enforced at the protocol level, not application logic.

### Subscription Patterns by Role

| Node | Subscribes To | Publishes To |
|------|--------------|-------------|
| **Hive** | `fractalaw/@{tenant}/**` (everything for its tenant) | `fractalaw/@{tenant}/data/ai/**`, `fractalaw/@{tenant}/events/**` |
| **Sertantai** | `fractalaw/@{tenant}/data/ai/**`, `fractalaw/@{tenant}/events/**` | `fractalaw/@{tenant}/data/legislation/**`, `fractalaw/@{tenant}/crdt/**/updates` |
| **Field Bee** | `fractalaw/@{tenant}/data/site/@{site}/**`, `fractalaw/@{tenant}/data/legislation/**` | `fractalaw/@{tenant}/data/site/@{site}/incidents`, `fractalaw/@{tenant}/data/site/@{site}/compliance` |
| **Sensor Bee** | None (publish-only) | `fractalaw/@{tenant}/data/site/@{site}/monitoring` |

---

## 5. Data Sync Over Zenoh

### Layer 1: Append-Only Data (Arrow IPC over Zenoh)

Legislation, inspections, monitoring data — immutable once written. No conflicts possible.

```rust
// Hive side: subscribe to legislation updates from Sertantai
let sub = session
    .declare_subscriber("fractalaw/@acme/data/legislation/*/meta")
    .await?;

while let Ok(sample) = sub.recv_async().await {
    let bytes: Vec<u8> = sample.payload().to_bytes().into_owned();
    // Deserialize Arrow IPC
    let reader = StreamReader::try_new(Cursor::new(bytes), None)?;
    for batch in reader {
        // Ingest into DuckDB
        duckdb.insert_batch("legislation", &batch?)?;
    }
}
```

```rust
// Sertantai side (via zenohex in Elixir, conceptual):
// After scraping new legislation:
//   1. Serialize to Arrow IPC (or JSON — Zenoh payloads are opaque bytes)
//   2. Publish to key expression
//   3. Any connected Hive/Bee receives immediately
//   4. Disconnected nodes recover via storage plugin on reconnect
```

Arrow IPC is the payload format for bulk data. For small metadata updates, JSON is fine — Zenoh doesn't care about payload format.

### Layer 2: Mutable Metadata (Loro CRDTs over Zenoh)

Risk assessments, site metadata, enforcement action status — mutable, concurrent edits possible.

```
Sync flow:
  1. Each node maintains a Loro Doc for mutable data
  2. On local edit: doc.commit() → export incremental update bytes
  3. Publish update: session.put("fractalaw/@acme/crdt/{doc_id}/updates", bytes)
  4. Subscribers receive and import: doc.import(bytes)
  5. Periodically publish version vector: session.put(".../vv", doc.oplog_vv().encode())
  6. On reconnect: query peer's VV, request missing updates, merge

Recovery for late joiners:
  1. New Bee declares queryable interest in "fractalaw/@acme/crdt/{doc_id}/snapshot"
  2. Hive (or Sertantai) responds with doc.export(Snapshot)
  3. Bee imports snapshot, then subscribes to incremental updates
```

**Loro's two-message sync protocol maps perfectly to Zenoh's query/reply:**

```rust
// Bee requests sync from Hive:
let replies = session.get(
    &format!("fractalaw/@acme/crdt/{}/sync", doc_id)
).payload(local_doc.oplog_vv().encode()).await?;

// Hive's queryable responds with missing updates:
while let Ok(reply) = replies.recv_async().await {
    if let Ok(sample) = reply.result() {
        let bytes = sample.payload().to_bytes();
        local_doc.import(&bytes)?;
    }
}
```

### Layer 3: AI-Generated Data (Recompute, Don't Sync)

Embeddings, classifications — deterministic outputs from deterministic inputs. Each node regenerates locally from source text using its local ONNX model. Trade compute for bandwidth. Only sync the classification labels and confidence scores (small), not the embedding vectors (large).

### Layer 4: Events (Pub/Sub Notifications)

Micro-app events (`data-ingested`, `regulatory-change`, `polishing-complete`) are published to `fractalaw/@{tenant}/events/{event_type}`. Any node subscribed to that key expression reacts. This replaces the planned `fractal:events/emit` WIT interface's delivery mechanism — the WIT interface remains (micro-apps call it), but the host now publishes the event to Zenoh instead of an internal bus.

---

## 6. Sertantai's Role: Elixir as the Concurrency & Auth Layer

### Why Elixir and Rust Are Complementary

| Concern | Sertantai (Elixir) | Fractalaw (Rust) |
|---------|-------------------|------------------|
| **Strength** | Massive concurrency (1000s simultaneous users), fault tolerance (OTP supervisors), web UI, auth | Raw compute, AI inference, columnar analytics, WASM sandboxing |
| **Data store** | Postgres (ACID, auth, users, permissions, web state) | DuckDB + LanceDB (analytics, vectors, embeddings) |
| **Runtime** | BEAM VM — millions of lightweight processes | Tokio — async I/O with native threads for compute |
| **Connectivity** | Always-on web server, handles user sessions | Intermittent hub, batch processing |
| **Role in mesh** | Auth gateway, tenant router, CRDT coordinator | Compute engine, AI pipeline, micro-app host |

### Sertantai as Zenoh Peer

Sertantai joins the Zenoh mesh as a **peer** via zenohex (NIF binding, v0.7.2):

```elixir
# In application supervisor
defmodule Sertantai.ZenohSupervisor do
  use Supervisor

  def start_link(opts) do
    Supervisor.start_link(__MODULE__, opts, name: __MODULE__)
  end

  def init(_opts) do
    children = [
      {Sertantai.Zenoh.Session, []},
      {Sertantai.Zenoh.LegislationPublisher, []},
      {Sertantai.Zenoh.AnnotationPublisher, []},
      {Sertantai.Zenoh.ResultSubscriber, []},
      {Sertantai.Zenoh.CrdtCoordinator, []}
    ]
    Supervisor.init(children, strategy: :one_for_one)
  end
end
```

```elixir
# Publishing DRRP annotations (replaces HTTP outbox)
defmodule Sertantai.Zenoh.AnnotationPublisher do
  use GenServer

  def init(_) do
    {:ok, session} = Zenohex.Session.open()
    {:ok, pub} = Zenohex.Session.declare_publisher(
      session,
      "fractalaw/@acme/data/legislation/*/drrp/*"
    )
    {:ok, %{session: session, publisher: pub}}
  end

  # Called when Elixir's DRRP regex engine produces new annotations
  def handle_cast({:publish_annotation, law_name, provision, annotation_json}, state) do
    key = "fractalaw/@acme/data/legislation/#{law_name}/drrp/#{provision}"
    :ok = Zenohex.Publisher.put(state.publisher, annotation_json)
    {:noreply, state}
  end
end
```

```elixir
# Subscribing to polished results (replaces HTTP inbox)
defmodule Sertantai.Zenoh.ResultSubscriber do
  use GenServer

  def init(_) do
    {:ok, session} = Zenohex.Session.open()
    {:ok, sub} = Zenohex.Session.declare_subscriber(
      session,
      "fractalaw/@acme/data/ai/**"
    )
    {:ok, %{session: session, subscriber: sub}}
  end

  def handle_info({:zenoh_sample, sample}, state) do
    # Decode and store in Postgres for web UI
    case decode_key(sample.key_expr) do
      {:classification, law_name} ->
        Sertantai.Laws.update_classification(law_name, sample.payload)
      {:polished_drrp, law_name, provision} ->
        Sertantai.Laws.update_polished_drrp(law_name, provision, sample.payload)
      _ -> :ok
    end
    {:noreply, state}
  end
end
```

### Sertantai Responsibilities in the Mesh

1. **Authentication & authorisation**: User login, JWT/session tokens, RBAC. Zenoh ACLs reference TLS certificate CNs, but user-level auth remains in Elixir/Phoenix.

2. **Tenant management**: Create/configure tenant namespaces. Sertantai is the control plane that provisions `fractalaw/@{tenant}/` key spaces and issues certificates for Hives and Bees.

3. **Postgres as source of truth for web state**: User accounts, permissions, UI state, annotation workflow status. This data doesn't flow through Zenoh — it's Elixir-internal.

4. **CRDT coordination**: Sertantai maintains Loro docs for shared mutable state (risk assessments, enforcement status). As the always-on node, it's the reliable merge point. When the Hive wakes, it syncs CRDTs with Sertantai. When Bees connect to Sertantai directly (Hive offline), Sertantai can serve as a relay.

5. **Legislation publication**: After scraping legislation.gov.uk, Sertantai publishes new/changed legislation to Zenoh. The Hive (when awake) and Bees (when connected) receive these updates.

6. **Massive concurrency**: 1000s of users browsing legislation, running searches, viewing compliance dashboards — Elixir/Phoenix handles this with BEAM's lightweight processes. Fractalaw doesn't need to serve web traffic.

---

## 7. The Hive: Router Bridging Postgres and Zenoh Mesh

### Hive Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Hive (Rust)                            │
│                                                          │
│  ┌──────────────────────────────────────────────────┐    │
│  │              zenohd (Router)                      │    │
│  │  ┌──────────┐  ┌───────────┐  ┌──────────────┐  │    │
│  │  │ Access   │  │ Liveliness│  │ (no Zenoh  │  │    │
│  │  │ Control  │  │ Monitor   │  │  storage   │  │    │
│  │  │ (ACL)    │  │           │  │  plugin)   │  │    │
│  │  └──────────┘  └───────────┘  └──────────────┘  │    │
│  └───────────────────────┬──────────────────────────┘    │
│                          │                               │
│  ┌───────────────────────┴──────────────────────────┐    │
│  │              Sync Engine                          │    │
│  │  ┌──────────┐  ┌───────────┐  ┌──────────────┐  │    │
│  │  │ Zenoh    │  │ Loro CRDT │  │ Arrow IPC    │  │    │
│  │  │ Session  │  │ Merge     │  │ Codec        │  │    │
│  │  │ (peer)   │  │ Engine    │  │              │  │    │
│  │  └──────────┘  └───────────┘  └──────────────┘  │    │
│  └───────────────────────┬──────────────────────────┘    │
│                          │                               │
│  ┌───────────────────────┴──────────────────────────┐    │
│  │              Fractal Core                         │    │
│  │  Wasmtime │ DuckDB │ LanceDB │ ONNX │ DataFusion │    │
│  └──────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

### Hive as Router

The Hive runs `zenohd` (Zenoh router daemon) as an embedded process or a sidecar. This means:

- Bees in client mode connect to the Hive's router
- Bees in peer mode discover the Hive via multicast scouting
- The Hive bridges its local Bee network to the Swarm (if configured)
- The Hive does **not** run a Zenoh storage plugin — DuckDB/LanceDB are its persistence (see §13, Resolved Q1)
- Access control rules enforce tenant isolation at the router level

### Hive Lifecycle (Intermittent Operation)

```
1. WAKE
   └─ Start zenohd router
   └─ Open Zenoh session (peer mode, connects to Sertantai and Swarm)
   └─ Scouting: discover local Bees via multicast
   └─ Late-joiner recovery: query Sertantai's storage for missed updates
   └─ Loro CRDT merge: exchange version vectors, import missing ops

2. PROCESS
   └─ Run micro-app pipeline:
      └─ DRRP Polisher: process new annotations
      └─ Classifier: classify new legislation
      └─ Centroid Trainer: recompute centroids if needed
   └─ Publish results to Zenoh key expressions
   └─ Sertantai (if connected) receives results immediately
   └─ Bees (if connected) receive relevant partitions

3. SLEEP
   └─ Publish liveliness token withdrawal (automatic on session close)
   └─ zenohd shuts down gracefully
   └─ All cached data persists locally in DuckDB/LanceDB
   └─ Sertantai notes Hive is offline, continues serving users from Postgres
```

### Swarm Management

Big tenants get their own dedicated Hive. The Swarm (optional cloud router) bridges multiple Hives:

```
Swarm: zenohd router at swarm.fractalaw.io:7447

Hive provisioning:
  1. Sertantai (control plane) creates tenant: @bigcorp
  2. Sertantai issues mTLS certificate for BigCorp's Hive
  3. BigCorp Hive configures: connect to swarm.fractalaw.io:7447
  4. Swarm ACL: BigCorp cert → allow fractalaw/@bigcorp/**
  5. BigCorp's Bees connect to BigCorp's Hive (never to Swarm directly)
  6. Cross-tenant data never flows — hermetic namespaces enforce isolation

Sub-Hive provisioning:
  1. BigCorp has 3 facilities: London, Manchester, Edinburgh
  2. Each facility gets a Sub-Hive (smaller box, Zenoh router)
  3. Sub-Hives connect to BigCorp's main Hive
  4. Facility-local Bees connect to their Sub-Hive
  5. Data flows: Bee → Sub-Hive → Hive → Swarm → Sertantai (and reverse)
  6. But: facility-local data stays local unless the access pattern requires it
```

---

## 8. DRRP Polisher: Rewritten for Zenoh

The DRRP polisher session's HTTP sync becomes Zenoh pub/sub. The micro-app itself doesn't change — it still uses WIT interfaces (`fractal:data/query`, `fractal:ai/inference`, `fractal:data/mutate`). What changes is how data arrives and results depart.

### Old Flow (HTTP)

```
Sertantai                           Hive
    │                                  │
    │◄── GET /api/outbox/annotations ──│  (pull)
    │── JSON response ────────────────>│
    │                                  │  (process)
    │◄── POST /api/inbox/polished ─────│  (push)
    │── 200 OK ───────────────────────>│
```

### New Flow (Zenoh)

```
Sertantai                           Hive
    │                                  │
    │── pub: .../drrp/{provision} ────>│  (push on scrape — auto-received)
    │   (Sertantai's advanced pub/sub  │
    │    caches if Hive is offline)    │
    │                                  │  (Hive wakes, late-joiner recovery
    │                                  │   from Sertantai's publisher cache)
    │                                  │  (process: DRRP polisher runs)
    │◄── pub: .../ai/polished/... ────│  (push results — auto-received)
    │                                  │
    │   (No HTTP. No endpoints.        │
    │    No polling. No outbox.)       │
```

### What Changes in fractalaw-sync

The `fractalaw-sync` crate's `http.rs` module (`SyncClient`) becomes a `zenoh.rs` module:

```rust
// crates/fractalaw-sync/src/zenoh.rs (conceptual)

use zenoh::Session;

pub struct ZenohSync {
    session: Session,
    tenant: String,
}

impl ZenohSync {
    pub async fn new(tenant: &str) -> Result<Self> {
        let session = zenoh::open(zenoh::Config::default()).await?;
        Ok(Self { session, tenant: tenant.to_string() })
    }

    /// Subscribe to incoming annotations from Sertantai
    pub async fn subscribe_annotations(&self) -> Result<zenoh::pubsub::Subscriber<()>> {
        let key = format!("fractalaw/@{}/data/legislation/*/drrp/*", self.tenant);
        Ok(self.session.declare_subscriber(&key).await?)
    }

    /// Publish polished DRRP results
    pub async fn publish_polished(&self, law_name: &str, provision: &str, ipc_bytes: &[u8]) -> Result<()> {
        let key = format!(
            "fractalaw/@{}/data/ai/polished/{}/{}",
            self.tenant, law_name, provision
        );
        self.session.put(&key, ipc_bytes).await?;
        Ok(())
    }

    /// Query for missed annotations since last sync (late-joiner recovery)
    pub async fn recover_missed(&self) -> Result<Vec<Vec<u8>>> {
        let key = format!("fractalaw/@{}/data/legislation/**/drrp/**", self.tenant);
        let replies = self.session.get(&key).await?;
        let mut results = Vec::new();
        while let Ok(reply) = replies.recv_async().await {
            if let Ok(sample) = reply.result() {
                results.push(sample.payload().to_bytes().into_owned());
            }
        }
        Ok(results)
    }
}
```

### CLI Changes

```bash
# Old:
fractalaw sync pull --url http://localhost:4000
fractalaw sync push --url http://localhost:4000

# New:
fractalaw sync start                    # Start Zenoh session, subscribe, auto-sync
fractalaw sync status                   # Show connected peers, pending data
fractalaw sync once                     # One-shot: connect, sync, process, disconnect
```

The `sync pull` / `sync push` commands become unnecessary — sync is continuous when connected, and recovery is automatic on reconnect. `sync once` preserves the batch workflow for the intermittent Hive: wake → sync → process → publish → sleep.

---

## 9. Multi-Tenancy Architecture

### Small Tenants: Shared Hive

Small tenants share a Hive (the operator's bedroom box). Tenant isolation is enforced by:

1. **Hermetic key expressions**: `fractalaw/@tenant-a/**` and `fractalaw/@tenant-b/**` are protocol-level isolated
2. **Zenoh ACL**: Each tenant's Bees have certificates with tenant-scoped CN → ACL rules restrict to their namespace
3. **DuckDB schema separation**: Each tenant's data in separate DuckDB schemas or databases
4. **LanceDB dataset separation**: Separate Lance datasets per tenant

```
Shared Hive (bedroom box)
├── zenohd router
│   ├── ACL: cert(tenant-a) → allow fractalaw/@tenant-a/**
│   └── ACL: cert(tenant-b) → allow fractalaw/@tenant-b/**
├── DuckDB
│   ├── tenant_a.legislation
│   └── tenant_b.legislation
└── LanceDB
    ├── tenant_a_legislation_text/
    └── tenant_b_legislation_text/
```

### Big Tenants: Dedicated Hive

Big tenants get their own Hive hardware. The Swarm router bridges them to Sertantai:

```
Sertantai (always-on, manages all tenants)
    │
    ├── Zenoh peer: publishes to fractalaw/@bigcorp/data/legislation/**
    │                subscribes to fractalaw/@bigcorp/data/ai/**
    │
    └── connects to Swarm router
         │
         └── BigCorp Hive (dedicated hardware, BigCorp's premises)
              ├── zenohd router (connects to Swarm)
              ├── Full DuckDB + LanceDB + ONNX + Wasmtime
              └── BigCorp's Bees connect here (never to shared infrastructure)
```

### Sub-Hive Scaling

Within a big tenant, Sub-Hives provide hierarchical data locality:

```
BigCorp Hive (HQ)
├── Complete legislation corpus
├── Cross-site analytics
└── AI model training

    ├── Sub-Hive: London Factory
    │   ├── London-relevant legislation partition
    │   ├── London site compliance data
    │   └── London Bees (sensors, tablets)
    │
    ├── Sub-Hive: Manchester Warehouse
    │   ├── Manchester-relevant legislation partition
    │   ├── Manchester site compliance data
    │   └── Manchester Bees
    │
    └── Sub-Hive: Edinburgh Office
        ├── Scottish legislation partition (devolved jurisdiction)
        ├── Edinburgh site compliance data
        └── Edinburgh Bees
```

Each Sub-Hive holds only the data partition relevant to its site. The Hive holds everything. Sync flows down (legislation updates, AI results) and up (field data, incidents).

---

## 10. Security Considerations

### Transport Security

Zenoh supports mTLS natively. All node-to-node communication is encrypted:

```json5
// zenohd config for Hive router
{
    mode: "router",
    listen: {
        endpoints: ["tls/0.0.0.0:7447"]
    },
    transport: {
        link: {
            tls: {
                root_ca_certificate: "/etc/fractalaw/ca.pem",
                server_certificate: "/etc/fractalaw/hive.pem",
                server_private_key: "/etc/fractalaw/hive.key",
                client_auth: true  // mTLS: require client certificates
            }
        }
    },
    access_control: {
        enabled: true,
        default_permission: "deny",
        rules: [
            {
                id: "acme-tenant",
                messages: ["put", "get", "declare_subscriber", "declare_queryable"],
                key_exprs: ["fractalaw/@acme/**"],
                flows: ["ingress", "egress"]
            }
        ],
        subjects: [
            {
                id: "acme-devices",
                cert_common_names: ["acme-*"]
            }
        ],
        policies: [
            {
                rules: ["acme-tenant"],
                subjects: ["acme-devices"]
            }
        ]
    }
}
```

### Certificate Management

Sertantai acts as the certificate authority (CA) for the Zenoh mesh:

1. Sertantai generates root CA keypair
2. When a Hive or Bee is enrolled, Sertantai signs a certificate with tenant-scoped CN (e.g., `acme-hive-01`, `acme-bee-tablet-03`)
3. Zenoh ACL rules match on certificate CN
4. Short-lived certificates (72h) with automated renewal via Zenoh queryable on the admin key space

### Data Sovereignty

Key expression prefixes can encode geographic constraints:

```
fractalaw/@acme/eu/data/**      -- EU-only data, never syncs outside EU
fractalaw/@acme/global/data/**  -- Global data, syncs everywhere
```

Zenoh ACL rules on the Swarm router enforce geographic fencing — a Hive in the US cannot subscribe to `fractalaw/@acme/eu/**`.

---

## 11. Implementation Roadmap

### Phase A: Zenoh Foundation (fractalaw-sync)

1. Add `zenoh` crate dependency to `fractalaw-sync` (feature-gated: `zenoh`)
2. Implement `ZenohSync` struct: session management, pub/sub, query/reply
3. Key expression builder with tenant namespace support
4. Arrow IPC ↔ Zenoh payload serialization helpers
5. Unit tests with in-process Zenoh peers (no external daemon needed)

### Phase B: CRDT Integration

6. Loro docs for mutable metadata (risk assessments, enforcement status)
7. CRDT sync protocol over Zenoh: version vector exchange, update publish, snapshot query
8. Merge engine: import updates, resolve conflicts, emit change events
9. Integration with DuckDB: CRDT-backed mutable tables

### Phase C: Hive Router

10. zenohd configuration for the Hive (router mode, storage plugin, ACL)
11. Hive lifecycle: wake → sync → process → publish → sleep
12. Late-joiner recovery: query storage for missed samples
13. `fractalaw sync start` / `fractalaw sync once` CLI commands

### Phase D: Sertantai Integration (Elixir Side)

14. Add `zenohex` dependency to sertantai
15. Zenoh supervisor tree: session, publishers, subscribers
16. Replace HTTP outbox/inbox with Zenoh pub/sub
17. CRDT coordinator: Loro doc management, merge on receive
18. Phoenix PubSub bridge: Zenoh samples → LiveView updates

### Phase E: Multi-Tenancy & Security

19. Hermetic key expression namespaces per tenant
20. mTLS certificate generation and management in Sertantai
21. Zenoh ACL configuration per tenant
22. Swarm router deployment for cross-Hive bridging

### Phase F: Edge Bees

23. Bee configuration: client mode, connect to Hive
24. Partition affinity: subscribe only to relevant key expressions
25. Offline outbox: queue publishes when disconnected, drain on reconnect
26. zenoh-pico evaluation for sensor gateways

### Relationship to Existing Phases

This work cuts across the existing Phase 4 (Distribution) from `fractal-plan.md`. The key difference: Phase 4 assumed Arrow Flight as the primary transport. Zenoh becomes the primary transport for real-time sync and events, while Arrow Flight remains available for bulk analytical queries via DataFusion federation. The Loro CRDT integration (already planned in Phase 4) now has a concrete transport layer.

The HTTP sync code in `fractalaw-sync/src/http.rs` and the `sync pull`/`sync push` CLI commands become deprecated once Zenoh sync is operational. They can remain as a fallback for environments where Zenoh deployment isn't feasible.

---

## 12. Zenoh Crate Dependencies

```toml
# crates/fractalaw-sync/Cargo.toml (additions)
[dependencies]
zenoh = { version = "1.7", optional = true }
zenoh-ext = { version = "1.7", optional = true }  # advanced pub/sub

[features]
zenoh = ["dep:zenoh", "dep:zenoh-ext"]
```

```toml
# sertantai mix.exs (Elixir side)
defp deps do
  [
    {:zenohex, "~> 0.7.2"},
    # ... existing deps
  ]
end
```

Zenoh is pure Rust — no C/C++ dependencies, no OpenSSL. Builds cleanly with the existing toolchain. The `zenoh` crate is ~2.5MB compiled. zenoh-pico (for embedded Bees) is a separate C library, not needed in the Rust workspace.

---

## 13. Open Questions

### Resolved

**Q1. Zenoh storage backend for Hive — RocksDB or custom DuckDB/LanceDB backend?**

**Answer: Neither. No Zenoh storage plugin on the Hive.**

RocksDB is the wrong fit. Its LSM-tree architecture produces 10-30x write amplification (leveled compaction), compaction I/O storms that can stall writes for minutes, and significant SSD wear. This directly contradicts the Fractal architecture's principle of minimised I/O. The Hive is an intermittent bedroom box, not a Facebook-scale write engine.

More fundamentally, the Hive already has DuckDB and LanceDB as its persistence layer. Adding RocksDB as a Zenoh storage backend would create a redundant intermediate store — data arrives via Zenoh, gets written to RocksDB, then read from RocksDB and written to DuckDB/LanceDB. Three writes instead of one.

Instead:
- **Hive persistence = DuckDB + LanceDB directly.** The Zenoh sync engine receives samples and writes them straight to the application stores. No intermediate Zenoh storage layer.
- **Rehydration on wake = Sertantai.** Sertantai is always-on and owns the god data (Postgres). When the Hive wakes, it recovers missed updates from Sertantai via advanced pub/sub (publisher-side caching + late-joiner recovery) or, if the cache is exhausted, Sertantai republishes from Postgres. Sertantai is the source of truth, not a Zenoh storage backend.
- **Option A** (advanced pub/sub caching on Sertantai, no storage plugin anywhere) or **Option B** (memory-backed Zenoh storage on Sertantai only, as a buffer for sleeping Hives) — decision deferred until we have real usage patterns. Both avoid RocksDB entirely.

A custom Zenoh backend writing directly to DuckDB/LanceDB remains a future possibility (the `zenoh_backend_traits` crate supports it), but the traits are marked unstable and the indirection is unnecessary when the sync engine can write to the stores directly.

**Q2. Arrow IPC vs JSON over Zenoh?**

**Answer: Both — match the format to the data shape.**

- **JSON** for metadata and single-record updates: LRT-shaped data (one law's classification, one DRRP annotation, one compliance status change). This is what Sertantai produces naturally from Postgres and what Elixir serializes trivially. Small payloads where Arrow IPC schema overhead would exceed the data itself.
- **Arrow IPC** for bulk/columnar data: LAT-shaped data (batches of legislation text with embeddings), batch classification results, monitoring time-series, anything where you're moving hundreds or thousands of records. Arrow IPC gives zero-copy deserialization into DuckDB/LanceDB on the receiving end.

Zenoh's `Encoding` metadata field signals the format per-message (`Encoding::APPLICATION_JSON` vs `Encoding::APPLICATION_OCTET_STREAM` with a custom suffix like `+arrow-ipc`). The receiver inspects encoding and dispatches to the appropriate deserializer. No negotiation needed — the publisher decides based on the data shape.

**Q3. Sertantai as relay when Hive is offline?**

**Answer: Sertantai relays what it owns. Hive-only data waits for the Hive.**

Sertantai exposes Zenoh queryables backed by Postgres for the data it naturally holds:
- Legislation metadata (LRT) — scraped from legislation.gov.uk
- Legislation text (LAT) — section text
- DRRP annotations — regex-flagged provisions
- Polished DRRP — received from Hive, stored in Postgres for web UI

When a Bee connects while the Hive is asleep, the Bee can recover this data from Sertantai's queryables. Sertantai doesn't need a Zenoh storage plugin — Postgres is its persistence layer, just as DuckDB/LanceDB are the Hive's.

Sertantai does **not** relay Hive-only data: AI classifications, centroids, embedding metadata, compliance gap analyses, risk scores, or any micro-app pipeline output that only exists in the Hive's stores. Mirroring these into Postgres would be building a shadow Hive — wrong use of time, wrong architecture. If a Bee needs Hive-only data and the Hive is asleep, the Bee works from its local partition (synced during its last connection to the Hive). This is offline-first working as designed.

**For Bee → Hive field data** (inspections, incidents published while Hive sleeps): Sertantai subscribes to `fractalaw/@{tenant}/**` for its web UI, so it receives and persists the Bee's publications to Postgres. When the Hive wakes, it pulls from Sertantai's queryable. The Bee doesn't need to know or care whether the Hive is awake.

**Q4. zenohex maturity — is it enough for Sertantai?**

**Answer: Yes. Keep Sertantai's Zenoh usage simple — basic pub/sub + queryables.**

zenohex (v0.7.2) supports the two things Sertantai needs:
- **Pub/sub**: Publish legislation updates, DRRP annotations. Subscribe to AI results from the Hive.
- **Queryables**: Serve data to late-joining Hives and Bees on demand.

Sertantai does **not** need advanced pub/sub (caching, miss detection, sequence numbers). Those are Hive-side concerns handled by `zenoh-ext` in Rust. Sertantai is the always-on node — it doesn't miss messages, it doesn't need recovery. It publishes when it has new data, and it answers queries when asked.

**Leverage Elixir's strengths**: Thousands of Bees hitting Sertantai's queryables concurrently is trivial for the BEAM — one lightweight process per query, no thread pool exhaustion, no connection limits. The hot path should serve legislation from ETS (in-memory cache) or pre-loaded GenServer state, not a Postgres round-trip per request. Postgres is for writes and cold reads. ETS or :persistent_term for the frequently-queried legislation corpus that Bees will request repeatedly.

The Rust sidecar fallback is unnecessary unless zenohex proves buggy in production. Given that zenohex tracks Zenoh 1.7.x closely (21 releases, 613 commits) and the features we need are basic, the risk is acceptable.

**Q5. Zenoh shared memory for co-located processes?**

**Answer: Not applicable.** The premise assumed Sertantai and the Hive might run on the same machine (dev mode). They won't — Sertantai is an Elixir app with its own dev environment, the Hive is a Rust binary. They communicate over the network even in development. Zenoh SHM is irrelevant here.

**Q6. WASI 0.3 async and Zenoh — should micro-apps hold Zenoh subscribers?**

**Answer: No.** The host manages all Zenoh sessions and bridges data to guests via WIT interfaces. Guests should not have network access — this is a core security property of the WASM sandbox. WASI 0.3 async doesn't change this design decision.
