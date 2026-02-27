# Skill: Taxa DRRP Verification & Validation

## When This Applies

After running `taxa enrich` on a batch of laws and you need to verify the results at scale — checking quality, measuring coverage, auditing purpose gates, finding regressions. This is the "did it work?" workflow, not the "how do I fix patterns?" workflow (see `taxa-gap-analysis` skill for that).

## The Verification Stack

Three complementary approaches, each at a different granularity:

| Level | Tool | Scope | Speed |
|-------|------|-------|-------|
| **Law-level** | DuckDB via `fractalaw query` | Coverage counts, holder distributions | Fast |
| **Provision-level** | LanceDB via Python (pyarrow) | Confidence, purpose gates, clause quality | Medium |
| **Eyeball** | `taxa eyeball` / `taxa show` CLI | Individual clause review | Slow, manual |

### Critical: DuckDB vs LanceDB

The CLI `query` command goes through **DataFusion** which only has the `legislation` table — law-level metadata. It does NOT have `legislation_text`. Per-provision data (confidence, purposes, clauses, drrp_types) is in **LanceDB only**. See `lancedb-validation` skill for query patterns.

## 1. Coverage Check (DuckDB)

First question after enrichment: how many laws got DRRP data?

```bash
# Summary counts for a family
fractalaw query "
SELECT
  count(*)::BIGINT AS total,
  count(CASE WHEN is_making = true THEN 1 END)::BIGINT AS making,
  count(CASE WHEN is_making = true AND body_paras > 0 THEN 1 END)::BIGINT AS making_with_text,
  count(CASE WHEN duty_holder IS NOT NULL AND array_length(duty_holder) > 0 THEN 1 END)::BIGINT AS with_drrp
FROM legislation
WHERE family = 'OH&S: Occupational / Personal Safety'"
```

DuckDB gotchas:
- DRRP columns (`duty_holder`, `duty_type`, `role`) are **arrays** — use `array_length()`, `array_has()`, `array_to_string()`, not `LIKE` or `length()`
- Column is `is_making` not `making`
- Cast counts with `::BIGINT` for clean display

### Making laws without enrichment

```bash
# Which Making laws have text but no DRRP? (gap analysis)
fractalaw query "
SELECT name, title, body_paras
FROM legislation
WHERE family = '...'
  AND is_making = true AND body_paras > 0
  AND (duty_holder IS NULL OR array_length(duty_holder) = 0)
ORDER BY name"
```

Cross-reference against LanceDB to distinguish:
- **Not in LanceDB** — sertantai hasn't parsed the text yet (infrastructure gap)
- **In LanceDB but no DRRP** — check provision content; may be legitimately non-DRRP (amendment SIs, offence acts, fee modification instruments)

### Laws with only Rights/Powers but no Duty or Responsibility

```bash
fractalaw query "
SELECT name, array_to_string(duty_type, ', ') AS dt
FROM legislation
WHERE family = '...'
  AND duty_holder IS NOT NULL AND array_length(duty_holder) > 0
  AND NOT array_has(duty_type, 'Duty') AND NOT array_has(duty_type, 'Responsibility')
ORDER BY name"
```

## 2. Confidence Distribution (LanceDB)

Second question: how good are the extracted clauses?

```python
import lancedb, pyarrow as pa, pyarrow.compute as pc

db = lancedb.connect("./data/lancedb")
tbl = db.open_table("legislation_text")

# Get OHS law names from DuckDB (see lancedb-validation skill for cross-ref pattern)
ohs_names = get_law_names_from_duckdb(family="OH&S: ...")

# All enriched provisions
results = tbl.search().where("taxa_confidence > 0") \
    .select(["law_name", "taxa_confidence", "purposes", "clause_refined", "drrp_types", "provision"]) \
    .limit(100000).to_arrow()

# Filter to family
mask = pc.is_in(results.column("law_name"), value_set=pa.array(list(ohs_names)))
family_data = results.filter(mask)
conf = family_data.column("taxa_confidence")

# Bucket distribution
bins = [(0, 0.2, "very low"), (0.2, 0.4, "low"), (0.4, 0.6, "medium"), (0.6, 0.8, "good"), (0.8, 0.86, "high")]
for lo, hi, label in bins:
    count = pc.sum(pc.and_(pc.greater_equal(conf, lo), pc.less(conf, hi))).as_py()
    print(f"  {label}: {count}")
```

### What the confidence values mean

Scores are quantized (additive signal weights in `confidence.rs`):

| Score | Signals | Typical content |
|------:|---------|-----------------|
| 0.85 | span + clean end + length + modal | Best quality — "The employer shall ensure..." ending with `.` |
| 0.60 | span + end + length (no modal) **or** end + length + modal (no span) | Good — may use "may" instead of "shall", or government pattern (no span) |
| 0.45 | end + length (no span, no modal) | Medium — clause captured but no strong obligation signal |
| 0.20 | length only | Low — typically fee/charge provisions, short headings |

### Investigating low-confidence provisions

```python
low_mask = pc.less(conf, 0.40)
low_indices = pc.indices_nonzero(low_mask)
for idx_scalar in low_indices[:20]:
    idx = idx_scalar.as_py()
    law = family_data.column("law_name")[idx].as_py()
    prov = family_data.column("provision")[idx].as_py() or "?"
    c = family_data.column("taxa_confidence")[idx].as_py()
    clause = family_data.column("clause_refined")[idx].as_py() or ""
    print(f"  conf={c:.2f} | {law} {prov} | {clause[:120]}")
```

Low confidence is almost always fee/charge provisions — correctly classified but weak regex signal. If you see genuine duty text at low confidence, investigate the confidence scorer signals.

## 3. Purpose Gate Audit

Third question: is the Application+Scope (or other) gate blocking genuine duties?

This is the most important validation step. The purpose-based gates in `should_skip_drrp()` can silently suppress entire provisions. You need to check both directions:

- **False positives** (gate wrongly blocked a genuine duty)
- **False negatives** (gate let through a scope/interpretation provision)

### Methodology

```python
# Get ALL provisions for the family (not just enriched)
all_provs = tbl.search().where("law_name IS NOT NULL") \
    .select(["law_name", "provision", "text", "taxa_confidence", "drrp_types", "purposes"]) \
    .limit(200000).to_arrow()

# Filter to family
family_all = all_provs.filter(pc.is_in(all_provs.column("law_name"), value_set=pa.array(list(ohs_names))))

import re
MODAL_RE = re.compile(r'\b(?:shall|must|is required to|has a duty)\b', re.I)
ACTOR_RE = re.compile(r'\b(?:employer|employee|person|worker|contractor|occupier|operator|owner)\b', re.I)

gated = []    # has purpose, no DRRP
passed = []   # has purpose AND has DRRP

for i in range(len(family_all)):
    purposes = family_all.column("purposes")[i].as_py() or []
    drrp = family_all.column("drrp_types")[i].as_py() or []

    if "Application+Scope" not in purposes:
        continue

    text = family_all.column("text")[i].as_py() or ""
    has_modal = bool(MODAL_RE.search(text))
    has_actor = bool(ACTOR_RE.search(text))

    row = { "text": text, "has_modal": has_modal, "has_actor": has_actor, ... }

    if len(drrp) > 0:
        passed.append(row)   # false negative candidate
    else:
        gated.append(row)    # false positive candidate if has_modal and has_actor
```

### Triage heuristic for false positives

Among gated provisions with modal+actor, further classify:

- **Actor-led** (text starts with actor pattern) — **highest false-positive risk**. The provision is likely a genuine duty with a scope qualifier embedded as a relative clause.
- **Scope-led** (text starts with "These Regulations...") — likely a genuine scope provision that mentions actors incidentally.

```python
text_stripped = text.strip()
starts_with_actor = bool(re.match(
    r'(?i)^(?:\d+\s+)?(?:every |the |each |a |an )?'
    r'(?:employer|employee|person|worker|contractor|occupier|operator|owner)',
    text_stripped
))
```

Actor-led gated provisions with modal verbs are almost always bugs. Inspect each one manually.

### Known false-positive pattern

The pattern `"[Actor] to whom/which this regulation applies [shall/must] [action]"` was identified as a systematic false positive in the Application+Scope gate (15 provisions in OHS family, 2.6% error rate). Fixed by requiring `these/this` at sentence-start position in the regex.

## 4. Miss Rate Measurement

For deeper quality assessment, measure what the pipeline *doesn't* catch.

### Heat-scored miss analysis

Use `taxa::analyse_miss()` to score unclassified provisions:

```
Heat scoring:
  +3 obligation modal (shall/must/is required to)
  +2 governed actor extracted
  +1 enabling modal (may/power to)
  +1 government actor extracted
  +1 operative purpose (Process+Rule)
  −2 structural purpose only (Interpretation/Amendment/Repeal)
  −1 short text (< 50 chars)
```

Provisions with heat >= 3 are genuine missed duties worth investigating.

### CLI miss analysis

```bash
cargo run -p fractalaw-cli -- taxa show --misses <LAW_NAME>
```

Shows unclassified provisions ranked by heat score with diagnostic metadata (actors found, modal presence, purposes).

## 5. Before/After Comparison

When changing patterns or gates, measure the delta:

```bash
# Before: capture baseline
fractalaw taxa show --clauses <LAW> > /tmp/before.txt

# After: re-enrich and compare
fractalaw taxa enrich --laws <LAW> --force
fractalaw taxa show --clauses <LAW> > /tmp/after.txt

diff /tmp/before.txt /tmp/after.txt
```

Key metrics to track:
- Total DRRP provisions (should go up or stay same)
- New true positives (provisions that gained DRRP)
- Lost provisions (provisions that lost DRRP — regressions)
- Confidence distribution shift

### Side-by-side v1 vs v2

```bash
cargo run -p fractalaw-cli -- taxa show --compare <LAW_NAME>
```

Shows per-provision v1 and v2 classifications side-by-side. Useful when testing pattern architecture changes.

## 6. Eyeball Review

For final human validation on a sample:

```bash
# Generate markdown QA artifact
fractalaw taxa eyeball \
  --laws "UK_uksi_2005_1643,UK_uksi_1992_2792,UK_ukpga_1974_37" \
  --output ./data/clause_eyeball.md
```

Output format per provision:
```markdown
### Reg 7 — Duty (conf: 0.85)
> The employer shall ensure the health and safety of employees.
```

Provisions ending without `.` or `;` get a `**[BAD END]**` marker — these have truncated clause extraction.

### Choosing a representative sample

Pick laws that cover:
- Different instrument types (Acts vs SIs)
- Different actor patterns (employer-heavy vs multi-actor)
- Different sizes (5 provisions vs 50+)
- Both pre-existing enrichment and newly enriched

## Test-Driven Verification

Every pattern or gate change should follow the cycle:

1. **Baseline**: measure current counts before the change
2. **True-negative tests**: add tests for provisions that correctly get NO DRRP today
3. **True-positive tests**: add failing tests using real provision text from the bug
4. **Implement**: minimal regex or gate change
5. **Full suite**: `cargo test -p fractalaw-core` — zero regressions
6. **Re-enrich**: `fractalaw taxa enrich --family "..." --force`
7. **Re-measure**: run the same validation queries to confirm improvement

## Key Files

| File | Role |
|------|------|
| `crates/fractalaw-core/src/taxa/confidence.rs` | Clause confidence scorer (signal weights) |
| `crates/fractalaw-core/src/taxa/purpose.rs` | Purpose classifier + Application+Scope regex |
| `crates/fractalaw-core/src/taxa/mod.rs` | `parse_v2()`, `should_skip_drrp()`, `analyse_miss()` |
| `crates/fractalaw-cli/src/main.rs` | `cmd_taxa_show`, `cmd_taxa_eyeball`, `cmd_taxa_enrich` |
| `crates/fractalaw-store/src/lance.rs` | `query_legislation_text()`, `update_taxa()` |
| `.claude/skills/lancedb-validation/SKILL.md` | LanceDB query patterns and pyarrow recipes |
| `.claude/skills/taxa-gap-analysis/SKILL.md` | Pattern improvement iteration cycle |
