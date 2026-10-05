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

- ✅ Training set: `scripts/ml/sample_drrp_training.py` → `data/training/drrp-v1.1/sample.csv`, 6,121 provisions in 656 laws (5,524 train / 597 test, split by law, 61 test laws). Details below
- ⬜ Label at `drrp-v1.1-2026-10-05` (#60 applying provisions); build prompts as `scripts/benchmarks/gold_v2/label.py` does (stems + references + `applying()`). Evaluate also on `data/audit/holder60_cases_20261005.tsv`
- ✅ (from #60) Data protection labels added (`37530cd`): `Public: Data Controller`, `Public: Data Processor` (gated to PUBLIC: Data), `Ind: Data Subject`, `Gvt: Agency: Information Commissioner`. DPA s.91(1) now labels `Public: Data Controller`
- ✅ (from #60) Holders in ANOTHER instrument → issue #77 and skill `cross-instrument-holders` (periodic Claude-agent pass, Jason-approved, adjudicated tier). Reminder: SessionStart hook + lat-sync step 7. First pass pending: 6 candidates in 4 laws
- ⬜ Cost check before running. Per call (smoke test, Gemini 3.8 Flash): ~6.35K tokens in, of which 6.1K is the system prompt (cacheable), and ~170 out + 200–600 thinking. **(Jason)** prices it in the console and approves. **Proposed:** a 200-provision pilot first (measure beneficiary/counterparty yield and real tokens), then the full run
- ⬜ **Before labelling (Jason 2026-10-05):**
  - ✅ LAT at legal's latest. The `pull-lat --stale` dry run (2026-10-05, after legal restarted :7447) gives 741 in_sync and 0 text changes; 154 not_held; the 4 known delete candidates were never applied
  - ✅ Actor dictionary reconciled with legal (`050e829`):
    - Jason's decisions: YAML canonical; group prefixes for cross-domain roles, domain prefixes (Data:, Offshore:, Maritime:, Env:) for domain-specific ones; trigger-only additions; Spc: Authorised Person governed.
    - 175 labels: 45 added, 9 renamed via `renamed_from`.
    - Legal has the final list and rename map.
  - ✅ Label rename migration applied 2026-10-05 (`scripts/migrations/rename_actor_labels_20261005.py`): 2,599 provision_actors rows and 53 gold rows renamed, 16 collisions merged; no old labels remain. Backup: `data/backups/pre_actor_rename_20261005.dump` (pg_dump -Fc of provision_actors, gold_benchmarks, gold_v2). Legal's side waits for the single run's publish (backlog checklist)
- ⬜ Label versions now include the dictionary hash (`label_version()`, currently `drrp-v1.1-2026-10-05+dict.85e950d2`), so the 245 pilot/probe labels made with the old dictionary will be relabelled in the family pilots
- ⬜ **Label by family, in order of actor-dictionary confidence** (memory `feedback_actor_dictionary_gap`). Per family group: a pilot of ~50 → add the OTHER actors to the dictionary (and tell legal) → bulk. High-confidence families first (OH&S sub-families, Fire, Nuclear, Consumer/Product Safety…). The LLM work improves the dictionary
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

## Training sample (2026-10-05)

`scripts/ml/sample_drrp_training.py` (seed 60). One unit is one provision.

**Universe:** 141,511 substantive, non-repealed provisions with text, outside Schedules, in 656 live hub laws. Excluded: 4,018 laws that are benchmark (`gold_benchmarks`, `is_benchmark`), gold v2 (7), `enabling_extent` (0 today) or revoked.

**Strata:** a provision takes the first that applies; flags kept. Round-robin across laws with a per-law cap of 3% of the stratum.

| Stratum | Available (laws) | Sampled train / test |
|---|---|---|
| ben (any tier beneficiary, or protective-purpose wording) | 15,401 (535) | 1,361 / 139 |
| cp (any tier counterparty) | 19,746 (555) | 1,087 / 113 |
| app (holder unknown + applying provision, #60) | 181 (30) | 96 / 25 |
| hu (other holder-unknown Obligation) | 3,905 (389) | 725 / 75 |
| lib (Liberty) | 8,716 (485) | 450 / 50 |
| none (no Obligation/Liberty: the negatives, raised to 1,000 for the #65 false positives) | 83,132 (644) | 902 / 98 |
| general | 10,430 (542) | 903 / 97 |

**Notes:**
- Flags in the sample: ben 1,500, cp 1,923, holder-unknown 1,402, applying 141.
- The tier positions are the noisy ones being replaced; they only steer the sample. The protective-wording cue adds only 244 provisions corpus-wide and is mixed ("for the benefit of the community").
- **Beneficiary yield is unknown until labelled.** If ~60% confirm, that's ~900 beneficiary actors, short of the 1,000+ target, so the pilot measures it before the full run.
- **Token estimate from gold v2 batch 1** (Gemini 3.8 Flash, 1,218 provisions: 6.00M in, 1.43M out incl. thinking; explicit caching was off then): ~4.9K in and ~1.2K out per provision. For 6,121 provisions that's ~30M in and ~7.2M out; with the 6.1K system prompt cached, most input is cache reads.

## Pilot (2026-10-05)

**Run:** `scripts/ml/label_drrp_training.py --pilot 200`, i.e. 206 provisions proportional per stratum. Raw responses go to `drrp_training_labels_raw`, declared in `scripts/pg_schema.sql`.

**Cost:**
- **$0.78**, or $3.76 per 1,000 provisions, with no errors.
- The explicit context cache works: 6,677 of 6,851 input tokens per call are cached.
- Output averages 835 tokens per provision, 706 of them thinking.
- Full sample (6,121) ≈ **$23** at standard rates.

**Yields:**
- Relation yes 123 / no 83.
- Actors: active 125, mentioned 104, counterparty 37, beneficiary 4.
- **Counterparty:** 0.18 per provision → ~1,100 in the full sample (on target).
- **Beneficiary:** 0.02 per provision → ~120 in the full sample (target 1,000+). The `ben` stratum, drawn from old tier labels, yielded 4 beneficiaries from 49 provisions. Those old labels mostly sit on definitions, notices and procedure. The model's calls look right; genuine beneficiaries are rare under the act test.
- **Holder unknown:** the `hu` stratum gave holders in 12 of 26 (actor gaps fixed). The `app` stratum gave 19 inferred active holders from 10 provisions.
- **OTHER labels:** 15 in total, e.g. consignor, consignee, producer, tenant, master of a vessel, safety committee, EU Council/Parliament. All other labels are dictionary labels.

**Protective-purpose probe:** 39 provisions where a duty word (in the provision or its stem) occurs with protective wording ("ensure … health/safety/risk", "health and safety of", "exposed to risks", "well-being"). They gave **0.36 beneficiaries per provision**, e.g. COSHH NI reg.7(1) employer → employee, quarry operator → workers. 1,654 such provisions are unsampled (241 laws), which projects to ~600 more beneficiaries for ~$6.

## Family order for labelling (2026-10-05)

**Jason:** the actor dictionary is a known gap, stale against legal's list. Order the LLM work by dictionary confidence per family, run high-confidence families first, and pilot even those so new actors are caught before the bulk run.

**First proxy (sample laws):** the share of "shall/must" provisions where the existing tiers found no actor at all.
- **Lowest:** Nuclear 19%, Fire 20%, Mines & Quarries 20%, Working Time 21%, Consumer/Product Safety 22%, Gas & Electrical 22%, Planning 23%, Offshore 24%, Dangerous & Explosive Substances 25%.
- **Occupational / Personal Safety:** 29% (776 sampled provisions).
- **Highest:** Road Safety 55%, Rail 47%, Climate Change 43%, Data 42%, Air Safety 40%.

**Caveat:** this proxy mixes dictionary gaps with passive/impersonal duties (#60). The per-family pilot's OTHER rate is the real confidence signal.

## Pilot re-run with the reconciled dictionary (2026-10-05)

**Run:** 206 provisions at `drrp-v1.1-2026-10-05+dict.85e950d2`, $0.85 ($4.13 per 1,000; the larger dictionary adds ~900 cached tokens), no errors.

**OTHER labels: 15 → 2.** The new labels are used: consignor, consignee, producer, holder, tenant, Maritime: Master, safety committee, advisor, agent, EU Council and Parliament. Remaining:
- "Officer of a body corporate": the corporate-offence officer, 134 provisions in 103 laws. Added as trigger-only `Ind: Company Officer`. **Regex risk:** `[Oo]fficer` (Gvt: Officer) catches it as a government officer; fix when patterns are added.
- "Taker of provisional measure": too specific, left as OTHER.

**Yields unchanged:** beneficiary 6 (0.03 per provision), counterparty 36 (0.17).

## Dictionary-gap loop built (2026-10-05, `ea2a0fd`)

**Jason:** jump on any actor dictionary diff as soon as it surfaces. Built:
1. **Labeller gap listing.** Every report lists the `OTHER:` and non-dictionary labels with example provisions.
2. **`llm_gaps.py`** (actor-drift skill, LLM gaps mode). It checks the training labels, gold v2 at the current prompt, the hub's free-text labels, and legal's regex library (via `legal_label_map.json`). Output goes to `data/audit/dictionary_gaps/<date>.json`, and it exits 1 on gaps.
3. **Relabel only what a dictionary change affects.**
   - `prompt_version` is now the rules only; each row has a `dict_version`, and the versions are registered in `drrp_dictionary_versions`. The 451 existing rows were split into these columns.
   - A provision relabels only if its response has OTHER labels or labels that no longer exist, or its text matches a label added since.
   - Example: the Company Officer change relabelled 4 of 206 pilot provisions instead of all 206.
4. **Bulk-run gate.** A bulk run pauses (exit 3) when a new OTHER actor appears 3 times. Tested offline: it paused after 6 of 30. `accepted_other.txt` holds actors accepted as unlabelled.

**First gap check** (226 → 3). New trigger-only labels:
- Building: Accountable Person and Principal Accountable Person (~160 uses in gold v2);
- Ind: Resident, Complainant, Accused; Org: Licensor;
- Gvt: Minister: Lord Advocate, Gvt: Parliament; SC: Buyer; Svc: Statutory Undertaker;
- extra triggers on existing labels.

**Open (Jason):**
- **`Ind: Public` (1,148 hub rows).** A correlative rule (`correlative-rules.yaml` rule 3) infers it as beneficiary for every active enforcement authority. The label is wrong (the dictionary label is `Public`; `actor_aliases.py` is fixed). The rule also contradicts the spec, which needs an explicit protective purpose for a beneficiary. Decide: retire the rule, or fix its label. The rows are inference-only, so they regenerate at re-parse. Asked legal whether they hold it.
- **116 adjudicated rows with legacy free-text labels.** Benchmark gold carried forward; resolved at the gold v2 cutover.

## Family group 1 pilot (2026-10-05)

**Sample regenerated** with a `prot` stratum (duty word + protective-purpose wording, in the provision or its stem) and `ben` cut to 500: 6,959 provisions (old sample kept as `sample_20261005_v1.csv`).

**Group 1** (Nuclear, Fire incl. Dangerous & Explosive Substances, Mines & Quarries, Gas & Electrical, Offshore Safety, Consumer/Product Safety) has 1,357 sample provisions.

**Pilot (50 provisions, $0.18, no errors):**
- beneficiary 0.22 per provision: `prot` gave 10 from 19 (0.53);
- counterparty 0.16; relation yes 36 / no 14.

**Gaps → fixed the same day:**
- "economic operator" → `SC: Economic Operator`, with a pattern. Regex clash: `Operator`'s pattern included "economic operator" (338 hub rows in 26 laws). Fixed by a new per-label `exclude` field (actors.rs; excluded spans are blanked for that label only, and matched spans are removed by range).
- "prosecutor" → `Gvt: Prosecutor` (trigger-only).

**Re-run:** 3 of 50 relabelled; 0 gaps.

## Family group 1 bulk (2026-10-05)

**Result:** 1,357 provisions labelled, no errors, $5.08 for the group ($3.74 per 1,000). The gate never triggered: every new actor appeared once.
- Actors: active 900, mentioned 788, counterparty 241, **beneficiary 228** (0.17 per provision; `prot` gave 212 from 521). Projects to ~1,180 beneficiaries across the sample.
- Holder unknown: 40 of 122 in `hu`; the other 82 got holders from the text. Relation yes 877 / no 480.

**13 one-off gaps:**
- **Added (trigger-only, `fc1adf8`):** Spc: Accreditation Body (UKAS / national accreditation body), Spc: Verifier, Spc: Laboratory, Svc: Gas Transporter, Ind: Transferor, Ind: Transferee, Org: Social Partner.
- **Accepted as unlabelled:** interested parties, entrusted body, members of a fact-finding mission, service authorities of a visiting force.
- **Relabel:** 23 of 1,357; 0 gaps after.

**Remaining sample by family:** OH&S Occupational/Personal 1,247; Environmental Protection 425; Maritime Safety 397; Climate Change 337; Water 309; **(no family) 288**; Waste 285; Town & Country Planning 188; HR Employment 179; Energy 176; Building Safety 173; Road Safety 169; …

**Open:** the 288 no-family provisions can't be selected by `--family`; they need their own run.

## Family group 2 pilot (2026-10-05)

**Group 2:** OH&S Occupational/Personal Safety, HR Employment, HR Working Time, Town & Country Planning; 1,635 sample provisions.

**Pilot:** 50 provisions, $0.20, no errors, **0 dictionary gaps**.
- Beneficiary 0.12 per provision (`prot` 6 from 21); counterparty 0.18; relation yes 30 / no 20.
- 1,561 remain for the bulk run (~$6).
