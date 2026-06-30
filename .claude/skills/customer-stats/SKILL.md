---
description: Run pipeline coverage statistics and QA checks per customer legal register. Reports base case scope, per-tier coverage, and identifies gaps.
---

# Customer Stats

## When This Applies

After running any pipeline step (parse, classify, reconcile, SLM, backfill) to verify coverage. Also before session close to confirm QA checks pass. Run against a customer's legal register to assess pipeline readiness.

## Usage

```bash
# Customer corpus from a law file (the standard way)
/usr/bin/python3 scripts/corpus_stats.py --law-file data/qq-applicable-laws.csv

# Benchmarks only
/usr/bin/python3 scripts/corpus_stats.py --benchmarks-only

# Specific laws
/usr/bin/python3 scripts/corpus_stats.py --laws UK_ukpga_1974_37,UK_uksi_1999_3242

# All laws (no filter)
/usr/bin/python3 scripts/corpus_stats.py --all

# Default (non-benchmark) — avoid, prefer --law-file for customer-specific view
/usr/bin/python3 scripts/corpus_stats.py
```

## Customer law file

The `--law-file` flag takes a CSV of comma-separated law names — one line, no header. This is the customer's legal register. Example:

```
UK_ukpga_1974_37,UK_uksi_1999_3242,UK_uksi_2005_1541,...
```

Current customer files:
- `data/qq-applicable-laws.csv` — QQ customer (274 laws)

Future: query sertantai for a customer's applicable laws and pipe to the script, or export from sertantai as CSV.

## What it reports

### Tier 0: Base Case

Provision scope — which provisions are in/out of the pipeline:

- **OUT (section_type)**: headings, part/chapter titles, signed blocks, schedule titles, commencement, tables, notes
- **OUT (short text)**: text < 20 chars — stubs, cross-ref fragments
- **IN SCOPE**: everything else — substantive provisions that may contain DRRP
- **QA check**: no actors exist on OUT provisions

### Tier 1: Regex + Embed

Foundation coverage for in-scope provisions:

- **Has embedding**: provisions with 384-dim MiniLM vector (required for classifier)
- **Embedding gap**: in-scope provisions without embedding — lists laws with gaps
- **Provisions with actors**: provisions that have actors in provision_actors
- **Regex coverage**: actors with regex_position
- **QA check**: embedding gap = 0 for full pipeline readiness

### Tier 2: Classifier (stub — expand per session)

- Actors with regex/dep/classifier coverage
- Eligible actors (embedding + dep features) vs classified
- **QA check**: classifier gap = 0

### Tier 3: Reconciliation + SLM (stub — expand per session)

- Reconciled actor distribution by extraction_method
- pending_slm / pending_llm counts

## Interpreting results

- **Tier 0 PASS + Tier 1 FAIL**: laws need embedding before classifier can run
- **Tier 1 PASS + Tier 2 FAIL**: dep features or classifier needs running
- **All PASS**: corpus ready for publish

## Future

- Tier 4 (backfill + publish) stats
- Pass 2 base case (purpose-based STRUCTURAL/SUBSTANTIVE split)
- Sertantai integration: query customer laws directly, persist stats for dashboard
