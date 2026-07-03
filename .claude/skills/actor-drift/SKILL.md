---
description: Surface missing actors from benchmark or LanceDB QA cycles. Identifies duty-bearers not in the actor dictionary that cause DRRP false negatives.
---

# Skill: Actor Drift — Dictionary Gap Surfacing

## When This Applies

After any benchmark run, QA cycle, or enrichment that shows provisions with gold DRRP but empty pipeline `drrp_types`. The cause is often a duty-bearer entity that isn't in `crates/fractalaw-core/data/actor-dictionary.yaml`.

**Trigger**: User asks to check for missing actors, fix actor drift, expand the dictionary, or investigate why provisions have no DRRP despite having modal verbs.

## What It Does

1. Scans benchmark provisions (NAS) or LanceDB for provisions where:
   - Gold/expected DRRP exists but pipeline returns `drrp_types = []`
   - A modal verb is present (shall/must/may) — no modal = LLM territory
2. Extracts the grammatical subject before the modal — likely the missing actor
3. Deduplicates and groups by entity name across families
4. Reports entities not in the actor dictionary, ranked by frequency

## Usage

```bash
# Scan golden benchmarks (requires NAS mount)
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py

# Show provision text for each missing entity
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py --text

# Only show entities appearing 2+ times
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py --min-count 2

# Scan full LanceDB corpus (no benchmarks needed)
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py --source lancedb
```

## Workflow: Fixing Actor Drift

1. **Run the surfacing script** — get list of missing entities
2. **Filter noise** — thing-subjects (notice, report, order) are not actors. Focus on person/org/body entities
3. **Classify each entity** — governed or government?
   - Exercises penalty/enforcement/approval powers → government
   - Private company/individual/worker bearing duties → governed
4. **Decide family gating** — does this entity only appear in one family? Use `families:` field. Appears in 3+ families → core dictionary
5. **Add to YAML** — edit `crates/fractalaw-core/data/actor-dictionary.yaml` with label, type, regex_patterns, triggers
6. **Test** — `cargo test -p fractalaw-core` (the YAML is embedded at compile time)
7. **Re-benchmark** — measure improvement

## Governed vs Government Decision Rules

| Signal | Classification |
|--------|---------------|
| Exercises penalty/enforcement powers | government |
| Grants approvals/licences/certificates | government |
| Named government agency/body/authority | government |
| Private company/individual/worker | governed |
| Bears statutory duties as a regulated entity | governed |
| Delegated regulatory function (compliance body) | government |

## Environment

- `/usr/bin/python3` (system Python)
- Dependencies: `lancedb`, `pyarrow`, `pyyaml`
- LanceDB at `data/lancedb`
- Actor dictionary at `crates/fractalaw-core/data/actor-dictionary.yaml`
- Benchmarks at `/mnt/nas/sertantai-data/data/fractalaw-benchmarks/` (optional)

## Limitations

- Subject extraction uses simple regex heuristics — misidentifies thing-subjects as actors
- Can't detect implied actors (context from parent provisions)
- Can't distinguish entities that SHOULD be actors from entities that happen to appear before a modal
- Manual review of output is always required before adding to dictionary
