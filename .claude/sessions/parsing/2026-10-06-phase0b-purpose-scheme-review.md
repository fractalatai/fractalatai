---
session: "Phase 0b: Purpose scheme review"
status: pending
opened: 2026-10-06
closed:
outcome:
related: ["parsing/2026-10-06-pipeline-from-the-regex-end.md", "docs/architecture/PURPOSE-CLASSIFICATION.md", "sertantai-legal#172", 76]
---

# Session: Phase 0b: Purpose scheme review (PENDING)

## Problem

The 18-value purpose scheme is a legacy of Jason's Airtable work. It's involved in **58%** of model disputes and about two-thirds of the ~45 prompt rules. Its values are invented categories (e.g. `Procedure+Detail`, `Establishment+Constitution`).

**Jason (2026-10-06):** UK laws use their own terms ("Interpretation", "Citation and commencement"…), which are easier to reason with than invented ones like "operative". Review whether the scheme is suitable before any gate or classifier is built on it.

## Todo

- ✅ (legal, 2026-10-06; below) Consumers: who uses purpose and how. That covers the purpose profile (legal#172, law shares ≥ 0.05), the DRRP gates (#76, `SKIP_PURPOSES`), compliance screening and the change list, and legal's UI/search. Asked legal 2026-10-06
- ⬜ Statutory vocabulary: the terms legislation itself uses in provision titles, cross-headings and Part titles (first counts below)
- ✅ (legal, 2026-10-06; below) **Provision titles: do we have them?** Our LAT rows have no titles (e.g. `UK_uksi_1992_3004:reg.2` is empty; "Interpretation" isn't stored). Asked legal whether it can serve titles, cross-headings and Part titles. Titles may be the cheapest purpose signal of all
- ✅ (12 coarse classes decided 2026-10-06; `PURPOSE-CLASSIFICATION.md` § Layered purpose) **Layered purpose** (Jason, 2026-10-06: "let's not limit ourselves to 1 method: a coarse/simple method could feed into a finer/more complex when needed"). Design the layers, with evidence on the 6,959 labels:
  - **coarse:** cheap cues (text, headings, titles when available) to a small set of statutory-term classes, high precision, else "undetermined" → escalate;
  - **fine:** classifier/SLM/LLM refines within the coarse class, only where a consumer needs it
- ✅ (Jason, 2026-10-06) Choose: 12 coarse classes in statutory terms; Enforcement its own class; Constitution kept
- ✅ (Jason, 2026-10-06) Rename to keep purpose clear of the DRRP vocabulary: Duties → **Requirements**, Powers → **Permissions**; fine split **Substantive** vs Procedure/Detail
- ⬜ Coarse layer build (phase 2): stem inheritance, enforcement cues, section titles when served; re-measure
- ⬜ Fine layer: Duties → Requirement vs Procedure/Detail (feeds the DRRP gate); others only on consumer need
- ⬜ Published-field migration with legal (legal-side refactoring list); retire legal's law-level classifier (#172)

## First evidence (2026-10-06)

Cross-heading and Part/Chapter title words across the hub (most frequent, generic words removed):
- enforcement 313, powers 211, information 204, functions 181, offences 163, duties 161;
- interpretation 128, notices 115, application 114, supplemental 103, introductory/introduction 187, preliminary 82;
- requirements 97, appeals 92, obligations 91, rights 89, penalties 85, compensation 70, licensing 69, register 70, reports 62;
- registration 59, procedure 59, charges 56, records 55, conditions 53, amendments 64, final 64.

These are the law's own purpose words, and they map onto a smaller, statute-shaped set than the current 18:
- Citation and commencement, Interpretation, Application, Extent;
- Duties, Powers / Functions, Rights;
- Notices, Information / Records / Registers, Procedure, Appeals;
- Enforcement, Offences and penalties, Compensation;
- Fees and charges, Licensing, Exemptions;
- Amendments, Revocations, Transitional and saving.

## Legal's answers (2026-10-06)

**Titles and headings:**
- **Cross-headings are held and served.** section_type `heading`: 5,538 rows in 261 of 1,069 laws, e.g. `UK_ukpga_1974_37:h.2` "General duties". Each provision's `hierarchy_path` names its heading (`part.I/heading.1/provision.1/sub.1`), so they join; they're already in our LAT.
- **Part/Chapter titles are held and served:** 3,258 part rows and 1,420 chapter rows in 551 laws.
- **Section/regulation titles are not held.** Legal's parser passes P1group through without reading its Title.
  - The fix is small: put P1group/Title on the article/section row, as text or a new title column.
  - But it changes `lat_hash` for nearly every law, so all ~1,069 re-pull. **Best folded into the single run's re-parse wave (Jason's call).**
  - For SIs without cross-headings (e.g. 1992/3004), the title is the **only** purpose signal.

**Consumers:**
- Per-provision `purposes` is stored as received. Nothing in legal or compliance filters on, validates or displays the values, so a vocabulary change breaks nothing.
- **Per-law `legal_register.purpose` isn't ours.** It comes from legal's own regex PurposeClassifier (15 hard-coded Airtable-era values, classified from the **law's title**). So there are two vocabularies for one concept today.
- #172 (the purpose profile) isn't built, so nothing depends on it yet.

**Constraints:** no DB enum, no Ash `one_of`, no TypeScript union. The only fixed list is legal's PurposeClassifier and its tests.

**Implications:**
1. A statutory-term vocabulary is cheap to adopt.
2. Retire legal's law-level classifier and derive law-level purpose from ours (#172), so there's one vocabulary.
3. Provision titles plus cross-headings plus Part titles could drive a regex-tier purpose (phase 2) with the LLM only for the residual.
4. **(Jason)** Fold the P1group/Title parser change into the single run's re-parse wave?

## Measurements (2026-10-06)

**Headings are mostly subject matter, not function.**
- Cross-headings cover only **14%** of the 6,959 labelled provisions; Part/Chapter titles cover **58%**.
- Most are topical ("height of chimneys", "glass", "parking in London"). Even functional ones are mixed: provisions under "Offences" most often label as Procedure+Detail (27%).
- The function words (Interpretation, Offences, Duties, Citation and commencement) live mainly in **section titles**, which we don't hold yet (the legal refactoring list).

**The stored (regex) purpose is essentially absent as a classifier.**
- 70% of provisions carry the legacy catch-all `Process+Rule+Constraint+Condition` and 16% `Unclassified`, so it agrees with the labelled purpose **5.5%** of the time (385/6,959).
- Only commencement is recognised well (77%), and Interpretation partly (32%).
- So the regex end of purpose has to be (re)built, not tuned.

**Layered design direction:**
- **Coarse layer (regex/text cues; headings and titles as extra evidence).** A few statutory-term classes, each with high-precision cues:
  - Citation and commencement ("may be cited as", "comes into force");
  - Interpretation ("means", "includes", "references to");
  - Application / Exemption ("applies to", "shall not apply");
  - Amendment / Revocation (already scope);
  - Offences and penalties ("commits an offence", "liable on conviction");
  - Duties (shall/must + actor);
  - Powers (may + actor);
  - otherwise undetermined → escalate.
- **Fine layer** only where a consumer needs it (e.g. within Duties: an own duty vs procedure/detail; within Powers: conferred vs enforcement).
- **Next:** measure each coarse cue's precision and coverage on the labels, and decide the class list in statutory terms (Jason).

## Coarse cue measurement, first pass (2026-10-06)

Text-only regex cues (first match wins; script in the session scratchpad, to move into `scripts/benchmarks/` when adopted), scored against Gemini v1.3 purposes (6,959) mapped onto coarse classes:
- **High precision, low recall:**

  | Class | Precision | Recall |
  |---|---|---|
  | Offences & penalties | 97% | 30% |
  | Interpretation | 93% | 19% |
  | Appeals & defences | 100% | 16% |
  | Fees & charges | 86% | 20% |
  | Amendment & revocation | 82% | 15% |

- **Citation & commencement:** 72% precision, 77% recall.
- **Duties:** 81% / 64%. **Powers:** 39%; enforcement powers use "may" too. **Enforcement:** no cue yet.
- **Undetermined:** 2,317 (33%), mostly list items with no modal of their own (1,310 Duties). Inheriting the stem's class should take most of them.

**Read:** the coarse layer works as a high-precision "decide what's easy" layer. Recall comes from stem inheritance and section titles, not from more regex.

## Dependencies

- Legal's answers on headings and purpose usage
- The classification scheme inventory (for the gold-set scaffolding, phase 0a)
