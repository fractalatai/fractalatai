---
description: Generate L4 Evidence patterns from L3 Controls for a law, family, or customer register. Runs the LLM pipeline, validates output, stores in DuckDB, and optionally publishes via zenoh.
---

# Evidence Creation

## When This Applies

When generating L4 Evidence patterns from L3 Controls — telling the customer what artefacts to register, whether judgement is needed, and where to invest evidence effort (VoI 2x2).

**Prerequisite**: Controls must already exist in `suggested_controls` for the target laws. Run the `/control-creation` skill first if they don't.

## Prerequisites

- DuckDB with `suggested_controls` table populated (controls already generated)
- Gemini API key: `export GEMINI_API_KEY="$(grep GEMINI_API_KEY ~/.bashrc | cut -d'"' -f2)"`
- Prompt: `scripts/compliance/prompts/evidence-system-prompt-v1.md`

No Postgres needed — evidence generation reads from DuckDB only.

## Quick Reference

```bash
# Single law
/usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713

# Single law — dry run (show prompt, no Gemini call)
/usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713 --dry-run

# All remaining QQ laws on Flash (recommended for batch)
/usr/bin/python3 scripts/compliance/generate_evidence.py --all --qq --model gemini-2.5-flash --thinking 4096

# Batch with limit (e.g. 50 at a time)
/usr/bin/python3 scripts/compliance/generate_evidence.py --all --qq --limit 50 --model gemini-2.5-flash --thinking 4096

# Family batch
/usr/bin/python3 scripts/compliance/generate_evidence.py --family "OH&S: Occupational / Personal Safety" --model gemini-2.5-flash --thinking 4096

# Force regenerate a law
/usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713 --force

# Model comparison (Flash vs Pro) — writes JSON only, no DuckDB
/usr/bin/python3 scripts/compliance/generate_evidence.py --law UK_uksi_1997_1713 --no-db --model gemini-2.5-flash --thinking 4096
```

## Model Selection

| Model | Thinking | Use case | Cost |
|-------|----------|----------|------|
| `gemini-2.5-pro` | 8192 | Pilot / quality benchmark | ~£1/law |
| `gemini-2.5-flash` | 4096 | **Production corpus runs** | ~£0.02/law |

**Always use Flash for batch runs.** Pro pilot cost ~£20 for 21 laws. Flash produces comparable quality at ~10-20x lower cost. Pro benchmark JSON files are saved as `{law_name}.pro.json` for comparison.

**max_tokens is 32,768.** Evidence output is ~5KB per control. Laws with 12+ controls will truncate at 16,384 (manifests as JSON parse errors, not an explicit signal).

## Workflow

### 1. Check what's already generated

```bash
/usr/bin/python3 -c "
import duckdb
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
total = conn.execute('SELECT count(*) FROM suggested_evidence').fetchone()[0]
laws = conn.execute('SELECT count(DISTINCT law_name) FROM suggested_evidence').fetchone()[0]
validated = conn.execute(\"SELECT count(*) FROM suggested_evidence WHERE status = 'validated'\").fetchone()[0]
flagged = conn.execute(\"SELECT count(*) FROM suggested_evidence WHERE status = 'flagged'\").fetchone()[0]
print(f'Laws: {laws}, Patterns: {total}, Validated: {validated} ({validated/total*100:.1f}%), Flagged: {flagged} ({flagged/total*100:.1f}%)')
conn.close()
"
```

### 2. Check which laws need evidence

```bash
/usr/bin/python3 -c "
import duckdb
from pathlib import Path
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
with_controls = set(r[0] for r in conn.execute(\"SELECT DISTINCT law_name FROM suggested_controls WHERE control_type = 'specific'\").fetchall())
with_evidence = set(r[0] for r in conn.execute('SELECT DISTINCT law_name FROM suggested_evidence').fetchall())
remaining = with_controls - with_evidence
print(f'Laws with controls: {len(with_controls)}')
print(f'Laws with evidence: {len(with_evidence)}')
print(f'Remaining: {len(remaining)}')
conn.close()
"
```

### 3. Generate evidence patterns

The script:
- Reads controls from DuckDB `suggested_controls` table (specific controls only, not predicates)
- Reads law metadata from DuckDB `legislation` table (title, family)
- Assembles prompt: law context + numbered controls with all properties
- Calls Gemini with structured JSON output
- Saves raw JSON to `data/compliance-evidence/generated/{law_name}.json` (written BEFORE DuckDB — crash-safe)
- Runs Phase 2 lint (13 checks)
- Stores in DuckDB `suggested_evidence` table (unless `--no-db`)

**Skips automatically** if evidence already exists for a law (use `--force` to regenerate).

**Rate limiting**: 10s delay between calls + retry with backoff on 429 (30s, 60s, 120s, 240s). For batch runs of 50+ laws, this prevents rate limit exhaustion.

### 4. Review output quality

```bash
# Check a specific law's evidence patterns
/usr/bin/python3 -c "
import duckdb, json
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
rows = conn.execute(\"\"\"
    SELECT control_title, status, evidence_json
    FROM suggested_evidence WHERE law_name = 'UK_uksi_1997_1713' ORDER BY control_title
\"\"\").fetchall()
for title, status, ej in rows:
    ev = json.loads(ej)
    arts = ev.get('artefacts', [])
    j = ev.get('judgement', {})
    s = ev.get('strategy', {})
    type_b = sum(1 for a in arts if a.get('artefact_class') == 'Outcome')
    print(f'[{status:9s}] {title[:70]}')
    print(f'  Artefacts: {len(arts)} ({type_b} Type-B) | Judgement: {j.get(\"needs_judgement\")} | VoI: {s.get(\"voi_quadrant\")}')
conn.close()
"
```

```bash
# Check validation flags across the corpus
/usr/bin/python3 -c "
import duckdb, json
from collections import Counter
conn = duckdb.connect('data/fractalaw.duckdb', read_only=True)
rows = conn.execute(\"SELECT validation_flags FROM suggested_evidence WHERE status = 'flagged'\").fetchall()
types = Counter()
for (flags_json,) in rows:
    for f in json.loads(flags_json):
        types[f.split(':')[0]] += 1
for t, c in types.most_common():
    print(f'  {t}: {c}')
conn.close()
"
```

### 5. Post-processing (before publish)

Flash produces enum drift that needs normalising before publish:

- **artefact_type**: `Inspection Report` → `Report`, `Observation` → `Report`, novel types → `Other`
- **recommended_method**: comma-separated lists → take first value, `Physical Inspection` → `Visual Inspection`
- **artefact_class**: string `"None"` → `"Activity"`
- **source**: `Observation`/`Interview` → `Upload`, `LMS` → `Linked System`
- **Deterministic overrides**: `needs_judgement` forced true when `load_bearing_judgement` is non-null, `evidence_standard` corrected from `blast_radius`

See session doc `07-15-26-evidence-records.md` for the full post-processing script pattern.

### 6. Publish to sertantai

```bash
# Publish single law
cargo run -p fractalaw-sync-cli -- publish-evidence --laws UK_uksi_1997_1713 --tenant dev --connect tcp/localhost:7447

# Publish all QQ laws
cargo run -p fractalaw-sync-cli -- publish-evidence --qq --tenant dev --connect tcp/localhost:7447

# Publish everything in staging
cargo run -p fractalaw-sync-cli -- publish-evidence --all --tenant dev --connect tcp/localhost:7447
```

Zenoh spec for sertantai: `sertantai-legal/docs/zenoh/ZENOH-EVIDENCE-SPEC.md`

## What the LLM Produces (per control)

Three sections in each evidence pattern:

### Artefact Patterns
1-3 artefacts per control. At least one Type-B (Outcome) required.

| Field | Description |
|-------|-------------|
| `title` | Domain-specific artefact description |
| `artefact_type` | Report, Risk Assessment, Certificate, Test Result, etc. |
| `artefact_class` | Activity (Type-A) or Outcome (Type-B) |
| `what_it_proves` | What belief this changes — the discriminating test |
| `source` | Upload, System Generated, Sensor, External, Linked System |
| `likelihood_ratio` | Low / Medium / High |
| `evidence_by_design` | True if byproduct of control execution |

### Judgement Guidance
| Field | Description |
|-------|-------------|
| `needs_judgement` | Deterministic: true if load_bearing_judgement present, or Manual+Remote, or Enterprise, or Directive |
| `judgement_rationale` | **Always populated** — why judgement is or isn't needed |
| `recommended_method` | Visual Inspection, Functional Test, Document Review, etc. |
| `basis_guidance` | What the person should look at |
| `discriminating_question` | The question the judge answers |
| `drift_signal` | How the measurement method can decouple from reality |
| `drift_conditions` | **Always populated** — when the control itself has drifted |

### Evidence Strategy
| Field | Description |
|-------|-------------|
| `voi_quadrant` | Table Stakes / No-Brainer / Judgement / Waste |
| `evidence_standard` | Basic / Focused / Comprehensive (from blast_radius) |
| `staleness_tolerance` | Low / Medium / High |
| `nature_strategy` | Automated → benchmark + ITGC; Manual → sample-based |

## Design Constraints (encoded in the prompt)

1. **Evidence as credence change** — each artefact states what belief it changes
2. **Type-B priority** — at least one Outcome artefact per control
3. **Judgement where judgement is needed** — load-bearing terms trigger judgement; always explain why
4. **Evidence-by-design** — prefer byproducts of control execution
5. **VoI drives effort** — classify on the Expected Loss vs Measurement Cost 2x2
6. **Domain-specific artefacts** — "atmospheric gas test reading" not "test result"
7. **Basis guidance is operational** — tells the person what to look at on the ground
8. **Proportionality** — evidence effort proportional to control risk profile

## Key Files

| File | Purpose |
|------|---------|
| `scripts/compliance/generate_evidence.py` | Pipeline script — prompt assembly, Gemini call, lint, storage |
| `scripts/compliance/prompts/evidence-system-prompt-v1.md` | Evidence generation prompt (8 constraints + 2 few-shot) |
| `.claude/plans/compliance/COMPLIANCE-EVIDENCE.md` | Design doc v0.2 |
| `data/compliance-evidence/generated/` | Raw JSON outputs per law |
| `data/compliance-evidence/generated/*.pro.json` | Pro benchmark outputs (for Flash comparison) |
| DuckDB `suggested_evidence` table | Staging table for all generated evidence patterns |

## DuckDB Schema

```sql
CREATE TABLE suggested_evidence (
    id VARCHAR PRIMARY KEY,
    law_name VARCHAR NOT NULL,
    control_id VARCHAR NOT NULL,    -- FK to suggested_controls.id
    control_title VARCHAR,
    evidence_json JSON NOT NULL,    -- full artefacts + judgement + strategy
    status VARCHAR,                 -- generated / validated / flagged / accepted / rejected / edited
    validation_flags JSON,
    generation_model VARCHAR,
    generated_at TIMESTAMP,
    base_hash VARCHAR,              -- for three-way merge on regeneration
    customer_edits JSON             -- (generated, edited) pairs for prompt improvement
)
```

## Troubleshooting

- **JSON parse error / "Unterminated string"**: output token limit hit. Ensure `max_tokens=32768` (the default). Laws with 15+ controls may still truncate — split into two calls if needed.
- **429 Too Many Requests**: rate limit. The retry backoff handles this automatically (30/60/120/240s). If persistent, wait 10 minutes for the Gemini spend cap to catch up.
- **No controls found**: run `/control-creation` first for that law.
- **CONSISTENCY flags**: lint caught a real mismatch (e.g., Enterprise+Manual classified as Table Stakes). Fix in post-processing or regenerate with `--force`.
- **INVALID_ENUM**: Flash invented a type outside the schema. Fix in post-processing — map to nearest valid value.
- **Missing judgement fields**: `needs_judgement` was auto-corrected to true but the LLM didn't generate basis_guidance/discriminating_question/drift_signal. Regenerate with `--force`, or accept the gap for manual review.

## Corpus Stats (as of 2026-07-15)

- 220 QQ laws, 1,333 evidence patterns
- 1,315 validated (98.6%), 18 flagged (1.4%)
- Pro pilot (21 laws) + Flash corpus (199 laws)
- Post-processed: 5 normalisation passes, 300+ fixes
