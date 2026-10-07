---
session: "Phase A: Sentence units"
status: active
opened: 2026-10-06
related: ["parsing/2026-10-06-meta-plan-sentence-units.md", "parsing/2026-10-06-phase0a-gold-set-scaffolding.md"]
---

# Session: Phase A: Sentence units (ACTIVE)

## Problem

Labels attached to the source's rows, and a sentence split into a stem and items needed rules for every piece. The legal sentence becomes the unit: a stem ("X shall—", "The notice must specify—") joined with its items and closing words, or a row with no stem above it. First measure (scratchpad `units.py`, a stem is an ancestor whose text ends in a dash or colon): the 61 test laws' 17,837 rows make 10,462 units (2,142 multi-row); the gold selection's 1,000 provisions make 883 units; unit text has a median of 265 characters, p90 718, max 11,880, max 37 rows.

## Todo

- ✅ (in `units.py` docstring and below) Unit definition, written down: what counts as a stem (dash, colon, "the following", "as follows"); nested stems ((2)— (a)— (i)); closing words that the source puts on the stem row; a stem whose items are in a Schedule
- ✅ (flags: nested 403, closing words 390, full-sentence item 165, sibling "But" 33, section text 17, long 14) Edge cases, with counts and examples: long lists (over 4,000 characters), "and"/"or" joins, items that are themselves full sentences with their own modal ("(b) the Secretary of State shall…"), provisos ("Provided that…") on separate rows
- ✅ (counted below: 14% of multi-row sentences with actors lose a role under one entry per label) **Holder shape per unit** (Gemini): count units with more than one active actor, with mixed Obligation/Liberty, and with the same label in two roles (#78 collisions, which merging items makes more likely); examples of each for Jason
- ✅ (first sentence review: all four defaults stood; mixed purposes go to the purpose scheme) **Policies, settled from the data in the app** (Jason, 2026-10-06). Provisional defaults until the sentence cards show otherwise: a proviso joins its sentence; a full-sentence item stays in the unit (two holders are two active actors); whole-sentence detail of another sentence keeps `continues`; a unit takes its own purpose or `inherit`. Questions: a proviso on its own row (part of the sentence, or its own unit?); an item that is a full sentence with its own modal and holder (stays in the unit, or splits out?)
- ✅ (keep `continues`) (from the app) a whole sentence that only details a duty or power in another sentence ("A notice of appeal shall be accompanied by…"): keep relation `continues` for it, or `no` with a pointer
- ✅ (own class or `inherit`) (from the app) purpose for a unit: its own class, or `inherit` from the section when it has no purpose of its own
- ✅ (built; PUWER reg.11(2) and Flood Risk s.43(5) read with closing words after the items) The assembler: `scripts/benchmarks/gold_v3/units.py` (unit id = root row id; ordered member rows; assembled text), tested on the known cases (PUWER reg.11(2), Flood Risk s.43(5), EAW reg.16, the 77 units whose items were reviewed)
- ⬜ (agent QA done, 5 fixes built; Jason's spot-check open: `data/gold/v4/assembler_spotcheck.md`) **Assembler QA** (Gemini): a stratified sample of assembled units (nested, long, schedule, proviso) checked by an agent and spot-checked by Jason before phase C labels anything
- ✅ (60 laws: 7,340 sentences, 2,362 multi-row; selection 848 units) Counts on the 61 test laws and the selection, written below; the selection re-keyed to units for phase C
- ⏸️ (moved to the meta-plan, phase E) Note for phase E: the same assembler in `fractalaw-core` so the pipeline labels sentences
- ⏸️ (moved to the dictionary-gap list in the meta-plan) **Disambiguate `Operator`** (Jason, 2026-10-07): one label covers an individual who operates (a machine, a vehicle) and an organisation that runs an installation, airport, well or regulated activity; split it (see below)
- ✅ (listed below; 21 Companies Act units, all three Parts out of domain) **Out-of-domain Parts of massive Acts** (Jason, 2026-10-07): the full Companies Act 1989 is held, so units from its company-law Parts bring dictionary drift and unfamiliar drafting. List the selection's units by Part, mark the out-of-domain ones and carry them to phase C's scope filter (legal's fix is on the legal-side list)

## Dependencies

- ✅ `legislation_text` rows with section ids (stem chain via `drrp_prompt.ancestors`)
- ✅ Phase 0a selection and reviewed rows

## First build and load (2026-10-06)

**Assembler** (`units.py`, `4b25eec`). A stem is a row with child rows whose text holds a dash or ends in a colon. The source puts closing words after the dash on the stem row ("shall consist of— and the provision of such information…"); the assembler moves them after the items. Document order comes from `position`.

| | text rows | units |
|---|---|---|
| 61 test laws | 16,354 | 7,915 (2,486 multi-row) |
| gold selection | 1,000 | 851 (380 provisions are items inside a sentence) |

Selection unit text: median 302 characters, p90 807. The long units (21 over 4,000) are mostly definition sections stored as one source row. Flags: nested 400, closing words 375, full-sentence item 165 (noisy heuristic), proviso row 33.

**First load to the sentence page** (https://claude.ai/artifact/3gViKiNKKikKepNBJft7dk; the row page stays as the record). The 226 sentences Jason had touched:
- 104 single-row: his row decisions carried as they are (502 rows);
- 37 stem reviewed and 85 items only: labelled at the sentence level by two Opus agents (`JUSTIFY_UNITS.md`), the items-only ones blind. Rows matching his stem decision loaded as approved (158).
- Open for review: 456 rows (365 easy, 87 hard, 4 new_edge).

**What the data says about the provisional defaults** (74 policy notes):
- *Proviso joins* (4 notes): a "But …" proviso that is its own subsection (a sibling, not a child) isn't joined (GHG ETS reg.34A(7) to (6); CC(S)A s.11(3)).
- *Full-sentence item stays* (15 notes): mostly false alarms. Where it matters, one label ends up with two roles (NRBW in HW Wales reg.53(5): "may prescribe" and "must publish"), and one entry per label drops one (#78). Two holders in one sentence worked cleanly (Water Act s.81(5)).
- *Whole-sentence detail continues* (14 notes): the holder of the continued duty is named but not listed (reg.40(2) HSE), and precedents split on that. "In determining X, SEPA must seek to ensure Y" reads as its own duty (Flood Risk s.16(5), s.49(7), s.50(8)).
- *Purpose* (4 notes): a sentence mixing two items' purposes forces single-select to drop one (PHA s.82(5)); compensation lands in Offences via the old Liability mapping. No sentence needed `inherit` in load 1a; 5 did in 1b.

**Source text faults** (legal's LAT parse, not the assembler; for the legal-side list):
- CAA 1982 s.44(6): the closing words carrying the duty ("… shall pay compensation") are missing;
- Biological Agents Directive Art.8(1)(d): the lead-in "any necessary protective equipment is:" sits at the end of the item;
- Companies Act 1989 s.164(1): closing words cut mid-phrase ("In the application of this subsection in Scotland,").

**For Jason:** gas regs reg.40(2) purpose: his current row decision approves Requirements (he changed it to Application in the pilot, then reverted), but PURPOSE-CLASSIFICATION.md cites reg.40(2) as the Application example.

**Resolved (Jason, 2026-10-06):** gas regs reg.40(2) purpose is **Application, exemption and transition**. *"The section is titled Exemption certificates."* Recorded on the sentence page and in both gold versions; the purpose doc's example stands. The title decided it: more evidence that section titles (P1group/Title, on the legal-side list) are the cheapest purpose signal, and that purpose belongs to the section.

## Operator is two actors (Jason, 2026-10-07)

*"We need to disambiguate Operator. It can mean an individual who operates, or something much larger."*

The dictionary has a single `Operator` label (governed, category `other`). Its regex matches any "operator", with only Economic Operator excluded. In the sentence gold it labels 13 units, and those span both meanings:
- **Organisations:** the ELD "relevant operator" (whoever runs an occupational activity), airport and aircraft operators, the well-operator, the operator of a permitted facility, a "digitally excluded operator";
- **Individuals:** a person working a machine or plant, as in PUWER reg.17(3).

The holder shape differs. An operator-organisation carries duty-holder obligations, permits and reporting. An operator-individual is a worker-like actor at the point of use.

Options, to settle from the data and not now:
- split into `Org: Operator` and `Ind: Operator`, using the `Ind`/`Org` prefixes that already exist;
- or tell them apart by context ("operator of a <installation/facility/airport>" → Org; operating "work equipment", a "machine" or a "vehicle" → Ind).

Most "the operator" mentions rely on a defined term, so the law's definitions (already in the evidence context) usually decide. For now, flag it in the review comments and don't add rules. It goes to the phase B recurrence check and the dictionary-gap list.

## First sentence review (Jason, 2026-10-07)

All 226 sentences reviewed (1,116 rows; pull `data/gold/v4/review_decisions/pull_20261007a`, imported to `gold-v4-sentence`). *"The quality appeared to be high."*

| | rows | approved | changed | queried |
|---|---|---|---|---|
| carried (singles, matching stem) | 660 | 656 | 0 | 4 (old batch-2 queries, carried) |
| open (sentence labels) | 456 | 447 | 1 | 8 |

Of the open rows, **98.0% were approved unchanged.** The row-level batches had 1,073 of 1,106 (97.0%) approved, but they needed about 6 rulings each. This review needed **no new labelling rulings**: every comment is about the dictionary, the domain or purpose.

**Queries and comments:**
- **Dictionary:**
  - Gvt: Agency: Scottish Natural Heritage (Flood Risk s.50(8));
  - a `Patient` label (Biological Agents Art.15(1));
  - Gvt: Judiciary: Employment Tribunal (CAW reg.18(8), useful for HR);
  - `Ind: Licensee` is possibly not an individual (SSI 2018/219 reg.30(1));
  - review `Authorised Person` (also the batch-2 query on SI 2012/3032 reg.2);
  - split `SC: T&L:` into Supply Chain and Transport & Logistics, e.g. Transport & Logistics: Gas Transporter (SI 2013/1471 reg.11(1)).
  - CAA and Registrar General: queries carried from batch 2, already added to the dictionary on 10-06.
- **Purpose:**
  - PHA s.82(5): daughter clauses carry different purposes (the single-select limit);
  - CAA s.44(6): compensation isn't Offences and penalties. *"Offences and penalties run from government to governed. Compensation runs in the opposite direction."* (The old Liability mapping is wrong.)
  - SI 1998/3111 reg.4(2): deemed compliance, purpose still open (carried).
- **Out of domain:** Companies Act s.155A(4) and s.164(1): *"remove from Gold standard"*. Only the EHS-relevant Parts will be stored, so the client of a clearing member doesn't need a dictionary entry.

**What it says about the provisional defaults:** the sentence unit holds. All four defaults stand for now:
- proviso;
- full-sentence item;
- whole-sentence `continues`;
- own purpose or `inherit`.

The one live issue is purpose on a mixed sentence (PHA s.82(5)). That is a purpose-model question (multi-select or section-level purpose), not a unit question.

## Companies Act 1989 in the selection (2026-10-07)

The selection has 21 units (23 rows) from the Act, in three Parts:
- **Part III**, Investigations and Powers to Obtain Information: s.83(1), s.83(8), s.87(5);
- **Part V**, Other amendments of Company Law: s.112(2), s.112(6);
- **Part VII**, Financial Markets and Insolvency: 16 units, including s.155A(4) and s.164(1).

None is EHS&HR. Proposal for phase C: drop all 21 from the gold selection (4 are already in `gold-v4-sentence`) and replace them in the top-up from in-domain laws. The dictionary drops their actors too.

**Dropped (Jason, 2026-10-07): all 21 Companies Act units are out of the gold set.**
- They are listed in `data/gold/v4/excluded_units.csv`. `unit_evidence.py` skips that list, so they never come back.
- The 4 reviewed units are gone from `gold-v4-sentence` (19 rows deleted, backup at `data/gold/v4/backup_companies_act_rows_20261007.json`), from the sentence page and from `evidence.jsonl`.
- `gold-v3-draft` keeps its rows as the record of the row scheme.
- The gold set is now 222 sentences (1,097 rows). The selection's 851 units drop to 830; phase C's top-up replaces them from in-domain laws.

**Legal closed #166 (2026-10-07).**
- **Companies Act 1989 is excluded from LAT.** An agent check found no EHS/HR provisions. Its rows are discarded, its `lat_scope` is "excluded" and it will show `row_count` 0 after the next manifest sync. This matches the gold drop.
- **Test laws: 61 → 60.** The assembler counts above include the Act's 1,062 rows and should be re-run after the sync.
- **Scoped Acts:** 14 large Acts are now scoped (58,507 → 10,679 rows). None is among the test laws, so the gold set is otherwise unaffected.
- **Still waiting on legal:** the list-text parser fix and its affected section_ids.

## Legal's list-text repair pulled (2026-10-07)

Legal fixed its LAT parser's list-text bug and gave our 60 test laws a whole-law check.

**Pull** (`pull-lat --laws <60> --apply`, local backup in `data/lat-sync/backup_20261007_test60/`):
- 55 laws applied, 5 already in sync, no gate failures.
- 202 rows `text_changed` and 2 `grown`; nothing archived or held.
- The 55 laws are in `data/lat-sync/reparse_20261007_111737.txt` for the re-parse.

**Gold impact:**
- 5 of the 222 sentences have new text: Directive 89/391 Art.3, Directive 2000/54 Art.8(1), Reg 1079/2012 Art.3, SI 2002/1861 reg.2, SI 2012/3032 reg.2.
- All 5 are reorderings: the chapeau moved back to the front, plus de-duplication. Their labels still hold, so there is no re-review.
- Their text hashes are updated in `gold-v4-sentence`, and their cards on the sentence page carry the repaired text.
- 20 of the 830 selection units touch repaired rows. They aren't labelled yet, so they'll simply get the new text.

**Assembler fix:**
- The repair filled the previously empty Water Act s.3 row with the s.3(12) definitions ("'the appropriate authority' means— …"). Their dash made s.3 a stem that swallowed all 22 subsections.
- A stem now needs list-item children (paragraphs or sub-paragraphs).
- 6 units in the test laws had this fault, all definitions placed on a section or regulation row: SSI 2000/95 reg.2, Water Act s.3 and s.58, SI 2000/1043 reg.2, SI 2004/1490 reg.2, WSI 2005/1806 reg.5.
- The selection's 851 units are unchanged.

**CAA 1982 s.44(6):** legal recovered the closing words "shall pay such compensation … appropriate tribunal". They sit on the s.44 row, because legislation.gov.uk has them as a BlockText after subsection (6). The gold labels already carry that duty.
- **Candidate assembler rule:** trailing text on a section row whose last subsection is a stem continues that stem. Not built yet; it goes to assembler QA.

**Not done:** re-parse of the 55 laws (regex tier). See below.

## Holder shape per sentence (2026-10-07)

**Source:** the pipeline's actors (`provision_actors`, the reconciled `drrp`/`position`) on the member rows, merged per sentence. These are row-level pipeline labels, not gold, so the figures are estimates of shape, not error rates.

**60 test laws:** 7,349 sentences (2,362 multi-row); 3,610 have actors, 1,413 of them multi-row.

| Shape | Sentences |
|---|---|
| more than one active holder | 298 (185 multi-row) |
| active Obligation and active Liberty in one sentence | 93 (90 multi-row) |
| same label in two roles (multi-row only; a single row can't have this) | 408 |
| ↳ role + mentioned only: the strongest role wins, **no loss** | 210 |
| ↳ **lossy**: two substantive roles for one label | 198 |

**The lossy collisions, per label:**
- active + counterparty: 131;
- active Obligation + active Liberty: 59;
- beneficiary + counterparty: 19;
- active + beneficiary: 10;
- all three: 4.

**What's behind them:**
- Most are a generic label standing for **two different people** in one sentence. `Ind: Person` is typical: "a person may request …; the authority shall notify the person".
- Some are one holder with a duty and a power: "the Minister may … and shall …" (`Gvt: Authority` in Water Act s.3(6)).

**Reading:**
- One entry per label (POS-15) loses a role in **198 of 1,413 multi-row sentences with actors (14%)**, which is 5% of all sentences with actors. Rows can't collide, so sentence units created these.
- Gold (222 reviewed): 23 of 191 sentences with actors have more than one active holder, and 2 mix Obligation and Liberty. The justifiers flagged 3 drops in policy notes, and Jason's review raised none.
- **This is #78, now with a size:** one entry per label loses a role in about 1 in 7 multi-row sentences. Two fixes:
  - allow a second entry for a label with a different role (actor key = label + referent/role);
  - or let `holds` be "both" for the duty-and-power case.
- **Decide before phase C labels at scale.** It changes the gold schema (`drrp_gold` key is gold_version, section_id, field, actor_label) and the review page.

## Assembler QA (2026-10-07)

An Opus agent checked 32 sampled sentences across the strata and scanned all 60 laws; the scripts are in the scratchpad.

**Sample:** 26 of 32 OK.
- nested 6/6, closing words 6/6, full-sentence items 5/5, long 1/1;
- the "not a stem" guard was right 4/4: those faults are source text (definitions sitting on the section row).

**Fixed in `units.py`:**
1. **Two-dash stems** ("If … believe— (a) (b) then … the commander may take … measures— (i) (ii) (iii) and …"): each dash opens the next run of items, so the middle words now sit between the two runs. 6 sentences: CAA s.94(2), s.84(1), s.78(9), s.88(10); SI 1988/1324 reg.7(1); PHA s.45(9).
2. **One sentence per row.** An item without a dash ("(b) the following instruments … that is to say (i) (ii)") left its sub-items as sentences of their own too. 9 rows under 4 parents were in two sentences; now 0.
3. **Lead-ins that lost their dash** are stems. List items always continue their parent's words: "the diving project plan shall;", "In Scotland", "may direct in writing that: shall be exempt …". With no dash, a colon marks where the closing words start. 25 new stems; 6 selection items re-keyed to their lead-in; no gold sentence changed.
4. **A root that isn't a stem prints only its own row.** A section row with subsections used to print the whole section (CAA s.84: 7,841 characters). 46 sentences affected.
5. **Stray dash runs** ("–—", SI 2020/1265 reg.47(2)).

**Flagged, not built:**
- **Sibling "But …" rows** (33 in 12 laws; 12 in GHG ETS): 11 cite the sibling before them, 1 cites another, 21 cite none. Most are full sentences, so the better fix is a `qualifies` link to the previous sibling rather than a join. A bare "But—" (SSI 2018/219 reg.78(4)) can't be read alone, so join that one. Flag: `proviso_row`.
- **Text on a section row with subsections** (17 rows; flag `section_text`): legislation.gov.uk's trailing BlockText, or definitions. The "last subsection" rule would be wrong 7 times out of 9. Two better rules:
  - text that opens with a quote mark (definitions) attaches to the child that ends in a dash and has no items: 6 of 6 resolve;
  - lower-case text attaches to the open stem, the one whose last item ends in ",", "or" or "and" and that has no closing words yet: 8 resolve, 1 is ambiguous.
  - Better still, legal fixes it at source. It goes on the legal-side list with the CAA s.44 case.
- `OWN_SUBJECT` over-fires ("any changes which could…", "that SEPA may…"): it's a flag only, so low priority.

**Jason's spot-check:** 15 sentences in `data/gold/v4/assembler_spotcheck.md`: 11 fixed or doubtful, 4 OK.

**Jason's spot-check query (2026-10-07): WSI 2005/1806 reg.5(1)** showed only "In these Regulations—". legislation.gov.uk has the definitions under 5.—(1); legal's parse puts them on the reg.5 row.
- **Fixed in the assembler:** definitions (text opening with a quote mark) on a section row go back to the one child that ends in a dash and has no items. With two such children, the interpretation lead-in ("In this section—") wins over "… there is inserted—".
- **All 6 cases resolve:** SSI 2000/95 reg.2(1), Water Act s.3(12) and s.58(13), SI 2004/1490 reg.2(1), WSI 2005/1806 reg.5(1), SI 2000/1043 reg.2(1). Flag `definitions_moved`.
- No gold or selection change. Reported to legal as a source fault.

## Gemini review feedback (2026-10-07): #78 option (c)

Gemini 2.5 Pro was asked for a harsh review of option (c) and an answer to question 2. Raw review: `data/code-review/drrp-issue78-option-c.md`; proposal and examples: scratchpad `issue78_option_c.md`.

**Accepted:**
- **Key the extra entry by `position`, not a free-text referent.** The key becomes (gold_version, section_id, field, actor_label, position). A quoted referent ("searcher") is hard to keep consistent, can't be predicted as a class, and can't be scored by exact match. Position is already a labelled field, and the split only fires when roles differ, so it is the natural key.
  - s.47(4) becomes `Ind: Person`/active (Obligation) + `Ind: Person`/beneficiary.
  - Residual: two parties with the same label and the **same** position but different holds can't both be kept. It hasn't been seen in the data; if it appears, `holds: both` is the fallback.
- **`holds: both`.** Minimal, still a single-class target, and trivial downstream (duties = Obligation or both).
- **Question 2: `act` becomes a list** (multi-label). Picking a "dominant" act is subjective and loses Art.8(1)'s distinct acts. The model task becomes multi-label `act`, a standard one.

**Not accepted:**
- **Splitting two different parties with the same label and the same role** (s.50(8): two authorities, both beneficiaries). Merging loses identity but no role, and the register is per label and role. Kept as one entry.

**Cost of the change, to do before phase C:**
- PK migration for 1,097 gold rows: position is backfilled from the value, so no data changes.
- The review page: slot key, add a second entry, `both` in holds, multi-select act.
- The loader and importer.
- The evaluation spec (row tiers against sentence gold) scores multi-label `act` and (label, position) pairs.
