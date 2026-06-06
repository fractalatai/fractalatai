# Skill: Tier 1 Inheritance QA

## When This Applies

After running `taxa enrich --gap-c` to validate precision of deterministic parent-clause inheritance. Also reusable for future tiers by changing the `--method` filter.

**Trigger**: User asks to QA inherited provisions, validate Tier 1 precision, or check Gap C quality.

## What It Does

1. Samples random inherited provisions from LanceDB (filterable by family)
2. For each: assembles target text + parent text + inherited actor
3. Sends to an LLM API for independent verification ("is this inheritance correct?")
4. Applies Bayesian inference (Beta-Binomial) to estimate precision with credible intervals
5. Produces a report with precision estimate, interval width, and detailed review log

## Usage

```bash
# Run with defaults (30 samples from all families, using Gemini)
/usr/bin/python3 .claude/skills/tier1-qa/run_qa.py

# Filter to OH&S family, larger sample
/usr/bin/python3 .claude/skills/tier1-qa/run_qa.py --family "OH&S" --sample-size 50

# Use a different extraction method (for future Tier 2/3 QA)
/usr/bin/python3 .claude/skills/tier1-qa/run_qa.py --method cross_ref

# Dry run — assemble samples without LLM calls (for inspection)
/usr/bin/python3 .claude/skills/tier1-qa/run_qa.py --dry-run
```

## Environment

- Requires `GEMINI_API_KEY` environment variable (or `LLM_API_KEY` + `--provider`)
- Uses `/usr/bin/python3` (system Python, not brew)
- Dependencies: `lancedb`, `google-generativeai` (or `anthropic` for Claude)
- LanceDB at `data/lancedb`
- DuckDB at `data/fractalaw.duckdb` (for family lookup)

## Bayesian Inference

- Prior: Beta(1, 1) — uninformative uniform prior
- Each sample updates: correct → Beta(α+1, β), incorrect → Beta(α, β+1)
- Reports: posterior mean, 95% credible interval, interval width
- Suggested stopping criterion: interval width < 0.05

## Output

Prints a summary table and writes detailed results to `data/tier1-qa-results.json`.

## References

- Design doc: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md` v0.4
- Meta-plan: `.claude/plans/gap-c-tiered-resolution.md`
- Phase 1A session: `.claude/sessions/06-05-26-gap-c-phase-1a.md`
