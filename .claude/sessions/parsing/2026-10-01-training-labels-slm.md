---
session: "Training labels and SLM retrain at drrp-v1.0"
status: pending
opened: 2026-10-01
closed:
outcome:
issue: 72
related: [74, 75, 76, "parsing/2026-09-30-issue-72.md"]
---

# Session: Training labels and SLM retrain at drrp-v1.0 (PENDING)

## Problem

The position SLM (`gemma3-position`) was fine-tuned in June on about 3,300 labels from the v1 gold (Gemini 2.5 Flash, 15 laws, 354 beneficiaries). That data had the old, inconsistent counterparty/beneficiary split, so the SLM brings it back on every new law and re-parse. Position accuracy was 79.7% overall, 56.6% on beneficiary.

The data model is now complete, and the definitive prompt is written: `scripts/drrp_prompt.py`, `drrp-v1.0-2026-10-01`. Labels made with it train the SLM, so the single run (backlog session) runs once with a model that doesn't recreate the problem.

## Todo

- ⬜ Training set: non-benchmark, live, substantive provisions spread across ~560 laws, stratified so beneficiary and counterparty get 1,000+ actors each. Hold out a test split
- ⬜ Cost check before running. Per call (smoke test, Gemini 3.8 Flash): ~6.35K tokens in, of which 6.1K is the system prompt (cacheable), and ~170 out + 200–600 thinking. **(Jason)** prices it in the console and approves
- ⬜ Label with the definitive prompt, one model, per provision. Resumable and versioned; nothing written to provision_actors
- ⬜ Retrain the SLM (RunPod) on position + type, and purpose per provision. Not `act`
- ⬜ Evaluate against held-out labels, the 50 hand-checked rows (`data/audit/poscorr_sample38_20261001.tsv`) and gold v2 when ready. It must match 3.8 Flash on the counterparty/beneficiary split before the run uses it
- ⬜ LLM tier: `gemini_llm_batch.py` moves to per-provision labelling with `drrp_prompt.py`; the per-actor `--position-correction` mode is retired
- ⬜ Regex `act` for new laws (#75), from the clause verb, measured against the labels
- ⬜ SLM writes purpose: reconcile precedence adjudicated > LLM > SLM > regex; scope re-evaluated after reconcile

## Dependencies

- ✅ Model complete and the definitive prompt written (`parsing/2026-09-30-issue-72.md`, `501edc2`)
- ⬜ RunPod (Jason launches)
- Enables: the single run (`09-29-26-reenrichment-backlog.md`)
