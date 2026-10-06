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
- ⬜ Options, with evidence (measured on the 6,959 labels):
  - (A) keep the 18 values, renamed to statutory terms;
  - (B) a heading-derived purpose: the provision's own title/heading class;
  - (C) a smaller set of statutory terms, scoped to what the consumers need
- ⬜ **(Jason)** Choose; then the spec, the published-field migration with legal, and the phase 2 gates

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

## Dependencies

- Legal's answers on headings and purpose usage
- The classification scheme inventory (for the gold-set scaffolding, phase 0a)
