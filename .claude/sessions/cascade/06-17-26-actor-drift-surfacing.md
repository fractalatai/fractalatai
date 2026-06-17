# Session: Actor Drift Surfacing

## Context

**Prior session**: `.claude/sessions/cascade/06-11-26-drrp-qa-plan.md`
**Trigger**: Benchmark analysis showed ~31 Duty provisions missed because the duty-bearer entity isn't in the actor dictionary. Each QA cycle surfaces these gaps. Need a systematic workflow to catch and fix actor drift.

## Problem

The actor dictionary (`docs/actor-dictionary.yaml`) is the single source of truth for who the pipeline recognises as a duty-bearer. When new legislation uses entities not in the dictionary, the pipeline can't anchor a DRRP pattern and returns `drrp_types = []`.

### Missing entities surfaced from 2026-06-17 benchmark analysis

| Entity | Family | Type | Status |
|--------|--------|------|--------|
| relevant body | Climate Change | governed | missing |
| GEMA | Energy | governed | missing |
| approved body | Energy, Fire | governed (regulatory function) | missing |
| appellant | Pollution | governed | missing |
| hazardous substances authority | Planning | governed | missing |
| licensee (non-offshore) | Fire: Explosives | governed | missing — only in Offshore specialist |
| holder of a licence | Energy | governed | missing |
| chief constable | Energy | government | missing |
| Civil Nuclear Police Federation | Energy | governed | missing |

### Previously added (this session)

| Entity | Type | Label |
|--------|------|-------|
| NDA | government | Gvt: Agency: NDA |
| Administrator / scheme administrator | government | Spc: Administrator |
| Compliance body | government | Spc: Compliance Body |
| Certification body | government | Spc: Certification Body |
| Approval body | government | Spc: Approval Body |
| Appeal body | government | Spc: Appeal Body |
| Authorised person | governed | Spc: Authorised Person |
| Responsible undertaking | governed | Org: Responsible Undertaking |
| Her Majesty | government | Crown |

## Workflow: Actor Drift Surfacing

### When to run

After any benchmark run or QA cycle that shows provisions with:
- Gold DRRP but pipeline `drrp_types = []`
- Actors extracted but no regex pattern match
- Classifier used but decomposition produced wrong DRRP type

### Steps

1. **Query**: find provisions where gold has DRRP but pipeline returns none, AND the provision has a modal verb
2. **Extract subjects**: for each, find the text before the first modal — this is likely the duty-bearer
3. **Deduplicate**: group by entity name, count occurrences across families
4. **Classify**: for each new entity, determine governed vs government
5. **Family gate**: decide if the entity is universal or family-specific
6. **Add to YAML**: update `docs/actor-dictionary.yaml` with regex pattern, triggers, type, optional family gate
7. **Test**: run `cargo test -p fractalaw-core` to verify extraction
8. **Re-benchmark**: measure improvement

### Skill design

Create `.claude/skills/actor-drift/` with:
- `SKILL.md` — when to use, workflow steps
- `scripts/surface_missing_actors.py` — queries LanceDB + benchmarks to find gaps
- `scripts/add_actor.py` — adds an entry to the YAML dictionary (or could be manual)

### Governed vs government decision rules

| Signal | Classification |
|--------|---------------|
| Exercises penalty/enforcement powers | government |
| Grants approvals/licences/certificates | government |
| Named government agency/body/authority | government |
| Private company/individual/worker | governed |
| Bears statutory duties as a regulated entity | governed |
| Delegated regulatory function (compliance body) | government |

### Family gating

Some entities only appear in specific legislation families. Adding them to the core dictionary creates false positive risk in other families. Use the `families` field in the YAML:

```yaml
- label: "Offshore: Licensee"
  type: governed
  category: Offshore
  regex_patterns:
    - "[Ll]icen[cs]ees?"
  families: ["OH&S: Offshore"]
```

Entities that appear in 3+ families should be in the core (ungated) dictionary.

## Progress

### Skill created (2026-06-17)

- `.claude/skills/actor-drift/SKILL.md` — workflow documentation
- `.claude/skills/actor-drift/scripts/surface_missing_actors.py` — automated gap surfacing
- Script scans benchmarks or LanceDB for provisions with modals but no DRRP, extracts subjects
- 88 entities surfaced from benchmarks, most are noise (thing-subjects). Manual review required.

### Actors added (2026-06-17)

From surfacing output + Duty miss analysis:

| Label | Type | Category | Notes |
|-------|------|----------|-------|
| Gvt: Agency: GEMA | government | Gvt | Gas and Electricity Markets Authority |
| Spc: Conformity Assessment Body | government | Spc | EU product safety |
| Spc: Approved Body | government | Spc | Grants type examination certificates |
| Spc: Notifying Authority | government | Spc | EU product safety notifications |
| Spc: Responsible Officer | governed | Spc | ESOS scheme officer role |
| Spc: Participant | governed | Spc | ESOS scheme — family-gated to ENERGY |
| Ind: Licensee | governed | Ind | General (non-offshore) licensee |
| Ind: Appellant | governed | Ind | Appeal provisions |
| Ind: Hirer | governed | Ind | Agency worker regulations |

### Still missing (deferred)

| Entity | Family | Blocker |
|--------|--------|---------|
| relevant body | Climate Change | too generic — need compound predicate or family gate |
| hazardous substances authority | Planning | "authority" already matches as Gvt: Authority |
| holder of a licence | Energy | phrase, not a single keyword — needs compound pattern |
| Civil Nuclear Police Federation | Energy | very specific — single law |
| chief constable | Energy | already matches as Gvt: Emergency Services: Police |

## Key files

- `docs/actor-dictionary.yaml` — unified actor dictionary
- `crates/fractalaw-core/src/taxa/actors.rs` — YAML loader, pattern compiler
- `.claude/skills/actor-drift/scripts/surface_missing_actors.py` — gap surfacing script
- `scripts/benchmark_classifier_disagreements.py` — surfaces gaps during benchmark analysis
