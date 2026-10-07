---
session: "Phase B: Principles and precedents"
status: active
opened: 2026-10-06
related: ["parsing/2026-10-06-meta-plan-sentence-units.md", "docs/architecture/DRRP-RULE-CATALOGUE.md", "docs/architecture/DRRP-CLASSIFICATION.md"]
---

# Session: Phase B: Principles and precedents (ACTIVE)

## Problem

`docs/architecture/DRRP-RULE-CATALOGUE.md` holds 123 active rules (REL 41, HOLD 17, TYPE 7, INF 4, POS 19, ACT 12, LBL 15, DEF 8). 44 were added or amended on 2026-10-06, many from a single provision, and some oscillated. The tiers below the LLM can't carry that many, and a gold set built on them measures case law. The catalogue becomes about 30 to 40 general principles; the rest become precedents.

## Todo

- ✅ (agent draft 2026-10-07: 30 principles, 66 merged, 12 retired, 8 dictionary, 7 precedents; `data/gold/v4/rule_classification.csv`) Classify every rule: **principle** (general, recurs, a model can learn it), **precedent** (one provision or a narrow pattern), **retired by sentence units** (stems, items, continuation: REL-02, REL-13, REL-15, REL-16, REL-30, REL-45, HOLD-02, TYPE-06, POS-17, POS-18, INF-04's stem clause, the item half of REL-28), or **merge** (overlapping rules)
- ✅ (`data/gold/v4/rule_recurrence.csv`; used in the classification) Recurrence check: for each candidate principle, count the provisions it decides in the 240 reviewed and the silver labels; under about 3 provisions, or all from one law → precedent
- ⬜ (draft at `data/gold/v4/catalogue_v2_draft.md`; with Jason for review, 7 questions) Draft catalogue v2 (principles only, one line plus one example each), for Jason's review
- ⬜ Precedent store: precedents live as reviewed rows in `drrp_gold` (section_id, decision, comment), tagged with a short pattern name, and are shown to the justifier as examples, not cited as rules
- ⬜ Promotion rule written into the brief: a new pattern becomes a principle only after about 3 provisions from at least 2 laws, **and Jason approves the promotion**; the justifier reports candidate patterns, it doesn't make rules
- ⬜ Precedent retrieval (Gemini): the justifier finds precedents by pattern tag, then same law and section, then text similarity; shown with Jason's comment
- ⬜ Spec (`DRRP-CLASSIFICATION.md`) trimmed to match: special-case rows that are precedents move to an appendix or the precedent store
- ⬜ The justifier brief rewritten for sentence units and principles (input to phase C)

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
