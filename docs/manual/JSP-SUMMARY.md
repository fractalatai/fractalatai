# MoD JSP Analysis: From Policy Documents to Actionable Compliance Intelligence

## The Problem

QQ operates under MoD contracts. Those contracts require compliance with Joint Service Publications (JSPs) — the Ministry of Defence's internal policy framework for safety, environment, and operational management. JSPs sit between legislation and day-to-day operations: they take statutory requirements like the Health and Safety at Work Act and translate them into specific MoD responsibilities, roles, and procedures.

Today, understanding what JSPs require means reading hundreds of pages of PDF across multiple publications. Answering basic questions — "What must a contractor do under JSP 375?" or "Which JSP obligations map to the Electricity at Work Regulations?" — requires manual cross-referencing between documents that were never designed to be queried.

## What We Built

We parsed 13,854 provisions across 10 MoD JSPs into structured, queryable data — then enriched every provision with obligation classification, role assignments, cross-references, and competence requirements. The same provisions that exist as paragraphs in a PDF are now rows in a database that can be filtered, linked, and reported on.

### Four Layers of Intelligence

**1. Obligation Classification**

Every JSP provision is classified by obligation strength:

| Strength | Meaning | Count (pilot) |
|----------|---------|---------------|
| Mandatory | "must", "shall", "will", "is to" — a binding requirement | 88 |
| Recommended | "should" — expected but not absolute | 22 |
| Permissive | "may" — allowed but not required | 7 |

JSPs use "will" and "is to" as mandatory — unlike legislation, where "will" is future tense. Our parser handles this distinction automatically.

**2. Role Assignments (RACI)**

JSPs assign organisational roles to obligations. For each obligation, we extract who is Responsible, Accountable, Consulted, or Informed:

| Role | Obligations assigned |
|------|---------------------|
| Accountable Person | 15 |
| User / Operator | 7 |
| Defence Organisation | 5 |
| Contractor | 2 |
| Defence Safety Authority | 2 (Informed) |

The query "show me everything a contractor must do across all applicable JSPs" is now a database filter, not a reading exercise.

**3. Legislative Traceability**

JSP provisions reference the legislation they implement. We extract and resolve these references automatically:

- "in accordance with the Electricity at Work Regulations 1989" → linked to the specific UK statutory instrument
- "as set out in JSP 375 Volume 1, Chapter 8" → linked to the specific JSP chapter

From a single JSP chapter (JSP 375 Chapter 23, Electrical Safety), we extracted 63 cross-references — 10 to legislation, 43 to other JSP chapters, 8 to HSE guidance, 2 to British Standards. 82% were automatically resolved to their source.

This means when legislation changes, you can immediately see which JSP obligations are affected.

**4. Competence Requirements**

JSPs frequently require "competent persons" for specific tasks. We extract these requirements and link them to the obligations they qualify. In the pilot chapter, 21 of 117 obligations carry competence requirements — identifying where training, certification, or demonstrated competence is needed.

## What This Means for QQ

### "What do the JSPs require of us?"

Filter all JSP obligations where the Contractor role is Responsible. For JSP 375 Chapter 23 alone, that's 2 mandatory obligations. Across the full JSP corpus (10 publications, 158 chapters), this produces the complete set of contractor-applicable requirements — no manual reading required.

### "Which JSP obligations map to which laws?"

Every JSP obligation that references legislation is linked to the specific law in our corpus. When the Electricity at Work Regulations are amended, you can trace the impact through to every JSP chapter that implements those regulations — and every obligation within those chapters.

### "What competence do our people need?"

Obligations with competence requirements are flagged. "Formal testing of electrical equipment must only be performed by a competent person" is linked to the specific obligation, the responsible role, and the JSP chapter. This feeds directly into training needs analysis.

### "How does this connect to our compliance controls?"

JSP obligations are more operationally detailed than legislation. Where the law says "the employer shall ensure health and safety", the JSP says "the Commanding Officer shall maintain a Safety Case that demonstrates ALARP, reviewed annually by the Operating Duty Holder." These JSP-specific controls can enrich the compliance control register — providing the operational detail that legislation leaves to interpretation.

## Scope

| | |
|---|---|
| JSP publications parsed | 10 (375, 376, 392, 403, 418, 425, 520, 815, 816, 975) |
| Chapters/elements | 158 |
| Provisions | 13,854 |
| Source PDFs processed | 167 |
| Obligation types | Mandatory, Recommended, Permissive |
| RACI types | Responsible, Accountable, Consulted, Informed |
| Cross-reference types | Legislation, JSP, Standards, HSE Guidance |

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
167 JSP PDFs    →      13,854 provisions     →      Obligations classified
                       (text, structure,             Roles assigned (RACI)
                        hierarchy)                   Legislation linked
                                                     Competence flagged
                                                     Controls generated
```

The system pulls JSP provisions from the database, runs them through a purpose-built parser that understands MoD policy language, and publishes enriched data back. The enrichment happens automatically — no manual tagging, no spreadsheets, no reading hundreds of pages.

Results are queryable in the compliance platform alongside the legislation they implement. A single view shows the legal obligation, the JSP policy that implements it, the role responsible, and the competence required.
