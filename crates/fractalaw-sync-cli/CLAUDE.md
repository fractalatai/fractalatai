# fractalaw-sync-cli

Zenoh sync CLI binary. Handles all publish/subscribe communication with sertantai, LAT/LRT pulls, sync watch (long-running event loop), triage, and CRDT document management.

Binary name: `fractalaw-sync` (built from `fractalaw-sync-cli` crate).

## Commands

All commands run via: `cargo run -p fractalaw-sync-cli -- <command> [args]`

### Sync Watch (long-running)
```bash
cargo run -p fractalaw-sync-cli -- watch --tenant dev --connect tcp/127.0.0.1:7447
```
Subscribes to sertantai events and runs the full round-trip pipeline: pull LAT → enrich → publish back.
With `--pg`:
- LAT events are diff-applied against the manifest;
- `lat_deleted` only flags a delete candidate;
- the full manifest is re-compared at startup and every `--manifest-interval-mins` (default 60).

### Publish Enrichment (LRT from DuckDB)
```bash
cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/127.0.0.1:7447 --laws UK_ukpga_1974_37
cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/127.0.0.1:7447 --family "🏗️ Construction"
cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/127.0.0.1:7447 --changed
cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/127.0.0.1:7447 --all
cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/127.0.0.1:7447 --pending
```

### Publish Provisions (from Postgres)
```bash
cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/127.0.0.1:7447 --provisions --pg postgres://... --laws UK_ukpga_1974_37
```

### Publish Controls / Evidence
```bash
cargo run -p fractalaw-sync-cli -- publish-controls --tenant dev --connect tcp/127.0.0.1:7447 --laws UK_ukpga_1974_37
cargo run -p fractalaw-sync-cli -- publish-controls --tenant dev --connect tcp/127.0.0.1:7447 --qq
cargo run -p fractalaw-sync-cli -- publish-evidence --tenant dev --connect tcp/127.0.0.1:7447 --laws UK_ukpga_1974_37
cargo run -p fractalaw-sync-cli -- publish-evidence --tenant dev --connect tcp/127.0.0.1:7447 --qq
```

### Publish Secondary Sources (JSP/ACoP)
```bash
cargo run -p fractalaw-sync-cli -- publish-secondary --tenant dev --connect tcp/127.0.0.1:7447 --source-id JSP-375-CH23
cargo run -p fractalaw-sync-cli -- publish-secondary --tenant dev --connect tcp/127.0.0.1:7447 --all
```

### Publish Triage Results
```bash
cargo run -p fractalaw-sync-cli -- publish-triage --tenant dev --connect tcp/127.0.0.1:7447 --laws UK_ukpga_1974_37
cargo run -p fractalaw-sync-cli -- publish-triage --tenant dev --connect tcp/127.0.0.1:7447 --family "🏗️ Construction"
cargo run -p fractalaw-sync-cli -- publish-triage --tenant dev --connect tcp/127.0.0.1:7447 --all
```

### Pull LAT (manifest diff-apply, #62) / LRT
With `--pg`, `pull-lat` compares against legal's LAT manifest and diffs. It's a dry run unless `--apply`. See the `lat-sync` skill.
```bash
cargo run -p fractalaw-sync-cli -- --pg postgres://... pull-lat --tenant dev --connect tcp/127.0.0.1:7447 --stale            # report
cargo run -p fractalaw-sync-cli -- --pg postgres://... pull-lat --tenant dev --connect tcp/127.0.0.1:7447 --stale --apply --limit 30
cargo run -p fractalaw-sync-cli -- --pg postgres://... pull-lat --tenant dev --connect tcp/127.0.0.1:7447 --laws UK_ssi_2016_88 --apply
cargo run -p fractalaw-sync-cli -- --pg postgres://... pull-lat --tenant dev --connect tcp/127.0.0.1:7447 --archive-laws approved.txt
cargo run -p fractalaw-sync-cli -- --pg postgres://... pull-lat --restore-laws UK_uksi_2000_1973
cargo run -p fractalaw-sync-cli -- pull-lrt --tenant dev --connect tcp/127.0.0.1:7447 --laws UK_ukpga_1974_37
cargo run -p fractalaw-sync-cli -- pull-lrt --tenant dev --connect tcp/127.0.0.1:7447 --qq
```

### Pull / Push (HTTP)
```bash
cargo run -p fractalaw-sync-cli -- pull --url http://localhost:4000
cargo run -p fractalaw-sync-cli -- push --url http://localhost:4000
```

### List / Pull Secondary Sources
```bash
cargo run -p fractalaw-sync-cli -- list-secondary --tenant dev --connect tcp/127.0.0.1:7447
cargo run -p fractalaw-sync-cli -- list-secondary --tenant dev --connect tcp/127.0.0.1:7447 --source-type jsp --ids-only
cargo run -p fractalaw-sync-cli -- pull-secondary --tenant dev --connect tcp/127.0.0.1:7447 --source-id JSP-375-CH23
```

### Customer Laws
```bash
cargo run -p fractalaw-sync-cli -- customer-laws --tenant dev --connect tcp/127.0.0.1:7447 --list
cargo run -p fractalaw-sync-cli -- customer-laws --tenant dev --connect tcp/127.0.0.1:7447 --name QQ
cargo run -p fractalaw-sync-cli -- customer-laws --tenant dev --connect tcp/127.0.0.1:7447 --name QQ --output data/sertantai/qq-applicable-laws.csv
```

### Triage
```bash
cargo run -p fractalaw-sync-cli -- triage --laws UK_ukpga_1974_37 --pg postgres://...
cargo run -p fractalaw-sync-cli -- triage --family "🏗️ Construction" --pg postgres://... --publish --tenant dev --connect tcp/127.0.0.1:7447
cargo run -p fractalaw-sync-cli -- triage --all --pg postgres://...
```

### CRDT Documents
```bash
cargo run -p fractalaw-sync-cli -- crdt status --tenant dev --connect tcp/127.0.0.1:7447
cargo run -p fractalaw-sync-cli -- crdt create <DOC_ID> --tenant dev --connect tcp/127.0.0.1:7447
cargo run -p fractalaw-sync-cli -- crdt inspect <DOC_ID> --tenant dev --connect tcp/127.0.0.1:7447
cargo run -p fractalaw-sync-cli -- crdt save --tenant dev --connect tcp/127.0.0.1:7447
```

## Module Layout

- `src/main.rs` — Clap arg definitions, dispatch, triage + customer-laws commands
- `src/sync.rs` — All sync subcommand implementations (publish, pull, watch, CRDT)
- `src/lat_sync.rs` — LAT manifest sync: per-law verify → plan → diff-apply, delete verdicts, archive/restore (#62)

## Notes

- `--tenant dev` is required when publishing to sertantai (default is `local`)
- `--connect tcp/127.0.0.1:7447` connects to a local Zenoh router; omit for peer mode
- TLS endpoints (`tls/...` or `quic/...`) require `--tls-ca` and optionally `--tls-cert`/`--tls-key`
- `--pg` is required for any command that reads provision text (triage, publish --provisions)
- Environment variables: `FRACTALAW_TENANT`, `ZENOH_ENDPOINT`, `FRACTALAW_PG`
