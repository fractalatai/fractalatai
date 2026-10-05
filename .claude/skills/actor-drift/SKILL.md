---
description: Surface missing actors from benchmark, LanceDB or Postgres hub QA cycles. Identifies duty-bearers not in the actor dictionary that cause DRRP false negatives.
---

# Skill: Actor Drift — Dictionary Gap Surfacing

## When This Applies

After any benchmark run, QA cycle, or enrichment that shows provisions with gold DRRP but empty pipeline `drrp_types`. The cause is often a duty-bearer entity that isn't in `crates/fractalaw-core/data/actor-dictionary.yaml`.

**Trigger**: User asks to check for missing actors, fix actor drift, expand the dictionary, or investigate why provisions have no DRRP despite having modal verbs.

## What It Does

1. Scans benchmark provisions (NAS), LanceDB or the Postgres hub for provisions where:
   - Gold/expected DRRP exists but pipeline returns `drrp_types = []` (benchmark/LanceDB), or
   - A substantive provision has no `provision_actors` rows at all (Postgres hub; amendment scope excluded)
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

# Postgres hub (the primary store): actorless substantive provisions
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py --source pg --laws UK_uksi_2015_398,UK_ssi_2016_88
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py --source pg --law-file laws.txt --text
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/surface_missing_actors.py --source pg --family "OH&S: Offshore" --min-count 3
```

`--family` matches the DuckDB family with the emoji prefix stripped (substring, case-insensitive).

**Before blaming the dictionary** (#58 lessons): actorless provisions are often not a dictionary gap. Check that
- the law has been re-parsed since its last LAT re-pull (`provision_actors` cascades on `legislation_text` delete);
- the matching entry has `regex_patterns` (entries with only `triggers` are LLM-only);
- family-gated entries match the law's family (DuckDB families carry an emoji prefix; `normalize_family` strips it).

## LLM gaps mode (`llm_gaps.py`): run after every LLM batch

**Jason (2026-10-05):** the actor dictionary is a known gap, so jump on any dictionary diff as soon as it surfaces.

```bash
/usr/bin/python3 /var/home/jason/fractalaw/.claude/skills/actor-drift/scripts/llm_gaps.py      # exit 1 when gaps remain
```

It reports:
- `OTHER:` actors from the training labels (`drrp_training_labels_raw`) and gold v2 at the current prompt version;
- labels not in the dictionary, from those sources and from hub `provision_actors` (free-text labels summarised by tier);
- labels in legal's regex library (`actor_definitions.ex`) that we lack, after `legal_label_map.json`.

`OTHER:` actors whose words now contain a label's trigger, and labels renamed via `renamed_from`, count as covered. Accepted unlabelled actors (too specific) live in `data/training/drrp-v1.1/accepted_other.txt`. The output goes to `data/audit/dictionary_gaps/<date>.json`.

The labeller (`scripts/ml/label_drrp_training.py`) lists the same gaps after every run. A bulk run **pauses** (exit 3) when one new `OTHER:` actor appears `--gate` times (default 3).

**Same-day loop for each gap:**
1. **Add the label**, trigger-only, with the prefix chosen by what the actor is: a group prefix (Ind/Org/SC/Spc/Svc, Gvt/EU) for cross-domain roles, a domain prefix (Data:, Building:, Offshore:, Maritime:, Env:) for domain-specific ones. Or accept it in `accepted_other.txt` if it's too specific.
2. **Check for regex clashes.** Does an existing pattern already catch the words as something else, like `[Oo]fficer` catching "officer of the body corporate" as Gvt: Officer? Fix it with a pattern, a mask (`GOVERNMENT_MASK` in actors.rs) and a test.
3. **Measure the hub rows** it affects and schedule the repair: a migration, or the single run's scope (backlog session checklist).
4. **Tell legal** (the sertantai-legal session): the label, any pattern, any rename.
5. **Re-run the labeller.** Only the provisions the change affects relabel: the dictionary version is tracked per row in `drrp_dictionary_versions`.

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
- Dependencies: `pyyaml`; `psycopg2` + `duckdb` (pg source); `lancedb`, `pyarrow` (benchmark/LanceDB sources)
- Postgres hub at localhost:5433 (fractalaw/fractalaw)
- LanceDB at `data/lancedb`
- Actor dictionary at `crates/fractalaw-core/data/actor-dictionary.yaml`
- Benchmarks at `/mnt/nas/sertantai-data/data/fractalaw-benchmarks/` (optional)

## Limitations

- Subject extraction uses simple regex heuristics — misidentifies thing-subjects as actors
- Can't detect implied actors (context from parent provisions)
- Can't distinguish entities that SHOULD be actors from entities that happen to appear before a modal
- Manual review of output is always required before adding to dictionary
