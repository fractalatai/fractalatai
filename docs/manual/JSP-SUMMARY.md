# MoD JSP Analysis: From Policy Documents to Actionable Compliance Intelligence

## The Problem

QQ operates under MoD contracts. Those contracts require compliance with Joint Service Publications (JSPs) — the Ministry of Defence's internal policy framework for safety, environment, and operational management. JSPs sit between legislation and day-to-day operations: they take statutory requirements like the Health and Safety at Work Act and translate them into specific MoD responsibilities, roles, and procedures.

Today, understanding what JSPs require means reading hundreds of pages of PDF across multiple publications. Answering basic questions — "What must a contractor do under JSP 375?" or "Which JSP obligations map to the Electricity at Work Regulations?" — requires manual cross-referencing between documents that were never designed to be queried.

## What We Built

We parsed 11,351 provisions across 10 MoD JSPs into structured, queryable data — then enriched every provision with obligation classification, role assignments, cross-references, competence requirements, and mandated artefact detection. The same provisions that exist as paragraphs in a PDF are now rows in a database that can be filtered, linked, and reported on.

### Six Layers of Intelligence

**1. Obligation Classification**

Every JSP provision is classified by obligation strength:

| Strength | Meaning | Count |
|----------|---------|-------|
| Mandatory | "must", "shall", "will", "is to" — a binding requirement | 3,226 |
| Recommended | "should" — expected but not absolute | 953 |
| Permissive | "may" — allowed but not required | 297 |

JSPs use "will" and "is to" as mandatory — unlike legislation, where "will" is future tense. Our parser handles this distinction automatically. 6,021 provisions were classified across 142 JSP chapters.

**2. Role Assignments (RACI)**

JSPs assign organisational roles to obligations. For each obligation, we extract who is Responsible, Accountable, Consulted, or Informed. Across the full corpus:

| Role | Obligations assigned |
|------|---------------------|
| Defence Safety Authority | 312 |
| Commander / Manager | 255 |
| Accountable Person | 222 |
| Defence Organisation | 200 |
| Commanding Officer | 139 |
| Head of Establishment | 132 |
| User / Operator | 112 |
| Top Level Budget Holder | 66 |
| Secretary of State for Defence | 52 |
| Contractor | 45 |

1,719 RACI assignments across 5,028 obligations. The query "show me everything a contractor must do across all applicable JSPs" returns 45 specific obligations — not a reading exercise.

**3. Legislative Traceability**

JSP provisions reference the legislation they implement. We extract and resolve these references automatically:

- "in accordance with the Electricity at Work Regulations 1989" → linked to the specific UK statutory instrument
- "as set out in JSP 375 Volume 1, Chapter 8" → linked to the specific JSP chapter

Across the full corpus: 1,969 cross-references extracted — 357 to legislation, 1,455 to other JSP chapters, 132 to HSE guidance, 25 to British Standards. 88% were automatically resolved to their source.

When legislation changes, you can immediately see which JSP obligations are affected.

**4. Mandated Artefacts**

JSPs mandate specific things — risk assessments, safety cases, permits, hazard logs. We detect these and classify them by type:

| Artefact type | Count |
|---------------|-------|
| Risk Assessment | 554 |
| Occurrence Report | 82 |
| Procedure | 67 |
| Safety Case | 59 |
| Audit Report | 35 |
| Training Record | 31 |
| Emergency Plan | 25 |
| Inspection Report | 22 |
| Method Statement | 18 |
| Permit | 14 |
| Maintenance Record | 8 |
| Hazard Log | 7 |

922 mandated artefacts detected. Each is linked to the obligation that requires it, the role responsible, and the JSP chapter. These map directly to compliance controls.

**5. Compliance Controls**

Each mandated artefact generates a compliance control — an actionable statement of what must exist and who owns it. These sit alongside the 1,556 legislation-derived controls in the same control register, but are specific to the defence sector.

922 JSP-derived controls, additive to the existing legislation controls. Defence customers see both; non-defence customers see only the legislation controls.

**6. Terms and Definitions**

JSPs define acronyms and terms inline. We extract 1,351 terms across the corpus and detect definitional conflicts — the same acronym defined differently in different JSPs. 1,670 cross-JSP term conflicts detected, surfacing inconsistencies that would otherwise require reading every glossary manually.

## What This Means for QQ

### "What do the JSPs require of us?"

Filter all JSP obligations where the Contractor role is Responsible. Across the full corpus, that's 45 specific obligations — each linked to the JSP chapter, the obligation text, and the mandated artefacts. No manual reading required.

### "Which JSP obligations map to which laws?"

Every JSP obligation that references legislation is linked to the specific law in our corpus. 357 legislation references across 10 JSPs, 88% resolved. When the Electricity at Work Regulations are amended, you can trace the impact through to every JSP chapter that implements those regulations — and every obligation within those chapters.

### "What artefacts do we need?"

554 risk assessments, 59 safety cases, 14 permits, 25 emergency plans — each linked to a specific obligation, a responsible role, and a JSP chapter. The complete inventory of what the MoD policy framework requires to exist, queryable by type, by role, or by JSP.

### "What competence do our people need?"

Obligations with competence requirements are flagged and linked to the responsible role and JSP chapter. This feeds directly into training needs analysis.

### "How does this connect to our compliance controls?"

JSP obligations are more operationally detailed than legislation. Where the law says "the employer shall ensure health and safety", the JSP says "the Commanding Officer shall maintain a Safety Case that demonstrates ALARP, reviewed annually by the Operating Duty Holder." 922 JSP-specific controls enrich the control register with this operational detail.

## By the Numbers

| | |
|---|---|
| JSP publications | 10 |
| Chapters enriched | 157 |
| Provisions pulled | 11,351 |
| Provisions classified | 6,021 |
| Obligations extracted | 5,028 |
| RACI assignments | 1,719 |
| Cross-references | 1,969 (88% resolved) |
| Mandated artefacts | 922 |
| JSP controls | 922 |
| Terms extracted | 1,351 |
| Term conflicts | 1,670 |
| Source PDFs processed | 167 |

### JSP Coverage

| JSP | Title | Provisions |
|-----|-------|-----------|
| JSP 375 | Health and Safety Handbook | 2,280 |
| JSP 392 | Radiation Protection | 2,263 |
| JSP 403 | Defence Ranges Safety | 1,588 |
| JSP 975 | MoD Lifting Policy | 1,490 |
| JSP 520 | Ordnance, Munitions & Explosives | 1,420 |
| JSP 816 | Defence Environmental Management | 1,221 |
| JSP 815 | Defence Safety & Environmental Management | 1,011 |
| JSP 418 | Leaflets (various safety topics) | 857 |
| JSP 376 | Defence Acquisition Safety Policy | 367 |
| JSP 425 | Radiation Detection Equipment Testing | 85 |

## How It Works

```
PDF Documents          Structured Data              Enriched Intelligence
──────────────         ───────────────              ─────────────────────
167 JSP PDFs    →      11,351 provisions     →      6,021 obligations classified
                       (text, structure,             1,719 roles assigned (RACI)
                        hierarchy)                   1,969 legislation links (88% resolved)
                                                     922 mandated artefacts detected
                                                     922 compliance controls generated
                                                     1,351 terms extracted
                                                     1,670 term conflicts surfaced
```

The system pulls JSP provisions from the database, runs them through a purpose-built parser that understands MoD policy language, and publishes enriched data back. The enrichment happens automatically — no manual tagging, no spreadsheets, no reading hundreds of pages.

Results are queryable in the compliance platform alongside the legislation they implement. A single view shows the legal obligation, the JSP policy that implements it, the role responsible, the artefacts required, and the competence needed.
