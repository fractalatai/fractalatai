---
session: "Taxa DRRP Gap Analysis: PUBLIC"
status: closed
opened: 2026-04-21
closed: 2026-04-25
outcome: success

summary: >
  Gap analysis for PUBLIC family (52 laws, 7,740 provisions). Discovered enrichment
  truncation bug (#33, 80 laws affected corpus-wide). Found 3 domain-specific actors
  (provider, keeper, dealer) and OFCOM gov pattern gap. Recall 19.4%→59.1% (+39.7pp),
  Online Safety Act from 0.4%→64.7%. Discovered offence-as-duty pattern tier (953
  corpus-wide provisions).

decisions:
  - what: "Family-gated PUBLIC_GOVERNED_DEFS with provider, keeper, dealer"
    why: "\"Provider\" is OSA's primary duty-holder but too generic for core GOVERNED_DEFS (would match healthcare, employment law)"
    result: "Family-gated on PUBLIC, same pattern as offshore licensee"
  - what: Add OFCOM + 6 other keywords to GOVERNMENT_ACTORS gate
    why: "Gate was built for OH&S law \u2014 missing OFCOM, chief officer, constable, sheriff, procurator fiscal"
    result: "292 OFCOM provisions recovered, dominant addressable gap"
  - what: "\"Ind: Person\" compound predicate expansion is at diminishing returns"
    why: "89% of 219 Person Gap A provisions have person in object/beneficiary/passive position \u2014 correctly not getting DRRP"
    result: "No code change, documented analysis"
  - what: Offence-as-duty is a new pattern tier (logged for separate session)
    why: "\"It shall be unlawful for any person to keep a dog\" expresses a duty without any modal verb \u2014 953 provisions corpus-wide, 76% have no DRRP"
    result: fractalaw/fractalaw#34 logged

metrics:
  laws: 52
  provisions: 7740
  recall_start: 19.4
  recall_end: 59.1
  f1_start: 32.0
  f1_end: 68.1
  osa_recall_start: 0.4
  osa_recall_end: 64.7
  offence_as_duty_corpus: 953
  enrichment_truncation_laws_affected: 80

lessons:
  - title: Enrichment truncation bug silently dropped provisions beyond limit=500
    detail: "enrich_single_law() queried LanceDB with limit=500. Laws with >500 provisions silently truncated. 80 laws, 52,846 provisions affected corpus-wide. Fixed in #33 (d72a702)."
    tag: infrastructure
  - title: Gap profile changes fundamentally after fixing data pipeline bugs
    detail: Before truncation fix, Gap C was 82% of FN. After, Gap A became 61%. Many former Gap C were actually truncated provisions with no data at all.
    tag: methodology
  - title: "Precision drop from gate bypass is mostly false — ground truth heuristic too conservative"
    detail: 134 of 216 FP are Interpretation-primary provisions that DO contain genuine duties. Real-world precision likely >85%, not 75.8%.
    tag: methodology
  - title: "Offence-creating language is a fundamentally new pattern class"
    detail: "\"It is an offence for...\" expresses duties without modal verbs. 922 of 953 provisions have no modal at all — completely invisible to the current pipeline. Needs its own tier."
    tag: architecture
  - title: "'Ind: User' in Online Safety Act is false actor extraction — user means service user (beneficiary)"
    detail: "53 Gap A provisions, 0 have user as duty-holder. The v2 matcher correctly rejects. Inflates Gap A count but no DRRP impact."
    tag: data

artifacts:
  - crates/fractalaw-core/src/taxa/actors.rs
  - crates/fractalaw-core/src/taxa/duty_patterns.rs
  - crates/fractalaw-cli/src/main.rs
  - .claude/skills/lat-qa/SKILL.md

depends_on:
  - 03-28-26-ohs-offshore-safety.md
  - 04-14-26-ohs-occupational-safety.md

enables:
  - "Offence-as-duty pattern tier (fractalaw/fractalaw#34)"
  - Gap C AI/LLM session
  - LAT QA skill for upstream data quality checks
---

# Session: 2026-04-21 — Taxa DRRP Gap Analysis: PUBLIC (CLOSED)

## Context

**Skill**: [taxa-gap-analysis/SKILL.md](../../../skills/taxa-gap-analysis/SKILL.md) (consolidated skill — covers full gap analysis + verification lifecycle)
**Family**: `PUBLIC`
**Objective**: QA taxa DRRP coverage for the PUBLIC family — public safety laws covering firearms, dangerous dogs, online safety, terrorism protection of premises, fatal accidents inquiries. Identify gaps, measure precision/recall, determine if pattern improvements are needed.

## Family Profile

- 52 laws in DuckDB under `PUBLIC`
- 16 making, 11 making with body text
- 14 laws with DRRP data
- 17 laws with LAT data in LanceDB (3,624 provisions per QA report; 7,740 from direct LanceDB query including sub-provisions)
- Dominated by UK_ukpga_2023_50 (Online Safety Act) at 4,181 provisions — 54% of all provision rows

### Key Laws

| Law | Title | Prov (LanceDB) | DRRP% | Domain |
|-----|-------|----------------|-------|--------|
| UK_ukpga_2023_50 | Online Safety Act | 4,181 | 0.4% | Internet platform regulation |
| UK_ukpga_1968_27 | Firearms Act | 839 | 10.6% | Firearms control |
| UK_nisi_1983_764 | Dogs (NI) Order | 535 | 33.9% | Dog control |
| UK_ukpga_2025_10 | Terrorism (Protection of Premises) Act | 501 | 59.4% | Venue security |
| UK_asp_2016_2 | Fatal Accidents Inquiries (Scotland) Act | 489 | 25.8% | Fatal accident inquiries |
| UK_ukpga_2003_22 | (unknown — 197 prov) | 200 | 26.2% | |
| UK_uksi_2015_138 | Dangerous Dogs Exemption Schemes | 183 | 42.9% | Dog control |
| UK_ukpga_1991_65 | Dangerous Dogs Act | 143 | 65.2% | Dog control |
| UK_nisr_2024_155 | (NI Regs 2024) | 133 | 11.6% | |
| UK_ssi_2024_70 | Dangerous Dogs (Scotland) Order | 124 | 71.4% | Dog control |

## Step 1: QA Report Baseline

### Coverage Summary

| Law | Provisions | DRRP% | Gated% | Notes |
|-----|-----------|-------|--------|-------|
| UK_asp_2016_2 | 472 | 8.7% | 10.4% | Fatal Accidents — low DRRP |
| UK_nisi_1983_764 | 486 | 18.7% | 8.0% | Dogs NI — moderate |
| UK_ukpga_1968_27 | 495 | 7.9% | 10.1% | Firearms Act — low DRRP |
| UK_ukpga_1991_65 | 143 | 37.1% | 11.2% | Dangerous Dogs — moderate |
| UK_ukpga_2023_50 | 498 | 4.8% | 6.2% | Online Safety Act — extremely low |
| UK_ukpga_2025_10 | 494 | 23.3% | 8.3% | Terrorism Premises — best large law |
| UK_ssi_2024_70 | 123 | 30.9% | 9.8% | Dogs Scotland |
| UK_uksi_2023_1204 | 88 | 39.8% | 14.8% | Dogs Exemption — good |
| UK_uksi_2023_1407 | 100 | 29.0% | 16.0% | Dogs Exemption/Misc |
| UK_wsi_2006_1702 | 36 | 22.2% | 19.4% | Housing Health (Wales) |

Corpus totals: 3,624 provisions (QA report), 84.0% Process+Rule, 8.6% Offence, 8.6% Interpretation, 3.3% Amendment.

### Gate Analysis

| Gate | Triggered | % corpus |
|------|-----------|----------|
| skip_drrp (all) | 355 | 9.8% |
| — Interpretation-primary | 159 | 4.4% |
| — Enactment-primary | 76 | 2.1% |
| — Application+Scope | 54 | 1.5% |
| — All structural | 66 | 1.8% |
| descriptive_summary | 6 | 0.2% |
| **Total gated** | **361** | **10.0%** |

Gate rate is healthy (10%) — not over-gating. Compare OH&S: Occupational at 24.9%.

### Anomalies

Only 1 anomaly flagged:
- UK_nia_2001_1: Enactment 40.0% (>10%) — only 15 provisions, small law

Purpose distribution anomalies are mostly high Offence% (UK_nisi_1983_764: 20.6%, UK_ukpga_1968_27: 18.2%, UK_ukpga_1991_65: 18.9%) and high Exempt% on dangerous dogs exemption schemes (UK_nisr_2024_155: 36.8%, UK_ssi_2024_70: 43.1%, UK_uksi_2023_1204: 40.9%, UK_uksi_2023_1407: 52.0%). Both are genuine characteristics of the subject matter — dangerous dogs law is heavily offence-driven, and exemption scheme SIs are naturally exemption-heavy.

## Step 2: Confusion Matrix

7,740 provisions from LanceDB across 17 laws.

|  | Predicted: DRRP | Predicted: No DRRP | Total |
|--|----------------:|-------------------:|------:|
| **Expected: DRRP** | 461 (TP) | 1,919 (FN) | 2,380 |
| **Expected: No DRRP** | 44 (FP) | 5,316 (TN) | 5,360 |
| **Total** | 505 | 7,235 | 7,740 |

| Metric | Value |
|--------|-------|
| **Precision** | 91.3% |
| **Recall** | 19.4% |
| **F1** | 32.0% |

### Excluding Online Safety Act

The Online Safety Act (4,181 provisions, 4 TP, 1,056 FN) dominates. Without it:

| Metric | All Laws | Excluding OSA |
|--------|----------|---------------|
| Provisions | 7,740 | 3,559 |
| Recall | 19.4% | 34.6% |
| FN total | 1,919 | 863 |

### Per-Law Breakdown

| Law | Prov | TP | FN | FN-A | FN-C | Recall |
|-----|------|----|----|------|------|--------|
| UK_ukpga_2023_50 (Online Safety) | 4,181 | 4 | 1,056 | 9 | 1,047 | 0.4% |
| UK_ukpga_1968_27 (Firearms) | 839 | 34 | 286 | 28 | 258 | 10.6% |
| UK_nisi_1983_764 (Dogs NI) | 535 | 83 | 162 | 82 | 80 | 33.9% |
| UK_asp_2016_2 (Fatal Accidents) | 489 | 33 | 95 | 57 | 38 | 25.8% |
| UK_ukpga_2025_10 (Terrorism Premises) | 501 | 92 | 63 | 18 | 45 | 59.4% |
| UK_ukpga_2003_22 | 200 | 17 | 48 | 27 | 21 | 26.2% |
| UK_uksi_2015_138 (Dogs Exemption) | 183 | 30 | 40 | 30 | 10 | 42.9% |
| UK_nisr_2024_155 | 133 | 5 | 38 | 35 | 3 | 11.6% |
| UK_uksi_1998_1941 (Firearms Rules) | 77 | 11 | 36 | 13 | 23 | 23.4% |
| UK_ukpga_1991_65 (Dangerous Dogs) | 143 | 45 | 24 | 8 | 16 | 65.2% |
| UK_nia_2011_9 (Dogs Amendment NI) | 43 | 5 | 16 | 15 | 1 | 23.8% |
| UK_uksi_2023_1407 (Dogs Exemption/Misc) | 101 | 27 | 15 | 7 | 8 | 64.3% |
| UK_ssi_2024_70 (Dogs Scotland) | 124 | 30 | 12 | 5 | 7 | 71.4% |
| UK_uksi_2006_798 (Dog Control Procedures) | 48 | 8 | 10 | 2 | 8 | 44.4% |
| UK_nia_2001_1 | 15 | 1 | 9 | 6 | 3 | 10.0% |
| UK_uksi_2023_1204 (Dogs Compensation) | 89 | 29 | 9 | 4 | 5 | 76.3% |
| UK_wsi_2006_1702 (Housing Wales) | 39 | 7 | 0 | 0 | 0 | 100.0% |

### False Negative Breakdown (1,919)

| Category | Count | % of FN |
|----------|-------|---------|
| **Gap C** (no actor at all) | 1,573 | 82% |
| **Gap A** (actor present, no DRRP) | 346 | 18% |

| Modal Type | Count |
|------------|-------|
| Obligation (shall/must) | 1,040 |
| Enabling only (may/power to) | 879 |

### Gap A: Governed Actor Labels in Misses

| Actor Label | Misses | In GOVERNED_DEFS? | Notes |
|-------------|--------|-------------------|-------|
| Ind: Person | 194 | YES | Known: bare "person" too broad |
| Public | 23 | YES | Broad — "public" appears in context |
| Org: Owner | 19 | YES | Dog owner — duty-holder in this family |
| Org: Employer | 14 | YES | Cross-domain |
| Org: Occupier | 11 | YES | Cross-domain |
| Spc: Assessor | 3 | YES | |
| Org: Company | 3 | YES | |
| Operator | 1 | YES | |
| Ind: Employee | 1 | YES | |

All Gap A governed actors are already in GOVERNED_DEFS — these are v2 matcher failures (actor detected but v2 pattern matching doesn't fire), same as the residual 50 provisions found in OH&S: Occupational analysis.

### Gap A: Government Actor Labels in Misses

| Actor Label | Misses | Notes |
|-------------|--------|-------|
| Gvt: Authority: Local | 58 | District council duties (dogs, housing) |
| Gvt: Judiciary | 49 | Court/sheriff powers and duties |
| Gvt: Ministry | 48 | Secretary of State, Department |
| Gvt: Officer | 36 | Authorised officers |
| Gvt: Agency | 16 | Generic agency |
| Gvt: Emergency Services: Police | 13 | Constable/chief officer powers |
| Gvt: Agency: OFCOM | 8 | OFCOM duties in Online Safety Act |
| Gvt: Minister | 4 | |
| Gvt: Devolved Admin | 1 | |
| Crown | 1 | |
| Gvt: Appropriate Person | 1 | |

Government actor FN is expected — gov v1/v2 patterns are keyword-based (not actor-anchored) so they have structural limitations in matching all government obligations. 235 government actor FN provisions is consistent with other families.

### False Positive Breakdown (44)

Low FP count — precision is 91.3%. Slightly lower than OH&S (96.4%) but healthy for a 52-law family.

## Step 3: Online Safety Act Investigation

### The Problem

UK_ukpga_2023_50 has 4,181 LanceDB provisions but only 4 get DRRP (0.4% recall). This is the single largest gap.

**Provision structure analysis**:
- Empty: 2
- Short (<200 chars): 2,739 (66%)
- Medium (200-1000): 1,189
- Long (1000-5000): 217
- Very long (5000-20000): 22
- Enormous (>20000): 12
- Max length: 148,890 chars

The OSA has 12 provisions over 20KB and the largest is **148KB** — a single provision containing an entire Part's worth of text. These are not per-section provisions — sertantai has parsed the Act into large structural blocks mixing definitions, duties, and procedural text.

### Root Cause 1: Mixed-Content Provision Problem (same as OH&S product safety SIs)

The enormous provisions (12 provisions >20KB, 22 >5KB) contain definitions at the start → Interpretation-primary gate fires → suppresses genuine duties buried later. This is the same issue identified in the OH&S: Occupational session (commit 2cc3aa0 added gate bypass when governed actor present).

However, the OSA uses "provider" as its primary duty-holder, and **"provider" is not in GOVERNED_DEFS**. So the gate bypass doesn't help — no governed actor is extracted → gate fires → DRRP suppressed.

### Root Cause 2: "Provider" Not in Actor Patterns

The Online Safety Act's primary duty-holder is **"provider"** (of regulated services). This appears in clear duty provisions:

- "A provider of a Part 3 service must carry out the first children's access assessment"
- "A provider must make and keep a written record"
- "The provider must carry out children's access assessments"

52 short/medium provisions have clear "provider...must" patterns with no DRRP classification. In the full corpus including long provisions, "provider" appears in 1,143 FN provisions.

**"Provider" is domain-specific to the PUBLIC family** (specifically online safety / telecoms regulation). It should be added as a **family-gated specialist actor** in `actors.rs`, not to the core GOVERNED_DEFS, to avoid false matches in OH&S law where "provider" has different semantics.

### Root Cause 3: Short Sub-Provision Fragments

2,739 provisions (66%) are <200 chars. Many are clause fragments from sertantai's parsing — subordinate clauses, cross-references, bare paragraph text. These often contain modal verbs ("must be given", "shall apply") but lack actor context. This is a sertantai text-parsing issue, not a fractalaw pattern issue.

### Domain-Specific Actors for PUBLIC Family

Keyword search across FN provisions identified these domain-specific actors:

| Keyword | FN Count | Role | Recommendation |
|---------|----------|------|----------------|
| **provider** | 1,143 | Duty-holder (Online Safety Act) | Add as family-gated specialist |
| **keeper** | 89 | Duty-holder (dog legislation) | Add as family-gated specialist |
| **dealer** | 148 | Duty-holder (Firearms Act — "registered firearms dealer") | Add as family-gated specialist |
| **applicant** | 151 | Mixed — sometimes duty-holder, often procedural | Investigate further |
| **court** | 300 | Government actor — already in Gvt: Judiciary | Already covered |
| **chief officer** | 285 | Government actor — already in Gvt: Emergency Services: Police | Already covered |
| **sheriff** | 113 | Government actor — already in Gvt: Judiciary | Already covered |
| **constable** | 110 | Government actor — already in Gvt: Emergency Services: Police | Already covered |

**Note**: The high counts for government actors (court 300, chief officer 285) in FN provisions are expected. Government DRRP uses gov v1/v2 patterns which are keyword-based and don't achieve full coverage. These aren't addressable via actor patterns alone.

## Step 4: Candidate Fixes

### Fix 1: Family-Gated Specialist Actors for PUBLIC

Add `PUBLIC_GOVERNED_DEFS` in `actors.rs` with:

```rust
const PUBLIC_GOVERNED_DEFS: &[(&str, &str)] = &[
    actor!(
        "Public: Provider",
        r"(?:[\s[:punct:]])(?:[Pp]roviders?|service provider)(?:[\s[:punct:]])"
    ),
    actor!(
        "Public: Keeper",
        r"(?:[\s[:punct:]])[Kk]eepers?(?:[\s[:punct:]])"
    ),
    actor!(
        "Public: Dealer",
        r"(?:[\s[:punct:]])(?:[Dd]ealers?|registered (?:firearms )?dealer)(?:[\s[:punct:]])"
    ),
];
```

Gate on `family == "PUBLIC"` in `specialist_governed_for()`.

**Expected impact**:
- "Provider" will be extracted as a governed actor in OSA provisions
- This enables the Interpretation-primary gate bypass (Fix 1 from OH&S session) to work for OSA mixed-content provisions
- v2 matcher can anchor against "provider" for short provisions
- "Keeper" addresses 89 FN in dogs legislation
- "Dealer" addresses up to 148 FN in Firearms Act

**Risk**: "Provider" is a common English word. Need true-negative tests for:
- "provider" in definition text (should extract actor but gate should still suppress)
- "provider" as object, not subject ("information from the provider" — v2 matcher should correctly reject)

### Fix 2 (Not Recommended): Core GOVERNED_ACTORS Expansion

Adding "provider", "keeper", "dealer" to core GOVERNED_DEFS would cause false positives across other families. "Provider" would match healthcare providers, service providers in employment law, etc. Family-gating is the correct approach.

## Triage Summary

| Issue | Impact | Addressable? | Priority |
|-------|--------|-------------|----------|
| **"Provider" not in actors** | ~1,143 FN (OSA alone) | Yes — family-gated specialist | HIGH |
| **Mixed-content provisions** (long OSA parts) | ~34 enormous provisions | Partially — gate bypass helps if actor extracted | MEDIUM |
| **"Keeper" not in actors** | ~89 FN (dogs legislation) | Yes — family-gated specialist | MEDIUM |
| **"Dealer" not in actors** | ~148 FN (firearms) | Yes — family-gated specialist | MEDIUM |
| **Gap A: v2 matcher failures** | 346 FN across all actors | Low ROI — same residual as OH&S | LOW |
| **Gap C: No actor** | 1,573 FN — passive voice, fragments | No (AI frontier) | DEFERRED |
| **Short fragment provisions** | 2,739 provisions <200 chars | No (sertantai parsing) | DEFERRED |

## Comparison with Other Families

| Metric | Offshore (2,737) | Occupational (20,192) | PUBLIC (7,740) |
|--------|-----------------|----------------------|----------------|
| Precision | 99.1% | 96.4% | 91.3% |
| Recall | 72.6% | 48.6% | 19.4% |
| F1 | 83.8% | 64.6% | 32.0% |
| Gap C % of FN | 58% | 78% | 82% |

PUBLIC has the lowest recall across analysed families. This is driven by:
1. The Online Safety Act's enormous provision blocks (unique parsing challenge)
2. Domain-specific actors ("provider", "keeper", "dealer") not yet added
3. High proportion of criminal/offence law (duties on "a person who..." — broad, Gap C territory)

## Next Steps

- [ ] **Fix 1**: Add `PUBLIC_GOVERNED_DEFS` with provider, keeper, dealer (family-gated specialist actors)
- [ ] **Fix 1b**: Add true-negative tests for provider in definitions, provider as object
- [ ] **Fix 1c**: Add true-positive tests using real OSA provision text
- [ ] **Fix 1d**: Re-enrich PUBLIC family, measure improvement
- [ ] Investigate "applicant" as potential 4th specialist actor (151 FN — need to check if duty-holder or procedural)
- [ ] Log OSA short-fragment issue for sertantai text-parsing review
- [ ] Log Gap C for future AI session

---

## Post-Fix Reanalysis (2026-04-24)

### Enrichment Truncation Fix

fractalaw/fractalaw#33 fixed in commit d72a702 — raised limit from 500 to 100,000 in all 4 affected call sites. PUBLIC family re-enriched with `--force`.

### Revised QA Baseline

| Metric | Before (truncated) | After (fixed) | Delta |
|--------|-------------------|---------------|-------|
| QA provisions | 3,624 | **7,413** | +3,789 |
| OSA provisions | 498 | **4,128** | +3,630 |
| OSA DRRP% | 4.8% | **8.9%** | +4.1pp |
| Corpus DRRP% | 15.8% | **11.3%** | -4.5pp (diluted by newly-visible provisions) |

### Revised Confusion Matrix (7,740 provisions)

|  | Predicted: DRRP | Predicted: No DRRP | Total |
|--|----------------:|-------------------:|------:|
| **Expected: DRRP** | 678 (TP) | 1,282 (FN) | 1,960 |
| **Expected: No DRRP** | 216 (FP) | 5,564 (TN) | 5,780 |
| **Total** | 894 | 6,846 | 7,740 |

| Metric | Before | After | Delta |
|--------|--------|-------|-------|
| **Precision** | 91.3% | 75.8% | -15.5pp |
| **Recall** | 19.4% | **34.6%** | **+15.2pp** |
| **F1** | 32.0% | **47.5%** | **+15.5pp** |
| OSA recall | 0.4% | **29.4%** | +29pp |

#### Precision Drop Explained

134 of 216 FP are `Interpretation+Definition`-primary provisions that DO contain genuine duties (gate bypass from OH&S Fix 1, commit 2cc3aa0). Examples: "the procurator fiscal must bring forward evidence", "The SCTS must publish". The ground truth heuristic ("Interpretation-primary = not expected positive") is too conservative for these mixed-content provisions. Real-world precision is likely >85%.

### Revised Gap Breakdown (1,282 FN)

| Category | Count | % of FN | Change from pre-fix |
|----------|-------|---------|---------------------|
| **Gap A** (actor present) | 787 | 61% | Was 346 (18%) — now dominant |
| **Gap C** (no actor) | 495 | 39% | Was 1,573 (82%) — dropped |
| FN obligation | 652 | 51% | |
| FN enabling | 630 | 49% | |

The ratio flipped: Gap A is now 61% of FN (was 18%). This is because the newly-enriched provisions are substantive duty text where actors ARE detected but DRRP matching fails. Gap C dropped from 1,573 to 495 — many former "Gap C" were actually truncated provisions with no data at all.

### Revised Gap A: Governed Actors

| Actor Label | Misses | Notes |
|-------------|--------|-------|
| Ind: Person | 339 | Known-broad, existing predicates only |
| **Ind: User** | **88** | **NEW — OSA "users of the service"** |
| Public | 31 | Broad |
| Org: Company | 24 | |
| Org: Owner | 9 | Dog owner |
| Org: Occupier | 5 | |
| Org: Employer | 5 | |
| Ind: Manager | 5 | |

### Revised Gap A: Government Actors

| Actor Label | Misses | Notes |
|-------------|--------|-------|
| **Gvt: Agency: OFCOM** | **292** | **Dominant gap — OSA regulatory powers/duties** |
| Gvt: Officer | 87 | Authorised officers |
| Gvt: Judiciary | 68 | Court/sheriff |
| Gvt: Authority: Local | 50 | District council |
| Gvt: Ministry | 42 | Secretary of State, Department |
| Gvt: Minister | 34 | |
| Gvt: Emergency Services: Police | 21 | |

### Revised Assessment

The gap profile has fundamentally changed:

1. **OFCOM (292 misses)** is now the dominant addressable gap. OFCOM is a government actor — already extracted by `actors.rs` as `Gvt: Agency: OFCOM`. But gov v1/v2 patterns aren't matching the OSA's regulatory language. The OSA uses patterns like "OFCOM must prepare a code of practice" which don't match the existing gov v1 keyword patterns (designed for HSE-style regulation).

2. **"Provider" (not in actors)** — still needed but impact is lower than originally estimated. Many "provider must" provisions now get DRRP through other paths (rule matcher, government patterns). Family-gated specialist still valuable.

3. **"User" (88 misses)** — already in `GOVERNED_DEFS` as `Ind: User`. These are v2 matcher failures, not missing actors.

4. **Gap C dropped to 495** (was 1,573) — the passive-voice problem is much smaller than originally thought. Most of the original "Gap C" was actually truncated provisions.

## Fix 1: OFCOM Gov Patterns + PUBLIC Specialist Actors (eef298f)

### Root Cause: OFCOM

`duty_patterns.rs` `GOVERNMENT_ACTORS` is a flat substring list that gates gov v1/v2 pattern matching. "ofcom" was not in the list — it was built for OH&S law. Added 7 keywords: ofcom, chief officer, constable, police, sheriff, procurator fiscal, department.

### Root Cause: Provider/Keeper/Dealer

These domain-specific actors were not in `GOVERNED_DEFS`. Added as `PUBLIC_GOVERNED_DEFS` in `actors.rs`, family-gated on `family == "PUBLIC"` (same pattern as offshore licensee).

### Results

| Metric | Pre-OFCOM fix | Post-OFCOM + specialists | Delta |
|--------|--------------|-------------------------|-------|
| **TP** | 678 | **1,158** | **+480** |
| **FN** | 1,282 | **802** | **-480** |
| **Recall** | 34.6% | **59.1%** | **+24.5pp** |
| **Precision** | 75.8% | **80.5%** | +4.7pp |
| **F1** | 47.5% | **68.1%** | **+20.6pp** |
| Gap A | 787 | **381** | -406 |
| Gap C | 495 | **421** | -74 |
| OSA recall | 29.4% | **64.7%** | +35.3pp |

### Full Session Delta (baseline → final)

| Metric | Baseline (start of session) | Final | Total delta |
|--------|---------------------------|-------|-------------|
| **Recall** | 19.4% | **59.1%** | **+39.7pp** |
| **F1** | 32.0% | **68.1%** | **+36.1pp** |
| OSA recall | 0.4% | **64.7%** | **+64.3pp** |

### Remaining Gaps

Gap A (381) and Gap C (421) are now roughly balanced — low-hanging fruit has been picked.

## Ind: Person Investigation (2026-04-25)

### Breakdown of 219 "Ind: Person" Gap A Provisions

| Category | Count | Addressable? |
|----------|-------|-------------|
| "(other)" — person as object/passive/reference | 115 | NO — grammatically passive |
| "the person" — anaphoric reference | 56 | NO — mostly object/beneficiary |
| "a person who" — compound should match | 22 | NO — 18/22 "person who" appears AFTER modal (object); 4 before modal are offence provisions |
| "person...offence/guilty" | 21 | NO — penalties, true negatives |
| "any person" (no qualifier) | 20 | NO — 16/20 are objects |
| "a person shall/must" | 2 | TOO FEW — not worth a pattern |
| "person responsible" / "person in charge" | 3 | TOO FEW |

### Key Finding

The existing `PERSON_QUALIFIERS` regex in `duty_patterns_v2.rs` is correctly restrictive. Of 219 provisions where `Ind: Person` is extracted but no DRRP results:

- **~195 (89%)** have "person" in object/beneficiary/passive position — correctly NOT getting DRRP
- **~22 (10%)** have "a person who" but AFTER the modal — object, not subject
- **~2 (1%)** might be genuine misses ("the person in interim charge must comply") — too few to justify pattern changes

**Conclusion**: "Ind: Person" compound predicate expansion is at diminishing returns — person is overwhelmingly object/beneficiary in PUBLIC law. However, the investigation uncovered a **new pattern class**: offence-creating language as implicit duty.

### Discovery: Offence-Creating Language as Implicit Duty

The provision "it shall be unlawful for any person to keep a dog" expresses a duty without any modal verb. The pipeline is blind to this pattern class because all pattern tiers require shall/must/may as an entry point.

**Corpus-wide scope**: 1,241 provisions match offence-creating patterns, **953 (76%) have no DRRP**. This is a fundamentally new tier — **"offence-as-duty"**.

| Pattern | Total | No DRRP | Miss% |
|---------|-------|---------|-------|
| "it is an offence for" | 209 | 195 | 93% |
| "commits an offence if" | 206 | 196 | 95% |
| "shall be guilty of an offence" | ~164 | ~164 | ~100% |
| "unlawful for" | 8 | 7 | 88% |
| **Total (deduplicated)** | **1,241** | **953** | **76%** |

Of the 953 misses, **922 have no modal verb at all** — completely invisible to the current pipeline. The remaining 31 have a modal but the offence language still causes them to miss.

**DRRP classification**: These are **Duty (Prohibitive)** — "it is an offence for a person to X" means the person has a duty not to do X.

**Architecture**: A new pattern tier in `duty_type.rs`, after governed v2 and gov v1/v2 but before rule. The matcher would:
1. Detect offence-creating language (regex)
2. Extract the duty-holder from "for [person/actor] to" or "[person] commits an offence if"
3. Classify as `Governed / Prohibitive`

**Decision**: This warrants a dedicated session. Logged for follow-up.

## Ind: User Investigation (2026-04-25)

53 `Ind: User` Gap A FN provisions — **all 53 from the Online Safety Act**.

### Finding: False Actor Extraction, Not v2 Failure

In the OSA, "user" means "user of the service" — the **protected party**, not a duty-holder. The `Ind: User` regex in `actors.rs` (`[Uu]sers?`) matches this, but:

- **23/53** contain "user-to-user" — a compound term for a service type, not an actor
- **14/53** have "user" as object/beneficiary ("users of the service", "United Kingdom users")
- **8/53** appear subject-adjacent but are Part-level definition blobs (6) or provisions where provider/OFCOM is the real duty-holder (2)
- **0/53** have "user" as the duty-holder

**Conclusion**: The v2 matcher is correctly NOT matching these. This is a false positive in actor extraction — `Ind: User` should not be extracted from OSA provisions where "user" means "service user" (beneficiary). However, since v2 doesn't anchor against it, the impact is limited to inflating Gap A counts in analysis. No code change needed.

**Possible future improvement**: Add "user-to-user" to the `BLACKLIST` in `actors.rs` to prevent `Ind: User` extraction from the compound term. Low priority — cosmetic improvement to actor labels, no DRRP impact.

## Session Summary

### Fixes Applied

| Fix | Commit | Recall Delta |
|-----|--------|-------------|
| Enrichment truncation (#33) | d72a702 | 19.4% → 34.6% |
| OFCOM gov patterns + PUBLIC specialist actors | eef298f | 34.6% → 59.1% |

### Final Metrics

| Metric | Start | End |
|--------|-------|-----|
| **Recall** | 19.4% | **59.1%** |
| **F1** | 32.0% | **68.1%** |
| **OSA Recall** | 0.4% | **64.7%** |

### Remaining Gaps (Not Addressable by Regex)

| Category | Count | Status |
|----------|-------|--------|
| Gap A (actor present) | 381 | Diminishing returns — Ind: Person (object), Ind: User (false extraction) |
| Gap C (no actor) | 421 | AI frontier — passive voice, actor-less obligations |
| Offence-as-duty (#34) | 953 corpus-wide | New pattern tier — separate session |

### Next Steps (Other Sessions)

- fractalaw/fractalaw#34: Offence-as-duty pattern tier (953 provisions)
- Gap C: AI/LLM session for passive-voice provisions
- Re-run after sertantai#69 re-sync (Part blob cleanup)

---

**Session closed**: 2026-04-25. Regex improvements for PUBLIC family exhausted.

## Appendix: Upstream Data Quality Issues (Added Post-Analysis)

### Critical: Enrichment Truncation Bug

`enrich_single_law()` at `crates/fractalaw-cli/src/main.rs:~2859` queries LanceDB with `limit=500`. Laws with >500 provisions are silently truncated — only the first 500 rows get enriched.

**Impact on this analysis**: 4 of 17 PUBLIC laws are truncated:

| Law | Total Provisions | Enriched (max) | Lost |
|-----|-----------------|----------------|------|
| UK_ukpga_2023_50 (Online Safety Act) | 4,181 | 500 | 3,681 |
| UK_ukpga_1968_27 (Firearms Act) | 839 | 500 | 339 |
| UK_nisi_1983_764 (Dogs NI Order) | 535 | 500 | 35 |
| UK_ukpga_2025_10 (Terrorism Premises) | 501 | 500 | 1 |

**This is the primary cause of the OSA's 0.4% recall** — not missing actor patterns. Only 5 out of 4,094 section-level provisions were enriched because the enricher hit its 500-row limit before reaching most section-level rows. The confusion matrix and Gap A/C analysis for these 4 laws is unreliable.

**Corpus-wide**: 80 laws (52,846 provisions) are affected across all families.

**Fix required**: Increase or remove the limit before re-running this gap analysis.

### Secondary: Part-Level Blob Duplication

Sertantai produces both section-level provisions AND large Part/Chapter blobs containing the full concatenated text. The Online Safety Act has 34 Part/Chapter blobs totalling 753KB of duplicated text (38% of all text). These inflate provision counts and consume enrichment budget on duplicate text.

**Mitigation**: The enricher should either skip Part/Ch/Sch rows or filter them at query time.

### Corrected Assessment

The gap analysis in Steps 1-4 above is **partially invalid** for the 4 truncated laws. The findings about family-gated specialist actors (provider, keeper, dealer) remain valid — those actors ARE missing from patterns — but the quantitative impact estimates are unreliable. The analysis should be re-run after the truncation fix.

**Revised next steps**:
1. Fix enrichment truncation bug (raise/remove 500-row limit)
2. Re-enrich PUBLIC family with `--force`
3. Re-run confusion matrix and gap analysis
4. Then implement family-gated specialist actors if still needed

---

**Session status**: RESUMED (2026-04-24). #33 fixed (d72a702), re-enriched. Reanalysis complete — gap profile changed fundamentally. OFCOM gov pattern gaps now dominant.

### LAT QA Skill Created

A new skill `.claude/skills/lat-qa/SKILL.md` formalises the upstream data quality checks that should run BEFORE DRRP gap analysis. Checks: enrichment truncation (BLOCKER), Part-level blob duplication (WARNING), provision granularity, empty text, section ID consistency.
