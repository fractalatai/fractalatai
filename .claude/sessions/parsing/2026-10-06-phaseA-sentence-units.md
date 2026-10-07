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

- ⬜ Unit definition, written down: what counts as a stem (dash, colon, "the following", "as follows"); nested stems ((2)— (a)— (i)); closing words that the source puts on the stem row; a stem whose items are in a Schedule
- ⬜ Edge cases, with counts and examples: long lists (over 4,000 characters), "and"/"or" joins, items that are themselves full sentences with their own modal ("(b) the Secretary of State shall…"), provisos ("Provided that…") on separate rows
- ⬜ **Holder shape per unit** (Gemini): count units with more than one active actor, with mixed Obligation/Liberty, and with the same label in two roles (#78 collisions, which merging items makes more likely); examples of each for Jason
- ⬜ **Policies, settled from the data in the app** (Jason, 2026-10-06). Provisional defaults until the sentence cards show otherwise: a proviso joins its sentence; a full-sentence item stays in the unit (two holders are two active actors); whole-sentence detail of another sentence keeps `continues`; a unit takes its own purpose or `inherit`. Questions: a proviso on its own row (part of the sentence, or its own unit?); an item that is a full sentence with its own modal and holder (stays in the unit, or splits out?)
- ⬜ (from the app) a whole sentence that only details a duty or power in another sentence ("A notice of appeal shall be accompanied by…"): keep relation `continues` for it, or `no` with a pointer
- ⬜ (from the app) purpose for a unit: its own class, or `inherit` from the section when it has no purpose of its own
- ✅ (built; PUWER reg.11(2) and Flood Risk s.43(5) read with closing words after the items) The assembler: `scripts/benchmarks/gold_v3/units.py` (unit id = root row id; ordered member rows; assembled text), tested on the known cases (PUWER reg.11(2), Flood Risk s.43(5), EAW reg.16, the 77 units whose items were reviewed)
- ⬜ **Assembler QA** (Gemini): a stratified sample of assembled units (nested, long, schedule, proviso) checked by an agent and spot-checked by Jason before phase C labels anything
- ⬜ Counts on the 61 test laws and the selection, written below; the selection re-keyed to units for phase C
- ⬜ Note for phase E: the same assembler in `fractalaw-core` so the pipeline labels sentences
- ⬜ **Disambiguate `Operator`** (Jason, 2026-10-07): one label covers an individual who operates (a machine, a vehicle) and an organisation that runs an installation, airport, well or regulated activity; split it (see below)
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
