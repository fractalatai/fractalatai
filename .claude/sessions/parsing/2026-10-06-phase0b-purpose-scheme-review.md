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

- ⬜ Consumers: who uses purpose and how. That covers the purpose profile (legal#172, law shares ≥ 0.05), the DRRP gates (#76, `SKIP_PURPOSES`), compliance screening and the change list, and legal's UI/search. Asked legal 2026-10-06
- ⬜ Statutory vocabulary: the terms legislation itself uses in provision titles, cross-headings and Part titles (first counts below)
- ⬜ **Provision titles: do we have them?** Our LAT rows have no titles (e.g. `UK_uksi_1992_3004:reg.2` is empty; "Interpretation" isn't stored). Asked legal whether it can serve titles, cross-headings and Part titles. Titles may be the cheapest purpose signal of all
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

## Dependencies

- Legal's answers on headings and purpose usage
- The classification scheme inventory (for the gold-set scaffolding, phase 0a)
