---
description: Use when the user asks you to create a new Claude Code skill. Guides structure, naming, and conventions.
---

# Create a Claude Code Skill

## When This Applies

When the user says "create a skill for X" or "make a skill that does Y".

## What a Skill Is

A skill is a **directory** with a required `SKILL.md` file that gives Claude instructions. It is NOT an executable — it's a prompt that Claude follows when invoked.

## Structure

```
.claude/skills/<skill-name>/
├── SKILL.md              # Instructions for Claude (required)
└── scripts/              # Helper scripts Claude executes (optional)
    ├── helper.py
    └── validate.sh
```

## Creating a New Skill — Step by Step

### 1. Choose a name

- Use lowercase kebab-case: `embedding-backfill`, `drrp-qa`, `nas-backup`
- Prefix with topic if part of a group: `embedding-backfill`, `embedding-train`
- Keep it short and descriptive

### 2. Create the directory

```bash
mkdir -p .claude/skills/<skill-name>
```

### 3. Write SKILL.md

The file has two parts:

**YAML frontmatter** (optional but recommended):
```yaml
---
description: One-line description of when to use this skill
---
```

**Markdown body** with:
- `## When This Applies` — triggers for when Claude should use this skill
- `## Usage` — how to invoke it (commands, flags, examples)
- `## How It Works` — what the skill does step by step
- `## Notes` — gotchas, constraints, references

### 4. Add helper scripts (if needed)

Put executable scripts in a `scripts/` subdirectory. Reference them from SKILL.md using `${CLAUDE_SKILL_DIR}`:

```markdown
Run the backfill:
```bash
/usr/bin/python3 ${CLAUDE_SKILL_DIR}/scripts/backfill.py --dry-run
```
```

Scripts are **executed by Claude**, not loaded as context. Keep them standalone with their own `--help`.

### 5. DO NOT put raw Python in SKILL.md

SKILL.md is instructions for Claude, not code. If a skill needs code:
- Put it in `scripts/`
- Reference it from SKILL.md
- Make the script independently runnable with argparse

### 6. Keep SKILL.md under 500 lines

For detailed reference material, create separate files (`reference.md`, `examples.md`) and link from SKILL.md.

## Conventions in This Project

- Skills live at `.claude/skills/<name>/SKILL.md`
- Scripts use `/usr/bin/python3` (not bare `python3` — brew shadows it)
- LanceDB at `data/lancedb`, DuckDB at `data/fractalaw.duckdb`
- GEMINI_API_KEY from environment when needed
- Ollama at `http://localhost:11434` when needed

## Existing Skills for Reference

| Skill | Purpose |
|---|---|
| `nas-backup` | NAS backup procedure |
| `bulk-enrichment` | Per-family batch enrichment ops |
| `enrich-and-publish` | Enrich → publish workflow |
| `drrp-qa` | QA with Gemini + write-back |
| `embedding-backfill` | Compute missing embeddings |
| `lancedb-validation` | LanceDB query patterns |
| `lat-qa` | Upstream data quality checks |

## Checklist

Before committing a new skill:
- [ ] Directory created at `.claude/skills/<name>/`
- [ ] `SKILL.md` has description frontmatter
- [ ] `SKILL.md` has "When This Applies" section
- [ ] Any Python scripts are in `scripts/` not inline
- [ ] Scripts use `/usr/bin/python3`
- [ ] Tested: skill appears in the skill list after creation
