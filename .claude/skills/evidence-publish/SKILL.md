---
description: Publish L4 Evidence patterns to sertantai via Zenoh. Single law, QQ corpus, or all laws.
---

# Evidence Publish

## When This Applies

When publishing generated evidence patterns from DuckDB `suggested_evidence` staging table to sertantai via Zenoh.

## Prerequisites

- Sertantai running with Zenoh peer (`mix phx.server` or systemd service)
- DuckDB with `suggested_evidence` table populated (run `/evidence-creation` first)
- Evidence patterns validated/post-processed (98.6% validated as of 2026-07-15)

## Quick Reference

```bash
# Publish single law (good for testing receiver)
cargo run -p fractalaw-sync-cli -- publish-evidence --laws UK_uksi_1997_1713 --tenant dev --connect tcp/localhost:7447

# Publish multiple laws
cargo run -p fractalaw-sync-cli -- publish-evidence --laws UK_uksi_1997_1713,UK_uksi_1999_3242 --tenant dev --connect tcp/localhost:7447

# Publish all QQ applicable laws
cargo run -p fractalaw-sync-cli -- publish-evidence --qq --tenant dev --connect tcp/localhost:7447

# Publish everything in staging
cargo run -p fractalaw-sync-cli -- publish-evidence --all --tenant dev --connect tcp/localhost:7447
```

## What Gets Published

Per law, one Arrow IPC payload on `fractalaw/@{tenant}/evidence/{law_name}` containing N rows (one per control). Each row has:

- `law_name`, `control_id`, `control_title` — identifiers
- `artefacts_json` — JSON array of artefact templates (Type-A + Type-B)
- Judgement fields: `needs_judgement`, `judgement_rationale`, `recommended_method`, `basis_guidance`, `discriminating_question`, `drift_signal`, `drift_conditions`
- Strategy fields: `voi_quadrant`, `voi_rationale`, `evidence_standard`, `recommended_interval`, `sample_size_guidance`, `staleness_tolerance`, `nature_strategy`
- Pipeline metadata: `generation_model`, `base_hash`

Full spec: `sertantai-legal/docs/zenoh/ZENOH-EVIDENCE-SPEC.md`

## Publish Order

Controls must be published before evidence — evidence patterns reference controls via `control_id` FK:

```
1. cargo run -p fractalaw-sync-cli -- publish-controls --qq --tenant dev --connect tcp/localhost:7447
2. cargo run -p fractalaw-sync-cli -- publish-evidence --qq --tenant dev --connect tcp/localhost:7447
```

## Check What's Available to Publish

```bash
/usr/bin/python3 -c "
import duckdb
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
total = conn.execute('SELECT count(*) FROM suggested_evidence').fetchone()[0]
laws = conn.execute('SELECT count(DISTINCT law_name) FROM suggested_evidence').fetchone()[0]
validated = conn.execute(\"SELECT count(*) FROM suggested_evidence WHERE status = 'validated'\").fetchone()[0]
flagged = conn.execute(\"SELECT count(*) FROM suggested_evidence WHERE status = 'flagged'\").fetchone()[0]
print(f'Laws: {laws}, Patterns: {total}, Validated: {validated}, Flagged: {flagged}')
conn.close()
"
```

## Tenant

Always use `--tenant dev` when publishing to sertantai. The CLI defaults to `local` which sertantai doesn't listen on.

## Troubleshooting

- **No peers connected (timeout)**: sertantai isn't running or isn't reachable. Check `mix phx.server` is up and Zenoh port 7447 is open.
- **0 patterns published**: the law has no rows in `suggested_evidence`. Run `/evidence-creation` first.
- **Sertantai doesn't receive**: check sertantai has a subscriber on `fractalaw/@dev/evidence/*`. See `ZENOH-EVIDENCE-SPEC.md` for the expected key expression.

## Corpus Stats (as of 2026-07-15)

- 220 QQ laws, 1,333 evidence patterns ready to publish
- 1,315 validated (98.6%), 18 flagged (1.4%)
