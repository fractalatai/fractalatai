---
session: "Training labels and SLM retrain at drrp-v1.1"
status: active
opened: 2026-10-01
closed:
outcome:
issue: 72
related: [74, 75, 76, "parsing/2026-09-30-issue-72.md"]
---

# Session: Training labels and SLM retrain at drrp-v1.1 (ACTIVE)

**Resumed 2026-10-05** after #60 (`parsing/2026-10-05-issue-60.md`, closed): label at `drrp-v1.1-2026-10-05`.

## Problem

The position SLM (`gemma3-position`) was fine-tuned in June on about 3,300 labels from the v1 gold (Gemini 2.5 Flash, 15 laws, 354 beneficiaries). That data had the old, inconsistent counterparty/beneficiary split, so the SLM brings it back on every new law and re-parse. Position accuracy was 79.7% overall, 56.6% on beneficiary.

The data model is now complete, and the definitive prompt is written: `scripts/drrp_prompt.py`, `drrp-v1.0-2026-10-01`, now `drrp-v1.1-2026-10-05` (#60 applying provisions). Labels made with it train the SLM, so the single run (backlog session) runs once with a model that doesn't recreate the problem.

## Todo

- ⬜ Training set: non-benchmark, live, substantive provisions spread across ~560 laws, stratified so beneficiary and counterparty get 1,000+ actors each, **and holder-unknown Obligations, including duties with an applying provision (701 in 51 laws, #60)**, are well represented. Hold out a test split
- ⬜ Label at `drrp-v1.1-2026-10-05` (#60 applying provisions); build prompts as `scripts/benchmarks/gold_v2/label.py` does (stems + references + `applying()`). Evaluate also on `data/audit/holder60_cases_20261005.tsv`
- ✅ (from #60) Data protection labels added (`37530cd`): `Public: Data Controller`, `Public: Data Processor` (gated to PUBLIC: Data), `Ind: Data Subject`, `Gvt: Agency: Information Commissioner`. DPA s.91(1) now labels `Public: Data Controller`
- ✅ (from #60) Holders in ANOTHER instrument → issue #77 and skill `cross-instrument-holders` (periodic Claude-agent pass, Jason-approved, adjudicated tier). Reminder: SessionStart hook + lat-sync step 7. First pass pending: 6 candidates in 4 laws
- ⬜ Cost check before running. Per call (smoke test, Gemini 3.8 Flash): ~6.35K tokens in, of which 6.1K is the system prompt (cacheable), and ~170 out + 200–600 thinking. **(Jason)** prices it in the console and approves
- ⬜ Label with the definitive prompt, one model, per provision. Resumable and versioned; nothing written to provision_actors
- ⬜ Retrain the SLM (RunPod) on position + type, and purpose per provision. Not `act`
- ⬜ Evaluate against held-out labels, the 50 hand-checked rows (`data/audit/poscorr_sample38_20261001.tsv`) and gold v2 when ready. It must match 3.8 Flash on the counterparty/beneficiary split before the run uses it
- ⬜ LLM tier: `gemini_llm_batch.py` moves to per-provision labelling with `drrp_prompt.py`; the per-actor `--position-correction` mode is retired
- ⬜ Regex `act` for new laws (#75), from the clause verb, measured against the labels
- ⬜ SLM writes purpose: reconcile precedence adjudicated > LLM > SLM > regex; scope re-evaluated after reconcile

## Dependencies

- ✅ #60 remote duty holders resolved and prompt at drrp-v1.1 (`parsing/2026-10-05-issue-60.md`)
- ✅ Model complete and the definitive prompt written (`parsing/2026-09-30-issue-72.md`, `501edc2`)
- ⬜ RunPod (Jason launches)
- Enables: the single run (`09-29-26-reenrichment-backlog.md`)

## #60 follow-ups (2026-10-05)

- **Data protection labels** (actor-drift).
  - "controller" outside DPA/PECR means a landfill site controller (Scottish and Welsh landfill taxes), a hazardous waste controller (EPA 1990), an air traffic controller, a controller of let premises (Equality Act) or the Controller of Audit, so `Public: Data Controller`/`Processor` are gated to `PUBLIC: Data`.
  - The Information Commissioner is ungated ("Information Commissioner" only). Family-gated patterns are only ever added to the governed list (`extract_actors_for_family`), so a government label can't be gated. DPA's bare "the Commissioner" (384 provisions) is left to the LLM/SLM via triggers.
  - Tests: `data_protection_actors_for_public_data_family`, `controller_not_extracted_outside_public_data` (702 pass).
  - Regex actors change only when the DPA laws are re-parsed (the single run).
- **Holders in another instrument (#77):**
  - Jason: a periodic Claude-agent pass, not a prompt change.
  - `enacted_by` is in DuckDB after all: 399 of 491 hub secondary laws; 95 gaps, which legal holds.
  - Skill `.claude/skills/cross-instrument-holders/`:
    - `find_candidates.py` resolves "the Act" from the instrument's own definition (including the Welsh bilingual form), a named Act, or `enacted_by`, and attaches the parent provision with its stem or subsections;
    - `apply_decisions.py` writes approved holders only, to the adjudicated tier, and stamps `last_run`;
    - `check_due.py --hook` runs from a SessionStart hook in `.claude/settings.json`.
  - Spec row "Holder named in another instrument".
  - First run: 6 candidates in 4 laws, all resolved, all parents in the hub.
