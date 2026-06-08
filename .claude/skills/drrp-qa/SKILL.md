# Skill: DRRP QA — Classification Quality Assurance

## When This Applies

After enrichment to validate the quality of DRRP extraction and actor position classification. Samples provisions from LanceDB, sends to Gemini for independent verification, and applies Bayesian inference to estimate precision.

**Trigger**: User asks to QA enrichment results, validate actor positions, check DRRP quality, or verify Tier 1/Tier 3 output.

## What It Does

1. Samples random provisions from LanceDB (filterable by family and extraction method)
2. For each: assembles provision text + actors struct + DRRP types
3. Sends to Gemini for independent verification of:
   - Actor position classification (active/counterparty/beneficiary/mentioned)
   - DRRP type correctness (is this really a Duty/Right/Responsibility/Power?)
   - Label accuracy (is the actor label correct for this text?)
4. Applies Bayesian inference (Beta-Binomial) to estimate precision with credible intervals
5. Produces a report with precision estimates and detailed review log

## Usage

```bash
# QA all extraction methods from OH&S family (30 samples)
source ~/.bashrc && GEMINI_API_KEY="$GEMINI_API_KEY" \
  /usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --family "OH&S" --sample-size 30

# QA only agentic (Tier 3 LLM) provisions
source ~/.bashrc && GEMINI_API_KEY="$GEMINI_API_KEY" \
  /usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --method agentic --sample-size 20

# QA inherited (Tier 1) provisions only
source ~/.bashrc && GEMINI_API_KEY="$GEMINI_API_KEY" \
  /usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --method inherited --family "OH&S"

# QA all methods across all families
source ~/.bashrc && GEMINI_API_KEY="$GEMINI_API_KEY" \
  /usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --method all --sample-size 40

# Dry run — assemble samples without LLM calls (for inspection)
/usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --dry-run --family "OH&S"

# Generate human-readable DRRP report for a specific law (no LLM calls)
/usr/bin/python3 .claude/skills/drrp-qa/run_qa.py --report UK_uksi_1992_2793
```

## Workflow: Human-Gated QA

1. **Report** — generate markdown table of all DRRP provisions for a law
2. **Human review** — read the report, identify provisions that look wrong
3. **QA with write-back** — run Gemini QA on the full law (or flagged provisions) with `--write-back`
4. **Re-report** — verify corrections landed

## What It Validates

### For regex/inherited provisions (position = active)
- Is the actor actually present in or implied by the provision text?
- Is the DRRP type correct (Duty vs Right vs Responsibility vs Power)?
- Should any actor be counterparty rather than active?

### For agentic provisions (Tier 3 LLM classified)
- Is the active/counterparty distinction correct?
- Does the reason field match the text?
- Are invented labels reasonable (signal for dictionary improvement)?
- Is relates_to correct when populated?

## Environment

- Requires `GEMINI_API_KEY` environment variable
- Uses `/usr/bin/python3` (system Python, not brew)
- Dependencies: `lancedb`, `google-genai`
- LanceDB at `data/lancedb`
- DuckDB at `data/fractalaw.duckdb` (for family lookup)

## Bayesian Inference

- Prior: Beta(1, 1) — uninformative uniform prior
- Each sample updates: correct → Beta(alpha+1, beta), incorrect → Beta(alpha, beta+1)
- Reports: posterior mean, 95% credible interval, interval width
- Suggested stopping criterion: interval width < 0.05
- Ambiguous verdicts don't update the Beta — measuring precision of clear judgements

## Output

Results saved to `data/qa-results/drrp-qa-METHOD-TIMESTAMP.json` with:
- Per-sample verdicts, reasons, and raw LLM text
- Running Bayesian precision estimate
- Summary statistics

## Limitations

This is LLM-checking-LLM for agentic provisions — useful for regression detection and pattern discovery but not a substitute for human review. The real validation is through the sertantai UI.

## References

- Hohfeldian position model: `docs/reviews/gemini-actors-struct-review-20260607.md`
- Design doc: `docs/GAP-C-AGENTIC-EXTRACTION-PLAN.md`
- Tier 3 POC: `.claude/skills/drrp-qa/tier3_poc.py`
