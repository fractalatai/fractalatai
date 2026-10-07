---
session: "Phase B: Principles and precedents"
status: closed
opened: 2026-10-06
closed: 2026-10-07
outcome: success

summary: >
  The 123-rule catalogue became catalogue v2: 31 principles, 5 precedent patterns, 12 retired, 8 dictionary decisions
  and a crosswalk for every v1 ID, settled with Jason in plain terms and stress-tested by Gemini. A consistency check
  brought the 220-sentence gold in line (56 changes, 2 exclusions, page synced with 0 mismatches), and the precedent
  store, brief v2 and a trimmed spec make phase C labelling run on principles plus retrieved examples.

decisions:
  - what: "Catalogue v2: 31 principles; the rest precedents, retired, dictionary or merged; v1 kept as history"
    why: "123 rules from row-level labelling did not converge; promotion needs ~3 provisions from 2+ laws and Jason's approval"
    result: "docs/architecture/DRRP-RULE-CATALOGUE.md v2; all 123 v1 IDs resolve through the crosswalk"
  - what: "Own-power test by the modal's named subject (REL-33); agentless manner or content is continues (REL-28); a holder named in a continues sentence is mentioned"
    why: "Gemini found REL-28 vs REL-33 had no tie-break; a grammatical test is learnable by small models (Q1-Q3)"
    result: "11 G2 and 9 G3 gold changes; the hirer reg.13(4) ruling kept"
  - what: "Commencement and parliamentary procedure are machinery (no); amending text excluded from gold"
    why: "Jason B, Q5, Q6 (Claude changed its Q6 answer after Gemini)"
    result: "asp 2019/15 s.32(2) changed; NIA s.40(1) and CAA s.69A(7) excluded"
  - what: "Designating an enforcing authority is that body's Obligation, purpose Requirements (REL-27); a dated regulator transfer is Application, relation no"
    why: "Jason: the HSE is literally required to be this thing; GHG ETS reg.13(2) is only about a date transfer"
    result: "New principle REL-27 reversing v1 REL-27; purpose doc updated"
  - what: "Exemptions and defences stay no; deeming that moves holders is no plus a deemed-holder tag; implied access right also takes a counterparty give_access entry"
    why: "Purpose carries exemptions and defences; the holder-linking step uses deemed-holder; one entry per role (#78)"
    result: "2 deemed-holder tags in gold; precedent patterns updated"
  - what: "Gvt: Authorised Person approved but not yet added"
    why: "598 provisions in 33 Families clear the rule, but electrical/mines/rail use a duty-holder specialist; 543 hub rows, prompt and aliases need migrating"
    result: "Dictionary task on the meta-plan list; the YAML edit was tried and reverted"
  - what: "Precedent retrieval: pattern tag, same section, then embedding similarity"
    why: "tf-idf over 220 short sentences gave unrelated neighbours"
    result: "reg.13(4) retrieves CAA s.56(6), its REL-33/REL-28 contrast pair"
  - what: "LAT pulls held until legal finishes the whole parse repair, then one pull after a NAS backup"
    why: "Jason: each row should change once; the pull also archives ~47K scoped rows"
    result: "Batch 3 (80 laws) not pulled; none of the 60 test laws affected"

metrics:
  catalogue: { v1_rules: 123, principles: 31, merged: 66, retired: 12, dictionary: 8, precedents: 6, crosswalk_ids_resolving: 123 }
  recurrence: { above_threshold: 67, below_threshold: 33, never_cited: 23 }
  consistency_check: { conflicts: 58, sentences: 35, applied_operations: 56, excluded: 2, page_mismatches_after_sync: 0 }
  gold_after: { sentences: 220, rows: 1117, approve: 1080, change: 26, query: 11 }
  precedents: { sentences: 220, with_jason_notes: 41, embedded: 197, principles_exercised: 31 }
  spec: { size_before_kb: 49.5, size_after_kb: 35.5 }

lessons:
  - title: "Explain vocabulary before asking for rulings"
    detail: "Jason couldn't decide A/B/C until 'machinery' was explained and each tension put as one sentence with both readings. His earlier 'rulings' were guesses on snippets; ask with whole sentences and plain words."
    tag: methodology
  - title: "Reusing a rule ID silently rewrites history"
    detail: "Giving the new enforcing-authority principle ID REL-20 made 5 gold rows citing v1 REL-20 (savings) display the wrong rule. When a principle reverses an old rule, take that rule's ID; never recycle an unrelated one. Check old citations resolve after any ID change."
    tag: data
  - title: "A dictionary label change is a cross-system migration"
    detail: "Adding Gvt: Authorised Person looked like a YAML edit, but it touched family semantics (duty-holder specialists in mines/rail), 543 hub rows, the LLM prompt, aliases and the controls generator. Survey every consumer before editing the dictionary."
    tag: architecture
  - title: "Keep the review page and the gold in lockstep after bulk edits"
    detail: "The importer applies every page decision, so editing the gold without rewriting the page docs would let a stale page export undo the change. After any bulk gold edit, rewrite the affected page docs and verify a pull-back with 0 mismatches."
    tag: tooling
  - title: "Use stored embeddings for retrieval over small corpora"
    detail: "Lexical tf-idf on 220 short legal sentences surfaced unrelated precedents; the hub's existing 384-d embeddings (mean of member rows) found the right contrast cases at no extra cost."
    tag: models
  - title: "A second reviewer earns its keep on schema and rules"
    detail: "Gemini rejected the free-text referent key (#78) for position, and found the REL-28/REL-33 tie-break gap and the deeming precedence. Claude changed its Q6 answer. Harsh reviews before adoption were cheap and changed outcomes."
    tag: methodology

artifacts:
  - docs/architecture/DRRP-RULE-CATALOGUE.md
  - docs/architecture/DRRP-RULE-CATALOGUE-V1.md
  - docs/architecture/DRRP-CLASSIFICATION.md
  - docs/architecture/PURPOSE-CLASSIFICATION.md
  - scripts/benchmarks/gold_v3/JUSTIFY_V2.md
  - scripts/benchmarks/gold_v3/precedents.py
  - scripts/benchmarks/gold_v3/apply_consistency.py
  - scripts/benchmarks/gold_v3/unit_evidence.py
  - scripts/benchmarks/gold_v3/load_units.py
  - scripts/benchmarks/gold_v3/review_units_export.py
  - data/gold/v4/rule_classification.csv
  - data/gold/v4/rule_recurrence.csv
  - data/gold/v4/consistency_review.md
  - data/gold/v4/precedents.jsonl
  - data/code-review/drrp-catalogue-v2-principles.md

depends_on:
  - 2026-10-06-phaseA-sentence-units
  - 2026-10-06-meta-plan-sentence-units

enables:
  - "Phase C: gold at sentence level on catalogue v2, brief v2 and precedents"
  - "Phase D: tier scoring with context principles (HOLD-05, INF-03) scored apart"
  - "Dictionary task: Gvt: Authorised Person with Family gating"
---

# Session: Phase B: Principles and precedents (CLOSED)

## Problem

`docs/architecture/DRRP-RULE-CATALOGUE.md` holds 123 active rules (REL 41, HOLD 17, TYPE 7, INF 4, POS 19, ACT 12, LBL 15, DEF 8). 44 were added or amended on 2026-10-06, many from a single provision, and some oscillated. The tiers below the LLM can't carry that many, and a gold set built on them measures case law. The catalogue becomes about 30 to 40 general principles; the rest become precedents.

## Todo

- ✅ (agent draft 2026-10-07: 30 principles, 66 merged, 12 retired, 8 dictionary, 7 precedents; `data/gold/v4/rule_classification.csv`) Classify every rule: **principle** (general, recurs, a model can learn it), **precedent** (one provision or a narrow pattern), **retired by sentence units** (stems, items, continuation: REL-02, REL-13, REL-15, REL-16, REL-30, REL-45, HOLD-02, TYPE-06, POS-17, POS-18, INF-04's stem clause, the item half of REL-28), or **merge** (overlapping rules)
- ✅ (`data/gold/v4/rule_recurrence.csv`; used in the classification) Recurrence check: for each candidate principle, count the provisions it decides in the 240 reviewed and the silver labels; under about 3 provisions, or all from one law → precedent
- ✅ (adopted 2026-10-07: `docs/architecture/DRRP-RULE-CATALOGUE.md` is v2, 31 principles, with a crosswalk for all 123 v1 IDs; v1 kept as `DRRP-RULE-CATALOGUE-V1.md`) Draft catalogue v2 (principles only, one line plus one example each), for Jason's review
- ✅ (`scripts/benchmarks/gold_v3/precedents.py` → `data/gold/v4/precedents.jsonl`: 220 sentences, 41 with Jason's notes, all 31 principles exercised) Precedent store: precedents live as reviewed rows in `drrp_gold` (section_id, decision, comment), tagged with a short pattern name, and are shown to the justifier as examples, not cited as rules
- ✅ (JUSTIFY_V2.md "Notes: report, don't legislate"; catalogue v2 header) Promotion rule written into the brief: a new pattern becomes a principle only after about 3 provisions from at least 2 laws, **and Jason approves the promotion**; the justifier reports candidate patterns, it doesn't make rules
- ✅ (pattern tag, then same section, then embedding similarity on mean member-row embeddings; tf-idf fallback; wired into unit_evidence.py) Precedent retrieval (Gemini): the justifier finds precedents by pattern tag, then same law and section, then text similarity; shown with Jason's comment
- ✅ (agent trim reviewed; 1 question left for Jason, the payload question moved to legal) Spec (`DRRP-CLASSIFICATION.md`) trimmed to match: special-case rows that are precedents move to an appendix or the precedent store
- ✅ (`scripts/benchmarks/gold_v3/JUSTIFY_V2.md`; old briefs marked superseded) The justifier brief rewritten for sentence units and principles (input to phase C)

- ✅ (58 conflicts in 35 sentences; Jason accepted G1–G6, G8, G9; G7 kept; G10 deferred; applied and synced to the page) **Consistency check:** run v2 against the 222 gold sentences. List the rows v2 would decide differently (REL-20 enforcing authority, REL-28/REL-33 tie-break, REL-07 commencement and parliamentary procedure, a holder in a `continues` sentence as `mentioned`) for Jason
- ✅ (NIA 2013/10 s.40(1) and CAA 1982 s.69A(7) excluded; the selection's remaining amending text to be filtered in phase C) **Exclude amending text from gold** (Q5): find amending-text sentences in gold and the selection and add them to `excluded_units.csv`
- ✅ (Jason: date transfer only → Application, relation `no`) **Enforcing authority and purpose:** the dated-regulator case (GHG ETS reg.13(2), Application) may be a REL-20 designation (Requirements). Ask Jason with the sentence

## Dependencies

- ✅ Catalogue, spec and 240 reviewed provisions with Jason's comments
- ✅ Phase A closed 2026-10-07: sentence units, the #78 schema (POS-15 revised), the dictionary enrichment rule

## Started 2026-10-07: what phase B changes in the gold and the review app

**Jason asked:** do the rules change the data in the sentence review app (hard/easy/edge, policy notes)?

- **Gold values: no.** All 1,097 sentence-gold rows are decided: 1,000 easy, 84 hard and 2 new_edge approved, 11 queried. Those values are Jason's labels, not the rules'.
  - **Exception:** if a v2 principle contradicts an earlier ruling, a **consistency check** flags the gold rows it would decide differently, for Jason to look at. Nothing is changed silently.
- **Rule citations (`rule_ids`, `reason`): kept as history.** `catalogue_ver` already records which catalogue they cite.
  - 53 rows cite rules that sentence units retire: REL-45 26, POS-17 12, REL-16 7, REL-15 6, POS-18 1, REL-30 1.
  - 502 carried rows cite `CARRIED`.
  - A crosswalk (old ID → principle, precedent, retired or merged) lets the page show what an old citation became.
- **Difficulty: redefined against v2 for future loads only.**
  - **easy:** a principle decides it;
  - **hard:** a principle applies, but it needs judgment or a precedent;
  - **new_edge:** nothing fits, so it's a candidate pattern.
  - On existing rows difficulty was review triage, and they are all decided.
- **Policy notes: historical.** The four provisional defaults they tested are settled and become principles in v2. From phase C, notes become **candidate-pattern reports** that feed the promotion rule.

## Recurrence evidence (2026-10-07)

`data/gold/v4/rule_recurrence.csv` lists, per rule, the reviewed provisions citing it (v3 decided rows plus sentence gold, Companies Act excluded), with laws, Families, and changed and queried rows.

| | Rules |
|---|---|
| ≥3 provisions from ≥2 laws | 67 |
| cited, below threshold | 33 |
| never cited | 23 |

Citations are an imperfect proxy: a rule can decide a case without being cited, and a citation can be decorative. The classification reads each rule's text as well as its counts.

## Classification draft (2026-10-07)

An Opus agent sorted all 123 rules using the catalogue, the spec, the purpose scheme, the recurrence table, Jason's 47 comments (`data/gold/v4/jason_comments.json`) and the phase A decisions. Checked: all 123 present, one category each, every merge targets a principle.

**Result:** 30 principles, 66 merged into them, 12 retired, 8 dictionary, 7 precedents.
- Two principles sit below the recurrence threshold by design, as schema invariants: TYPE-03 (only active actors hold) and ACT-01 (act only for an Obligation's counterparty).
- Ten merged rules clear the threshold and could be split back out: REL-17, REL-20, REL-38, POS-04, POS-11, ACT-03, ACT-04, ACT-11, LBL-13, HOLD-06.

**Consequential calls:**
- REL-28 is the single `continues` principle at sentence level, and REL-10 is retired (its `no` outcome was overturned on 10-06).
- "Own duty or power wins" splits into REL-33 (detail-looking sentences, including REL-46's permitted discharge) and REL-41 (machinery-looking sentences, including laying duties and transitional powers).
- **To precedents:** HOLD-04 and HOLD-12 (both reversed on CC(S)A s.31(2)), INF-01 (0 gold provisions), REL-44 (2), HOLD-17 (2 provisions, 1 law).
- POS-19 is the single beneficiary test, absorbing POS-06, POS-07 and POS-11. The eight act-class rules fold into ACT-12.

**7 questions for Jason** are at the end of the draft.

## Gemini review feedback (2026-10-07): catalogue v2 draft and the 7 questions

Gemini 2.5 Pro, harsh review of the draft plus Claude's recommended answers. Raw: `data/code-review/drrp-catalogue-v2-principles.md`.

**Valid, to fold into v2:**
- **Tie-break for REL-28 vs REL-33** (manner of discharge vs own power). Make the test syntactic: the modal's **subject** is a named party → that party's own relation (`yes`; Jason's hirer ruling on reg.13(4)). A passive or agentless manner sentence ("may be given … by leaving it at his address", "may be served by post") → `continues`. Learnable from the dependency parse (nsubj of the modal vs passive). This refines Claude's Q3 answer, which Gemini read as making "served by post" a Liberty.
- **Precedence: REL-06 (deeming) beats REL-42 (passive shall).** The lexical cues are "shall be treated as", "shall be taken to", "deemed". "shall be ventilated" stays REL-42.
- **Context principles are not sentence-model principles.** HOLD-05 (holder from an applying provision) and INF-03 (holder from the parent Act) need multi-hop context. Gold keeps them; the cascade decides them in a linking step (cross-instrument holders, #77), and phase D scores them separately from the sentence tiers.
- **POS-19: drop "the same kind of duty gets the same position".** That's reviewer guidance, not a cue in the sentence.
- **Q6 parliamentary procedure → `no`** (Gemini disagrees with Claude's `continues`). It is a condition precedent on making the instrument, which is machinery, with lexical cues ("laid before", "resolution of each House"). The purpose ruling (Requirements) stays separate. Claude now recommends `no`.

**For Jason (challenges to earlier rulings, not adopted by Claude):**
- **Exemptions, defences, enforcing-authority designations are `no`** (REL-08, REL-04, REL-20 in REL-07). Gemini says the register loses who is exempt, which defences exist, and who enforces.
  - Claude's view: purpose carries the first two ("Application, exemption and transition", "Offences and penalties"), and the actors are `mentioned`, so they're findable.
  - An enforcing-authority designation ("HSE shall be the enforcing authority for…") arguably gives HSE functions; Jason's call.
- **Commencement by order** ("The Secretary of State shall by order appoint a day…"): REL-07 (machinery) vs REL-41 (own duty beats machinery) conflict. A tie-break is needed. Claude leans to machinery: commencement doesn't bind anyone downstream.
- **Deeming that changes who holds a duty** ("a person who does X shall be treated as the employer"): it's `no` under REL-06, but it decides holders elsewhere. A precedent pattern, or a link like HOLD-05?

**Noted, out of scope for the label schema:**
- SFAIRP / reasonably practicable qualifiers: the duty is still `yes` (REL-33 absorbs REL-43). A qualifier attribute could come later.
- Appeals ("a person aggrieved may appeal") are already Liberty under HOLD-01/TYPE-02; add one as a v2 example.

**Agreed by Gemini:** Q1 (retire HOLD-04/HOLD-12 into REL-28), Q2 (a holder in a `continues` sentence is `mentioned`), Q4 (rare rules stay precedents until the top-up), Q5 (exclude amending text), Q7 (add `Gvt: Authorised Person`).

## Decisions on catalogue v2 (Jason, 2026-10-07)

Jason: *"my so called rulings are drafts based on my best guess looking at snippets of law (not sentences)"*. Earlier rulings are evidence, not law. Explained in plain terms: "machinery" means sentences that make the law work as a law rather than tell anyone to do anything.

- **Q1** Retire HOLD-04 and HOLD-12 into REL-28; a content sentence that names no holder is `continues`.
- **Q2** A holder named in a `continues` sentence is `mentioned`.
- **Q3** Own-power test: the modal's **subject** is a named party → own relation (the hirer may inform…). Passive or agentless manner ("may be served by post") → `continues`.
- **Q4** Rare general rules (INF-01, REL-44, HOLD-07 to HOLD-09) stay precedents until the phase C top-up; target the top-up at them.
- **Q5** Exclude amending text from gold.
- **Q6** Parliamentary procedure → `no` (machinery). Claude changed its recommendation after Gemini; the purpose ruling (Requirements) stands separately.
- **Q7** Add `Gvt: Authorised Person`. **Dictionary task pending:** the 10-05 reconciliation found duty-holder specialists in electrical, mines and rail law (`Spc: Authorised Person`). The new label needs Family gating, plus migration of 543 hub rows, the LLM prompt rule (`drrp_prompt.py`), `actor_aliases.py` and the controls generator. A dictionary edit was tried and reverted.
- **A** Exemptions and defences stay `no`; purpose carries them and the actors are `mentioned`.
  - **Enforcing-authority designation is `yes`:** the designated body is `active`, Obligation, purpose **Requirements**. *"'HSE shall be the enforcing authority' is an Obligation that later clauses expand. It's not 'Application' Purpose. It's a Requirement - the HSE is literally required to be this thing and not eg a local authority."* New principle REL-20.
- **B** Commencement by order → `no` (machinery).
- **C** A deeming sentence that changes who holds duties → `no`, tagged as the `deemed-holder` precedent; the holder-linking step uses it later.

**Also adopted from Gemini:**
- REL-06 beats REL-42 (deeming cues);
- HOLD-05 and INF-03 are context principles, scored apart from the sentence tiers;
- POS-19 drops its consistency clause.

**Written:**
- `docs/architecture/DRRP-RULE-CATALOGUE.md` (v2: 31 principles, 5 precedent patterns, retired, dictionary, crosswalk); `rule_texts()` resolves all 123 v1 IDs;
- `PURPOSE-CLASSIFICATION.md` (designation → Requirements; the dated-regulator case flagged open).

**LAT pulls on hold (Jason, 2026-10-07):** wait for legal to finish the whole parse repair (batches 4–11 from its local legislation.gov.uk copy, plus the #174 definitions/BlockText fix), **even if legal says a phase is complete**. Then do one pull, after a NAS backup, because the pull also archives about 47K rows from the 14 scoped Acts. A dry run on 10-07 showed batch 3 (80 laws, 1,579 rows changed) touches none of the 60 test laws.

## Consistency check (2026-10-07)

Two Opus agents read the 222 gold sentences against v2 (`data/gold/v4/consistency/out1.jsonl`, `out2.jsonl`; merged with groups in `conflicts.json`). The automated invariants (TYPE-03, TYPE-04, ACT-01, label repeats) were mostly clean.

**58 possible conflicts across 35 sentences** (27 high confidence, 31 medium), grouped for Jason in `data/gold/v4/consistency_review.md`:

| Group | Changes | What |
|---|---|---|
| G1 | 16 | #78 restorations: act lists (notify + supply), `holds: both` (NRBW reg.53(5), PUWER reg.18(10)), second entries for recipients; ACT-01 fixes |
| G2 | 11 | named subject → own duty (RIDDOR reg.20(2), CTSA s.38(6), SSI 2018/219 reg.27(6), EAW reg.30(2), gas regs reg.40(2)) |
| G3 | 9 | agentless manner or content → `continues` (Directive 89/391 Art.14(3), CAA s.56(6), s.83(6), SI 2005/1726 reg.5(2)) |
| G4 | 11 | named holder in a `continues` sentence → add as `mentioned` |
| G5 | 2 | commencement → `no` (asp 2019/15 s.32(2)) |
| G6 | 2 | amending text → exclude (NIA s.40(1); CAA s.69A(7) modifications, Jason's call) |
| G7 | 3 | GHG ETS reg.13(2): designation or transition (the open purpose question) |
| G8 | 2 | `deemed-holder` tags (AWR reg.24(3), WSI 2005/1806 reg.43(9)) |
| G9 | 1 | duplicate label (`OTHER: well-operator` next to `Operator`) |
| G10 | 1 | → `Gvt: Authorised Person` (deferred to the dictionary task) |

**Agent notes:**
- 10 actor rows with a null value were treated as removed actors.
- Definitions of holder terms are not deeming.
- HW Wales reg.72(2) cites v1 REL-20; the value is unaffected, but under v2 the citation is stale.

**Jason's answers (2026-10-07):** accept G1–G5, G8, G9; G6 both. G7: *"tough to call but its only about date transfer so the purpose is Application … Therefore, relation = no"*. Gold unchanged; the purpose doc's open note is resolved. G10 stays deferred to the Authorised Person dictionary task.

**Applied** (`scripts/benchmarks/gold_v3/apply_consistency.py`; backup at `data/gold/v4/backup_v4_pre_consistency_20261007.copy`):
- 56 operations: changes commented "v2 consistency (Jason 2026-10-07, G…)", new actor rows and second entries (rule `CONSISTENCY`), 2 removed duplicates/holders as `listed: false`, 2 `deemed-holder` tags;
- informal labels mapped to dictionary labels: HSE, `Aviation: Crew`, and `OTHER: Partner of a support panel (CTSA 2015 s.38)`;
- 2 amending-text units added to `excluded_units.csv` and their rows deleted.

**Gold now:** 220 sentences, 1,117 rows, all decided (approve 1,080, change 26, query 11).

**Review page synced:** the 31 changed sentences were rewritten (provisions + decisions) and the 2 excluded removed. A pull-back compared against gold: 1,117 page rows = 1,117 gold rows, **0 mismatches**, 0 unknown keys, so a stale page import can't overwrite these changes.

## Precedent store and brief v2 (2026-10-07)

**Precedent store** (`precedents.py`, `dc2c41c`):
- Every gold sentence is a precedent: final labels, Jason's notes and changes, the v2 principles it exercises (v1 citations mapped through the catalogue crosswalk; carried sentences take their member rows' row-level citations; consistency changes add the principle they cite), and pattern tags (cue regexes for the 5 v2 patterns, or a tag in rule_ids).
- 220 precedents, 41 with Jason's notes, 16 changed. Patterns found: 2 functions-list, 2 no-person-shall-be-engaged, 2 deemed-holder, 1 participation-right, 1 implied-access-right. All 31 principles are exercised; the thinnest are TYPE-03 (1) and REL-20 (3).
- **Retrieval:** pattern tag (+3), then same section (+1), then similarity. Law and Jason's notes only break near-ties.
- Lexical tf-idf on 220 short sentences was weak (the hirer's reg.13(4) pulled unrelated sentences). The hub's stored embeddings fixed it: the cosine of mean member-row embeddings (197 of 220 embedded), with tf-idf as the fallback.
  - reg.13(4) → CAA s.56(6) "service … may be effected by sending" (the REL-33/REL-28 contrast pair);
  - RIDDOR reg.11(1) → PUWER reg.34(1) (the other notify + supply case).
- `unit_evidence.py` adds the 5 nearest precedents to every evidence pack.

**Brief v2** (`JUSTIFY_V2.md`, `168eccd`) replaces `JUSTIFY_GOLD.md` and `JUSTIFY_UNITS.md`:
- binding order: catalogue v2, then purpose, then dictionary; cite v2 IDs only;
- the key tests in short form;
- precedents as examples ("as precedent X"; only patterns may be cited, as `PREC:<pattern>`);
- difficulty redefined;
- notes by kind (candidate_pattern, dictionary_candidate, source_fault, question), with the promotion and dictionary rules;
- exclusions (`{"unit_id", "exclude"}`).

`load_units.py` reads v2 notes and lists proposed exclusions without loading them; the export labels `PREC:` citations.

**ID fix (2026-10-07):** the new enforcing-authority principle first took ID REL-20, but v1 REL-20 was *savings and continuity* (5 gold rows cite it in that sense), and v1 REL-27 was the designation rule it reverses. The principle is now **REL-27**, and REL-20 is back in the crosswalk as merged into REL-07. Earlier mentions of "REL-20" in this doc for designation mean REL-27. Also removed "powers to commence" from REL-41 (decision B: commencement is machinery).

## Spec trimmed (2026-10-07)

An agent trimmed `DRRP-CLASSIFICATION.md` to catalogue v2, from 49.5 KB to 35.5 KB:
- a sentence-unit note and the #78 schema section;
- layers cut to principle pointers;
- special cases split into pipeline/scope, v2-covered, and "Retired with sentence units";
- a "Precedents (examples, not rules)" appendix; all 8 of Jason's 10-07 decisions applied.

Claude reviewed it and resolved the open markers:
- HOLD-07 to HOLD-09 are forms of HOLD-05, kept as its examples (Q4);
- REL-41 drops "powers to commence";
- the payload schema for #78 goes to legal (meta-plan legal-side list).

**One left for Jason:** under #67, does a party with an implied access right (active, inferred Liberty) also take a counterparty entry with `give_access` on the government's duty, now that POS-15 allows one entry per role?

The trim also caught the REL-20/REL-27 ID collision (fixed above).
