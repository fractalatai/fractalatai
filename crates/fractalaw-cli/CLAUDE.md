# fractalaw-cli

Binary entry point. Depends on all crates with all features enabled.

## Commands

### Taxa Pipeline
```bash
fractalaw taxa parse --pg <PG_URL> --laws <LAWS>        # Regex extraction → provision_actors
fractalaw taxa classify --pg <PG_URL> --laws <LAWS>     # DRRP classifier → provision_actors
fractalaw taxa reconcile --pg <PG_URL> --laws <LAWS>    # Resolve tier disagreements
fractalaw taxa infer --pg <PG_URL> --laws <LAWS>        # Hohfeldian correlative inference
fractalaw taxa enrich [--force] [--family <FAM>]        # Full enrichment pipeline
fractalaw taxa backfill --pg <PG_URL> --laws <LAWS>     # Aggregate actors → DuckDB, compute significance
fractalaw taxa qa [--laws <LAWS>] [--family <FAM>]      # QA validation report
fractalaw taxa status [--law-file <CSV>] [--summary]    # Enrichment status overview
fractalaw taxa eyeball --pg <PG_URL> --laws <LAWS>      # Human-readable provision dump
```

### Sync / Publish
```bash
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --laws <LAWS>            # Enrichment (DuckDB)
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --laws <LAWS> --provisions --pg <PG_URL>  # Provisions (Postgres)
fractalaw sync publish --tenant dev --connect tcp/localhost:7447 --changed                # Changed laws only
fractalaw sync watch --tenant dev --connect tcp/localhost:7447                             # Watch for sertantai events
```

### Data Access
```bash
fractalaw law <LAW_NAME>                               # Show law metadata
fractalaw query "SELECT ..."                           # DuckDB SQL query
fractalaw stats                                        # Corpus statistics
fractalaw embed [--force]                              # Compute embeddings (LanceDB)
fractalaw validate                                     # Schema validation
```

## Module Layout

- `src/main.rs` — Clap arg definitions and dispatch
- `src/commands/taxa.rs` — All taxa subcommands (3,700 lines)
- `src/commands/sync.rs` — Zenoh publish/watch
- `src/commands/pipeline.rs` — Enrichment pipeline orchestration
- `src/commands/misc.rs` — law, query, stats, embed, validate
- `src/llm.rs` — LLM integration (Gemini batch)

## Key Paths

- Classifier weights: `docs/dictionaries/drrp_classifier_v8.json`, `docs/dictionaries/position_classifier_v3.json`
- Actor dictionary: `docs/actor-dictionary.yaml` (also compiled into fractalaw-core)
- LLM audit logs: `data/audit/`
