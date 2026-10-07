---
session: "Phase C: Gold set at the sentence level"
status: active
opened: 2026-10-07
related: ["parsing/2026-10-06-meta-plan-sentence-units.md", "parsing/2026-10-06-phaseB-principles-precedents.md", "docs/architecture/DRRP-RULE-CATALOGUE.md", "scripts/benchmarks/gold_v3/JUSTIFY_V2.md"]
---

# Session: Phase C: Gold set at the sentence level (ACTIVE)

## Problem

The gold set has 220 reviewed sentences. The rest of the selection (848 sentences, less 23 excluded) still needs labels: **605 sentences**, labelled on catalogue v2 by brief v2, with the 5 nearest precedents in every evidence pack, and reviewed by Jason on the sentence page. Phase C also tops up the scarce classes, checks Jason against himself, writes the scoring spec phase D needs, and freezes the set.

## Todo

- ⬜ (loaded 2026-10-07: 99 sentences + 1 proposed exclusion, 466 open rows; Jason reviewing) **Batch 1** (about 100 sentences): evidence packs (`unit_evidence.py --selection`, the not-yet-gold units), two Opus justifier agents on `JUSTIFY_V2.md`, load (`load_units.py`), export and sync to the sentence page; Jason reviews
- ⬜ Convergence after each batch: approve-unchanged rate, new candidate patterns and dictionary candidates (notes), new principles promoted (target: none without Jason, ~3 provisions from 2+ laws)
- ⬜ Batches 2 to 6 until the 605 are reviewed; stop early if the approve rate holds and no new patterns appear (meta-plan stop signals)
- ⬜ Amending text and out-of-domain sentences that justifiers propose for exclusion → `excluded_units.csv` once Jason agrees
- ⬜ **Top-up** of scarce classes from other held-out laws (not the 60 test laws): REL-27 enforcing-authority designations, `implied-access-right`, `functions-list`, applying-provision holders (HOLD-05 forms), TYPE-03 cases; target ~3 provisions from 2+ laws each so rare precedents can be judged for promotion
- ⬜ **Intra-rater check:** about 50 reviewed sentences re-reviewed blind after a gap; agreement per field
- ⬜ **Row-to-sentence scoring spec** for phase D: how row-level tier outputs (regex, classifier, SLM) combine into one sentence prediction, and how (label, position) pairs and list `act` are scored; context principles (HOLD-05, INF-03) scored apart
- ⬜ Dictionary candidates from notes → the dictionary list (2+ Families rule); `Gvt: Authorised Person` with Family gating (pending task)
- ⬜ **Freeze:** gold-v4-sentence → a frozen version (`gold-v4.0`), with counts per field and class

## Dependencies

- ✅ Phase A: sentence units, the assembler and its QA, the #78 schema
- ✅ Phase B: catalogue v2, the precedent store, `JUSTIFY_V2.md`, the trimmed spec, gold made consistent with v2
- ✅ The 60 test laws repaired by legal (list-text fix pulled 2026-10-07)
- ⬜ Legal's remaining parse repair (batches 4–11, #174 definitions/BlockText). It doesn't touch the 60 test laws' text except for the #174 section-row fix; pull once after it lands (NAS backup first). Top-up laws may change, so take top-up sentences after the pull, or re-check their text

## Batch 1 (2026-10-07)

**The batch:** 100 of the 605, drawn round-robin across laws (58 laws, 39 multi-row; median 298 characters). Evidence packs for all 825 selection sentences (`unit_evidence.py --selection`), each with 5 precedents. Two Opus justifiers on `JUSTIFY_V2.md` (`data/gold/v4/justified/c1a.jsonl`, `c1b.jsonl`); every rule ID cited is a v2 principle, `P:`, `PREC:` or `NEW`.

**Loaded:** 466 open rows for 99 sentences (357 easy, 106 hard, 3 new_edge). Synced to the sentence page as 99 new cards; the page now holds 319 sentences.

**Proposed exclusion (not loaded):** SI 2008/1911 reg.58(1), which revokes the LLP Regulations 2001 (amending text). Needs Jason's agreement.

**Bug fixed:** `load_units.py` still had the pre-#78 conflict target. The first insert failed inside its transaction, so nothing was written; fixed and reloaded.

**Notes: {'question': 33, 'candidate_pattern': 7, 'dictionary_candidate': 7, 'source_fault': 3}.**
- **Candidate patterns:**
  - `establishment-of-body`;
  - `amount-computation` (fees owed under another provision);
  - `passive-with-agent` ("shall be made by the operator": REL-33 or REL-28?);
  - `agentless-power` ("A reasonable charge may be made");
  - `thing-subject-may-relaxation`; `references-do-not-include`; `mandatory-sentence` (a court "must impose" a minimum sentence: REL-04 against REL-33).
- **Dictionary candidates:** air traffic controller, judgment creditor, partner (in a partnership), training provider, a government nominee.
- **Recurring question:** three laws that look outside EHS&HR (Foreign Judgments Act 1933, CTSA 2015, VCRA 2006); the LLP accounts regs are similar.

## Research: powers to make further law, a missing purpose class? (2026-10-07)

**Jason, reviewing batch 1:** *"stuck on law that gives powers to a minister to make further law - does our purpose have a missing category?"*

**How the 11 classes handle it now:**
- "The Secretary of State may by regulations make provision requiring…" → Permissions (Minister active Liberty).
- "Regulations under this section may include provision about—" → Permissions or Requirements: the gold is split (CTSA s.46(3) Permissions, CAA s.83(6) Requirements).
- Parliamentary procedure → Requirements (batch-1 ruling, "the procedure is the how").
- Commencement by regulations → Citation and commencement.

So the same kind of sentence lands in three classes, and Permissions mixes a minister's law-making powers with permissions people act on.

**How much:**
- hub: 3,878 rows in 241 laws (1.1% of rows) match enabling-power wording;
- the gold selection: 17 of 825 sentences; the reviewed gold: 7 of 319.

**What the law calls it:**
- **Interpretation Act 1978 s.21(1):** "subordinate legislation" means *"Orders in Council, orders, rules, regulations, schemes, warrants, byelaws and other instruments made or to be made under any Act"*.
- **Headings in our corpus:** "Subordinate legislation" (4 laws), "Regulations" (5), "Powers to make regulations" (3), "Orders and regulations" / "Regulations and orders" (5), "Byelaws" (6), and EU "Delegated and implementing acts" (2). The class name meets Jason's rule that classes use the terms laws use in their headings.

**legislation.gov.uk `ConfersPower` is not usable for this.** On HSWA 1974 it is `true` on s.7 (employees' duties, no power) and absent on s.21 (improvement notices, a real power). It doesn't mark enabling powers, or powers generally. Tell legal before it is treated as an authoritative power flag.

**Proposal: a 12th coarse class, "Subordinate legislation".**
- **Covers:**
  - a power or duty to make regulations, orders, rules, schemes or byelaws;
  - what such instruments may or must contain ("Regulations under this section may include…");
  - their making procedure: consultation, laying, affirmative and negative resolution;
  - EU delegated and implementing acts.
- **Not:**
  - administrative orders or notices addressed to a person (enforcement and improvement notices, compliance notices);
  - commencement by order, which stays Citation and commencement;
  - the duties the regulations later create.
- **Relation is unchanged:** a minister's power to make regulations is still `yes`, with the Minister active Liberty. Purpose is orthogonal.
- **What it resolves:** the gold split above, and the batch-1 ruling that parliamentary procedure is Requirements (it would move here).
- **Cost:** about 7 gold sentences and the batch-1 open rows re-checked; PURPOSE-CLASSIFICATION.md, the brief and the review page choices updated; legal's purpose vocabulary (#172) told.
- **Boundary case:** local instruments made under an Act that bind a place or class (tree preservation orders, development orders) fit s.21, but they read as administrative. Proposed: include them only when they are made generally, not addressed to a named person. To confirm with Jason.

**Adopted (Jason, 2026-10-07):** *"agree - add Subordinate legislation and tell legal - this fixes the strain we are seeing in the data and should make Purpose tagging these sentences 'easy'"*.
- **Written:**
  - PURPOSE-CLASSIFICATION.md (12 classes; the class rules; it replaces the parliamentary-procedure → Requirements ruling);
  - JUSTIFY_V2.md;
  - the catalogue's purpose pointer;
  - the meta-plan;
  - `coarse_purpose.py`: a cue that fires after commencement and before Requirements/Permissions. All 19 hand-judged sentences cue correctly; across the 825 selection sentences it fires on 35, with 3 misfires (about 91%): CAA s.45(7) offence proceedings, a district designation, a 1933 proceedings clause;
  - the review page: version 3, with the class in the purpose dropdown.
- **Gold:**
  - Batch-1 decisions so far were imported first (1,532 decided, 51 batch-1 rows still open).
  - Then 14 sentences' purpose was set to Subordinate legislation as `change` decisions: 9 earlier ones (asp 2019/15 s.31(2), s.11(3); CTSA s.27(6), s.46(3); Water Act s.10(1); PHA s.82(5), which resolves Jason's mixed-purpose query; CAA s.83(6); Directive 89/654 Art.9a(4); VCRA s.39(1)) and 5 in batch 1 (s.20(9), which answers Jason's "review the reason this got put as Requirement", plus s.13(4), s.31(1), Art.9a(3), s.104(2)). Jason's earlier comments were kept after "earlier:".
  - **Kept:** commencement (asp s.32(2), CTSA s.23(1)), a compensation duty, an offence, an application clause, and a report laid before Parliament (Water Act s.81(5)).
- **Page:** the 14 decision slots were written with pinned versions; a pull-back shows 1,532 = 1,532 decided rows, 0 mismatches.
- **Legal told** (#172 vocabulary) and warned that ConfersPower is unreliable.
- Precedents rebuilt.
