---
description: Run the fitness applicability pipeline end-to-end. Extracts who/what/where a law applies to, compiles expression trees, publishes to sertantai.
---

# Fitness Pipeline

## When This Applies

When processing fitness (applicability) data for laws — extracting who/what/where a law applies to, and publishing compiled expression trees to sertantai for customer matching.

The fitness pipeline is **independent of DRRP taxa**. It has its own CLI commands, its own Postgres table (`fitness_mentions`), and its own DuckDB columns (`fitness_entities`, `compiled_applicability`).

## Pipeline Steps

### Step 1: Extract (regex + polarity detection)

```bash
# All laws (skips provisions that already have mentions)
cargo run -p fractalaw-cli -- fitness extract

# Specific laws
cargo run -p fractalaw-cli -- fitness extract --laws UK_ukpga_1981_69,UK_ukpga_2009_23

# Re-extract (clears regex_entities only, preserves SLM/ft data)
cargo run -p fractalaw-cli -- fitness extract --force
```

Writes to `fitness_mentions` table in Postgres. Runs polarity detection on ALL provisions (not gated by APPLICATION_SCOPE), then dictionary extraction with family-scoped specialists.

Also extracts commencement dates as temporal entities.

### Step 2: SLM extraction (RunPod)

For provisions where dictionaries found polarity but no entities. See `/runpod-batch-inference` skill.

```bash
# On RunPod (after tunnel + Ollama setup):
python3 -u /workspace/scripts/runpod_fitness_batch.py --workers 4
```

Writes to `ft_entities` column (fine-tuned model) or `slm_entities` column (base model). Each tier has its own column — never overwrites other tiers.

### Step 3: Reconcile tiers

**Always run after any extraction batch.** Fills `entities`/`scope_dimensions` for mentions that have tier output but were never reconciled (ft > regex > slm; dimensions unioned). Skipping this left 1,714 mentions (136 laws) with empty `entities` after the July batches, which silently degraded trees.

```bash
cargo run -p fractalaw-cli -- fitness reconcile --dry-run
cargo run -p fractalaw-cli -- fitness reconcile
```

### Step 4: Derive application

Application is the set of nations where the law operates. It is **not extent**: an "(England)" SI extends E+W but applies in England. The order is text clause > title parenthetical > type code > extent (extent clause, then LAT, then LRT). Writes DuckDB `application_regions/source/evidence`. See sertantai-legal #162/#163.

```bash
cargo run -p fractalaw-cli -- fitness application --out /tmp/app.jsonl   # review
cargo run -p fractalaw-cli -- fitness application                        # write DuckDB
```

### Step 5: Compile expression trees

```bash
# Dry run: JSONL, DuckDB untouched, then lint
cargo run -p fractalaw-cli -- fitness compile --out /tmp/trees.jsonl
/usr/bin/python3 scripts/maintenance/lint_trees.py --scope <laws.txt> --jsonl /tmp/trees.jsonl

# Write DuckDB (back up compiled_applicability first)
cargo run -p fractalaw-cli -- fitness compile [--laws UK_ukpga_1981_69]
```

Compiler (`fractalaw-core/src/taxa/applicability_compile.rs`):
- AppliesTo mentions → Or; different dimensions within a mention → And
- Root And gets one `territorial` gate holding the law's application. Jurisdiction codes (england, scotland, united_kingdom ...) never appear elsewhere (L3/L7)
- Every code must be grounded in its provision text + sub-provisions, or the law's title, after stripping citations of other Acts (L4; the ft model emits `construction` for unrelated provisions)
- Government actors dropped (L5); `construction` in the interpretation or vehicle/product sense dropped
- Only law-level disapplications ("These Regulations do not apply to …") become the root Not; provision-level exceptions are dropped
- TimeWindow: `from` = earliest law-level commencement; `to` only from a law-level sunset (L2)
- Output normalised: duplicates removed, Or-Matches merged (L1)
- Laws with a tree but no mentions left are repaired in place (`repair_tree`)

`taxa_hash` does **not** cover fitness or tree fields, so `publish --changed` never picks up tree changes. Publish with explicit `--laws`.

### Step 6: Publish to sertantai

```bash
# Specific laws
cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/localhost:7447 --laws <LAWS>

# All laws with compiled trees
# (generate law list from DuckDB first)
python3 -c "
import duckdb
db = duckdb.connect('data/fractalaw.duckdb', read_only=True)
rows = db.sql(\"SELECT name FROM legislation WHERE compiled_applicability IS NOT NULL\").fetchall()
print(','.join(r[0] for r in rows))
db.close()
" | xargs -I{} cargo run -p fractalaw-sync-cli -- publish --tenant dev --connect tcp/localhost:7447 --laws {}
```

Publishes `fitness_entities`, `fitness_scope_dimensions`, `fitness_mention_count`, `fitness_applies_count`, `fitness_disapplies_count`, and `compiled_applicability` as part of the LRT Arrow IPC payload.

### Step 7: Check coverage

```bash
cargo run -p fractalaw-cli -- fitness status
cargo run -p fractalaw-cli -- fitness status --laws UK_ukpga_1981_69,UK_ukpga_2009_23
```

## Data Model

### Postgres: `fitness_mentions` (per-provision, one-to-many)

| Column | Description |
|--------|-------------|
| `section_id` | FK to legislation_text |
| `polarity` | AppliesTo / DisappliesTo / ExtendsTo |
| `scope_unit` | law / Part N / provision |
| `regex_entities` | Entities from dictionary extraction |
| `slm_entities` | Entities from base SLM |
| `ft_entities` | Entities from fine-tuned SLM |
| `entities` | Reconciled final (COALESCE ft > regex > slm) |
| `scope_dimensions` | personal / material / territorial / temporal / conditional |

### Postgres: `fitness_entities` (entity catalogue)

| Column | Description |
|--------|-------------|
| `uri` | Canonical identifier |
| `display_name` | Human-readable name |
| `scope_dimensions` | Scope dimension labels |
| `source` | pdim_person / corpus_mining / etc |
| `family_scope` | NULL = core, else family-gated |

### DuckDB: `legislation` (law-level)

| Column | Description |
|--------|-------------|
| `fitness_entities` | Aggregated entity list |
| `fitness_scope_dimensions` | Scope dimensions present |
| `fitness_mention_count` | Total mentions |
| `fitness_applies_count` | AppliesTo count |
| `fitness_disapplies_count` | DisappliesTo count |
| `compiled_applicability` | JSON expression tree |

## Per-Tier Column Rules

**CRITICAL**: Each extraction tier writes to its OWN column. Never overwrite another tier.

- `regex_entities` — dictionary extraction only
- `slm_entities` — base SLM (prompted)
- `ft_entities` — fine-tuned SLM
- `entities` — reconciled final (COALESCE, not overwritten by any tier)

`--force` clears only the requesting tier's column via UPDATE SET NULL. Never DELETE FROM.

## Key Files

| File | Purpose |
|------|---------|
| `crates/fractalaw-core/src/taxa/fitness.rs` | Polarity detection + P-dimension dictionaries |
| `crates/fractalaw-core/src/taxa/applicability.rs` | ApplicabilityNode enum + serde JSON |
| `crates/fractalaw-cli/src/commands/fitness.rs` | extract, compile, status CLI commands |
| `scripts/ml/runpod_fitness_batch.py` | SLM batch extraction script |
| `scripts/ml/finetune_fitness_16bit.py` | Fine-tuning script |
| `models/gemma3-fitness-q4.gguf` | Fine-tuned GGUF (local) |

## Related Skills

- `/runpod-batch-inference` — SLM batch extraction on RunPod
- `/runpod-finetune` — fine-tuning the fitness SLM
- `/lrt-sync` — ensure DuckDB LRT records exist before fitness
- `/publish` — publish enriched data to sertantai

## Strategy + Design Docs

- `.claude/plans/fitness/FITNESS-STRATEGY.md` — full strategy (COMPLETE)
- `.claude/plans/fitness/FITNESS-GRAPH.md` — propagation design
- `.claude/plans/fitness/FITNESS-RULES-ENGINE.md` — rules engine design
