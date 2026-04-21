# Session: 2026-04-21 — Taxa DRRP Gap Analysis: PUBLIC

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

**Session status**: Analysis complete. Fix 1 ready to implement.
