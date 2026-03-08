# Session: Zenoh WAN Sync (#25)

**Date**: 2026-03-07
**Issue**: [#25 — Zenoh WAN sync: enable cross-network publish/subscribe](https://github.com/fractalaw/fractalaw/issues/25)
**Depends on**: Phases A–D complete (peer mode, LAN multicast, pub/sub + query/reply, Arrow IPC, Loro CRDTs)

## Problem

The current Zenoh sync pipeline (`sync watch`, `sync publish`, `sync pull-lat`) operates on LAN only. `ZenohSync::new()` uses `zenoh::Config::default()` — peer mode with multicast scouting on `224.0.0.224:7446`. This works when fractalaw and sertantai are on the same network. In production:

- **sertantai**: Hetzner VPS (public IP, always-on)
- **fractalaw**: Desktop PC on domestic LAN behind NAT router (no public IP, no inbound connections)

Multicast doesn't cross WAN. Fractalaw can't accept inbound connections. The current architecture is dead on arrival for this topology.

## Deployment Topology

```
Internet
    │
    ├── Hetzner VPS (public IP: X.X.X.X)
    │   └── sertantai (Elixir/Phoenix)
    │       └── zenoh peer (currently: multicast scouting)
    │
    └── Domestic Router (NAT, dynamic IP via ISP)
        └── LAN
            └── Desktop PC (192.168.x.x)
                └── fractalaw (Rust)
                    └── zenoh peer (currently: multicast scouting)
```

**Core constraint**: fractalaw initiates all connections outbound. Sertantai cannot reach in.

## Research Findings

### Current Zenoh Config (all 3 sync modules)

| Module | Constructor | Config |
|--------|------------|--------|
| `zenoh_sync.rs:178` | `ZenohSync::new()` | `zenoh::Config::default()` (peer, multicast) |
| `crdt_sync.rs:116` | `CrdtSync::new()` | `zenoh::Config::default()` (peer, multicast) |
| `hive.rs:126` | `HiveSync::with_config()` | Accepts custom `zenoh::Config` (already wired) |

All three already have `with_config()` variants that accept `zenoh::Config`. The plumbing exists — we just need to build and pass the right config.

### Zenoh WAN Options (from docs + plan)

**Option A: Zenoh Router on Hetzner (recommended)**

Run `zenohd` as a router on the VPS alongside sertantai. Fractalaw connects out to it as a client.

```
Hetzner VPS
├── sertantai (zenoh peer, connects to local zenohd)
└── zenohd (router, listens on tls/0.0.0.0:7447)
        │
        └──── fractalaw (client mode, connects to tls/X.X.X.X:7447)
```

- Fractalaw connects outbound (NAT-friendly)
- zenohd routes pub/sub between sertantai (peer) and fractalaw (client)
- TLS encrypts the WAN link
- Simple, well-documented, no NAT tricks

**Option B: Sertantai as Router (simpler, fewer processes)**

Sertantai's zenoh session runs in router mode instead of peer mode. Fractalaw connects directly.

- Fewer moving parts (no separate zenohd process)
- But: zenohex (Elixir NIF) may not support router mode — need to verify
- Coupling sertantai's zenoh lifecycle to the Phoenix process

**Option C: QUIC Transport (bonus)**

QUIC over UDP might help with NAT keep-alive and connection migration. Can be layered on top of Option A or B. Zenoh supports `quic/X.X.X.X:7447` endpoints natively. Note: QUIC streamed and datagram modes are incompatible — both sides must match.

### Zenoh TLS Configuration (from docs)

**Router (Hetzner VPS):**
```json5
{
    mode: "router",
    listen: { endpoints: ["tls/0.0.0.0:7447"] },
    transport: {
        link: {
            tls: {
                listen_private_key: "/etc/fractalaw/server.key.pem",
                listen_certificate: "/etc/fractalaw/server.cert.pem",
                root_ca_certificate: "/etc/fractalaw/ca.pem",
                enable_mtls: true,
                close_link_on_expiration: true
            }
        }
    }
}
```

**Client (fractalaw, behind NAT):**
```json5
{
    mode: "client",
    connect: { endpoints: ["tls/hetzner.example.com:7447"] },
    transport: {
        link: {
            tls: {
                root_ca_certificate: "/etc/fractalaw/ca.pem",
                enable_mtls: true,
                connect_private_key: "/etc/fractalaw/client.key.pem",
                connect_certificate: "/etc/fractalaw/client.cert.pem",
                close_link_on_expiration: true
            }
        }
    }
}
```

**Peer (sertantai on same VPS as router):**
```json5
{
    mode: "peer",
    connect: { endpoints: ["tcp/127.0.0.1:7447"] },
    scouting: { multicast: { enabled: false } }
}
```

### What Exists in Code Already

1. **`with_config()` on all sync structs** — custom zenoh::Config fully supported
2. **CLI `--tenant` flag** — tenant namespace already parameterized
3. **No `--endpoint` or `--connect` CLI flags** — need to add
4. **No config file support** — `zenoh::Config::from_file()` exists in the Zenoh API
5. **Phase E in zenoh plan** (.claude/plans/zenoh.md §11) — specifies mTLS, ACL, router mode, but no implementation yet

### What Needs to Change

**`fractalaw-sync` (library):**
- `ZenohSync::new()` should accept optional endpoint + TLS config, falling back to default peer mode for LAN
- Or: accept a `zenoh::Config` directly (the `with_config()` path already works)
- Config builder helper: given endpoint + cert paths, produce a `zenoh::Config`

**`fractalaw-cli` (binary):**
- New CLI flags: `--connect <endpoint>` (e.g., `tls/hetzner.example.com:7447`)
- Or: `--zenoh-config <path>` to load a JSON5 config file
- Or both: CLI flags for simple cases, config file for complex setups
- Apply to: `sync watch`, `sync publish`, `sync pull-lat`

**Hetzner VPS (sertantai side):**
- Install `zenohd` (standalone binary, ~15MB)
- Configure router mode + TLS
- Firewall: open port 7447/tcp
- Certificate generation (minica or similar)
- systemd service for zenohd

**Certificates:**
- Generate CA key/cert
- Generate server cert (for zenohd on Hetzner)
- Generate client cert (for fractalaw)
- Store securely, distribute out-of-band

## Proposed Implementation Phases

### Phase 1: Config Plumbing

Add `--connect` and `--zenoh-config` flags to the CLI sync subcommands. Build `zenoh::Config` from these flags. Pass through to `ZenohSync::with_config()` / `CrdtSync::with_config()`.

No TLS yet — test with plain TCP first (`tcp/X.X.X.X:7447`). This validates the connection topology before adding encryption.

**Files**: `fractalaw-cli/src/main.rs` (CLI args + config builder)

### Phase 2: TLS Transport

Add TLS cert path flags (`--tls-ca`, `--tls-cert`, `--tls-key`) or read them from the zenoh config file. Configure `transport.link.tls` in the `zenoh::Config`.

Test with self-signed certs (minica) over WAN.

**Files**: `fractalaw-cli/src/main.rs`, cert generation script

### Phase 3: Router Deployment

Deploy `zenohd` on Hetzner VPS. Configure sertantai to connect to local zenohd instead of using multicast. Test full round-trip: sertantai publishes event → zenohd routes → fractalaw receives, enriches, publishes back → zenohd routes → sertantai receives.

**Files**: deployment configs (zenohd.json5, systemd unit), sertantai zenoh config

### Phase 4: Resilience

- Auto-reconnect on WAN drop (zenoh handles this natively, but verify)
- Local queue for publishes when disconnected
- Graceful degradation: if WAN is down, continue enriching locally, publish when reconnected
- `--fallback-mode` or equivalent: detect WAN unreachability, switch to local-only

## Deferred

- **mTLS with ACL** (Phase E in zenoh plan): tenant-scoped access control rules on the router. Not needed for single-tenant dev.
- **QUIC transport**: Evaluate once TCP+TLS works. QUIC may improve NAT traversal resilience.
- **zenohd as embedded process**: Currently fractalaw embeds a zenoh peer session. Could embed a router, but adds complexity. Separate zenohd on VPS is simpler.
- **Certificate rotation**: Short-lived certs with auto-renewal. Not needed for initial deployment.

## Phase 1 Implementation: Config Plumbing

### Changes

#### `crates/fractalaw-cli/Cargo.toml`
- Added `zenoh = { workspace = true }` — needed for `zenoh::Config`, `from_json5()`, `from_file()`

#### `crates/fractalaw-cli/src/main.rs`

**New `ZenohArgs` struct** (flattened into all sync/CRDT subcommands):
- `--tenant` (env: `FRACTALAW_TENANT`, default: `local`) — moved from per-variant duplication
- `--connect` (env: `ZENOH_ENDPOINT`) — endpoint to connect to (e.g., `tcp/1.2.3.4:7447`)
- `--zenoh-config` (env: `ZENOH_CONFIG`) — path to JSON5 config file
- `--connect` and `--zenoh-config` are mutually exclusive (clap `conflicts_with`)

**`build_zenoh_config()` method** — single config builder:
- `--connect` → `from_json5()` with client mode, explicit endpoint, multicast disabled
- `--zenoh-config` → `from_file()`
- Neither → `Config::default()` (peer mode, multicast — LAN P2P preserved)

Note: `zenoh::Config` doesn't expose fields directly — uses `from_json5()` / `insert_json5()` API, not struct field access.

**Enum refactoring**: `SyncAction::Publish/PullLat/Watch` and `CrdtAction::Status/Create/Inspect/Save` all use `#[command(flatten)] zenoh: ZenohArgs` instead of individual `tenant: String` fields.

**7 handler functions updated**: `cmd_sync_publish`, `cmd_sync_pull_lat`, `cmd_sync_watch`, `cmd_crdt_status`, `cmd_crdt_create`, `cmd_crdt_inspect`, `cmd_crdt_save` — all take `zenoh: &ZenohArgs` and use `ZenohSync::with_config()` / `CrdtSync::with_config()`.

### Results

- 519 tests pass (all workspace)
- `--help` shows `--connect`, `--zenoh-config`, env vars
- Mutual exclusion works: `--connect` + `--zenoh-config` gives clap error
- Default (no flags) = LAN P2P unchanged
- No changes to `fractalaw-sync` library crate

## Status: Phase 1 complete, Phase 2 (TLS) pending
