---
session: "Training labels and SLM retrain at drrp-v1.1"
status: suspended
opened: 2026-10-01
closed:
outcome:
issue: 72
related: [74, 75, 76, "parsing/2026-09-30-issue-72.md"]
---

# Session: Training labels and SLM retrain at drrp-v1.1 (SUSPENDED)

**Suspended 2026-10-06 (Jason): the approach changed.** The rulings and the one-prompt-for-everything labelling had grown complex and costly. Review: `docs/architecture/DRRP-LABELLING-REVIEW-2026-10-06.md`.

**Jason:** we already have the pipeline, and it starts with regex. **The mistake was building training data from the LLM end of the pipe, not the regex end.**

The work continues under the meta-plan `parsing/2026-10-06-pipeline-from-the-regex-end.md`. The labels made here become its **evaluation data**: Gemini v1.3 on all 6,959, plus refereed batches 1–2.

**State at suspension:**
- **Gemini labels:** 6,959 provisions at `drrp-v1.3-2026-10-05`, total spend ~$27 + refreshes ~$10.
- **Second model + Claude referee:** batch 1 (GPT-5.5, 1,357; consensus + 322 referee decisions) and batch 2 (GPT-5.4-mini, 1,635; consensus + 137 auto-Gemini + 544 referee decisions). Files are in `data/training/drrp-v1.1/referee/`.
- **Prompt v1.4** (rulings 1–15) and spec are committed; the v1.4 label refresh (489 Gemini, ~$2) is **not run**.
- **OpenAI credit:** ~$12.70 left. No paid run is pending.
- **Dictionary:** 132 → ~245 labels, reconciled with legal.

**Tooling** (all in `scripts/ml/`):
- `label_drrp_training.py`: `--carry-from`, `--max-cost`, gate, stale check, `--model-name`, `--ids`;
- `compare_training_labels.py`, `score_vs_referee.py`, `REFEREE_TRAINING_LABELS.md`;
- actor-drift `llm_gaps.py`.

## Problem

The position SLM (`gemma3-position`) was fine-tuned in June on about 3,300 labels from the v1 gold (Gemini 2.5 Flash, 15 laws, 354 beneficiaries). That data had the old, inconsistent counterparty/beneficiary split, so the SLM brings it back on every new law and re-parse. Position accuracy was 79.7% overall, 56.6% on beneficiary.

The data model is now complete, and the definitive prompt is written: `scripts/drrp_prompt.py`, `drrp-v1.0-2026-10-01`, now `drrp-v1.1-2026-10-05` (#60 applying provisions). Labels made with it train the SLM, so the single run (backlog session) runs once with a model that doesn't recreate the problem.

## Todo

- ✅ Training set: `scripts/ml/sample_drrp_training.py` → `data/training/drrp-v1.1/sample.csv`, 6,121 provisions in 656 laws (5,524 train / 597 test, split by law, 61 test laws). Details below
- ✅ (superseded: labelled at v1.3) Label at `drrp-v1.1-2026-10-05` (#60 applying provisions); build prompts as `scripts/benchmarks/gold_v2/label.py` does (stems + references + `applying()`). Evaluate also on `data/audit/holder60_cases_20261005.tsv`
- ✅ (from #60) Data protection labels added (`37530cd`): `Public: Data Controller`, `Public: Data Processor` (gated to PUBLIC: Data), `Ind: Data Subject`, `Gvt: Agency: Information Commissioner`. DPA s.91(1) now labels `Public: Data Controller`
- ✅ (from #60) Holders in ANOTHER instrument → issue #77 and skill `cross-instrument-holders` (periodic Claude-agent pass, Jason-approved, adjudicated tier). Reminder: SessionStart hook + lat-sync step 7. First pass pending: 6 candidates in 4 laws
- ✅ Cost check: Jason approved per group; actual $3.92 per 1,000 with the explicit context cache
- ✅ **Before labelling (Jason 2026-10-05):**
  - ✅ LAT at legal's latest. The `pull-lat --stale` dry run (2026-10-05, after legal restarted :7447) gives 741 in_sync and 0 text changes; 154 not_held; the 4 known delete candidates were never applied
  - ✅ Actor dictionary reconciled with legal (`050e829`):
    - Jason's decisions: YAML canonical; group prefixes for cross-domain roles, domain prefixes (Data:, Offshore:, Maritime:, Env:) for domain-specific ones; trigger-only additions; Spc: Authorised Person governed.
    - 175 labels: 45 added, 9 renamed via `renamed_from`.
    - Legal has the final list and rename map.
  - ✅ Label rename migration applied 2026-10-05 (`scripts/migrations/rename_actor_labels_20261005.py`): 2,599 provision_actors rows and 53 gold rows renamed, 16 collisions merged; no old labels remain. Backup: `data/backups/pre_actor_rename_20261005.dump` (pg_dump -Fc of provision_actors, gold_benchmarks, gold_v2). Legal's side waits for the single run's publish (backlog checklist)
- ✅ (replaced by the `dict_version` column and stale check) Label versions now include the dictionary hash (`label_version()`, currently `drrp-v1.1-2026-10-05+dict.85e950d2`), so the 245 pilot/probe labels made with the old dictionary will be relabelled in the family pilots
- ✅ (Gemini: all 5 groups; second model + referee: batches 1–2) **Label by family, in order of actor-dictionary confidence** (memory `feedback_actor_dictionary_gap`). Per family group: a pilot of ~50 → add the OTHER actors to the dictionary (and tell legal) → bulk. High-confidence families first (OH&S sub-families, Fire, Nuclear, Consumer/Product Safety…). The LLM work improves the dictionary
- ✅ Label with the definitive prompt (drrp-v1.1), one model, per provision: all 6,959 sample provisions labelled 2026-10-05, $27.28, 0 errors; 0 open dictionary gaps (summary below)
- ⏸️ (moved to meta-plan phase 4: SLM on the residual only, relation + actors, no purpose) Retrain the SLM (RunPod) on position + type, and purpose per provision. Not `act`
- ⏸️ (moved to meta-plan phase 1: measure every tier against these labels) Evaluate against held-out labels, the 50 hand-checked rows (`data/audit/poscorr_sample38_20261001.tsv`) and gold v2 when ready. It must match 3.8 Flash on the counterparty/beneficiary split before the run uses it
- ⏸️ (moved to meta-plan phase 5: short relation/actor prompt on the low-confidence residual) LLM tier: `gemini_llm_batch.py` moves to per-provision labelling with `drrp_prompt.py`; the per-actor `--position-correction` mode is retired
- ⏸️ (moved to meta-plan phase 2) Regex `act` for new laws (#75), from the clause verb, measured against the labels
- ⏸️ (superseded by meta-plan phase 2/3: regex purpose + guards first, then a cheap purpose classifier) SLM writes purpose: reconcile precedence adjudicated > LLM > SLM > regex; scope re-evaluated after reconcile

- ⏸️ (stopped by Jason 2026-10-06) Second model + referee for batches 3–5
- ⏸️ (held: the review may move purpose rulings out of the LLM prompt) v1.4 label refresh (489 Gemini, ~$2)
- ⏸️ (meta-plan phase 6) Edge-case rule governance: new rules enter the relation prompt only if they can change a verdict, holder or correlative

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

## Family group 2 bulk (2026-10-05)

**Result:** 1,635 provisions labelled, no errors, $6.37 for the group ($3.90 per 1,000).
- Actors: active 1,144, mentioned 923, counterparty 334, **beneficiary 277** (`prot` gave 249 from 674); 0.17 per provision.
- Holder unknown: 40 of 143 in `hu`.

**Running totals (groups 1 + 2):** 2,992 provisions, 505 beneficiaries, 575 counterparties.

**9 gaps, fixed the same day (`c5876c1`):**
- **"Temporary work agency":** it was on the extraction **blacklist** (to stop `Gvt: Agency`), which hid it from the governed pass as well. Moved to `GOVERNMENT_MASK` with "agency worker"; `Org: Temporary Work Agency` gets a pattern; test added. No hub repair is needed: there were never wrong rows, the actor was just invisible.
- **Trigger-only:** Org: Developer, Org: Community Body (incl. crofting), SC: Filler, EU: Economic and Social Committee, EU: Advisory Committee on Safety and Health at Work.
- **Accepted:** visiting force, headquarters.

**Relabel:** 19 of 1,635; 0 gaps.

## Family group 3 pilot (2026-10-05)

**Group 3:** Environmental Protection, Waste, Water & Wastewater, Pollution, Air Quality, Noise; 1,222 sample provisions.

**Pilot:** 50 provisions, $0.17, no errors, **0 dictionary gaps**.
- Beneficiary 0.00, as expected: environmental duties protect the environment, not a party, and `prot` is small here (105 in the group).
- Counterparty 0.14; fewer actors per provision (49 / 50); 6 holder-unknown.
- 1,172 remain for the bulk run (~$4).

## Family group 3 bulk (2026-10-05)

**Result:** 1,222 provisions labelled, no errors, $4.77 for the group ($3.91 per 1,000).
- Actors: active 623, mentioned 665, counterparty 183, **beneficiary 3**, as expected for environmental law.
- Holder unknown: 42 of 181 in `hu`; relation yes 682 / no 540.

**Gaps:**
- Added: SC: T&L: Notifier (waste shipments), Ind: Designated Person.
- Accepted: (envisaged) country of destination, Secretariat of the Basel Convention.
- Relabel: 7 of 1,222; 0 gaps after.

**Running totals (groups 1–3):** 4,214 provisions; 508 beneficiaries, 758 counterparties; $16.22.

## Family group 4 pilot (2026-10-05)

**Group 4:** Maritime Safety, Energy, Building Safety, Planning & Infrastructure, Health (Public, Drug & Medicine, Coronavirus), Wildlife, Marine & Riverine, Animals, Plant Health, Fisheries, GMOs, Agriculture (incl. pesticides), Food, Trees, Historic Environment, Buildings; 1,564 sample provisions.

**Pilot:** 51 provisions, $0.25, no errors.
- Thinking is heavier (965 tokens per provision): $4.96 per 1,000.
- Counterparty 0.37 per provision (licensing, grants, planning), beneficiary 0.04.

**Gap:** "eligible farm business" → `Org: Farm Business` (trigger-only). 0 gaps after relabel.

## Family group 4 bulk (2026-10-05)

**Result:** 1,564 provisions labelled, no errors, $6.54 for the group ($4.18 per 1,000).
- Actors: active 864, mentioned 959, counterparty 276, beneficiary 146 (`prot` 140 from 419).
- Holder unknown: 37 of 188 in `hu`.

**13 gaps (16 uses):**
- **Added (trigger-only):** Spc: Health Care Professional (27 provisions/9 laws), Gvt: Consular Officer (consular/diplomatic/maritime representative; 27/12), Spc: Public Analyst (food examiner; 28/4), Gvt: CfD Counterparty (15/2), Spc: Insolvency Practitioner, Spc: Inspection Body (implementing body), Ind: Pilot.
- **Triggers:** NRI → Org: Investor; approved certifier → Spc: Certification Body.
- **Accepted:** welfare attorney, interested party.
- **Relabel:** 18 of 1,564; 0 gaps after.

**Running totals (groups 1–4):** 5,778 provisions; 654 beneficiaries, 1,034 counterparties; $22.76.

## Overlaps from legal's case-insensitive re-check (2026-10-05)

Legal's first checks missed bracket-cased patterns; the re-check found three overlaps that we have too. Fixed (`9b91854`):
- **Verifier** was an alternative under `Spc: Inspector`; it is now its own pattern (182 hub rows).
- **Gas transporter** was caught by `SC: T&L: Carrier`'s "transporter"; it's now excluded from Carrier and has its own pattern (64 hub rows).
- **OH Advisor** keeps the occupational forms; plain doctor/physician/nurse move to `Spc: Health Care Professional`.

**Stale check extended:** a provision also relabels when its response uses a label whose triggers or patterns changed. 245 of 5,778 relabelled (~$1); only accepted one-offs remain.

## Family group 5 pilot (2026-10-05)

**Group 5:** everything still unlabelled: Climate Change, Road/Air/Rail transport, Data, Finance, bare PUBLIC, HR Insurance, X: No Family, todo, Oil & Gas, and laws with no family (`--family "(none)"`).

**Pilot:** 50 drawn, of which 26 were new (the rest were labelled in earlier groups, where the substring filters overlap). $0.22, no errors, **0 dictionary gaps**.
- Beneficiary 0.04, counterparty 0.18 per provision; 4 holder-unknown in `hu`.
- About 1,147 remain for the bulk run (~$5).

## Family group 5 bulk and the full sample (2026-10-05)

**Group 5** paused at the gate **three times**, each time on a new domain body:
1. "evaluation body" → Spc: Evaluation Body, plus EU: Committee of the Regions;
2. "recognised body" → Finance: Recognised Body / Clearing Member, plus Aviation: Commander / Crew;
3. "Committee on Climate Change" → Gvt: Agency: Committee on Climate Change, Gvt: Intelligence Service, Gvt: Authority: Safeguarding Children Board, and the school governing body. Jason: governed, with a new domain prefix, so **Education: School Governing Body**.

**End-of-run gaps:** Org: Insurer, Spc: Auditor, Spc: Standardisation Body, Data: Subscriber, Finance: Auction Platform; "designated counterparty" trigger. Final relabel: 65 provisions.

**Full sample, labelled:** 6,959 provisions, 0 errors, **$27.28** ($3.92 per 1,000). Relation yes 4,164 / no 2,795.

| Split | Provisions | Active | Mentioned | Counterparty | Beneficiary | Holder unknown | Inferred |
|---|---|---|---|---|---|---|---|
| train | 6,357 | 3,628 | 3,847 | 1,081 | 598 | 485 | 190 |
| test | 741 | 509 | 407 | 129 | 82 | 66 | 114 |

- The table holds 139 more provisions than the sample: the earlier pilot (206) and probe (39) from the first draw, at the same prompt version and usable as training rows.
- **Beneficiary: 680 in total, short of the 1,000 target.** Genuine protective-purpose duties are rare under the act test. Use class weighting in training.
- **Dictionary over the day:** 132 → ~230 labels, reconciled with legal. Regex clashes fixed: company officer, economic operator, verifier, gas transporter, temporary work agency, prosecutor / Lord Advocate.

**Next:** export the labels into the SLM fine-tune format and retrain on RunPod (Jason launches), then evaluate against the held-out test laws, the 50 hand-checked rows, `holder60_cases` and gold v2.

## Second model (GPT-5.5 low) + Claude referee: batch 1 (2026-10-05)

**Jason:** add the second model and the Claude diff analysis (the gold v2 approach) to the training labels, starting with batch 1 (family group 1, 1,357 provisions).

**Run:** `label_drrp_training.py --model openai` (gpt-5.5:low, `prompt_cache_key` for cache routing). 1,357 labelled, 0 errors, **$22.38** ($16.49 per 1,000; GPT-5.5 is $5 / $0.50 cached / $30 per M).
- GPT is much looser with labels than Gemini (~30 kinds of OTHER, many with existing labels; some non-actors). The gate paused twice on GPT noise, so the rest of the GPT pass ran with `--gate 0`.
- Real gaps added: Spc: Conformity Assessment Personnel, Org: Subsidiary, Org: Subcontractor, Spc: Shotfirer, Ind: Legal Representative; triggers for installation manager, proprietor and partner. `Spc: Notified Body` (a GPT slip) is an alias.

**Diff** (`compare_training_labels.py`): **75.0% agree, 339 disputed (25%).**
- By kind: purpose 200, relation 118, active↔mentioned 83, GPT-only actor 55, counterparty↔mentioned 44, raw_type 37, beneficiary↔mentioned 35, label 30, **counterparty↔beneficiary 6**.
- Agreement by stratum: from 88% (none) down to 60% (app).
- The counterparty/beneficiary split the retrain is for is nearly settled; disputes concentrate on purpose and relation.

**Referee:** brief in `scripts/ml/REFEREE_TRAINING_LABELS.md`; disputes in `data/training/drrp-v1.1/disputes_batch1.jsonl`; decisions go to `referee/batch1.jsonl`. Test batch of 10 running.

## Rulings 1–5 and prompt v1.2 (2026-10-05)

**Referee test batch** (10 v1.1 disputes, kept as `referee/batch1_v1.1_test.jsonl`): Gemini 6, GPT 3, mixed 1.

**Jason agreed rulings 1–5 and option (b):** bump the prompt and refresh by targeted relabel.
1. A passive duty's counterparty gets an `act`.
2. A headless stem ("X shall—") is relation yes with X active.
3. A transitional provision that itself confers a time-limited power or duty is relation yes, purpose `Transitional Arrangement`.
4. A Member State or country that is only a place is not an actor.
5. `Ind: Interested Party` added.

**Commits:** spec `dd0b541`, prompt `drrp-v1.2-2026-10-05` `4e2c491`, carry-forward `25b0ab9`.

**Refresh** (`--carry-from drrp-v1.1-2026-10-05`):
- Gemini: 6,394 carried, 708 relabelled (565 v1.2-affected + dictionary-stale), $3.51.
- GPT batch 1: 1,237 carried, 151 relabelled, $2.83.

**Batch 1 at v1.2: 75.9% agree, 327 disputed.** Purpose 197, relation 114, active↔mentioned 79, GPT-only actor 53, counterparty↔mentioned 46, raw_type 35, beneficiary↔mentioned 33, label 26, counterparty↔beneficiary 8.

**Referee:** running on all 327 in batches of 50 → `referee/batch1.jsonl`.

## Referee: batch 1 (327 disputes, 2026-10-05)

**Output:** `referee/batch1.jsonl`, validated: 327 decisions, ids match, all labels and purposes valid, non-active hold none.

**sided_with:** **Gemini 219, GPT 76**, mixed 13, neither 19.
- Relation disputes: Gemini right 99, GPT 15.
- Purpose disputes: Gemini 163, GPT 31.

**GPT is systematically wrong on:**
- purpose precedence: Requirement/Power Conferred for enforcement, appeals, fees and liability (~60);
- detail provisions as relations (exemption-power conditions, "have regard to", notice/report contents, trigger items);
- applying-provision overreach.

**Gemini is systematically wrong on:**
- beneficiary for people in trigger conditions (→ mentioned; most "neither");
- the party subject to a power as mentioned (should be counterparty);
- `inferred` on applying-provision holders;
- the v1.2 transitional duty;
- the machinery order for disapplication and deeming.

**The referee overrode agreed labels** in a few cases (e.g. UK_uksi_2016_721 reg.26(2), UK_uksi_1989_971 reg.22 items). Consensus isn't always right.

**Rule gaps to rule on:**
- trigger-condition actors;
- commencement power vs consistency;
- whether laying before Parliament/Assembly is "parliamentary procedure";
- enforcing-authority designation and "functions" lists;
- class-definition items outside content lists;
- Exemption vs Transitional precedence;
- money clauses ("paid out of money provided by Parliament");
- two roles for one actor.

**Dictionary gaps:** recognised third party organisation, arbiter, weights and measures authority (trigger), persons in lawful occupation (trigger).

## Rulings round 2 (prompt v1.3), gap fill, and the second model for groups 2–5 (2026-10-05)

**Referee batch 1 rulings → prompt `drrp-v1.3-2026-10-05`** (Jason agreed all):
- trigger-condition actors are `mentioned`;
- commencement powers are a Liberty;
- laying before Parliament/Assembly is a government Obligation;
- enforcing-authority designations and functions lists: relation no, Establishment+Constitution;
- class-definition items: relation no;
- time-limited disapplications: Exemption;
- money provided by Parliament: relation no, Charge+Fee;
- one actor, two roles: strongest role.

**Commits:** spec `c85f577`, prompt `1377a8c`, per-version carry rules `7cdb038`.

**Dictionary gaps filled** (`221c561`): Gvt: Visiting Force, Intl: International Organisation (new `Intl:` prefix: Basel Secretariat, OPCW fact-finding missions), Intl: Foreign State (only when it acts or receives the act), EU: Euratom. Plus Spc: Recognised Third Party Organisation, Spc: Arbitrator, and triggers for weights & measures authority and lawful occupation.

**Why GPT-5.5 is dearer:** $5 / $0.50 / $30 per M against Gemini Flash $0.75 / $0.075 / $3.75. That's ~$16.49 per 1,000 against $3.92.

**GPT-5.4-mini** ($0.75 / $0.075 / $4.50) on the 327 refereed provisions: $0.71 ($2.17 per 1,000). Scored against the referee (`score_vs_referee.py`):

| Model | Exact | Relation | Purpose | Actors |
|---|---|---|---|---|
| Gemini | 71% | 94% | 90% | 79% |
| GPT-5.4-mini | 36% | 82% | 59% | 57% |
| GPT-5.5 (v1.2) | 27% | 69% | 48% | 50% |

Mini invents labels (18 on 327) but is better than GPT-5.5 here at an eighth of the cost.

**Jason: GPT-5.4-mini is the second model for groups 2–5** (5,602 provisions, ~$12, gate off). It's a disagreement detector; the referee decides.

## Batch 1 at v1.3: complete (2026-10-05)

**v1.3 refresh:** Gemini 745 relabelled ($3.19), GPT-5.5 batch 1 168 ($3.27).

**Batch 1 at v1.3:** 78.5% agree, 292 disputed.
- 314 of the 327 referee decisions are still valid under v1.3; 13 were invalidated (money 4, commencement 3, disapplication 3, laying 2, enforcing 1).
- 284 of the disputes are covered by a valid decision.
- The referee decided the other 8 (`referee/batch1_v13.jsonl`): Gemini 4, GPT 4.

**Batch 1 training labels** = model consensus (1,065) + valid referee decisions (314) + the v1.3 referee decisions (8).

**Edge-case rule gaps, to rule on together after groups 2–5:**
- a notification duty inside a commencement provision;
- electronic-delivery deeming vs notice service;
- designation powers inside definitions;
- "means at their disposal" (counterparty/supply?);
- definition of "time-limited" for evidential savings.

## Batch 2 with GPT-5.4-mini (2026-10-06)

**Credit:** the OpenAI account ran out overnight (2026-10-05 ~21:00). The groups 2–5 mini run failed ~2,230 calls on retries; it was stopped, and the labeller now stops at the first billing error (`6c8a60c`).
- Jason: ~£40 went on GPT-5.5 because the total cost wasn't approved up front. He topped up $20 ($16 left) and asked for batch 2 only. Memory: `feedback_paid_api_walk_dont_run`.
- Added `--max-cost` (`e1989f9`).

**Batch 2 (group 2, 1,635 provisions):** smoke test 10 ($0.03), then the rest with a $5 cap. **$3.26 actual** ($2.02 per 1,000); ~$12.70 OpenAI credit left.

**Gemini vs mini: 58.3% agree, 681 disputed** (vs 22–25% with GPT-5.5 on batch 1).
- 137 were only label slips or dropped actors on mini's side: **settled for Gemini** (`referee/batch2_auto.jsonl`, `auto-gemini`).
- 544 substantive disputes: Opus referee, two agents in parallel (`referee/batch2a.jsonl`, `batch2b.jsonl`).

## Referee: batch 2 (544 disputes, Opus, 2026-10-06)

**Output:** validated: both halves 272/272, ids match, all labels/purposes valid.
- Half A: Gemini 221, mini 40, mixed 11.
- Half B: Gemini 177, mini 77, mixed 11, neither 7.
- **Total: Gemini 398, mini 117, mixed 22, neither 7.** Plus 137 `auto-gemini`.

**Batch 2 training labels** = consensus (954) + auto-gemini (137) + referee (544).

**Mini is wrong on:**
- list items under a duty (demotes the stem holder; ~100 in half A);
- own procedural duties labelled Procedure+Detail;
- purpose precedence;
- raw_type set alongside an active holder;
- `inferred` on express or referenced holders;
- invented labels;
- dropped applying-provision holders.

**Gemini is wrong on:**
- beneficiary for trigger-condition and "have regard to" parties;
- beneficiary where employees receive the act (health surveillance etc.);
- detail provisions (notice/report/register contents, timing items) as relations;
- class-definition items as relations;
- missed `answer_request`.

**Rule gaps added to the joint ruling:**
- functions lists of a *governed* party (safety representatives: both referees used Power Conferred);
- content lists vs detail provisions for "the notice/report … must—";
- timing items ("at suitable intervals", "within 8 weeks");
- appeal/inquiry procedure → Defence+Appeal?;
- exemption powers (Power Conferred vs Exemption);
- deeming savings vs Transitional Arrangement;
- "shall not grant … unless" restrictions on another provision's power;
- "comply with his duty by…" discharge details;
- mixed provisions (exemption + fallback duty);
- withheld conduct and counterparty;
- purpose of relation-no class-definition items.

**Consistency note:** Workplace Regs reg.25(2)(a) was ruled detail by the referee, while sibling 25(2)(b) was auto-settled yes. Resolve with the content-list/detail ruling.

**Dictionary gaps:** diving contractor, appointed body/poison centre (CLP Art 45), visitor (Occupiers' Liability), EU designated experts, Independent Anti-slavery Commissioner, regional vs district planning authority, appeal/inquiry inspectors, Central Arbitration Committee, works-council recipient, mines workmen's inspector, appointed doctor vs employment medical adviser.
