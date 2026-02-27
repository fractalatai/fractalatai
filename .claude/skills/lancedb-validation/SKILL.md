# Skill: LanceDB Validation Queries

## When This Applies

When you need to query, validate, or analyse the `legislation_text` table in LanceDB — confidence distributions, purpose gate audits, coverage checks, enrichment verification. This is the at-scale validation workflow for taxa enrichment.

## The Two Data Stores

Legislative data lives in **two separate stores** with different query paths:

| Store | Table | Query path | Contains |
|-------|-------|------------|----------|
| **DuckDB** | `legislation` | `fractalaw query "<SQL>"` (via DataFusion) | Law metadata: name, family, is_making, body_paras, duty_holder, duty_type (lists) |
| **LanceDB** | `legislation_text` | Python (`lancedb` + `pyarrow`) or Rust (`LanceStore`) | Per-provision data: text, taxa_confidence, drrp_types, purposes, clause_refined, governed_actors |

**The CLI `query` command goes through DataFusion and does NOT have access to `legislation_text`.** You cannot query LanceDB provision data via the CLI query command. This is the single most common mistake.

### What lives where

- **Law-level DRRP summary** (duty_holder, duty_type, role) → DuckDB `legislation`
- **Per-provision DRRP detail** (taxa_confidence, clause_refined, drrp_types, purposes) → LanceDB `legislation_text`
- **Full legislative text** (the raw provision content) → LanceDB `legislation_text`

## Querying DuckDB (Law-Level)

Via the CLI. Note: DRRP columns are **arrays**, not strings. Use array functions.

```bash
# Works — array functions
fractalaw query "SELECT name FROM legislation
  WHERE family = 'OH&S: Occupational / Personal Safety'
  AND duty_holder IS NOT NULL AND array_length(duty_holder) > 0"

# Fails — LIKE doesn't work on List columns
fractalaw query "SELECT name FROM legislation WHERE duty_type LIKE '%Duty%'"

# Use array_has instead
fractalaw query "SELECT name FROM legislation WHERE array_has(duty_type, 'Duty')"

# Use array_to_string to flatten for display
fractalaw query "SELECT name, array_to_string(duty_type, ', ') AS dt FROM legislation"
```

Other gotchas:
- Column is `is_making` not `making`
- Use `length()` not `len()` (DataFusion, not DuckDB native)
- `count(*)` returns UInt64 — cast with `::BIGINT` for clean display

## Querying LanceDB (Provision-Level)

No pandas on this system. Use `lancedb` + `pyarrow` + `pyarrow.compute`:

```python
import lancedb
import pyarrow as pa
import pyarrow.compute as pc

db = lancedb.connect("./data/lancedb")
tbl = db.open_table("legislation_text")

# Basic query — filter + select + to_arrow()
results = tbl.search() \
    .where("taxa_confidence > 0") \
    .select(["law_name", "taxa_confidence", "purposes", "clause_refined", "drrp_types", "provision"]) \
    .limit(100000) \
    .to_arrow()

# IMPORTANT: use .to_arrow() not .to_pandas() — no pandas installed
```

### Key LanceDB columns in `legislation_text`

| Column | Type | Notes |
|--------|------|-------|
| `law_name` | Utf8 | e.g. `UK_uksi_2005_1643` |
| `provision` | Utf8 (nullable) | e.g. `7`, `?` for schedule/preamble |
| `section_id` | Utf8 | Unique key for merge_insert |
| `text` | Utf8 | Raw provision text |
| `taxa_confidence` | Float32 (nullable) | 0.0–0.85, null if no DRRP |
| `drrp_types` | List\<Utf8\> (nullable) | e.g. `["Duty"]`, `["Right", "Power"]` |
| `purposes` | List\<Utf8\> (nullable) | e.g. `["Application+Scope", "Process+Rule+Constraint+Condition"]` |
| `clause_refined` | Utf8 (nullable) | Extracted clause text |
| `governed_actors` | List\<Utf8\> (nullable) | e.g. `["Org: Employer"]` |
| `government_actors` | List\<Utf8\> (nullable) | e.g. `["Gvt: Minister"]` |
| `duty_family` | Utf8 (nullable) | e.g. `Governed` |
| `popimar` | List\<Utf8\> (nullable) | POPIMAR categories |
| `taxa_classified_at` | Timestamp (nullable) | When taxa pipeline ran |

### Cross-referencing DuckDB and LanceDB

A common pattern: get law names from DuckDB, then filter LanceDB provisions to those laws.

```python
import subprocess

# Step 1: Get law names from DuckDB via CLI
result = subprocess.run(
    ["cargo", "run", "-p", "fractalaw-cli", "--", "query",
     "SELECT name FROM legislation WHERE family = 'OH&S: Occupational / Personal Safety' "
     "AND duty_holder IS NOT NULL AND array_length(duty_holder) > 0"],
    capture_output=True, text=True, cwd="/var/home/jason/fractalaw"
)
names = set()
for line in result.stdout.split('\n'):
    line = line.strip()
    if line.startswith('| UK_'):
        names.add(line.split('|')[1].strip())

# Step 2: Filter LanceDB to those laws
all_provs = tbl.search().where("taxa_confidence > 0") \
    .select([...]).limit(100000).to_arrow()
mask = pc.is_in(all_provs.column("law_name"), value_set=pa.array(list(names)))
filtered = all_provs.filter(mask)
```

### Working with pyarrow.compute (no pandas)

```python
import pyarrow.compute as pc

col = table.column("taxa_confidence")

# Aggregates
pc.mean(col).as_py()
pc.min(col).as_py()
pc.max(col).as_py()
pc.sum(mask).as_py()  # count True values in a boolean array

# Filtering
mask = pc.and_(pc.greater_equal(col, 0.4), pc.less(col, 0.6))
filtered = table.filter(mask)

# Value counts (manual — no pandas)
rounded = pc.round(col, 2)
for val in pc.unique(rounded).sort():
    v = val.as_py()
    count = pc.sum(pc.equal(rounded, v)).as_py()

# Membership test
pc.is_in(col, value_set=pa.array(["value1", "value2"]))

# Accessing list columns (purposes, drrp_types)
for i in range(len(table)):
    purposes = table.column("purposes")[i].as_py()  # returns Python list or None
    if purposes and "Application+Scope" in purposes:
        ...

# Getting sparse row data
indices = pc.indices_nonzero(some_boolean_mask)
for idx_scalar in indices[:20]:
    idx = idx_scalar.as_py()
    val = table.column("col_name")[idx].as_py()
```

## Validation Workflows

### 1. Confidence distribution

Query LanceDB for `taxa_confidence > 0`, bucket into bands, surface the low tail.

The confidence score is quantized to 6 values due to additive signal weights:

| Score | Signals present |
|------:|-----------------|
| 0.85 | span + clean end + length + modal |
| 0.70 | span + length + modal (no clean end) |
| 0.60 | span + clean end + length (no modal) **or** clean end + length + modal (no span) |
| 0.45 | clean end + length (no span, no modal) |
| 0.35 | length + modal (no span, no clean end) |
| 0.20 | length only |

Low confidence (< 0.40) is almost always fee/charge provisions or short headings.

### 2. Purpose gate audit (Application+Scope)

Goal: verify the Application+Scope gate isn't blocking genuine duties.

```
For each OHS provision in LanceDB:
  if purposes contains "Application+Scope":
    if drrp_types is non-empty → "passed" (false negative candidate)
    else → "gated" (false positive candidate)
      → has modal verb AND governed actor? → higher risk
        → text starts with actor? → "actor-led" (HIGHEST false-positive risk)
        → text starts with "These Regulations"? → "scope-led" (likely correct)
```

The actor-led bucket is where real bugs hide. The pattern `"[Actor] to whom/which this regulation applies [shall/must] [action]"` is the known false-positive shape — relative clauses qualifying the actor.

### 3. Missing enrichment check

Cross-reference DuckDB Making laws with `body_paras > 0` against LanceDB:
- Not in LanceDB at all → sertantai hasn't parsed the text yet
- In LanceDB but no taxa data → check provision content (may be legitimately non-DRRP)

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-store/src/lance.rs` | `LanceStore` — `query_legislation_text()`, `update_taxa()` |
| `crates/fractalaw-cli/src/main.rs` | `cmd_taxa_enrich`, `cmd_taxa_eyeball`, `cmd_taxa_show` |
| `crates/fractalaw-core/src/taxa/confidence.rs` | Scoring weights (0.25/0.25/0.20/0.15) |
| `crates/fractalaw-core/src/taxa/purpose.rs` | Purpose classifier, Application+Scope regex |
| `crates/fractalaw-core/src/taxa/mod.rs` | `parse_v2()` pipeline, `should_skip_drrp()` gate |
