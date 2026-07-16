---
description: Generate compliance controls from legal obligations for a law, family, or customer register. Runs the LLM pipeline, validates output, stores in DuckDB, and publishes via zenoh.
---

# Control Creation

## When This Applies

When generating L3 Controls from L1 Legal Obligations — the canonical control set that customers use as a starting point.

## Prerequisites

- Postgres running with provisions (`systemctl --user start fractalaw-pg.service`)
- DuckDB legislation table populated with law metadata
- Gemini API key: `export GEMINI_API_KEY="$(grep GEMINI_API_KEY ~/.bashrc | cut -d'"' -f2)"`
- Prompts validated: `scripts/compliance/prompts/system-prompt-v1.md` and `policy-predicate-prompt-v1.md`

## Quick Reference

```bash
# Single law
/usr/bin/python3 scripts/compliance/generate_controls.py --law UK_uksi_1997_1713

# Single law — dry run (show prompt, no Gemini call)
/usr/bin/python3 scripts/compliance/generate_controls.py --law UK_uksi_1997_1713 --dry-run

# Family batch, QQ-scoped
/usr/bin/python3 scripts/compliance/generate_controls.py --family "💙 OH&S: Occupational / Personal Safety" --qq

# All remaining QQ laws (skips already-done)
/usr/bin/python3 scripts/compliance/generate_controls.py --all --qq

# Regenerate a law (overwrites existing)
/usr/bin/python3 scripts/compliance/generate_controls.py --law UK_uksi_1997_1713 --force

# Publish controls to sertantai
cargo run -p fractalaw-sync-cli -- publish-controls --laws UK_uksi_1997_1713 --tenant dev --connect tcp/localhost:7447
cargo run -p fractalaw-sync-cli -- publish-controls --qq --tenant dev --connect tcp/localhost:7447
```

## Workflow

### 1. Check what's already generated

```bash
/usr/bin/python3 -c "
import duckdb
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
total = conn.execute('SELECT count(DISTINCT law_name) FROM suggested_controls').fetchone()[0]
ctrls = conn.execute(\"SELECT count(*) FROM suggested_controls WHERE control_type = 'specific'\").fetchone()[0]
preds = conn.execute(\"SELECT count(*) FROM suggested_controls WHERE control_type = 'predicate'\").fetchone()[0]
flagged = conn.execute(\"SELECT count(*) FROM suggested_controls WHERE status = 'flagged'\").fetchone()[0]
print(f'Laws: {total}, Controls: {ctrls}, Predicates: {preds}, Flagged: {flagged}')
conn.close()
"
```

### 2. Generate controls

The script:
- Queries DuckDB for law metadata (title, family, fitness, explanatory_note)
- Queries Postgres for governed obligations (Obligation + governed actors + HIGH/MEDIUM significance)
- Assembles a prompt with the law outline + provisions
- Calls Gemini Pro for controls, then again for the policy predicate
- Runs Phase 2 lint (deontic verbs, paperwork referents, missing judgement, invalid refs, enum validation)
- Stores results in DuckDB `suggested_controls` table
- Saves raw JSON to `data/compliance-controls/generated/{law_name}.json`

**Skips automatically** if controls already exist for a law (use `--force` to regenerate).

**Skips automatically** if a law has no governed provisions in Postgres.

### 3. Review output quality

```bash
# Check a specific law's controls
/usr/bin/python3 -c "
import duckdb, json
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
rows = conn.execute(\"\"\"
    SELECT control_type, json_extract_string(control_json, '$.title') as title, status
    FROM suggested_controls WHERE law_name = 'UK_uksi_1997_1713' ORDER BY control_type, title
\"\"\").fetchall()
for r in rows:
    print(f'[{r[0]:9s}] {r[2]:9s} | {r[1][:80]}')
conn.close()
"
```

```bash
# Check validation flags across the corpus
/usr/bin/python3 -c "
import duckdb, json
from collections import Counter
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
rows = conn.execute(\"SELECT validation_flags FROM suggested_controls WHERE status = 'flagged'\").fetchall()
types = Counter()
for (flags_json,) in rows:
    for f in json.loads(flags_json):
        types[f.split(':')[0]] += 1
for t, c in types.most_common():
    print(f'  {t}: {c}')
conn.close()
"
```

### 4. Spot-check controls

```bash
# Show controls for a random law with their evidence hints
/usr/bin/python3 -c "
import json
data = json.loads(open('data/compliance-controls/generated/UK_uksi_1997_1713.json').read())
for c in data['controls']:
    print(f\"Title: {c['title'][:80]}\")
    print(f\"  Type: {c['control_type']} | Nature: {c['nature']} | Domain: {c['domain']}\")
    print(f\"  Info Distance: {c.get('info_distance')} | Blast Radius: {c.get('blast_radius')}\")
    print(f\"  LBJ: {c.get('load_bearing_judgement')}\")
    print(f\"  Type-A: {c['evidence_hint']['type_a'][:80]}\")
    print(f\"  Type-B: {c['evidence_hint']['type_b'][:80]}\")
    print()
"
```

### 5. Publish to sertantai

```bash
# Publish single law
cargo run -p fractalaw-sync-cli -- publish-controls --laws UK_uksi_1997_1713 --tenant dev --connect tcp/localhost:7447

# Publish all QQ laws
cargo run -p fractalaw-sync-cli -- publish-controls --qq --tenant dev --connect tcp/localhost:7447

# Publish everything in staging
cargo run -p fractalaw-sync-cli -- publish-controls --all --tenant dev --connect tcp/localhost:7447
```

## Design Constraints (encoded in the prompt)

1. **Indicative mood** — "Isolation is verified" not "Must verify isolation"
2. **Referent, not paperwork** — the control checks reality, not whether a document exists
3. **Discriminating test** — Type-B evidence that looks different if the control failed
4. **Honest limits** — flag judgement terms (adequate, competent, proportionate) that resist a tick
5. **Consolidation** — merge provisions into shared controls where the mechanism is the same
6. **Proportionality** — skip definitional/procedural provisions
7. **Control type accuracy** — Preventive / Detective / Corrective / Directive
8. **Operational properties** — estimate Info Distance, Blast Radius, Expected Touch Frequency

## Expected Ratios

| Law type | Consolidation ratio | Notes |
|----------|:---:|-------|
| Small prescriptive SI | 3-4:1 | Confined Spaces: 12 prov → 3 controls |
| Medium management SI | 4:1 | MHSW: 49 prov → 12 controls |
| Goal-setting Act | 3:1 | HSWA: 30 prov → 11 controls |
| Technical SI | 4:1 | COSHH: 51 prov → 12 controls |
| QQ corpus average | 3.9:1 | 1,341 controls from 220 laws |

## Key Files

| File | Purpose |
|------|---------|
| `scripts/compliance/generate_controls.py` | Pipeline script — prompt assembly, Gemini call, lint, storage |
| `scripts/compliance/test_generate_controls.py` | 30 tests for the pipeline |
| `scripts/compliance/prompts/system-prompt-v1.md` | Controls generation prompt (8 constraints + 3 few-shot) |
| `scripts/compliance/prompts/policy-predicate-prompt-v1.md` | Policy predicate prompt |
| `.claude/plans/compliance/COMPLIANCE-CONTROLS.md` | Design doc v0.2 (plans = ephemeral scaffold) |
| `data/compliance-controls/generated/` | Raw JSON outputs per law |
| DuckDB `suggested_controls` table | Staging table for all generated controls |

## Troubleshooting

- **No governed provisions**: the law has no Obligation provisions with governed (non-government) actors at HIGH/MEDIUM significance. Check Postgres: `SELECT count(*) FROM legislation_text WHERE law_name = '...' AND 'Obligation' = ANY(drrp_types)`
- **Deontic verb in title**: the LLM drifted back to imperative mood. Rate is ~0.15% — flag for manual edit or regenerate with `--force`
- **INVALID_REF**: the LLM referenced a provision excluded by the filter (offence, exemption, low significance). Not a quality issue.
- **JUDGEMENT_MISSING**: a judgement term (adequate, competent etc.) is in the control text but `load_bearing_judgement` is null. Soft flag — the term is there, just in the wrong field.
- **Empty output**: check if the law has provisions in Postgres and if they pass the filter.
