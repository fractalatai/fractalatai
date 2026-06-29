---
description: Run pipeline coverage statistics and QA checks. Reports base case scope, per-tier coverage, and identifies gaps.
---

# Corpus Stats

## When This Applies

After running any pipeline step (parse, classify, reconcile, SLM, backfill) to verify coverage. Also before session close to confirm QA checks pass.

## Usage

```bash
# QQ corpus (excludes benchmarks)
/usr/bin/python3 scripts/corpus_stats.py

# Benchmarks only
/usr/bin/python3 scripts/corpus_stats.py --benchmarks-only

# Specific laws
/usr/bin/python3 scripts/corpus_stats.py --laws UK_ukpga_1974_37,UK_uksi_1999_3242
```

## What it reports

### Tier 0: Base Case

Provision scope — which provisions are in/out of the pipeline:

- **OUT (section_type)**: headings, part/chapter titles, signed blocks, schedule titles, commencement, tables, notes
- **OUT (short text)**: text < 20 chars — stubs, cross-ref fragments
- **IN SCOPE**: everything else — substantive provisions that may contain DRRP
- **QA check**: no actors exist on OUT provisions

### Tier 2: Classifier (stub — expand per session)

- Actors with regex/dep/classifier coverage
- Eligible actors (embedding + dep features) vs classified
- **QA check**: classifier gap = 0

### Tier 3: Reconciliation + SLM (stub — expand per session)

- Reconciled actor distribution by extraction_method
- pending_slm / pending_llm counts

## Future

- Tier 1 (regex + embed) stats to be added
- Tier 4 (backfill + publish) stats to be added
- Pass 2 base case (purpose-based STRUCTURAL/SUBSTANTIVE split) not yet implemented in stats
- Sertantai integration: persist stats for dashboard/trending (not yet scoped)
