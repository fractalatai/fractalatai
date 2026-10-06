---
session: "Phase 0a: Gold set scaffolding (1,000 provisions, rule-justified)"
status: active
opened: 2026-10-06
closed:
outcome:
related: ["parsing/2026-10-06-pipeline-from-the-regex-end.md", "parsing/2026-10-06-phase0b-purpose-scheme-review.md", "benchmarks/2026-09-30-gold-v2.md", 74]
---

# Session: Phase 0a: Gold set scaffolding (ACTIVE)

## Problem

Every tier will be measured against a yardstick. LLM labels can't be that yardstick (circularity: Gemini review). **Jason (2026-10-06):** a human-approved gold set of **1,000** provisions.

He isn't an expert in assigning the hard provisions, and we've built a complex flat set of rules. So:
- each gold label carries an **argument/justification**: a citation of one of **our rules** (a pre-defined set) plus a one-line reason;
- most rows fall out easily, leaving a core of truly hard ones, which may surface edge cases no rule covers yet;
- **Jason reviews a readable table of evidence and approves or not.** He shouldn't have to match raw provision text to rules;
- **all classification schemes** are covered.

Getting the scaffolding right makes the final task easier.

## Todo

- ✅ (Jason, 2026-10-06: the core now; POPIMAR, significance and fitness later as modules) Which schemes go into the gold set (inventory below; recommendation: the core now, the others as later modules on the same scaffolding)
- ✅ (`docs/architecture/DRRP-RULE-CATALOGUE.md`, edbb86b: 114 rules, conflicts C1–C13, gaps G1–G3; ⬜ Jason's rulings on C2, C4, C7, C9, C10, G1–G3) **Rule catalogue:** every rule with a stable ID, grouped by scheme, from DRRP-CLASSIFICATION.md (layers 1–5 and special cases), PURPOSE-CLASSIFICATION.md and `drrp_prompt.py`. One line each plus an example. Rules that only exist in prompt wording become catalogue entries. Purpose rules wait for phase 0b
- ✅ (`drrp_gold` in `scripts/pg_schema.sql`, created; loader `scripts/benchmarks/gold_v3/load.py`) **Gold record schema** (below): per provision, per scheme label, rule IDs, reason, evidence, difficulty, Jason's decision
- ✅ (`scripts/benchmarks/gold_v3/select_gold.py` → `data/gold/v3/selection.csv`; below) **Selection of 1,000:** only from laws held out of SLM training (the 61 test-split laws, 741 already labelled, plus further held-out laws). Stratified across schemes, with rare classes over-sampled (Liberty, beneficiary, applying-provision holders, passive duties, counterparty acts, each purpose)
- ⬜ (brief `scripts/benchmarks/gold_v3/JUSTIFY_GOLD.md`; evidence pack `evidence.py`; pilot of 50 done, loaded; below) **Auto-justification:** a Claude agent (referee-style, no paid API) assigns each label its rule IDs and reason, using the catalogue and the tier evidence. It grades difficulty: **easy** (tiers agree and a rule clearly applies), **hard** (tiers disagree or rules conflict), **new edge** (no rule fits, so a candidate rule)
- ✅ (https://claude.ai/artifact/PkoqADCn2TXvSAEkPaTE6w; `review_page.html`, `review_export.py`, `review_import.py`) **Review table** for Jason: **an interactive page** (Jason, 2026-10-06, "if easy enough to spin up"): easy rows first in bulk, then the hard core
- ⬜ **Jason's review;** hard and new-edge rows → rulings → catalogue updates → re-justify the affected rows
- ⬜ The gold set is frozen (versioned) and becomes the yardstick for phase 1 and the release QA; it replaces the gold v2 plan's role

## Build (2026-10-06)

**Rulings that shape the gold set** (Jason, 2026-10-06):
- the spec, not the v1.3 prompt, is the gold standard: functions lists, applying-provision pointer, `serve` → `notify`, #67 without a named holder, parent-Act holders;
- **one entry per label stays** (POS-15). The second role is dropped and noted in the reason; #78 tracks the loss;
- purpose is labelled with the **12 coarse classes** (Duties/Powers renamed Requirements/Permissions);
- **purpose detail is derived, not reviewed** (Jason, 2026-10-06). Substantive vs Procedure/Detail = Requirements + "creates an obligation or liberty?" yes/no. In the pilot the two matched on all 24 rows. It's hidden from the page and dropped from the justifier brief.

**Pool:** `sample_drrp_training.py --pool test` writes every provision of the 61 test-split laws: 11,733 provisions, no training overlap. Same universe as the sample, so no Schedules (a known limit). The #77 finder found **0** cross-instrument candidates in these laws, so that pass isn't a prerequisite here.

**Selection** (seed 63): 1,000 provisions in all 61 laws.

| Part | Rows | What it is |
|---|---|---|
| labelled | 726 | already labelled (Gemini v1.3 on all; mini 258, GPT-5.5 134, referee 129) |
| natural | 150 | random from the rest: the corpus's own mix, for unbiased rates |
| target | 124 | rare cases, from unlabelled rows |

The target rows: Liberty 45, Enforcement 12, Appeals 10, Fees 10, Transitional 10, Constitution 9, Amendment 8, Citation 8, Interpretation 6, applying holders 4, functions lists 2. Applying holders and functions lists are scarce in these laws.

Flags across the 1,000:

| Flag | Rows |
|---|---|
| Liberty | 164 |
| beneficiary cue | 182 |
| counterparty | 263 |
| holder unknown | 188 |
| applying | 33 |

**Coarse purpose cue with stem inheritance** (`coarse_purpose.py`, on the 6,959 labels): undetermined drops from 2,317 to **624**. Duties: 82% precision, 86% recall. Enforcement (42%) and Constitution (11%) cues still need work (phase 2).

**Evidence pack** (`evidence.py` → `data/gold/v3/evidence.jsonl`), per provision:
- the law title, Part/Chapter titles and cross-heading;
- the exact model context (stem, referenced and applying provisions);
- the pipeline tiers per actor;
- each model's v1.3 label, the referee decision and the coarse cue.

**Gold rows** (`drrp_gold`): one row per (provision, field[, actor]), holding:
- proposed value, catalogue rule IDs and a one-line reason;
- evidence (models, referee, pipeline, cue);
- `agree` (models/referee only) and difficulty;
- Jason's decision (approve / change / query).

Actors that a model listed but the justifier left out get their own row for review. Decided rows are never overwritten.

## Pilot: 50 provisions justified (2026-10-06)

Opus justifier, brief `JUSTIFY_GOLD.md`, no paid API. Output `data/gold/v3/justified/pilot.jsonl`. **271 gold rows**, including 8 actors that a model listed and the justifier dropped.

| Field | Easy | Hard | New edge |
|---|---|---|---|
| relation | 37 | 13 | 0 |
| raw_type | 44 | 6 | 0 |
| purpose | 33 | 17 | 0 |
| purpose_fine | 14 | 10 | 0 |
| actor | 68 | 27 | 2 |

**Agreement** with Gemini and the referee: 162 agree, 14 disagree, 95 have no label. Every easy row agrees where labels exist.
- Agreement counts Gemini and the referee only. GPT-mini and GPT-5.5 are too noisy (mini put `act: other` on active actors), so they're shown but not counted.

**Edge candidates:**
- One active actor holding a Liberty and an Obligation in one provision (G1/#78; TCP inquiry rules reg.18(10));
- `OTHER: Registry administrator`, a dictionary gap (the government registry administrator; `Spc: Administrator` is governed).

**Where the catalogue was hard to apply** (to rule on with Jason):
- list items, completing vs criterion (REL-31/32 vs REL-15; 6 rows), incl. "unless (a) it has been thoroughly examined" conditions under a prohibition;
- REL-16 making a substantive content list relation `no` ("The measures required by paragraph (1) shall consist of—", PUWER reg.11(2));
- transitional power vs time-limited disapplication (REL-38 vs REL-08);
- a saved duty (REL-20 vs REL-38);
- Powers vs Enforcement when a regulator has a notice or information power;
- conditions on a power mapping to Duties;
- two acts in one duty ("allow time off and provide means");
- listing stem actors on relation-`no` items.

**Evidence gaps the pilot found:**
- defined terms ("the Authority", "the Agency") aren't in the context;
- some referenced provisions are missing, and some applying/referenced provisions are wrong (Energy Information reg.4(2) is an exemption);
- garbled formula text (UK_uksi_2020_1265 reg.13(2));
- an empty law title (UK_ukpga_2015_6);
- cue false positives: "revoked", the "FINAL PROVISIONS" heading, "shall consist of".

## Option A re-justification (2026-10-06)

Jason ruled that stems and list items are counted once (REL-45/REL-28/POS-17). The 22 list items in the pilot were re-justified (`justified/pilot_optionA.jsonl`):
- **11 `continues`.** The stem holder is left off the item; the item's own actors keep their role.
- **11 `no`.** 4 are criterion items. In the other 7 the stem itself begins no duty or power (exemption, offence, saving, detail).

Jason's 5 approvals on s.16(5)(b) predated the ruling, so they were cleared and are to be re-reviewed. Backup: `data/gold/v3/decisions_backup/`.

Open rule questions from the re-justification:
- where REL-15 (class definition) stops and POS-17 (item names the party) starts;
- "unless" items under a prohibition;
- the stem's holder named again in the item's own text;
- "X required by para (1) shall consist of—" lists (REL-16 detail, or a stem?);
- the same label as the stem holder (e.g. Gvt: Minister).

## Pilot review (2026-10-06)

Jason reviewed the pilot: **215 approved, 12 changed**, 15 left for discussion.

Rulings from the discussion, now in the spec, catalogue and brief:
- detail of a duty continues it (REL-28 extended);
- the engager holds "no person shall be engaged" (HOLD-17);
- participation is a Liberty (TYPE-07);
- conditions on an exemption are Application and exemption;
- purpose is single-select.

Decisions agreed in chat were recorded on the page with a comment (reg.8(10), Art.11(2)(e), EAW reg.16 inferred). The 3 detail provisions were re-justified to `continues`. Jason's earlier approval of reg.34N(3) relation `no` was cleared for re-review. SI 2000/1043 reg.11(2) was added to #78.

Open:
- Transitional merged into Application (11 coarse classes, with a Scope/Exemption/Transitional detail level)?
- the #67 public on a `continues` row: counterparty `give_access`, beneficiary, or mentioned?
- conditions on a power: should they follow the detail rule?

## Final pilot rulings and the batch plan (2026-10-06)

**Rulings:**
- Transitional merged into **Application, exemption and transition** (11 coarse classes);
- the #67 party on a `continues` row of an access duty is counterparty `give_access` (POS-18);
- conditions on a power continue it (REL-14 → `continues`).

Two pilot provisions were re-justified (gas regs reg.40(2), GHG ETS reg.44A(8)(a)), and the decisions their new proposals supersede were cleared. **Pilot state:** 226 approved, 11 changed, 5 open (on those two provisions).

**Context fixes before the batches** (`evidence.py`):
- the law's own definitions of terms used (566/1,000 provisions get some; a term defined only in parent legislation, e.g. "the Agency" in SI 2005/1806, still isn't found);
- references made in the stem (PUWER reg.11(2)(c) now sees reg.11(1));
- tighter "revoked" and "shall consist of" cues.

The selection is frozen.

**Adopted (Jason, 2026-10-06): the beneficiary test (POS-19)**, now in the spec, catalogue and brief. reg.40(2) Person was changed to beneficiary on the page and in Postgres. The test: "Health and safety **of** X", "protect X", "the interests of X" → beneficiary, whether in a duty or a condition; "where/if/having regard to X" → mentioned. It would flip the persons in reg.40(2) to beneficiary.

**Batches:** the 950 are split into 10 mixed batches of 95 (`data/gold/v3/batches/batch01..10.jsonl`, seed 64). Each has about 70 labelled, 14 natural and 11 target. **One batch at a time** (Jason), because each can change the rules for the next. Jason keeps reviewing the easy rows until he's confident in them.

Per batch:
1. Pull decisions from the page (ArtifactData list `decisions`, out_dir), then `review_import.py --write`.
2. Fold any new rulings into the spec, catalogue and `JUSTIFY_GOLD.md`.
3. Opus agent: brief + `batches/batchNN.jsonl` → `justified/batchNN.jsonl`.
4. `load.py justified/batchNN.jsonl --write`.
5. `review_export.py` (no `--only` exports everything in drrp_gold).
6. Write the new provision docs to the page. Existing docs need `if_version`, tracked in `data/gold/v3/review_page_versions.json` (update it after each write).
7. Report the counts, new edges and rule questions to Jason.

Page: https://claude.ai/artifact/PkoqADCn2TXvSAEkPaTE6w

## Classification scheme inventory (2026-10-06)

**Per actor on a provision:**
- label (~245 dictionary labels);
- position (`active` / `counterparty` / `beneficiary` / `mentioned`);
- holds (`Obligation` / `Liberty` / `none`);
- `inferred` (#67 access rights, #60 applying holders);
- act (9 values, #75);
- holder class (government/governed, from the dictionary);
- correlatives (derived, so not labelled).

**Per provision:**
- relation (yes/no) and raw_type (holder unknown);
- DRRP types (derived);
- **purpose** (18 published; phase 0b may change it);
- scope (substantive / structural / amendment / out);
- duty_family (4) / duty_sub_type (21) (regex);
- **POPIMAR** (16, multi-label, regex);
- **significance** (gravity, strength, scope duty-bearer, protected class: H/M/L, SLM; hierarchy and overall derived);
- **fitness/applicability** (polarity AppliesTo/DisappliesTo/ExtendsTo; person/process/place/plant/property/sector; scope dimensions);
- extent (source data);
- clause modal/qualifier (internal).

**Per law (all derived):**
- verdict (making / empowering / no_obligations / holder unknown), current_*;
- holder lists, correlative holder lists;
- purpose profile, significance rating, compiled applicability.

Also: legal's own law-level purpose (15 values, from the law title), plus triage, application regions and domain/family classification.

**Recommendation for gold scope:**
1. **Core now:** relation, raw_type, purpose, and per actor label / position / holds / inferred / act. These are labelled per provision and feed the verdict, holder lists and correlatives.
2. **Later modules, same scaffolding:** POPIMAR, significance, fitness. They're separate pipelines with their own consumers, and each would need its own rule catalogue.
3. **Out of scope:** derived schemes (DRRP types, correlatives, law-level), which are checked by computing them from gold. Also source data (extent) and internal ones (clause structure).

## Gold record (draft)

One row per (provision, scheme field), so the table reads naturally:

| Field | Content |
|---|---|
| provision | section_id, law title, **provision title / cross-heading** (phase 0b), short text excerpt; the full text and stem one click away |
| scheme / field | e.g. `relation`, `purpose`, `actor: Org: Employer → position` |
| proposed value | e.g. `yes`, `Requirement`, `active` |
| rule(s) | catalogue IDs + one-line rule text, e.g. `D-07 Passive duty: holder from applying provision` |
| reason | one sentence linking the rule to this text |
| evidence | regex / classifier / SLM / Gemini / 2nd model / referee outputs; agree ✓ or ✗ |
| difficulty | easy / hard / new edge |
| Jason | approve / change to … / comment |

## Review table: options

- **(a) An interactive page** (a claude.ai artifact). Filter by difficulty, scheme and law; one-click approve; comments; decisions saved in the page's database, so Claude reads them back. Easy rows can be approved in bulk.
- **(b) A spreadsheet** (xlsx/CSV), one sheet per difficulty. Simple, but approvals come back by file.

## Dependencies

- Phase 0b: the purpose vocabulary decision. **Gold purpose labels wait for the purpose refactor** (Jason, 2026-10-06)
- Section titles (P1group/Title) would make provision titles show in the review table; cross-headings and Part titles are available now
- Labels and evidence already in hand: Gemini v1.3 on 6,959 (incl. 741 test-split), refereed batches 1–2, `holder60_cases`, the 50 hand-checked rows
